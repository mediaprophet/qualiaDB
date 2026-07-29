//! `mel_to_hz` — mel → Hertz, the inverse of [`super::hz_mel::hz_to_mel`] (HTK).

/// Convert a value on the HTK mel scale back to Hertz.
///
/// Inverse of `2595 * log10(1 + hz/700)`, i.e. `hz = 700 * (10^(mel/2595) - 1)`.
/// Round-trips `hz_to_mel` to within ~1e-3 relative error over the audible band.
#[inline]
pub fn mel_to_hz(mel: f32) -> f32 {
    700.0 * (10.0f32.powf(mel / 2595.0) - 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::mel::hz_mel::hz_to_mel;

    #[test]
    fn round_trip_within_tolerance() {
        for &hz in &[50.0f32, 200.0, 440.0, 1000.0, 4000.0, 8000.0, 16000.0] {
            let back = mel_to_hz(hz_to_mel(hz));
            let rel = (back - hz).abs() / hz;
            assert!(rel < 1e-3, "round-trip {hz} -> {back} (rel {rel})");
        }
    }

    #[test]
    fn dc_maps_to_dc() {
        assert!(mel_to_hz(0.0).abs() < 1e-4);
    }
}
