//! 4-term Blackman–Harris window (periodic / DFT-even convention).

use crate::types::AudioError;

/// Fill `out` with a 4-term Blackman–Harris window of length `out.len()`.
///
/// Periodic convention with the standard minimum-side-lobe coefficients
/// (`≈ −92 dB`):
/// `w[n] = a0 − a1·cos(2πn/N) + a2·cos(4πn/N) − a3·cos(6πn/N)`
/// where `a0 = 0.35875`, `a1 = 0.48829`, `a2 = 0.14128`, `a3 = 0.01168` and
/// `N = out.len()`.
///
/// Returns [`AudioError::InvalidParameter`] if `out` is empty.
#[inline]
pub fn blackman_harris_window(out: &mut [f32]) -> Result<(), AudioError> {
    etalon_signal::window::blackman_harris_window(out).map_err(|_| AudioError::InvalidParameter)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_small_centre_one() {
        let mut w = vec![0.0f32; 64];
        blackman_harris_window(&mut w).unwrap();
        // w[0] = a0 - a1 + a2 - a3 ≈ 6e-5.
        assert!(w[0].abs() < 1e-3, "w[0] = {}", w[0]);
        // Centre: a0 + a1 + a2 + a3 = 1.0.
        assert!((w[32] - 1.0).abs() < 1e-4, "centre = {}", w[32]);
    }

    #[test]
    fn rejects_empty() {
        let mut w: [f32; 0] = [];
        assert_eq!(
            blackman_harris_window(&mut w),
            Err(AudioError::InvalidParameter)
        );
    }
}
