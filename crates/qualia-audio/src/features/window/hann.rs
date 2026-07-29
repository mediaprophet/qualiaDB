//! Hann window (periodic / DFT-even convention).

use crate::types::AudioError;

/// Fill `out` with a Hann window of length `out.len()`.
///
/// Uses the **periodic** (DFT-even) convention `w[n] = 0.5·(1 − cos(2πn/N))`
/// with denominator `N = out.len()` (not `N−1`). This is the correct convention
/// for spectral analysis / overlap-add STFT, where the window tiles seamlessly.
/// Consequently `w[0] = 0` and, for even `N`, the centre sample `w[N/2] = 1`;
/// the final sample `w[N−1]` is **not** zero (that is the symmetric convention).
///
/// Returns [`AudioError::InvalidParameter`] if `out` is empty.
pub fn hann_window(out: &mut [f32]) -> Result<(), AudioError> {
    let n = out.len();
    if n == 0 {
        return Err(AudioError::InvalidParameter);
    }
    if n == 1 {
        out[0] = 1.0;
        return Ok(());
    }
    for (i, w) in out.iter_mut().enumerate() {
        let ang = core::f64::consts::TAU * i as f64 / n as f64;
        *w = (0.5 * (1.0 - ang.cos())) as f32;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_zero_centre_one() {
        let mut w = vec![0.0f32; 64];
        hann_window(&mut w).unwrap();
        assert!(w[0].abs() < 1e-6, "w[0] = {}", w[0]);
        assert!((w[32] - 1.0).abs() < 1e-6, "centre = {}", w[32]);
    }

    #[test]
    fn symmetric_about_centre() {
        let mut w = vec![0.0f32; 32];
        hann_window(&mut w).unwrap();
        // Periodic window is symmetric about N/2 for n in 1..N.
        for i in 1..16 {
            assert!((w[i] - w[32 - i]).abs() < 1e-6);
        }
    }

    #[test]
    fn rejects_empty() {
        let mut w: [f32; 0] = [];
        assert_eq!(hann_window(&mut w), Err(AudioError::InvalidParameter));
    }
}
