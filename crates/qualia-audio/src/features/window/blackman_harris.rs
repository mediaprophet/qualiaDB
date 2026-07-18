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
pub fn blackman_harris_window(out: &mut [f32]) -> Result<(), AudioError> {
    let n = out.len();
    if n == 0 {
        return Err(AudioError::InvalidParameter);
    }
    if n == 1 {
        out[0] = 1.0;
        return Ok(());
    }
    const A0: f64 = 0.35875;
    const A1: f64 = 0.48829;
    const A2: f64 = 0.14128;
    const A3: f64 = 0.01168;
    let nf = n as f64;
    for (i, w) in out.iter_mut().enumerate() {
        let x = i as f64 / nf;
        let c1 = (core::f64::consts::TAU * x).cos();
        let c2 = (2.0 * core::f64::consts::TAU * x).cos();
        let c3 = (3.0 * core::f64::consts::TAU * x).cos();
        *w = (A0 - A1 * c1 + A2 * c2 - A3 * c3) as f32;
    }
    Ok(())
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
        assert_eq!(blackman_harris_window(&mut w), Err(AudioError::InvalidParameter));
    }
}
