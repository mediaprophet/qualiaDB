//! Normalised time-domain cross-correlation (caller-buffered).

use crate::types::AudioError;

/// Normalised cross-correlation of `a` against `b`, written into `out`.
///
/// `out[lag]` receives `sum_i a[i]*b[i+lag]` divided by
/// `sqrt(energy(a) * energy(b))`, giving a correlation coefficient in
/// `[-1, 1]`. The peak lag is the shift at which `b` best aligns with `a`: if
/// `b` is `a` delayed by `d` samples then `out` peaks at `lag == d` with value
/// `~1.0`. One value is written per element of `out`; lags with no overlap are
/// written as `0.0`.
///
/// Cost is `O(min(|a|,|b|) * out.len())`; no allocation.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if either input is empty or has no energy.
pub fn crosscorrelation(a: &[f32], b: &[f32], out: &mut [f32]) -> Result<usize, AudioError> {
    if a.is_empty() || b.is_empty() {
        return Err(AudioError::InvalidParameter);
    }
    if out.is_empty() {
        return Ok(0);
    }
    let mut ea = 0.0f64;
    for &v in a {
        ea += (v as f64) * (v as f64);
    }
    let mut eb = 0.0f64;
    for &v in b {
        eb += (v as f64) * (v as f64);
    }
    if ea == 0.0 || eb == 0.0 {
        return Err(AudioError::InvalidParameter);
    }
    let norm = (ea * eb).sqrt();

    let na = a.len();
    let nb = b.len();
    let lags = out.len();
    for (lag, o) in out.iter_mut().enumerate() {
        if lag >= nb {
            *o = 0.0; // b[i+lag] out of range for every i
            continue;
        }
        let upper = na.min(nb - lag); // i < na and i+lag < nb
        let mut acc = 0.0f64;
        for i in 0..upper {
            acc += (a[i] as f64) * (b[i + lag] as f64);
        }
        *o = (acc / norm) as f32;
    }
    Ok(lags)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Golden: `b` is `a` delayed by 2 -> cross-correlation peaks at lag 2 (~1.0).
    #[test]
    fn peak_at_delay() {
        let a = [1.0f32, 2.0, 3.0, 4.0, 0.0, 0.0, 0.0, 0.0];
        let b = [0.0f32, 0.0, 1.0, 2.0, 3.0, 4.0, 0.0, 0.0]; // a shifted right by 2
        let mut out = [0.0f32; 8];
        let lags = crosscorrelation(&a, &b, &mut out).expect("xcorr");
        assert_eq!(lags, 8);
        let mut best = 0usize;
        for lag in 1..lags {
            if out[lag] > out[best] {
                best = lag;
            }
        }
        assert_eq!(best, 2, "argmax lag = {best}");
        assert!((out[2] - 1.0).abs() < 1e-5, "out[2]={}", out[2]);
    }

    /// Identical inputs correlate to 1.0 at lag 0.
    #[test]
    fn identical_unity_at_zero() {
        let a = [1.0f32, -2.0, 3.0, -1.0];
        let mut out = [0.0f32; 4];
        crosscorrelation(&a, &a, &mut out).expect("xcorr");
        assert!((out[0] - 1.0).abs() < 1e-6, "out0={}", out[0]);
    }

    #[test]
    fn rejects_empty_and_silent() {
        let mut out = [0.0f32; 4];
        assert_eq!(
            crosscorrelation(&[], &[1.0], &mut out),
            Err(AudioError::InvalidParameter)
        );
        assert_eq!(
            crosscorrelation(&[0.0, 0.0], &[1.0, 2.0], &mut out),
            Err(AudioError::InvalidParameter)
        );
    }

    #[test]
    fn lag_beyond_b_is_zero() {
        let a = [1.0f32, 2.0, 3.0];
        let b = [1.0f32, 2.0];
        let mut out = [0.0f32; 4];
        crosscorrelation(&a, &b, &mut out).expect("xcorr");
        assert_eq!(out[2], 0.0);
        assert_eq!(out[3], 0.0);
    }
}
