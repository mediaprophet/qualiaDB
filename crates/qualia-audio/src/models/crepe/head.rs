//! CREPE pitch head (P64, fail-closed) — AU-LEARNED.
//!
//! `CrepeHead` estimates a fundamental frequency from a single audio frame using learned weights
//! loaded through the common fail-closed loader. **With no weights it abstains**
//! (`BackendUnavailable`) — it never returns a fabricated pitch. With a (possibly synthetic/test)
//! blob present it runs the CPU reference forward pass: a linear projection onto a bank of pitch
//! bins, an argmax over the bin activations, and the standard CREPE bin→frequency mapping.
//!
//! Blob convention: `dims = [num_bins, frame_len]`. `data` is a row-major `num_bins * frame_len`
//! projection followed by `num_bins` biases. Per bin `act_b = bias_b + Σ_i proj[b,i] * frame[i]`;
//! the argmax bin maps to a frequency via CREPE's cents grid (bin 0 = `CREPE_CENTS_BASE` cents
//! above 10 Hz, `CREPE_CENTS_STEP` cents per bin). The forward pass is the hot path and allocates
//! nothing (streaming argmax over the caller's frame buffer).

use crate::models::loader::{parse_weight_blob, require_weights, WeightBlob, WeightState};
use crate::types::AudioError;

/// CREPE cents grid: bin 0 sits this many cents above the 10 Hz reference.
const CREPE_CENTS_BASE: f32 = 1997.379_4;
/// Cents per pitch bin (CREPE's canonical 20-cent spacing).
const CREPE_CENTS_STEP: f32 = 20.0;
/// Reference frequency for the cents grid (Hz).
const CREPE_REF_HZ: f32 = 10.0;

/// Learned CREPE pitch head. Fail-closed: `Absent` state ⇒ inference abstains.
#[derive(Debug, Default)]
pub struct CrepeHead {
    pub state: WeightState,
}

impl CrepeHead {
    /// Construct with no weights — fails closed until [`CrepeHead::load`] succeeds.
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

    /// Whether a real forward pass is available.
    pub fn is_ready(&self) -> bool {
        self.state.is_loaded()
    }

    /// Estimate fundamental frequency (Hz) from a single `frame`.
    ///
    /// - No weights ⇒ `Err(BackendUnavailable)` (abstain; no fabricated pitch).
    /// - `sample_rate` must be non-zero, else `InvalidParameter`.
    /// - `frame.len()` must equal the blob's `frame_len`, else `InvalidParameter`.
    ///
    /// Returns the argmax-bin frequency. Hot path: no allocation.
    pub fn infer_pitch(&self, frame: &[f32], sample_rate: u32) -> Result<f32, AudioError> {
        let blob = require_weights(&self.state)?;
        if sample_rate == 0 {
            return Err(AudioError::InvalidParameter);
        }
        let (num_bins, frame_len) = shape(blob)?;
        if frame.len() != frame_len {
            return Err(AudioError::InvalidParameter);
        }

        // Streaming argmax over bin activations (zero-heap).
        let bias_off = num_bins * frame_len;
        let mut best_bin = 0usize;
        let mut best_act = f32::NEG_INFINITY;
        for b in 0..num_bins {
            let row = b * frame_len;
            let mut acc = blob.data[bias_off + b];
            for i in 0..frame_len {
                acc += blob.data[row + i] * frame[i];
            }
            if acc > best_act {
                best_act = acc;
                best_bin = b;
            }
        }

        Ok(bin_to_hz(best_bin))
    }
}

/// CREPE bin → frequency (Hz): `cents = base + step*bin`, `f = ref * 2^(cents/1200)`.
#[inline]
fn bin_to_hz(bin: usize) -> f32 {
    let cents = CREPE_CENTS_BASE + CREPE_CENTS_STEP * bin as f32;
    CREPE_REF_HZ * (cents / 1200.0 * core::f32::consts::LN_2).exp()
}

fn shape(blob: &WeightBlob) -> Result<(usize, usize), AudioError> {
    if blob.dims.len() != 2 {
        return Err(AudioError::BackendUnavailable);
    }
    Ok((blob.dims[0] as usize, blob.dims[1] as usize))
}

fn validate_shape(blob: &WeightBlob) -> Result<(), AudioError> {
    let (num_bins, frame_len) = shape(blob)?;
    if num_bins == 0 || frame_len == 0 {
        return Err(AudioError::BackendUnavailable);
    }
    let need = num_bins * frame_len + num_bins;
    if blob.data.len() != need {
        return Err(AudioError::BackendUnavailable);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::loader::{make_blob, write_weight_blob};

    // 3 bins, frame_len 2. Bin b fires on a distinct frame pattern.
    fn synthetic_blob() -> Vec<u8> {
        let data = vec![
            // proj rows (bin-major)
            1.0, 0.0, // bin0 -> feature[0]
            0.0, 1.0, // bin1 -> feature[1]
            1.0, 1.0, // bin2 -> both
            // biases
            0.0, 0.0, 0.0,
        ];
        write_weight_blob(&make_blob(vec![3, 2], data))
    }

    #[test]
    fn absent_abstains_never_fabricates() {
        let head = CrepeHead::new();
        assert_eq!(
            head.infer_pitch(&[0.5, 0.5], 16_000),
            Err(AudioError::BackendUnavailable)
        );
    }

    #[test]
    fn loaded_runs_reference_forward_pass() {
        let mut head = CrepeHead::new();
        head.load(&synthetic_blob()).expect("load");
        assert!(head.is_ready());
        // frame favours bin1 (feature[1] large) → its mapped frequency.
        let f = head.infer_pitch(&[0.0, 9.0], 16_000).expect("infer");
        assert!((f - bin_to_hz(1)).abs() < 1e-3);
        assert!(f > 0.0);
    }

    #[test]
    fn zero_sample_rate_invalid() {
        let mut head = CrepeHead::new();
        head.load(&synthetic_blob()).expect("load");
        assert_eq!(
            head.infer_pitch(&[0.0, 1.0], 0),
            Err(AudioError::InvalidParameter)
        );
    }

    #[test]
    fn wrong_frame_len_invalid() {
        let mut head = CrepeHead::new();
        head.load(&synthetic_blob()).expect("load");
        assert_eq!(
            head.infer_pitch(&[1.0, 2.0, 3.0], 16_000),
            Err(AudioError::InvalidParameter)
        );
    }

    #[test]
    fn bin_mapping_is_monotonic_and_positive() {
        // Higher bins map to higher frequencies; all positive.
        assert!(bin_to_hz(0) > 0.0);
        assert!(bin_to_hz(10) > bin_to_hz(0));
    }
}
