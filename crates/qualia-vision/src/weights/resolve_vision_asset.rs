//! Locate a MANIFEST-listed vision weight on disk (caller supplies roots).

use std::path::{Path, PathBuf};

/// Stable asset ids matching `vendor/vision/MANIFEST.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VisionAssetId {
    Yunet,
    Sface,
    MediapipeFaceLandmarker,
    YoloNasS,
    EmotionsRetail,
    MediapipePose,
    MediapipeHands,
}

impl VisionAssetId {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Yunet => "yunet",
            Self::Sface => "sface",
            Self::MediapipeFaceLandmarker => "mediapipe_face_landmarker",
            Self::YoloNasS => "yolo_nas_s",
            Self::EmotionsRetail => "emotions_retail_0003",
            Self::MediapipePose => "mediapipe_pose",
            Self::MediapipeHands => "mediapipe_hands",
        }
    }

    pub const fn licence_tag(self) -> AssetLicenceTag {
        // All current pack entries are PermissiveReady (MIT or Apache-2.0).
        AssetLicenceTag::PermissiveReady
    }

    pub const fn licence_spdx(self) -> &'static str {
        match self {
            Self::Yunet => "MIT",
            Self::Sface
            | Self::MediapipeFaceLandmarker
            | Self::YoloNasS
            | Self::EmotionsRetail
            | Self::MediapipePose
            | Self::MediapipeHands => "Apache-2.0",
        }
    }

    /// Relative dir under vendor/vision/ and expected filename.
    pub const fn rel_parts(self) -> (&'static str, &'static str) {
        match self {
            Self::Yunet => ("face/yunet", "face_detection_yunet_2023mar.onnx"),
            Self::Sface => ("face/sface", "face_recognition_sface_2021dec.onnx"),
            Self::MediapipeFaceLandmarker => ("face/mediapipe_landmarker", "face_landmarker.task"),
            Self::YoloNasS => ("detect/yolo_nas", "yolo_nas_s.onnx"),
            Self::EmotionsRetail => (
                "affect/openvino_emotions_retail",
                "emotions-recognition-retail-0003.onnx",
            ),
            Self::MediapipePose => ("pose/mediapipe_pose", "pose_landmarker_lite.task"),
            Self::MediapipeHands => ("pose/mediapipe_hands", "hand_landmarker.task"),
        }
    }
}

/// Honest diligence tags — not a paywall for MIT/Apache zoo models.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetLicenceTag {
    /// MIT / Apache-2.0 (or equivalent) weights may be used when present.
    PermissiveReady,
    /// True non-commercial / hostile weight licence (avoid as product default).
    LicenceHostile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedAsset {
    pub id: VisionAssetId,
    pub path: PathBuf,
    pub licence_spdx: &'static str,
    pub licence_tag: AssetLicenceTag,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisionAssetError {
    /// File not on disk under any root — run vendor/vision/download.ps1.
    WeightAbsent,
    /// No search roots supplied.
    NoRoots,
}

/// Search roots in order (typically vendor/vision, bundled/models/vision, storage).
///
/// For each root, tries `{root}/{rel_dir}/{filename}`.
pub fn resolve_vision_asset(
    id: VisionAssetId,
    roots: &[&Path],
) -> Result<ResolvedAsset, VisionAssetError> {
    if roots.is_empty() {
        return Err(VisionAssetError::NoRoots);
    }
    let (rel_dir, filename) = id.rel_parts();
    for root in roots {
        let candidate = root.join(rel_dir).join(filename);
        if candidate.is_file() {
            return Ok(ResolvedAsset {
                id,
                path: candidate,
                licence_spdx: id.licence_spdx(),
                licence_tag: id.licence_tag(),
            });
        }
        // Also allow flat layout: {root}/{filename}
        let flat = root.join(filename);
        if flat.is_file() {
            return Ok(ResolvedAsset {
                id,
                path: flat,
                licence_spdx: id.licence_spdx(),
                licence_tag: id.licence_tag(),
            });
        }
    }
    Err(VisionAssetError::WeightAbsent)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn yunet_is_mit_permissive() {
        assert_eq!(VisionAssetId::Yunet.licence_spdx(), "MIT");
        assert_eq!(
            VisionAssetId::Yunet.licence_tag(),
            AssetLicenceTag::PermissiveReady
        );
    }

    #[test]
    fn sface_is_apache_permissive() {
        assert_eq!(VisionAssetId::Sface.licence_spdx(), "Apache-2.0");
        assert_eq!(
            VisionAssetId::Sface.licence_tag(),
            AssetLicenceTag::PermissiveReady
        );
    }

    #[test]
    fn absent_without_file() {
        let tmp = std::env::temp_dir().join(format!(
            "qv_asset_none_{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::create_dir_all(&tmp);
        let r = resolve_vision_asset(VisionAssetId::Yunet, &[tmp.as_path()]);
        assert_eq!(r, Err(VisionAssetError::WeightAbsent));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn finds_nested_layout() {
        let tmp = std::env::temp_dir().join(format!(
            "qv_asset_ok_{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let (rel, name) = VisionAssetId::Yunet.rel_parts();
        let dir = tmp.join(rel);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        fs::write(&path, b"fake-onnx-bytes-for-test").unwrap();
        let r = resolve_vision_asset(VisionAssetId::Yunet, &[tmp.as_path()]).unwrap();
        assert_eq!(r.path, path);
        assert_eq!(r.licence_tag, AssetLicenceTag::PermissiveReady);
        let _ = fs::remove_dir_all(&tmp);
    }
}
