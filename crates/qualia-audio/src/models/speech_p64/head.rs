//! Speech CTC encoder head (P64, fail-closed) — AU-LEARNED.
//!
//! `SpeechHead` performs **greedy CTC decoding** over learned weights loaded through the common
//! fail-closed loader. It abstains (`BackendUnavailable`) when (a) no weights are present, or
//! (b) the loaded model declares an **unknown language** (`language_slot == 0`). There is **no
//! silent fallback map**: without a real model the head produces no tokens rather than inventing a
//! transcription.
//!
//! Blob convention: `dims = [vocab, feat_dim, language_slot]`. Token index `0` is the CTC blank.
//! `data` is a row-major projection of `vocab * feat_dim` weights followed by `vocab` biases. Per
//! log-mel frame the head computes `logit_v = bias_v + Σ_f proj[v,f] * logmel[f]`, takes the argmax
//! token, then applies the standard CTC collapse (drop repeats, then drop blanks). Decoding is the
//! hot path and allocates nothing — collapse state is a single previous-index register, and tokens
//! are written straight into the caller's buffer.

use crate::models::loader::{parse_weight_blob, require_weights, WeightBlob, WeightState};
use crate::types::{AudioError, TranscriptToken};

/// Base for synthesized token-form hashes (low-assurance proposal, not a curated lexicon URI).
const SPEECH_FORM_BASE: u64 = 0xC2B2_AE3D_27D4_EB4F;
/// CTC blank token index.
const BLANK: usize = 0;
/// `language_slot` value meaning "unknown language" — forces abstention.
const LANG_UNKNOWN: u32 = 0;

/// Learned speech CTC head. Fail-closed: `Absent` state or unknown language ⇒ abstain.
#[derive(Debug, Default)]
pub struct SpeechHead {
    pub state: WeightState,
}

impl SpeechHead {
    /// Construct with no weights — fails closed until [`SpeechHead::load`] succeeds.
    pub fn new() -> Self {
        Self {
            state: WeightState::Absent,
        }
    }

    /// Load weights from a P64 blob (cold path).
    pub fn load(&mut self, bytes: &[u8]) -> Result<(), AudioError> {
        let blob = parse_weight_blob(bytes)?;
        validate_shape(&blob)?;
        self.state = WeightState::Loaded(blob);
        Ok(())
    }

    /// Whether a real forward pass is available (weights present *and* a known language).
    pub fn is_ready(&self) -> bool {
        match &self.state {
            WeightState::Loaded(blob) => language_slot(blob) != LANG_UNKNOWN,
            WeightState::Absent => false,
        }
    }

    /// Greedy-CTC decode `logmel` (row-major `frames × feat_dim`) into `out_tokens`.
    ///
    /// - No weights ⇒ `Err(BackendUnavailable)` (abstain).
    /// - Unknown language (`language_slot == 0`) ⇒ `Err(BackendUnavailable)` (no silent map).
    /// - `logmel.len()` must be a whole multiple of `feat_dim`, else `InvalidParameter`.
    /// - If more tokens survive collapse than `out_tokens` can hold ⇒ `OutputBufferTooSmall`.
    ///
    /// Returns the number of tokens written. Hot path: no allocation.
    pub fn infer_ctc(
        &self,
        logmel: &[f32],
        out_tokens: &mut [TranscriptToken],
    ) -> Result<usize, AudioError> {
        let blob = require_weights(&self.state)?;
        let (vocab, feat_dim, lang) = shape(blob)?;
        if lang == LANG_UNKNOWN {
            // Weights present but no language mapping — abstain, do not guess.
            return Err(AudioError::BackendUnavailable);
        }
        if feat_dim == 0 || !logmel.len().is_multiple_of(feat_dim) {
            return Err(AudioError::InvalidParameter);
        }
        let frames = logmel.len() / feat_dim;
        let bias_off = vocab * feat_dim;

        let mut written = 0usize;
        let mut prev_arg = usize::MAX; // sentinel: no previous emission
        for frame in 0..frames {
            let base = frame * feat_dim;
            let logmel_frame = &logmel[base..base + feat_dim];

            // Streaming argmax over vocab logits (zero-heap).
            let mut best_tok = 0usize;
            let mut best_logit = f32::NEG_INFINITY;
            for v in 0..vocab {
                let logit = token_logit(blob, v, feat_dim, bias_off, logmel_frame);
                if logit > best_logit {
                    best_logit = logit;
                    best_tok = v;
                }
            }

            // CTC collapse: skip repeats of the immediately previous argmax, then skip blanks.
            if best_tok == prev_arg {
                continue;
            }
            prev_arg = best_tok;
            if best_tok == BLANK {
                continue;
            }

            // Softmax confidence of the emitted token.
            let mut sum_exp = 0.0f32;
            for v in 0..vocab {
                let logit = token_logit(blob, v, feat_dim, bias_off, logmel_frame);
                sum_exp += (logit - best_logit).exp();
            }
            let prob = if sum_exp > 0.0 { 1.0 / sum_exp } else { 0.0 };
            let confidence_u16 = (prob.clamp(0.0, 1.0) * 65535.0) as u16;

            if written >= out_tokens.len() {
                return Err(AudioError::OutputBufferTooSmall);
            }
            out_tokens[written] = TranscriptToken {
                form_hash: form_hash(best_tok),
                proposed_meaning_hash: 0,
                confidence_u16,
                language_slot: lang as u16,
                start_frame: frame as u64,
                end_frame: frame as u64 + 1,
                speaker_track: 0,
                flags: 0,
            };
            written += 1;
        }
        Ok(written)
    }
}

#[inline]
fn token_logit(
    blob: &WeightBlob,
    v: usize,
    feat_dim: usize,
    bias_off: usize,
    frame: &[f32],
) -> f32 {
    let row = v * feat_dim;
    let mut acc = blob.data[bias_off + v];
    for f in 0..feat_dim {
        acc += blob.data[row + f] * frame[f];
    }
    acc
}

#[inline]
fn form_hash(idx: usize) -> u64 {
    SPEECH_FORM_BASE
        ^ (idx as u64)
            .wrapping_add(1)
            .wrapping_mul(0x9E37_79B9_7F4A_7C15)
}

#[inline]
fn language_slot(blob: &WeightBlob) -> u32 {
    blob.dims.get(2).copied().unwrap_or(LANG_UNKNOWN)
}

fn shape(blob: &WeightBlob) -> Result<(usize, usize, u32), AudioError> {
    if blob.dims.len() != 3 {
        return Err(AudioError::BackendUnavailable);
    }
    Ok((blob.dims[0] as usize, blob.dims[1] as usize, blob.dims[2]))
}

fn validate_shape(blob: &WeightBlob) -> Result<(), AudioError> {
    let (vocab, feat_dim, _lang) = shape(blob)?;
    if vocab == 0 || feat_dim == 0 {
        return Err(AudioError::BackendUnavailable);
    }
    let need = vocab * feat_dim + vocab;
    if blob.data.len() != need {
        return Err(AudioError::BackendUnavailable);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::loader::{make_blob, write_weight_blob};

    // vocab=3 (0=blank,1,2), feat_dim=2, language=7 (known).
    // token 1 fires on feature[0]; token 2 fires on feature[1]; blank is small everywhere.
    fn known_lang_blob() -> Vec<u8> {
        let data = vec![
            // proj rows (vocab-major): blank, tok1, tok2
            0.0, 0.0, // blank
            3.0, 0.0, // tok1
            0.0, 3.0, // tok2
            // biases
            0.1, 0.0, 0.0,
        ];
        write_weight_blob(&make_blob(vec![3, 2, 7], data))
    }

    fn unknown_lang_blob() -> Vec<u8> {
        let data = vec![0.0, 0.0, 3.0, 0.0, 0.0, 3.0, 0.1, 0.0, 0.0];
        write_weight_blob(&make_blob(vec![3, 2, 0], data)) // language_slot 0 = unknown
    }

    #[test]
    fn absent_abstains_never_fabricates() {
        let head = SpeechHead::new();
        let mut out = [TranscriptToken::empty(); 8];
        let logmel = [1.0, 0.0, 0.0, 1.0];
        assert_eq!(
            head.infer_ctc(&logmel, &mut out),
            Err(AudioError::BackendUnavailable)
        );
        assert_eq!(out[0], TranscriptToken::empty());
    }

    #[test]
    fn unknown_language_abstains() {
        let mut head = SpeechHead::new();
        head.load(&unknown_lang_blob()).expect("load");
        assert!(!head.is_ready());
        let mut out = [TranscriptToken::empty(); 8];
        let logmel = [1.0, 0.0];
        assert_eq!(
            head.infer_ctc(&logmel, &mut out),
            Err(AudioError::BackendUnavailable)
        );
    }

    #[test]
    fn greedy_ctc_collapses_and_decodes() {
        let mut head = SpeechHead::new();
        head.load(&known_lang_blob()).expect("load");
        assert!(head.is_ready());
        let mut out = [TranscriptToken::empty(); 8];
        // frames: tok1, tok1(repeat→collapse), blank(→drop), tok2  => [tok1, tok2]
        let logmel = [
            5.0, 0.0, // -> tok1
            5.0, 0.0, // -> tok1 (repeat)
            0.0, 0.0, // -> blank
            0.0, 5.0, // -> tok2
        ];
        let n = head.infer_ctc(&logmel, &mut out).expect("infer");
        assert_eq!(n, 2);
        assert_eq!(out[0].form_hash, form_hash(1));
        assert_eq!(out[1].form_hash, form_hash(2));
        assert_eq!(out[0].language_slot, 7);
    }

    #[test]
    fn ragged_input_is_invalid() {
        let mut head = SpeechHead::new();
        head.load(&known_lang_blob()).expect("load");
        let mut out = [TranscriptToken::empty(); 8];
        // length 3 is not a multiple of feat_dim=2.
        assert_eq!(
            head.infer_ctc(&[1.0, 0.0, 1.0], &mut out),
            Err(AudioError::InvalidParameter)
        );
    }

    #[test]
    fn output_overflow_reported() {
        let mut head = SpeechHead::new();
        head.load(&known_lang_blob()).expect("load");
        let mut out = [TranscriptToken::empty(); 1];
        // decodes tok1 then tok2 -> needs 2 slots, only 1 provided.
        let logmel = [5.0, 0.0, 0.0, 5.0];
        assert_eq!(
            head.infer_ctc(&logmel, &mut out),
            Err(AudioError::OutputBufferTooSmall)
        );
    }
}
