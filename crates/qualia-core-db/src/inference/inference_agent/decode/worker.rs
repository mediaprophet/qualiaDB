use super::*;
use crate::gguf_bridge::QTensorEngine;

pub(super) struct Worker<'a> {
    pub engine: &'a mut QTensorEngine,
    pub model_path: &'a str,
    pub model_instance_hash: u64,
    pub prompt_owned: String,
    pub lora_for_thread: Option<crate::lora::LoRAAdapter>,
    pub sieve_spec: Option<crate::neuro_symbolic_sieve::SieveLexSpec>,
    pub sieve_lex_path: Option<String>,
    pub use_sieve: bool,
    pub control_thread: Option<DecodeControl>,
    pub control: Option<DecodeControl>,
    pub stream_tx_thread: Option<std::sync::mpsc::SyncSender<String>>,
    pub prov_hash: u64,
    pub lp: rtrb::Producer<LlmMsg>,
    pub cc: rtrb::Consumer<SentMsg>,
}
pub(super) fn run(input: Worker<'_>) -> WorkerResult {
    let Worker {
        engine,
        model_path,
        model_instance_hash,
        prompt_owned,
        lora_for_thread,
        sieve_spec,
        sieve_lex_path,
        use_sieve,
        control_thread,
        control,
        stream_tx_thread,
        prov_hash,
        mut lp,
        mut cc,
    } = input;
    // Initialize Tokio runtime for the sticky thread (once per job; cheap if already warm).
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap_or_else(|e| panic!("Failed to create Tokio runtime for LLM thread: {}", e));
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

    let model::PreparedModel { tok, tensor_idx } = model::prepare(engine, &model_path);
    // Lifecycle stage telemetry (`stage:*` in
    // core-ontologies/inference-lifecycle.n3): a fixed-capacity,
    // zero-heap trace coalesced per stage kind. Presence of the
    // model-native SSM / MoE stages is probed once per turn from
    // the tensor index — cold work, not per-token.
    let stage_telemetry = crate::llm_bench::stage_telemetry_enabled();
    let mut stage_trace = StageTrace::new();
    let thinking_budget = control_thread
        .as_ref()
        .map(DecodeControl::thinking_token_budget)
        .filter(|&budget| budget > 0);
    let track_thinking = stage_telemetry || thinking_budget.is_some();
    let mut think_tracker = ThinkingTracker::new();
    let mut think_piece = [0u8; 64];
    let (ssm_layer_count, moe_layer_count) = tensor_idx
        .as_ref()
        .map(|idx| {
            let mut ssm = 0u32;
            let mut moe = 0u32;
            for layer in 0..idx.hyperparams.n_layer.min(512) {
                let tensors = idx.get_layer_tensors(layer);
                if tensors.is_hybrid_ssm_layer() {
                    ssm += 1;
                }
                if tensors.moe_router.is_some() {
                    moe += 1;
                }
            }
            (ssm, moe)
        })
        .unwrap_or((0, 0));

    // The fail-soft loader is still used by interactive callers;
    // enforce compatibility at the execution boundary so an
    // unsupported architecture cannot masquerade as a prefill
    // throughput failure.
    if let Some(idx) = tensor_idx.as_ref() {
        if let Err(reason) = idx.hyperparams.decode_supported() {
            eprintln!("[decode] Native model rejected: {reason}");
            let _ = lp.push(LlmMsg::Eos);
            return (
                String::from("[unsupported-model]"),
                0u32,
                None,
                false,
                stage_trace,
            );
        }
    }

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
    const MAX_EMB_DIM: usize = 8192;
    const MAX_FFN_DIM: usize = 16384;
    let mut emb_buf = [0f32; MAX_EMB_DIM];
    let mut scratch_a = [0f32; MAX_FFN_DIM];
    let mut scratch_b = [0f32; MAX_FFN_DIM];
    let mut prefill_chunk = [0f32; crate::gguf_bridge::PREFILL_CHUNK_STACK_FLOATS];
    let emb_dim = emb_dim.min(MAX_EMB_DIM);
    let prompt_len = ctx.len();
    let prefill_tokens = prompt_len.saturating_sub(1);
    let (kv_floats, n_layers) = engine
        .kv_layout()
        .map(|l| (l.total_f32_elems, l.n_layer))
        .unwrap_or((0, 0));
    let cache_key = super::super::prefix_cache::InferenceCacheKey::new(
        model_instance_hash,
        (tok.bos_token_id as u64) | ((tok.eos_token_id as u64) << 32),
        &ctx[..prefill_tokens],
        prov_hash,
        kv_floats,
        n_layers as usize,
    );

    let mut prefix_cached = false;
    if prefill_tokens > 0 && kv_floats > 0 && crate::llm_bench::prefix_cache_enabled() {
        if let Ok(mut cache) = get_prefix_cache().lock() {
            if cache
                .restore_if_match_exact(&cache_key, &ctx[..prefill_tokens], |cached_kv| {
                    engine.set_kv_cache_cpu(cached_kv)
                })
                .unwrap_or(false)
            {
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
    crate::tensor::kv_provenance::rebuild_prompt_provenance(
        prompt_len as u32,
        crate::tensor::resident_substrate::global_resident_substrate().node_count(),
        0,
    );
    let draft_mapper = crate::topology_draft::TopologyDraftMapper::new(&tok);
    stage_trace = match prefill::run(prefill::Prefill {
        engine,
        tensor_idx: &tensor_idx,
        ctx: &ctx,
        emb_dim,
        prefill_chunk: &mut prefill_chunk,
        scratch_a: &mut scratch_a,
        scratch_b: &mut scratch_b,
        control_thread: &control_thread,
        prefix_cached,
        prefill_tokens,
        prompt_len,
        kv_floats,
        cache_key,
        t_prefill,
        lp: &mut lp,
        stage_telemetry,
        stage_trace,
    }) {
        Ok(trace) => trace,
        Err(result) => return result,
    };
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
    }
    .min(
        control
            .as_ref()
            .map(DecodeControl::token_budget)
            .unwrap_or(usize::MAX),
    );

    #[cfg(not(target_arch = "wasm32"))]
    crate::compute_universe::start_tensor_search_producer();
    crate::compute_universe::publish_query_tensor(crate::tensor::Tensor10D::default(), 0);
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
    let mut sampler = crate::llm_bench::sampler_config().map(crate::sampler::SamplerState::new);
    #[cfg(target_arch = "wasm32")]
    let mut sampler: Option<crate::sampler::SamplerState> = None;
    let mut sampler_logits: Vec<f32> = Vec::new();
    // Decode-profiler (gated): one-shot empty submit→wait baseline on the SAME device, so
    // the bench can separate per-token fence latency from real kernel compute time.
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    if std::env::var("QUALIA_LLM_PROFILE_DECODE").is_ok() {
        let n = 64u32;
        crate::llm_bench::record_empty_rt(engine.bench_empty_submit_roundtrip(n), n as u64);
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

    let (graph_force_emitted, force_streamed_len) = forced::run(forced::Forced {
        prompt_owned: &prompt_owned,
        tok: &tok,
        control_thread: &control_thread,
        vlen,
        out_ids: &mut out_ids,
        ctx: &mut ctx,
        stream_tx_thread: &stream_tx_thread,
        streamed_len,
        gen_budget,
    });
    streamed_len = force_streamed_len;
    if !graph_force_emitted {
        for step in 0..gen_budget {
            if control_thread
                .as_ref()
                .is_some_and(DecodeControl::is_cancelled)
            {
                break;
            }
            crate::gpu_context::record_llm_decode_step();

            // Cooperative deadline: break BEFORE the wall-clock timeout instead of
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

            let (spec_step, spec_streamed_len) = speculative::run(speculative::Speculative {
                engine,
                tensor_idx: &tensor_idx,
                sieve: &sieve,
                sampler: &sampler,
                ctx: &mut ctx,
                tok: &tok,
                prompt_owned: &prompt_owned,
                lp: &mut lp,
                vlen,
                out_ids: &mut out_ids,
                stream_tx_thread: &stream_tx_thread,
                streamed_len,
                gen_budget,
                stage_telemetry,
                ssm_layer_count,
                moe_layer_count,
                stage_trace: &mut stage_trace,
            });
            streamed_len = spec_streamed_len;
            match spec_step {
                speculative::SpecStep::Continue => continue,
                speculative::SpecStep::Stop => break,
                speculative::SpecStep::NotHandled => {}
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
                gen_budget,
                None,
            );
            // A draft batch can accept several tokens in one outer decode step. Enforce
            // the same output cap used by the scalar path before any continuation.
            if out_ids.len() >= gen_budget {
                break;
            }
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
            let sentinel_rollback = if crate::inference_modes::sentinel_mid_decode_enabled() {
                cc.pop().is_ok()
            } else {
                false
            };
            let mut rollback = sentinel_rollback;
            if matches!(draft_step, TopologyDraftStep::Denied) {
                rollback = true;
            }

            let cur = *ctx.last().unwrap_or(&tok.bos_token_id);
            crate::compute_universe::publish_decode_hint(cur, step as u32);

            let (top_i, top_v, updated_page, updated_sieve_failure) =
                select::run(select::Selection {
                    engine,
                    tensor_idx: &tensor_idx,
                    ctx: &ctx,
                    tok: &tok,
                    cur,
                    emb_dim,
                    emb_buf: &mut emb_buf,
                    scratch_a: &mut scratch_a,
                    scratch_b: &mut scratch_b,
                    lora_adapter: &lora_adapter,
                    sieve: &sieve,
                    sampler: &mut sampler,
                    sampler_logits: &mut sampler_logits,
                    gpu_topk_enabled,
                    prompt_owned: &prompt_owned,
                    chunk_policy: &chunk_policy,
                    current_page_id,
                    thermal_wal_opt: &mut thermal_wal_opt,
                    sieve_failed,
                });
            current_page_id = updated_page;
            sieve_failed = updated_sieve_failure;
            diagnostic::run(diagnostic::Diagnostic {
                step,
                eos,
                vlen,
                cur,
                top_i,
                top_v,
                tok: &tok,
                engine,
                tensor_idx: &tensor_idx,
                emb_buf: &emb_buf,
                emb_dim,
            });
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
                if stage_telemetry {
                    stage_trace.record(
                        InferenceStageKind::SentinelMidDecodeGuard,
                        StageStatus::RolledBack,
                        0,
                        1,
                    );
                }
                // Sentinel DenyRollback: the denied token is discarded and
                // the turn ends — loop exit pushes `LlmMsg::Eos` below.
                if sentinel_rollback {
                    break;
                }
            }
            let (stop, next_streamed_len, next_sieve_failed, next_semantic_quin) =
                publication::run(publication::Publication {
                    top_i,
                    vlen,
                    tok: &tok,
                    sieve: &mut sieve,
                    out_ids: &mut out_ids,
                    ctx: &mut ctx,
                    prov_hash,
                    semantic_quin,
                    sieve_failed,
                    stage_telemetry,
                    ssm_layer_count,
                    moe_layer_count,
                    stage_trace: &mut stage_trace,
                    track_thinking,
                    think_piece: &mut think_piece,
                    think_tracker: &mut think_tracker,
                    thinking_budget,
                    stream_tx_thread: &stream_tx_thread,
                    streamed_len,
                    gen_budget,
                });
            streamed_len = next_streamed_len;
            sieve_failed = next_sieve_failed;
            semantic_quin = next_semantic_quin;
            if stop {
                break;
            }
        }
    } // end else: normal decode (not QUALIA_GRAPH_FORCE)

    render::run(render::Finalization {
        t_decode,
        out_ids: &out_ids,
        stage_telemetry,
        stage_trace,
        lp: &mut lp,
        semantic_quin,
        sieve_failed,
        tok: &tok,
        prompt_owned: &prompt_owned,
    })
}
