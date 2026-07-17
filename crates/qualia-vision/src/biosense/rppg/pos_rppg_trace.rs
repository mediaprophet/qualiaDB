//! POS (Plane-Orthogonal-to-Skin) style rPPG projection on RGB means over time.

use crate::cv::error::CvError;

/// `rgb_means` interleaved [r0,g0,b0, r1,g1,b1, ...] length 3*n_frames.
/// Writes scalar pulse trace into `out` (n_frames).
pub fn pos_rppg_trace(rgb_means: &[f32], n_frames: usize, out: &mut [f32]) -> Result<(), CvError> {
    if n_frames == 0 || rgb_means.len() < n_frames * 3 || out.len() < n_frames {
        return Err(CvError::BufferTooSmall);
    }
    // Temporal normalize per channel
    let mut rs = 0.0f32;
    let mut gs = 0.0f32;
    let mut bs = 0.0f32;
    for i in 0..n_frames {
        rs += rgb_means[i * 3];
        gs += rgb_means[i * 3 + 1];
        bs += rgb_means[i * 3 + 2];
    }
    let n = n_frames as f32;
    let rm = rs / n;
    let gm = gs / n;
    let bm = bs / n;
    // POS: S1 = G-B, S2 = G+B-2R projected (simplified)
    for i in 0..n_frames {
        let r = rgb_means[i * 3] - rm;
        let g = rgb_means[i * 3 + 1] - gm;
        let b = rgb_means[i * 3 + 2] - bm;
        let s1 = g - b;
        let s2 = g + b - 2.0 * r;
        // α ≈ std(s1)/std(s2) approx with running scale
        out[i] = s1 - 0.5 * s2;
    }
    Ok(())
}
