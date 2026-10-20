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
    Container10dHeader, SectionDescriptor, SectionType, HEADER_BYTE_SIZE, MAGIC_10D,
    SECTION_DESCRIPTOR_SIZE,
};

use super::format::{
    BundleEntry, BundleError, BUNDLE_HEADER_SIZE, BUNDLE_MAGIC, BUNDLE_VERSION, OFF_CRC32C,
    OFF_ENTRY_COUNT, OFF_FLAGS, OFF_INDEX_LENGTH, OFF_INDEX_OFFSET, OFF_MAGIC, OFF_TOTAL_LENGTH,
    OFF_VERSION, SHA256_LEN,
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

/// Validated 64-byte `.hmc` header metadata extracted from the initial `[0..64)` range read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HmcHeaderProbe {
    pub version: u16,
    pub flags: u16,
    pub entry_count: u32,
    pub index_offset: u64,
    pub index_length: u64,
    pub total_length: u64,
    pub whole_file_crc32c: u32,
}

impl HmcHeaderProbe {
    /// Parse the 64-byte front header of a `.hmc` bundle without needing the rest of the file.
    pub fn parse_header_range(header_bytes: &[u8]) -> Result<Self, BundleError> {
        if header_bytes.len() < BUNDLE_HEADER_SIZE {
            return Err(BundleError::TooShort);
        }
        if header_bytes[OFF_MAGIC..OFF_MAGIC + 4] != BUNDLE_MAGIC {
            return Err(BundleError::BadMagic);
        }
        let version = u16::from_le_bytes([header_bytes[OFF_VERSION], header_bytes[OFF_VERSION + 1]]);
        if version != BUNDLE_VERSION {
            return Err(BundleError::UnsupportedVersion(version));
        }
        let flags = u16::from_le_bytes([header_bytes[OFF_FLAGS], header_bytes[OFF_FLAGS + 1]]);
        let entry_count = u32::from_le_bytes(
            header_bytes[OFF_ENTRY_COUNT..OFF_ENTRY_COUNT + 4]
                .try_into()
                .unwrap(),
        );
        let index_offset = u64::from_le_bytes(
            header_bytes[OFF_INDEX_OFFSET..OFF_INDEX_OFFSET + 8]
                .try_into()
                .unwrap(),
        );
        let index_length = u64::from_le_bytes(
            header_bytes[OFF_INDEX_LENGTH..OFF_INDEX_LENGTH + 8]
                .try_into()
                .unwrap(),
        );
        let total_length = u64::from_le_bytes(
            header_bytes[OFF_TOTAL_LENGTH..OFF_TOTAL_LENGTH + 8]
                .try_into()
                .unwrap(),
        );
        let whole_file_crc32c = u32::from_le_bytes(
            header_bytes[OFF_CRC32C..OFF_CRC32C + 4]
                .try_into()
                .unwrap(),
        );
        let end = index_offset
            .checked_add(index_length)
            .ok_or(BundleError::BadIndexPointer {
                offset: index_offset,
                length: index_length,
                total: total_length as usize,
            })?;
        if index_offset < BUNDLE_HEADER_SIZE as u64 || end > total_length {
            return Err(BundleError::BadIndexPointer {
                offset: index_offset,
                length: index_length,
                total: total_length as usize,
            });
        }
        Ok(Self {
            version,
            flags,
            entry_count,
            index_offset,
            index_length,
            total_length,
            whole_file_crc32c,
        })
    }

    /// Byte range `(offset, length)` required to fetch the CBOR index footer.
    #[inline]
    pub fn index_footer_range(&self) -> (u64, u64) {
        (self.index_offset, self.index_length)
    }
}

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
        if index_footer_bytes.len() as u64 != header.index_length {
            return Err(BundleError::BadIndexPointer {
                offset: header.index_offset,
                length: header.index_length,
                total: index_footer_bytes.len(),
            });
        }
        let entries: Vec<BundleEntry> = ciborium::from_reader(index_footer_bytes)
            .map_err(|e| BundleError::Cbor(e.to_string()))?;
        for e in &entries {
            let e_end = e
                .offset
                .checked_add(e.length)
                .ok_or_else(|| BundleError::EntryOutOfBounds {
                    key: e.key.clone(),
                    offset: e.offset,
                    length: e.length,
                })?;
            if e.offset < BUNDLE_HEADER_SIZE as u64 || e_end > header.index_offset {
                return Err(BundleError::EntryOutOfBounds {
                    key: e.key.clone(),
                    offset: e.offset,
                    length: e.length,
                });
            }
        }
        Ok(Self { header, entries })
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

    /// Given the front prefix of a `.10d` entry inside the bundle (`>= 64 + section_count * 32` bytes),
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
        let hdr = Container10dHeader::parse(&entry_prefix_bytes[..HEADER_BYTE_SIZE])
            .map_err(|_| BundleError::BadMagic)?;
        if hdr.magic != MAGIC_10D {
            return Err(BundleError::BadMagic);
        }
        let sec_count = hdr.section_count as usize;
        let toc_start = hdr.section_table_offset as usize;
        let toc_end = toc_start + sec_count * SECTION_DESCRIPTOR_SIZE;
        if entry_prefix_bytes.len() < toc_end {
            return Err(BundleError::TooShort);
        }
        let is_low_lod_key = entry.key.contains("lod1") || entry.key.contains("lod2");
        let mut out = Vec::with_capacity(sec_count);
        for i in 0..sec_count {
            let off = toc_start + i * SECTION_DESCRIPTOR_SIZE;
            let sec: SectionDescriptor =
                bytemuck::pod_read_unaligned(&entry_prefix_bytes[off..off + SECTION_DESCRIPTOR_SIZE]);
            let tier = classify_10d_section_tier(u16::from(sec.section_type), is_low_lod_key);
            out.push(HmcRangeRequest {
                entry_key: entry.key.clone(),
                entry_kind: entry.kind.clone(),
                tier,
                offset: entry.offset + u64::from(sec.byte_offset),
                length: u64::from(sec.byte_length),
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
pub fn classify_10d_section_tier(section_type_raw: u16, is_mid_lod_entry: bool) -> StreamPriorityTier {
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
pub fn verify_arrived_range(req: &HmcRangeRequest, arrived_bytes: &[u8]) -> Result<(), BundleError> {
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
mod tests {
    use super::*;
    use crate::bundle::writer::BundleWriter;

    #[test]
    fn progressive_planner_orders_tier0_before_tier1_and_tier2_and_verifies_ranges() {
        let mut writer = BundleWriter::new();
        writer
            .add_file("mesh_lod0.10d", "10d", vec![0x11; 512], None)
            .unwrap();
        writer
            .add_file("mesh_lod2.10d", "10d", vec![0x22; 128], None)
            .unwrap();
        writer
            .add_file("semantics.q42", "q42", vec![0x33; 96], None)
            .unwrap();
        let bundle_bytes = writer.build().unwrap();

        // Phase 1: probe only the first 64 bytes
        let probe = HmcHeaderProbe::parse_header_range(&bundle_bytes[..BUNDLE_HEADER_SIZE]).unwrap();
        assert_eq!(probe.entry_count, 3);
        let (idx_off, idx_len) = probe.index_footer_range();

        // Phase 2: parse only the CBOR index footer range
        let footer_slice = &bundle_bytes[idx_off as usize..(idx_off + idx_len) as usize];
        let planner = HmcProgressiveRangePlanner::from_header_and_index(probe, footer_slice).unwrap();
        let schedule = planner.plan_entry_ranges();
        assert_eq!(schedule.len(), 3);

        // Tier 0 (.q42) is scheduled first, then Tier 1 (lod2), then Tier 2 (lod0)
        assert_eq!(schedule[0].entry_key, "semantics.q42");
        assert_eq!(schedule[0].tier, StreamPriorityTier::Tier0SemanticAndProcedural);
        assert_eq!(schedule[1].entry_key, "mesh_lod2.10d");
        assert_eq!(schedule[1].tier, StreamPriorityTier::Tier1MidLodMeshes);
        assert_eq!(schedule[2].entry_key, "mesh_lod0.10d");
        assert_eq!(schedule[2].tier, StreamPriorityTier::Tier2HighPolyAndMip0);

        // Verify arrived slices pass SHA-256 check and tampered slices fail closed
        for req in &schedule {
            let start = req.offset as usize;
            let end = start + req.length as usize;
            verify_arrived_range(req, &bundle_bytes[start..end]).unwrap();
        }
        let mut corrupted = bundle_bytes[schedule[0].offset as usize
            ..(schedule[0].offset + schedule[0].length) as usize]
            .to_vec();
        corrupted[0] ^= 0xFF;
        assert!(verify_arrived_range(&schedule[0], &corrupted).is_err());
    }
}
