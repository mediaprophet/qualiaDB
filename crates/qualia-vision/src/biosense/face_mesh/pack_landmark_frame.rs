//! Pack a flat MediaPipe Face Mesh buffer into a PAD [`LandmarkFrame`].
//!
//! Accepts **xy** (`468 × 2`) or **xyz** (`468 × 3`) interleaved `f32` layouts.
//! **Model Z is always ignored** — only \(x, y\) enter [`Landmark2`].

use super::mediapipe_index::{
    mediapipe_index_for_pad, PAD_LANDMARK_IDS, MEDIAPIPE_FACE_MESH_COUNT,
};
use crate::biosense::liveness::landmark_types::{Landmark2, LandmarkFrame};
use crate::cv::error::CvError;

/// Flat buffer layout from a MediaPipe-compatible landmarker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LandmarkBufferLayout {
    /// Interleaved `[x0, y0, x1, y1, …]` — length `468 * 2`.
    XyInterleaved,
    /// Interleaved `[x0, y0, z0, x1, y1, z1, …]` — length `468 * 3`.
    /// The \(z\) components are **discarded** (never used for PAD).
    XyzInterleaved,
}

impl LandmarkBufferLayout {
    #[inline]
    pub const fn components_per_landmark(self) -> usize {
        match self {
            Self::XyInterleaved => 2,
            Self::XyzInterleaved => 3,
        }
    }

    #[inline]
    pub const fn expected_len(self) -> usize {
        MEDIAPIPE_FACE_MESH_COUNT * self.components_per_landmark()
    }
}

/// Pack one MediaPipe mesh frame into the eight-slot PAD landmark frame.
///
/// * `t_ms` — challenge-relative timestamp.
/// * `buf` — flat landmark buffer (xy or xyz).
/// * `layout` — stride / component count.
///
/// Only the eight PAD indices are read. Finite \(x,y\) are marked valid;
/// non-finite or missing slots stay unset. Z is never read into geometry.
pub fn pack_landmark_frame(
    t_ms: u32,
    buf: &[f32],
    layout: LandmarkBufferLayout,
) -> Result<LandmarkFrame, CvError> {
    if buf.len() < layout.expected_len() {
        return Err(CvError::BufferTooSmall);
    }

    let stride = layout.components_per_landmark();
    let mut frame = LandmarkFrame::empty(t_ms);

    for id in PAD_LANDMARK_IDS {
        let mp = mediapipe_index_for_pad(id) as usize;
        if mp >= MEDIAPIPE_FACE_MESH_COUNT {
            continue;
        }
        let base = mp * stride;
        // xy always at base+0 / base+1; if xyz, base+2 is Z — intentionally unread.
        let x = buf[base];
        let y = buf[base + 1];
        if x.is_finite() && y.is_finite() {
            frame.set(id, Landmark2::new(x, y));
        }
    }

    Ok(frame)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::biosense::liveness::landmark_types::PadLandmarkId;
    use crate::biosense::face_mesh::mediapipe_index::PAD_MEDIAPIPE_INDICES;

    fn fill_xy() -> [f32; MEDIAPIPE_FACE_MESH_COUNT * 2] {
        let mut buf = [0.0f32; MEDIAPIPE_FACE_MESH_COUNT * 2];
        for (slot, &mp) in PAD_MEDIAPIPE_INDICES.iter().enumerate() {
            let i = mp as usize;
            buf[i * 2] = 10.0 + slot as f32;
            buf[i * 2 + 1] = 100.0 + slot as f32;
        }
        buf
    }

    #[test]
    fn packs_xy_slots() {
        let buf = fill_xy();
        let f = pack_landmark_frame(42, &buf, LandmarkBufferLayout::XyInterleaved).unwrap();
        assert_eq!(f.t_ms, 42);
        for id in PAD_LANDMARK_IDS {
            let p = f.get(id).unwrap();
            let slot = id as usize;
            assert!((p.x - (10.0 + slot as f32)).abs() < 1e-5);
            assert!((p.y - (100.0 + slot as f32)).abs() < 1e-5);
        }
    }

    #[test]
    fn xyz_ignores_z() {
        let mut buf = [0.0f32; MEDIAPIPE_FACE_MESH_COUNT * 3];
        let mp = PadLandmarkId::NoseTip.mediapipe_index() as usize;
        buf[mp * 3] = 1.5;
        buf[mp * 3 + 1] = 2.5;
        buf[mp * 3 + 2] = 999.0; // must not appear in Landmark2
        let f = pack_landmark_frame(0, &buf, LandmarkBufferLayout::XyzInterleaved).unwrap();
        let n = f.get(PadLandmarkId::NoseTip).unwrap();
        assert!((n.x - 1.5).abs() < 1e-6);
        assert!((n.y - 2.5).abs() < 1e-6);
        // Landmark2 has no z field — packing path never stored 999.
    }

    #[test]
    fn short_buffer_errors() {
        let tiny = [0.0f32; 10];
        assert!(matches!(
            pack_landmark_frame(0, &tiny, LandmarkBufferLayout::XyInterleaved),
            Err(CvError::BufferTooSmall)
        ));
    }
}
