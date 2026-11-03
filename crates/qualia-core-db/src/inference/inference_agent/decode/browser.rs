// Mechanical transitional partition; no acceptance changes. See proposal.md.
use super::*;
impl LocalLlmAgent {
    #[allow(unused_variables, unused_mut)]
    pub(super) fn infer_wasm_partition<F: FnMut(String) + Send + 'static>(
        &self,
        prompt: &str,
        graph_context: &str,
        control: Option<DecodeControl>,
        mut on_token: Option<F>,
        prov_hash: u64,
        use_sieve: bool,
        sieve_spec: Option<crate::neuro_symbolic_sieve::SieveLexSpec>,
        sieve_lex_path: Option<String>,
    ) -> (String, Vec<u64>, u32, Option<NQuin>) {
        #[cfg(all(target_arch = "wasm32", feature = "gpu-runtime"))]
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
                const MAX_FFN_DIM: usize = 16384;
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
                            if engine
                                .dispatch_prefill_chunk(
                                    idx,
                                    &mut prefill_chunk[..batch_elems],
                                    emb_dim,
                                    n as u32,
                                    pos as u32,
                                    &mut scratch_a,
                                    &mut scratch_b,
                                    TEST_TRANSFORMER_LAYER_CAP,
                                )
                                .is_err()
                            {
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
                }
                .min(
                    control
                        .as_ref()
                        .map(DecodeControl::token_budget)
                        .unwrap_or(usize::MAX),
                );

                crate::compute_universe::start_tensor_search_producer();
                crate::compute_universe::publish_query_tensor(
                    crate::tensor::Tensor10D::default(),
                    0,
                );
                crate::qualia_hybrid::prepare_hybrid_decode(&prompt_owned);

                for step in 0..gen_budget {
                    if control.as_ref().is_some_and(DecodeControl::is_cancelled) {
                        break;
                    }
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
                        gen_budget,
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

                    let (top_i, top_v, selection_failed) =
                        browser_select::run(browser_select::BrowserSelection {
                            engine: &mut engine,
                            tensor_idx: &tensor_idx,
                            ctx: &ctx,
                            cur,
                            emb_dim,
                            emb_buf: &mut emb_buf,
                            scratch_a: &mut scratch_a,
                            scratch_b: &mut scratch_b,
                            lora_adapter: &lora_adapter,
                            sieve: &sieve,
                            vlen,
                            sieve_failed,
                        });
                    sieve_failed = selection_failed;
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

                    // T53/W11: feed the selected token back into the DOMINO
                    // grammar state machine so the mask advances for the next
                    // token. No-op when DOMINO is not active.
                    if crate::llm_bench::domino_active() {
                        let token_bytes = tok.decode(&[next]);
                        crate::llm_bench::domino_feed_token(token_bytes.as_bytes());
                    }

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
        #[cfg(all(target_arch = "wasm32", not(feature = "gpu-runtime")))]
        {
            if crate::extension_bus::wasm_bus::is_connected() {
                if let Some(cb) = on_token {
                    let _ = crate::extension_bus::wasm_bus::send_intent(prompt, graph_context, cb);
                } else {
                    let _ =
                        crate::extension_bus::wasm_bus::send_intent(prompt, graph_context, |_| {});
                }
                return (String::new(), vec![prov_hash], 0, None);
            }
            (
                String::from("[local LLM execution not compiled into this WASM profile]"),
                vec![prov_hash],
                0,
                None,
            )
        }
    }
}
