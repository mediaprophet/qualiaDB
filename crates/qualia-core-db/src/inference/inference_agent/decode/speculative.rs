use super::*;
use crate::gguf_bridge::QTensorEngine;
use crate::gguf_sharder::{GgufTensorIndex, GgufTokenizer};
pub(super) enum SpecStep {
    NotHandled,
    Continue,
    Stop,
}

pub(super) struct Speculative<'a, 'slot, 'bytes> {
    pub gate: &'a mut super::super::native_request::PublicationGate<'slot, 'bytes>,
    pub engine: &'a mut QTensorEngine,
    pub tensor_idx: &'a Option<GgufTensorIndex>,
    pub sieve: &'a Option<crate::neuro_symbolic_sieve::NeuroSymbolicSieve>,
    pub sampler: &'a Option<crate::sampler::SamplerState>,
    pub ctx: &'a mut Vec<u32>,
    pub tok: &'a GgufTokenizer,
    pub prompt_owned: &'a str,
    pub lp: &'a mut rtrb::Producer<LlmMsg>,
    pub vlen: u32,
    pub out_ids: &'a mut Vec<u32>,
    pub stream_tx_thread: &'a Option<std::sync::mpsc::SyncSender<String>>,
    pub streamed_len: usize,
    pub gen_budget: usize,
    pub stage_telemetry: bool,
    pub ssm_layer_count: u32,
    pub moe_layer_count: u32,
    pub stage_trace: &'a mut StageTrace,
}
pub(super) fn run(input: Speculative<'_, '_, '_>) -> (SpecStep, usize) {
    let Speculative {
        gate,
        engine,
        tensor_idx,
        sieve,
        sampler,
        ctx,
        tok,
        prompt_owned,
        lp,
        vlen,
        out_ids,
        stream_tx_thread,
        mut streamed_len,
        gen_budget,
        stage_telemetry,
        ssm_layer_count,
        moe_layer_count,
        stage_trace,
    } = input;
    if lp.is_abandoned() {
        gate.invalidate();
        return (SpecStep::Stop, streamed_len);
    }
    // W6a — prompt-lookup / graph fact speculative decode (default OFF, exact-output).
    // Prefer quant-graph fact draft when mode=quant-graph; else n-gram prompt-lookup.
    // FastVerify: skip mid-decode fact draft (post-turn heal only) for Ollama-like speed.
    // Verify drafts in ONE batched forward. Bit-identical to greedy when accepted.
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    if (crate::llm_bench::spec_decode_enabled()
        || (crate::inference_modes::quant_graph_grounding_enabled()
            && crate::inference_modes::sentinel_mid_decode_enabled()))
        && sieve.is_none()
        && sampler.is_none()
        && TEST_TRANSFORMER_LAYER_CAP == 0
    {
        if let Some(idx) = tensor_idx.as_ref() {
            let cur = *ctx.last().unwrap_or(&tok.bos_token_id);
            let draft =
                crate::qualia_hybrid::propose_best_draft(&prompt_owned, &ctx, &|s| tok.encode(s));
            if draft.len > 0 {
                // inputs = [cur, d0..d_{m-1}] at positions [pos, pos+m], pos = ctx.len()-1.
                let mut inputs = Vec::with_capacity(draft.len + 1);
                inputs.push(cur);
                inputs.extend_from_slice(draft.as_slice());
                let pos = ctx.len().saturating_sub(1) as u32;
                let mut amax: Vec<u32> = Vec::new();
                let mut alog: Vec<f32> = Vec::new();
                gate.tentative_branch();
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
                        if lp.is_abandoned() {
                            gate.invalidate();
                            return (SpecStep::Stop, streamed_len);
                        }
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
                        let mut accepted = None;
                        if gate
                            .token(
                                tokn % vlen,
                                ctx.len().saturating_sub(1) as u64,
                                super::super::governed::evidence::CandidateSource::SpeculativeToken,
                                Some(super::super::governed::evidence::NumericalObservation {
                                    meaning: super::super::governed::evidence::ScoreMeaning::Logit,
                                    value: _logv,
                                }),
                                tok,
                                &mut accepted,
                            )
                            .is_err()
                        {
                            return (SpecStep::Stop, streamed_len);
                        }
                        let next = accepted
                            .expect("successful speculative token output transfer")
                            .token_id();
                        out_ids.push(next);
                        ctx.push(next);
                        if let Some(ref tx) = stream_tx_thread {
                            let full = tok.decode(&out_ids);
                            if full.len() > streamed_len {
                                let delta = full[streamed_len..].to_string();
                                streamed_len = full.len();
                                if tx.send(delta).is_err() {
                                    gate.invalidate();
                                    return (SpecStep::Stop, streamed_len);
                                }
                            }
                        }
                        let fixed = crate::llm_bench::decode_budget_fixed_tokens();
                        if out_ids.len() >= gen_budget || (!fixed && tok.is_stop_token(next)) {
                            stop = true;
                            break;
                        }
                    }
                    if stop {
                        return (SpecStep::Stop, streamed_len);
                    }
                    // The batched draft verification ran k+1 token
                    // forwards — count the SSM / MoE work per token.
                    if stage_telemetry {
                        if ssm_layer_count > 0 {
                            stage_trace.record(
                                InferenceStageKind::RecurrentStateUpdate,
                                StageStatus::Executed,
                                0,
                                ssm_layer_count.saturating_mul((k + 1) as u32),
                            );
                        }
                        if moe_layer_count > 0 {
                            stage_trace.record(
                                InferenceStageKind::MoEExpertDispatch,
                                StageStatus::Executed,
                                0,
                                moe_layer_count.saturating_mul((k + 1) as u32),
                            );
                        }
                    }
                    return (SpecStep::Continue, streamed_len); // skip scalar path
                }
            }
        }
    }

    (SpecStep::NotHandled, streamed_len)
}
