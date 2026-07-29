//! Cold-path ONNX file validation (no runtime inference).
//!
//! PermissiveReady zoo weights: check magic + size so adapters fail closed
//! before claiming ProductionWeights. Actual ORT/tract session is feature work.

use std::fs;
use std::path::Path;

/// ONNX protobuf files typically start with field tags; many begin with 0x08
/// after optional size, but OpenCV Zoo files often start with `\x08` varint
/// or the ASCII is not reliable. We require: readable file, min size, and
/// either protobuf-ish leading bytes or presence of "onnx" string in header.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnnxFileMeta {
    pub bytes: usize,
    pub looks_like_onnx: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnnxLoadError {
    Io,
    TooSmall,
    NotOnnxLike,
}

const MIN_ONNX_BYTES: usize = 256;

/// Load entire ONNX file into memory (cold path; weights stay outside NQuin).
pub fn load_onnx_file(path: &Path) -> Result<(Vec<u8>, OnnxFileMeta), OnnxLoadError> {
    let data = fs::read(path).map_err(|_| OnnxLoadError::Io)?;
    let meta = validate_onnx_bytes(&data)?;
    Ok((data, meta))
}

/// Validate bytes without reading disk.
pub fn validate_onnx_bytes(data: &[u8]) -> Result<OnnxFileMeta, OnnxLoadError> {
    if data.len() < MIN_ONNX_BYTES {
        return Err(OnnxLoadError::TooSmall);
    }
    let looks = looks_like_onnx(data);
    if !looks {
        return Err(OnnxLoadError::NotOnnxLike);
    }
    Ok(OnnxFileMeta {
        bytes: data.len(),
        looks_like_onnx: true,
    })
}

fn looks_like_onnx(data: &[u8]) -> bool {
    // Scan first 512 bytes for "onnx" or "ONNX" or GraphProto-ish patterns.
    let n = data.len().min(512);
    let head = &data[..n];
    if find_subslice(head, b"onnx") || find_subslice(head, b"ONNX") {
        return true;
    }
    // Protobuf ModelProto often starts with 0x08 (ir_version field).
    if head[0] == 0x08 || head[0] == 0x0a {
        return true;
    }
    false
}

fn find_subslice(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::weights::{resolve_vision_asset, VisionAssetId};
    use std::path::PathBuf;

    fn vendor_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/vision")
    }

    #[test]
    fn reject_tiny() {
        assert_eq!(
            validate_onnx_bytes(&[0u8; 10]),
            Err(OnnxLoadError::TooSmall)
        );
    }

    #[test]
    fn yunet_on_disk_validates_if_present() {
        let root = vendor_root();
        match resolve_vision_asset(VisionAssetId::Yunet, &[root.as_path()]) {
            Ok(a) => {
                let (_b, meta) = load_onnx_file(&a.path).expect("yunet load");
                assert!(meta.looks_like_onnx);
                assert!(meta.bytes > 10_000);
            }
            Err(_) => {
                // WeightAbsent on CI without download — skip
            }
        }
    }

    #[test]
    fn sface_on_disk_validates_if_present() {
        let root = vendor_root();
        match resolve_vision_asset(VisionAssetId::Sface, &[root.as_path()]) {
            Ok(a) => {
                let (_b, meta) = load_onnx_file(&a.path).expect("sface load");
                assert!(meta.bytes > 1_000_000);
            }
            Err(_) => {}
        }
    }
}
