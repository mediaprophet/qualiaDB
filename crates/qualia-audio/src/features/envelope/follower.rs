//! Asymmetric one-pole envelope follower (fast attack / slow release).

use crate::types::AudioError;

/// Track the amplitude envelope of `x` with an asymmetric one-pole filter.
///
/// `attack` and `release` are the smoothing **coefficients** in `[0, 1)` applied
/// when the rectified input is, respectively, rising above or falling below the
/// current envelope value. Small coefficient = fast response, large = slow.
/// A "fast attack / slow release" follower uses a small `attack` and a large
/// `release`. The per-sample recursion is:
///
/// ```text
/// r = |x[n]|
/// coeff = if r > env { attack } else { release }
/// env = coeff * env + (1 - coeff) * r
/// out[n] = env
/// ```
///
/// Zero-heap: writes into caller-provided `out` (must be at least `x.len()`).
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if `attack`/`release` are non-finite or
///   outside `[0, 1)`.
/// - [`AudioError::OutputBufferTooSmall`] if `out.len() < x.len()`.
pub fn envelope_follow(
    x: &[f32],
    attack: f32,
    release: f32,
    out: &mut [f32],
) -> Result<(), AudioError> {
    if !is_valid_coeff(attack) || !is_valid_coeff(release) {
        return Err(AudioError::InvalidParameter);
    }
    if out.len() < x.len() {
        return Err(AudioError::OutputBufferTooSmall);
    }
    let mut env = 0.0f32;
    for (i, &sample) in x.iter().enumerate() {
        let r = sample.abs();
        let coeff = if r > env { attack } else { release };
        env = coeff * env + (1.0 - coeff) * r;
        out[i] = env;
    }
    Ok(())
}

#[inline]
fn is_valid_coeff(c: f32) -> bool {
    c.is_finite() && (0.0..1.0).contains(&c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_coeffs() {
        let x = [0.0f32; 4];
        let mut out = [0.0f32; 4];
        assert_eq!(
            envelope_follow(&x, 1.0, 0.5, &mut out),
            Err(AudioError::InvalidParameter)
        );
        assert_eq!(
            envelope_follow(&x, -0.1, 0.5, &mut out),
            Err(AudioError::InvalidParameter)
        );
        assert_eq!(
            envelope_follow(&x, 0.5, f32::NAN, &mut out),
            Err(AudioError::InvalidParameter)
        );
    }

    #[test]
    fn rejects_small_out() {
        let x = [0.0f32; 8];
        let mut out = [0.0f32; 4];
        assert_eq!(
            envelope_follow(&x, 0.0, 0.9, &mut out),
            Err(AudioError::OutputBufferTooSmall)
        );
    }

    /// Golden: on a decaying sinusoid the follower yields an envelope that is
    /// monotonically decreasing at coarse scale after the (instant) attack.
    #[test]
    fn tracks_monotonic_decay_of_decaying_sinusoid() {
        let sr = 8000.0f32;
        let f = 200.0f32;
        let tau = 4000.0f32; // decay time constant in samples (0.5 s)
        let n = 8000usize;
        let mut x = vec![0.0f32; n];
        for (i, s) in x.iter_mut().enumerate() {
            let t = i as f32;
            let a = (-t / tau).exp();
            *s = a * (2.0 * std::f32::consts::PI * f * t / sr).sin();
        }
        // Fast attack (instant), slow release.
        let mut env = vec![0.0f32; n];
        envelope_follow(&x, 0.0, 0.9990, &mut env).expect("follow");

        // Sample the upper envelope well past the attack transient.
        let probes = [1000usize, 2500, 4000, 5500, 7000];
        for w in probes.windows(2) {
            assert!(
                env[w[0]] > env[w[1]],
                "envelope not decreasing: env[{}]={} !> env[{}]={}",
                w[0],
                env[w[0]],
                w[1],
                env[w[1]]
            );
        }
        // Envelope stays positive and never exceeds the input peak (~1.0).
        assert!(env[1000] > 0.0 && env[1000] <= 1.0);
    }
}
