//! Generic local-maximum peak picker over a 1-D array.

use crate::types::AudioError;

/// Parabolic sub-sample offset of a peak located at the centre sample of the
/// three ordinates `(ym1, y0, yp1)`. Returns a value in `(-0.5, 0.5)`; `0.0`
/// when the three points are collinear (degenerate parabola).
#[inline]
fn parabolic_offset(ym1: f32, y0: f32, yp1: f32) -> f32 {
    let denom = ym1 - 2.0 * y0 + yp1;
    if denom == 0.0 || !denom.is_finite() {
        return 0.0;
    }
    let d = 0.5 * (ym1 - yp1) / denom;
    // A genuine local maximum yields |d| < 0.5; clamp defends against noise.
    d.clamp(-0.5, 0.5)
}

/// Detect local-maximum peaks in `x`.
///
/// A sample `x[i]` (interior index `1..len-1`) is a candidate peak when it is
/// strictly greater than its left neighbour, greater-or-equal to its right
/// neighbour, and at least `threshold`. When two accepted peaks are closer than
/// `min_distance` samples, the one with the **smaller** magnitude is discarded
/// (greedy, cascading — the tallest survivor within any `min_distance` window
/// is kept). Accepted peaks are written in ascending index order:
/// `out_pos[k]` receives the parabolically-interpolated sub-sample position and
/// `out_mag[k]` the peak sample magnitude.
///
/// Returns the number of peaks written.
///
/// Zero-heap: the output arrays double as the working stack; no allocation.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if `threshold` is not finite.
/// - [`AudioError::OutputBufferTooSmall`] if more peaks are found than the
///   output arrays can hold (`min(out_pos.len(), out_mag.len())`).
pub fn detect_peaks(
    x: &[f32],
    threshold: f32,
    min_distance: usize,
    out_pos: &mut [f32],
    out_mag: &mut [f32],
) -> Result<usize, AudioError> {
    if !threshold.is_finite() {
        return Err(AudioError::InvalidParameter);
    }
    let cap = out_pos.len().min(out_mag.len());
    if x.len() < 3 || cap == 0 {
        return Ok(0);
    }

    // Phase 1: forward scan. Store the integer index (as f32) so distance
    // comparisons stay exact; parabolic interpolation is applied once at the end.
    let mut count: usize = 0;
    for i in 1..x.len() - 1 {
        let v = x[i];
        let is_peak = v > x[i - 1] && v >= x[i + 1] && v >= threshold;
        if !is_peak {
            continue;
        }
        // Cascading min-distance suppression against the accepted stack.
        let mut rejected = false;
        while count > 0 {
            let last = out_pos[count - 1] as usize;
            if i - last >= min_distance {
                break;
            }
            if v > out_mag[count - 1] {
                count -= 1; // pop the shorter neighbour, re-check the new top
            } else {
                rejected = true;
                break;
            }
        }
        if rejected {
            continue;
        }
        if count == cap {
            return Err(AudioError::OutputBufferTooSmall);
        }
        out_pos[count] = i as f32;
        out_mag[count] = v;
        count += 1;
    }

    // Phase 2: replace integer indices with interpolated sub-sample positions.
    for k in 0..count {
        let idx = out_pos[k] as usize;
        let off = parabolic_offset(x[idx - 1], x[idx], x[idx + 1]);
        out_pos[k] = idx as f32 + off;
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Golden: `[0,1,0,0,3,0,2,0]`, threshold 0.5 -> peaks at indices 1,4,6.
    #[test]
    fn golden_three_peaks() {
        let x = [0.0f32, 1.0, 0.0, 0.0, 3.0, 0.0, 2.0, 0.0];
        let mut pos = [0.0f32; 8];
        let mut mag = [0.0f32; 8];
        let n = detect_peaks(&x, 0.5, 1, &mut pos, &mut mag).expect("detect");
        assert_eq!(n, 3);
        // Symmetric neighbours -> zero parabolic offset -> exact integer positions.
        assert!((pos[0] - 1.0).abs() < 1e-6, "pos0={}", pos[0]);
        assert!((pos[1] - 4.0).abs() < 1e-6, "pos1={}", pos[1]);
        assert!((pos[2] - 6.0).abs() < 1e-6, "pos2={}", pos[2]);
        assert_eq!(mag[0], 1.0);
        assert_eq!(mag[1], 3.0);
        assert_eq!(mag[2], 2.0);
    }

    /// Threshold rejects the small peak at index 1.
    #[test]
    fn threshold_filters() {
        let x = [0.0f32, 1.0, 0.0, 0.0, 3.0, 0.0, 2.0, 0.0];
        let mut pos = [0.0f32; 8];
        let mut mag = [0.0f32; 8];
        let n = detect_peaks(&x, 1.5, 1, &mut pos, &mut mag).expect("detect");
        assert_eq!(n, 2); // only 3.0 and 2.0 survive
        assert_eq!(mag[0], 3.0);
        assert_eq!(mag[1], 2.0);
    }

    /// min_distance suppresses the weaker of two close peaks.
    /// Peaks at 1,4,6; gap(4,6)=2 < 3 -> the shorter (index 6, mag 2) is dropped.
    #[test]
    fn min_distance_suppresses_close() {
        let x = [0.0f32, 1.0, 0.0, 0.0, 3.0, 0.0, 2.0, 0.0];
        let mut pos = [0.0f32; 8];
        let mut mag = [0.0f32; 8];
        let n = detect_peaks(&x, 0.5, 3, &mut pos, &mut mag).expect("detect");
        assert_eq!(n, 2);
        assert!((pos[0] - 1.0).abs() < 1e-6);
        assert!((pos[1] - 4.0).abs() < 1e-6);
        assert_eq!(mag[1], 3.0);
    }

    /// Cascading: a tall central peak swallows shorter neighbours on both sides.
    #[test]
    fn cascading_keeps_tallest() {
        // indices 1(=2),3(=5),5(=3) all within distance 2 of the tallest (idx 3)
        let x = [0.0f32, 2.0, 0.0, 5.0, 0.0, 3.0, 0.0];
        let mut pos = [0.0f32; 8];
        let mut mag = [0.0f32; 8];
        let n = detect_peaks(&x, 0.5, 3, &mut pos, &mut mag).expect("detect");
        assert_eq!(n, 1);
        assert!((pos[0] - 3.0).abs() < 1e-6);
        assert_eq!(mag[0], 5.0);
    }

    /// Sub-sample interpolation: an asymmetric parabola peaks between bins.
    #[test]
    fn parabolic_interpolation() {
        // Downward parabola with vertex at 4.3: vertex = i + 0.5*(ym1-yp1)/(ym1-2y0+yp1)
        let center = 4.3f32;
        let mut x = [0.0f32; 9];
        for (i, xi) in x.iter_mut().enumerate() {
            let v = 1.0 - 0.05 * (i as f32 - center) * (i as f32 - center);
            *xi = v.max(0.0);
        }
        let mut pos = [0.0f32; 4];
        let mut mag = [0.0f32; 4];
        let n = detect_peaks(&x, 0.1, 1, &mut pos, &mut mag).expect("detect");
        assert_eq!(n, 1);
        assert!((pos[0] - 4.3).abs() < 1e-4, "pos={}", pos[0]);
    }

    #[test]
    fn buffer_too_small_errors() {
        let x = [0.0f32, 1.0, 0.0, 3.0, 0.0, 2.0, 0.0];
        let mut pos = [0.0f32; 1];
        let mut mag = [0.0f32; 1];
        assert_eq!(
            detect_peaks(&x, 0.5, 1, &mut pos, &mut mag),
            Err(AudioError::OutputBufferTooSmall)
        );
    }

    #[test]
    fn rejects_nan_threshold() {
        let x = [0.0f32, 1.0, 0.0];
        let mut pos = [0.0f32; 2];
        let mut mag = [0.0f32; 2];
        assert_eq!(
            detect_peaks(&x, f32::NAN, 1, &mut pos, &mut mag),
            Err(AudioError::InvalidParameter)
        );
    }
}
