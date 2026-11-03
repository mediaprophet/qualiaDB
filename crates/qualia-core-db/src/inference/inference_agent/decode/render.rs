use super::*;

use crate::gguf_sharder::GgufTokenizer;

pub(super) struct Finalization<'a> {
    pub t_decode: std::time::Instant,
    pub out_ids: &'a Vec<u32>,
    pub stage_telemetry: bool,
    pub stage_trace: StageTrace,
    pub lp: &'a mut rtrb::Producer<LlmMsg>,
    pub semantic_quin: Option<NQuin>,
    pub sieve_failed: bool,
    pub tok: &'a GgufTokenizer,
    pub prompt_owned: &'a str,
}
pub(super) fn run(input: Finalization<'_>) -> WorkerResult {
    let Finalization {
        t_decode,
        out_ids,
        stage_telemetry,
        mut stage_trace,
        lp,
        semantic_quin,
        sieve_failed,
        tok,
        prompt_owned,
    } = input;
    // Phase boundary: decode loop complete.
    crate::llm_bench::record_decode(t_decode.elapsed().as_nanos() as u64, out_ids.len() as u64);

    if stage_telemetry {
        stage_trace.record(
            InferenceStageKind::AutoregressiveDecode,
            StageStatus::Executed,
            u32::try_from(t_decode.elapsed().as_micros()).unwrap_or(u32::MAX),
            out_ids.len() as u32,
        );
        if stage_trace
            .get(InferenceStageKind::SentinelMidDecodeGuard)
            .is_none()
        {
            stage_trace.record(
                InferenceStageKind::SentinelMidDecodeGuard,
                if crate::inference_modes::sentinel_mid_decode_enabled() {
                    StageStatus::Executed
                } else {
                    StageStatus::Bypassed
                },
                0,
                out_ids.len() as u32,
            );
        }
    }

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
            crate::quant_graph_grounding::maybe_ground_generation(&prompt_owned, &raw).text
        }
    };
    (
        text,
        out_ids.len() as u32,
        semantic_quin,
        sieve_failed,
        stage_trace,
    )
}
