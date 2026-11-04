use super::*;

use crate::gguf_sharder::GgufTokenizer;

pub(super) struct Finalization<'a, 'slot, 'bytes> {
    pub gate: &'a mut super::super::native_request::PublicationGate<'slot, 'bytes>,
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
pub(super) fn run(input: Finalization<'_, '_, '_>) -> WorkerResult {
    let Finalization {
        gate,
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
    if lp.is_abandoned() {
        gate.invalidate();
    }
    if let Err(error) = gate.checkpoint() {
        return admission::refused(lp, stage_trace, error);
    }
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
                if gate.is_governed() || crate::inference_modes::sentinel_mid_decode_enabled() {
                    StageStatus::Executed
                } else {
                    StageStatus::Bypassed
                },
                0,
                out_ids.len() as u32,
            );
        }
    }

    let mut semantic_quin = semantic_quin;
    if let Some(q) = semantic_quin {
        let mut bytes = [0u8; 48];
        for (dst, word) in bytes.chunks_exact_mut(8).zip([
            q.subject,
            q.predicate,
            q.object,
            q.context,
            q.metadata,
            q.parity,
        ]) {
            dst.copy_from_slice(&word.to_le_bytes())
        }
        if gate
            .artifact_in_place(
                super::super::governed::evidence::CandidateSource::SemanticArtifact,
                super::super::governed::evidence::ArtifactOrigin::SemanticProjection,
                super::super::governed::evidence::ArtifactMedia::CanonicalQuins,
                &mut bytes,
            )
            .is_err()
        {
            semantic_quin = None;
        } else {
            let words: [u64; 6] = std::array::from_fn(|i| {
                u64::from_le_bytes(bytes[i * 8..i * 8 + 8].try_into().expect("canonical word"))
            });
            semantic_quin = Some(NQuin {
                subject: words[0],
                predicate: words[1],
                object: words[2],
                context: words[3],
                metadata: words[4],
                parity: words[5],
            });
        }
    }
    let text = if semantic_quin.is_some() {
        String::new()
    } else if sieve_failed {
        String::from("[sieve-misaligned]")
    } else {
        let raw = tok.decode(&out_ids);
        // Post-turn path (FastVerify / QuantGraph): generate full draft at
        // full speed, then graph/CML self-heal + optional HTML surface.
        let (rendered, html) = if crate::inference_modes::post_turn_verify_enabled() {
            let v = crate::post_turn_verify::verify_and_heal_turn(&prompt_owned, &raw);
            if crate::post_turn_verify::return_html_as_text() {
                (v.display_html, true)
            } else {
                (v.final_text, false)
            }
        } else {
            (
                crate::quant_graph_grounding::maybe_ground_generation(&prompt_owned, &raw).text,
                false,
            )
        };
        if rendered == raw {
            raw
        } else {
            use super::super::governed::evidence::{
                ArtifactMedia, ArtifactOrigin, CandidateSource,
            };
            let source = if html {
                CandidateSource::HtmlReplacement
            } else {
                CandidateSource::TextReplacement
            };
            let media = if html {
                ArtifactMedia::Utf8Html
            } else {
                ArtifactMedia::Utf8Text
            };
            let mut rendered = rendered.into_bytes();
            if gate
                .artifact_in_place(
                    source,
                    ArtifactOrigin::GroundedReplacement,
                    media,
                    &mut rendered,
                )
                .is_ok()
            {
                String::from_utf8(rendered).expect("accepted original UTF-8 artifact")
            } else {
                raw
            }
        }
    };
    if lp.is_abandoned() {
        gate.invalidate();
    }
    if let Err(error) = gate.checkpoint() {
        return admission::refused(lp, stage_trace, error);
    }
    let _ = lp.push(LlmMsg::Eos);
    (
        text,
        out_ids.len() as u32,
        semantic_quin,
        sieve_failed,
        stage_trace,
    )
}
