//! Blackman window (periodic / DFT-even convention).

use crate::types::AudioError;

/// Fill `out` with a Blackman window of length `out.len()`.
///
/// Periodic convention using the classic (unexact) coefficients:
/// `w[n] = a0 − a1·cos(2πn/N) + a2·cos(4πn/N)` with `a0 = 0.42`, `a1 = 0.5`,
/// `a2 = 0.08` and denominator `N = out.len()`. Lower side lobes than Hann at
/// the cost of a wider main lobe.
///
/// Returns [`AudioError::InvalidParameter`] if `out` is empty.
#[inline]
pub fn blackman_window(out: &mut [f32]) -> Result<(), AudioError> {
    etalon_signal::window::blackman_window(out).map_err(|_| AudioError::InvalidParameter)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_near_zero_centre_one() {
        let mut w = vec![0.0f32; 64];
        blackman_window(&mut w).unwrap();
        // w[0] = a0 - a1 + a2 = 0.0.
        assert!(w[0].abs() < 1e-4, "w[0] = {}", w[0]);
        assert!((w[32] - 1.0).abs() < 1e-4, "centre = {}", w[32]);
    }

    #[test]
    fn rejects_empty() {
        let mut w: [f32; 0] = [];
        assert_eq!(blackman_window(&mut w), Err(AudioError::InvalidParameter));
    }
}
