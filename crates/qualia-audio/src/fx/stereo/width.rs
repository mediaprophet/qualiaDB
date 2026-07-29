//! Mid/side stereo width control.
//!
//! Decomposes each interleaved stereo frame into mid `M = (L+R)/2` and side
//! `S = (L-R)/2`, scales the side by `width`, and recombines:
//! `L' = M + width·S`, `R' = M − width·S`.
//!
//! - `width == 0.0` → `S` removed → `L' == R' == M` (mono collapse).
//! - `width == 1.0` → identity (`L' == L`, `R' == R`).
//! - `width  > 1.0` → widened; `width < 0` → channels swapped in the side image.
//!
//! Caller-buffered, **zero-alloc**.

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
    let n = in_stereo.len().min(out_stereo.len()) / 2;
    for i in 0..n {
        let l = in_stereo[i * 2];
        let r = in_stereo[i * 2 + 1];
        let mid = 0.5 * (l + r);
        let side = 0.5 * (l - r) * width;
        out_stereo[i * 2] = mid + side;
        out_stereo[i * 2 + 1] = mid - side;
    }
    Ok(n)
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
            // Collapsed value is the arithmetic mean of the inputs.
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
        // A hard-panned frame: L=1, R=-1 → mid 0, side 1. width=2 doubles side.
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
