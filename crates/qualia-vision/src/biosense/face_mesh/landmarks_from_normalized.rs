//! Convert normalized landmark coordinates (typically \(0..1\)) to pixel space.
//!
//! MediaPipe often reports landmarks in image-normalized units. PAD geometry
//! (interocular scale, PAR in image \(x\)) needs consistent pixel units when
//! thresholds assume pixels — scale once after packing.

use crate::biosense::liveness::landmark_types::{Landmark2, LandmarkFrame, PadLandmarkId};

/// Scale a packed frame from normalized \(x,y \in [0,1]\) (or similar) to pixels.
///
/// \(x' = x \cdot width\), \(y' = y \cdot height\). Invalid slots stay invalid.
/// Does not touch any depth channel (there is none on [`Landmark2`]).
pub fn landmarks_from_normalized(frame: LandmarkFrame, width: f32, height: f32) -> LandmarkFrame {
    let mut out = LandmarkFrame::empty(frame.t_ms);
    if !(width.is_finite() && height.is_finite()) || width <= 0.0 || height <= 0.0 {
        return out;
    }
    for i in 0..PadLandmarkId::COUNT {
        if (frame.valid_mask & (1u8 << i)) == 0 {
            continue;
        }
        let p = frame.points[i];
        // Reconstruct id from slot index (matches discriminant order).
        let id = match i {
            0 => PadLandmarkId::NoseTip,
            1 => PadLandmarkId::Chin,
            2 => PadLandmarkId::LeftEyeOuter,
            3 => PadLandmarkId::RightEyeOuter,
            4 => PadLandmarkId::LeftCheek,
            5 => PadLandmarkId::RightCheek,
            6 => PadLandmarkId::UpperLip,
            7 => PadLandmarkId::LowerLip,
            _ => continue,
        };
        out.set(id, Landmark2::new(p.x * width, p.y * height));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scales_xy() {
        let mut f = LandmarkFrame::empty(7);
        f.set(PadLandmarkId::NoseTip, Landmark2::new(0.5, 0.25));
        f.set(PadLandmarkId::LeftEyeOuter, Landmark2::new(0.1, 0.2));
        let p = landmarks_from_normalized(f, 640.0, 480.0);
        let n = p.get(PadLandmarkId::NoseTip).unwrap();
        assert!((n.x - 320.0).abs() < 1e-3);
        assert!((n.y - 120.0).abs() < 1e-3);
        let e = p.get(PadLandmarkId::LeftEyeOuter).unwrap();
        assert!((e.x - 64.0).abs() < 1e-3);
        assert!((e.y - 96.0).abs() < 1e-3);
        assert_eq!(p.t_ms, 7);
    }

    #[test]
    fn bad_size_clears() {
        let mut f = LandmarkFrame::empty(0);
        f.set(PadLandmarkId::Chin, Landmark2::new(0.5, 0.5));
        let p = landmarks_from_normalized(f, 0.0, 100.0);
        assert!(p.get(PadLandmarkId::Chin).is_none());
    }
}
