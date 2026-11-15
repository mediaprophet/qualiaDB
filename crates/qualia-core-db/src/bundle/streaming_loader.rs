//! Progressive HTTP Range / OPFS slice streaming loader for `.hmc` bundles.
//!
//! Fetches the 64-byte `.hmc` header and CBOR index footer first, then schedules
//! prioritized byte-range requests across three tiers:
//! 1. [`StreamPriorityTier::Tier0SemanticAndProcedural`] — `.q42` semantic slices,
//!    manifests, and `.10d` `EONT1` / `PROC1` / `SKL1` / `MRP1` / `IMPO` / `SURF` sections.
//! 2. [`StreamPriorityTier::Tier1MidLodMeshes`] — Mid-LOD (`LOD2` / `LOD1`) mesh sections.
//! 3. [`StreamPriorityTier::Tier2HighPolyAndMip0`] — High-poly `LOD0` geometry and `KTX2` mip0.
//!
//! Every range arrival is verified against either the entry's SHA-256 digest or the
//! `.10d` section's hardware-accelerated CRC-32C checksum before admission.

use sha2::{Digest, Sha256};

use crate::container_10d::crc32c::crc32c;
use crate::container_10d::{
    Container10dHeader, HEADER_BYTE_SIZE, MAGIC_10D, SECTION_DESCRIPTOR_SIZE, SectionDescriptor,
    SectionType,
};

use super::format::{BundleEntry, BundleError, SHA256_LEN};
#[cfg(test)]
use super::format::BUNDLE_HEADER_SIZE;
use super::stream::index::{
    CheckedHeader, ExtentLimit, IndexError, IndexReadLimits, admit_header, decode_owned_index,
};

/// Priority tier for progressive `.hmc` range hydration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum StreamPriorityTier {
    /// Immediate semantic + procedural + impostor/surfel readiness (`< 50 KB`).
    Tier0SemanticAndProcedural = 0,
    /// Mid-LOD (`LOD2` / `LOD1`) geometry and materials.
    Tier1MidLodMeshes = 1,
    /// High-poly `LOD0` geometry, `KTX2` mip0 textures, and bulk weights.
    Tier2HighPolyAndMip0 = 2,
}

/// A single byte-range request within a `.hmc` bundle (`[offset .. offset + length)`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HmcRangeRequest {
    pub entry_key: String,
    pub entry_kind: String,
    pub tier: StreamPriorityTier,
    pub offset: u64,
    pub length: u64,
    /// Expected SHA-256 when the range covers an entire `BundleEntry`.
    pub expected_sha256: Option<[u8; SHA256_LEN]>,
    /// Expected CRC-32C when the range covers an interior `.10d` section.
    pub expected_crc32c: Option<u32>,
    /// Optional `.10d` section type tag (`SectionType as u16`).
    pub section_type_raw: Option<u16>,
}

pub use super::stream::index::HmcHeaderProbe;

/// Progressive range planner constructed from the 64B `.hmc` header and CBOR index footer.
#[derive(Debug, Clone)]
pub struct HmcProgressiveRangePlanner {
    header: HmcHeaderProbe,
    entries: Vec<BundleEntry>,
}

impl HmcProgressiveRangePlanner {
    /// Construct a planner from the 64-byte header probe and the fetched CBOR index footer bytes.
    pub fn from_header_and_index(
        header: HmcHeaderProbe,
        index_footer_bytes: &[u8],
    ) -> Result<Self, BundleError> {
        Self::from_header_and_index_with_limits(
            header,
            index_footer_bytes,
            header.total_length,
            ExtentLimit::default(),
            IndexReadLimits::default(),
        )
    }

    /// Bounded structural admission. The public probe cannot prove original reserved bytes.
    /// expected_extent must be pinned independently by the caller for a remote object.
    pub fn from_header_and_index_with_limits(
        header: HmcHeaderProbe,
        index_footer_bytes: &[u8],
        expected_extent: u64,
        extent_limit: ExtentLimit,
        limits: IndexReadLimits,
    ) -> Result<Self, BundleError> {
        let checked = CheckedHeader::from_probe(header, expected_extent, extent_limit, limits)
            .map_err(IndexError::into_bundle_error)?;
        Self::from_checked_index(checked, index_footer_bytes, limits)
    }

    /// Preferred remote facade: admit exact original header bytes and an independent extent.
    /// Success validates directory structure; payload CRC/hash/proofs remain incomplete.
    pub fn from_raw_header_and_index_with_limits(
        header_bytes: &[u8],
        index_footer_bytes: &[u8],
        expected_extent: u64,
        extent_limit: ExtentLimit,
        limits: IndexReadLimits,
    ) -> Result<Self, BundleError> {
        let checked = admit_header(header_bytes, expected_extent, extent_limit, limits)
            .map_err(IndexError::into_bundle_error)?;
        Self::from_checked_index(checked, index_footer_bytes, limits)
    }

    fn from_checked_index(
        checked: CheckedHeader,
        bytes: &[u8],
        limits: IndexReadLimits,
    ) -> Result<Self, BundleError> {
        let entries =
            decode_owned_index(&checked, bytes, limits).map_err(IndexError::into_bundle_error)?;
        Ok(Self {
            header: checked.probe(),
            entries,
        })
    }

    /// Validated header probe.
    #[inline]
    pub fn header(&self) -> &HmcHeaderProbe {
        &self.header
    }

    /// Decoded index entries.
    #[inline]
    pub fn entries(&self) -> &[BundleEntry] {
        &self.entries
    }

    /// Classify an entry into its default [`StreamPriorityTier`] and produce whole-entry
    /// range requests sorted by `(tier, offset)`.
    pub fn plan_entry_ranges(&self) -> Vec<HmcRangeRequest> {
        let mut reqs = Vec::with_capacity(self.entries.len());
        for entry in &self.entries {
            let tier = classify_entry_tier(&entry.key, &entry.kind);
            let expected_sha256 = entry.sha256.as_slice().try_into().ok();
            reqs.push(HmcRangeRequest {
                entry_key: entry.key.clone(),
                entry_kind: entry.kind.clone(),
                tier,
                offset: entry.offset,
                length: entry.length,
                expected_sha256,
                expected_crc32c: None,
                section_type_raw: None,
            });
        }
        reqs.sort_by_key(|r| (r.tier, r.offset));
        reqs
    }

    /// Given the front prefix of a `.10d` entry inside the bundle (`>= 64 + section_count * 24` bytes),
    /// plan sub-entry section range requests ordered by [`StreamPriorityTier`].
    pub fn plan_10d_interior_section_ranges(
        &self,
        entry_key: &str,
        entry_prefix_bytes: &[u8],
    ) -> Result<Vec<HmcRangeRequest>, BundleError> {
        let entry = self
            .entries
            .iter()
            .find(|e| e.key == entry_key)
            .ok_or(BundleError::EmptyKey)?;
        if entry_prefix_bytes.len() < HEADER_BYTE_SIZE {
            return Err(BundleError::TooShort);
        }
        // Header validation also checks the table pointer against the supplied
        // slice. Include the available entry prefix, but never bytes belonging
        // to a following bundle entry.
        let prefix_len = entry_prefix_bytes
            .len()
            .min(usize::try_from(entry.length).unwrap_or(usize::MAX));
        let hdr = Container10dHeader::parse(&entry_prefix_bytes[..prefix_len])
            .map_err(|_| BundleError::BadMagic)?;
        if hdr.magic != MAGIC_10D {
            return Err(BundleError::BadMagic);
        }
        let sec_count = hdr.section_count as usize;
        let toc_start = hdr.section_table_offset as usize;
        let toc_end = sec_count
            .checked_mul(SECTION_DESCRIPTOR_SIZE)
            .and_then(|size| toc_start.checked_add(size))
            .ok_or_else(|| BundleError::EntryOutOfBounds {
                key: entry.key.clone(),
                offset: entry.offset,
                length: entry.length,
            })?;
        if toc_end as u64 > entry.length || entry_prefix_bytes.len() < toc_end {
            return Err(BundleError::TooShort);
        }
        let is_low_lod_key = entry.key.contains("lod1") || entry.key.contains("lod2");
        let mut out = Vec::with_capacity(sec_count);
        for i in 0..sec_count {
            let off = toc_start + i * SECTION_DESCRIPTOR_SIZE;
            let sec: SectionDescriptor = bytemuck::pod_read_unaligned(
                &entry_prefix_bytes[off..off + SECTION_DESCRIPTOR_SIZE],
            );
            let section_offset = u64::from(sec.byte_offset);
            let section_length = u64::from(sec.byte_length);
            if section_offset
                .checked_add(section_length)
                .is_none_or(|end| end > entry.length)
            {
                return Err(BundleError::EntryOutOfBounds {
                    key: entry.key.clone(),
                    offset: section_offset,
                    length: section_length,
                });
            }
            let absolute_offset = entry.offset.checked_add(section_offset).ok_or_else(|| {
                BundleError::EntryOutOfBounds {
                    key: entry.key.clone(),
                    offset: section_offset,
                    length: section_length,
                }
            })?;
            let tier = classify_10d_section_tier(u16::from(sec.section_type), is_low_lod_key);
            out.push(HmcRangeRequest {
                entry_key: entry.key.clone(),
                entry_kind: entry.kind.clone(),
                tier,
                offset: absolute_offset,
                length: section_length,
                expected_sha256: None,
                expected_crc32c: Some(sec.crc32c),
                section_type_raw: Some(u16::from(sec.section_type)),
            });
        }
        out.sort_by_key(|r| (r.tier, r.offset));
        Ok(out)
    }
}

/// Classify a bundle entry into a progressive priority tier.
pub fn classify_entry_tier(key: &str, kind: &str) -> StreamPriorityTier {
    let lower_key = key.to_ascii_lowercase();
    let lower_kind = kind.to_ascii_lowercase();
    if lower_kind == "q42"
        || lower_kind == "manifest"
        || lower_kind.contains("cbor")
        || lower_key.ends_with(".q42")
        || lower_key.contains("manifest")
    {
        return StreamPriorityTier::Tier0SemanticAndProcedural;
    }
    if lower_key.contains("lod1") || lower_key.contains("lod2") {
        return StreamPriorityTier::Tier1MidLodMeshes;
    }
    if lower_kind == "ktx2" || lower_kind == "p64" || lower_key.contains("lod0") {
        return StreamPriorityTier::Tier2HighPolyAndMip0;
    }
    StreamPriorityTier::Tier1MidLodMeshes
}

/// Classify a `.10d` interior section into a progressive priority tier.
pub fn classify_10d_section_tier(
    section_type_raw: u16,
    is_mid_lod_entry: bool,
) -> StreamPriorityTier {
    if section_type_raw == SectionType::MORPH_TARGETS_DISCRIMINANT {
        return StreamPriorityTier::Tier0SemanticAndProcedural;
    }
    let Ok(raw_u8) = u8::try_from(section_type_raw) else {
        return StreamPriorityTier::Tier1MidLodMeshes;
    };
    match SectionType::from_u8(raw_u8) {
        Some(
            SectionType::EntityOntology
            | SectionType::ProceduralSynthesis
            | SectionType::SkeletalAnimation
            | SectionType::ImpostorAtlas
            | SectionType::HierarchicalSurfels
            | SectionType::Tensor10DNodes
            | SectionType::ManifoldIdentityV2
            | SectionType::ProvenanceSidecar,
        ) => StreamPriorityTier::Tier0SemanticAndProcedural,
        Some(
            SectionType::QuantizedMesh
            | SectionType::Materials
            | SectionType::TextureCoordinates
            | SectionType::FieldSidecar
            | SectionType::HyperspectralCube
            | SectionType::VoxelLattice,
        ) => {
            if is_mid_lod_entry {
                StreamPriorityTier::Tier1MidLodMeshes
            } else {
                StreamPriorityTier::Tier2HighPolyAndMip0
            }
        }
        _ => StreamPriorityTier::Tier1MidLodMeshes,
    }
}

/// Verify an arrived byte range against its `HmcRangeRequest` integrity digest (SHA-256 and/or CRC-32C).
pub fn verify_arrived_range(
    req: &HmcRangeRequest,
    arrived_bytes: &[u8],
) -> Result<(), BundleError> {
    if req.expected_sha256.is_none() && req.expected_crc32c.is_none() {
        return Err(BundleError::Cbor(
            "Range arrival requires an integrity digest".into(),
        ));
    }
    if arrived_bytes.len() as u64 != req.length {
        return Err(BundleError::LengthMismatch {
            header: req.length,
            actual: arrived_bytes.len(),
        });
    }
    if let Some(expected_crc) = req.expected_crc32c {
        let got_crc = crc32c(arrived_bytes);
        if got_crc != expected_crc {
            return Err(BundleError::CrcMismatch {
                expected: expected_crc,
                got: got_crc,
            });
        }
    }
    if let Some(expected_sha) = req.expected_sha256 {
        let mut hasher = Sha256::new();
        hasher.update(arrived_bytes);
        let digest = hasher.finalize();
        if digest.as_slice() != expected_sha.as_slice() {
            return Err(BundleError::CrcMismatch {
                expected: u32::from_le_bytes([
                    expected_sha[0],
                    expected_sha[1],
                    expected_sha[2],
                    expected_sha[3],
                ]),
                got: u32::from_le_bytes([digest[0], digest[1], digest[2], digest[3]]),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "streaming_loader/tests.rs"]
mod tests;
