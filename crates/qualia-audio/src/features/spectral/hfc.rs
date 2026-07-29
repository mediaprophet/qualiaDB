//! High-frequency content (HFC) of a magnitude spectrum.

use crate::types::AudioError;

/// High-frequency content of a one-sided magnitude spectrum `mag`.
///
/// `HFC = Σ_k k · |X[k]|` — each bin is weighted by its index, so energy at
/// higher bins contributes more. It is a percussive-onset / brightness measure:
/// larger for spectra whose energy sits in the upper bins.
///
/// Zero-heap: single pass, scalar result.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if `mag` is empty.
pub fn high_frequency_content(mag: &[f32]) -> Result<f32, AudioError> {
    if mag.is_empty() {
        return Err(AudioError::InvalidParameter);
    }
    let mut acc = 0.0f64;
    for (k, m) in mag.iter().enumerate() {
        acc += (k as f64) * (*m as f64);
    }
    Ok(acc as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn high_bins_weigh_more() {
        // Same total magnitude, placed low vs high in the spectrum.
        let low = [0.0f32, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let high = [0.0f32, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0];
        let hl = high_frequency_content(&low).expect("hfc");
        let hh = high_frequency_content(&high).expect("hfc");
        assert!((hl - 1.0).abs() < 1e-6, "low hfc={hl}"); // 1 * 1.0
        assert!((hh - 7.0).abs() < 1e-6, "high hfc={hh}"); // 7 * 1.0
        assert!(hh > hl);
    }

    #[test]
    fn dc_only_is_zero() {
        // All energy at bin 0 -> weight 0 -> HFC 0.
        let mag = [5.0f32, 0.0, 0.0, 0.0];
        assert!(high_frequency_content(&mag).expect("hfc").abs() < 1e-6);
    }

    #[test]
    fn known_sum() {
        // 0*1 + 1*2 + 2*3 + 3*4 = 0 + 2 + 6 + 12 = 20
        let mag = [1.0f32, 2.0, 3.0, 4.0];
        assert!((high_frequency_content(&mag).expect("hfc") - 20.0).abs() < 1e-5);
    }

    #[test]
    fn rejects_empty() {
        assert_eq!(
            high_frequency_content(&[]),
            Err(AudioError::InvalidParameter)
        );
    }
}
