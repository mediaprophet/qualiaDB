//! Sliding-window median filter (odd window) — non-linear despiker.
//!
//! Removes isolated impulse spikes while preserving edges better than a mean.
//! Centered and causal-safe at the boundaries via edge replication. Zero-heap:
//! each window is gathered into a small fixed-size stack scratch buffer and
//! sorted in place; no allocation occurs.

use crate::types::AudioError;

/// Largest supported window. Bounds the on-stack scratch buffer.
pub const MAX_MEDIAN_WINDOW: usize = 63;

/// Centered sliding median over `x` into `out`.
///
/// `window` must be odd and in `1..=MAX_MEDIAN_WINDOW`. Boundary samples are
/// handled by replicating the first/last element.
///
/// Errors: `InvalidParameter` if `window` is even, zero, or too large;
/// `OutputBufferTooSmall` if `out` is shorter than `x`.
pub fn median_filter(x: &[f32], out: &mut [f32], window: usize) -> Result<(), AudioError> {
    if window == 0 || window % 2 == 0 || window > MAX_MEDIAN_WINDOW {
        return Err(AudioError::InvalidParameter);
    }
    if out.len() < x.len() {
        return Err(AudioError::OutputBufferTooSmall);
    }
    if x.is_empty() {
        return Ok(());
    }
    let n = x.len();
    let half = (window / 2) as isize;
    let mut scratch = [0.0f32; MAX_MEDIAN_WINDOW];
    for i in 0..n {
        for (k, slot) in scratch.iter_mut().take(window).enumerate() {
            let src = i as isize + (k as isize - half);
            let idx = src.clamp(0, n as isize - 1) as usize;
            *slot = x[idx];
        }
        // Deterministic total order (NaN-safe), operates in place — no heap.
        scratch[..window].sort_unstable_by(|a, b| a.total_cmp(b));
        out[i] = scratch[window / 2];
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_single_impulse_spike() {
        // Flat baseline of 0.5 with one large spike at index 10.
        let mut x = [0.5f32; 21];
        x[10] = 9.0;
        let mut out = [0.0f32; 21];
        median_filter(&x, &mut out, 3).expect("valid");
        // The spike is gone; every output equals the baseline.
        for (i, &y) in out.iter().enumerate() {
            assert!((y - 0.5).abs() < 1e-6, "index {i} = {y}");
        }
    }

    #[test]
    fn preserves_step_edge() {
        let mut x = [0.0f32; 20];
        for v in x.iter_mut().skip(10) {
            *v = 1.0;
        }
        let mut out = [0.0f32; 20];
        median_filter(&x, &mut out, 5).expect("valid");
        // Median preserves the step (unlike a mean, which would ramp).
        assert!((out[8]).abs() < 1e-6);
        assert!((out[9]).abs() < 1e-6);
        assert!((out[10] - 1.0).abs() < 1e-6);
        assert!((out[11] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn window_one_is_identity() {
        let x = [0.3, -0.7, 5.0, 0.1];
        let mut out = [0.0f32; 4];
        median_filter(&x, &mut out, 1).expect("valid");
        for i in 0..x.len() {
            assert!((out[i] - x[i]).abs() < 1e-6);
        }
    }

    #[test]
    fn even_window_errors() {
        let x = [1.0f32; 8];
        let mut out = [0.0f32; 8];
        assert_eq!(
            median_filter(&x, &mut out, 4),
            Err(AudioError::InvalidParameter)
        );
    }

    #[test]
    fn oversized_window_errors() {
        let x = [1.0f32; 8];
        let mut out = [0.0f32; 8];
        assert_eq!(
            median_filter(&x, &mut out, MAX_MEDIAN_WINDOW + 2),
            Err(AudioError::InvalidParameter)
        );
    }
}
