//! Spectral flatness (Wiener entropy) of a magnitude/power spectrum.

use crate::types::AudioError;

/// Spectral flatness of a one-sided spectrum `spectrum`.
///
/// Flatness is the ratio of the geometric mean to the arithmetic mean of the
/// bin values:
///
/// ```text
/// flatness = exp(mean(ln x_k)) / mean(x_k)
/// ```
///
/// It lies in `[0, 1]`: a perfectly flat (white) spectrum gives `1.0`
/// (`0 dB`), while a single-tone spectrum with most bins near zero gives a
/// value near `0` (a large negative dB). When `as_db` is `true` the result is
/// returned as `10·log10(flatness)` (floored so a degenerate ratio maps to a
/// large negative dB rather than `-inf`).
///
/// Any non-positive bin drives the geometric mean to zero, so the linear
/// flatness is `0.0` (or the dB floor) whenever a bin is `<= 0`.
///
/// Zero-heap: single pass, scalar result.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if `spectrum` is empty.
pub fn spectral_flatness_db(spectrum: &[f32], as_db: bool) -> Result<f32, AudioError> {
    if spectrum.is_empty() {
        return Err(AudioError::InvalidParameter);
    }

    // dB floor for a degenerate (zero geometric mean) ratio: 10*log10(1e-10).
    const RATIO_FLOOR: f64 = 1e-10;

    let n = spectrum.len() as f64;
    let mut sum = 0.0f64;
    let mut sum_ln = 0.0f64;
    let mut any_nonpositive = false;
    for &v in spectrum {
        let x = v as f64;
        sum += x;
        if x > 0.0 {
            sum_ln += x.ln();
        } else {
            any_nonpositive = true;
        }
    }

    let ratio = if any_nonpositive || sum <= 0.0 {
        0.0
    } else {
        let geo = (sum_ln / n).exp();
        let arith = sum / n;
        (geo / arith).clamp(0.0, 1.0)
    };

    if as_db {
        Ok((10.0 * ratio.max(RATIO_FLOOR).log10()) as f32)
    } else {
        Ok(ratio as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_spectrum_is_unity_and_zero_db() {
        let spectrum = [1.0f32; 32];
        let lin = spectral_flatness_db(&spectrum, false).expect("flatness");
        assert!((lin - 1.0).abs() < 1e-5, "linear flatness={lin}");
        let db = spectral_flatness_db(&spectrum, true).expect("flatness");
        assert!(db.abs() < 1e-3, "dB flatness={db}");
    }

    #[test]
    fn flat_at_any_level_is_unity() {
        let spectrum = [7.5f32; 16];
        let lin = spectral_flatness_db(&spectrum, false).expect("flatness");
        assert!((lin - 1.0).abs() < 1e-5, "flatness={lin}");
    }

    #[test]
    fn single_tone_is_low() {
        // One dominant bin, the rest a small noise floor -> peaky -> low flatness.
        let mut spectrum = [0.001f32; 64];
        spectrum[10] = 10.0;
        let lin = spectral_flatness_db(&spectrum, false).expect("flatness");
        assert!(lin < 0.1, "tone flatness={lin}");
        let db = spectral_flatness_db(&spectrum, true).expect("flatness");
        assert!(db < -10.0, "tone flatness dB={db}");

        // And it is strictly flatter for the white case.
        let white = [1.0f32; 64];
        let lin_white = spectral_flatness_db(&white, false).expect("flatness");
        assert!(lin_white > lin, "white={lin_white} tone={lin}");
    }

    #[test]
    fn zero_bin_drives_flatness_to_zero() {
        let spectrum = [1.0f32, 1.0, 0.0, 1.0];
        assert_eq!(spectral_flatness_db(&spectrum, false), Ok(0.0));
    }

    #[test]
    fn rejects_empty() {
        assert_eq!(
            spectral_flatness_db(&[], false),
            Err(AudioError::InvalidParameter)
        );
    }
}
