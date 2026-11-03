use super::*;
use crate::gguf_bridge::{QTensor, QTensorEngine};
use crate::gguf_sharder::{GgufTensorIndex, GgufTokenizer};

pub(super) struct Selection<'a> {
    pub engine: &'a mut QTensorEngine,
    pub tensor_idx: &'a Option<GgufTensorIndex>,
    pub ctx: &'a Vec<u32>,
    pub tok: &'a GgufTokenizer,
    pub cur: u32,
    pub emb_dim: usize,
    pub emb_buf: &'a mut [f32; 8192],
    pub scratch_a: &'a mut [f32; 16384],
    pub scratch_b: &'a mut [f32; 16384],
    pub lora_adapter: &'a Option<crate::lora::LoRAAdapter>,
    pub sieve: &'a Option<crate::neuro_symbolic_sieve::NeuroSymbolicSieve>,
    pub sampler: &'a mut Option<crate::sampler::SamplerState>,
    pub sampler_logits: &'a mut Vec<f32>,
    pub gpu_topk_enabled: bool,
    pub prompt_owned: &'a str,
    pub chunk_policy: &'a crate::q42::q42_kvp::Q42ChunkPolicy,
    pub current_page_id: u64,
    pub thermal_wal_opt: &'a mut Option<crate::inference::thermal_wal::ThermalWal>,
    pub sieve_failed: bool,
}
pub(super) fn run(input: Selection<'_>) -> (usize, f32, u64, bool) {
    let Selection {
        engine,
        tensor_idx,
        ctx,
        tok,
        cur,
        emb_dim,
        emb_buf,
        scratch_a,
        scratch_b,
        lora_adapter,
        sieve,
        sampler,
        sampler_logits,
        gpu_topk_enabled,
        prompt_owned,
        chunk_policy,
        mut current_page_id,
        thermal_wal_opt,
        mut sieve_failed,
    } = input;
    // 1) Embedding lookup → hidden state (stack dequant).
    let hidden_ok = tensor_idx
        .as_ref()
        .and_then(|idx| {
            engine
                .model_bytes()
                .map(|m| idx.dequantize_token_embedding_into(m, cur, &mut emb_buf[..emb_dim]))
        })
        .unwrap_or(0);

    // 1b) LoRA delta — additive correction to the embedding vector.
    // Applied after dequantize so the base model is unmodified.
    // Silently skipped if dimensions don't match (wrong adapter for model).
    if hidden_ok > 0 {
        if let Some(ref adapter) = lora_adapter {
            if adapter.meta.n_in == hidden_ok && adapter.meta.n_out == hidden_ok {
                let snap: Vec<f32> = emb_buf[..hidden_ok].to_vec();
                let _ = adapter.apply_cpu(&snap, &mut emb_buf[..hidden_ok]);
            }
        }
    }

    let (top_i, top_v) = if hidden_ok > 0 {
        if let Some(idx) = tensor_idx.as_ref() {
            let token_idx = ctx.len().saturating_sub(1) as u32;
            let sieve_mask = sieve.as_ref().map(|s| s.current_mask());
            // Resident-token fast path: the WHOLE forward (32 layers + output norm
            // + logits top-1) in ONE submit with ONE fence. `Some` means the token
            // was produced and the KV cache was written; `None` falls through to
            // the legacy per-layer path unchanged (non-sieve, full-depth only —
            // the unit-test 2-layer cap keeps its per-layer semantics).
            // Resident single-fence forward:
            //   • greedy → GPU top-1 inside the encoder
            //   • sampler → same layer stack, read back post-norm hidden, then
            //     full logits + CPU sample (chat no longer pays ~107 fences/token)
            #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
            let (resident_hit, resident_hidden_ok) = if sieve_mask.is_none()
                && TEST_TRANSFORMER_LAYER_CAP == 0
            {
                let t_res = std::time::Instant::now();
                if sampler.is_some() {
                    // Sampling does not need GPU top-1; resident still wins.
                    // Copy embedding input aside so out_hidden can reuse emb_buf.
                    scratch_a[..emb_dim].copy_from_slice(&emb_buf[..emb_dim]);
                    let ok = engine.dispatch_token_forward_resident_hidden(
                        idx,
                        &scratch_a[..emb_dim],
                        token_idx,
                        &mut emb_buf[..emb_dim],
                    );
                    if ok {
                        crate::llm_bench::add_decode_forward_ns(t_res.elapsed().as_nanos() as u64);
                        crate::llm_bench::record_resident_hit();
                        (None, true)
                    } else {
                        crate::llm_bench::record_resident_fallback();
                        (None, false)
                    }
                } else if gpu_topk_enabled {
                    let hit =
                        engine.dispatch_token_forward_resident(idx, &emb_buf[..emb_dim], token_idx);
                    if hit.is_some() {
                        crate::llm_bench::add_decode_forward_ns(t_res.elapsed().as_nanos() as u64);
                        crate::llm_bench::record_resident_hit();
                    } else {
                        crate::llm_bench::record_resident_fallback();
                    }
                    (hit, false)
                } else {
                    (None, false)
                }
            } else {
                (None, false)
            };
            #[cfg(any(target_arch = "wasm32", not(feature = "gpu-runtime")))]
            let (resident_hit, resident_hidden_ok): (
                Option<crate::gguf_bridge::StreamingArgmaxResult>,
                bool,
            ) = (None, false);

            // CUDA mega-pass: attempt single-fence all-layer forward.
            // Returns Some(token_id) when fully done (including logits).
            // Returns Some(u32::MAX) when forward is done but logits projection
            // is still needed (hidden state has been read back into emb_buf).
            // Returns None to fall back to per-layer path.
            #[cfg(all(not(target_arch = "wasm32"), feature = "cuda"))]
            let mega_token: Option<u32> = if resident_hit.is_none() && !resident_hidden_ok {
                engine.try_cuda_mega_pass_decode(idx, &mut emb_buf[..emb_dim], emb_dim, token_idx)
            } else {
                None
            };
            #[cfg(any(target_arch = "wasm32", not(feature = "cuda")))]
            let mega_token: Option<u32> = None;

            // mega_forward_done = mega-pass completed the 32-layer forward
            // (either fully or just the forward — sentinel u32::MAX means
            // forward done but logits still needed).
            let mega_forward_done = mega_token.is_some();
            let mega_full_token = mega_token.filter(|&t| t != u32::MAX);
            #[cfg(all(not(target_arch = "wasm32"), feature = "cuda"))]
            {
                if mega_forward_done {
                    crate::llm_bench::record_cuda_mega_hit();
                } else if matches!(
                    std::env::var("QUALIA_LLM_CUDA_DECODE").ok().as_deref(),
                    Some("1") | Some("true") | Some("on")
                ) {
                    crate::llm_bench::record_cuda_mega_fallback();
                }
            }

            if resident_hit.is_none() && !resident_hidden_ok {
                if mega_forward_done {
                    crate::llm_bench::add_decode_forward_ns(0);
                    // Skip forward — mega-pass did it. If mega_full_token is
                    // Some, logits are also done. If None (sentinel), logits
                    // projection still runs below.
                } else {
                    // Decode-profiler: time the 32-layer forward (legacy path).
                    let t_fwd = std::time::Instant::now();
                    let _layers = engine.dispatch_transformer_forward(
                        idx,
                        &mut emb_buf[..emb_dim],
                        emb_dim,
                        scratch_a,
                        scratch_b,
                        token_idx,
                        TEST_TRANSFORMER_LAYER_CAP,
                    );
                    let _ = engine.apply_output_norm_inplace(idx, &mut emb_buf[..emb_dim], emb_dim);
                    crate::llm_bench::add_decode_forward_ns(t_fwd.elapsed().as_nanos() as u64);
                }
            }
            // If mega-pass produced a full token, use it directly and skip projection.
            // mega_full_token is None when the sentinel was returned (forward done,
            // logits still needed) — in that case the output projection runs normally.
            let mega_pass_done = mega_full_token.is_some();
            let mega_pass_tok = mega_full_token.unwrap_or(0) as usize;
            // Decode-profiler: time the output projection (argmax / top-k).
            let t_out = std::time::Instant::now();
            // W2: exact sampling — read back the FULL logit vector for this token and
            // run the CPU chain. Only when a non-greedy sampler is installed; on any
            // readback failure, `sampled` stays None and the greedy paths below run
            // (never a silent hang). The legacy forward above left the normed hidden
            // in `emb_buf`, so the projection input is correct.
            #[cfg(not(target_arch = "wasm32"))]
            let sampled: Option<(usize, f32)> = if let Some(s) = sampler.as_mut() {
                let vocab = idx
                    .logits_projection_info()
                    .map(|i| QTensorEngine::matmul_dims(i).1)
                    .unwrap_or(0);
                if vocab > 0 {
                    if sampler_logits.len() < vocab {
                        sampler_logits.resize(vocab, 0.0);
                    }
                    // Existing chunked projection; `written == vocab` iff it produced
                    // REAL logits (else it degraded to copying hidden → not sampleable,
                    // so we fall through to the greedy paths rather than sample garbage).
                    let written = engine.dispatch_output_logits_into(
                        idx,
                        &emb_buf[..emb_dim],
                        emb_dim,
                        &mut sampler_logits[..vocab],
                    );
                    if written == vocab {
                        // Neuro-symbolic: soft-boost graph answer tokens before sample.
                        let _ = crate::qualia_hybrid::apply_graph_logit_bias(
                            &prompt_owned,
                            &mut sampler_logits[..vocab],
                            &|s| {
                                let ids = tok.encode(s);
                                ids.first().copied()
                            },
                        );
                        // R9: DOMINO constrained decoding — if a masker is
                        // installed and active, apply the grammar constraint
                        // mask before sampling. Falls back to plain sample
                        // when no masker is active (backward compatible).
                        let tid = match crate::llm_bench::domino_sample(
                            s,
                            &mut sampler_logits[..vocab],
                            &ctx,
                        ) {
                            Some(constrained_tid) => constrained_tid,
                            None => s.sample(&mut sampler_logits[..vocab], &ctx),
                        };
                        Some((tid as usize, sampler_logits[tid as usize]))
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };
            #[cfg(target_arch = "wasm32")]
            let sampled: Option<(usize, f32)> = None;

            // A1a: GPU top-1 path (additive, default-on; non-sieve only in v1).
            // Returns the argmax token via the on-GPU block reduction; falls
            // through to the existing argmax path if disabled or on any failure — the
            // working path is never bypassed. Skipped when sampling produced a token.
            let topk_hit = if sampled.is_some() {
                None
            } else if resident_hit.is_some() {
                resident_hit
            } else if gpu_topk_enabled && sieve_mask.is_none() {
                #[cfg(feature = "gpu-runtime")]
                {
                    engine.dispatch_output_top1_chunked(idx, &emb_buf[..emb_dim], emb_dim)
                }
                #[cfg(not(feature = "gpu-runtime"))]
                {
                    None
                }
            } else {
                None
            };
            let out_sel = if mega_pass_done {
                crate::llm_bench::add_decode_output_ns(0);
                (mega_pass_tok, 0.0f32)
            } else if let Some(sel) = sampled {
                crate::llm_bench::record_sampled_token();
                sel
            } else if let Some(item) = topk_hit {
                crate::llm_bench::record_topk_hit();
                (item.best_token_id as usize, item.max_logit)
            } else if let Some(argmax) = engine.dispatch_output_argmax_chunked(
                idx,
                &emb_buf[..emb_dim],
                emb_dim,
                &mut scratch_a[..],
                TEST_VOCAB_CHUNK_CAP,
                sieve_mask,
            ) {
                crate::llm_bench::record_argmax_fallback();
                if argmax.max_logit > f32::NEG_INFINITY {
                    (argmax.best_token_id as usize, argmax.max_logit)
                } else {
                    sieve_failed = true;
                    (0usize, f32::NEG_INFINITY)
                }
            } else {
                let mut top1_v = f32::NEG_INFINITY;
                let mut top1_i = 0usize;
                let mut top2_v = f32::NEG_INFINITY;
                for (i, &v) in emb_buf[..emb_dim].iter().enumerate() {
                    if v > top1_v {
                        top2_v = top1_v;
                        top1_v = v;
                        top1_i = i;
                    } else if v > top2_v {
                        top2_v = v;
                    }
                }

                // Phase 6: Semantic Chunking Entropy Calculation
                let fast_entropy = -(top1_v - top2_v);
                if fast_entropy > chunk_policy.max_entropy_drop {
                    current_page_id += 1;

                    #[cfg(not(target_arch = "wasm32"))]
                    if let Some(ref mut wal) = thermal_wal_opt {
                        let record = crate::inference::thermal_wal::ThermalEvictionRecord {
                            timestamp_ms: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis() as u64,
                            page_id: (current_page_id - 1) as u32,
                            fast_entropy,
                            top1_v,
                            top2_v,
                            reserved: [0; 8],
                        };
                        wal.append(record);
                    }

                    if std::env::var("QUALIA_LLM_DEBUG_DECODE").is_ok() {
                        eprintln!(
                            "[NEW CHUNK] page_id={} entropy={:.3}",
                            current_page_id, fast_entropy
                        );
                        eprintln!(
                            "[THERMAL EVICT] Hard eviction logged for page_id={}",
                            current_page_id - 1
                        );
                    }
                }

                // Update KV provenance for this new token
                let token_idx = ctx.len() as u32;
                crate::tensor::kv_provenance::record_kv_provenance(
                    token_idx,
                    token_idx,
                    current_page_id,
                );

                (top1_i, top1_v)
            };
            crate::llm_bench::add_decode_output_ns(t_out.elapsed().as_nanos() as u64);
            out_sel
        } else {
            (0usize, 0.0)
        }
    } else {
        let wt = QTensor::new(vec![emb_dim, emb_dim], 0, true);
        let logits = embedding_fallback_logits(
            &engine,
            tensor_idx.as_ref(),
            lora_adapter.as_ref(),
            cur,
            emb_dim,
            &mut emb_buf[..],
            &wt,
        );
        logits
            .iter()
            .enumerate()
            .fold((0usize, f32::NEG_INFINITY), |(bi, bv), (i, &v)| {
                if v > bv {
                    (i, v)
                } else {
                    (bi, bv)
                }
            })
    };

    (top_i, top_v, current_page_id, sieve_failed)
}
