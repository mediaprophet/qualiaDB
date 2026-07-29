//! Block RMS (root-mean-square) level, linear and in dBFS.
//!
//! Pure time-domain statistic over a caller-owned mono block. Zero-heap: a
//! single scalar accumulator, no allocation.

/// Linear RMS of a mono block: `sqrt(mean(x^2))`.
///
/// Returns `0.0` for an empty block. Accumulates in `f64` for numerical
/// stability over long blocks, then narrows to `f32`.
pub fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let mut acc = 0.0f64;
    for &x in samples {
        let xd = x as f64;
        acc += xd * xd;
    }
    (acc / samples.len() as f64).sqrt() as f32
}

/// RMS expressed in dBFS (`20*log10(rms)`), full-scale reference `1.0`.
///
/// Returns [`f32::NEG_INFINITY`] for digital silence (RMS `== 0`).
pub fn rms_dbfs(samples: &[f32]) -> f32 {
    let r = rms(samples);
    if r <= 0.0 {
        return f32::NEG_INFINITY;
    }
    20.0 * r.log10()
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f32::consts::PI;

    fn sine(freq: f32, sr: f32, n: usize, amp: f32) -> Vec<f32> {
        (0..n)
            .map(|i| amp * (2.0 * PI * freq * i as f32 / sr).sin())
            .collect()
    }

    #[test]
    fn rms_of_unit_sine_is_root_half() {
        // Exactly 1000 whole periods (48-sample period) → RMS = 1/sqrt(2).
        let s = sine(1_000.0, 48_000.0, 48_000, 1.0);
        let r = rms(&s);
        assert!((r - 0.7071067).abs() < 1e-3, "rms {r}");
    }

    #[test]
    fn rms_dbfs_of_unit_sine_near_minus_3() {
        let s = sine(1_000.0, 48_000.0, 48_000, 1.0);
        let db = rms_dbfs(&s);
        assert!((db - (-3.0103)).abs() < 0.05, "dbfs {db}");
    }

    #[test]
    fn empty_is_zero_and_neg_inf() {
        assert_eq!(rms(&[]), 0.0);
        assert_eq!(rms_dbfs(&[]), f32::NEG_INFINITY);
    }

    #[test]
    fn dc_block_rms_equals_level() {
        let s = [0.5f32; 256];
        assert!((rms(&s) - 0.5).abs() < 1e-6);
    }
}
