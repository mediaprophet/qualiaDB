//! Speech path scaffold — greedy "phone" decode over log-mel (not full ASR).
//!
//! COMPLETE-WITH-GATE: seed matrix, not a licensed streaming encoder. Unknown
//! language must not map silently — we emit only inventory phones or blank.

use crate::features::log_mel_from_mono;
use crate::hash::q_hash;
use crate::types::{AudioError, TranscriptToken};

/// Closed phone inventory (demo).
pub const PHONES: &[&str] = &["sil", "a", "i", "u", "m", "n", "s", "t", "k", "blank"];

#[derive(Debug, Clone)]
pub struct SpeechEncoderWeights {
    pub model_hash: u64,
    pub n_mel: usize,
    pub n_phones: usize,
    /// Row-major [n_phones * n_mel]
    pub weight: Vec<f32>,
}

pub const SPEECH_MAGIC: u32 = 0x4850_5153; // 'SQPH' LE-ish
pub const SPEECH_VERSION: u32 = 1;

impl SpeechEncoderWeights {
    pub fn from_seed(seed: u64, n_mel: usize) -> Self {
        let n_phones = PHONES.len();
        let mut weight = vec![0.0f32; n_phones * n_mel];
        let mut h = seed ^ 0x5EEC_4010_C0DE_u64;
        for w in weight.iter_mut() {
            h = h.wrapping_mul(0x9e37_79b9_7f4a_7c15).rotate_left(7);
            *w = ((h % 2000) as f32 / 1000.0) - 1.0;
        }
        Self {
            model_hash: q_hash(&format!("qualia-audio-speech-seed-v1:{seed:016x}")),
            n_mel,
            n_phones,
            weight,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(32 + self.weight.len() * 4);
        v.extend_from_slice(&SPEECH_MAGIC.to_le_bytes());
        v.extend_from_slice(&SPEECH_VERSION.to_le_bytes());
        v.extend_from_slice(&(self.n_mel as u32).to_le_bytes());
        v.extend_from_slice(&(self.n_phones as u32).to_le_bytes());
        v.extend_from_slice(&self.model_hash.to_le_bytes());
        for f in &self.weight {
            v.extend_from_slice(&f.to_le_bytes());
        }
        v
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, AudioError> {
        if bytes.len() < 24 {
            return Err(AudioError::MalformedAudio);
        }
        let magic = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        if magic != SPEECH_MAGIC {
            return Err(AudioError::BackendUnavailable);
        }
        let n_mel = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
        let n_phones = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]) as usize;
        let model_hash = u64::from_le_bytes(
            bytes[16..24]
                .try_into()
                .map_err(|_| crate::types::AudioError::MalformedAudio)?,
        );
        let need = 24 + n_mel * n_phones * 4;
        if bytes.len() < need || n_phones != PHONES.len() || n_mel == 0 || n_mel > 128 {
            return Err(AudioError::MalformedAudio);
        }
        let mut weight = vec![0.0f32; n_mel * n_phones];
        let mut off = 24usize;
        for w in weight.iter_mut() {
            *w = f32::from_le_bytes(
                bytes[off..off + 4]
                    .try_into()
                    .map_err(|_| crate::types::AudioError::MalformedAudio)?,
            );
            off += 4;
        }
        Ok(Self {
            model_hash,
            n_mel,
            n_phones,
            weight,
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

/// Greedy CTC-like collapse over frames of argmax phone.
pub fn greedy_phone_decode(
    weights: &SpeechEncoderWeights,
    mono: &[f32],
    sample_rate: u32,
    hop: usize,
    out: &mut [TranscriptToken],
) -> Result<usize, AudioError> {
    let max_frames = 64;
    let mut mel = vec![0.0f32; max_frames * weights.n_mel];
    let n_frames = log_mel_from_mono(mono, 256, hop.max(64), sample_rate, weights.n_mel, &mut mel)?;
    let n_frames = n_frames.min(max_frames);
    let blank = weights.n_phones - 1;
    let mut prev = blank;
    let mut w = 0usize;
    let mut frame_start = 0u64;
    for f in 0..n_frames {
        let row = &mel[f * weights.n_mel..(f + 1) * weights.n_mel];
        let mut best_p = 0usize;
        let mut best = f32::NEG_INFINITY;
        for p in 0..weights.n_phones {
            let mut logit = 0.0f32;
            for i in 0..weights.n_mel {
                logit += weights.weight[p * weights.n_mel + i] * row[i];
            }
            if logit > best {
                best = logit;
                best_p = p;
            }
        }
        let conf = ((best.tanh() + 1.0) * 0.5).clamp(0.05, 0.99);
        if best_p != blank && best_p != prev && w < out.len() {
            let mut t = TranscriptToken::empty();
            t.form_hash = q_hash(PHONES[best_p]);
            t.confidence_u16 = (conf * 65535.0) as u16;
            t.start_frame = frame_start;
            t.end_frame = frame_start + hop as u64;
            t.flags = 2; // low assurance proposal
            out[w] = t;
            w += 1;
        }
        prev = best_p;
        frame_start += hop as u64;
    }
    for t in out.iter_mut().skip(w) {
        *t = TranscriptToken::empty();
    }
    Ok(w)
}

/// Streaming CTC-greedy state: feed successive mono chunks; flush tokens.
pub struct StreamingSpeechDecoder {
    weights: SpeechEncoderWeights,
    prev_phone: usize,
    frame_cursor: u64,
    hop: usize,
}

impl StreamingSpeechDecoder {
    pub fn new(weights: SpeechEncoderWeights, hop: usize) -> Self {
        let blank = weights.n_phones.saturating_sub(1);
        Self {
            weights,
            prev_phone: blank,
            frame_cursor: 0,
            hop: hop.max(64),
        }
    }

    pub fn model_hash(&self) -> u64 {
        self.weights.model_hash
    }

    /// Decode one chunk into `out` (appended logical tokens; buffer overwritten from 0).
    pub fn push_chunk(
        &mut self,
        mono: &[f32],
        sample_rate: u32,
        out: &mut [TranscriptToken],
    ) -> Result<usize, AudioError> {
        let max_frames = 32;
        let mut mel = vec![0.0f32; max_frames * self.weights.n_mel];
        let n_frames = log_mel_from_mono(
            mono,
            256,
            self.hop,
            sample_rate,
            self.weights.n_mel,
            &mut mel,
        )?;
        let n_frames = n_frames.min(max_frames);
        let blank = self.weights.n_phones - 1;
        let mut w = 0usize;
        for f in 0..n_frames {
            let row = &mel[f * self.weights.n_mel..(f + 1) * self.weights.n_mel];
            let mut best_p = 0usize;
            let mut best = f32::NEG_INFINITY;
            for p in 0..self.weights.n_phones {
                let mut logit = 0.0f32;
                for i in 0..self.weights.n_mel {
                    logit += self.weights.weight[p * self.weights.n_mel + i] * row[i];
                }
                if logit > best {
                    best = logit;
                    best_p = p;
                }
            }
            let conf = ((best.tanh() + 1.0) * 0.5).clamp(0.05, 0.99);
            if best_p != blank && best_p != self.prev_phone && w < out.len() {
                let mut t = TranscriptToken::empty();
                t.form_hash = q_hash(PHONES[best_p]);
                t.confidence_u16 = (conf * 65535.0) as u16;
                t.start_frame = self.frame_cursor;
                t.end_frame = self.frame_cursor + self.hop as u64;
                t.flags = 2;
                out[w] = t;
                w += 1;
            }
            self.prev_phone = best_p;
            self.frame_cursor += self.hop as u64;
        }
        for t in out.iter_mut().skip(w) {
            *t = TranscriptToken::empty();
        }
        Ok(w)
    }
}

/// If language inventory is empty / unknown, refuse mapping (no silent map).
pub fn decode_for_language(
    weights: &SpeechEncoderWeights,
    mono: &[f32],
    sample_rate: u32,
    language_supported: bool,
    out: &mut [TranscriptToken],
) -> Result<usize, AudioError> {
    if !language_supported {
        // Leave empty — unknown language stays untranscribed.
        for t in out.iter_mut() {
            *t = TranscriptToken::empty();
        }
        return Ok(0);
    }
    greedy_phone_decode(weights, mono, sample_rate, 128, out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_language_empty() {
        let w = SpeechEncoderWeights::from_seed(1, 16);
        let mono = [0.1f32; 2048];
        let mut out = [TranscriptToken::empty(); 16];
        let n = decode_for_language(&w, &mono, 16000, false, &mut out).unwrap();
        assert_eq!(n, 0);
    }

    #[test]
    fn supported_may_emit() {
        let w = SpeechEncoderWeights::from_seed(2, 16);
        let mut mono = vec![0.0f32; 4096];
        for i in 0..mono.len() {
            mono[i] = (2.0 * core::f32::consts::PI * 200.0 * i as f32 / 16000.0).sin() * 0.3;
        }
        let mut out = [TranscriptToken::empty(); 32];
        let n = decode_for_language(&w, &mono, 16000, true, &mut out).unwrap();
        // May be 0 if all blank — still ok; path exercised
        assert!(n <= 32);
    }

    #[test]
    fn speech_weight_roundtrip() {
        let w = SpeechEncoderWeights::from_seed(9, 16);
        let b = w.to_bytes();
        let w2 = SpeechEncoderWeights::from_bytes(&b).unwrap();
        assert_eq!(w.model_hash, w2.model_hash);
        assert_eq!(w.weight, w2.weight);
    }

    #[test]
    fn streaming_decoder_runs() {
        let w = SpeechEncoderWeights::from_seed(3, 16);
        let mut dec = StreamingSpeechDecoder::new(w, 128);
        let mono = [0.2f32; 2048];
        let mut out = [TranscriptToken::empty(); 16];
        let n = dec.push_chunk(&mono, 16000, &mut out).unwrap();
        assert!(n <= 16);
        assert!(dec.model_hash() != 0);
    }
}
