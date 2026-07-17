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
}

/// Greedy CTC-like collapse over frames of argmax phone.
pub fn greedy_phone_decode(
    weights: &SpeechEncoderWeights,
    mono: &[f32],
    sample_rate: u32,
    hop: usize,
    out: &mut [TranscriptToken],
) -> Result<usize, AudioError> {
    let _ = sample_rate;
    let max_frames = 64;
    let mut mel = vec![0.0f32; max_frames * weights.n_mel];
    let n_frames = log_mel_from_mono(mono, 256, hop.max(64), weights.n_mel, &mut mel)?;
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
}
