//! Linear predictive coding (LPC) via Levinson-Durbin recursion.

use crate::types::AudioError;

/// Maximum LPC order supported (bounds the on-stack working arrays so the hot
/// path stays zero-heap).
pub const MAX_LPC_ORDER: usize = 32;

/// Compute `order` linear-prediction coefficients of `signal` by the
/// Levinson-Durbin recursion, plus the associated reflection (PARCOR)
/// coefficients.
///
/// The predictor models each sample as a linear combination of its `order`
/// predecessors, `x[n] ≈ Σ_{j=1}^{order} a_j · x[n-j]`. On success `out_lpc[j]`
/// receives the predictor coefficient `a_{j+1}` (so `out_lpc[0]` is the weight
/// of `x[n-1]`) and `out_reflection[i]` the reflection coefficient `k_{i+1}` of
/// stage `i+1`.
///
/// The biased autocorrelation is computed internally from `signal`; all
/// recursion state lives in fixed on-stack arrays (`MAX_LPC_ORDER`), so the
/// call performs no heap allocation.
///
/// Zero-heap: caller-supplied outputs; bounded on-stack scratch.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if `order` is `0` or exceeds
///   [`MAX_LPC_ORDER`], if `signal` has `<= order` samples, or if the signal
///   carries no energy (a singular, non-invertible autocorrelation).
/// - [`AudioError::OutputBufferTooSmall`] if `out_lpc` or `out_reflection` is
///   shorter than `order`.
pub fn lpc(
    signal: &[f32],
    order: usize,
    out_lpc: &mut [f32],
    out_reflection: &mut [f32],
) -> Result<(), AudioError> {
    if order == 0 || order > MAX_LPC_ORDER || signal.len() <= order {
        return Err(AudioError::InvalidParameter);
    }
    if out_lpc.len() < order || out_reflection.len() < order {
        return Err(AudioError::OutputBufferTooSmall);
    }

    // Biased autocorrelation r[0..=order] (f64 for conditioning).
    let mut r = [0.0f64; MAX_LPC_ORDER + 1];
    for (lag, rl) in r.iter_mut().enumerate().take(order + 1) {
        let mut acc = 0.0f64;
        for n in 0..signal.len() - lag {
            acc += signal[n] as f64 * signal[n + lag] as f64;
        }
        *rl = acc;
    }
    if r[0] <= 0.0 {
        return Err(AudioError::InvalidParameter);
    }

    // Levinson-Durbin: `a` holds the current predictor coefficients a[1..=i];
    // `prev` is the copy of the previous stage used for the symmetric update.
    let mut a = [0.0f64; MAX_LPC_ORDER + 1];
    let mut err = r[0];
    for i in 1..=order {
        // Reflection coefficient k = (r[i] - Σ_{j<i} a[j]·r[i-j]) / err.
        let mut acc = r[i];
        for j in 1..i {
            acc -= a[j] * r[i - j];
        }
        let k = acc / err;

        // Symmetric coefficient update using the previous stage's values.
        let prev = a; // [f64; N] is Copy — on-stack snapshot, no heap.
        for j in 1..i {
            a[j] = prev[j] - k * prev[i - j];
        }
        a[i] = k;

        out_reflection[i - 1] = k as f32;
        err *= 1.0 - k * k;
        // A perfectly predictable stage drives err to 0; floor it so the
        // recursion stays finite instead of dividing by zero.
        if err <= 0.0 {
            err = f64::MIN_POSITIVE;
        }
    }

    for j in 1..=order {
        out_lpc[j - 1] = a[j] as f32;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic LCG in [-1, 1) for reproducible excitation.
    struct Lcg(u64);
    impl Lcg {
        fn next(&mut self) -> f32 {
            self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let u = (self.0 >> 40) as f32 / (1u64 << 24) as f32; // [0,1)
            2.0 * u - 1.0
        }
    }

    /// Golden: an AR(2) process x[n] = a1·x[n-1] + a2·x[n-2] + e[n] driven by
    /// white noise; order-2 LPC recovers (a1, a2) close to the true values.
    #[test]
    fn recovers_ar2_coefficients() {
        let a1 = 1.2f32;
        let a2 = -0.5f32;
        let n = 8000usize;
        let mut x = vec![0.0f32; n];
        let mut rng = Lcg(0x1234_5678_9abc_def0);
        for i in 2..n {
            x[i] = a1 * x[i - 1] + a2 * x[i - 2] + 0.1 * rng.next();
        }
        // Drop the initial transient.
        let stable = &x[200..];

        let mut coeffs = [0.0f32; 2];
        let mut refl = [0.0f32; 2];
        lpc(stable, 2, &mut coeffs, &mut refl).expect("lpc");

        assert!((coeffs[0] - a1).abs() < 0.1, "a1: got {} want {a1}", coeffs[0]);
        assert!((coeffs[1] - a2).abs() < 0.1, "a2: got {} want {a2}", coeffs[1]);
        // Reflection coefficients of a stable AR(2) have magnitude < 1.
        assert!(refl[0].abs() < 1.0 && refl[1].abs() < 1.0, "refl={refl:?}");
        // The second reflection coefficient equals a2 for an AR(2) model.
        assert!((refl[1] - a2).abs() < 0.1, "k2: got {} want {a2}", refl[1]);
    }

    #[test]
    fn order_one_matches_normalized_lag1() {
        // For order 1, a1 = r[1]/r[0] — the lag-1 correlation coefficient.
        let x = [1.0f32, 0.8, 0.6, 0.4, 0.2, 0.0, -0.2, -0.4];
        let mut coeffs = [0.0f32; 1];
        let mut refl = [0.0f32; 1];
        lpc(&x, 1, &mut coeffs, &mut refl).expect("lpc");

        let mut r0 = 0.0f64;
        let mut r1 = 0.0f64;
        for i in 0..x.len() {
            r0 += x[i] as f64 * x[i] as f64;
        }
        for i in 0..x.len() - 1 {
            r1 += x[i] as f64 * x[i + 1] as f64;
        }
        let expect = (r1 / r0) as f32;
        assert!((coeffs[0] - expect).abs() < 1e-5, "a1={} want {expect}", coeffs[0]);
        assert!((refl[0] - expect).abs() < 1e-5, "k1={}", refl[0]);
    }

    #[test]
    fn rejects_bad_order() {
        let x = [1.0f32, 2.0, 3.0, 4.0];
        let mut coeffs = [0.0f32; 4];
        let mut refl = [0.0f32; 4];
        assert_eq!(
            lpc(&x, 0, &mut coeffs, &mut refl),
            Err(AudioError::InvalidParameter)
        );
        // order >= signal length.
        assert_eq!(
            lpc(&x, 4, &mut coeffs, &mut refl),
            Err(AudioError::InvalidParameter)
        );
        // order > MAX_LPC_ORDER.
        let big = vec![0.1f32; 128];
        let mut c = [0.0f32; MAX_LPC_ORDER + 1];
        let mut rf = [0.0f32; MAX_LPC_ORDER + 1];
        assert_eq!(
            lpc(&big, MAX_LPC_ORDER + 1, &mut c, &mut rf),
            Err(AudioError::InvalidParameter)
        );
    }

    #[test]
    fn rejects_silence() {
        let x = [0.0f32; 16];
        let mut coeffs = [0.0f32; 2];
        let mut refl = [0.0f32; 2];
        assert_eq!(
            lpc(&x, 2, &mut coeffs, &mut refl),
            Err(AudioError::InvalidParameter)
        );
    }

    #[test]
    fn rejects_small_output() {
        let x = [1.0f32, 2.0, 3.0, 4.0, 5.0];
        let mut coeffs = [0.0f32; 1];
        let mut refl = [0.0f32; 4];
        assert_eq!(
            lpc(&x, 2, &mut coeffs, &mut refl),
            Err(AudioError::OutputBufferTooSmall)
        );
    }
}
