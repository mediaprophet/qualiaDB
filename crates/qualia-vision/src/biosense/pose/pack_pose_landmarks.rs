//! Pack MediaPipe Pose Landmarker xy (ignore z) into fixed landmark slots.

use crate::cv::error::CvError;

/// MediaPipe pose landmarker lite: 33 landmarks.
pub const MAX_POSE_LANDMARKS: usize = 33;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PoseLandmark {
    pub x: f32,
    pub y: f32,
    pub visibility: f32,
}

/// `xy` length ≥ count*2 (or *3 if has_z — z discarded). `vis` optional per-landmark.
pub fn pack_pose_landmarks_xy(
    xy: &[f32],
    has_z: bool,
    vis: Option<&[f32]>,
    count: usize,
    out: &mut [PoseLandmark],
) -> Result<usize, CvError> {
    let n = count.min(MAX_POSE_LANDMARKS).min(out.len());
    let stride = if has_z { 3 } else { 2 };
    if xy.len() < n * stride {
        return Err(CvError::BufferTooSmall);
    }
    for i in 0..n {
        let base = i * stride;
        out[i] = PoseLandmark {
            x: xy[base],
            y: xy[base + 1],
            visibility: vis.map(|v| v.get(i).copied().unwrap_or(1.0)).unwrap_or(1.0),
        };
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packs_xy_ignores_z() {
        let mut xy = [0.0f32; 33 * 3];
        xy[0] = 0.5;
        xy[1] = 0.25;
        xy[2] = 9.9; // z ignored
        let mut out = [PoseLandmark::default(); MAX_POSE_LANDMARKS];
        let n = pack_pose_landmarks_xy(&xy, true, None, 33, &mut out).unwrap();
        assert_eq!(n, 33);
        assert!((out[0].x - 0.5).abs() < 1e-6);
        assert!((out[0].y - 0.25).abs() < 1e-6);
    }
}
