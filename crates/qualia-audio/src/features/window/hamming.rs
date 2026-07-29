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
pub fn hamming_window(out: &mut [f32]) -> Result<(), AudioError> {
    let n = out.len();
    if n == 0 {
        return Err(AudioError::InvalidParameter);
    }
    if n == 1 {
        out[0] = 1.0;
        return Ok(());
    }
    const A0: f64 = 0.54;
    const A1: f64 = 0.46;
    for (i, w) in out.iter_mut().enumerate() {
        let ang = core::f64::consts::TAU * i as f64 / n as f64;
        *w = (A0 - A1 * ang.cos()) as f32;
    }
    Ok(())
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
