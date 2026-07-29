//! Acoustic-event detection head (P64, fail-closed) — AU-LEARNED.
//!
//! `AedHead` is a learned acoustic-event classifier that loads its weights through the common
//! fail-closed loader. **With no weights it abstains** (`BackendUnavailable`) — it never emits a
//! fabricated event. With a (possibly synthetic/test) weight blob present it runs a CPU reference
//! linear classifier over a caller-supplied feature vector and writes a single **low-assurance**
//! event *proposal* (flagged `FLAG_LOW_ASSURANCE | FLAG_REFERENCE_BACKEND`), never ground truth.
//!
//! Blob convention: `dims = [num_classes, num_features]`, row-major `data` of
//! `num_classes * num_features` weights followed by `num_classes` bias terms. The forward pass is
//! `logit_c = bias_c + Σ_f weight[c,f] * features[f]`; the argmax class is proposed. Inference is
//! the hot path and allocates nothing (streaming argmax over caller buffers).

use crate::models::loader::{parse_weight_blob, require_weights, WeightBlob, WeightState};
use crate::types::{AudioError, AuditoryEvent};

/// Base for synthesized class hashes (a low-assurance proposal identifier, not a curated URI).
const AED_CLASS_BASE: u64 = 0x9E37_79B9_7F4A_7C15;

/// Learned acoustic-event detection head. Fail-closed: `Absent` state ⇒ inference abstains.
#[derive(Debug, Default)]
pub struct AedHead {
    pub state: WeightState,
}

impl AedHead {
    /// Construct with no weights — fails closed until [`AedHead::load`] succeeds.
    pub fn new() -> Self {
        Self {
            state: WeightState::Absent,
        }
    }

    /// Load weights from a P64 blob (cold path). On success the head leaves fail-closed mode.
    pub fn load(&mut self, bytes: &[u8]) -> Result<(), AudioError> {
        let blob = parse_weight_blob(bytes)?;
        validate_shape(&blob)?;
        self.state = WeightState::Loaded(blob);
        Ok(())
    }

    /// Whether a real forward pass is available.
    pub fn is_ready(&self) -> bool {
        self.state.is_loaded()
    }

    /// Infer at most one low-assurance event proposal from `features` into `out_events`.
    ///
    /// - No weights ⇒ `Err(BackendUnavailable)` (abstain; nothing written).
    /// - `features.len()` must equal the blob's `num_features`, else `InvalidParameter`.
    /// - `out_events` must have room for ≥1 event, else `OutputBufferTooSmall`.
    ///
    /// Returns the number of events written (0 or 1). Hot path: no allocation.
    pub fn infer(
        &self,
        features: &[f32],
        out_events: &mut [AuditoryEvent],
    ) -> Result<usize, AudioError> {
        let blob = require_weights(&self.state)?;
        let (num_classes, num_features) = shape(blob)?;
        if features.len() != num_features {
            return Err(AudioError::InvalidParameter);
        }
        if out_events.is_empty() {
            return Err(AudioError::OutputBufferTooSmall);
        }

        // Streaming argmax over class logits (zero-heap).
        let bias_off = num_classes * num_features;
        let mut best_class = 0usize;
        let mut best_logit = f32::NEG_INFINITY;
        let mut sum_exp = 0.0f32;
        // Two-pass would need storage; instead track max, then a running softmax denominator using
        // the shift-by-current-max trick is awkward stateless. We keep it simple and bounded:
        // first find the max logit, accumulating exp against a provisional max is unnecessary for a
        // small class count, so compute logits twice (cheap; classes are few).
        for c in 0..num_classes {
            let logit = class_logit(blob, c, num_features, bias_off, features);
            if logit > best_logit {
                best_logit = logit;
                best_class = c;
            }
        }
        for c in 0..num_classes {
            let logit = class_logit(blob, c, num_features, bias_off, features);
            sum_exp += (logit - best_logit).exp();
        }
        // Softmax probability of the winning class ∈ (0,1].
        let prob = if sum_exp > 0.0 { 1.0 / sum_exp } else { 0.0 };
        let confidence_u16 = (prob.clamp(0.0, 1.0) * 65535.0) as u16;

        out_events[0] = AuditoryEvent {
            class_hash: class_hash(best_class),
            source_hash: 0,
            confidence_u16,
            channel: 0,
            start_frame: 0,
            end_frame: 0,
            track_id: 0,
            flags: AuditoryEvent::FLAG_LOW_ASSURANCE | AuditoryEvent::FLAG_REFERENCE_BACKEND,
        };
        Ok(1)
    }
}

#[inline]
fn class_logit(
    blob: &WeightBlob,
    c: usize,
    num_features: usize,
    bias_off: usize,
    features: &[f32],
) -> f32 {
    let row = c * num_features;
    let mut acc = blob.data[bias_off + c];
    for f in 0..num_features {
        acc += blob.data[row + f] * features[f];
    }
    acc
}

#[inline]
fn class_hash(idx: usize) -> u64 {
    AED_CLASS_BASE
        ^ (idx as u64)
            .wrapping_add(1)
            .wrapping_mul(0xD6E8_FEB8_6659_FD93)
}

fn shape(blob: &WeightBlob) -> Result<(usize, usize), AudioError> {
    if blob.dims.len() != 2 {
        return Err(AudioError::BackendUnavailable);
    }
    Ok((blob.dims[0] as usize, blob.dims[1] as usize))
}

fn validate_shape(blob: &WeightBlob) -> Result<(), AudioError> {
    let (num_classes, num_features) = shape(blob)?;
    if num_classes == 0 || num_features == 0 {
        return Err(AudioError::BackendUnavailable);
    }
    let need = num_classes * num_features + num_classes;
    if blob.data.len() != need {
        return Err(AudioError::BackendUnavailable);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::loader::{make_blob, write_weight_blob};

    fn synthetic_blob() -> Vec<u8> {
        // 2 classes, 2 features. Class 1 responds strongly to feature[1].
        // weights (row-major): c0=[1,0], c1=[0,1]; bias=[0,0].
        let blob = make_blob(vec![2, 2], vec![1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
        write_weight_blob(&blob)
    }

    #[test]
    fn absent_abstains_never_fabricates() {
        let head = AedHead::new();
        let mut out = [AuditoryEvent::empty(); 4];
        assert_eq!(
            head.infer(&[0.9, 0.1], &mut out),
            Err(AudioError::BackendUnavailable)
        );
        // Nothing written.
        assert_eq!(out[0], AuditoryEvent::empty());
    }

    #[test]
    fn loaded_produces_low_assurance_proposal() {
        let mut head = AedHead::new();
        head.load(&synthetic_blob()).expect("load");
        assert!(head.is_ready());
        let mut out = [AuditoryEvent::empty(); 4];
        // feature[1] dominant ⇒ class 1.
        let n = head.infer(&[0.0, 5.0], &mut out).expect("infer");
        assert_eq!(n, 1);
        assert_eq!(out[0].class_hash, class_hash(1));
        assert_ne!(out[0].flags & AuditoryEvent::FLAG_LOW_ASSURANCE, 0);
        assert_ne!(out[0].flags & AuditoryEvent::FLAG_REFERENCE_BACKEND, 0);
    }

    #[test]
    fn wrong_feature_len_is_invalid() {
        let mut head = AedHead::new();
        head.load(&synthetic_blob()).expect("load");
        let mut out = [AuditoryEvent::empty(); 4];
        assert_eq!(
            head.infer(&[1.0], &mut out),
            Err(AudioError::InvalidParameter)
        );
    }

    #[test]
    fn empty_out_buffer_rejected() {
        let mut head = AedHead::new();
        head.load(&synthetic_blob()).expect("load");
        let mut out: [AuditoryEvent; 0] = [];
        assert_eq!(
            head.infer(&[0.0, 1.0], &mut out),
            Err(AudioError::OutputBufferTooSmall)
        );
    }
}
