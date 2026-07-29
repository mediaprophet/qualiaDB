//! Loudness Range (LRA) per EBU Tech 3342.
//!
//! LRA is the spread between the 10th and 95th percentiles of the *gated*
//! short-term loudness distribution. The caller supplies the short-term
//! loudness values (LUFS, e.g. from a 3 s sliding
//! [`short_term_lufs`](crate::features::loudness::r128::short_term_lufs)) plus a
//! scratch buffer; this function gates them (absolute −70 LUFS, then relative
//! −20 LU below the power-mean) and reports `P95 − P10` in LU.
//!
//! **Zero-heap.** Gated values are copied into the caller-owned `scratch` slice
//! and sorted in place there — no allocation, and the input is left untouched.

use crate::types::AudioError;

const OFFSET: f64 = -0.691;
const ABS_GATE_LUFS: f64 = -70.0;
/// EBU 3342 relative gate for LRA is −20 LU (distinct from R128's −10 LU).
const REL_GATE_LU: f64 = 20.0;
const LOW_PERCENTILE: f64 = 0.10;
const HIGH_PERCENTILE: f64 = 0.95;

#[inline]
fn lufs_to_z(l: f64) -> f64 {
    10f64.powf((l - OFFSET) / 10.0)
}

/// Loudness Range (LU) of a short-term loudness series.
///
/// `short_term_lufs` are LUFS values; `scratch` must be at least as long
/// (holds the gated subset for percentile ranking; contents on return are
/// unspecified). Returns:
/// - `Err(OutputBufferTooSmall)` if `scratch` is shorter than the input,
/// - `Ok(0.0)` if fewer than two values survive gating (range undefined/degenerate),
/// - otherwise `P95 − P10` of the gated distribution.
pub fn loudness_range(short_term_lufs: &[f32], scratch: &mut [f32]) -> Result<f32, AudioError> {
    if scratch.len() < short_term_lufs.len() {
        return Err(AudioError::OutputBufferTooSmall);
    }

    // Absolute gate (−70 LUFS): copy survivors into scratch, accumulate energy.
    let abs_thresh = ABS_GATE_LUFS as f32;
    let mut n = 0usize;
    let mut sum_z = 0.0f64;
    for &l in short_term_lufs {
        if l.is_finite() && l >= abs_thresh {
            scratch[n] = l;
            n += 1;
            sum_z += lufs_to_z(l as f64);
        }
    }
    if n < 2 {
        return Ok(0.0);
    }

    // Relative gate: −20 LU below the power-mean loudness of the abs-gated set.
    let mean_loudness = OFFSET + 10.0 * (sum_z / n as f64).log10();
    let rel_thresh = (mean_loudness - REL_GATE_LU) as f32;
    let mut m = 0usize;
    for i in 0..n {
        if scratch[i] >= rel_thresh {
            scratch[m] = scratch[i];
            m += 1;
        }
    }
    if m < 2 {
        return Ok(0.0);
    }

    let gated = &mut scratch[..m];
    gated.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));

    let lo = percentile_sorted(gated, LOW_PERCENTILE);
    let hi = percentile_sorted(gated, HIGH_PERCENTILE);
    Ok(hi - lo)
}

/// Nearest-rank percentile of an ascending-sorted slice.
///
/// `index = round(p * (n - 1))`, clamped to bounds. `sorted` must be non-empty.
fn percentile_sorted(sorted: &[f32], p: f64) -> f32 {
    let n = sorted.len();
    let idx = (p * (n - 1) as f64).round() as usize;
    sorted[idx.min(n - 1)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_of_ungated_ramp() {
        // 11 short-term values −10..0 LUFS, all within 20 LU of the mean so the
        // relative gate keeps all. Sorted: [−10..0].
        // P10: round(0.10*10)=1 → −9 ; P95: round(0.95*10)=10 → 0 ; LRA = 9.
        let st: Vec<f32> = (0..=10).map(|i| -10.0 + i as f32).collect();
        let mut scratch = vec![0.0f32; st.len()];
        let lra = loudness_range(&st, &mut scratch).expect("lra");
        assert!((lra - 9.0).abs() < 1e-4, "lra {lra}");
    }

    #[test]
    fn flat_signal_has_zero_range() {
        let st = [-18.0f32; 50];
        let mut scratch = [0.0f32; 50];
        let lra = loudness_range(&st, &mut scratch).expect("lra");
        assert!(lra.abs() < 1e-4, "lra {lra}");
    }

    #[test]
    fn silence_below_abs_gate_is_zero() {
        let st = [-80.0f32; 20]; // all below −70 LUFS absolute gate
        let mut scratch = [0.0f32; 20];
        assert_eq!(loudness_range(&st, &mut scratch), Ok(0.0));
    }

    #[test]
    fn small_scratch_errors() {
        let st = [-10.0f32; 8];
        let mut scratch = [0.0f32; 4];
        assert_eq!(
            loudness_range(&st, &mut scratch),
            Err(AudioError::OutputBufferTooSmall)
        );
    }

    #[test]
    fn input_is_not_mutated() {
        let st: Vec<f32> = (0..=10).map(|i| -10.0 + i as f32).collect();
        let orig = st.clone();
        let mut scratch = vec![0.0f32; st.len()];
        let _ = loudness_range(&st, &mut scratch);
        assert_eq!(st, orig);
    }
}
