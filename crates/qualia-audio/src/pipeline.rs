//! End-to-end Ears MVP demo path (A5 product surface).

use crate::convert::to_mono_f32;
use crate::events::ReferenceEventModel;
use crate::features::log_mel_from_mono;
use crate::hash::media_digest;
use crate::media_store::{synth_tone_f32, AudioMediaStore, RetentionClass};
use crate::semantic::compile_auditory_quins;
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
    pub event_classes: Vec<u64>,
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
    let mel_frames = log_mel_from_mono(&mono2, 256, 128, 16, &mut mel).map_err(|e| format!("{e:?}"))?;

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
        event_classes: classes,
        note: "Ears MVP path: synthetic tone → WAV store → log-mel → reference VAD/events → epistemic quins. Not production AED."
            .into(),
    })
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
        assert!(r.is_reference);
    }
}
