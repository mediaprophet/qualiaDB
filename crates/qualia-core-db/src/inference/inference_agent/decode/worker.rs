use super::*;
use crate::gguf_bridge::QTensorEngine;

pub(super) struct Worker<'a> {
    pub reusable: &'a std::sync::atomic::AtomicBool,
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
    pub execution: super::super::native_request::NativeDecodeRequest,
}
pub(super) fn run(input: Worker<'_>) -> WorkerResult {
    let Worker {
        reusable,
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
        mut execution,
    } = input;
    // Every refusal/cancellation/closed output defaults to destruction. Only a
    // successful completed turn opts the sticky numerical owner back into reuse.
    reusable.store(false, std::sync::atomic::Ordering::Release);
    // Initialize Tokio runtime for the sticky thread (once per job; cheap if already warm).
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap_or_else(|e| panic!("Failed to create Tokio runtime for LLM thread: {}", e));
    let _rt_guard = rt.enter();

    // A0 phase timing (D17/D22): once-per-phase, off the per-token hot path.
    let t_phase = std::time::Instant::now();

    let mut thermal_wal_opt = admission::thermal_wal();

    let lora_adapter = std::sync::Arc::new(lora_for_thread);
    let mut stage_trace = StageTrace::new();
    if let Err(error) = execution.prepare_model(engine) {
        return admission::refused(&mut lp, stage_trace, error);
    }

    let model::PreparedModel { tok, tensor_idx } = model::prepare(engine, &model_path);
    // Resolve model stage telemetry once per cold request.
    let stage_telemetry = crate::llm_bench::stage_telemetry_enabled();
    let thinking_budget = admission::thinking_budget(control_thread.as_ref());
    let track_thinking = stage_telemetry || thinking_budget.is_some();
    let mut think_tracker = ThinkingTracker::new();
    let mut think_piece = [0u8; 64];
    let (ssm_layer_count, moe_layer_count) = model::layer_kinds(tensor_idx.as_ref());

    // Qwen3.5 requires its own reviewed mixed-attention owner. The generic
    // GGUF architecture guard remains authoritative for every other model.
    let qwen35_plan = if let Some(index) = tensor_idx
        .as_ref()
        .filter(|index| index.hyperparams.architecture == etalon_model::inference::ARCH_QWEN35_MOE)
    {
        let Some(plan) = engine.qwen35_plan(index).cloned() else {
            return admission::unsupported(&mut lp, stage_trace);
        };
        Some(plan)
    } else {
        if !model::decode_supported(tensor_idx.as_ref()) {
            return admission::unsupported(&mut lp, stage_trace);
        }
        None
    };

    let cold::Inputs {
        mut ctx,
        mut sieve,
        gen_budget,
        grammar_scope: _grammar_scope,
    } = match cold::prepare(
        &execution,
        &tok,
        &prompt_owned,
        use_sieve,
        sieve_spec.as_ref(),
        sieve_lex_path.as_deref(),
        control.as_ref(),
    ) {
        Ok(inputs) => inputs,
        Err(error) => return admission::refused(&mut lp, stage_trace, error),
    };
    let governed = matches!(
        &execution,
        super::super::native_request::NativeDecodeRequest::Governed(_)
    );
    let compute_epoch = crate::compute_universe::begin_request_epoch();
    let _grounding_scope = match execution.input_budget() {
        Some(budget) => match crate::quant_graph_grounding::scoped::GroundingScope::capture(
            budget.retained_bytes / 3,
        ) {
            Ok(scope) => Some(scope),
            Err(_) => {
                return admission::refused(
                    &mut lp,
                    stage_trace,
                    super::super::governed::PolicyError::Budget,
                )
            }
        },
        None => None,
    };
    let _provider_scope = if let (Some(budget), Some(facts), Some(grammar)) = (
        execution.input_budget(),
        _grounding_scope.as_ref(),
        _grammar_scope.as_ref(),
    ) {
        let context = match super::super::native_request::capture_context(
            &compute_epoch,
            facts,
            &prompt_owned,
            prov_hash,
            &ctx,
            budget.retained_bytes - budget.retained_bytes / 3,
        ) {
            Ok(context) => context,
            Err(error) => return admission::refused(&mut lp, stage_trace, error),
        };
        match execution.seal_request(
            engine,
            context,
            grammar.lease(),
            &prompt_owned,
            prov_hash,
            &ctx,
            std::sync::Arc::clone(&tok),
            std::sync::Arc::clone(&lora_adapter),
            sieve.clone(),
            gen_budget,
            thinking_budget.unwrap_or(0),
        ) {
            Ok(scope) => scope,
            Err(error) => return admission::refused(&mut lp, stage_trace, error),
        }
    } else {
        None
    };
    let mut evidence_bytes = [0u8; admission::EVIDENCE_BYTES];
    let mut evidence = match admission::storage(&mut evidence_bytes, gen_budget as u64 + 16) {
        Ok(storage) => storage,
        Err(error) => return admission::refused(&mut lp, stage_trace, error),
    };
    let (mut gate, _compute_epoch) = match admission::start(
        execution,
        compute_epoch,
        engine,
        &prompt_owned,
        prov_hash,
        &ctx,
        &mut evidence,
    ) {
        Ok(admitted) => admitted,
        Err(error) => return admission::refused(&mut lp, stage_trace, error),
    };
    gate.bind_control(control_thread.clone());
    // Initial input admission precedes prefix restoration and numerical prefill.
    crate::tensor::kv_provenance::rebuild_prompt_provenance(
        ctx.len() as u32,
        crate::compute_universe::context_node_count(),
        0,
    );
    if let Err(error) = gate.context_input() {
        return admission::refused(&mut lp, stage_trace, error);
    }
    if let Some(plan) = qwen35_plan {
        // This typed route currently implements exact scalar greedy decoding.
        // Refuse modifiers it cannot execute rather than silently changing the
        // frozen numerical request or falling back to the generic architecture.
        if sieve.is_some()
            || lora_adapter.is_some()
            || crate::llm_bench::sampler_config().is_some()
            || crate::llm_bench::domino_active()
            || crate::qualia_hybrid::graph_force_enabled()
        {
            gate.invalidate();
            return admission::refused(
                &mut lp,
                stage_trace,
                super::super::governed::PolicyError::InvalidCandidate,
            );
        }
        // Check again after source preparation and policy sealing. A detached
        // old mmap owner cannot execute under a replacement source's receipt.
        let index = tensor_idx.as_ref().expect("typed Qwen index");
        if !engine
            .qwen35_plan(index)
            .is_some_and(|current| std::sync::Arc::ptr_eq(current, &plan))
        {
            gate.invalidate();
            return admission::refused(
                &mut lp,
                stage_trace,
                super::super::governed::PolicyError::InvalidCandidate,
            );
        }
        let Some(session) = engine.take_qwen35_session(index) else {
            gate.invalidate();
            return admission::refused(
                &mut lp,
                stage_trace,
                super::super::governed::PolicyError::InvalidCandidate,
            );
        };
        // This turn consumes the numerical owner. Keep sticky reuse disabled
        // even after success so the next request cold-loads a fresh session.
        reusable.store(false, std::sync::atomic::Ordering::Release);
        crate::llm_bench::record_load_ns(t_phase.elapsed().as_nanos() as u64);
        return qwen35::run(qwen35::Request {
            plan,
            session,
            ctx: &mut ctx,
            tok: &tok,
            gate: &mut gate,
            epoch: &_compute_epoch,
            control: &control_thread,
            lp: &mut lp,
            cc: &mut cc,
            stream: &stream_tx_thread,
            prompt: &prompt_owned,
            graph: prov_hash,
            gen_budget,
            stage_telemetry,
            stage_trace,
            thinking_budget,
        });
    }
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
    )
    .with_governed_scope(gate.cache_scope());
    // Recurrent state is a separate arena: a KV-only prefix cannot restore it.
    // Each new turn starts clean; scalar successful owners retain weights/plans.
    engine.reset_recurrent_state();
    let prefix_cache_allowed = ssm_layer_count == 0;

    let mut prefix_cached = false;
    if prefix_cache_allowed
        && prefill_tokens > 0
        && kv_floats > 0
        && crate::llm_bench::prefix_cache_enabled()
    {
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
        prefix_cache_allowed,
    }) {
        Ok(trace) => trace,
        Err(result) => return result,
    };
    let mut out_ids: Vec<u32> = Vec::new();
    let mut streamed_len = 0usize;
    let mut semantic_quin: Option<NQuin> = None;
    let mut sieve_failed = false;

    if let Err(error) = admission::activate_compute(&_compute_epoch, &prompt_owned) {
        return admission::refused(&mut lp, stage_trace, error);
    }
    let t_decode = std::time::Instant::now();
    let gpu_topk_enabled = crate::llm_bench::gpu_topk_enabled();
    // Exact sampler retains the existing penalty/temperature/top-k/top-p chain.
    #[cfg(not(target_arch = "wasm32"))]
    let mut sampler = crate::llm_bench::sampler_config().map(crate::sampler::SamplerState::new);
    #[cfg(target_arch = "wasm32")]
    let mut sampler: Option<crate::sampler::SamplerState> = None;
    let mut sampler_logits: Vec<f32> = Vec::new();
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    if crate::gguf_bridge::execution_env::var("QUALIA_LLM_PROFILE_DECODE").is_ok() {
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
        gate: &mut gate,
        lp: &lp,
    });
    streamed_len = force_streamed_len;
    if !graph_force_emitted {
        for step in 0..gen_budget {
            if lp.is_abandoned() {
                gate.invalidate();
                break;
            }
            let Some(_context_step) = _compute_epoch.pause() else {
                log::warn!("LLM_DECODE|native-epoch-refused");
                return admission::refused(
                    &mut lp,
                    stage_trace,
                    super::super::governed::PolicyError::Revoked,
                );
            };
            if let Err(error) = gate.context_input() {
                log::warn!("LLM_DECODE|native-context-refused|{error}");
                return admission::refused(&mut lp, stage_trace, error);
            }
            if control_thread
                .as_ref()
                .is_some_and(DecodeControl::is_cancelled)
            {
                gate.invalidate();
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
                gate: &mut gate,
            });
            streamed_len = spec_streamed_len;
            match spec_step {
                speculative::SpecStep::Continue => continue,
                speculative::SpecStep::Stop => break,
                speculative::SpecStep::NotHandled => {}
            }
            let stream_failed = std::cell::Cell::new(false);
            let mut stream_draft = |delta| {
                if stream_tx_thread
                    .as_ref()
                    .is_some_and(|tx| tx.send(delta).is_err())
                {
                    stream_failed.set(true);
                }
            };
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
                None,
                gen_budget,
                Some(&mut stream_draft),
                Some(&mut |proposal| {
                    if stream_failed.get() || lp.is_abandoned() {
                        gate.invalidate();
                        return None;
                    }
                    match proposal {
                        Some((token, position)) => {
                            let mut accepted = None;
                            if gate.token(
                                token,
                                position,
                                super::super::governed::evidence::CandidateSource::TopologyToken,
                                None,
                                &tok,
                                &mut accepted,
                            )
                            .is_ok() {
                                accepted.map(super::super::native_request::NativeAcceptedToken::token_id)
                            } else { None }
                        }
                        None => {
                            gate.tentative_branch();
                            Some(0)
                        }
                    }
                }),
            );
            if stream_failed.get() {
                gate.invalidate();
            }
            if gate.is_terminal() {
                break;
            }
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
            if lp.is_abandoned() {
                gate.invalidate();
                break;
            }
            let (stop, next_streamed_len, next_sieve_failed, next_semantic_quin) =
                publication::run(publication::Publication {
                    top_i,
                    top_v,
                    gate: &mut gate,
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

    let result = render::run(render::Finalization {
        t_decode,
        out_ids: &out_ids,
        stage_telemetry,
        stage_trace,
        lp: &mut lp,
        semantic_quin,
        sieve_failed,
        tok: &tok,
        prompt_owned: &prompt_owned,
        gate: &mut gate,
    });
    if !sieve_failed
        && !gate.is_terminal()
        && !control_thread
            .as_ref()
            .is_some_and(DecodeControl::is_cancelled)
        && (t_decode.elapsed().as_millis() as u64) < effective_inference_timeout_ms()
    {
        reusable.store(gate.reusable(), std::sync::atomic::Ordering::Release);
    }
    result
}
