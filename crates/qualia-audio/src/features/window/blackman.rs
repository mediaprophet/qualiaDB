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
pub fn blackman_window(out: &mut [f32]) -> Result<(), AudioError> {
    let n = out.len();
    if n == 0 {
        return Err(AudioError::InvalidParameter);
    }
    if n == 1 {
        out[0] = 1.0;
        return Ok(());
    }
    const A0: f64 = 0.42;
    const A1: f64 = 0.5;
    const A2: f64 = 0.08;
    let nf = n as f64;
    for (i, w) in out.iter_mut().enumerate() {
        let x = i as f64 / nf;
        let c1 = (core::f64::consts::TAU * x).cos();
        let c2 = (2.0 * core::f64::consts::TAU * x).cos();
        *w = (A0 - A1 * c1 + A2 * c2) as f32;
    }
    Ok(())
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
