//! Pack MediaPipe Hand Landmarker 21 points (xy only).

use crate::cv::error::CvError;

pub const MAX_HAND_LANDMARKS: usize = 21;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct HandLandmark {
    pub x: f32,
    pub y: f32,
}

pub fn pack_hand_landmarks_xy(
    xy: &[f32],
    has_z: bool,
    count: usize,
    out: &mut [HandLandmark],
) -> Result<usize, CvError> {
    let n = count.min(MAX_HAND_LANDMARKS).min(out.len());
    let stride = if has_z { 3 } else { 2 };
    if xy.len() < n * stride {
        return Err(CvError::BufferTooSmall);
    }
    for i in 0..n {
        let base = i * stride;
        out[i] = HandLandmark {
            x: xy[base],
            y: xy[base + 1],
        };
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn twenty_one_points() {
        let xy = [0.1f32; 21 * 2];
        let mut out = [HandLandmark::default(); MAX_HAND_LANDMARKS];
        assert_eq!(pack_hand_landmarks_xy(&xy, false, 21, &mut out).unwrap(), 21);
    }
}
