//! Client-facing auditory Ears MVP + later swarm helpers.

use qualia_audio::cross_modal::{
    frames_to_media_ms, propose_temporal_correlations, TimeIntervalMs,
};
use qualia_audio::generation::{synthesize_reference_tone, VoiceConsent};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use qualia_audio::capture::{CapturePurpose, CaptureSession};
use qualia_audio::pipeline::{
    run_ears_demo, run_ears_on_wav_file, run_ears_weighted, section18_smoke, sonify_demo_to_wav,
    speech_phone_demo, EarsDemoResult,
};
use qualia_audio::semantic::{human_correct_quin, human_reject_quin};
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
    pub cqt_peak: f32,
    pub event_instance_hashes: Vec<String>,
    pub note: String,
}

impl From<EarsDemoResult> for EarsDemoDto {
    fn from(r: EarsDemoResult) -> Self {
        let instances: Vec<String> = r
            .events
            .iter()
            .map(|e| {
                format!(
                    "0x{:016x}",
                    e.source_hash ^ e.start_frame ^ e.end_frame.wrapping_mul(0x9e37_79b9)
                )
            })
            .collect();
        Self {
            sample_rate: r.sample_rate,
            frames: r.frames,
            n_events: r.n_events,
            n_quins: r.n_quins,
            model_hash: format!("0x{:016x}", r.model_hash),
            media_hash: format!("0x{:016x}", r.media_hash),
            is_reference: r.is_reference,
            mel_frames: r.mel_frames,
            cqt_peak: r.cqt_peak,
            event_instance_hashes: instances,
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

pub fn ears_from_wav(
    storage_root: Option<&std::path::Path>,
    path: &std::path::Path,
) -> Result<EarsDemoDto, String> {
    run_ears_on_wav_file(storage_root, path).map(Into::into)
}

pub fn section18_smoke_dto() -> Result<String, String> {
    section18_smoke()
}

pub fn audio_reject_instance(instance_hex: &str) -> Result<String, String> {
    let inst = parse_hex(instance_hex)?;
    let q = human_reject_quin(
        qualia_audio::q_hash("did:webizen:local-principal"),
        inst,
        0,
    );
    Ok(format!(
        "reject_quin parity=0x{:016x} instance=0x{:016x} (machine claim retained)",
        q.parity, inst
    ))
}

pub fn audio_correct_instance(instance_hex: &str, new_class_hex: &str) -> Result<String, String> {
    let inst = parse_hex(instance_hex)?;
    let cls = parse_hex(new_class_hex)?;
    let q = human_correct_quin(
        qualia_audio::q_hash("did:webizen:local-principal"),
        inst,
        cls,
    );
    Ok(format!(
        "correct_quin parity=0x{:016x} new_class=0x{:016x}",
        q.parity, cls
    ))
}

fn parse_hex(s: &str) -> Result<u64, String> {
    let t = s.trim().trim_start_matches("0x").trim_start_matches("0X");
    u64::from_str_radix(t, 16).map_err(|e| e.to_string())
}

pub fn ears_weighted_demo(storage_root: Option<&std::path::Path>) -> Result<EarsDemoDto, String> {
    run_ears_weighted(storage_root, 440.0, 16000, 300).map(Into::into)
}

/// U3-style hear: demo events → WAV data URL + optional file under storage.
pub fn sonify_ears_demo(
    storage_root: Option<&std::path::Path>,
) -> Result<serde_json::Value, String> {
    let r = run_ears_demo(None, 440.0, 16000, 400)?;
    let wav = sonify_demo_to_wav(r.sample_rate, &r.events, r.frames as usize)?;
    let mut path_out = None;
    if let Some(root) = storage_root {
        let dir = root.join("audio_hear");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let p = dir.join(format!("sonify_{:016x}.wav", r.media_hash));
        std::fs::write(&p, &wav).map_err(|e| e.to_string())?;
        path_out = Some(p.display().to_string());
    }
    Ok(serde_json::json!({
        "wav_data_url": format!("data:audio/wav;base64,{}", B64.encode(&wav)),
        "path": path_out,
        "n_events": r.n_events,
        "frames": r.frames,
        "sample_rate": r.sample_rate,
        "note": "Parametric sonification of event intervals (U3-style navigate/hear). Not original PCM playback."
    }))
}

pub fn speech_demo(supported: bool) -> Result<serde_json::Value, String> {
    let (n, model) = speech_phone_demo(supported)?;
    Ok(serde_json::json!({
        "tokens": n,
        "model_hash": format!("0x{:016x}", model),
        "language_supported": supported,
        "note": if supported {
            "Greedy phone decode over seed speech weights (not full ASR)."
        } else {
            "Unknown language: empty transcript (no silent map)."
        }
    }))
}

/// Capture policy demo: intent required before live ring accepts PCM.
pub fn capture_policy_demo() -> Result<serde_json::Value, String> {
    let mut s = CaptureSession::new(CapturePurpose::Analysis, 16000, 1);
    let denied = s.start().is_err();
    s.grant_intent();
    s.start().map_err(|e| format!("{e:?}"))?;
    let pushed = s.push_mono(&[0.1, 0.2, 0.3, 0.0]);
    let mut out = [0.0f32; 8];
    let pulled = s.pull_mono(&mut out);
    Ok(serde_json::json!({
        "denied_without_intent": denied,
        "pushed": pushed,
        "pulled": pulled,
        "note": "Shell must call grant_intent before device/file stream. Hardware mic via shell push_mono."
    }))
}

/// Seed and persist AED weights under storage_root/models/aed_seed.qaed
pub fn ensure_aed_weights(storage_root: &std::path::Path) -> Result<String, String> {
    let path = storage_root.join("models").join("aed_seed.qaed");
    if path.is_file() {
        let b = qualia_audio::AedWeightBundle::load_path(&path)?;
        return Ok(format!(
            "loaded AED weights hash=0x{:016x} path={}",
            b.model_hash,
            path.display()
        ));
    }
    let b = qualia_audio::AedWeightBundle::from_seed(0xAED1);
    b.save_path(&path)?;
    Ok(format!(
        "wrote AED seed weights hash=0x{:016x} path={}",
        b.model_hash,
        path.display()
    ))
}

pub fn ensure_speech_weights(storage_root: &std::path::Path) -> Result<String, String> {
    let path = storage_root.join("models").join("speech_seed.qspk");
    if path.is_file() {
        let w = qualia_audio::SpeechEncoderWeights::load_path(&path)?;
        return Ok(format!(
            "loaded speech weights hash=0x{:016x} path={}",
            w.model_hash,
            path.display()
        ));
    }
    let w = qualia_audio::SpeechEncoderWeights::from_seed(7, 16);
    w.save_path(&path)?;
    Ok(format!(
        "wrote speech seed weights hash=0x{:016x} path={}",
        w.model_hash,
        path.display()
    ))
}

pub fn daw_history_demo() -> Result<serde_json::Value, String> {
    use qualia_audio::{OpKind, ProcessPlan, SessionHistory, SessionOp, TrackState};
    let mut plan = ProcessPlan::new(48000, 64);
    plan.add_track(TrackState::default());
    let mut hist = SessionHistory::new();
    hist.apply_and_record(
        &mut plan,
        SessionOp {
            kind: OpKind::SetGain,
            track: 0,
            value_f32: 0.4,
            value_bool: false,
            prev_f32: 1.0,
            prev_bool: false,
        },
    );
    let g1 = plan.tracks[0].gain;
    hist.undo(&mut plan);
    let g0 = plan.tracks[0].gain;
    hist.redo(&mut plan);
    let g2 = plan.tracks[0].gain;
    let mut lane = qualia_audio::AutomationLane::new(0);
    lane.add(0, 0.0);
    lane.add(1000, 1.0);
    let mid = lane.value_at(500);
    Ok(serde_json::json!({
        "gain_after_set": g1,
        "gain_after_undo": g0,
        "gain_after_redo": g2,
        "automation_mid": mid,
        "note": "SessionHistory undo/redo + AutomationLane interp (cold path)."
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ears_dto() {
        let d = ears_demo(None).unwrap();
        assert!(d.n_events >= 1);
        assert!(d.is_reference);
        assert!(d.cqt_peak > 0.0);
    }

    #[test]
    fn cross_modal_not_causal() {
        let d = cross_modal_demo();
        assert!(!d.asserts_causality_any);
    }

    #[test]
    fn section18_ok() {
        assert!(section18_smoke_dto().unwrap().contains("OK"));
    }
}
