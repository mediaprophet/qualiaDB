//! `hz_to_mel` — Hertz → mel using the HTK convention `2595 * log10(1 + hz/700)`.
//!
//! The inverse lives in the sibling file [`super::mel_to_hz`] (one pub fn per file).

/// Convert a frequency in Hertz to the HTK mel scale.
///
/// HTK convention: `mel = 2595 * log10(1 + hz / 700)`.
///
/// `hz_to_mel(1000.0) ≈ 1000.0` (the mel scale is anchored so that 1000 Hz maps
/// to ≈ 1000 mel under HTK). Negative inputs are clamped to 0 Hz.
#[inline]
pub fn hz_to_mel(hz: f32) -> f32 {
    let hz = hz.max(0.0);
    2595.0 * (1.0 + hz / 700.0).log10()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mel_1000hz_is_known_htk_value() {
        // HTK: 2595*log10(1+1000/700) = 2595*log10(2.428571...) ≈ 1000.02
        let m = hz_to_mel(1000.0);
        assert!((m - 1000.02).abs() < 0.1, "hz_to_mel(1000)={m}");
    }

    #[test]
    fn mel_is_monotonic_increasing() {
        assert!(hz_to_mel(0.0) < hz_to_mel(100.0));
        assert!(hz_to_mel(100.0) < hz_to_mel(1000.0));
        assert!(hz_to_mel(1000.0) < hz_to_mel(8000.0));
    }

    #[test]
    fn mel_zero_at_dc() {
        assert!(hz_to_mel(0.0).abs() < 1e-6);
        // Negative clamps to DC.
        assert!(hz_to_mel(-50.0).abs() < 1e-6);
    }
}
