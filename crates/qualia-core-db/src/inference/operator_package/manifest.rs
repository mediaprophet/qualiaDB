//! Companion operator package manifest v1 (W2: EOS-020).
//!
//! Declarative operator metadata, source provenance, fidelity contract,
//! and 64-bit segment references.

use std::fmt;
use serde::{Deserialize, Serialize};
use qualia_inference_kernel::operators::{
    AccumKind, OperatorDescriptor, OperatorError, OperatorKind, ScaleLayout,
};
use super::segment::{SegmentDescriptor, SegmentKind};

pub const OPERATOR_PACKAGE_MAGIC: [u8; 4] = *b"QOP1";
pub const OPERATOR_PACKAGE_VERSION: u32 = 1;

/// Maximum allowed operators or segments in a single package manifest (prevents OOM on corrupt input).
pub const MAX_MANIFEST_ENTRIES: usize = 65536;

/// Declared fidelity contract for the package artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum FidelityContract {
    /// Reconstructs exact source tensor bytes (including scales and special floats).
    SourceBytePreserving = 1,
    /// Implements declared operator behavior within bounded numerical tolerances.
    OperatorFidelity = 2,
    /// Evaluates within declared downstream task quality budgets.
    ModelQuality = 3,
}

impl FidelityContract {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            1 => Some(Self::SourceBytePreserving),
            2 => Some(Self::OperatorFidelity),
            3 => Some(Self::ModelQuality),
            _ => None,
        }
    }
}

/// Errors occurring during manifest encoding or decoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageError {
    InvalidMagic([u8; 4]),
    UnsupportedVersion(u32),
    UnsupportedFidelity(u8),
    UnsupportedKind(u8),
    UnsupportedScaleLayout(u8),
    UnsupportedAccum(u8),
    UnsupportedSegmentKind(u8),
    MalformedDescriptor(OperatorError),
    TruncatedInput,
    InvalidEntryCount(usize),
    InvalidStringUtf8,
    DuplicateSegmentId(u32),
    SegmentNotFound(u32),
}

impl fmt::Display for PackageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMagic(m) => write!(f, "invalid package magic: {:?}", m),
            Self::UnsupportedVersion(v) => write!(f, "unsupported package version: {v}"),
            Self::UnsupportedFidelity(fc) => write!(f, "unsupported fidelity contract id: {fc}"),
            Self::UnsupportedKind(k) => write!(f, "unsupported operator kind id: {k}"),
            Self::UnsupportedScaleLayout(sl) => write!(f, "unsupported scale layout id: {sl}"),
            Self::UnsupportedAccum(a) => write!(f, "unsupported accum kind id: {a}"),
            Self::UnsupportedSegmentKind(sk) => write!(f, "unsupported segment kind id: {sk}"),
            Self::MalformedDescriptor(oe) => write!(f, "malformed operator descriptor: {oe:?}"),
            Self::TruncatedInput => write!(f, "manifest input data is truncated"),
            Self::InvalidEntryCount(c) => write!(f, "entry count exceeds manifest limit: {c}"),
            Self::InvalidStringUtf8 => write!(f, "invalid utf-8 string in manifest"),
            Self::DuplicateSegmentId(id) => write!(f, "duplicate segment id in manifest: {id}"),
            Self::SegmentNotFound(id) => write!(f, "referenced segment id not found: {id}"),
        }
    }
}

impl std::error::Error for PackageError {}

/// One operator entry in the companion package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorRecord {
    pub tensor_role: u16,
    pub name: String,
    pub descriptor: OperatorDescriptor,
    pub source_segment_id: u32,
    pub primary_segment_id: u32,
    pub scale_segment_id: u32,
}

/// Package manifest holding metadata, operators, and 64-bit segment descriptors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorPackageManifest {
    pub magic: [u8; 4],
    pub version: u32,
    pub source_digest: u64,
    pub representation_digest: u64,
    pub fidelity_contract: FidelityContract,
    pub operators: Vec<OperatorRecord>,
    pub segments: Vec<SegmentDescriptor>,
}

impl OperatorPackageManifest {
    pub fn new(
        source_digest: u64,
        representation_digest: u64,
        fidelity_contract: FidelityContract,
    ) -> Self {
        Self {
            magic: OPERATOR_PACKAGE_MAGIC,
            version: OPERATOR_PACKAGE_VERSION,
            source_digest,
            representation_digest,
            fidelity_contract,
            operators: Vec::new(),
            segments: Vec::new(),
        }
    }

    pub fn add_segment(&mut self, segment: SegmentDescriptor) -> Result<(), PackageError> {
        if self.segments.iter().any(|s| s.segment_id == segment.segment_id) {
            return Err(PackageError::DuplicateSegmentId(segment.segment_id));
        }
        self.segments.push(segment);
        Ok(())
    }

    pub fn add_operator(&mut self, record: OperatorRecord) -> Result<(), PackageError> {
        record
            .descriptor
            .check_shape()
            .map_err(PackageError::MalformedDescriptor)?;
        self.operators.push(record);
        Ok(())
    }

    pub fn find_segment(&self, segment_id: u32) -> Option<&SegmentDescriptor> {
        self.segments.iter().find(|s| s.segment_id == segment_id)
    }

    /// Serialize manifest to little-endian binary bytes.
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(256);
        buf.extend_from_slice(&self.magic);
        buf.extend_from_slice(&self.version.to_le_bytes());
        buf.extend_from_slice(&self.source_digest.to_le_bytes());
        buf.extend_from_slice(&self.representation_digest.to_le_bytes());
        buf.push(self.fidelity_contract as u8);
        buf.extend_from_slice(&[0u8; 3]); // padding

        // Operators
        buf.extend_from_slice(&(self.operators.len() as u32).to_le_bytes());
        for op in &self.operators {
            buf.extend_from_slice(&op.tensor_role.to_le_bytes());
            let name_bytes = op.name.as_bytes();
            buf.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
            buf.extend_from_slice(name_bytes);

            // Descriptor
            buf.push(op.descriptor.kind as u8);
            buf.extend_from_slice(&op.descriptor.in_features.to_le_bytes());
            buf.extend_from_slice(&op.descriptor.out_features.to_le_bytes());
            buf.extend_from_slice(&op.descriptor.batch_hint.to_le_bytes());
            buf.extend_from_slice(&op.descriptor.tile_elems.to_le_bytes());
            buf.push(op.descriptor.scale_layout as u8);
            buf.push(op.descriptor.accum as u8);
            buf.extend_from_slice(&op.descriptor.max_workspace_bytes.to_le_bytes());
            buf.extend_from_slice(&op.descriptor.representation_digest.to_le_bytes());

            buf.extend_from_slice(&op.source_segment_id.to_le_bytes());
            buf.extend_from_slice(&op.primary_segment_id.to_le_bytes());
            buf.extend_from_slice(&op.scale_segment_id.to_le_bytes());
        }

        // Segments
        buf.extend_from_slice(&(self.segments.len() as u32).to_le_bytes());
        for seg in &self.segments {
            buf.extend_from_slice(&seg.segment_id.to_le_bytes());
            buf.push(seg.kind as u8);
            buf.extend_from_slice(&[0u8; 3]); // padding
            buf.extend_from_slice(&seg.offset.to_le_bytes());
            buf.extend_from_slice(&seg.length.to_le_bytes());
            buf.extend_from_slice(&seg.checksum.to_le_bytes());
            buf.extend_from_slice(&[0u8; 4]); // alignment padding
        }

        buf
    }

    /// Deserialize manifest from little-endian binary bytes.
    pub fn decode(bytes: &[u8]) -> Result<Self, PackageError> {
        if bytes.len() < 32 {
            return Err(PackageError::TruncatedInput);
        }

        let magic: [u8; 4] = [bytes[0], bytes[1], bytes[2], bytes[3]];
        if magic != OPERATOR_PACKAGE_MAGIC {
            return Err(PackageError::InvalidMagic(magic));
        }

        let version = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        if version != OPERATOR_PACKAGE_VERSION {
            return Err(PackageError::UnsupportedVersion(version));
        }

        let source_digest = u64::from_le_bytes(bytes[8..16].try_into().unwrap());
        let representation_digest = u64::from_le_bytes(bytes[16..24].try_into().unwrap());
        let fidelity_contract = FidelityContract::from_u8(bytes[24])
            .ok_or(PackageError::UnsupportedFidelity(bytes[24]))?;

        let mut pos = 28usize; // 25 + 3 padding
        if bytes.len() < pos + 4 {
            return Err(PackageError::TruncatedInput);
        }

        let op_count = u32::from_le_bytes(bytes[pos..pos + 4].try_into().unwrap()) as usize;
        pos += 4;
        if op_count > MAX_MANIFEST_ENTRIES {
            return Err(PackageError::InvalidEntryCount(op_count));
        }

        let mut operators = Vec::with_capacity(op_count);
        for _ in 0..op_count {
            if bytes.len() < pos + 4 {
                return Err(PackageError::TruncatedInput);
            }
            let tensor_role = u16::from_le_bytes(bytes[pos..pos + 2].try_into().unwrap());
            let name_len = u16::from_le_bytes(bytes[pos + 2..pos + 4].try_into().unwrap()) as usize;
            pos += 4;

            if bytes.len() < pos + name_len {
                return Err(PackageError::TruncatedInput);
            }
            let name = std::str::from_utf8(&bytes[pos..pos + name_len])
                .map_err(|_| PackageError::InvalidStringUtf8)?
                .to_string();
            pos += name_len;

            // Descriptor (kind 1 + in 4 + out 4 + batch 4 + tile 4 + scale 1 + accum 1 + max_ws 4 + rep 8 = 31 bytes)
            // Plus segment ids (source 4 + primary 4 + scale 4 = 12 bytes) => 43 bytes total
            if bytes.len() < pos + 43 {
                return Err(PackageError::TruncatedInput);
            }

            let kind_raw = bytes[pos];
            let kind = match kind_raw {
                1 => OperatorKind::DenseF32,
                2 => OperatorKind::Q4KBitPlane,
                other => return Err(PackageError::UnsupportedKind(other)),
            };
            let in_features = u32::from_le_bytes(bytes[pos + 1..pos + 5].try_into().unwrap());
            let out_features = u32::from_le_bytes(bytes[pos + 5..pos + 9].try_into().unwrap());
            let batch_hint = u32::from_le_bytes(bytes[pos + 9..pos + 13].try_into().unwrap());
            let tile_elems = u32::from_le_bytes(bytes[pos + 13..pos + 17].try_into().unwrap());
            let scale_layout = match bytes[pos + 17] {
                0 => ScaleLayout::None,
                1 => ScaleLayout::GgmlQ4K,
                other => return Err(PackageError::UnsupportedScaleLayout(other)),
            };
            let accum = match bytes[pos + 18] {
                1 => AccumKind::F32,
                other => return Err(PackageError::UnsupportedAccum(other)),
            };
            let max_workspace_bytes = u32::from_le_bytes(bytes[pos + 19..pos + 23].try_into().unwrap());
            let op_rep_digest = u64::from_le_bytes(bytes[pos + 23..pos + 31].try_into().unwrap());

            let descriptor = OperatorDescriptor {
                kind,
                in_features,
                out_features,
                batch_hint,
                tile_elems,
                scale_layout,
                accum,
                max_workspace_bytes,
                representation_digest: op_rep_digest,
            };
            descriptor
                .check_shape()
                .map_err(PackageError::MalformedDescriptor)?;

            let source_segment_id = u32::from_le_bytes(bytes[pos + 31..pos + 35].try_into().unwrap());
            let primary_segment_id = u32::from_le_bytes(bytes[pos + 35..pos + 39].try_into().unwrap());
            let scale_segment_id = u32::from_le_bytes(bytes[pos + 39..pos + 43].try_into().unwrap());
            pos += 43;

            operators.push(OperatorRecord {
                tensor_role,
                name,
                descriptor,
                source_segment_id,
                primary_segment_id,
                scale_segment_id,
            });
        }

        if bytes.len() < pos + 4 {
            return Err(PackageError::TruncatedInput);
        }
        let seg_count = u32::from_le_bytes(bytes[pos..pos + 4].try_into().unwrap()) as usize;
        pos += 4;
        if seg_count > MAX_MANIFEST_ENTRIES {
            return Err(PackageError::InvalidEntryCount(seg_count));
        }

        let mut segments = Vec::with_capacity(seg_count);
        for _ in 0..seg_count {
            // Segment entry: id 4 + kind 1 + pad 3 + offset 8 + len 8 + csum 4 + pad 4 = 32 bytes
            if bytes.len() < pos + 32 {
                return Err(PackageError::TruncatedInput);
            }
            let segment_id = u32::from_le_bytes(bytes[pos..pos + 4].try_into().unwrap());
            let kind_raw = bytes[pos + 4];
            let kind = SegmentKind::from_u8(kind_raw)
                .ok_or(PackageError::UnsupportedSegmentKind(kind_raw))?;
            let offset = u64::from_le_bytes(bytes[pos + 8..pos + 16].try_into().unwrap());
            let length = u64::from_le_bytes(bytes[pos + 16..pos + 24].try_into().unwrap());
            let checksum = u32::from_le_bytes(bytes[pos + 24..pos + 28].try_into().unwrap());
            pos += 32;

            segments.push(SegmentDescriptor {
                segment_id,
                kind,
                offset,
                length,
                checksum,
            });
        }

        Ok(Self {
            magic,
            version,
            source_digest,
            representation_digest,
            fidelity_contract,
            operators,
            segments,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qualia_inference_kernel::operators::Q4K_SUPERBLOCK_ELEMS;

    fn sample_manifest() -> OperatorPackageManifest {
        let mut m = OperatorPackageManifest::new(
            0x1234_5678_9abc_def0,
            0xfeed_face_cafe_beef,
            FidelityContract::SourceBytePreserving,
        );
        m.add_segment(SegmentDescriptor::new(1, SegmentKind::SourcePayload, 64, 144, 0x1111)).unwrap();
        m.add_segment(SegmentDescriptor::new(2, SegmentKind::BitPlaneTiles, 208, 128, 0x2222)).unwrap();
        m.add_operator(OperatorRecord {
            tensor_role: 1,
            name: "layer.0.attn_q".to_string(),
            descriptor: OperatorDescriptor {
                kind: OperatorKind::Q4KBitPlane,
                in_features: 256,
                out_features: 1,
                batch_hint: 1,
                tile_elems: Q4K_SUPERBLOCK_ELEMS,
                scale_layout: ScaleLayout::GgmlQ4K,
                accum: AccumKind::F32,
                max_workspace_bytes: 0,
                representation_digest: 0xfeed_face_cafe_beef,
            },
            source_segment_id: 1,
            primary_segment_id: 2,
            scale_segment_id: 0,
        }).unwrap();
        m
    }

    #[test]
    fn manifest_encode_decode_round_trip() {
        let original = sample_manifest();
        let encoded = original.encode();
        let decoded = OperatorPackageManifest::decode(&encoded).expect("decode must succeed");
        assert_eq!(original, decoded);
    }

    #[test]
    fn invalid_magic_rejected() {
        let mut encoded = sample_manifest().encode();
        encoded[0] = b'X';
        assert!(matches!(
            OperatorPackageManifest::decode(&encoded),
            Err(PackageError::InvalidMagic(_))
        ));
    }

    #[test]
    fn unsupported_version_rejected() {
        let mut encoded = sample_manifest().encode();
        encoded[4] = 2; // version 2
        assert!(matches!(
            OperatorPackageManifest::decode(&encoded),
            Err(PackageError::UnsupportedVersion(2))
        ));
    }

    #[test]
    fn truncated_manifest_fails_closed() {
        let encoded = sample_manifest().encode();
        assert!(matches!(
            OperatorPackageManifest::decode(&encoded[..20]),
            Err(PackageError::TruncatedInput)
        ));
    }
}
