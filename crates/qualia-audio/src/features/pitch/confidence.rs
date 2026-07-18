//! Map a YIN aperiodicity (the CMND value at the selected lag) to a voicing
//! confidence in `[0, 1]`.

/// Voicing confidence from the CMND minimum `aperiodicity`.
///
/// The CMND value at the chosen lag is YIN's aperiodicity: ~0 for a perfectly
/// periodic frame and ~1 for white noise (where the normalised difference stays
/// near unity for every lag). Confidence is the complementary
/// `1 − aperiodicity`, clamped to `[0, 1]`, with a mild square-law lift so that
/// clean tones (aperiodicity ≈ 1e-3–1e-2) still map close to 1 while noisy
/// frames (aperiodicity ≳ 0.5) collapse well below 0.5.
///
/// NaN / non-finite input maps to `0.0` (treated as unvoiced).
pub fn pitch_confidence(aperiodicity: f32) -> f32 {
    if !aperiodicity.is_finite() {
        return 0.0;
    }
    let a = aperiodicity.clamp(0.0, 1.0);
    let base = 1.0 - a;
    // Square-law lift: keeps clean tones near 1, pushes mid/high aperiodicity
    // (noise) down faster than a bare linear map.
    (base * (2.0 - base)).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_tone_is_confident() {
        // Aperiodicity of a clean tone is tiny.
        assert!(pitch_confidence(0.01) > 0.95, "{}", pitch_confidence(0.01));
        assert!(pitch_confidence(0.001) > 0.99);
    }

    #[test]
    fn noise_is_unconfident() {
        assert!(pitch_confidence(0.8) < 0.5, "{}", pitch_confidence(0.8));
        assert!(pitch_confidence(1.0) <= 0.0 + 1e-6);
    }

    #[test]
    fn monotonic_decreasing() {
        let mut prev = pitch_confidence(0.0);
        for i in 1..=10 {
            let c = pitch_confidence(i as f32 / 10.0);
            assert!(c <= prev + 1e-6, "not monotone at {i}: {c} > {prev}");
            prev = c;
        }
    }

    #[test]
    fn non_finite_is_zero() {
        assert_eq!(pitch_confidence(f32::NAN), 0.0);
        assert_eq!(pitch_confidence(f32::INFINITY), 0.0);
    }
}
