//! Crest factor — peak-to-RMS ratio, delegated to [`etalon_signal::loudness`].

/// Linear crest factor `max|x| / rms(x)` of a mono block.
#[inline]
pub fn crest_factor(samples: &[f32]) -> f32 {
    etalon_signal::loudness::crest_factor_f32(samples)
}

/// Crest factor in dB (`20*log10(peak/rms)`).
#[inline]
pub fn crest_factor_db(samples: &[f32]) -> f32 {
    etalon_signal::loudness::crest_factor_db(samples)
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f32::consts::PI;

    fn sine(n: usize, amp: f32) -> Vec<f32> {
        (0..n)
            .map(|i| amp * (2.0 * PI * 1_000.0 * i as f32 / 48_000.0).sin())
            .collect()
    }

    #[test]
    fn sine_crest_is_root_two() {
        let s = sine(48_000, 0.8);
        let c = crest_factor(&s);
        assert!((c - core::f32::consts::SQRT_2).abs() < 1e-2, "crest {c}");
    }

    #[test]
    fn sine_crest_db_is_about_3() {
        let s = sine(48_000, 0.8);
        let db = crest_factor_db(&s);
        assert!((db - 3.0103).abs() < 0.1, "crest dB {db}");
    }

    #[test]
    fn square_wave_crest_is_one() {
        let s: Vec<f32> = (0..1000)
            .map(|i| if i % 2 == 0 { 0.5 } else { -0.5 })
            .collect();
        let c = crest_factor(&s);
        assert!((c - 1.0).abs() < 1e-4, "crest {c}");
    }

    #[test]
    fn empty_and_silent_are_zero() {
        assert_eq!(crest_factor(&[]), 0.0);
        assert_eq!(crest_factor(&[0.0; 32]), 0.0);
        assert_eq!(crest_factor_db(&[0.0; 32]), f32::NEG_INFINITY);
    }
}
