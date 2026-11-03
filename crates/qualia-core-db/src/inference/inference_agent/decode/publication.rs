use super::*;

use crate::gguf_sharder::GgufTokenizer;

pub(super) struct Publication<'a> {
    pub top_i: usize,
    pub vlen: u32,
    pub tok: &'a GgufTokenizer,
    pub sieve: &'a mut Option<crate::neuro_symbolic_sieve::NeuroSymbolicSieve>,
    pub out_ids: &'a mut Vec<u32>,
    pub ctx: &'a mut Vec<u32>,
    pub prov_hash: u64,
    pub semantic_quin: Option<NQuin>,
    pub sieve_failed: bool,
    pub stage_telemetry: bool,
    pub ssm_layer_count: u32,
    pub moe_layer_count: u32,
    pub stage_trace: &'a mut StageTrace,
    pub track_thinking: bool,
    pub think_piece: &'a mut [u8; 64],
    pub think_tracker: &'a mut ThinkingTracker,
    pub thinking_budget: Option<u32>,
    pub stream_tx_thread: &'a Option<std::sync::mpsc::SyncSender<String>>,
    pub streamed_len: usize,
    pub gen_budget: usize,
}
pub(super) fn run(input: Publication<'_>) -> (bool, usize, bool, Option<NQuin>) {
    let Publication {
        top_i,
        vlen,
        tok,
        sieve,
        out_ids,
        ctx,
        prov_hash,
        mut semantic_quin,
        mut sieve_failed,
        stage_telemetry,
        ssm_layer_count,
        moe_layer_count,
        stage_trace,
        track_thinking,
        think_piece,
        think_tracker,
        thinking_budget,
        stream_tx_thread,
        mut streamed_len,
        gen_budget,
    } = input;
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
                    return (true, streamed_len, sieve_failed, semantic_quin);
                }
            }
            Err(_) => {
                sieve_failed = true;
                return (true, streamed_len, sieve_failed, semantic_quin);
            }
        }
    } else {
        out_ids.push(next);
        ctx.push(next);
        // Lifecycle stages, model-native: every accepted token ran
        // one forward — SSM state steps and MoE dispatches inside it.
        if stage_telemetry {
            if ssm_layer_count > 0 {
                stage_trace.record(
                    InferenceStageKind::RecurrentStateUpdate,
                    StageStatus::Executed,
                    0,
                    ssm_layer_count,
                );
            }
            if moe_layer_count > 0 {
                stage_trace.record(
                    InferenceStageKind::MoEExpertDispatch,
                    StageStatus::Executed,
                    0,
                    moe_layer_count,
                );
            }
        }
        // Deliberation (`<think>…</think>`) tracking; enforced
        // against the contract-reserved thinking budget when set.
        if track_thinking {
            let piece_len = tok.decode_token_bytes_into(next, think_piece).unwrap_or(0);
            match think_tracker.feed_token(&think_piece[..piece_len]) {
                ThinkingEvent::Entered => {
                    if stage_telemetry {
                        stage_trace.record(
                            InferenceStageKind::ThinkingDeliberation,
                            StageStatus::Executed,
                            0,
                            0,
                        );
                    }
                }
                ThinkingEvent::Exited => {
                    if stage_telemetry {
                        stage_trace.record(
                            InferenceStageKind::ThinkingDeliberation,
                            StageStatus::Executed,
                            0,
                            think_tracker.thinking_tokens(),
                        );
                    }
                }
                ThinkingEvent::None => {}
            }
            if let Some(budget) = thinking_budget {
                if think_tracker.in_thinking() && think_tracker.thinking_tokens() > budget {
                    // Deliberation exhausted its reservation —
                    // quarantine the trace and end the turn.
                    if stage_telemetry {
                        stage_trace.record(
                            InferenceStageKind::ThinkingDeliberation,
                            StageStatus::Quarantined,
                            0,
                            think_tracker.thinking_tokens(),
                        );
                    }
                    return (true, streamed_len, sieve_failed, semantic_quin);
                }
            }
        }
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
        if out_ids.len() >= gen_budget || (!fixed && tok.is_stop_token(next)) {
            return (true, streamed_len, sieve_failed, semantic_quin);
        }
    }

    (false, streamed_len, sieve_failed, semantic_quin)
}
