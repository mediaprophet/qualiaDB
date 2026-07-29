//! Crest factor — peak-to-RMS ratio, a measure of transient/dynamic content.
//!
//! `crest = peak / rms`. A pure sine has crest `sqrt(2)` (≈ 3.01 dB); a square
//! wave `1.0` (0 dB); heavily peaked / transient signals are higher. Zero-heap
//! (single pass, no allocation).

/// Linear crest factor `max|x| / rms(x)` of a mono block.
///
/// Returns `0.0` for an empty or all-zero block (RMS `== 0`).
pub fn crest_factor(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let mut peak = 0.0f32;
    let mut sq = 0.0f64;
    for &x in samples {
        let a = x.abs();
        if a > peak {
            peak = a;
        }
        sq += (x as f64) * (x as f64);
    }
    let rms = (sq / samples.len() as f64).sqrt() as f32;
    if rms <= 0.0 {
        return 0.0;
    }
    peak / rms
}

/// Crest factor in dB (`20*log10(peak/rms)`).
///
/// Returns [`f32::NEG_INFINITY`] for an empty or all-zero block.
pub fn crest_factor_db(samples: &[f32]) -> f32 {
    let c = crest_factor(samples);
    if c <= 0.0 {
        return f32::NEG_INFINITY;
    }
    20.0 * c.log10()
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f32::consts::PI;

    fn sine(n: usize, amp: f32) -> Vec<f32> {
        // 1000 whole periods; sample lands exactly on the peak at i=12.
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
