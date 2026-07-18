//! StrongDecay — non-linear combination of energy and temporal centroid.

use crate::types::AudioError;

/// StrongDecay of an amplitude `envelope`.
///
/// Built from the non-linear combination of the envelope energy and its
/// temporal centroid (in seconds): `sqrt(energy * centroid_seconds)`. A signal
/// with both high energy and a centroid pushed later in time yields a larger
/// value; the measure discriminates envelopes with a pronounced decay tail.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if `sample_rate <= 0`, the envelope is
///   empty, or its total weight is zero (undefined centroid).
pub fn strong_decay(envelope: &[f32], sample_rate: f32) -> Result<f32, AudioError> {
    if sample_rate <= 0.0 || !sample_rate.is_finite() || envelope.is_empty() {
        return Err(AudioError::InvalidParameter);
    }
    let mut energy = 0.0f64;
    let mut weighted = 0.0f64;
    let mut total = 0.0f64;
    for (i, &v) in envelope.iter().enumerate() {
        let a = v.abs() as f64;
        energy += a * a;
        weighted += i as f64 * a;
        total += a;
    }
    if total == 0.0 {
        return Err(AudioError::InvalidParameter);
    }
    let centroid_seconds = (weighted / total) / sample_rate as f64;
    Ok((energy * centroid_seconds).sqrt() as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_degenerate() {
        assert_eq!(
            strong_decay(&[1.0, 2.0], 0.0),
            Err(AudioError::InvalidParameter)
        );
        assert_eq!(strong_decay(&[], 8000.0), Err(AudioError::InvalidParameter));
        assert_eq!(
            strong_decay(&[0.0, 0.0], 8000.0),
            Err(AudioError::InvalidParameter)
        );
    }

    /// Golden: a rectangular envelope of unit amplitude has a known StrongDecay.
    /// energy = N, centroid_seconds = ((N-1)/2)/sr; result = sqrt(N * that).
    #[test]
    fn rectangular_envelope_matches_closed_form() {
        let sr = 1000.0f32;
        let n = 100usize;
        let env = vec![1.0f32; n];
        let got = strong_decay(&env, sr).expect("sd");
        let centroid_s = ((n - 1) as f64 / 2.0) / sr as f64;
        let expected = ((n as f64) * centroid_s).sqrt() as f32;
        assert!((got - expected).abs() < 1e-3, "got={got} expected={expected}");
    }

    /// A later-decaying envelope has a larger StrongDecay than an early one
    /// of equal energy.
    #[test]
    fn later_centroid_gives_larger_value() {
        let sr = 1000.0f32;
        let mut early = vec![0.0f32; 200];
        let mut late = vec![0.0f32; 200];
        for i in 0..50 {
            early[i] = 1.0; // energy concentrated at the start
            late[150 + i] = 1.0; // same energy, concentrated at the end
        }
        let se = strong_decay(&early, sr).expect("e");
        let sl = strong_decay(&late, sr).expect("l");
        assert!(sl > se, "late={sl} should exceed early={se}");
    }
}
