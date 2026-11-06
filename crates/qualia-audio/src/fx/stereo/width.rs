//! Mid/side stereo width control.
//!
//! Delegated to [`etalon_signal::reverb::stereo_width_interleaved_into`].

use crate::types::AudioError;

/// Apply mid/side width to interleaved stereo `in_stereo` → `out_stereo`.
///
/// Processes `n = min(in_stereo.len(), out_stereo.len()) / 2` frames and returns
/// the frame count. Errors with [`AudioError::InvalidParameter`] if `width` is
/// not finite.
pub fn width(in_stereo: &[f32], out_stereo: &mut [f32], width: f32) -> Result<usize, AudioError> {
    if !width.is_finite() {
        return Err(AudioError::InvalidParameter);
    }
    Ok(etalon_signal::reverb::stereo_width_interleaved_into(
        in_stereo, out_stereo, width,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn width_zero_collapses_to_mono() {
        let inp = [1.0f32, -1.0, 0.5, 0.1, -0.3, 0.9];
        let mut out = [0.0f32; 6];
        let n = width(&inp, &mut out, 0.0).unwrap();
        assert_eq!(n, 3);
        for i in 0..3 {
            let l = out[i * 2];
            let r = out[i * 2 + 1];
            assert!((l - r).abs() < 1e-9, "mono: L {} == R {}", l, r);
            let mid = 0.5 * (inp[i * 2] + inp[i * 2 + 1]);
            assert!((l - mid).abs() < 1e-9);
        }
    }

    #[test]
    fn width_one_is_identity() {
        let inp: Vec<f32> = (0..64).map(|i| (i as f32 * 0.07).sin()).collect();
        let mut out = vec![0.0f32; 64];
        let n = width(&inp, &mut out, 1.0).unwrap();
        assert_eq!(n, 32);
        for i in 0..64 {
            assert!((out[i] - inp[i]).abs() < 1e-6, "identity at {}", i);
        }
    }

    #[test]
    fn width_two_widens_side() {
        let inp = [1.0f32, -1.0];
        let mut out = [0.0f32; 2];
        width(&inp, &mut out, 2.0).unwrap();
        assert!((out[0] - 2.0).abs() < 1e-9);
        assert!((out[1] + 2.0).abs() < 1e-9);
    }

    #[test]
    fn non_finite_width_errors() {
        let inp = [1.0f32, -1.0];
        let mut out = [0.0f32; 2];
        assert_eq!(
            width(&inp, &mut out, f32::NAN),
            Err(AudioError::InvalidParameter)
        );
    }
}
