// Mechanical decoder partition; token-acceptance integration remains separate.
use super::*;
impl LocalLlmAgent {
    pub(super) fn infer_native_partition<F: FnMut(String) + Send + 'static>(
        &self,
        prompt: &str,
        _graph_context: &str,
        control: Option<DecodeControl>,
        mut on_token: Option<F>,
        prov_hash: u64,
        use_sieve: bool,
        sieve_spec: Option<crate::neuro_symbolic_sieve::SieveLexSpec>,
        sieve_lex_path: Option<String>,
        execution: Option<super::super::native_request::NativeDecodeRequest>,
    ) -> (String, Vec<u64>, u32, Option<NQuin>) {
        use crate::gguf_bridge::QTensorEngine;
        use rtrb::RingBuffer;

        let execution = execution.or_else(|| {
            self.native_request
                .lock()
                .ok()?
                .take()
                .map(super::super::native_request::NativeDecodeRequest::Governed)
        });
        let Some(execution) = execution else {
            log::error!("LLM_DECODE|missing-prepared-governed-request");
            return (
                String::from("[native-governance-required]"),
                vec![prov_hash],
                0,
                None,
            );
        };
        let discard = matches!(
            &execution,
            super::super::native_request::NativeDecodeRequest::Governed(_)
        );

        let (model_path, model_instance_hash) = match &self.backend {
            AgentBackend::Local { model_path, .. } => {
                (model_path.clone(), crate::q_hash(model_path))
            }
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

        // LogitStream: LLM engine → Webizen Sentinel
        let (lp, mut lc) = RingBuffer::<LlmMsg>::new(1024);
        // ControlStream: Webizen Sentinel → LLM engine
        let (mut cp, cc) = RingBuffer::<SentMsg>::new(16);

        let stream_pair = if on_token.is_some() {
            Some(std::sync::mpsc::sync_channel::<String>(512))
        } else {
            None
        };
        let stream_tx_thread = stream_pair.as_ref().map(|(tx, _)| tx.clone());
        let control = control.as_ref().map(DecodeControl::frozen_budgets);
        let control_thread = control.clone();

        // Move the (optional) LoRA adapter into the inference thread.
        let lora_for_thread = lora_active_adapter;

        // Sticky pool thread owns the engine (thread_local); caller runs Sentinel.
        let (done_tx, done_rx) =
            std::sync::mpsc::sync_channel::<(String, u32, Option<NQuin>, bool, StageTrace)>(1);

        // ── LLM engine job (sticky 1-thread pool) ────────────────────────
        let prepared_settings = match crate::inference::inference_bench::prepared_settings::PreparedInferenceSettings::capture() {
            Ok(settings) => settings,
            Err(error) => {
                log::error!("LLM_DECODE|settings-capacity-refused|{error:?}");
                return (String::from("[native-preparation-refused]"), vec![prov_hash], 0, None);
            }
        };
        sticky_infer::pool().spawn(move || {
            let _settings_scope = prepared_settings.clone().enter();
            let reusable = std::sync::atomic::AtomicBool::new(!discard);
            let result = sticky_infer::with_engine_disposition(
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
                    worker::run(worker::Worker {
                        engine,
                        model_path: &model_path,
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
                        lp,
                        cc,
                        execution,
                        reusable: &reusable,
                    })
                }, // sticky_infer::with_engine f
                || !reusable.load(std::sync::atomic::Ordering::Acquire),
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

        let mut completed = None;
        loop {
            drain_tokens();
            match lc.pop() {
                Ok(LlmMsg::Eos) => break,
                Ok(LlmMsg::Logit(s)) => {
                    if mid_sentinel && s.anomaly == 0x99 {
                        let _ = cp.push(SentMsg::DenyRollback);
                    }
                }
                Err(_) => match done_rx.try_recv() {
                    Ok(result) => {
                        completed = Some(result);
                        break;
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
                    Err(std::sync::mpsc::TryRecvError::Empty) => std::hint::spin_loop(),
                },
            }
        }

        drain_tokens();

        let (text, tokens, semantic_quin, sieve_failed, stage_trace) = completed
            .map(Ok)
            .unwrap_or_else(|| done_rx.recv())
            .unwrap_or_else(|_| (String::new(), 0, None, false, StageTrace::new()));
        if !stage_trace.is_empty() {
            crate::inference::runtime::stage::set_last_stage_trace(stage_trace);
        }
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
