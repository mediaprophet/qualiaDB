//! Client-facing auditory Ears MVP + later swarm helpers.

use qualia_audio::cross_modal::{
    frames_to_media_ms, propose_temporal_correlations, TimeIntervalMs,
};
use qualia_audio::generation::{synthesize_reference_tone, VoiceConsent};
use qualia_audio::pipeline::{run_ears_demo, EarsDemoResult};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct EarsDemoDto {
    pub sample_rate: u32,
    pub frames: u32,
    pub n_events: usize,
    pub n_quins: usize,
    pub model_hash: String,
    pub media_hash: String,
    pub is_reference: bool,
    pub mel_frames: usize,
    pub note: String,
}

impl From<EarsDemoResult> for EarsDemoDto {
    fn from(r: EarsDemoResult) -> Self {
        Self {
            sample_rate: r.sample_rate,
            frames: r.frames,
            n_events: r.n_events,
            n_quins: r.n_quins,
            model_hash: format!("0x{:016x}", r.model_hash),
            media_hash: format!("0x{:016x}", r.media_hash),
            is_reference: r.is_reference,
            mel_frames: r.mel_frames,
            note: r.note,
        }
    }
}

pub fn ears_demo(storage_root: Option<&std::path::Path>) -> Result<EarsDemoDto, String> {
    run_ears_demo(storage_root, 440.0, 16000, 300).map(Into::into)
}

#[derive(Debug, Clone, Serialize)]
pub struct CrossModalDemoDto {
    pub n_correlations: usize,
    pub asserts_causality_any: bool,
    pub note: String,
}

pub fn cross_modal_demo() -> CrossModalDemoDto {
    let v = [TimeIntervalMs {
        start_ms: 0,
        end_ms: 2000,
        instance: 0x15,
    }];
    let a_start = frames_to_media_ms(8000, 16000, 0);
    let a = [TimeIntervalMs {
        start_ms: a_start,
        end_ms: a_start + 1000,
        instance: 0xA0,
    }];
    let mut out = [qualia_audio::AvCorrelationProposal {
        media_hash: 1,
        visual_instance: 0,
        auditory_instance: 0,
        overlap_start_ms: 0,
        overlap_end_ms: 0,
        confidence_u16: 0,
        asserts_causality: true,
    }; 8];
    let n = propose_temporal_correlations(1, &v, &a, &mut out);
    CrossModalDemoDto {
        n_correlations: n,
        asserts_causality_any: out[..n].iter().any(|c| c.asserts_causality),
        note: "Temporal overlap only — asserts_causality always false.".into(),
    }
}

pub fn synth_with_consent(allow: bool) -> Result<u32, String> {
    let c = if allow {
        VoiceConsent::synthesis_only("demo-voice")
    } else {
        VoiceConsent::denied("demo-voice")
    };
    let mut o = [0.0f32; 512];
    let r =
        synthesize_reference_tone(c, 440.0, 16000, 512, 1, &mut o).map_err(|e| format!("{e:?}"))?;
    Ok(r.frames)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ears_dto() {
        let d = ears_demo(None).unwrap();
        assert!(d.n_events >= 1);
        assert!(d.is_reference);
    }

    #[test]
    fn cross_modal_not_causal() {
        let d = cross_modal_demo();
        assert!(!d.asserts_causality_any);
    }
}
