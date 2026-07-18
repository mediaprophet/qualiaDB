//! FlatnessSFX — flatness of the envelope (geometric / arithmetic mean).

use crate::types::AudioError;

/// FlatnessSFX of an amplitude `envelope`: the ratio of its geometric mean to
/// its arithmetic mean, computed over the rectified (absolute) values. The
/// result lies in `[0, 1]`: a perfectly flat envelope returns ≈ 1, while a
/// spiky, impulsive envelope returns a value near 0.
///
/// A tiny floor is added before the log to keep the geometric mean finite for
/// envelopes that contain exact zeros.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if the envelope is empty or its
///   arithmetic mean is zero.
pub fn flatness_sfx(envelope: &[f32]) -> Result<f32, AudioError> {
    if envelope.is_empty() {
        return Err(AudioError::InvalidParameter);
    }
    const FLOOR: f64 = 1e-12;
    let n = envelope.len() as f64;
    let mut sum = 0.0f64;
    let mut log_sum = 0.0f64;
    for &v in envelope {
        let a = v.abs() as f64 + FLOOR;
        sum += a;
        log_sum += a.ln();
    }
    let arithmetic = sum / n;
    if arithmetic <= 0.0 {
        return Err(AudioError::InvalidParameter);
    }
    let geometric = (log_sum / n).exp();
    Ok((geometric / arithmetic).clamp(0.0, 1.0) as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty() {
        assert_eq!(flatness_sfx(&[]), Err(AudioError::InvalidParameter));
    }

    /// Golden: a constant (perfectly flat) envelope has flatness ≈ 1.
    #[test]
    fn flat_envelope_is_one() {
        let env = vec![0.7f32; 256];
        let f = flatness_sfx(&env).expect("f");
        assert!((f - 1.0).abs() < 1e-4, "flatness={f}");
    }

    /// A single spike among near-zeros is far from flat (≈ 0).
    #[test]
    fn spiky_envelope_is_small() {
        let mut env = vec![0.0f32; 256];
        env[10] = 1.0;
        let f = flatness_sfx(&env).expect("f");
        assert!(f < 0.01, "flatness={f} should be near 0");
    }

    /// Flatness is bounded and a flat signal exceeds a spiky one.
    #[test]
    fn flat_exceeds_spiky() {
        let flat = vec![0.5f32; 128];
        let mut spiky = vec![0.01f32; 128];
        spiky[0] = 5.0;
        let ff = flatness_sfx(&flat).expect("ff");
        let fs = flatness_sfx(&spiky).expect("fs");
        assert!(ff > fs);
        assert!((0.0..=1.0).contains(&ff) && (0.0..=1.0).contains(&fs));
    }
}
