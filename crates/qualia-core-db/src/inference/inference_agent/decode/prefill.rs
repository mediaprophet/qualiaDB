use super::*;
use crate::gguf_bridge::QTensorEngine;
use crate::gguf_sharder::GgufTensorIndex;

pub(super) struct Prefill<'a> {
    pub engine: &'a mut QTensorEngine,
    pub tensor_idx: &'a Option<GgufTensorIndex>,
    pub ctx: &'a Vec<u32>,
    pub emb_dim: usize,
    pub prefill_chunk: &'a mut [f32; crate::gguf_bridge::PREFILL_CHUNK_STACK_FLOATS],
    pub scratch_a: &'a mut [f32; 16384],
    pub scratch_b: &'a mut [f32; 16384],
    pub control_thread: &'a Option<DecodeControl>,
    pub prefix_cached: bool,
    pub prefill_tokens: usize,
    pub prompt_len: usize,
    pub kv_floats: usize,
    pub cache_key: super::super::prefix_cache::InferenceCacheKey,
    pub t_prefill: std::time::Instant,
    pub lp: &'a mut rtrb::Producer<LlmMsg>,
    pub stage_telemetry: bool,
    pub stage_trace: StageTrace,
}
pub(super) fn run(input: Prefill<'_>) -> Result<StageTrace, WorkerResult> {
    let Prefill {
        engine,
        tensor_idx,
        ctx,
        emb_dim,
        prefill_chunk,
        scratch_a,
        scratch_b,
        control_thread,
        prefix_cached,
        prefill_tokens,
        prompt_len,
        kv_floats,
        cache_key,
        t_prefill,
        lp,
        stage_telemetry,
        mut stage_trace,
    } = input;
    let prefill_outcome = if !prefix_cached {
        super::super::prefill_executor::execute_chunked_prefill(
            engine,
            tensor_idx.as_ref(),
            &ctx,
            emb_dim,
            prefill_chunk,
            scratch_a,
            scratch_b,
            control_thread.as_ref(),
            TEST_TRANSFORMER_LAYER_CAP,
        )
    } else {
        super::super::prefill_executor::PrefillOutcome::Completed {
            tokens_processed: prefill_tokens,
        }
    };

    match prefill_outcome {
        super::super::prefill_executor::PrefillOutcome::Cancelled { tokens_processed } => {
            let _ = lp.push(LlmMsg::Eos);
            crate::llm_bench::record_prefill(
                t_prefill.elapsed().as_nanos() as u64,
                tokens_processed as u64,
            );
            return Err((String::from("[cancelled]"), 0u32, None, false, stage_trace));
        }
        super::super::prefill_executor::PrefillOutcome::Failed {
            pos,
            tokens_processed,
            reason,
        } => {
            eprintln!(
                "[decode] Prefill failed at token pos {} (processed {}): {:?}",
                pos, tokens_processed, reason
            );
            let _ = lp.push(LlmMsg::Eos);
            crate::llm_bench::record_prefill(
                t_prefill.elapsed().as_nanos() as u64,
                tokens_processed as u64,
            );
            return Err((
                String::from("[prefill-failed]"),
                0u32,
                None,
                false,
                stage_trace,
            ));
        }
        super::super::prefill_executor::PrefillOutcome::Completed { .. } => {
            if !prefix_cached && kv_floats > 0 && crate::llm_bench::prefix_cache_enabled() {
                if let Some(cpu_kv) = engine.get_kv_cache_cpu() {
                    if let Ok(mut cache) = get_prefix_cache().lock() {
                        if cpu_kv.len() * std::mem::size_of::<f32>() <= cache.max_bytes() {
                            let _ = cache.insert_with_prefix(
                                cache_key,
                                &ctx[..prefill_tokens],
                                cpu_kv.to_vec().into_boxed_slice(),
                                prefill_tokens as u32,
                                true,
                            );
                        }
                    }
                }
            }
        }
        super::super::prefill_executor::PrefillOutcome::NoPrefillNeeded => {}
    }

    // Phase boundary: prefill done. Decode phase begins below.
    crate::llm_bench::record_prefill(
        t_prefill.elapsed().as_nanos() as u64,
        prompt_len.saturating_sub(1) as u64,
    );
    if stage_telemetry {
        stage_trace.record(
            InferenceStageKind::ChunkedPrefill,
            if prefix_cached {
                StageStatus::Bypassed
            } else {
                StageStatus::Executed
            },
            u32::try_from(t_prefill.elapsed().as_micros()).unwrap_or(u32::MAX),
            prefill_tokens as u32,
        );
    }

    Ok(stage_trace)
}
