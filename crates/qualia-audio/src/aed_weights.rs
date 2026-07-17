//! P64-style acoustic event head (QVWT-like seed weights) — COMPLETE-WITH-GATE for real P64.

use crate::features::{frame_energy, frame_zcr};
use crate::hash::q_hash;
use crate::types::{
    AudioError, AudioView, AuditoryCapabilities, AuditoryEvent, AuditoryModel, AuditoryOutputCounts,
    TranscriptToken, MAX_EVENTS,
};
use crate::convert::to_mono_f32;
use crate::events::{CLASS_NOISE, CLASS_SILENCE, CLASS_SPEECH_LIKE, CLASS_TONAL};

const MODEL: &str = "qualia-audio-aed-weight-v1";

/// Compact linear head over [energy, zcr, spectral_flux_proxy].
#[derive(Debug, Clone)]
pub struct AedWeightBundle {
    pub model_hash: u64,
    /// 4 classes × 4 features
    pub weight: [f32; 16],
    pub bias: [f32; 4],
    pub class_hashes: [u64; 4],
}

/// File magic `QAED` LE.
pub const AED_MAGIC: u32 = 0x4445_4151;
pub const AED_VERSION: u32 = 1;

impl AedWeightBundle {
    pub fn from_seed(seed: u64) -> Self {
        let mut weight = [0.0f32; 16];
        let mut h = seed ^ 0xAED0_AED0_AED0_AED0;
        for w in weight.iter_mut() {
            h = h.wrapping_mul(0x9e37_79b9_7f4a_7c15).rotate_left(11);
            *w = ((h % 1000) as f32 / 500.0) - 1.0;
        }
        // Bias silence low, speech/tonal toward energy
        let bias = [-0.2, 0.1, 0.05, 0.15];
        Self {
            model_hash: q_hash(&format!("{MODEL}:{seed:016x}")),
            weight,
            bias,
            class_hashes: [
                q_hash(CLASS_SILENCE),
                q_hash(CLASS_SPEECH_LIKE),
                q_hash(CLASS_NOISE),
                q_hash(CLASS_TONAL),
            ],
        }
    }

    /// Serialize to disk (cold path). Layout: magic, ver, model_hash, weight[16], bias[4], class[4].
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(8 + 8 + 16 * 4 + 4 * 4 + 4 * 8);
        v.extend_from_slice(&AED_MAGIC.to_le_bytes());
        v.extend_from_slice(&AED_VERSION.to_le_bytes());
        v.extend_from_slice(&self.model_hash.to_le_bytes());
        for f in &self.weight {
            v.extend_from_slice(&f.to_le_bytes());
        }
        for f in &self.bias {
            v.extend_from_slice(&f.to_le_bytes());
        }
        for h in &self.class_hashes {
            v.extend_from_slice(&h.to_le_bytes());
        }
        v
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, crate::types::AudioError> {
        let need = 4 + 4 + 8 + 16 * 4 + 4 * 4 + 4 * 8;
        if bytes.len() < need {
            return Err(crate::types::AudioError::MalformedAudio);
        }
        let magic = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        if magic != AED_MAGIC {
            return Err(crate::types::AudioError::BackendUnavailable);
        }
        let ver = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        if ver != AED_VERSION {
            return Err(crate::types::AudioError::BackendUnavailable);
        }
        let model_hash = u64::from_le_bytes(bytes[8..16].try_into().unwrap());
        let mut off = 16usize;
        let mut weight = [0.0f32; 16];
        for w in weight.iter_mut() {
            *w = f32::from_le_bytes(bytes[off..off + 4].try_into().unwrap());
            off += 4;
        }
        let mut bias = [0.0f32; 4];
        for b in bias.iter_mut() {
            *b = f32::from_le_bytes(bytes[off..off + 4].try_into().unwrap());
            off += 4;
        }
        let mut class_hashes = [0u64; 4];
        for c in class_hashes.iter_mut() {
            *c = u64::from_le_bytes(bytes[off..off + 8].try_into().unwrap());
            off += 8;
        }
        Ok(Self {
            model_hash,
            weight,
            bias,
            class_hashes,
        })
    }

    pub fn save_path(&self, path: &std::path::Path) -> Result<(), String> {
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
        }
        std::fs::write(path, self.to_bytes()).map_err(|e| e.to_string())
    }

    pub fn load_path(path: &std::path::Path) -> Result<Self, String> {
        let b = std::fs::read(path).map_err(|e| e.to_string())?;
        Self::from_bytes(&b).map_err(|e| format!("{e:?}"))
    }
}

pub struct WeightedAedModel {
    pub bundle: AedWeightBundle,
    pub frame_len: usize,
    pub hop: usize,
    pub vad_threshold: f32,
}

impl WeightedAedModel {
    pub fn from_seed(seed: u64) -> Self {
        Self {
            bundle: AedWeightBundle::from_seed(seed),
            frame_len: 512,
            hop: 256,
            vad_threshold: 0.015,
        }
    }

    pub fn model_hash(&self) -> u64 {
        self.bundle.model_hash
    }

    fn classify_feat(&self, energy: f32, zcr: f32, flux: f32) -> (usize, f32) {
        let feat = [energy, zcr, flux, energy * zcr];
        let mut best_i = 0usize;
        let mut best = f32::NEG_INFINITY;
        for c in 0..4 {
            let mut logit = self.bundle.bias[c];
            for i in 0..4 {
                logit += self.bundle.weight[c * 4 + i] * feat[i];
            }
            if logit > best {
                best = logit;
                best_i = c;
            }
        }
        let conf = ((best.tanh() + 1.0) * 0.5).clamp(0.0, 1.0);
        (best_i, conf)
    }
}

impl AuditoryModel for WeightedAedModel {
    fn capabilities(&self) -> AuditoryCapabilities {
        AuditoryCapabilities {
            max_events: MAX_EVENTS as u16,
            embed_dim: 4,
            supports_vad: true,
            supports_transcript: false,
            // Seed weights are production-shaped but not certified — still flag low assurance via events.
            is_reference_backend: false,
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
        let _ = workspace;
        for t in tokens_out.iter_mut() {
            *t = TranscriptToken::empty();
        }
        let mut mono = vec![0.0f32; audio.frames as usize];
        let n = to_mono_f32(audio, &mut mono)?;
        let mut event_n = 0usize;
        let mut prev_e = 0.0f32;
        let mut i = 0usize;
        let mut active: Option<usize> = None;
        let mut sum_e = 0.0f32;
        let mut sum_z = 0.0f32;
        let mut sum_f = 0.0f32;
        let mut cnt = 0u32;

        while i + self.frame_len <= n {
            let frame = &mono[i..i + self.frame_len];
            let e = frame_energy(frame);
            let z = frame_zcr(frame);
            let flux = (e - prev_e).max(0.0);
            prev_e = e;
            sum_e += e;
            sum_z += z;
            sum_f += flux;
            cnt += 1;
            if e >= self.vad_threshold {
                if active.is_none() {
                    active = Some(i);
                }
            } else if let Some(st) = active.take() {
                if event_n < events_out.len() {
                    let (ci, conf) = self.classify_feat(e, z, flux);
                    events_out[event_n] = make_ev(
                        self.bundle.class_hashes[ci],
                        conf,
                        st as u64,
                        i as u64,
                        self.bundle.model_hash,
                    );
                    event_n += 1;
                }
            }
            i += self.hop;
        }
        if let Some(st) = active {
            if event_n < events_out.len() {
                let e = frame_energy(&mono[st.min(n)..n]);
                let z = frame_zcr(&mono[st.min(n)..n]);
                let (ci, conf) = self.classify_feat(e, z, 0.0);
                events_out[event_n] = make_ev(
                    self.bundle.class_hashes[ci],
                    conf,
                    st as u64,
                    n as u64,
                    self.bundle.model_hash,
                );
                event_n += 1;
            }
        }
        if event_n == 0 && n > 0 {
            events_out[0] = make_ev(
                self.bundle.class_hashes[0],
                0.35,
                0,
                n as u64,
                self.bundle.model_hash,
            );
            event_n = 1;
        }
        for e in events_out.iter_mut().skip(event_n) {
            *e = AuditoryEvent::empty();
        }
        let emb_n = embedding_out.len().min(4);
        if cnt > 0 && emb_n > 0 {
            embedding_out[0] = sum_e / cnt as f32;
            if emb_n > 1 {
                embedding_out[1] = sum_z / cnt as f32;
            }
            if emb_n > 2 {
                embedding_out[2] = sum_f / cnt as f32;
            }
            if emb_n > 3 {
                embedding_out[3] = audio.sample_rate as f32 / 48000.0;
            }
        }
        Ok(AuditoryOutputCounts {
            events: event_n,
            tokens: 0,
            embedding_written: emb_n,
        })
    }
}

fn make_ev(class: u64, conf: f32, start: u64, end: u64, model: u64) -> AuditoryEvent {
    let mut e = AuditoryEvent::empty();
    e.class_hash = class;
    e.source_hash = model ^ start;
    e.confidence_u16 = (conf * 65535.0) as u16;
    e.start_frame = start;
    e.end_frame = end.max(start + 1);
    e.flags = AuditoryEvent::FLAG_LOW_ASSURANCE | AuditoryEvent::FLAG_VAD;
    e
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SampleFormat;

    #[test]
    fn aed_weight_roundtrip() {
        let b = AedWeightBundle::from_seed(42);
        let bytes = b.to_bytes();
        let b2 = AedWeightBundle::from_bytes(&bytes).unwrap();
        assert_eq!(b.model_hash, b2.model_hash);
        assert_eq!(b.weight, b2.weight);
    }

    #[test]
    fn weighted_aed_runs() {
        let n = 3000usize;
        let mut pcm = vec![0i16; n];
        for i in 0..n {
            pcm[i] = (0.3 * (2.0 * core::f32::consts::PI * 300.0 * i as f32 / 16000.0).sin()
                * 32767.0) as i16;
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
            sample_rate: 16000,
            frame_stride_bytes: 2,
            format: SampleFormat::I16,
        };
        let mut m = WeightedAedModel::from_seed(1);
        assert!(!m.capabilities().is_reference_backend);
        let mut ev = [AuditoryEvent::empty(); 16];
        let mut tok = [TranscriptToken::empty(); 4];
        let mut emb = [0.0f32; 4];
        let mut ws = [0u8; 8];
        let c = m.infer_chunk(view, &mut ev, &mut tok, &mut emb, &mut ws).unwrap();
        assert!(c.events >= 1);
    }
}
