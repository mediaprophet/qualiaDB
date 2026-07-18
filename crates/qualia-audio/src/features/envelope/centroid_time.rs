//! Time-domain temporal centroid (energy-weighted mean time of a signal).

use crate::types::AudioError;

/// Temporal centroid of `signal` in **seconds**: the energy-weighted mean time
/// `sum(t_i * x_i^2) / sum(x_i^2)` where `t_i = i / sample_rate`. This is the
/// time-domain analogue of the spectral centroid — the "balance point" of the
/// signal's energy along the time axis.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if `sample_rate <= 0`, the signal is
///   empty, or it carries no energy.
pub fn centroid_time(signal: &[f32], sample_rate: f32) -> Result<f32, AudioError> {
    if sample_rate <= 0.0 || !sample_rate.is_finite() || signal.is_empty() {
        return Err(AudioError::InvalidParameter);
    }
    let mut weighted = 0.0f64;
    let mut energy = 0.0f64;
    for (i, &x) in signal.iter().enumerate() {
        let e = (x as f64) * (x as f64);
        weighted += i as f64 * e;
        energy += e;
    }
    if energy == 0.0 {
        return Err(AudioError::InvalidParameter);
    }
    let centroid_samples = weighted / energy;
    Ok((centroid_samples / sample_rate as f64) as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_degenerate() {
        assert_eq!(centroid_time(&[], 8000.0), Err(AudioError::InvalidParameter));
        assert_eq!(
            centroid_time(&[1.0, 1.0], 0.0),
            Err(AudioError::InvalidParameter)
        );
        assert_eq!(
            centroid_time(&[0.0, 0.0, 0.0], 8000.0),
            Err(AudioError::InvalidParameter)
        );
    }

    /// Golden: all energy at one sample -> centroid equals that sample's time.
    #[test]
    fn single_impulse_centroid() {
        let sr = 1000.0f32;
        let mut sig = vec![0.0f32; 100];
        sig[40] = 1.0; // t = 40/1000 = 0.04 s
        let c = centroid_time(&sig, sr).expect("c");
        assert!((c - 0.04).abs() < 1e-6, "centroid={c}");
    }

    /// A symmetric energy distribution centres on the middle time.
    #[test]
    fn symmetric_energy_is_centred() {
        let sr = 1000.0f32;
        let n = 101usize; // symmetric around index 50 -> 0.05 s
        let mut sig = vec![0.0f32; n];
        for i in 0..n {
            // Symmetric triangular magnitude about the centre.
            sig[i] = 1.0 - (i as f32 - 50.0).abs() / 50.0;
        }
        let c = centroid_time(&sig, sr).expect("c");
        assert!((c - 0.05).abs() < 1e-3, "centroid={c}");
    }
}
