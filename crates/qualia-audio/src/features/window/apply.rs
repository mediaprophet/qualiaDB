//! Elementwise window application (length-checked, in place).

use crate::types::AudioError;

/// Multiply `samples` in place by `window`, elementwise.
///
/// The two slices must have equal length. Zero-heap: delegated to
/// [`etalon_signal::window::apply_window_in_place`].
///
/// Returns [`AudioError::InvalidParameter`] if the lengths differ.
#[inline]
pub fn apply_window(samples: &mut [f32], window: &[f32]) -> Result<(), AudioError> {
    etalon_signal::window::apply_window_in_place(samples, window)
        .map_err(|_| AudioError::InvalidParameter)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multiplies_elementwise() {
        let mut s = [1.0f32, 2.0, 4.0, 8.0];
        let w = [0.5f32, 0.25, 0.0, 1.0];
        apply_window(&mut s, &w).unwrap();
        assert_eq!(s, [0.5, 0.5, 0.0, 8.0]);
    }

    #[test]
    fn length_mismatch_is_invalid_parameter() {
        let mut s = [1.0f32; 4];
        let w = [1.0f32; 3];
        assert_eq!(apply_window(&mut s, &w), Err(AudioError::InvalidParameter));
    }
}
