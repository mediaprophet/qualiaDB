// Phase 8 bifurcated-compute decode path for `LocalLlmAgent`.
//
// This is the LIVE LLM decode agent: it loads the GGUF/P64 model, tokenises,
// runs the autoregressive loop through `QTensorEngine`, and streams logit
// summaries from the LLM engine thread to the Webizen Sentinel over wait-free
// SPSC ring buffers. Moved verbatim from the `inference_agent.rs` monolith —
// no logic, control-flow, or signature changes.

#[cfg(not(target_arch = "wasm32"))]
use super::config::effective_inference_timeout_ms;
use super::config::{DECODE_TOKEN_BUDGET, TEST_TRANSFORMER_LAYER_CAP, TEST_VOCAB_CHUNK_CAP};
#[cfg(not(target_arch = "wasm32"))]
use super::decode_helpers::get_prefix_cache;
use super::decode_helpers::{
    apply_model_helper_stops, build_sieve, drain_tensor_context_inject, embedding_fallback_logits,
    try_accept_topology_draft, TopologyDraftStep,
};
use super::local_agent::LocalLlmAgent;
#[cfg(not(target_arch = "wasm32"))]
use super::sticky_infer;
use super::types::AgentBackend;
use crate::{q_hash, NQuin};

impl LocalLlmAgent {
    /// Phase 8: Bifurcated Compute — SPSC Wait-Free Intercept.
    ///
    /// On native targets: loads the GGUF model, tokenises the prompt, and runs an
    /// autoregressive decode loop via `QTensorEngine::dispatch_fused_transformer_block`.
    /// Logit summaries flow from the LLM engine thread to the Webizen Sentinel (this
    /// thread) over a wait-free SPSC ring. The Sentinel may inject `DenyRollback`
    /// for real governance signals; the old IEEE-754 `0x99` mantissa check was
    /// removed (it fired randomly ~1/256 tokens and corrupted the stream).
    ///
    /// On WASM / non-local backends: falls through to the original mock path.
    /// Run local inference, optionally streaming decoded text deltas to `on_token`.
    pub fn infer_local_model_streaming<F: FnMut(String) + Send + 'static>(
        &self,
        prompt: &str,
        graph_context: &str,
        on_token: Option<F>,
    ) -> (String, Vec<u64>, u32, Option<NQuin>) {
        self.infer_local_model_inner(prompt, graph_context, on_token)
    }

    pub(super) fn infer_local_model(
        &self,
        prompt: &str,
        graph_context: &str,
    ) -> (String, Vec<u64>, u32, Option<NQuin>) {
        self.infer_local_model_inner::<fn(String)>(prompt, graph_context, None)
    }

    #[cfg_attr(target_arch = "wasm32", allow(unused_variables, unused_mut))]
    fn infer_local_model_inner<F: FnMut(String) + Send + 'static>(
        &self,
        prompt: &str,
        graph_context: &str,
        mut on_token: Option<F>,
    ) -> (String, Vec<u64>, u32, Option<NQuin>) {
        let prov_hash = graph_context
            .bytes()
            .take(8)
            .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
        let use_sieve = self
            .use_sieve_output
            .load(std::sync::atomic::Ordering::Relaxed);
        let sieve_spec = if use_sieve {
            Some(*self.sieve_spec.lock().unwrap_or_else(|e| e.into_inner()))
        } else {
            None
        };
        let sieve_lex_path = if use_sieve {
            self.sieve_lex_path
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone()
        } else {
            None
        };

        // ── Native GPU path ─────────────────────────────────────────────────
        #[cfg(not(target_arch = "wasm32"))]
        {
            use crate::gguf_bridge::{QTensor, QTensorEngine};
            use crate::gguf_sharder::GgufTokenizer;
            use rtrb::RingBuffer;

            let model_path = match &self.backend {
                AgentBackend::Local { model_path, .. } => model_path.clone(),
                _ => {
                    return (
                        String::from("[no local model configured]"),
                        vec![prov_hash],
                        0,
                        None,
                    );
                }
            };
            let prompt_owned = prompt.to_string();

            // Multi-mode: portable | cuda | quant-graph (`QUALIA_INFERENCE_MODE`).
            let _mode = crate::inference_modes::bootstrap_inference_mode();

            // ── LoRA context detection (before thread spawn) ─────────────────
            // Detect the prompt domain and pre-load the matching LoRA adapter.
            // The pre-computed delta vectors are cloned into the inference thread
            // as fixed-size heap data — one allocation per infer call, not per token.
            #[allow(unused_variables)]
            let lora_active_adapter: Option<crate::lora::LoRAAdapter> = {
                let mut guard = self.lora_manager.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(ref mut mgr) = *guard {
                    let (ctx, conf, _switched) =
                        mgr.auto_switch(&prompt_owned, mgr.detector.confidence_threshold);
                    log::debug!("LoRA|context-detect|domain={ctx}|conf={conf:.3}");
                    mgr.active().cloned()
                } else {
                    None
                }
            };

            // Fixed-size types keep the hot-path allocation-free in the ring buffer.
            #[derive(Clone, Copy)]
            struct LogitSummary {
                _top_id: u32,
                anomaly: u8,
            }
            #[derive(Clone)]
            enum LlmMsg {
                Logit(LogitSummary),
                Eos,
            }
            #[derive(Clone)]
            enum SentMsg {
                DenyRollback,
            }

            // LogitStream: LLM engine → Webizen Sentinel
            let (mut lp, mut lc) = RingBuffer::<LlmMsg>::new(1024);
            // ControlStream: Webizen Sentinel → LLM engine
            let (mut cp, mut cc) = RingBuffer::<SentMsg>::new(16);

            let stream_pair = if on_token.is_some() {
                Some(std::sync::mpsc::sync_channel::<String>(512))
            } else {
                None
            };
            let stream_tx_thread = stream_pair.as_ref().map(|(tx, _)| tx.clone());

            // Move the (optional) LoRA adapter into the inference thread.
            let lora_for_thread = lora_active_adapter;

            // Sticky pool thread owns the engine (thread_local); caller runs Sentinel.
            let (done_tx, done_rx) =
                std::sync::mpsc::sync_channel::<(String, u32, Option<NQuin>, bool)>(1);

            // ── LLM engine job (sticky 1-thread pool) ────────────────────────
            sticky_infer::pool().spawn(move || {
                let result = sticky_infer::with_engine(
                    model_path.as_str(),
                    |engine| {
                        // Load only on cache miss / path change.
                        if let Some(mmap) =
                            crate::resident_model::resident_mmap_for_path(model_path.as_str())
                        {
                            let is_p64 = crate::p64_weight::has_p64_magic(&mmap[..]);
                            let adopted = if is_p64 {
                                engine.adopt_resident_p64_mmap(mmap).is_ok()
                            } else {
                                engine.adopt_resident_mmap(mmap).is_ok()
                            };
                            if !adopted {
                                engine.load_model(&model_path);
                            }
                        } else {
                            engine.load_model(&model_path);
                        }
                    },
                    |engine: &mut QTensorEngine| {
                // Initialize Tokio runtime for the sticky thread (once per job; cheap if already warm).
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap_or_else(|e| {
                        panic!("Failed to create Tokio runtime for LLM thread: {}", e)
                    });
                let _rt_guard = rt.enter();

                // A0 phase timing (D17/D22): once-per-phase, off the per-token hot path.
                let t_phase = std::time::Instant::now();

                // Thermal-eviction WAL is a native-only file mmap (`memmap2`); on
                // wasm there is no mmap'd-file WAL, so the telemetry is simply absent.
                #[cfg(not(target_arch = "wasm32"))]
                let mut thermal_wal_opt = {
                    let wal_path = std::env::var("QUALIA_DATA_DIR")
                        .map(|p| std::path::PathBuf::from(p).join("thermal_eviction.wal"))
                        .unwrap_or_else(|_| std::env::temp_dir().join("thermal_eviction.wal"));
                    crate::inference::thermal_wal::ThermalWal::open(&wal_path, 1024).ok()
                };

                let lora_adapter = lora_for_thread;
                let sieve_spec = sieve_spec;
                let sieve_lex_path = sieve_lex_path;

                // Tokenizer + tensor index come from the matching on-disk
                // format. P64 carries a Q42T tokenizer section and a manifest;
                // GGUF carries both in its own metadata.
                let is_p64_mmap = engine
                    .gguf_mmap
                    .as_ref()
                    .map(|m| crate::p64_weight::has_p64_magic(&m[..]))
                    .unwrap_or(false);
                // Prefer engine-cached index from adopt (zero re-CRC); else parse once.
                let p64_index = engine.p64_index.clone().or_else(|| {
                    if is_p64_mmap {
                        engine
                            .gguf_mmap
                            .as_ref()
                            .and_then(|m| crate::p64_weight::P64TensorIndex::from_p64(m).ok())
                    } else {
                        None
                    }
                });
                let mut tok = if let (Some(qi), Some(m)) =
                    (p64_index.as_ref(), engine.gguf_mmap.as_ref())
                {
                    GgufTokenizer::from_p64_section(qi.tokenizer_bytes(m)).unwrap_or_default()
                } else {
                    engine
                        .gguf_mmap
                        .as_ref()
                        .map(|m| GgufTokenizer::from_gguf(m))
                        .unwrap_or_default()
                };
                // Sibling canonical `.q42` metadata (convert-time stop set / chat metadata).
                apply_model_helper_stops(&model_path, &mut tok);

                let tensor_idx = engine.tensor_index_cache.clone().or_else(|| {
                    if let Some(qi) = p64_index {
                        Some(qi.to_gguf_index())
                    } else {
                        engine
                            .gguf_mmap
                            .as_ref()
                            .map(|m| crate::gguf_sharder::GgufTensorIndex::from_gguf(m))
                    }
                });

                let mut ctx = tok.encode_chat_prompt(&prompt_owned);
                // Keep `eos` for draft/topology APIs that still take a single id; decode
                // termination uses the full stop set (eos + chat end-of-turn specials).
                let eos = tok.eos_token_id;
                let vlen = tok.vocab_len().max(1);

                // Use the real embedding dimension if the tensor was found; fall back to 4096.
                let emb_dim = tensor_idx
                    .as_ref()
                    .map(|idx| idx.emb_dim())
                    .filter(|&d| d > 0)
                    .unwrap_or(4096);

                // Stack buffers — zero-heap path (512MB floor safe).
                use crate::gguf_bridge::{PREFILL_CHUNK_SIZE, PREFILL_CHUNK_STACK_FLOATS};

                const MAX_EMB_DIM: usize = 8192;
                const MAX_FFN_DIM: usize = 10240;
                let mut emb_buf = [0f32; MAX_EMB_DIM];
                let mut scratch_a = [0f32; MAX_FFN_DIM];
                let mut scratch_b = [0f32; MAX_FFN_DIM];
                let mut prefill_chunk = [0f32; PREFILL_CHUNK_STACK_FLOATS];
                let emb_dim = emb_dim.min(MAX_EMB_DIM);
                let mut prefix_cached = false;
                if prov_hash != 0 {
                    if let Ok(cache) = get_prefix_cache().lock() {
                        if let Some(cached_kv) = cache.get(&prov_hash) {
                            engine.set_kv_cache_cpu(cached_kv);
                            prefix_cached = true;
                        }
                    }
                }

                if !prefix_cached {
                    engine.reset_kv_cache();
                }

                // Phase boundary: load (mmap/adopt + tokenizer + tensor index + setup) done.
                crate::llm_bench::record_load_ns(t_phase.elapsed().as_nanos() as u64);
                let t_prefill = std::time::Instant::now();

                // Chunked prefill: populate KV for prompt tokens [0, prompt_len-1).
                let prompt_len = ctx.len();
                crate::tensor::kv_provenance::rebuild_prompt_provenance(
                    prompt_len as u32,
                    crate::tensor::resident_substrate::global_resident_substrate().node_count(),
                    0,
                );
                let draft_mapper = crate::topology_draft::TopologyDraftMapper::new(&tok);
                if !prefix_cached {
                    if prompt_len > 1 {
                        if let Some(idx) = tensor_idx.as_ref() {
                            let prefill_tokens = prompt_len - 1;
                            let chunk_cap = (PREFILL_CHUNK_STACK_FLOATS / emb_dim)
                                .min(PREFILL_CHUNK_SIZE)
                                .max(1);
                            let mut pos = 0usize;
                            while pos < prefill_tokens {
                                let n = (prefill_tokens - pos).min(chunk_cap);
                                let batch_elems = n * emb_dim;
                                {
                                    let mmap = match engine.gguf_mmap.as_deref() {
                                        Some(m) => m,
                                        None => break,
                                    };
                                    for t in 0..n {
                                        let _ = idx.dequantize_token_embedding_into(
                                            mmap,
                                            ctx[pos + t],
                                            &mut prefill_chunk[t * emb_dim..(t + 1) * emb_dim],
                                        );
                                    }
                                }
                                if !engine.dispatch_prefill_chunk(
                                    idx,
                                    &mut prefill_chunk[..batch_elems],
                                    emb_dim,
                                    n as u32,
                                    pos as u32,
                                    &mut scratch_a,
                                    &mut scratch_b,
                                    TEST_TRANSFORMER_LAYER_CAP,
                                ) {
                                    crate::gguf_bridge::wlog(&format!(
                                        "[llm] PREFILL chunk FAILED pos={pos} n={n}"
                                    ));
                                }
                                pos += n;
                            }
                        }
                    }

                    if prov_hash != 0 {
                        if let Some(cpu_kv) = engine.get_kv_cache_cpu() {
                            if let Ok(mut cache) = get_prefix_cache().lock() {
                                cache.insert(prov_hash, cpu_kv.into());
                            }
                        }
                    }
                }

                // Phase boundary: prefill done. Decode phase begins below.
                crate::llm_bench::record_prefill(
                    t_prefill.elapsed().as_nanos() as u64,
                    prompt_len.saturating_sub(1) as u64,
                );

                let mut out_ids: Vec<u32> = Vec::new();
                let mut streamed_len = 0usize;
                let mut sieve = if use_sieve {
                    build_sieve(&tok, sieve_spec.as_ref(), sieve_lex_path.as_deref())
                } else {
                    None
                };
                let mut semantic_quin: Option<NQuin> = None;
                let mut sieve_failed = false;
                let gen_budget = if sieve.is_some() {
                    3usize
                } else {
                    // Benchmark override (A0): a fixed decode count for stable tok/s; 0 = default.
                    let ov = crate::llm_bench::decode_budget_override();
                    if ov > 0 {
                        ov as usize
                    } else {
                        DECODE_TOKEN_BUDGET as usize
                    }
                };

                #[cfg(not(target_arch = "wasm32"))]
                crate::compute_universe::start_tensor_search_producer();
                crate::compute_universe::publish_query_tensor(
                    crate::tensor::Tensor10D::default(),
                    0,
                );
                // Qualia-unique hybrid: graph route mask + 10D query + deontic obligation.
                // Must run *after* the default query publish so it is not wiped.
                crate::qualia_hybrid::prepare_hybrid_decode(&prompt_owned);

                let t_decode = std::time::Instant::now();
                // A1a: GPU top-1 decode path toggle (default-on; QUALIA_LLM_GPU_TOPK / set_gpu_topk).
                let gpu_topk_enabled = crate::llm_bench::gpu_topk_enabled();
                // W2: exact CPU sampler. `None` ⇒ greedy argmax (pre-W2 byte-identical path). When
                // active, decode uses the legacy forward (leaves the normed hidden in `emb_buf`),
                // reads back the FULL logit vector, and runs the penalty/temp/top-k/top-p chain.
                #[cfg(not(target_arch = "wasm32"))]
                let mut sampler =
                    crate::llm_bench::sampler_config().map(crate::sampler::SamplerState::new);
                #[cfg(target_arch = "wasm32")]
                let mut sampler: Option<crate::sampler::SamplerState> = None;
                let mut sampler_logits: Vec<f32> = Vec::new();
                // Decode-profiler (gated): one-shot empty submit→wait baseline on the SAME device, so
                // the bench can separate per-token fence latency from real kernel compute time.
                #[cfg(not(target_arch = "wasm32"))]
                if std::env::var("QUALIA_LLM_PROFILE_DECODE").is_ok() {
                    let n = 64u32;
                    crate::llm_bench::record_empty_rt(
                        engine.bench_empty_submit_roundtrip(n),
                        n as u64,
                    );
                }

                // Phase 6: Initialize Semantic Chunking State
                let mut current_page_id = 100u64; // Starting mock page ID
                let chunk_policy = crate::q42::q42_kvp::Q42ChunkPolicy {
                    max_tokens: 128,
                    semantic_shift_threshold: 0.0,
                    discourse_boundary_weight: 0.0,
                    attention_phase_weight: 0.0,
                    max_entropy_drop: -2.0, // A threshold that will trigger when top1 and top2 are close
                    thermal_pressure_bias: 0.0,
                    reserved: [0; 40],
                };

                // QUALIA_GRAPH_FORCE=1: emit grounded repair tokens without model decode.
                let mut graph_force_emitted = false;
                #[cfg(not(target_arch = "wasm32"))]
                if let Some(forced) = crate::qualia_hybrid::force_fact_tokens(&prompt_owned, &|s| {
                    tok.encode(s)
                }) {
                    for &tid in &forced {
                        let next = tid % vlen.max(1);
                        out_ids.push(next);
                        ctx.push(next);
                        if let Some(ref tx) = stream_tx_thread {
                            let full = tok.decode(&out_ids);
                            if full.len() > streamed_len {
                                let delta = full[streamed_len..].to_string();
                                streamed_len = full.len();
                                let _ = tx.send(delta);
                            }
                        }
                        let fixed = crate::llm_bench::decode_budget_fixed_tokens();
                        if out_ids.len() >= gen_budget
                            || (!fixed && tok.is_stop_token(next))
                        {
                            break;
                        }
                    }
                    graph_force_emitted = true;
                }

                if !graph_force_emitted {
                for step in 0..gen_budget {
                    crate::gpu_context::record_llm_decode_step();

                    // Codex P0 — cooperative deadline: break BEFORE the wall-clock timeout instead of
                    // the old post-hoc check in infer() that let a no-EOS run continue for minutes.
                    // t_decode starts at decode entry (post-prefill); INFERENCE_TIMEOUT_MS bounds the
                    // generation phase so the call never appears frozen.
                    if t_decode.elapsed().as_millis() as u64 >= effective_inference_timeout_ms() {
                        break;
                    }

                    // W7 — periodic GPU thermal check during sustained decode: recommends a TDP cap,
                    // and (when the auto-cap user option is on + a real NVML governor is present)
                    // applies it under sustained Critical, restoring on cool-down. Cheap NVML read
                    // every 32 tokens; no-op without the `nvml` feature or an NVIDIA card.
                    #[cfg(not(target_arch = "wasm32"))]
                    if step > 0 && step % 32 == 0 {
                        crate::inference::thermal_telemetry::thermal_tick();
                    }

                    // W6a — prompt-lookup / graph fact speculative decode (default OFF, exact-output).
                    // Prefer quant-graph fact draft when mode=quant-graph; else n-gram prompt-lookup.
                    // FastVerify: skip mid-decode fact draft (post-turn heal only) for Ollama-like speed.
                    // Verify drafts in ONE batched forward. Bit-identical to greedy when accepted.
                    #[cfg(not(target_arch = "wasm32"))]
                    if (crate::llm_bench::spec_decode_enabled()
                        || (crate::inference_modes::quant_graph_grounding_enabled()
                            && crate::inference_modes::sentinel_mid_decode_enabled()))
                        && sieve.is_none()
                        && sampler.is_none()
                        && TEST_TRANSFORMER_LAYER_CAP == 0
                    {
                        if let Some(idx) = tensor_idx.as_ref() {
                            let cur = *ctx.last().unwrap_or(&tok.bos_token_id);
                            let draft = crate::qualia_hybrid::propose_best_draft(
                                &prompt_owned,
                                &ctx,
                                &|s| tok.encode(s),
                            );
                            if draft.len > 0 {
                                // inputs = [cur, d0..d_{m-1}] at positions [pos, pos+m], pos = ctx.len()-1.
                                let mut inputs = Vec::with_capacity(draft.len + 1);
                                inputs.push(cur);
                                inputs.extend_from_slice(draft.as_slice());
                                let pos = ctx.len().saturating_sub(1) as u32;
                                let mut amax: Vec<u32> = Vec::new();
                                let mut alog: Vec<f32> = Vec::new();
                                if engine
                                    .verify_draft_batch(idx, &inputs, pos, &mut amax, &mut alog)
                                    .is_some()
                                    && amax.len() == inputs.len()
                                {
                                    // Accept the longest prefix where argmax[i] == draft[i]; then emit
                                    // d0..d_{k-1} (accepted) + argmax[k] (correction/bonus) = k+1 tokens.
                                    let m = draft.len;
                                    let mut k = 0usize;
                                    while k < m && amax[k] == draft.tokens[k] {
                                        k += 1;
                                    }
                                    crate::llm_bench::record_spec_step(m as u64, k as u64);
                                    let mut stop = false;
                                    for i in 0..=k {
                                        let (tokn, _logv) = if i < k {
                                            (draft.tokens[i], alog[i])
                                        } else {
                                            (amax[k], alog[k])
                                        };
                                        // anomaly 0x01 = normal. Do not use random mantissa bytes.
                                        let _ = lp.push(LlmMsg::Logit(LogitSummary {
                                            _top_id: tokn,
                                            anomaly: 0x01u8,
                                        }));
                                        let next = tokn % vlen;
                                        out_ids.push(next);
                                        ctx.push(next);
                                        if let Some(ref tx) = stream_tx_thread {
                                            let full = tok.decode(&out_ids);
                                            if full.len() > streamed_len {
                                                let delta = full[streamed_len..].to_string();
                                                streamed_len = full.len();
                                                let _ = tx.send(delta);
                                            }
                                        }
                                        let fixed = crate::llm_bench::decode_budget_fixed_tokens();
                                        if out_ids.len() >= gen_budget
                                            || (!fixed && tok.is_stop_token(next))
                                        {
                                            stop = true;
                                            break;
                                        }
                                    }
                                    if stop {
                                        break;
                                    }
                                    continue; // skip the normal single-token path this step
                                }
                            }
                        }
                    }

                    let draft_step = try_accept_topology_draft(
                        engine,
                        tensor_idx.as_ref(),
                        &draft_mapper,
                        &mut ctx,
                        emb_dim,
                        &mut emb_buf,
                        &mut scratch_a,
                        &mut scratch_b,
                        &mut out_ids,
                        &mut sieve,
                        prov_hash,
                        eos,
                        &tok,
                        &mut streamed_len,
                        stream_tx_thread.as_ref(),
                        None,
                    );
                    match draft_step {
                        TopologyDraftStep::AcceptedFull => continue,
                        TopologyDraftStep::Stop {
                            sieve_failed: sf,
                            semantic_quin: sq,
                        } => {
                            sieve_failed = sf;
                            semantic_quin = sq;
                            break;
                        }
                        _ => {}
                    }

                    drain_tensor_context_inject();
                    let _attention_mask = crate::compute_universe::attention_route_mask();
                    // FastVerify: skip ControlStream — no mid-decode DenyRollback tax.
                    let mut rollback = if crate::inference_modes::sentinel_mid_decode_enabled() {
                        cc.pop().is_ok()
                    } else {
                        false
                    };
                    if matches!(draft_step, TopologyDraftStep::Denied) {
                        rollback = true;
                    }

                    let cur = *ctx.last().unwrap_or(&tok.bos_token_id);
                    crate::compute_universe::publish_decode_hint(cur, step as u32);

                    // 1) Embedding lookup → hidden state (stack dequant).
                    let hidden_ok = tensor_idx
                        .as_ref()
                        .and_then(|idx| {
                            engine.gguf_mmap.as_deref().map(|m| {
                                idx.dequantize_token_embedding_into(m, cur, &mut emb_buf[..emb_dim])
                            })
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
                            #[cfg(not(target_arch = "wasm32"))]
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
                                        crate::llm_bench::add_decode_forward_ns(
                                            t_res.elapsed().as_nanos() as u64,
                                        );
                                        crate::llm_bench::record_resident_hit();
                                        (None, true)
                                    } else {
                                        crate::llm_bench::record_resident_fallback();
                                        (None, false)
                                    }
                                } else if gpu_topk_enabled {
                                    let hit = engine.dispatch_token_forward_resident(
                                        idx,
                                        &emb_buf[..emb_dim],
                                        token_idx,
                                    );
                                    if hit.is_some() {
                                        crate::llm_bench::add_decode_forward_ns(
                                            t_res.elapsed().as_nanos() as u64,
                                        );
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
                            #[cfg(target_arch = "wasm32")]
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
                                engine.try_cuda_mega_pass_decode(
                                    idx,
                                    &mut emb_buf[..emb_dim],
                                    emb_dim,
                                    token_idx,
                                )
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
                                        &mut scratch_a,
                                        &mut scratch_b,
                                        token_idx,
                                        TEST_TRANSFORMER_LAYER_CAP,
                                    );
                                    let _ = engine.apply_output_norm_inplace(
                                        idx,
                                        &mut emb_buf[..emb_dim],
                                        emb_dim,
                                    );
                                    crate::llm_bench::add_decode_forward_ns(
                                        t_fwd.elapsed().as_nanos() as u64
                                    );
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
                                engine.dispatch_output_top1_chunked(
                                    idx,
                                    &emb_buf[..emb_dim],
                                    emb_dim,
                                )
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
                                        let record =
                                            crate::inference::thermal_wal::ThermalEvictionRecord {
                                                timestamp_ms: std::time::SystemTime::now()
                                                    .duration_since(std::time::UNIX_EPOCH)
                                                    .unwrap_or_default()
                                                    .as_millis()
                                                    as u64,
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
                            crate::llm_bench::add_decode_output_ns(
                                t_out.elapsed().as_nanos() as u64
                            );
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
                        logits.iter().enumerate().fold(
                            (0usize, f32::NEG_INFINITY),
                            |(bi, bv), (i, &v)| if v > bv { (i, v) } else { (bi, bv) },
                        )
                    };

                    // #48 diagnostic: reveal eos vs argmax for the first step (gated, native).
                    if step == 0 && std::env::var("QUALIA_LLM_DEBUG_DECODE").is_ok() {
                        eprintln!(
                            "[decode-dbg] step0 eos={} vlen={} prompt_last={} top_i={} top_v={} decoded={:?}",
                            eos,
                            vlen,
                            cur,
                            top_i,
                            top_v,
                            tok.decode(&[top_i as u32])
                        );
                        if let Some(idx) = tensor_idx.as_ref() {
                            if let Some(top5) = engine.dispatch_output_topk_chunked(
                                idx,
                                &emb_buf[..emb_dim],
                                emb_dim,
                                5,
                            ) {
                                for it in &top5 {
                                    eprintln!(
                                        "[top5] id={} logit={:.3} dec={:?}",
                                        it.token_id,
                                        it.logit,
                                        tok.decode(&[it.token_id])
                                    );
                                }
                            }
                        }
                    }

                    // FastVerify: skip per-token Logit ring push (only Eos ends the turn).
                    if crate::inference_modes::sentinel_mid_decode_enabled() {
                        // anomaly 0x01 = normal. Removed IEEE mantissa 0x99 check (random ~1/256 fire).
                        let _ = lp.push(LlmMsg::Logit(LogitSummary {
                            _top_id: top_i as u32,
                            anomaly: 0x01u8,
                        }));
                    }

                    if sieve_failed {
                        break;
                    }

                    // DenyRollback must never inject sequential garbage (cur+1).
                    if rollback {
                        log::warn!(
                            "LLM_DECODE|sentinel-deny-rollback|keeping argmax token {} (no cur+1)",
                            top_i
                        );
                    }
                    let next = (top_i as u32) % vlen;

                    if let Some(ref mut s) = sieve {
                        match s.apply_token(next) {
                            Ok(()) => {
                                out_ids.push(next);
                                ctx.push(next);
                                if s.is_complete() {
                                    semantic_quin = Some(s.assemble_quin(prov_hash));
                                    break;
                                }
                            }
                            Err(_) => {
                                sieve_failed = true;
                                break;
                            }
                        }
                    } else {
                        out_ids.push(next);
                        ctx.push(next);
                        if let Some(ref tx) = stream_tx_thread {
                            let full = tok.decode(&out_ids);
                            if full.len() > streamed_len {
                                let delta = full[streamed_len..].to_string();
                                streamed_len = full.len();
                                let _ = tx.send(delta);
                            }
                        }
                        // Stop on eos AND chat end-of-turn — unless fixed-token bench override.
                        let fixed = crate::llm_bench::decode_budget_fixed_tokens();
                        if out_ids.len() >= gen_budget
                            || (!fixed && tok.is_stop_token(next))
                        {
                            break;
                        }
                    }
                }
                } // end else: normal decode (not QUALIA_GRAPH_FORCE)

                // Phase boundary: decode loop complete.
                crate::llm_bench::record_decode(
                    t_decode.elapsed().as_nanos() as u64,
                    out_ids.len() as u64,
                );

                let _ = lp.push(LlmMsg::Eos);
                let text = if semantic_quin.is_some() {
                    String::new()
                } else if sieve_failed {
                    String::from("[sieve-misaligned]")
                } else {
                    let raw = tok.decode(&out_ids);
                    // Post-turn path (FastVerify / QuantGraph): generate full draft at
                    // full speed, then graph/CML self-heal + optional HTML surface.
                    if crate::inference_modes::post_turn_verify_enabled() {
                        let v = crate::post_turn_verify::verify_and_heal_turn(&prompt_owned, &raw);
                        if crate::post_turn_verify::return_html_as_text() {
                            v.display_html
                        } else {
                            v.final_text
                        }
                    } else {
                        crate::quant_graph_grounding::maybe_ground_generation(&prompt_owned, &raw)
                            .text
                    }
                };
                (text, out_ids.len() as u32, semantic_quin, sieve_failed)
                    }, // sticky_infer::with_engine f
                ); // sticky_infer::with_engine
                let _ = done_tx.send(result);
            }); // sticky pool spawn

            // ── Webizen Sentinel (calling thread) ────────────────────────────
            // FastVerify: still drain stream + wait for Eos, but ignore anomaly mid-decode
            // (no DenyRollback) so generation is uninterrupted like Ollama.
            let mid_sentinel = crate::inference_modes::sentinel_mid_decode_enabled();
            let mut drain_tokens = || {
                if let (Some((_, ref rx)), Some(cb)) = (&stream_pair, on_token.as_mut()) {
                    while let Ok(delta) = rx.try_recv() {
                        cb(delta);
                    }
                }
            };

            loop {
                drain_tokens();
                match lc.pop() {
                    Ok(LlmMsg::Eos) => break,
                    Ok(LlmMsg::Logit(s)) => {
                        if mid_sentinel && s.anomaly == 0x99 {
                            let _ = cp.push(SentMsg::DenyRollback);
                        }
                    }
                    Err(_) => std::hint::spin_loop(),
                }
            }

            drain_tokens();

            let (text, tokens, semantic_quin, sieve_failed) = done_rx
                .recv()
                .unwrap_or_else(|_| (String::new(), 0, None, false));
            let mut prov = vec![prov_hash];
            if prov_hash == 0 {
                prov.push(q_hash("qualia:grounded"));
            }
            if let Some(q) = semantic_quin {
                prov.push(q.subject);
                prov.push(q.predicate);
                prov.push(q.object);
            }
            if sieve_failed && semantic_quin.is_none() {
                return (text, prov, tokens, None);
            }
            return (text, prov, tokens, semantic_quin);
        }

        // ── Native GPU path ─────────────────────────────────────────────────
        #[cfg(target_arch = "wasm32")]
        {
            use crate::gguf_bridge::QTensor;
            use crate::gguf_sharder::GgufTokenizer;

            let model_path = match &self.backend {
                AgentBackend::Local { model_path, .. } => model_path.clone(),
                _ => {
                    return (
                        String::from("[no local model configured]"),
                        vec![prov_hash],
                        0,
                        None,
                    );
                }
            };

            // ── WASM Extension Bus Offloading ────────────────────────────────
            if crate::extension_bus::wasm_bus::is_connected() {
                if let Some(cb) = on_token {
                    let _ = crate::extension_bus::wasm_bus::send_intent(prompt, graph_context, cb);
                } else {
                    let _ =
                        crate::extension_bus::wasm_bus::send_intent(prompt, graph_context, |_| {});
                }
                return (String::new(), vec![prov_hash], 0, None);
            }
            let prompt_owned = prompt.to_string();

            // Multi-mode: portable | cuda | quant-graph (`QUALIA_INFERENCE_MODE`).
            let _mode = crate::inference_modes::bootstrap_inference_mode();

            // ── LoRA context detection (before thread spawn) ─────────────────
            // Detect the prompt domain and pre-load the matching LoRA adapter.
            // The pre-computed delta vectors are cloned into the inference thread
            // as fixed-size heap data — one allocation per infer call, not per token.
            #[allow(unused_variables)]
            let lora_active_adapter: Option<crate::lora::LoRAAdapter> = {
                let mut guard = self.lora_manager.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(ref mut mgr) = *guard {
                    let (ctx, conf, _switched) =
                        mgr.auto_switch(&prompt_owned, mgr.detector.confidence_threshold);
                    log::debug!("LoRA|context-detect|domain={ctx}|conf={conf:.3}");
                    mgr.active().cloned()
                } else {
                    None
                }
            };

            // Move the (optional) LoRA adapter into the inference thread.
            let lora_for_thread = lora_active_adapter;

            // ── LLM engine synchronous execution ─────────────────────────────
            let (text, tokens, semantic_quin, sieve_failed) = {
                let mut rollback = false;

                let lora_adapter = lora_for_thread;
                let sieve_spec = sieve_spec;
                let sieve_lex_path = sieve_lex_path;
                // Build the GPU engine and memory-map the GGUF inside the thread to
                // avoid Send constraints on the DirectML / wgpu device handles.
                let mut engine = {
                    let engine_guard =
                        crate::gguf_bridge::WASM_ENGINE_INSTANCE.with(|g| g.borrow_mut().take());
                    engine_guard.expect(
                        "WASM WebGPU engine not initialized. Call initialize_webgpu_engine first.",
                    )
                };

                let is_p64_mmap = engine
                    .gguf_mmap
                    .as_ref()
                    .map(|m| crate::p64_weight::has_p64_magic(&m[..]))
                    .unwrap_or(false);
                let p64_index = if is_p64_mmap {
                    engine
                        .gguf_mmap
                        .as_ref()
                        .and_then(|m| crate::p64_weight::P64TensorIndex::from_p64(m).ok())
                } else {
                    None
                };
                let mut tok =
                    if let (Some(qi), Some(m)) = (p64_index.as_ref(), engine.gguf_mmap.as_ref()) {
                        GgufTokenizer::from_p64_section(qi.tokenizer_bytes(m)).unwrap_or_default()
                    } else {
                        engine
                            .gguf_mmap
                            .as_ref()
                            .map(|m| GgufTokenizer::from_gguf(m))
                            .unwrap_or_default()
                    };
                apply_model_helper_stops(&model_path, &mut tok);

                let tensor_idx = if let Some(qi) = p64_index {
                    Some(qi.to_gguf_index())
                } else {
                    engine
                        .gguf_mmap
                        .as_ref()
                        .map(|m| crate::gguf_sharder::GgufTensorIndex::from_gguf(m))
                };

                let mut ctx = tok.encode_chat_prompt(&prompt_owned);
                let eos = tok.eos_token_id;
                let vlen = tok.vocab_len().max(1);

                // Use the real embedding dimension if the tensor was found; fall back to 4096.
                let emb_dim = tensor_idx
                    .as_ref()
                    .map(|idx| idx.emb_dim())
                    .filter(|&d| d > 0)
                    .unwrap_or(4096);

                // Stack buffers — zero-heap path (512MB floor safe).
                use crate::gguf_bridge::{PREFILL_CHUNK_SIZE, PREFILL_CHUNK_STACK_FLOATS};

                const MAX_EMB_DIM: usize = 8192;
                const MAX_FFN_DIM: usize = 10240;
                let mut emb_buf = [0f32; MAX_EMB_DIM];
                let mut scratch_a = [0f32; MAX_FFN_DIM];
                let mut scratch_b = [0f32; MAX_FFN_DIM];
                let mut prefill_chunk = [0f32; PREFILL_CHUNK_STACK_FLOATS];
                let emb_dim = emb_dim.min(MAX_EMB_DIM);
                engine.reset_kv_cache();

                // Chunked prefill: populate KV for prompt tokens [0, prompt_len-1).
                let prompt_len = ctx.len();
                crate::tensor::kv_provenance::rebuild_prompt_provenance(
                    prompt_len as u32,
                    crate::tensor::resident_substrate::global_resident_substrate().node_count(),
                    0,
                );
                let draft_mapper = crate::topology_draft::TopologyDraftMapper::new(&tok);
                if prompt_len > 1 {
                    if let Some(idx) = tensor_idx.as_ref() {
                        let prefill_tokens = prompt_len - 1;
                        let chunk_cap = (PREFILL_CHUNK_STACK_FLOATS / emb_dim)
                            .min(PREFILL_CHUNK_SIZE)
                            .max(1);
                        let mut pos = 0usize;
                        while pos < prefill_tokens {
                            let n = (prefill_tokens - pos).min(chunk_cap);
                            let batch_elems = n * emb_dim;
                            {
                                let mmap = match engine.gguf_mmap.as_deref() {
                                    Some(m) => m,
                                    None => break,
                                };
                                for t in 0..n {
                                    let _ = idx.dequantize_token_embedding_into(
                                        mmap,
                                        ctx[pos + t],
                                        &mut prefill_chunk[t * emb_dim..(t + 1) * emb_dim],
                                    );
                                }
                            }
                            if !engine.dispatch_prefill_chunk(
                                idx,
                                &mut prefill_chunk[..batch_elems],
                                emb_dim,
                                n as u32,
                                pos as u32,
                                &mut scratch_a,
                                &mut scratch_b,
                                TEST_TRANSFORMER_LAYER_CAP,
                            ) {
                                crate::gguf_bridge::wlog(&format!(
                                    "[llm] PREFILL chunk FAILED pos={pos} n={n}"
                                ));
                            }
                            pos += n;
                        }
                    }
                }

                let mut out_ids: Vec<u32> = Vec::new();
                let mut streamed_len = 0usize;
                let mut sieve = if use_sieve {
                    build_sieve(&tok, sieve_spec.as_ref(), sieve_lex_path.as_deref())
                } else {
                    None
                };
                let mut semantic_quin: Option<NQuin> = None;
                let mut sieve_failed = false;
                let gen_budget = if sieve.is_some() {
                    3usize
                } else {
                    DECODE_TOKEN_BUDGET as usize
                };

                crate::compute_universe::start_tensor_search_producer();
                crate::compute_universe::publish_query_tensor(
                    crate::tensor::Tensor10D::default(),
                    0,
                );
                crate::qualia_hybrid::prepare_hybrid_decode(&prompt_owned);

                for step in 0..gen_budget {
                    crate::gpu_context::record_llm_decode_step();

                    let on_token_sink = on_token.as_mut().map(|cb| cb as &mut dyn FnMut(String));
                    let draft_step = try_accept_topology_draft(
                        &mut engine,
                        tensor_idx.as_ref(),
                        &draft_mapper,
                        &mut ctx,
                        emb_dim,
                        &mut emb_buf,
                        &mut scratch_a,
                        &mut scratch_b,
                        &mut out_ids,
                        &mut sieve,
                        prov_hash,
                        eos,
                        &tok,
                        &mut streamed_len,
                        None,
                        on_token_sink,
                    );
                    match draft_step {
                        TopologyDraftStep::AcceptedFull => continue,
                        TopologyDraftStep::Stop {
                            sieve_failed: sf,
                            semantic_quin: sq,
                        } => {
                            sieve_failed = sf;
                            semantic_quin = sq;
                            break;
                        }
                        _ => {}
                    }

                    drain_tensor_context_inject();
                    let _attention_mask = crate::compute_universe::attention_route_mask();

                    let rollback_val = rollback;
                    rollback = false;
                    let mut rollback = rollback_val;
                    if matches!(draft_step, TopologyDraftStep::Denied) {
                        rollback = true;
                    }

                    let cur = *ctx.last().unwrap_or(&tok.bos_token_id);
                    crate::compute_universe::publish_decode_hint(cur, step as u32);

                    // 1) Embedding lookup → hidden state (stack dequant).
                    let hidden_ok = tensor_idx
                        .as_ref()
                        .and_then(|idx| {
                            engine.gguf_mmap.as_deref().map(|m| {
                                idx.dequantize_token_embedding_into(m, cur, &mut emb_buf[..emb_dim])
                            })
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
                            // CUDA mega-pass: attempt single-fence all-layer forward.
                            // Returns Some(u32::MAX) sentinel when forward is done but logits
                            // still needed (hidden state read back into emb_buf).
                            #[cfg(all(not(target_arch = "wasm32"), feature = "cuda"))]
                            let mega_token: Option<u32> = engine.try_cuda_mega_pass_decode(
                                idx,
                                &mut emb_buf[..emb_dim],
                                emb_dim,
                                token_idx,
                            );
                            #[cfg(any(target_arch = "wasm32", not(feature = "cuda")))]
                            let mega_token: Option<u32> = None;

                            if let Some(mt) = mega_token.filter(|&t| t != u32::MAX) {
                                (mt as usize, 0.0f32)
                            } else if mega_token.is_some() {
                                // Sentinel: forward done, logits needed.
                                let sieve_mask = sieve.as_ref().map(|s| s.current_mask());
                                if let Some(argmax) = engine.dispatch_output_argmax_chunked(
                                    idx,
                                    &emb_buf[..emb_dim],
                                    emb_dim,
                                    &mut scratch_a[..],
                                    TEST_VOCAB_CHUNK_CAP,
                                    sieve_mask,
                                ) {
                                    if argmax.max_logit > f32::NEG_INFINITY {
                                        (argmax.best_token_id as usize, argmax.max_logit)
                                    } else {
                                        sieve_failed = true;
                                        (0usize, f32::NEG_INFINITY)
                                    }
                                } else {
                                    emb_buf[..emb_dim].iter().enumerate().fold(
                                        (0usize, f32::NEG_INFINITY),
                                        |(bi, bv), (i, &v)| {
                                            if v > bv {
                                                (i, v)
                                            } else {
                                                (bi, bv)
                                            }
                                        },
                                    )
                                }
                            } else {
                                let _layers = engine.dispatch_transformer_forward(
                                    idx,
                                    &mut emb_buf[..emb_dim],
                                    emb_dim,
                                    &mut scratch_a,
                                    &mut scratch_b,
                                    token_idx,
                                    TEST_TRANSFORMER_LAYER_CAP,
                                );
                                let _ = engine.apply_output_norm_inplace(
                                    idx,
                                    &mut emb_buf[..emb_dim],
                                    emb_dim,
                                );
                                let sieve_mask = sieve.as_ref().map(|s| s.current_mask());
                                if let Some(argmax) = engine.dispatch_output_argmax_chunked(
                                    idx,
                                    &emb_buf[..emb_dim],
                                    emb_dim,
                                    &mut scratch_a[..],
                                    TEST_VOCAB_CHUNK_CAP,
                                    sieve_mask,
                                ) {
                                    if argmax.max_logit > f32::NEG_INFINITY {
                                        (argmax.best_token_id as usize, argmax.max_logit)
                                    } else {
                                        sieve_failed = true;
                                        (0usize, f32::NEG_INFINITY)
                                    }
                                } else {
                                    emb_buf[..emb_dim].iter().enumerate().fold(
                                        (0usize, f32::NEG_INFINITY),
                                        |(bi, bv), (i, &v)| {
                                            if v > bv {
                                                (i, v)
                                            } else {
                                                (bi, bv)
                                            }
                                        },
                                    )
                                }
                            }
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
                        logits.iter().enumerate().fold(
                            (0usize, f32::NEG_INFINITY),
                            |(bi, bv), (i, &v)| {
                                if v > bv {
                                    (i, v)
                                } else {
                                    (bi, bv)
                                }
                            },
                        )
                    };

                    // anomaly 0x01 = normal. Removed IEEE mantissa 0x99 check (random ~1/256 fire).
                    let anomaly = 0x01u8;
                    let _ = anomaly; // reserved for real governance signals

                    if sieve_failed {
                        break;
                    }

                    // DenyRollback must never inject sequential garbage (cur+1).
                    if rollback {
                        log::warn!(
                            "LLM_DECODE|sentinel-deny-rollback|keeping argmax token {} (no cur+1)",
                            top_i
                        );
                    }
                    let next = (top_i as u32) % vlen;

                    if let Some(ref mut s) = sieve {
                        match s.apply_token(next) {
                            Ok(()) => {
                                out_ids.push(next);
                                ctx.push(next);
                                if s.is_complete() {
                                    semantic_quin = Some(s.assemble_quin(prov_hash));
                                    break;
                                }
                            }
                            Err(_) => {
                                sieve_failed = true;
                                break;
                            }
                        }
                    } else {
                        out_ids.push(next);
                        ctx.push(next);
                        if let Some(ref mut cb) = on_token {
                            let full = tok.decode(&out_ids);
                            if full.len() > streamed_len {
                                let delta = full[streamed_len..].to_string();
                                streamed_len = full.len();
                                cb(delta);
                            }
                        }
                        let fixed = crate::llm_bench::decode_budget_fixed_tokens();
                        if out_ids.len() >= gen_budget || (!fixed && tok.is_stop_token(next)) {
                            break;
                        }
                    }
                }

                let text = if semantic_quin.is_some() {
                    String::new()
                } else if sieve_failed {
                    String::from("[sieve-misaligned]")
                } else {
                    let raw = tok.decode(&out_ids);
                    if crate::inference_modes::post_turn_verify_enabled() {
                        let v = crate::post_turn_verify::verify_and_heal_turn(&prompt_owned, &raw);
                        if crate::post_turn_verify::return_html_as_text() {
                            v.display_html
                        } else {
                            v.final_text
                        }
                    } else {
                        crate::quant_graph_grounding::maybe_ground_generation(&prompt_owned, &raw)
                            .text
                    }
                };

                // Return engine to global instance
                {
                    crate::gguf_bridge::WASM_ENGINE_INSTANCE.with(|g| {
                        *g.borrow_mut() = Some(engine);
                    });
                }

                (text, out_ids.len() as u32, semantic_quin, sieve_failed)
            };
            let mut prov = vec![prov_hash];
            if prov_hash == 0 {
                prov.push(q_hash("qualia:grounded"));
            }
            if let Some(q) = semantic_quin {
                prov.push(q.subject);
                prov.push(q.predicate);
                prov.push(q.object);
            }
            if sieve_failed && semantic_quin.is_none() {
                return (text, prov, tokens, None);
            }
            return (text, prov, tokens, semantic_quin);
        }
    }
}
