//! Hamming window (periodic / DFT-even convention).

use crate::types::AudioError;

/// Fill `out` with a Hamming window of length `out.len()`.
///
/// Periodic convention `w[n] = a0 − a1·cos(2πn/N)` with `a0 = 0.54`,
/// `a1 = 0.46` and denominator `N = out.len()`. Unlike Hann it does not reach
/// zero at the edges (`w[0] = a0 − a1 = 0.08`), which is the defining property
/// of the Hamming taper.
///
/// Returns [`AudioError::InvalidParameter`] if `out` is empty.
#[inline]
pub fn hamming_window(out: &mut [f32]) -> Result<(), AudioError> {
    etalon_signal::window::hamming_window(out).map_err(|_| AudioError::InvalidParameter)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_and_centre_values() {
        let mut w = vec![0.0f32; 64];
        hamming_window(&mut w).unwrap();
        assert!((w[0] - 0.08).abs() < 1e-4, "w[0] = {}", w[0]);
        assert!((w[32] - 1.0).abs() < 1e-4, "centre = {}", w[32]);
    }

    #[test]
    fn rejects_empty() {
        let mut w: [f32; 0] = [];
        assert_eq!(hamming_window(&mut w), Err(AudioError::InvalidParameter));
    }
}
