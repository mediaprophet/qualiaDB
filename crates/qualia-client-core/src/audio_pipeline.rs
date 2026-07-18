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

/// Honest, machine-readable status of one audio capability (for Listen UI honesty chips).
#[derive(Debug, Clone, Serialize)]
pub struct AudioCapabilityDto {
    pub id: String,
    pub domain: String,
    pub status: String,
    pub zero_heap_hot: bool,
    pub streaming: bool,
    pub test_name: String,
    pub note: String,
}

/// Snapshot the audio capability registry as serializable DTOs.
pub fn audio_capabilities() -> Vec<AudioCapabilityDto> {
    qualia_audio::capability_registry::CAPABILITIES
        .iter()
        .map(|c| AudioCapabilityDto {
            id: c.id.to_string(),
            domain: c.domain.as_str().to_string(),
            status: c.status.as_str().to_string(),
            zero_heap_hot: c.zero_heap_hot,
            streaming: c.streaming,
            test_name: c.test_name.to_string(),
            note: c.note.to_string(),
        })
        .collect()
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

/// Run weighted AED on mono PCM (from mic pull or file).
pub fn analyze_mono_pcm(
    mono: &[f32],
    sample_rate: u32,
    storage_root: Option<&std::path::Path>,
) -> Result<EarsDemoDto, String> {
    use qualia_audio::types::{AuditoryEvent, AuditoryModel, TranscriptToken};
    use qualia_audio::wav::encode_wav_i16_mono;
    use qualia_audio::{AedWeightBundle, WeightedAedModel};

    if mono.is_empty() {
        return Err("no PCM (arm mic or import WAV first)".into());
    }
    let mut i16s = vec![0i16; mono.len()];
    for (i, &s) in mono.iter().enumerate() {
        i16s[i] = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
    }
    let mut wav = vec![0u8; 44 + mono.len() * 2];
    let wn = encode_wav_i16_mono(&i16s, sample_rate.max(8000), &mut wav)
        .map_err(|e| format!("{e:?}"))?;
    let decoded = qualia_audio::decode_wav(&wav[..wn]).map_err(|e| format!("{e:?}"))?;

    let bundle = if let Some(root) = storage_root {
        let path = root.join("models").join("aed_seed.qaed");
        if path.is_file() {
            AedWeightBundle::load_path(&path)?
        } else {
            let b = AedWeightBundle::from_seed(0xAED1);
            let _ = b.save_path(&path);
            b
        }
    } else {
        AedWeightBundle::from_seed(0xAED1)
    };

    let mut model = WeightedAedModel::from_bundle(bundle);

    let mut events = [AuditoryEvent::empty(); 32];
    let mut tokens = [TranscriptToken::empty(); 8];
    let mut emb = [0.0f32; 4];
    let mut ws = [0u8; 8];
    let counts = model
        .infer_chunk(decoded.view(), &mut events, &mut tokens, &mut emb, &mut ws)
        .map_err(|e| format!("{e:?}"))?;

    let media_hash = qualia_audio::media_digest(&wav[..wn]).hash;
    let mut quins = [qualia_audio::AudioQuin::with_parity(0, 0, 0, 0, 0); 64];
    let n_q = qualia_audio::compile_auditory_quins(
        qualia_audio::MediaDigest {
            hash: media_hash,
            byte_len: wn as u64,
        },
        &events[..counts.events],
        model.model_hash(),
        &mut quins,
    );

    Ok(EarsDemoDto {
        sample_rate: decoded.sample_rate,
        frames: mono.len() as u32,
        n_events: counts.events,
        n_quins: n_q,
        model_hash: format!("0x{:016x}", model.model_hash()),
        media_hash: format!("0x{:016x}", media_hash),
        is_reference: false,
        mel_frames: 0,
        cqt_peak: 0.0,
        event_instance_hashes: events[..counts.events]
            .iter()
            .map(|e| format!("0x{:016x}", e.source_hash ^ e.start_frame))
            .collect(),
        note: "Live/imported mono analyzed with disk AED weights (seed-shaped, not certified foundation)."
            .into(),
    })
}

/// One mixer track strip (UI ↔ process plan).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MixerTrackDto {
    pub name: String,
    pub gain: f32,
    pub pan: f32,
    pub mute: bool,
    pub solo: bool,
    pub lowpass: f32,
    pub eq_gain_db: f32,
    pub eq_freq_hz: f32,
    pub comp_threshold: f32,
    pub comp_ratio: f32,
    pub delay_samples: u32,
    pub delay_mix: f32,
}

impl Default for MixerTrackDto {
    fn default() -> Self {
        Self {
            name: "Track".into(),
            gain: 0.85,
            pan: 0.0,
            mute: false,
            solo: false,
            lowpass: 0.0,
            eq_gain_db: 0.0,
            eq_freq_hz: 1000.0,
            comp_threshold: 1.0,
            comp_ratio: 1.0,
            delay_samples: 0,
            delay_mix: 0.0,
        }
    }
}

/// Default 3-strip mixer session for Listen UI.
pub fn mixer_default_session() -> serde_json::Value {
    let tracks = vec![
        MixerTrackDto {
            name: "Tone A".into(),
            pan: -0.4,
            ..Default::default()
        },
        MixerTrackDto {
            name: "Tone B".into(),
            pan: 0.4,
            gain: 0.7,
            ..Default::default()
        },
        MixerTrackDto {
            name: "Pad".into(),
            gain: 0.5,
            lowpass: 0.25,
            ..Default::default()
        },
    ];
    serde_json::json!({
        "sample_rate": 48000,
        "block_frames": 64,
        "tracks": tracks,
        "note": "Reference mixer — not a commercial DAW. EQ/comp/delay are deterministic primitives."
    })
}

/// Bounce a mixer session (synthetic tones per track) through ProcessPlan FX chain.
pub fn mixer_bounce(tracks: &[MixerTrackDto]) -> Result<serde_json::Value, String> {
    use qualia_audio::{ProcessPlan, TrackState};
    let n = tracks.len().min(16).max(1);
    let frames = 2048usize;
    let sr = 48000u32;
    let mut plan = ProcessPlan::new(sr, 64);
    let mut mono_bufs: Vec<Vec<f32>> = Vec::with_capacity(n);
    for (i, t) in tracks.iter().take(n).enumerate() {
        plan.add_track(TrackState {
            gain: t.gain.clamp(0.0, 2.0),
            pan: t.pan.clamp(-1.0, 1.0),
            mute: t.mute,
            solo: t.solo,
            lowpass: t.lowpass.clamp(0.0, 0.99),
            eq_gain_db: t.eq_gain_db,
            eq_freq_hz: t.eq_freq_hz.max(20.0),
            comp_threshold: t.comp_threshold.clamp(0.05, 1.0),
            comp_ratio: t.comp_ratio.max(1.0),
            delay_samples: t.delay_samples.min(512),
            delay_mix: t.delay_mix.clamp(0.0, 1.0),
        });
        let freq = 220.0 * (i as f32 + 1.0);
        let mut buf = vec![0.0f32; frames];
        for (s, sample) in buf.iter_mut().enumerate() {
            *sample = (2.0 * core::f32::consts::PI * freq * s as f32 / sr as f32).sin() * 0.4;
        }
        mono_bufs.push(buf);
    }
    let refs: Vec<&[f32]> = mono_bufs.iter().map(|b| b.as_slice()).collect();
    let mut out = vec![0.0f32; frames * 2];
    let written = plan
        .bounce_interleaved(&refs, &mut out)
        .map_err(|e| e.to_string())?;
    let peak = out
        .iter()
        .take(written * 2)
        .map(|x| x.abs())
        .fold(0.0f32, f32::max);
    let energy: f32 = out.iter().take(written * 2).map(|x| x * x).sum();
    Ok(serde_json::json!({
        "frames_written": written,
        "peak": peak,
        "energy": energy,
        "n_tracks": n,
        "note": "Offline bounce of synthetic tones through mixer FX — reference quality only."
    }))
}

/// Music analysis demo: onsets + tempo + structure (reference quality).
pub fn music_analysis_demo() -> Result<serde_json::Value, String> {
    use qualia_audio::{
        detect_onsets, estimate_tempo_from_onsets, propose_structure_segments, MusicAssumptions,
        OnsetEvent, StructureSegment,
    };
    let sr = 16000u32;
    let mut mono = vec![0.0f32; 8000];
    // Impulses ~2 Hz → ~120 BPM when folded
    for k in 0..8 {
        let i = (k * 4000 / 2) as usize;
        if i < mono.len() {
            mono[i] = 1.0;
        }
    }
    let mut onsets = [OnsetEvent {
        frame: 0,
        strength: 0.0,
    }; 32];
    let n_on = detect_onsets(&mono, 256, 128, 0.01, &mut onsets);
    let tempo = estimate_tempo_from_onsets(
        &onsets[..n_on],
        sr,
        MusicAssumptions {
            assumes_4_4: true,
            assumes_12tet: false,
            tuning_a4_hz: 440.0,
        },
    );
    let mut segs = [StructureSegment {
        start_frame: 0,
        end_frame: 0,
        mean_energy: 0.0,
        label_hash: 0,
    }; 8];
    let n_seg = propose_structure_segments(&mono, 256, 128, &mut segs);
    Ok(serde_json::json!({
        "n_onsets": n_on,
        "bpm": tempo.bpm,
        "tempo_confidence": tempo.confidence,
        "n_segments": n_seg,
        "note": "Reference music analysis — not a production beat tracker."
    }))
}

/// DAW FX chain demo (EQ + comp + delay on mono).
pub fn daw_fx_demo() -> Result<serde_json::Value, String> {
    use qualia_audio::{ProcessPlan, TrackState};
    let mut tr = TrackState::default();
    tr.eq_gain_db = 3.0;
    tr.comp_threshold = 0.3;
    tr.comp_ratio = 3.0;
    tr.delay_samples = 48;
    tr.delay_mix = 0.2;
    let mono: Vec<f32> = (0..512)
        .map(|i| (2.0 * core::f32::consts::PI * 440.0 * i as f32 / 48000.0).sin() * 0.5)
        .collect();
    let mut out = vec![0.0f32; 512];
    let n = ProcessPlan::process_mono_fx(&tr, 48000, &mono, &mut out);
    let energy: f32 = out.iter().map(|x| x * x).sum();
    Ok(serde_json::json!({
        "frames": n,
        "energy": energy,
        "note": "EQ+comp+delay offline path (deterministic FX primitives)."
    }))
}

/// TTS consent + stem separation demo.
pub fn gen_audio_demo() -> Result<serde_json::Value, String> {
    use qualia_audio::{
        separate_two_stems_reference, synthesize_reference_tone, VoiceConsent,
    };
    let mut consent = VoiceConsent::synthesis_only("demo-voice");
    let mut tone = [0.0f32; 512];
    let rec = synthesize_reference_tone(consent, 440.0, 16000, 512, 7, &mut tone)
        .map_err(|e| format!("{e:?}"))?;
    consent.revoke(1);
    let denied = synthesize_reference_tone(consent, 440.0, 16000, 512, 7, &mut tone).is_err();
    let mut body = [0.0f32; 512];
    let mut detail = [0.0f32; 512];
    let (s0, s1) = separate_two_stems_reference(&tone, 0x00E0_D1A0_u64, &mut body, &mut detail)
        .map_err(|e| format!("{e:?}"))?;
    Ok(serde_json::json!({
        "synth_frames": rec.frames,
        "is_reference_synth": rec.is_reference_synth,
        "revoke_denies": denied,
        "stem_body": format!("0x{:016x}", s0.stem_class),
        "stem_detail": format!("0x{:016x}", s1.stem_class),
        "note": "Reference synth + sep; licensed TTS/demucs COMPLETE-WITH-GATE."
    }))
}

/// Shared media clock + joint window demo.
pub fn shared_clock_demo() -> Result<serde_json::Value, String> {
    use qualia_audio::{
        events_overlapping_window, SharedMediaClock, TimeIntervalMs,
    };
    let clock = SharedMediaClock::new(0xC10C, 16000, 25.0);
    let v_ms = clock.video_frame_to_ms(25);
    let a_ms = clock.audio_frame_to_ms(16000);
    let intervals = [
        TimeIntervalMs {
            start_ms: 0,
            end_ms: 500,
            instance: 1,
        },
        TimeIntervalMs {
            start_ms: 800,
            end_ms: 1200,
            instance: 2,
        },
    ];
    let win = TimeIntervalMs {
        start_ms: 400,
        end_ms: 900,
        instance: 0,
    };
    let mut out = [0u64; 4];
    let n = events_overlapping_window(&intervals, win, &mut out);
    Ok(serde_json::json!({
        "video_25_frames_ms": v_ms,
        "audio_1s_ms": a_ms,
        "drift_at_1s": clock.drift_ms(25, 16000),
        "window_hits": n,
        "asserts_causality": false,
        "note": "Shared clock + joint interval query; overlap ≠ causality."
    }))
}

/// Speech using disk weights if present.
pub fn speech_from_disk(
    storage_root: &std::path::Path,
    supported: bool,
) -> Result<serde_json::Value, String> {
    use qualia_audio::{decode_for_language, SpeechEncoderWeights, TranscriptToken};
    let path = storage_root.join("models").join("speech_seed.qspk");
    let w = if path.is_file() {
        SpeechEncoderWeights::load_path(&path)?
    } else {
        let w = SpeechEncoderWeights::from_seed(7, 16);
        w.save_path(&path)?;
        w
    };
    let mut mono = vec![0.0f32; 4096];
    for i in 0..mono.len() {
        mono[i] = (2.0 * core::f32::consts::PI * 180.0 * i as f32 / 16000.0).sin() * 0.25;
    }
    let mut tok = [TranscriptToken::empty(); 32];
    let n = decode_for_language(&w, &mono, 16000, supported, &mut tok)
        .map_err(|e| format!("{e:?}"))?;
    Ok(serde_json::json!({
        "tokens": n,
        "model_hash": format!("0x{:016x}", w.model_hash),
        "weights_path": path.display().to_string(),
        "language_supported": supported,
        "note": "Speech weights loaded from disk when present."
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

    #[test]
    fn analyze_mono_pcm_seed() {
        let mut mono = vec![0.0f32; 4096];
        for i in 0..mono.len() {
            mono[i] = (2.0 * core::f32::consts::PI * 440.0 * i as f32 / 16000.0).sin() * 0.3;
        }
        let d = analyze_mono_pcm(&mono, 16000, None).unwrap();
        assert_eq!(d.sample_rate, 16000);
        assert!(d.frames > 0);
        assert!(!d.model_hash.is_empty());
    }
}
