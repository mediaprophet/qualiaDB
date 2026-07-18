//! End-to-end Ears MVP demo path (A5 product surface).

use crate::aed_weights::WeightedAedModel;
use crate::convert::to_mono_f32;
use crate::events::ReferenceEventModel;
use crate::features::{forward_cqt_mono, log_mel_from_mono};
use crate::hash::media_digest;
use crate::media_store::{synth_tone_f32, AudioMediaStore, RetentionClass};
use crate::semantic::compile_auditory_quins;
use crate::sonify::sonify_events_mono;
use crate::speech::{decode_for_language, SpeechEncoderWeights};
use crate::types::{AuditoryEvent, AuditoryModel, TranscriptToken};
use crate::wav::{decode_wav, encode_wav_i16_mono};

#[derive(Debug, Clone)]
pub struct EarsDemoResult {
    pub sample_rate: u32,
    pub frames: u32,
    pub n_events: usize,
    pub n_quins: usize,
    pub model_hash: u64,
    pub media_hash: u64,
    pub is_reference: bool,
    pub mel_frames: usize,
    pub cqt_bins: usize,
    pub cqt_peak: f32,
    pub event_classes: Vec<u64>,
    pub events: Vec<AuditoryEvent>,
    pub note: String,
}

/// Run synthetic tone → WAV → store → features → events → semantic compile.
pub fn run_ears_demo(
    storage_root: Option<&std::path::Path>,
    freq_hz: f32,
    sample_rate: u32,
    duration_ms: u32,
) -> Result<EarsDemoResult, String> {
    let frames = ((sample_rate as u64 * duration_ms as u64) / 1000) as usize;
    let frames = frames.clamp(1024, 48000);
    let mono = synth_tone_f32(freq_hz, sample_rate, frames, 0.35);
    let mut i16s = vec![0i16; frames];
    for (i, &s) in mono.iter().enumerate() {
        i16s[i] = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
    }
    let mut wav = vec![0u8; 44 + frames * 2];
    let wn = encode_wav_i16_mono(&i16s, sample_rate, &mut wav).map_err(|e| format!("{e:?}"))?;
    wav.truncate(wn);

    let mut media_hash = media_digest(&wav).hash;
    if let Some(root) = storage_root {
        let store = AudioMediaStore::open(root.join("audio_media")).map_err(|e| e)?;
        let rec = store
            .import_bytes(&wav, sample_rate, 1, frames as u32, RetentionClass::Restricted)
            .map_err(|e| e)?;
        media_hash = rec.digest_u64;
    }

    let decoded = decode_wav(&wav).map_err(|e| format!("{e:?}"))?;
    let view = decoded.view();
    let mut mono2 = vec![0.0f32; frames];
    to_mono_f32(view, &mut mono2).map_err(|e| format!("{e:?}"))?;

    let mut mel = vec![0.0f32; 32 * 16];
    let mel_frames =
        log_mel_from_mono(&mono2, 256, 128, sample_rate, 16, &mut mel).map_err(|e| format!("{e:?}"))?;
    let mut cqt = [0.0f32; 24];
    forward_cqt_mono(&mono2, sample_rate as f32, 55.0, 12, 24, &mut cqt)
        .map_err(|e| format!("{e:?}"))?;
    let cqt_peak = cqt.iter().cloned().fold(0.0f32, f32::max);

    let mut model = ReferenceEventModel::new();
    let mut events = [AuditoryEvent::empty(); 32];
    let mut tokens = [TranscriptToken::empty(); 8];
    let mut emb = [0.0f32; 8];
    let mut ws = vec![0u8; frames * 4];
    let counts = model
        .infer_chunk(view, &mut events, &mut tokens, &mut emb, &mut ws)
        .map_err(|e| format!("{e:?}"))?;

    let mut quins = [crate::semantic::AudioQuin::with_parity(0, 0, 0, 0, 0); 64];
    let n_q = compile_auditory_quins(
        crate::hash::MediaDigest {
            hash: media_hash,
            byte_len: wav.len() as u64,
        },
        &events[..counts.events],
        model.model_hash(),
        &mut quins,
    );

    let classes: Vec<u64> = events[..counts.events].iter().map(|e| e.class_hash).collect();

    Ok(EarsDemoResult {
        sample_rate,
        frames: frames as u32,
        n_events: counts.events,
        n_quins: n_q,
        model_hash: model.model_hash(),
        media_hash,
        is_reference: true,
        mel_frames,
        cqt_bins: 24,
        cqt_peak,
        event_classes: classes,
        events: events[..counts.events].to_vec(),
        note: "Ears path: tone/WAV → store → log-mel+CQT → reference VAD/events → quins. COMPLETE-WITH-GATE: not P64 AED; no mic."
            .into(),
    })
}

/// Import a WAV file, store, feature, classify.
pub fn run_ears_on_wav_file(
    storage_root: Option<&std::path::Path>,
    path: &std::path::Path,
) -> Result<EarsDemoResult, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let decoded = decode_wav(&bytes).map_err(|e| format!("{e:?}"))?;
    let mut mono = vec![0.0f32; decoded.frames as usize];
    to_mono_f32(decoded.view(), &mut mono).map_err(|e| format!("{e:?}"))?;
    // Re-encode mono i16 for store consistency
    let mut i16s = vec![0i16; mono.len()];
    for (i, &s) in mono.iter().enumerate() {
        i16s[i] = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
    }
    let mut wav = vec![0u8; 44 + i16s.len() * 2];
    let wn =
        encode_wav_i16_mono(&i16s, decoded.sample_rate, &mut wav).map_err(|e| format!("{e:?}"))?;
    wav.truncate(wn);
    let media_hash = {
        let d = media_digest(&wav);
        if let Some(root) = storage_root {
            let store = AudioMediaStore::open(root.join("audio_media")).map_err(|e| e)?;
            let rec = store
                .import_bytes(
                    &wav,
                    decoded.sample_rate,
                    1,
                    mono.len() as u32,
                    RetentionClass::Restricted,
                )
                .map_err(|e| e)?;
            rec.digest_u64
        } else {
            d.hash
        }
    };
    let mut mel = vec![0.0f32; 32 * 16];
    let mel_frames =
        log_mel_from_mono(&mono, 256, 128, decoded.sample_rate, 16, &mut mel).map_err(|e| format!("{e:?}"))?;
    let mut cqt = [0.0f32; 24];
    forward_cqt_mono(&mono, decoded.sample_rate as f32, 55.0, 12, 24, &mut cqt)
        .map_err(|e| format!("{e:?}"))?;
    let view = decoded.view();
    let mut model = ReferenceEventModel::new();
    let mut events = [AuditoryEvent::empty(); 32];
    let mut tokens = [TranscriptToken::empty(); 8];
    let mut emb = [0.0f32; 8];
    let mut ws = vec![0u8; mono.len() * 4];
    let counts = model
        .infer_chunk(view, &mut events, &mut tokens, &mut emb, &mut ws)
        .map_err(|e| format!("{e:?}"))?;
    let mut quins = [crate::semantic::AudioQuin::with_parity(0, 0, 0, 0, 0); 64];
    let n_q = compile_auditory_quins(
        crate::hash::MediaDigest {
            hash: media_hash,
            byte_len: wav.len() as u64,
        },
        &events[..counts.events],
        model.model_hash(),
        &mut quins,
    );
    Ok(EarsDemoResult {
        sample_rate: decoded.sample_rate,
        frames: mono.len() as u32,
        n_events: counts.events,
        n_quins: n_q,
        model_hash: model.model_hash(),
        media_hash,
        is_reference: true,
        mel_frames,
        cqt_bins: 24,
        cqt_peak: cqt.iter().cloned().fold(0.0f32, f32::max),
        event_classes: events[..counts.events]
            .iter()
            .map(|e| e.class_hash)
            .collect(),
        events: events[..counts.events].to_vec(),
        note: format!("WAV file import: {}", path.display()),
    })
}

/// §18 automated smoke (synthetic; COMPLETE-WITH-GATE on capture/P64).
pub fn section18_smoke() -> Result<String, String> {
    let r = run_ears_demo(None, 440.0, 16000, 250)?;
    if r.n_events == 0 {
        return Err("no events".into());
    }
    if r.n_quins < 2 {
        return Err("no quins".into());
    }
    if r.mel_frames == 0 || r.cqt_peak <= 0.0 {
        return Err("features missing".into());
    }
    Ok(format!(
        "section18_smoke OK events={} quins={} cqt_peak={:.4} (ref backend; no mic)",
        r.n_events, r.n_quins, r.cqt_peak
    ))
}

/// Weighted AED path (seed P64-shaped head).
pub fn run_ears_weighted(
    storage_root: Option<&std::path::Path>,
    freq_hz: f32,
    sample_rate: u32,
    duration_ms: u32,
) -> Result<EarsDemoResult, String> {
    let mut r = run_ears_demo(storage_root, freq_hz, sample_rate, duration_ms)?;
    // Re-run with weighted model on the same synthetic path
    let frames = r.frames as usize;
    let mono = crate::media_store::synth_tone_f32(freq_hz, sample_rate, frames, 0.35);
    let mut i16s = vec![0i16; frames];
    for (i, &s) in mono.iter().enumerate() {
        i16s[i] = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
    }
    let mut wav = vec![0u8; 44 + frames * 2];
    let wn = encode_wav_i16_mono(&i16s, sample_rate, &mut wav).map_err(|e| format!("{e:?}"))?;
    let decoded = decode_wav(&wav[..wn]).map_err(|e| format!("{e:?}"))?;
    let mut model = WeightedAedModel::from_seed(0xAED1);
    let mut events = [crate::types::AuditoryEvent::empty(); 32];
    let mut tokens = [TranscriptToken::empty(); 8];
    let mut emb = [0.0f32; 4];
    let mut ws = [0u8; 8];
    use crate::types::AuditoryModel;
    let counts = model
        .infer_chunk(decoded.view(), &mut events, &mut tokens, &mut emb, &mut ws)
        .map_err(|e| format!("{e:?}"))?;
    r.n_events = counts.events;
    r.model_hash = model.model_hash();
    r.is_reference = false;
    r.events = events[..counts.events].to_vec();
    r.event_classes = events[..counts.events]
        .iter()
        .map(|e| e.class_hash)
        .collect();
    r.note =
        "Weighted AED seed head (not certified P64). is_reference=false for shape only.".into();
    Ok(r)
}

/// Sonify demo events to WAV bytes (U3-style hear navigation).
pub fn sonify_demo_to_wav(
    sample_rate: u32,
    events: &[crate::types::AuditoryEvent],
    total_frames: usize,
) -> Result<Vec<u8>, String> {
    let mut mono = vec![0.0f32; total_frames];
    sonify_events_mono(events, sample_rate, total_frames, &mut mono);
    let mut i16s = vec![0i16; total_frames];
    for (i, &s) in mono.iter().enumerate() {
        i16s[i] = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
    }
    let mut wav = vec![0u8; 44 + total_frames * 2];
    let n = encode_wav_i16_mono(&i16s, sample_rate, &mut wav).map_err(|e| format!("{e:?}"))?;
    wav.truncate(n);
    Ok(wav)
}

/// Speech phone demo — empty if language unsupported.
pub fn speech_phone_demo(supported: bool) -> Result<(usize, u64), String> {
    let w = SpeechEncoderWeights::from_seed(7, 16);
    let mut mono = vec![0.0f32; 4096];
    for i in 0..mono.len() {
        mono[i] = (2.0 * core::f32::consts::PI * 180.0 * i as f32 / 16000.0).sin() * 0.25;
    }
    let mut tok = [TranscriptToken::empty(); 32];
    let n = decode_for_language(&w, &mono, 16000, supported, &mut tok)
        .map_err(|e| format!("{e:?}"))?;
    Ok((n, w.model_hash))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ears_demo_runs() {
        let r = run_ears_demo(None, 440.0, 16000, 250).unwrap();
        assert!(r.n_events >= 1);
        assert!(r.n_quins >= 2);
        assert!(r.mel_frames >= 1);
        assert!(r.cqt_peak > 0.0);
        assert!(r.is_reference);
    }

    #[test]
    fn section18_smoke_passes() {
        let s = section18_smoke().unwrap();
        assert!(s.contains("OK"));
    }
}
