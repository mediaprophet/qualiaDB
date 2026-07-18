//! Normalised time-domain autocorrelation (caller-buffered).

use crate::types::AudioError;

/// Normalised autocorrelation of `x` written into `out`.
///
/// `out[lag]` receives the biased autocorrelation
/// `sum_{i=0}^{N-lag-1} x[i]*x[i+lag]` divided by the zero-lag energy
/// `sum x[i]^2`, so `out[0] == 1.0`. One value is written per element of `out`;
/// lags `>= N` (no overlap) are written as `0.0`. For a periodic signal of
/// period `P` the strongest lag `> 0` sits at `P` (and its multiples).
///
/// Cost is `O(N * out.len())` — bounded by the caller's chosen lag count; no
/// allocation.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if `x` is empty or carries no energy.
pub fn autocorrelation(x: &[f32], out: &mut [f32]) -> Result<usize, AudioError> {
    if x.is_empty() {
        return Err(AudioError::InvalidParameter);
    }
    if out.is_empty() {
        return Ok(0);
    }
    let n = x.len();
    let mut energy = 0.0f64;
    for &v in x {
        energy += (v as f64) * (v as f64);
    }
    if energy == 0.0 {
        return Err(AudioError::InvalidParameter);
    }

    let lags = out.len();
    for (lag, o) in out.iter_mut().enumerate() {
        if lag >= n {
            *o = 0.0;
            continue;
        }
        let mut acc = 0.0f64;
        for i in 0..n - lag {
            acc += (x[i] as f64) * (x[i + lag] as f64);
        }
        *o = (acc / energy) as f32;
    }
    Ok(lags)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Golden: an impulse train of period P peaks (over lag>0) at lag P.
    #[test]
    fn impulse_train_period() {
        let period = 8usize;
        let n = 64usize;
        let mut x = vec![0.0f32; n];
        let mut i = 0;
        while i < n {
            x[i] = 1.0;
            i += period;
        }
        let mut out = [0.0f32; 40];
        let lags = autocorrelation(&x, &mut out).expect("acf");
        assert_eq!(lags, 40);
        assert!((out[0] - 1.0).abs() < 1e-6, "out0={}", out[0]);

        // argmax over lag > 0.
        let mut best = 1usize;
        for lag in 2..lags {
            if out[lag] > out[best] {
                best = lag;
            }
        }
        assert_eq!(best, period, "argmax lag = {best}");
        // Non-multiples of the period correlate to zero.
        assert!(out[3].abs() < 1e-6, "out3={}", out[3]);
    }

    /// A sinusoid's autocorrelation returns to a local maximum near one period.
    #[test]
    fn sinusoid_local_peak_at_period() {
        let period = 32usize;
        let n = 512usize;
        let mut x = vec![0.0f32; n];
        for (i, xi) in x.iter_mut().enumerate() {
            *xi = (2.0 * std::f32::consts::PI * i as f32 / period as f32).sin();
        }
        let mut out = [0.0f32; 48];
        autocorrelation(&x, &mut out).expect("acf");
        // out[P] is a local maximum vs its neighbours and strongly positive.
        assert!(out[period] > out[period - 1]);
        assert!(out[period] > out[period + 1]);
        assert!(out[period] > 0.8, "out[P]={}", out[period]);
    }

    #[test]
    fn rejects_empty_and_silent() {
        let mut out = [0.0f32; 4];
        assert_eq!(
            autocorrelation(&[], &mut out),
            Err(AudioError::InvalidParameter)
        );
        assert_eq!(
            autocorrelation(&[0.0, 0.0, 0.0], &mut out),
            Err(AudioError::InvalidParameter)
        );
    }

    #[test]
    fn zero_padded_beyond_length() {
        let x = [1.0f32, 1.0];
        let mut out = [0.0f32; 5];
        autocorrelation(&x, &mut out).expect("acf");
        assert_eq!(out[2], 0.0); // lag >= N -> no overlap
        assert_eq!(out[3], 0.0);
    }
}
