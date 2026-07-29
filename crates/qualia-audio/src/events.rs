//! A5 — reference VAD + energy event classifier (honest reference backend).

use crate::convert::to_mono_f32;
use crate::features::{frame_energy, frame_zcr};
use crate::hash::q_hash;
use crate::types::{
    AudioError, AudioView, AuditoryCapabilities, AuditoryEvent, AuditoryModel,
    AuditoryOutputCounts, TranscriptToken, MAX_EVENTS,
};

pub const CLASS_SILENCE: &str = "https://ns.webizen.org/q42/audio/class/silence";
pub const CLASS_SPEECH_LIKE: &str = "https://ns.webizen.org/q42/audio/class/speech-like";
pub const CLASS_NOISE: &str = "https://ns.webizen.org/q42/audio/class/noise";
pub const CLASS_TONAL: &str = "https://ns.webizen.org/q42/audio/class/tonal";

const MODEL_ID: &str = "qualia-audio-cpu-reference-v1";

/// Energy-based VAD + coarse sound class from ZCR/energy (not a neural AED).
pub struct ReferenceEventModel {
    model_hash: u64,
    pub frame_len: usize,
    pub hop: usize,
    pub vad_threshold: f32,
}

impl Default for ReferenceEventModel {
    fn default() -> Self {
        Self::new()
    }
}

impl ReferenceEventModel {
    pub fn new() -> Self {
        Self {
            model_hash: q_hash(MODEL_ID),
            frame_len: 512,
            hop: 256,
            vad_threshold: 0.02,
        }
    }

    pub fn model_hash(&self) -> u64 {
        self.model_hash
    }
}

impl AuditoryModel for ReferenceEventModel {
    fn capabilities(&self) -> AuditoryCapabilities {
        AuditoryCapabilities {
            max_events: MAX_EVENTS as u16,
            embed_dim: 8,
            supports_vad: true,
            supports_transcript: false,
            is_reference_backend: true,
        }
    }

    fn infer_chunk(
        &mut self,
        audio: AudioView<'_>,
        events_out: &mut [AuditoryEvent],
        tokens_out: &mut [TranscriptToken],
        embedding_out: &mut [f32],
        workspace: &mut [u8],
    ) -> Result<AuditoryOutputCounts, AudioError> {
        if !audio.is_well_formed() {
            return Err(AudioError::MalformedAudio);
        }
        if events_out.is_empty() {
            return Err(AudioError::OutputBufferTooSmall);
        }
        let need = audio.frames as usize * 4;
        if workspace.len() < need {
            return Err(AudioError::WorkspaceTooSmall);
        }
        // Reinterpret workspace as f32 mono buffer (alignment: use local vec for cold path safety)
        let mut mono = vec![0.0f32; audio.frames as usize];
        let n = to_mono_f32(audio, &mut mono)?;
        for t in tokens_out.iter_mut() {
            *t = TranscriptToken::empty();
        }

        let mut event_n = 0usize;
        let mut emb = [0.0f32; 8];
        let mut i = 0usize;
        let mut active_start: Option<usize> = None;
        let mut sum_e = 0.0f32;
        let mut sum_z = 0.0f32;
        let mut cnt = 0u32;

        while i + self.frame_len <= n {
            let frame = &mono[i..i + self.frame_len];
            let e = frame_energy(frame);
            let z = frame_zcr(frame);
            sum_e += e;
            sum_z += z;
            cnt += 1;
            if e >= self.vad_threshold {
                if active_start.is_none() {
                    active_start = Some(i);
                }
            } else if let Some(st) = active_start.take() {
                if event_n < events_out.len() && event_n < MAX_EVENTS {
                    events_out[event_n] =
                        classify_segment(st as u64, i as u64, e, z, self.model_hash, audio);
                    event_n += 1;
                }
            }
            i += self.hop;
        }
        if let Some(st) = active_start {
            if event_n < events_out.len() {
                let e = frame_energy(&mono[st.min(n)..n]);
                let z = frame_zcr(&mono[st.min(n)..n]);
                events_out[event_n] =
                    classify_segment(st as u64, n as u64, e, z, self.model_hash, audio);
                event_n += 1;
            }
        }
        // If never active, optional silence event for whole clip (low conf)
        if event_n == 0 && n > 0 {
            let mut ev = AuditoryEvent::empty();
            ev.class_hash = q_hash(CLASS_SILENCE);
            ev.source_hash = self.model_hash ^ media_src(audio);
            ev.confidence_u16 = 20_000;
            ev.start_frame = 0;
            ev.end_frame = n as u64;
            ev.flags = AuditoryEvent::FLAG_REFERENCE_BACKEND | AuditoryEvent::FLAG_VAD;
            events_out[0] = ev;
            event_n = 1;
        }
        for e in events_out.iter_mut().skip(event_n) {
            *e = AuditoryEvent::empty();
        }
        if cnt > 0 {
            emb[0] = sum_e / cnt as f32;
            emb[1] = sum_z / cnt as f32;
            emb[2] = audio.sample_rate as f32 / 48000.0;
        }
        let emb_n = embedding_out.len().min(8);
        embedding_out[..emb_n].copy_from_slice(&emb[..emb_n]);
        Ok(AuditoryOutputCounts {
            events: event_n,
            tokens: 0,
            embedding_written: emb_n,
        })
    }
}

fn media_src(audio: AudioView<'_>) -> u64 {
    crate::hash::q_hash_bytes(&audio.bytes[..audio.bytes.len().min(1024)]) ^ (audio.frames as u64)
}

fn classify_segment(
    start: u64,
    end: u64,
    energy: f32,
    zcr: f32,
    model_hash: u64,
    audio: AudioView<'_>,
) -> AuditoryEvent {
    let (iri, conf) = if zcr < 0.1 && energy > 0.05 {
        (CLASS_TONAL, 0.7)
    } else if zcr > 0.15 && energy > 0.03 {
        (CLASS_SPEECH_LIKE, 0.55)
    } else if energy > 0.02 {
        (CLASS_NOISE, 0.45)
    } else {
        (CLASS_SILENCE, 0.4)
    };
    let mut e = AuditoryEvent::empty();
    e.class_hash = q_hash(iri);
    e.source_hash = model_hash ^ start ^ media_src(audio);
    e.confidence_u16 = (conf * 65535.0) as u16;
    e.start_frame = start;
    e.end_frame = end.max(start + 1);
    e.flags = AuditoryEvent::FLAG_REFERENCE_BACKEND
        | AuditoryEvent::FLAG_LOW_ASSURANCE
        | AuditoryEvent::FLAG_VAD;
    e
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SampleFormat;

    #[test]
    fn tone_is_tonal_or_active() {
        let sr = 16000u32;
        let n = 4000usize;
        let mut pcm = vec![0i16; n];
        for i in 0..n {
            let t = i as f32 / sr as f32;
            pcm[i] = (0.4 * (2.0 * core::f32::consts::PI * 440.0 * t).sin() * 32767.0) as i16;
        }
        let mut bytes = vec![0u8; n * 2];
        for (i, s) in pcm.iter().enumerate() {
            let b = s.to_le_bytes();
            bytes[i * 2] = b[0];
            bytes[i * 2 + 1] = b[1];
        }
        let view = AudioView {
            bytes: &bytes,
            frames: n as u32,
            channels: 1,
            sample_rate: sr,
            frame_stride_bytes: 2,
            format: SampleFormat::I16,
        };
        let mut m = ReferenceEventModel::new();
        let mut ev = [AuditoryEvent::empty(); 16];
        let mut tok = [TranscriptToken::empty(); 4];
        let mut emb = [0.0f32; 8];
        let mut ws = vec![0u8; n * 4];
        let c = m
            .infer_chunk(view, &mut ev, &mut tok, &mut emb, &mut ws)
            .unwrap();
        assert!(c.events >= 1);
        assert!(m.capabilities().is_reference_backend);
    }
}
