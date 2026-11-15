//! Selective chunk-aligned range planning and arrived-chunk verification for `.emf` (v4 `Q42EMF64`)
//! and `.qac` (v1 `qac\0\0\0\0\0`) entries embedded inside `.hmc` bundles.

use std::ops::Range;
use sha2::{Digest, Sha256};
use crate::audio::qac::signal::{self, Limits as QacSignalLimits, SignalView};
use crate::container_10d::crc32c::crc32c;
use crate::q42::emf_profiles::{Limits as EmfProfileLimits, ProfileChunk, ProfileReader};
use crate::q42::field_envelope::{self as env, DIRECTORY_BYTES, Family, HEADER_BYTES, INDEX_DOMAIN, Limits as EnvelopeLimits, StructuralView};
use crate::q42::field_semantics::{Limits as SharedSemanticLimits, Scratch};
use crate::sensing::acquisition::{self as acq, storage::{self as acq_storage, Document as AcquisitionDocument, Limits as AcquisitionLimits}};
use super::format::{BundleEntry, SHA256_LEN};

/// Errors returned during selective field range planning, arrived-chunk verification, or admission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldRangeError {
    PrefixTooShort, InvalidBundleDigest, InvalidFamilyMagic, UnsupportedVersion(u16),
    HeaderCrcMismatch { expected: u32, got: u32 },
    DirectoryEntryCrcMismatch { index: u32, expected: u32, got: u32 },
    IndexCommitmentMismatch, InvalidHeaderFields, InvalidDirectoryEntry { index: u32 },
    InvalidDirectoryOrder { index: u32 }, InvalidCompressionCodec { index: u32, codec: u16 },
    OverlappingExtents { index: u32 }, ExtentMismatch, LimitExceeded, InvalidMetadataTlv,
    EntryIndexOutOfBounds { index: u32, count: u32 },
    TruncatedOrMismatchedRange { expected: u64, got: u64 },
    MetadataCrcMismatch { expected: u32, got: u32 },
    PayloadBlake3Mismatch, CrossContainerSubstitution,
    CrossModeReinterpretation { expected: u16, got: u16 },
    PartialRangeCannotClaimWholeVolume,
    EmfAdmission(crate::q42::emf_profiles::Error),
    QacAdmission(crate::audio::qac::signal::Error),
    AcquisitionAdmission(crate::sensing::acquisition::storage::Error),
}

#[inline] fn u16le(b: &[u8], o: usize) -> u16 { u16::from_le_bytes([b[o], b[o + 1]]) }
#[inline] fn u32le(b: &[u8], o: usize) -> u32 { u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]) }
#[inline] fn u64le(b: &[u8], o: usize) -> u64 { u64::from_le_bytes(b[o..o + 8].try_into().unwrap()) }
#[inline] fn align_up(n: u64, a: u64) -> Result<u64, FieldRangeError> { Ok(n.checked_add(a - 1).ok_or(FieldRangeError::ExtentMismatch)? & !(a - 1)) }
#[inline] fn block_crc32c_124(b: &[u8; 128]) -> u32 { let mut c = *b; c[124..128].fill(0); crc32c(&c) }

fn validate_metadata_tlv(family: Family, mut bytes: &[u8], limits: EnvelopeLimits, req_desc: &[u32]) -> Result<(), FieldRangeError> {
    env::EnvelopeInput { family, flags: 0, metadata: bytes, entries: &[] }
        .encoded_len(limits)
        .map_err(|e| if e == env::Error::Limit { FieldRangeError::LimitExceeded } else { FieldRangeError::InvalidMetadataTlv })?;
    let mut found_schema3 = Vec::new();
    while bytes.len() >= 16 {
        if u16le(bytes, 4) == 3 && u16le(bytes, 6) == 1 { found_schema3.push(u32le(bytes, 0)); }
        bytes = bytes.get(16 + ((u32le(bytes, 8) as usize + 7) & !7)..).ok_or(FieldRangeError::InvalidMetadataTlv)?;
    }
    if req_desc.iter().any(|req| !found_schema3.contains(req)) { return Err(FieldRangeError::InvalidMetadataTlv); }
    Ok(())
}

fn row_to_entry_input<'a>(row: &[u8; 128], payload: &'a [u8]) -> env::EntryInput<'a> {
    env::EntryInput {
        mode: u16le(row, 0), revision: u16le(row, 2), flags: u32le(row, 4),
        stream_id: u32le(row, 8), frame_id: u32le(row, 12), start_tick: u64le(row, 16) as i64,
        time_count: u64le(row, 24), spatial_key: u64le(row, 32), record_count: u64le(row, 64),
        stride: u32le(row, 72), codec: u16le(row, 76), descriptor_id: u32le(row, 112), payload,
    }
}

/// Planned `.hmc` byte ranges and authenticated cryptographic commitments for one chunk entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannedChunkRange {
    pub bundle_key: String, pub bundle_offset: u64, pub bundle_length: u64,
    pub family: Family, pub entry_index: u32, pub total_entry_count: u32,
    pub mode: u16, pub entry_flags: u32, pub codec: u16, pub descriptor_id: u32,
    pub data_offset: u64, pub data_length: u64, pub decoded_bytes: u64,
    pub meta_offset: u64, pub meta_length: u64,
    pub hmc_data_range: Range<u64>, pub hmc_meta_range: Range<u64>,
    pub payload_blake3: [u8; 32], pub meta_crc32c: u32, pub entry_crc32c: u32, pub header_crc32c: u32,
    pub parent_header_digest: [u8; 32], pub index_commitment: [u8; 32],
    pub raw_header_bytes: [u8; HEADER_BYTES], pub raw_directory_entry_bytes: [u8; DIRECTORY_BYTES],
}

/// Selective chunk-aligned range planner for an `.emf` v4 or `.qac` v1 entry inside an `.hmc` bundle.
#[derive(Clone, Debug)]
pub struct FieldEntryRangePlan {
    bundle_key: String, bundle_offset: u64, bundle_length: u64, bundle_sha256: [u8; SHA256_LEN],
    family: Family, entry_count: u32, meta_offset: u64, meta_length: u64,
    meta_crc32c: u32, header_crc32c: u32, parent_header_digest: [u8; 32], index_commitment: [u8; 32],
    raw_header_bytes: [u8; HEADER_BYTES], directory_entries: Vec<[u8; DIRECTORY_BYTES]>,
}

impl FieldEntryRangePlan {
    /// Validate header + directory + metadata prefix bytes for `entry` and build a range plan.
    pub fn from_prefix(entry: &BundleEntry, prefix_bytes: &[u8], limits: EnvelopeLimits) -> Result<Self, FieldRangeError> {
        let bundle_sha256: [u8; SHA256_LEN] = entry.sha256.as_slice().try_into().map_err(|_| FieldRangeError::InvalidBundleDigest)?;
        entry.offset.checked_add(entry.length).ok_or(FieldRangeError::ExtentMismatch)?;
        if prefix_bytes.len() < HEADER_BYTES { return Err(FieldRangeError::PrefixTooShort); }
        let raw_header_bytes: [u8; HEADER_BYTES] = prefix_bytes[..HEADER_BYTES].try_into().unwrap();
        let family = match &raw_header_bytes[..8] {
            m if m == Family::EmfV4Candidate.magic() => Family::EmfV4Candidate,
            m if m == Family::QacV1Candidate.magic() => Family::QacV1Candidate,
            _ => return Err(FieldRangeError::InvalidFamilyMagic),
        };
        let version = u16le(&raw_header_bytes, 8);
        if version != family.version() { return Err(FieldRangeError::UnsupportedVersion(version)); }
        let (header_crc32c, got_hdr_crc) = (u32le(&raw_header_bytes, 124), block_crc32c_124(&raw_header_bytes));
        if got_hdr_crc != header_crc32c { return Err(FieldRangeError::HeaderCrcMismatch { expected: header_crc32c, got: got_hdr_crc }); }
        let (flags, file_bytes, entry_count) = (u32le(&raw_header_bytes, 12), u64le(&raw_header_bytes, 16), u32le(&raw_header_bytes, 32));
        let (meta_offset, meta_length) = (u64le(&raw_header_bytes, 40), u64le(&raw_header_bytes, 48));
        if u16le(&raw_header_bytes, 10) != 128 || flags & !1 != 0 || u64le(&raw_header_bytes, 24) != 128
            || entry_count == 0 || u16le(&raw_header_bytes, 36) != 128 || u16le(&raw_header_bytes, 38) != 1
            || raw_header_bytes[88..124].iter().any(|&b| b != 0)
        {
            return Err(FieldRangeError::InvalidHeaderFields);
        }
        if file_bytes != entry.length { return Err(FieldRangeError::ExtentMismatch); }
        if file_bytes > limits.file_bytes || entry_count > limits.entries || meta_length > limits.metadata_bytes {
            return Err(FieldRangeError::LimitExceeded);
        }
        let dir_len = u64::from(entry_count).checked_mul(128).ok_or(FieldRangeError::ExtentMismatch)?;
        if meta_offset != 128u64.checked_add(dir_len).ok_or(FieldRangeError::ExtentMismatch)? { return Err(FieldRangeError::InvalidHeaderFields); }
        let meta_end = meta_offset.checked_add(meta_length).ok_or(FieldRangeError::ExtentMismatch)?;
        if (prefix_bytes.len() as u64) < meta_end { return Err(FieldRangeError::PrefixTooShort); }
        let (dir_slice, meta_slice) = (&prefix_bytes[128..meta_offset as usize], &prefix_bytes[meta_offset as usize..meta_end as usize]);
        let alignment = if flags & 1 != 0 { 4096u64 } else { 64u64 };
        let mut next = align_up(meta_end, alignment)?;
        if prefix_bytes.len() as u64 >= next && prefix_bytes[meta_end as usize..next as usize].iter().any(|&b| b != 0) {
            return Err(FieldRangeError::ExtentMismatch);
        }
        let (mut directory_entries, mut required_descriptors) = (Vec::with_capacity(entry_count as usize), Vec::with_capacity(entry_count as usize));
        let (mut prior_key, mut agg_decoded) = (None, 0u64);
        for (idx0, row) in dir_slice.chunks_exact(128).enumerate() {
            let (index, row_arr): (u32, [u8; 128]) = ((idx0 + 1) as u32, row.try_into().unwrap());
            let (expected_crc, got_crc) = (u32le(&row_arr, 124), block_crc32c_124(&row_arr));
            if got_crc != expected_crc { return Err(FieldRangeError::DirectoryEntryCrcMismatch { index, expected: expected_crc, got: got_crc }); }
            let codec = u16le(&row_arr, 76);
            if codec != 0 { return Err(FieldRangeError::InvalidCompressionCodec { index, codec }); }
            let (eflags, stream_id, frame_id) = (u32le(&row_arr, 4), u32le(&row_arr, 8), u32le(&row_arr, 12));
            let (start_tick, time_count, spatial_key) = (u64le(&row_arr, 16) as i64, u64le(&row_arr, 24), u64le(&row_arr, 32));
            let (data_off, stored, decoded) = (u64le(&row_arr, 40), u64le(&row_arr, 48), u64le(&row_arr, 56));
            let (records, stride, desc_id) = (u64le(&row_arr, 64), u32le(&row_arr, 72), u32le(&row_arr, 112));
            if row_arr[78..80].iter().any(|&b| b != 0) || row_arr[116..124].iter().any(|&b| b != 0)
                || eflags & !7 != 0 || desc_id == 0 || stride == 0 || records == 0
                || (eflags & 2 == 0 && (start_tick != 0 || time_count != 0))
                || (eflags & 2 != 0 && (time_count == 0 || stream_id == 0))
                || (eflags & 4 == 0 && spatial_key != 0) || (eflags & 4 != 0 && frame_id == 0)
            {
                return Err(FieldRangeError::InvalidDirectoryEntry { index });
            }
            let key = (stream_id, start_tick, spatial_key, u16le(&row_arr, 0), desc_id);
            if prior_key.is_some_and(|p| p >= key) { return Err(FieldRangeError::InvalidDirectoryOrder { index }); }
            prior_key = Some(key);
            let size = records.checked_mul(u64::from(stride)).ok_or(FieldRangeError::ExtentMismatch)?;
            if stored != size || decoded != size { return Err(FieldRangeError::InvalidCompressionCodec { index, codec }); }
            if data_off < next { return Err(FieldRangeError::OverlappingExtents { index }); }
            if data_off != next { return Err(FieldRangeError::ExtentMismatch); }
            agg_decoded = agg_decoded.checked_add(size).ok_or(FieldRangeError::ExtentMismatch)?;
            if size > limits.chunk_bytes || agg_decoded > limits.aggregate_decoded_bytes { return Err(FieldRangeError::LimitExceeded); }
            let end = next.checked_add(size).ok_or(FieldRangeError::ExtentMismatch)?;
            next = align_up(end, alignment)?;
            if prefix_bytes.len() as u64 >= next && prefix_bytes[end as usize..next as usize].iter().any(|&b| b != 0) {
                return Err(FieldRangeError::InvalidCompressionCodec { index, codec });
            }
            required_descriptors.push(desc_id);
            directory_entries.push(row_arr);
        }
        if next != file_bytes { return Err(FieldRangeError::ExtentMismatch); }
        let mut root_header = raw_header_bytes;
        root_header[56..88].fill(0);
        root_header[124..128].fill(0);
        let mut hasher = blake3::Hasher::new();
        hasher.update(INDEX_DOMAIN).update(&root_header).update(dir_slice).update(meta_slice);
        let index_commitment: [u8; 32] = raw_header_bytes[56..88].try_into().unwrap();
        if *hasher.finalize().as_bytes() != index_commitment { return Err(FieldRangeError::IndexCommitmentMismatch); }
        validate_metadata_tlv(family, meta_slice, limits, &required_descriptors)?;
        Ok(Self {
            bundle_key: entry.key.clone(), bundle_offset: entry.offset, bundle_length: entry.length,
            bundle_sha256, family, entry_count, meta_offset, meta_length, meta_crc32c: crc32c(meta_slice),
            header_crc32c, parent_header_digest: *blake3::hash(&raw_header_bytes).as_bytes(),
            index_commitment, raw_header_bytes, directory_entries,
        })
    }

    pub fn family(&self) -> Family { self.family }
    pub fn entry_count(&self) -> u32 { self.entry_count }
    pub fn parent_header_digest(&self) -> [u8; 32] { self.parent_header_digest }
    pub fn index_commitment(&self) -> [u8; 32] { self.index_commitment }
    pub fn hmc_meta_range(&self) -> Range<u64> { (self.bundle_offset + self.meta_offset)..(self.bundle_offset + self.meta_offset + self.meta_length) }

    pub fn plan_chunk_range(&self, entry_index: u32) -> Result<PlannedChunkRange, FieldRangeError> {
        if entry_index == 0 || entry_index > self.entry_count {
            return Err(FieldRangeError::EntryIndexOutOfBounds { index: entry_index, count: self.entry_count });
        }
        let row = self.directory_entries[(entry_index - 1) as usize];
        let (data_offset, data_length) = (u64le(&row, 40), u64le(&row, 48));
        let hmc_data_start = self.bundle_offset.checked_add(data_offset).ok_or(FieldRangeError::ExtentMismatch)?;
        let hmc_data_end = hmc_data_start.checked_add(data_length).ok_or(FieldRangeError::ExtentMismatch)?;
        let hmc_meta_start = self.bundle_offset.checked_add(self.meta_offset).ok_or(FieldRangeError::ExtentMismatch)?;
        let hmc_meta_end = hmc_meta_start.checked_add(self.meta_length).ok_or(FieldRangeError::ExtentMismatch)?;
        Ok(PlannedChunkRange {
            bundle_key: self.bundle_key.clone(), bundle_offset: self.bundle_offset, bundle_length: self.bundle_length,
            family: self.family, entry_index, total_entry_count: self.entry_count, mode: u16le(&row, 0),
            entry_flags: u32le(&row, 4), codec: u16le(&row, 76), descriptor_id: u32le(&row, 112),
            data_offset, data_length, decoded_bytes: u64le(&row, 56), meta_offset: self.meta_offset,
            meta_length: self.meta_length, hmc_data_range: hmc_data_start..hmc_data_end,
            hmc_meta_range: hmc_meta_start..hmc_meta_end, payload_blake3: row[80..112].try_into().unwrap(),
            meta_crc32c: self.meta_crc32c, entry_crc32c: u32le(&row, 124), header_crc32c: self.header_crc32c,
            parent_header_digest: self.parent_header_digest, index_commitment: self.index_commitment,
            raw_header_bytes: self.raw_header_bytes, raw_directory_entry_bytes: row,
        })
    }

    pub fn plan_chunk_ranges(&self, indices: &[u32]) -> Result<Vec<PlannedChunkRange>, FieldRangeError> {
        indices.iter().map(|&i| self.plan_chunk_range(i)).collect()
    }

    pub fn verify_arrived_chunk(&self, plan: &PlannedChunkRange, payload: &[u8], metadata: &[u8]) -> Result<VerifiedArrivedChunk, FieldRangeError> {
        if plan.parent_header_digest != self.parent_header_digest || plan.index_commitment != self.index_commitment || plan.bundle_key != self.bundle_key {
            return Err(FieldRangeError::CrossContainerSubstitution);
        }
        verify_arrived_field_chunk(plan, payload, metadata)
    }

    pub fn claim_whole_volume_receipt(&self, verified_chunks: &[VerifiedArrivedChunk], complete_entry_bytes: Option<&[u8]>) -> Result<[u8; SHA256_LEN], FieldRangeError> {
        if verified_chunks.len() < self.entry_count as usize { return Err(FieldRangeError::PartialRangeCannotClaimWholeVolume); }
        for idx in 1..=self.entry_count {
            let Some(c) = verified_chunks.iter().find(|c| c.plan.entry_index == idx) else { return Err(FieldRangeError::PartialRangeCannotClaimWholeVolume); };
            if c.plan.parent_header_digest != self.parent_header_digest { return Err(FieldRangeError::CrossContainerSubstitution); }
        }
        let Some(full_bytes) = complete_entry_bytes else { return Err(FieldRangeError::PartialRangeCannotClaimWholeVolume); };
        if full_bytes.len() as u64 != self.bundle_length || Sha256::digest(full_bytes).as_slice() != self.bundle_sha256.as_slice() {
            return Err(FieldRangeError::InvalidBundleDigest);
        }
        Ok(self.bundle_sha256)
    }
}

/// A cryptographically verified arrived chunk and metadata TLV bound to its parent container.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedArrivedChunk {
    plan: PlannedChunkRange, payload: Vec<u8>, metadata: Vec<u8>, sparse_container_bytes: Vec<u8>,
}

/// Verify arrived chunk payload + metadata TLV bytes against the authenticated directory entry's
/// `payload_blake3`, `meta_crc32c`, `entry_crc32c`, `mode`, `entry_flags`, and parent container
/// header digest BEFORE any decode or admission.
pub fn verify_arrived_field_chunk(plan: &PlannedChunkRange, arrived_payload: &[u8], arrived_metadata: &[u8]) -> Result<VerifiedArrivedChunk, FieldRangeError> {
    if &plan.raw_header_bytes[..8] != plan.family.magic() || u16le(&plan.raw_header_bytes, 8) != plan.family.version() {
        return Err(FieldRangeError::InvalidFamilyMagic);
    }
    let got_hdr_crc = block_crc32c_124(&plan.raw_header_bytes);
    if got_hdr_crc != plan.header_crc32c || u32le(&plan.raw_header_bytes, 124) != plan.header_crc32c {
        return Err(FieldRangeError::HeaderCrcMismatch { expected: plan.header_crc32c, got: got_hdr_crc });
    }
    if *blake3::hash(&plan.raw_header_bytes).as_bytes() != plan.parent_header_digest || plan.raw_header_bytes[56..88] != plan.index_commitment {
        return Err(FieldRangeError::CrossContainerSubstitution);
    }
    let got_entry_crc = block_crc32c_124(&plan.raw_directory_entry_bytes);
    if got_entry_crc != plan.entry_crc32c || u32le(&plan.raw_directory_entry_bytes, 124) != plan.entry_crc32c {
        return Err(FieldRangeError::DirectoryEntryCrcMismatch { index: plan.entry_index, expected: plan.entry_crc32c, got: got_entry_crc });
    }
    let row = &plan.raw_directory_entry_bytes;
    let row_mode = u16le(row, 0);
    if row_mode != plan.mode { return Err(FieldRangeError::CrossModeReinterpretation { expected: row_mode, got: plan.mode }); }
    let row_codec = u16le(row, 76);
    if row_codec != 0 || plan.codec != 0 {
        return Err(FieldRangeError::InvalidCompressionCodec { index: plan.entry_index, codec: plan.codec.max(row_codec) });
    }
    let expected_size = u64le(row, 64).checked_mul(u64::from(u32le(row, 72))).ok_or(FieldRangeError::ExtentMismatch)?;
    if plan.data_length != expected_size || plan.decoded_bytes != expected_size {
        return Err(FieldRangeError::InvalidCompressionCodec { index: plan.entry_index, codec: 0 });
    }
    if u32le(row, 4) != plan.entry_flags || plan.entry_flags & !7 != 0 || u64le(row, 40) != plan.data_offset
        || u64le(row, 48) != plan.data_length || u64le(row, 56) != plan.decoded_bytes
        || u32le(row, 112) != plan.descriptor_id || plan.descriptor_id == 0 || row[80..112] != plan.payload_blake3
    {
        return Err(FieldRangeError::InvalidDirectoryEntry { index: plan.entry_index });
    }
    if arrived_payload.len() as u64 != plan.data_length {
        return Err(FieldRangeError::TruncatedOrMismatchedRange { expected: plan.data_length, got: arrived_payload.len() as u64 });
    }
    if arrived_metadata.len() as u64 != plan.meta_length {
        return Err(FieldRangeError::TruncatedOrMismatchedRange { expected: plan.meta_length, got: arrived_metadata.len() as u64 });
    }
    let got_meta_crc = crc32c(arrived_metadata);
    if got_meta_crc != plan.meta_crc32c { return Err(FieldRangeError::MetadataCrcMismatch { expected: plan.meta_crc32c, got: got_meta_crc }); }
    validate_metadata_tlv(plan.family, arrived_metadata, EnvelopeLimits::default(), &[plan.descriptor_id])?;
    if *blake3::hash(arrived_payload).as_bytes() != plan.payload_blake3 { return Err(FieldRangeError::PayloadBlake3Mismatch); }
    let entry_input = row_to_entry_input(row, arrived_payload);
    let env_input = env::EnvelopeInput {
        family: plan.family, flags: u32le(&plan.raw_header_bytes, 12),
        metadata: arrived_metadata, entries: std::slice::from_ref(&entry_input),
    };
    let env_len = env_input.encoded_len(EnvelopeLimits::default()).map_err(|_| FieldRangeError::ExtentMismatch)?;
    let mut sparse_container_bytes = vec![0u8; env_len];
    env_input.encode_into(&mut sparse_container_bytes, EnvelopeLimits::default()).map_err(|_| FieldRangeError::ExtentMismatch)?;
    if plan.total_entry_count == 1 && sparse_container_bytes[56..88] != plan.index_commitment {
        return Err(FieldRangeError::IndexCommitmentMismatch);
    }
    Ok(VerifiedArrivedChunk { plan: plan.clone(), payload: arrived_payload.to_vec(), metadata: arrived_metadata.to_vec(), sparse_container_bytes })
}

impl VerifiedArrivedChunk {
    pub fn plan(&self) -> &PlannedChunkRange { &self.plan }
    pub fn payload(&self) -> &[u8] { &self.payload }
    pub fn metadata(&self) -> &[u8] { &self.metadata }
    pub fn is_whole_volume_verified(&self) -> bool { false }
    pub fn claim_whole_volume_receipt(&self) -> Result<[u8; SHA256_LEN], FieldRangeError> { Err(FieldRangeError::PartialRangeCannotClaimWholeVolume) }
    pub fn require_mode(&self, expected_mode: u16) -> Result<(), FieldRangeError> {
        if self.plan.mode != expected_mode { return Err(FieldRangeError::CrossModeReinterpretation { expected: expected_mode, got: self.plan.mode }); }
        Ok(())
    }

    pub fn assemble_envelope_with_companions(&self, companions: &[&VerifiedArrivedChunk]) -> Result<Vec<u8>, FieldRangeError> {
        if companions.is_empty() && self.plan.entry_index == 1 { return Ok(self.sparse_container_bytes.clone()); }
        let mut all = vec![self];
        for &c in companions {
            if c.plan.parent_header_digest != self.plan.parent_header_digest || c.plan.index_commitment != self.plan.index_commitment || c.plan.family != self.plan.family {
                return Err(FieldRangeError::CrossContainerSubstitution);
            }
            all.push(c);
        }
        all.sort_by_key(|c| c.plan.entry_index);
        let inputs: Vec<env::EntryInput<'_>> = all.iter().map(|c| row_to_entry_input(&c.plan.raw_directory_entry_bytes, &c.payload)).collect();
        let env_input = env::EnvelopeInput { family: self.plan.family, flags: u32le(&self.plan.raw_header_bytes, 12), metadata: &self.metadata, entries: &inputs };
        let len = env_input.encoded_len(EnvelopeLimits::default()).map_err(|_| FieldRangeError::ExtentMismatch)?;
        let mut out = vec![0u8; len];
        env_input.encode_into(&mut out, EnvelopeLimits::default()).map_err(|_| FieldRangeError::ExtentMismatch)?;
        Ok(out)
    }

    pub fn inspect_emf_profile<'a>(&'a self, scratch: &mut Scratch<'_>, limits: EmfProfileLimits, total_payload_work: u64) -> Result<ProfileChunk<'a>, FieldRangeError> {
        if self.plan.family != Family::EmfV4Candidate { return Err(FieldRangeError::CrossModeReinterpretation { expected: self.plan.mode, got: self.plan.mode }); }
        let view = StructuralView::open(&self.sparse_container_bytes, Family::EmfV4Candidate, EnvelopeLimits::default())
            .map_err(|e| FieldRangeError::EmfAdmission(crate::q42::emf_profiles::Error::Storage(e)))?;
        ProfileReader::open(&view, scratch, limits, total_payload_work).map_err(FieldRangeError::EmfAdmission)?.chunk(1).map_err(FieldRangeError::EmfAdmission)
    }

    pub fn inspect_qac_signal<'a>(&'a self, scratch: &mut Scratch<'_>, shared_limits: SharedSemanticLimits, limits: QacSignalLimits) -> Result<SignalView<'a>, FieldRangeError> {
        if self.plan.family != Family::QacV1Candidate { return Err(FieldRangeError::CrossModeReinterpretation { expected: 0x0100, got: self.plan.mode }); }
        let view = StructuralView::open(&self.sparse_container_bytes, Family::QacV1Candidate, EnvelopeLimits::default())
            .map_err(|e| FieldRangeError::QacAdmission(signal::Error::Envelope(e)))?;
        signal::admit(&view, 1, scratch, shared_limits, limits).map_err(FieldRangeError::QacAdmission)
    }

    pub fn inspect_acquisition_in_envelope<'a, 's>(
        &self, assembled_envelope: &'a [u8], originals: &'s mut [Option<acq::Original<'a>>],
        scratch: &mut Scratch<'_>, shared_limits: SharedSemanticLimits, limits: AcquisitionLimits,
    ) -> Result<AcquisitionDocument<'a, 's>, FieldRangeError> {
        self.require_mode(acq_storage::ORIGINAL_MODE)?;
        let view = StructuralView::open(assembled_envelope, self.plan.family, EnvelopeLimits::default())
            .map_err(|e| FieldRangeError::AcquisitionAdmission(acq_storage::Error::Envelope(e)))?;
        acq_storage::admit(&view, self.plan.entry_index, originals, scratch, shared_limits, limits).map_err(FieldRangeError::AcquisitionAdmission)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::qac::signal::{
        writer::{
            PcmDocument, PcmQuantity, PcmSamples, Provenance, ProvenanceRole, SignalRelation,
            WriteBuffers, WriteOptions,
        },
        ExactTime, Rate, Timing,
    };
    use crate::bundle::BundleWriter;
    use crate::q42::emf_profiles::{encode_field_samples, field_entry_slot, field_metadata_len, prepare_field_document, FieldChunkInput, FieldClock, FieldDocument, FieldFrame, FieldProvenance, FieldSample, FieldWriteLimits};
    use crate::q42::field_semantics::NodeState;
    use crate::sensing::SemanticId;

    fn build_valid_emf_bytes(carrier_hz: f64) -> Vec<u8> {
        let samples = [FieldSample { electric_v_per_m: [1.5, -0.5, 0.25], magnetic_tesla: [0.0, 5.0e-9, 0.0], position_m: [1.0, 2.0, 3.0], tick: 10, carrier_hz: Some(carrier_hz), stored_poynting_w_per_m2: None, confidence: Some(0.95) }];
        let mut payload_buf = [0u8; 128];
        let payload = encode_field_samples(&samples, &mut payload_buf, EmfProfileLimits::default()).unwrap();
        let chunks = [FieldChunkInput { stream_id: 1, start_tick: 10, time_count: 1, spatial_key: None, payload }];
        let doc = FieldDocument {
            clock: FieldClock { scale: 1, epoch_seconds: 1_700_000_000, epoch_nanoseconds: 0, ticks_per_second_num: 1000, ticks_per_second_den: 1 },
            frame: FieldFrame { origin_m: [0.0; 3], rotation_xyzw: [0.0, 0.0, 0.0, 1.0], grid: None },
            provenance: FieldProvenance { role: 0, algorithm: Some("maxwell_probe"), algorithm_version: Some("1.0"), lineage: None },
            temporal_step_ticks: Some(1), chunks: &chunks, align_payloads_to_4096: false,
        };
        let limits = FieldWriteLimits::default();
        let (mut meta, mut slots) = (vec![0u8; field_metadata_len(&doc, limits).unwrap()], [field_entry_slot(); 1]);
        let prep = prepare_field_document(&doc, &mut meta, &mut slots, limits).unwrap();
        let len = prep.encoded_len().unwrap();
        let (mut staging, mut out, mut m, mut e) = (vec![0u8; len], vec![0u8; len], [NodeState::default(); 64], [NodeState::default(); 16]);
        prep.encode_into(&mut staging, &mut out, &mut Scratch { metadata: &mut m, entries: &mut e }).unwrap();
        out
    }

    fn build_valid_qac_bytes(sample_val: f32) -> Vec<u8> {
        let samples = [sample_val, -sample_val, 0.25, -0.25];
        let rate = Rate::new(48_000, 1).unwrap();
        let doc = PcmDocument {
            samples: PcmSamples::F32(&samples), timing: Timing { scale: 1, epoch_seconds: 0, epoch_nanoseconds: 0, ticks: rate, samples: rate, start_tick: 0 },
            stream_id: 1, channels: 1, channel_order: "mono", purpose: "test-pcm", quantity: PcmQuantity::Pressure, relation: SignalRelation::PhysicalBaseUnits,
            units_per_stored_sample: 1.0, stored_offset: 0.0,
            provenance: Provenance { role: ProvenanceRole::Measured, algorithm: None, algorithm_version: None, instrument_uri: Some("urn:qualia:mic:1"), lineage: None }, original: None,
        };
        let (mut meta, mut payload, mut staging, mut out) = (vec![0u8; 4096], vec![0u8; 4096], vec![0u8; 8192], vec![0u8; 8192]);
        let (mut m, mut e) = ([NodeState::default(); 64], [NodeState::default(); 16]);
        let n = doc.encode_into(WriteBuffers { metadata: &mut meta, payload: &mut payload, file: &mut staging }, &mut Scratch { metadata: &mut m, entries: &mut e }, &mut out, WriteOptions::default()).unwrap();
        out.truncate(n);
        out
    }

    fn build_acquisition_bytes(family: Family) -> Vec<u8> {
        let (out_bytes, src_bytes) = ([0x10u8, 0x20, 0, 0, 0, 0, 0, 0, 0, 0x40], [1u8, 2, 3, 4, 5, 6, 7, 8]);
        let (original, src_orig) = (acq::Original::admit(&out_bytes, 10).unwrap(), acq::Original::admit(&src_bytes, 8).unwrap());
        let layout = acq::Layout { rank: 1, extents: [1, 0, 0, 0], strides: [8, 0, 0, 0], axes: [acq::Axis::Sample; 4], encoding: acq::Encoding::Shared(acq::Scalar::F64), byte_order: acq::ByteOrder::Little, components: acq::ComponentOrder::Scalar };
        let ctx = acq::Context { quantity: acq::Quantity { kind: SemanticId::new(11).unwrap(), unit: SemanticId::new(12).unwrap() }, stage: acq::SampleStage::ReportedPhysical, sensor: SemanticId::new(13).unwrap(), geometry: None, calibration: None, uncertainty: None };
        let axes = acq::SignalAxes { carrier: acq::Carrier::Unknown, baseband_hz: None, pulse_rate: None, complex_convention: None, phase_origin: None };
        let tv = acq::TensorView::admit(original.region(2, 8).unwrap(), layout, ctx, None, axes).unwrap();
        let (inputs, losses) = ([src_orig], [acq::Loss::Quantization]);
        let link = acq::ArtifactLink { digest: [0x55; 32], identifier: SemanticId::new(99).unwrap() };
        let deriv = Some(acq::Derivation::admit(original, &inputs, link, link, &losses, 8).unwrap());
        let (mut meta, mut staging, mut out, mut m, mut e) = (vec![0u8; 16_384], vec![0u8; 65_536], vec![0u8; 65_536], [NodeState::default(); 256], [NodeState::default(); 128]);
        let n = acq_storage::encode(acq_storage::Input { family, view: acq_storage::View::Tensor(tv), derivation: deriv }, &mut meta, &mut staging, &mut Scratch { metadata: &mut m, entries: &mut e }, &mut out, Default::default(), Default::default(), Default::default()).unwrap();
        out.truncate(n);
        out
    }

    #[test]
    fn selective_hmc_range_planning_and_typed_emf_qac_verification() {
        let (emf_bytes, qac_bytes) = (build_valid_emf_bytes(2.4e9), build_valid_qac_bytes(0.5));
        let mut w = BundleWriter::new();
        w.add_file("field.emf", "emf", emf_bytes.clone(), None).unwrap();
        w.add_file("voice.qac", "qac", qac_bytes.clone(), None).unwrap();
        let hmc = w.build().unwrap();
        let reader = crate::bundle::BundleReader::parse(&hmc).unwrap();
        let emf_plan = FieldEntryRangePlan::from_prefix(reader.entry("field.emf").unwrap(), &emf_bytes, EnvelopeLimits::default()).unwrap();
        let c1 = emf_plan.plan_chunk_range(1).unwrap();
        let verified_emf = emf_plan.verify_arrived_chunk(&c1, &hmc[c1.hmc_data_range.start as usize..c1.hmc_data_range.end as usize], &hmc[c1.hmc_meta_range.start as usize..c1.hmc_meta_range.end as usize]).unwrap();
        assert!(!verified_emf.is_whole_volume_verified());
        assert_eq!(verified_emf.claim_whole_volume_receipt(), Err(FieldRangeError::PartialRangeCannotClaimWholeVolume));
        let (mut m, mut e) = ([NodeState::default(); 64], [NodeState::default(); 16]);
        let chunk = verified_emf.inspect_emf_profile(&mut Scratch { metadata: &mut m, entries: &mut e }, EmfProfileLimits::default(), 1 << 20).unwrap();
        assert_eq!(chunk.field(0).unwrap().unwrap().carrier_hz(), Some(2.4e9));
        let qac_plan = FieldEntryRangePlan::from_prefix(reader.entry("voice.qac").unwrap(), &qac_bytes, EnvelopeLimits::default()).unwrap();
        let qc1 = qac_plan.plan_chunk_range(1).unwrap();
        let verified_qac = verify_arrived_field_chunk(&qc1, &hmc[qc1.hmc_data_range.start as usize..qc1.hmc_data_range.end as usize], &hmc[qc1.hmc_meta_range.start as usize..qc1.hmc_meta_range.end as usize]).unwrap();
        let sig = verified_qac.inspect_qac_signal(&mut Scratch { metadata: &mut m, entries: &mut e }, SharedSemanticLimits::default(), QacSignalLimits::default()).unwrap();
        assert_eq!(sig.records(), 4);
        assert_eq!(sig.time_at(0).unwrap(), ExactTime { numerator: 0, denominator: 48_000 });
    }

    #[test]
    fn mixed_domain_mode_0x0104_schema_6_and_7_inspection_on_arrived_chunks() {
        for family in [Family::EmfV4Candidate, Family::QacV1Candidate] {
            let acq_bytes = build_acquisition_bytes(family);
            let mut w = BundleWriter::new();
            w.add_file("sensor.bin", "acq", acq_bytes.clone(), None).unwrap();
            let hmc = w.build().unwrap();
            let reader = crate::bundle::BundleReader::parse(&hmc).unwrap();
            let plan = FieldEntryRangePlan::from_prefix(reader.entry("sensor.bin").unwrap(), &acq_bytes, EnvelopeLimits::default()).unwrap();
            let ranges = plan.plan_chunk_ranges(&[1, 2]).unwrap();
            let meta = &hmc[ranges[0].hmc_meta_range.start as usize..ranges[0].hmc_meta_range.end as usize];
            let v1 = plan.verify_arrived_chunk(&ranges[0], &hmc[ranges[0].hmc_data_range.start as usize..ranges[0].hmc_data_range.end as usize], meta).unwrap();
            let v2 = plan.verify_arrived_chunk(&ranges[1], &hmc[ranges[1].hmc_data_range.start as usize..ranges[1].hmc_data_range.end as usize], meta).unwrap();
            assert_eq!(plan.claim_whole_volume_receipt(std::slice::from_ref(&v1), Some(&acq_bytes)), Err(FieldRangeError::PartialRangeCannotClaimWholeVolume));
            let assembled = v1.assemble_envelope_with_companions(&[&v2]).unwrap();
            let (mut originals, mut m, mut e) = ([None; 8], [NodeState::default(); 256], [NodeState::default(); 128]);
            let doc = v1.inspect_acquisition_in_envelope(&assembled, &mut originals, &mut Scratch { metadata: &mut m, entries: &mut e }, SharedSemanticLimits::default(), AcquisitionLimits::default()).unwrap();
            let acq_storage::View::Tensor(tv) = doc.view else { panic!("expected tensor view") };
            assert_eq!(tv.numeric([0, 0, 0, 0]).unwrap(), [2.0, 0.0]);
            let lineage = doc.lineage.expect("Schema 7 derivation lineage present");
            assert_eq!((lineage.output_entry(), lineage.input_entry(0).unwrap()), (1, 2));
        }
    }

    #[test]
    fn anti_substitution_tampering_and_codec_invariants_reject() {
        let (emf_a, emf_b, qac_a) = (build_valid_emf_bytes(1.0e9), build_valid_emf_bytes(5.0e9), build_valid_qac_bytes(0.25));
        let mut w = BundleWriter::new();
        w.add_file("a.emf", "emf", emf_a.clone(), None).unwrap();
        w.add_file("b.emf", "emf", emf_b.clone(), None).unwrap();
        w.add_file("a.qac", "qac", qac_a.clone(), None).unwrap();
        let hmc = w.build().unwrap();
        let reader = crate::bundle::BundleReader::parse(&hmc).unwrap();
        let plan_a = FieldEntryRangePlan::from_prefix(reader.entry("a.emf").unwrap(), &emf_a, Default::default()).unwrap();
        let plan_b = FieldEntryRangePlan::from_prefix(reader.entry("b.emf").unwrap(), &emf_b, Default::default()).unwrap();
        let plan_q = FieldEntryRangePlan::from_prefix(reader.entry("a.qac").unwrap(), &qac_a, Default::default()).unwrap();
        let (ca, cb, cq) = (plan_a.plan_chunk_range(1).unwrap(), plan_b.plan_chunk_range(1).unwrap(), plan_q.plan_chunk_range(1).unwrap());
        let (pay_a, meta_a, pay_b) = (&hmc[ca.hmc_data_range.start as usize..ca.hmc_data_range.end as usize], &hmc[ca.hmc_meta_range.start as usize..ca.hmc_meta_range.end as usize], &hmc[cb.hmc_data_range.start as usize..cb.hmc_data_range.end as usize]);
        assert_eq!(plan_a.verify_arrived_chunk(&cb, pay_b, meta_a), Err(FieldRangeError::CrossContainerSubstitution));
        assert_eq!(verify_arrived_field_chunk(&ca, pay_b, meta_a), Err(FieldRangeError::PayloadBlake3Mismatch));
        let mut forged_mode = ca.clone();
        forged_mode.mode = 0x0104;
        assert_eq!(verify_arrived_field_chunk(&forged_mode, pay_a, meta_a), Err(FieldRangeError::CrossModeReinterpretation { expected: 0, got: 0x0104 }));
        let va = verify_arrived_field_chunk(&ca, pay_a, meta_a).unwrap();
        let vq = verify_arrived_field_chunk(&cq, &hmc[cq.hmc_data_range.start as usize..cq.hmc_data_range.end as usize], &hmc[cq.hmc_meta_range.start as usize..cq.hmc_meta_range.end as usize]).unwrap();
        assert_eq!(va.assemble_envelope_with_companions(&[&vq]), Err(FieldRangeError::CrossContainerSubstitution));
        let (mut bad_pay, mut bad_meta) = (pay_a.to_vec(), meta_a.to_vec());
        bad_pay[0] ^= 1;
        bad_meta[20] ^= 1;
        assert_eq!(verify_arrived_field_chunk(&ca, &bad_pay, meta_a), Err(FieldRangeError::PayloadBlake3Mismatch));
        assert!(matches!(verify_arrived_field_chunk(&ca, pay_a, &bad_meta), Err(FieldRangeError::MetadataCrcMismatch { .. })));
        assert!(matches!(verify_arrived_field_chunk(&ca, &pay_a[..pay_a.len() - 1], meta_a), Err(FieldRangeError::TruncatedOrMismatchedRange { .. })));
        let mut bad_codec = ca.clone();
        bad_codec.decoded_bytes += 64;
        assert_eq!(verify_arrived_field_chunk(&bad_codec, pay_a, meta_a), Err(FieldRangeError::InvalidCompressionCodec { index: 1, codec: 0 }));
    }
}
