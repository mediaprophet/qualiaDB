//! Dynamic complexity — average absolute deviation of loudness over time.
//!
//! Given a per-frame loudness series (dB or LUFS), dynamic complexity is the
//! mean absolute deviation of the frames from their own mean: a low value means
//! a consistent level, a high value means large loudness swings. Zero-heap
//! (two scalar passes, no allocation).

/// Average absolute deviation of a loudness series from its mean.
///
/// Returns `0.0` for an empty series or a single frame. Non-finite frames
/// (e.g. `−inf` for silent frames) are ignored so a stray silent frame does not
/// blow the statistic to infinity.
pub fn dynamic_complexity(loudness_frames: &[f32]) -> f32 {
    let mut mean = 0.0f64;
    let mut n = 0u64;
    for &l in loudness_frames {
        if l.is_finite() {
            mean += l as f64;
            n += 1;
        }
    }
    if n < 2 {
        return 0.0;
    }
    mean /= n as f64;

    let mut dev = 0.0f64;
    for &l in loudness_frames {
        if l.is_finite() {
            dev += (l as f64 - mean).abs();
        }
    }
    (dev / n as f64) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_level_has_zero_complexity() {
        assert!(dynamic_complexity(&[-18.0; 64]).abs() < 1e-6);
    }

    #[test]
    fn symmetric_swing_matches_closed_form() {
        // [−10, −20, −10, −20]: mean −15, each |dev| = 5 → MAD = 5.
        let c = dynamic_complexity(&[-10.0, -20.0, -10.0, -20.0]);
        assert!((c - 5.0).abs() < 1e-6, "complexity {c}");
    }

    #[test]
    fn empty_and_single_are_zero() {
        assert_eq!(dynamic_complexity(&[]), 0.0);
        assert_eq!(dynamic_complexity(&[-12.0]), 0.0);
    }

    #[test]
    fn silent_frames_ignored() {
        // The −inf frame must not poison the deviation.
        let c = dynamic_complexity(&[-10.0, f32::NEG_INFINITY, -20.0, -10.0, -20.0]);
        assert!((c - 5.0).abs() < 1e-6, "complexity {c}");
    }
}
