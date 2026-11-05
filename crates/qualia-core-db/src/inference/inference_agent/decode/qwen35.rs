//! Typed scalar Qwen3.5 route. Generic transformer admission is unchanged.
//! The admitted architecture owns all KV/recurrent state for one request; no
//! generic prefix cache, topology draft or partial forward can stand in for it.
use super::super::{governed::PolicyError, native_request::PublicationGate};
use super::*;
use crate::gguf_bridge::qwen35::ftw::{
    session::{Session, SessionError},
    Plan,
};
use crate::gguf_sharder::GgufTokenizer;
use std::sync::Arc;

fn numerical_failure(
    stage: &str,
    position: usize,
    error: SessionError,
    elapsed: std::time::Duration,
    budget_ms: u64,
) {
    log::error!("LLM_DECODE|qwen35-{stage}-failed|position={position}|error={error:?}|elapsed_ms={}|budget_ms={budget_ms}", elapsed.as_millis());
    #[cfg(test)]
    eprintln!("LLM_DECODE|qwen35-{stage}-failed|position={position}|error={error:?}|elapsed_ms={}|budget_ms={budget_ms}", elapsed.as_millis());
}

pub(super) struct Request<'a, 'slot, 'bytes> {
    pub plan: Arc<Plan>,
    pub session: Session,
    pub ctx: &'a mut Vec<u32>,
    pub tok: &'a GgufTokenizer,
    pub gate: &'a mut PublicationGate<'slot, 'bytes>,
    pub epoch: &'a crate::compute_universe::ComputeRequestEpoch,
    pub control: &'a Option<DecodeControl>,
    pub lp: &'a mut rtrb::Producer<LlmMsg>,
    pub cc: &'a mut rtrb::Consumer<SentMsg>,
    pub stream: &'a Option<std::sync::mpsc::SyncSender<String>>,
    pub prompt: &'a str,
    pub graph: u64,
    pub gen_budget: usize,
    pub stage_telemetry: bool,
    pub stage_trace: StageTrace,
    pub thinking_budget: Option<u32>,
}

pub(super) fn run(input: Request<'_, '_, '_>) -> WorkerResult {
    let Request {
        plan,
        mut session,
        ctx,
        tok,
        gate,
        epoch,
        control,
        lp,
        cc,
        stream,
        prompt,
        graph,
        gen_budget,
        stage_telemetry,
        mut stage_trace,
        thinking_budget,
    } = input;
    let hidden_len = plan.profile.hidden;
    let layers = plan.layers.len();
    let linear_layers = plan
        .layers
        .iter()
        .filter(|layer| {
            matches!(
                layer.attention,
                crate::gguf_bridge::qwen35::ftw::attention::AttentionPlan::Linear(_)
            )
        })
        .count();
    // Exporters may pad the projection/embedding rows beyond the tokenizer's
    // highest ID. Such rows remain numerical model data, never publishable IDs.
    let vocab = tok.vocab_len() as usize;
    if ctx.is_empty()
        || hidden_len == 0
        || hidden_len > 8192
        || gen_budget == 0
        || ctx
            .len()
            .checked_add(gen_budget - 1)
            .is_none_or(|n| n > plan.profile.max_context)
        || vocab == 0
        || vocab > plan.profile.vocab
    {
        gate.invalidate();
        return admission::refused(lp, stage_trace, PolicyError::Capacity);
    }
    if ctx.try_reserve_exact(gen_budget).is_err() {
        gate.invalidate();
        return admission::refused(lp, stage_trace, PolicyError::Budget);
    }
    // Session is fresh for every request. Failed/cancelled states are never
    // returned to the sticky engine or reused as a KV-only prefix.
    session.reset();
    let mut hidden = [0.0f32; 8192];
    let t_prefill = std::time::Instant::now();
    let timeout_ms = effective_inference_timeout_ms();
    let canceled = || {
        control.as_ref().is_some_and(DecodeControl::is_cancelled)
            || t_prefill.elapsed().as_millis() as u64 >= timeout_ms
    };
    for position in 0..ctx.len() - 1 {
        let Some(_pause) = epoch.pause() else {
            gate.invalidate();
            return admission::refused(lp, stage_trace, PolicyError::Revoked);
        };
        if lp.is_abandoned()
            || control.as_ref().is_some_and(DecodeControl::is_cancelled)
            || gate.checkpoint().is_err()
        {
            gate.invalidate();
            return admission::refused(lp, stage_trace, PolicyError::Revoked);
        }
        if let Err(error) = session.embed(ctx[position], &mut hidden[..hidden_len]) {
            numerical_failure(
                "prefill-embedding",
                position,
                error,
                t_prefill.elapsed(),
                timeout_ms,
            );
            gate.invalidate();
            return admission::refused(lp, stage_trace, PolicyError::InvalidCandidate);
        }
        if let Err(error) =
            session.prefill_controlled(&mut hidden[..hidden_len], position, &canceled)
        {
            numerical_failure("prefill", position, error, t_prefill.elapsed(), timeout_ms);
            gate.invalidate();
            return admission::refused(lp, stage_trace, PolicyError::InvalidCandidate);
        }
    }
    crate::llm_bench::record_prefill(
        t_prefill.elapsed().as_nanos() as u64,
        (ctx.len() - 1) as u64,
    );
    let t_decode = std::time::Instant::now();
    let mut out_ids = Vec::with_capacity(gen_budget);
    let mut streamed_len = 0;
    let mut semantic_quin = None;
    let mut sieve_failed = false;
    let mut no_sieve = None;
    let mut think_piece = [0; 64];
    let mut think_tracker = ThinkingTracker::new();
    for _ in 0..gen_budget {
        let Some(_pause) = epoch.pause() else {
            gate.invalidate();
            return admission::refused(lp, stage_trace, PolicyError::Revoked);
        };
        if lp.is_abandoned()
            || control.as_ref().is_some_and(DecodeControl::is_cancelled)
            || t_decode.elapsed().as_millis() as u64 >= effective_inference_timeout_ms()
            || gate.context_input().is_err()
            || (crate::inference_modes::sentinel_mid_decode_enabled() && cc.pop().is_ok())
        {
            gate.invalidate();
            return admission::refused(lp, stage_trace, PolicyError::Revoked);
        }
        let position = ctx.len() - 1;
        if let Err(error) = session.embed(ctx[position], &mut hidden[..hidden_len]) {
            numerical_failure(
                "decode-embedding",
                position,
                error,
                t_prefill.elapsed(),
                timeout_ms,
            );
            gate.invalidate();
            return admission::refused(lp, stage_trace, PolicyError::InvalidCandidate);
        }
        let result = session.step_controlled(&mut hidden[..hidden_len], position, &canceled);
        let (token, logit) = match result {
            Ok((token, logit)) if (token as usize) < vocab && logit.is_finite() => (token, logit),
            Err(error) => {
                numerical_failure("decode", position, error, t_prefill.elapsed(), timeout_ms);
                gate.invalidate();
                return admission::refused(lp, stage_trace, PolicyError::InvalidCandidate);
            }
            Ok((token, logit)) => {
                log::error!("LLM_DECODE|qwen35-output-invalid|position={position}|token={token}|logit={logit}|vocab={vocab}");
                #[cfg(test)]
                eprintln!("LLM_DECODE|qwen35-output-invalid|position={position}|token={token}|logit={logit}|vocab={vocab}");
                gate.invalidate();
                return admission::refused(lp, stage_trace, PolicyError::InvalidCandidate);
            }
        };
        if crate::inference_modes::sentinel_mid_decode_enabled() {
            let _ = lp.push(LlmMsg::Logit(LogitSummary {
                _top_id: token,
                anomaly: 1,
            }));
        }
        let (stop, next_streamed, next_failed, next_quin) =
            publication::run(publication::Publication {
                gate,
                top_i: token as usize,
                top_v: logit,
                vlen: vocab as u32,
                tok,
                sieve: &mut no_sieve,
                out_ids: &mut out_ids,
                ctx,
                prov_hash: graph,
                semantic_quin,
                sieve_failed,
                stage_telemetry,
                ssm_layer_count: linear_layers as u32,
                moe_layer_count: layers as u32,
                stage_trace: &mut stage_trace,
                track_thinking: stage_telemetry || thinking_budget.is_some(),
                think_piece: &mut think_piece,
                think_tracker: &mut think_tracker,
                thinking_budget,
                stream_tx_thread: stream,
                streamed_len,
                gen_budget,
            });
        streamed_len = next_streamed;
        sieve_failed = next_failed;
        semantic_quin = next_quin;
        if stop {
            break;
        }
    }
    render::run(render::Finalization {
        gate,
        t_decode,
        out_ids: &out_ids,
        stage_telemetry,
        stage_trace,
        lp,
        semantic_quin,
        sieve_failed,
        tok,
        prompt_owned: prompt,
    })
}
