//! Map a YIN aperiodicity (the CMND value at the selected lag) to a voicing
//! confidence in `[0, 1]`, delegated to [`etalon_signal::pitch::yin_pitch_confidence`].

/// Voicing confidence from the CMND minimum `aperiodicity`.
#[inline]
pub fn pitch_confidence(aperiodicity: f32) -> f32 {
    etalon_signal::pitch::yin_pitch_confidence(aperiodicity)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_tone_is_confident() {
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
