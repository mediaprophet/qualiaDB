//! MediaPipe Face Mesh index → [`PadLandmarkId`] mapping for pure-landmark PAD.
//!
//! Full Face Mesh exposes **468** landmarks. PAD keeps only the eight
//! geometry-critical slots. Indices below match MediaPipe Face Mesh topology
//! (same numbers as `PadLandmarkId::mediapipe_index`).
//!
//! | Role | MediaPipe index | [`PadLandmarkId`] |
//! |------|-----------------|-------------------|
//! | Nose tip | **1** | `NoseTip` |
//! | Chin | **152** | `Chin` |
//! | Left eye outer | **33** | `LeftEyeOuter` |
//! | Right eye outer | **263** | `RightEyeOuter` |
//! | Left cheek / face edge | **234** | `LeftCheek` |
//! | Right cheek / face edge | **454** | `RightCheek` |
//! | Upper lip | **13** | `UpperLip` |
//! | Lower lip | **14** | `LowerLip` |
//!
//! PAR flat-mask lock uses **1 / 234 / 454** (\(N, L, R\)) on raw image \(x\) only.
//! Never use model \(Z\).

use crate::biosense::liveness::landmark_types::PadLandmarkId;

/// MediaPipe Face Mesh landmark count (standard tesselation).
pub const MEDIAPIPE_FACE_MESH_COUNT: usize = 468;

/// Canonical MediaPipe indices for the PAD packing order
/// (`PadLandmarkId` discriminant order 0..7).
pub const PAD_MEDIAPIPE_INDICES: [u16; PadLandmarkId::COUNT] = [
    1,   // NoseTip
    152, // Chin
    33,  // LeftEyeOuter
    263, // RightEyeOuter
    234, // LeftCheek
    454, // RightCheek
    13,  // UpperLip
    14,  // LowerLip
];

/// MediaPipe index for a PAD slot (delegates to [`PadLandmarkId::mediapipe_index`]).
#[inline]
pub const fn mediapipe_index_for_pad(id: PadLandmarkId) -> u16 {
    id.mediapipe_index()
}

/// Inverse map: MediaPipe index → PAD slot, if that index is one of the eight.
pub const fn pad_id_for_mediapipe_index(mp: u16) -> Option<PadLandmarkId> {
    match mp {
        1 => Some(PadLandmarkId::NoseTip),
        152 => Some(PadLandmarkId::Chin),
        33 => Some(PadLandmarkId::LeftEyeOuter),
        263 => Some(PadLandmarkId::RightEyeOuter),
        234 => Some(PadLandmarkId::LeftCheek),
        454 => Some(PadLandmarkId::RightCheek),
        13 => Some(PadLandmarkId::UpperLip),
        14 => Some(PadLandmarkId::LowerLip),
        _ => None,
    }
}

/// All PAD slots as a fixed table (stack-friendly iteration).
pub const PAD_LANDMARK_IDS: [PadLandmarkId; PadLandmarkId::COUNT] = [
    PadLandmarkId::NoseTip,
    PadLandmarkId::Chin,
    PadLandmarkId::LeftEyeOuter,
    PadLandmarkId::RightEyeOuter,
    PadLandmarkId::LeftCheek,
    PadLandmarkId::RightCheek,
    PadLandmarkId::UpperLip,
    PadLandmarkId::LowerLip,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_matches_pad_landmark_id() {
        for id in PAD_LANDMARK_IDS {
            assert_eq!(
                PAD_MEDIAPIPE_INDICES[id as usize],
                id.mediapipe_index(),
                "slot {:?}",
                id
            );
            assert_eq!(mediapipe_index_for_pad(id), id.mediapipe_index());
            assert_eq!(pad_id_for_mediapipe_index(id.mediapipe_index()), Some(id));
        }
    }

    #[test]
    fn par_triple_documented() {
        assert_eq!(PadLandmarkId::NoseTip.mediapipe_index(), 1);
        assert_eq!(PadLandmarkId::LeftCheek.mediapipe_index(), 234);
        assert_eq!(PadLandmarkId::RightCheek.mediapipe_index(), 454);
    }

    #[test]
    fn unknown_index_none() {
        assert!(pad_id_for_mediapipe_index(0).is_none());
        assert!(pad_id_for_mediapipe_index(100).is_none());
        assert!(pad_id_for_mediapipe_index(467).is_none());
    }
}
