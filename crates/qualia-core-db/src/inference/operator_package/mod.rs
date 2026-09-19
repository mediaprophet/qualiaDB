//! Companion operator package runtime and serialization (W2: EO-02 / EOS-023).
//!
//! Provides the format container for companion operator packages alongside P64/GGUF models:
//! - Manifest v1 with 64-bit segment descriptors (`manifest`).
//! - Checked 64-bit segment addressing and WASM addressability protection (`segment`).
//! - GGML Q4_K streaming bit-plane and scale decomposition (`q4k_repack`).
//! - Safe reading, writing, and zero-copy borrowing of segment payloads.

pub mod delta_dist;
pub mod lossless_tile;
pub mod manifest;
pub mod q4k_repack;
pub mod segment;

#[cfg(test)]
mod tests;

use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;

pub use delta_dist::{
    ContentAddressedAnchor, DeltaPackageManifest, DistributionError, DistributionReceipt,
    DistributionResolver, ResolvedSpecialist, SpecialistDelta,
};
pub use lossless_tile::{
    decode_tile_into, encode_tile_raw, encode_tile_shared, encode_tile_split_f16, encode_tile_xor,
    TileCodecError, TileCodecKind, TileHeader,
};
pub use manifest::{
    FidelityContract, OperatorPackageManifest, OperatorRecord, PackageError,
    OPERATOR_PACKAGE_MAGIC, OPERATOR_PACKAGE_VERSION,
};
pub use q4k_repack::{
    reconstruct_source_q4k, repack_q4k_tensor, verify_q4k_repack_roundtrip, RepackError,
    RepackedQ4KTensor, Q4K_BITPLANE_BYTES_PER_BLOCK, Q4K_SCALE_PLANE_BYTES_PER_BLOCK,
};
pub use segment::{
    compute_segment_checksum, SegmentDescriptor, SegmentError, SegmentKind, SegmentView,
};

/// An in-memory, validated companion operator package borrowing or owning its byte backing.
#[derive(Debug, Clone)]
pub struct OperatorPackage {
    manifest: OperatorPackageManifest,
    data: Vec<u8>,
}

impl OperatorPackage {
    /// Parse and validate a companion package from full container bytes.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, PackageError> {
        let manifest = OperatorPackageManifest::decode(&bytes)?;

        // Validate all segments against the payload size and platform constraints
        for seg in &manifest.segments {
            seg.check_bounds(bytes.len() as u64)
                .map_err(|e| match e {
                    SegmentError::OutOfBounds { .. } => PackageError::TruncatedInput,
                    _ => PackageError::MalformedDescriptor(
                        qualia_inference_kernel::operators::OperatorError::InvalidPayload,
                    ),
                })?;
            seg.check_platform_fit()
                .map_err(|_| PackageError::MalformedDescriptor(
                    qualia_inference_kernel::operators::OperatorError::InvalidPayload,
                ))?;
        }

        Ok(Self {
            manifest,
            data: bytes,
        })
    }

    pub fn manifest(&self) -> &OperatorPackageManifest {
        &self.manifest
    }

    pub fn raw_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Obtain a safe, borrowed view into a specific segment, verifying checksum if present.
    pub fn get_segment_view(&self, segment_id: u32) -> Result<SegmentView<'_>, SegmentError> {
        let desc = self
            .manifest
            .find_segment(segment_id)
            .ok_or(SegmentError::OutOfBounds {
                offset: 0,
                length: 0,
                total: self.data.len() as u64,
            })?;
        SegmentView::try_new(*desc, &self.data, true)
    }
}

/// Package builder to construct a companion package binary with aligned segments.
pub struct PackageBuilder {
    manifest: OperatorPackageManifest,
    segments_payload: Vec<(u32, SegmentKind, Vec<u8>)>,
}

impl PackageBuilder {
    pub fn new(
        source_digest: u64,
        representation_digest: u64,
        fidelity_contract: FidelityContract,
    ) -> Self {
        Self {
            manifest: OperatorPackageManifest::new(
                source_digest,
                representation_digest,
                fidelity_contract,
            ),
            segments_payload: Vec::new(),
        }
    }

    pub fn add_operator(&mut self, record: OperatorRecord) -> Result<(), PackageError> {
        self.manifest.add_operator(record)
    }

    pub fn add_segment_payload(
        &mut self,
        segment_id: u32,
        kind: SegmentKind,
        payload: Vec<u8>,
    ) -> Result<(), PackageError> {
        if self.segments_payload.iter().any(|(id, _, _)| *id == segment_id) {
            return Err(PackageError::DuplicateSegmentId(segment_id));
        }
        self.segments_payload.push((segment_id, kind, payload));
        Ok(())
    }

    /// Build the binary package, calculating 64-bit offsets and 32-bit checksums.
    /// Payload segments are 64-byte aligned for cache-line DMA efficiency.
    pub fn build(mut self) -> Result<Vec<u8>, PackageError> {
        // First, temporarily encode manifest with placeholder segments to estimate header size
        for (id, kind, payload) in &self.segments_payload {
            let checksum = compute_segment_checksum(payload);
            self.manifest.add_segment(SegmentDescriptor::new(
                *id,
                *kind,
                0,
                payload.len() as u64,
                checksum,
            ))?;
        }

        let initial_manifest_bytes = self.manifest.encode();
        let manifest_len = initial_manifest_bytes.len();

        // 64-byte align the start of the payload data
        let payload_start = (manifest_len + 63) & !63;

        // Re-compute exact offsets
        let mut current_offset = payload_start as u64;
        self.manifest.segments.clear();
        for (id, kind, payload) in &self.segments_payload {
            let checksum = compute_segment_checksum(payload);
            self.manifest.add_segment(SegmentDescriptor::new(
                *id,
                *kind,
                current_offset,
                payload.len() as u64,
                checksum,
            ))?;
            // 64-byte align each subsequent segment
            let next_offset = current_offset + payload.len() as u64;
            current_offset = (next_offset + 63) & !63;
        }

        let final_manifest = self.manifest.encode();
        assert!(final_manifest.len() <= payload_start, "manifest length drift");

        let total_size = current_offset as usize;
        let mut out = vec![0u8; total_size];

        // Write final manifest
        out[..final_manifest.len()].copy_from_slice(&final_manifest);

        // Write segments
        for (seg_desc, (_, _, payload)) in self.manifest.segments.iter().zip(&self.segments_payload) {
            let start = seg_desc.offset as usize;
            let end = start + payload.len();
            out[start..end].copy_from_slice(payload);
        }

        Ok(out)
    }

    /// Build and write package directly to a file.
    pub fn write_to_file(self, path: &Path) -> io::Result<()> {
        let bytes = self.build().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let mut f = File::create(path)?;
        f.write_all(&bytes)?;
        f.flush()?;
        Ok(())
    }
}

/// Read and validate an operator package from a file.
pub fn load_package_from_file(path: &Path) -> io::Result<OperatorPackage> {
    let mut f = File::open(path)?;
    let mut data = Vec::new();
    f.read_to_end(&mut data)?;
    OperatorPackage::from_bytes(data)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}
