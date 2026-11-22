//! Sparse SDFB fields. Samples are little-endian i16 or f16 in 8³ bricks.
//!
//! V2 stores X-fastest distance samples and solid membership. Opt-in v3
//! appends the OpenVDB leaf's independent Z-fastest activity mask. Both
//! versions share the 64-byte header and the first 1,152 bytes of each brick.

use super::{crc32c::crc32c, manifold_identity_v2::SIGNED_DISTANCE_BRICK_TAG};

pub const SDFB_HEADER_BYTES: usize = 64;
pub const SDFB_SAMPLES: usize = 8 * 8 * 8;
pub const SDFB_OCCUPANCY_BYTES: usize = SDFB_SAMPLES / 8;
pub const SDFB_RECORD_BYTES: usize = 64 + 2 * SDFB_SAMPLES + SDFB_OCCUPANCY_BYTES;
pub const SDFB_OPENVDB_ACTIVE_MASK_BYTES: usize = SDFB_SAMPLES / 8;
pub const SDFB_OPENVDB_RECORD_BYTES: usize = SDFB_RECORD_BYTES + SDFB_OPENVDB_ACTIVE_MASK_BYTES;
const VERSION_V2: u16 = 2;
const VERSION_OPENVDB: u16 = 3;
const MAX_BYTES: usize = 128 * 1024 * 1024;
const MAX_REFINEMENT: u8 = 8;

pub type OpenVdbActiveMask = [u8; SDFB_OPENVDB_ACTIVE_MASK_BYTES];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SdfSampleEncoding {
    ScaledI16,
    F16,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SdfBrickGrid {
    pub origin_m: [f32; 3],
    pub voxel_spacing_m: [f32; 3],
    pub base_bricks: [u32; 3],
    pub encoding: SdfSampleEncoding,
    pub meters_per_i16: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SdfBrickKey {
    pub refinement_level: u8,
    pub coordinate: [u32; 3],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdfBrickRecord {
    pub key: SdfBrickKey,
    pub wall_semantic_id: u64,
    pub lumen_semantic_id: u64,
    /// X-fastest sample bit patterns.
    pub samples: [u16; SDFB_SAMPLES],
    /// X-fastest solid membership, one bit per sample.
    pub occupancy: [u8; SDFB_OCCUPANCY_BYTES],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SdfBrickError {
    InvalidHeader,
    InvalidGrid,
    Length,
    Empty,
    Budget,
    Checksum,
    InvalidKey,
    Order,
    InvalidSemanticId,
    NonFiniteSample,
}

impl std::fmt::Display for SdfBrickError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SDFB: {self:?}")
    }
}

impl std::error::Error for SdfBrickError {}

pub fn sdfb_encoded_len(count: usize) -> Result<usize, SdfBrickError> {
    encoded_len(count, SDFB_RECORD_BYTES)
}

/// Size of the opt-in payload with one OpenVDB activity mask per brick.
pub fn sdfb_openvdb_encoded_len(count: usize) -> Result<usize, SdfBrickError> {
    encoded_len(count, SDFB_OPENVDB_RECORD_BYTES)
}

fn encoded_len(count: usize, stride: usize) -> Result<usize, SdfBrickError> {
    if count == 0 {
        return Err(SdfBrickError::Empty);
    }
    count
        .checked_mul(stride)
        .and_then(|bytes| bytes.checked_add(SDFB_HEADER_BYTES))
        .filter(|&bytes| bytes <= MAX_BYTES)
        .ok_or(SdfBrickError::Budget)
}

impl SdfBrickGrid {
    fn validate(self) -> Result<(), SdfBrickError> {
        if !self.origin_m.iter().all(|v| v.is_finite())
            || !self.voxel_spacing_m.iter().all(|v| v.is_finite() && *v > 0.0)
            || self.base_bricks.contains(&0)
            || !self.meters_per_i16.is_finite()
            || !(0.0..=f32::MAX / 32768.0).contains(&self.meters_per_i16)
            || (self.encoding == SdfSampleEncoding::ScaledI16 && self.meters_per_i16 == 0.0)
        {
            return Err(SdfBrickError::InvalidGrid);
        }
        Ok(())
    }

    fn validate_record(
        self,
        key: SdfBrickKey,
        wall: u64,
        lumen: u64,
        samples: impl Iterator<Item = u16>,
    ) -> Result<(), SdfBrickError> {
        if key.refinement_level > MAX_REFINEMENT
            || key.coordinate.into_iter().zip(self.base_bricks).any(|(index, base)| {
                u64::from(index) >= (u64::from(base) << key.refinement_level)
            })
        {
            return Err(SdfBrickError::InvalidKey);
        }
        if wall == 0 || lumen == 0 || wall == lumen {
            return Err(SdfBrickError::InvalidSemanticId);
        }
        if self.encoding == SdfSampleEncoding::F16
            && samples.into_iter().any(|bits| !half::f16::from_bits(bits).is_finite())
        {
            return Err(SdfBrickError::NonFiniteSample);
        }
        Ok(())
    }
}

fn le_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

fn le_u64(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap())
}

pub fn encode_sdfb_into(
    grid: SdfBrickGrid,
    records: &[SdfBrickRecord],
    output: &mut [u8],
) -> Result<usize, SdfBrickError> {
    encode_sdfb_payload(grid, records, None, output)
}

/// Encode opt-in SDFB v3. Every OpenVDB mask belongs to the record at the same
/// index and is copied verbatim. Bit `z + 8*y + 64*x` is LSB-first in its byte
/// after interpreting the source leaf's 64-bit mask words as little-endian.
pub fn encode_sdfb_with_openvdb_active_masks_into(
    grid: SdfBrickGrid,
    records: &[SdfBrickRecord],
    active_masks: &[OpenVdbActiveMask],
    output: &mut [u8],
) -> Result<usize, SdfBrickError> {
    if records.len() != active_masks.len() {
        return Err(SdfBrickError::Length);
    }
    encode_sdfb_payload(grid, records, Some(active_masks), output)
}

fn encode_sdfb_payload(
    grid: SdfBrickGrid,
    records: &[SdfBrickRecord],
    active_masks: Option<&[OpenVdbActiveMask]>,
    output: &mut [u8],
) -> Result<usize, SdfBrickError> {
    grid.validate()?;
    let (version, stride) = if active_masks.is_some() {
        (VERSION_OPENVDB, SDFB_OPENVDB_RECORD_BYTES)
    } else {
        (VERSION_V2, SDFB_RECORD_BYTES)
    };
    let len = encoded_len(records.len(), stride)?;
    let output = output.get_mut(..len).ok_or(SdfBrickError::Length)?;
    let mut previous = None;
    for record in records {
        grid.validate_record(
            record.key, record.wall_semantic_id, record.lumen_semantic_id,
            record.samples.iter().copied(),
        )?;
        if previous.is_some_and(|key| key >= record.key) {
            return Err(SdfBrickError::Order);
        }
        previous = Some(record.key);
    }

    output.fill(0);
    output[..4].copy_from_slice(&SIGNED_DISTANCE_BRICK_TAG);
    output[4..6].copy_from_slice(&version.to_le_bytes());
    output[6] = match grid.encoding {
        SdfSampleEncoding::ScaledI16 => 1,
        SdfSampleEncoding::F16 => 2,
    };
    output[8..12].copy_from_slice(&(records.len() as u32).to_le_bytes());
    for axis in 0..3 {
        let offset = axis * 4;
        output[12 + offset..16 + offset].copy_from_slice(&grid.origin_m[axis].to_le_bytes());
        output[24 + offset..28 + offset].copy_from_slice(&grid.voxel_spacing_m[axis].to_le_bytes());
        output[40 + offset..44 + offset].copy_from_slice(&grid.base_bricks[axis].to_le_bytes());
    }
    output[36..40].copy_from_slice(&grid.meters_per_i16.to_le_bytes());
    for (index, (record, bytes)) in records
        .iter()
        .zip(output[SDFB_HEADER_BYTES..].chunks_exact_mut(stride))
        .enumerate()
    {
        for (chunk, value) in bytes[..12].chunks_exact_mut(4).zip(record.key.coordinate) {
            chunk.copy_from_slice(&value.to_le_bytes());
        }
        bytes[12] = record.key.refinement_level;
        bytes[16..24].copy_from_slice(&record.wall_semantic_id.to_le_bytes());
        bytes[24..32].copy_from_slice(&record.lumen_semantic_id.to_le_bytes());
        let (samples, occupancy) = bytes[64..SDFB_RECORD_BYTES].split_at_mut(2 * SDFB_SAMPLES);
        for (chunk, value) in samples.chunks_exact_mut(2).zip(record.samples.iter()) {
            chunk.copy_from_slice(&value.to_le_bytes());
        }
        occupancy.copy_from_slice(&record.occupancy);
        if let Some(active_masks) = active_masks {
            bytes[SDFB_RECORD_BYTES..].copy_from_slice(&active_masks[index]);
        }
    }
    let crc = crc32c(&output[SDFB_HEADER_BYTES..]);
    output[52..56].copy_from_slice(&crc.to_le_bytes());
    Ok(len)
}

#[derive(Clone, Copy)]
pub struct SdfBrickView<'a> {
    grid: SdfBrickGrid,
    records: &'a [u8],
    stride: usize,
}

impl<'a> SdfBrickView<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, SdfBrickError> {
        let header = bytes.get(..SDFB_HEADER_BYTES).ok_or(SdfBrickError::Length)?;
        if header[..4] != SIGNED_DISTANCE_BRICK_TAG
            || header[7] != 0
            || header[56..].iter().any(|&byte| byte != 0)
        {
            return Err(SdfBrickError::InvalidHeader);
        }
        let stride = match u16::from_le_bytes([header[4], header[5]]) {
            VERSION_V2 => SDFB_RECORD_BYTES,
            VERSION_OPENVDB => SDFB_OPENVDB_RECORD_BYTES,
            _ => return Err(SdfBrickError::InvalidHeader),
        };
        let encoding = match header[6] {
            1 => SdfSampleEncoding::ScaledI16,
            2 => SdfSampleEncoding::F16,
            _ => return Err(SdfBrickError::InvalidHeader),
        };
        let count = le_u32(header, 8) as usize;
        if bytes.len() != encoded_len(count, stride)? {
            return Err(SdfBrickError::Length);
        }
        let grid = SdfBrickGrid {
            origin_m: std::array::from_fn(|axis| f32::from_bits(le_u32(header, 12 + axis * 4))),
            voxel_spacing_m: std::array::from_fn(|axis| f32::from_bits(le_u32(header, 24 + axis * 4))),
            base_bricks: std::array::from_fn(|axis| le_u32(header, 40 + axis * 4)),
            encoding,
            meters_per_i16: f32::from_bits(le_u32(header, 36)),
        };
        grid.validate()?;
        let records = &bytes[SDFB_HEADER_BYTES..];
        if crc32c(records) != le_u32(header, 52) {
            return Err(SdfBrickError::Checksum);
        }
        let view = Self { grid, records, stride };
        let mut previous = None;
        for record in view.bricks() {
            if record.bytes[13..16].iter().chain(&record.bytes[32..64]).any(|&byte| byte != 0) {
                return Err(SdfBrickError::InvalidHeader);
            }
            let key = record.key();
            grid.validate_record(key, record.wall_semantic_id(), record.lumen_semantic_id(), record.sample_bits())?;
            if previous.is_some_and(|prev| prev >= key) {
                return Err(SdfBrickError::Order);
            }
            previous = Some(key);
        }
        Ok(view)
    }

    pub fn grid(self) -> SdfBrickGrid {
        self.grid
    }

    pub fn has_openvdb_active_masks(self) -> bool {
        self.stride == SDFB_OPENVDB_RECORD_BYTES
    }

    pub fn bricks(self) -> impl ExactSizeIterator<Item = SdfBrickRef<'a>> {
        self.records.chunks_exact(self.stride).map(move |bytes| SdfBrickRef { bytes, grid: self.grid })
    }

    /// Resolve a sparse brick by its grid key without allocating or decoding neighboring pages.
    pub fn brick(self, key: SdfBrickKey) -> Option<SdfBrickRef<'a>> {
        let mut lower = 0;
        let mut upper = self.records.len() / self.stride;
        while lower < upper {
            let middle = lower + (upper - lower) / 2;
            let start = middle * self.stride;
            let brick = SdfBrickRef {
                bytes: &self.records[start..start + self.stride],
                grid: self.grid,
            };
            match brick.key().cmp(&key) {
                std::cmp::Ordering::Less => lower = middle + 1,
                std::cmp::Ordering::Equal => return Some(brick),
                std::cmp::Ordering::Greater => upper = middle,
            }
        }
        None
    }
}

/// Resolve the unique SDFB sidecar through MID2, with codec and checksum validation.
/// The supplied container must already have passed its outer integrity checks.
pub fn read_container_sdfb(bytes: &[u8]) -> Result<Option<SdfBrickView<'_>>, String> {
    read_container_sdfb_with_iris(bytes).map(|resolved| resolved.map(|(view, _)| view))
}

/// Resolve the SDFB payload and each verified wall/lumen lexical identity from
/// its enclosing CBLD. Consumers may select an IRI in either direction by
/// computing its canonical q_hash and requiring exact equality with this map.
pub fn read_container_sdfb_with_iris(
    bytes: &[u8],
) -> Result<Option<(SdfBrickView<'_>, std::collections::BTreeMap<u64, String>)>, String> {
    use super::{
        decode_symbolic_rdf_cbor_ld, TypedFieldRecord, TypedPayloadResolver,
        FIELD_KIND_SYMBOLIC_RDF_CBOR_LD, MANIFOLD_IDENTITY_V2_MAX_FIELDS,
    };
    use super::manifold_identity_v2::FIELD_KIND_SIGNED_DISTANCE_BRICK;
    use crate::{bundle::semantic_model::valid_iri, q_hash};
    use std::collections::BTreeMap;

    let mut read = [TypedFieldRecord::default(); MANIFOLD_IDENTITY_V2_MAX_FIELDS];
    let mut fields = [TypedFieldRecord::default(); MANIFOLD_IDENTITY_V2_MAX_FIELDS];
    let resolved = TypedPayloadResolver::new(bytes, &[], &mut read, &mut fields)
        .map_err(|error| error.to_string())?;
    let mut payloads = resolved.payloads_of_kind(FIELD_KIND_SIGNED_DISTANCE_BRICK);
    let Some(first) = payloads.next() else {
        if resolved.section_payload(super::SectionType::SignedDistanceBrick).is_some() {
            return Err("10d: SDFB section is missing its MID2 record".into());
        }
        return Ok(None);
    };
    if payloads.next().is_some() {
        return Err("10d: repeated SDFB fields".into());
    }
    let first = first.map_err(|error| error.to_string())?;
    let view = SdfBrickView::parse(first.bytes).map_err(|error| error.to_string())?;
    let mut required = BTreeMap::new();
    for brick in view.bricks() {
        required.insert(brick.wall_semantic_id(), None);
        required.insert(brick.lumen_semantic_id(), None);
    }
    for payload in resolved.payloads_of_kind(FIELD_KIND_SYMBOLIC_RDF_CBOR_LD) {
        let payload = payload.map_err(|error| error.to_string())?;
        let cbld = decode_symbolic_rdf_cbor_ld(payload.bytes).map_err(|error| error.to_string())?;
        for term in cbld.lexicon {
            let Some(iri) = required.get_mut(&term.term_hash) else {
                continue;
            };
            if !valid_iri(&term.iri_utf8) || q_hash(&term.iri_utf8) != term.term_hash {
                return Err("SDFB: invalid local semantic IRI".into());
            }
            if let Some(previous) = iri {
                if previous != &term.iri_utf8 {
                    return Err("SDFB: conflicting local semantic IRIs".into());
                }
            } else {
                *iri = Some(term.iri_utf8);
            }
        }
    }
    let lexicon = required.into_iter()
        .map(|(hash, iri)| iri.map(|iri| (hash, iri)).ok_or("SDFB: unresolved wall or lumen hash in local CBLD"))
        .collect::<Result<_, _>>()?;
    Ok(Some((view, lexicon)))
}

pub struct SdfBrickRef<'a> {
    bytes: &'a [u8],
    grid: SdfBrickGrid,
}

impl<'a> SdfBrickRef<'a> {
    pub fn key(&self) -> SdfBrickKey {
        SdfBrickKey {
            refinement_level: self.bytes[12],
            coordinate: std::array::from_fn(|axis| le_u32(self.bytes, axis * 4)),
        }
    }

    pub fn wall_semantic_id(&self) -> u64 {
        le_u64(self.bytes, 16)
    }

    pub fn lumen_semantic_id(&self) -> u64 {
        le_u64(self.bytes, 24)
    }

    fn sample_bits(&self) -> impl Iterator<Item = u16> + '_ {
        self.bytes[64..64 + 2 * SDFB_SAMPLES].chunks_exact(2)
            .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    pub fn distance_m(&self, sample: usize) -> Option<f32> {
        if sample >= SDFB_SAMPLES {
            return None;
        }
        let offset = 64 + 2 * sample;
        let bits = u16::from_le_bytes([self.bytes[offset], self.bytes[offset + 1]]);
        Some(match self.grid.encoding {
            SdfSampleEncoding::ScaledI16 => self.grid.meters_per_i16 * f32::from(bits as i16),
            SdfSampleEncoding::F16 => half::f16::from_bits(bits).to_f32(),
        })
    }

    /// Solid membership for an X-fastest SDF sample in either format version.
    pub fn occupied(&self, sample: usize) -> Option<bool> {
        if sample >= SDFB_SAMPLES {
            return None;
        }
        let offset = 64 + 2 * SDFB_SAMPLES + sample / 8;
        Some(self.bytes[offset] & (1 << (sample % 8)) != 0)
    }

    /// Borrow the independently authored OpenVDB mask from the verified v3
    /// record. V2 records do not carry voxel activity.
    pub fn openvdb_active_mask(&self) -> Option<&'a OpenVdbActiveMask> {
        self.bytes.get(SDFB_RECORD_BYTES..)?.try_into().ok()
    }

    /// Read OpenVDB leaf activity at local brick coordinates in Z-fastest order.
    pub fn openvdb_active(&self, x: usize, y: usize, z: usize) -> Option<bool> {
        if x >= 8 || y >= 8 || z >= 8 {
            return None;
        }
        let bit = z + 8 * y + 64 * x;
        let mask = self.openvdb_active_mask()?;
        Some(mask[bit / 8] & (1 << (bit % 8)) != 0)
    }
}

/// Attach a verified sparse field and its wall/lumen bindings to a sealed HOLO asset.
/// This retains the original render, identity, provenance, and authored stratum fields.
pub fn seal_10d_with_sdfb(
    bytes: &[u8],
    sdfb: &[u8],
    wall_iri: &str,
    lumen_iri: &str,
) -> Result<Vec<u8>, String> {
    use super::{
        encode_container, encode_manifold_identity_v2, encode_symbolic_rdf_cbor_ld,
        manifold_identity_v2_encoded_len, parse_section_table, seal_whole_file_crc32c,
        AlignmentTier, Container10dHeader, MultilingualLexicalTerm, SectionInput,
        SectionType, SymbolicRdfCborLdPayload, TypedFieldRecord, TypedPayloadResolver,
        FIELD_KIND_HOLOGRAPHIC_PHASE_SPACE, FIELD_KIND_MULTISCALE_STRATA,
        FIELD_KIND_SYMBOLIC_RDF_CBOR_LD, MANIFOLD_IDENTITY_V2_MAX_FIELDS,
    };
    use super::manifold_identity_v2::FIELD_KIND_SIGNED_DISTANCE_BRICK;
    use crate::{bundle::semantic_model::valid_iri, q_hash, NQuin};

    const CONTEXT: &str = "https://qualiadb.org/contexts/10d-sdfb-v2";
    const PART_OF: &str = "http://purl.obolibrary.org/obo/BFO_0000050";
    if !valid_iri(wall_iri) || !valid_iri(lumen_iri) {
        return Err("SDFB wall and lumen require canonical RDF IRIs".into());
    }
    let wall = q_hash(wall_iri);
    let lumen = q_hash(lumen_iri);
    let sdf = SdfBrickView::parse(sdfb).map_err(|error| error.to_string())?;
    for brick in sdf.bricks() {
        if brick.wall_semantic_id() != wall || brick.lumen_semantic_id() != lumen {
            return Err("SDFB brick hashes differ from the supplied wall/lumen IRIs".into());
        }
    }
    if bytes.len().checked_add(sdfb.len()).is_none_or(|size| size > MAX_BYTES) {
        return Err("SDFB and its 10d base exceed the 128 MiB ring budget".into());
    }

    let header = Container10dHeader::parse(bytes).map_err(|error| error.to_string())?;
    let mut scratch = [TypedFieldRecord::default(); MANIFOLD_IDENTITY_V2_MAX_FIELDS];
    let mut records = [TypedFieldRecord::default(); MANIFOLD_IDENTITY_V2_MAX_FIELDS];
    let source = TypedPayloadResolver::new(bytes, &[], &mut scratch, &mut records)
        .map_err(|error| error.to_string())?;
    let identity = source.identity().ok_or("SDFB base requires MID2")?;
    let holo = source.payloads_of_kind(FIELD_KIND_HOLOGRAPHIC_PHASE_SPACE).next()
        .ok_or("SDFB base requires HOLO")?.map_err(|error| error.to_string())?;
    if holo.bytes.len() < super::HOLOGRAPHIC_PHASE_HEADER_SIZE
        || !holo.bytes.starts_with(&super::HOLOGRAPHIC_PHASE_TAG)
    {
        return Err("SDFB base has malformed HOLO".into());
    }
    if source.payloads_of_kind(FIELD_KIND_MULTISCALE_STRATA).next().is_none()
        || source.payloads_of_kind(FIELD_KIND_SYMBOLIC_RDF_CBOR_LD).next().is_none()
    {
        return Err("SDFB base requires MSTR and CBLD".into());
    }
    if source.payloads_of_kind(FIELD_KIND_SIGNED_DISTANCE_BRICK).next().is_some()
        || source.section_payload(SectionType::SignedDistanceBrick).is_some()
    {
        return Err("SDFB base already carries a signed-distance brick field".into());
    }

    let holo_header: super::HolographicPhaseHeader =
        bytemuck::pod_read_unaligned(&holo.bytes[..super::HOLOGRAPHIC_PHASE_HEADER_SIZE]);
    let part_of = q_hash(PART_OF);
    let root = holo_header.nquin_subject_hash;
    let context = q_hash(CONTEXT);
    if root == 0 || root == wall || root == lumen {
        return Err("SDFB wall and lumen require identities distinct from the asset root".into());
    }
    let cbld = encode_symbolic_rdf_cbor_ld(&SymbolicRdfCborLdPayload {
        context_iri: CONTEXT.into(),
        lexicon: [wall_iri, lumen_iri, PART_OF, CONTEXT].into_iter()
            .map(|iri| MultilingualLexicalTerm {
                term_hash: q_hash(iri), iri_utf8: iri.into(), lang_bcp47: None, label_utf8: String::new(),
            }).collect(),
        nquins: [wall, lumen].into_iter().map(|s| [
            s, part_of, root, context, 0, NQuin::calculate_parity(s, part_of, root, context, 0),
        ]).collect(),
        ..Default::default()
    }).map_err(|error| error.to_string())?;

    let mut sidecar = source.section_payload(SectionType::FieldSidecar).unwrap_or_default().to_vec();
    let cbld_offset = sidecar.len() as u64;
    sidecar.extend_from_slice(&cbld);
    let mut fields = source.fields().to_vec();
    let last = fields.last().map_or(0, |field| field.field_id);
    let end = last.checked_add(2).ok_or("SDFB field IDs exhausted")?;
    fields.extend([
        TypedFieldRecord {
            field_id: end - 1,
            kind: FIELD_KIND_SYMBOLIC_RDF_CBOR_LD,
            payload_section_id: SectionType::FieldSidecar as u32,
            payload_offset: cbld_offset,
            payload_length: cbld.len() as u64,
            ..TypedFieldRecord::default()
        },
        TypedFieldRecord {
            field_id: end,
            kind: FIELD_KIND_SIGNED_DISTANCE_BRICK,
            payload_section_id: SectionType::SignedDistanceBrick as u32,
            payload_length: sdfb.len() as u64,
            ..TypedFieldRecord::default()
        },
    ]);
    let mid2_len = manifold_identity_v2_encoded_len(fields.len()).map_err(|error| format!("{error:?}"))?;
    let mut mid2 = vec![0; mid2_len];
    encode_manifold_identity_v2(identity, &fields, &mut mid2).map_err(|error| format!("{error:?}"))?;
    let mut sections = Vec::new();
    for descriptor in parse_section_table(bytes, &header).map_err(|error| error.to_string())? {
        let kind = descriptor.typ().ok_or("Unknown 10d section")?;
        if matches!(kind, SectionType::ManifoldIdentityV2 | SectionType::FieldSidecar) {
            continue;
        }
        sections.push(SectionInput {
            section_type: kind,
            alignment_tier: AlignmentTier::from_u8(descriptor.alignment_tier).ok_or("Invalid section tier")?,
            stride: descriptor.stride,
            element_count: descriptor.element_count,
            payload: source.section_payload(kind).ok_or("Missing section bytes")?,
        });
    }
    for (kind, payload) in [
        (SectionType::FieldSidecar, sidecar.as_slice()),
        (SectionType::ManifoldIdentityV2, mid2.as_slice()),
        (SectionType::SignedDistanceBrick, sdfb),
    ] {
        sections.push(SectionInput {
            section_type: kind, alignment_tier: AlignmentTier::Page,
            stride: 0, element_count: 0, payload,
        });
    }
    let overhead = (sections.len() + 1).saturating_mul(4096);
    let capacity = sections.iter().try_fold(overhead, |len, item| len.checked_add(item.payload.len()))
        .ok_or("SDFB container size overflow")?;
    let mut output = vec![0; capacity.min(MAX_BYTES)];
    let written = encode_container(&header, &sections, &mut output).map_err(|error| error.to_string())?;
    output.truncate(written);
    seal_whole_file_crc32c(&mut output);
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sparse_field_roundtrips_refined_bricks_and_rejects_tampering() {
        let grid = SdfBrickGrid {
            origin_m: [0.0; 3],
            voxel_spacing_m: [0.25; 3],
            base_bricks: [1; 3],
            encoding: SdfSampleEncoding::ScaledI16,
            meters_per_i16: 0.125,
        };
        let mut base = SdfBrickRecord {
            key: SdfBrickKey { refinement_level: 0, coordinate: [0; 3] },
            wall_semantic_id: crate::q_hash("urn:test:wall"),
            lumen_semantic_id: crate::q_hash("urn:test:lumen"),
            samples: [(-2i16) as u16; SDFB_SAMPLES],
            occupancy: [0xff; SDFB_OCCUPANCY_BYTES],
        };
        let mut refined = base.clone();
        refined.key = SdfBrickKey { refinement_level: 1, coordinate: [1; 3] };
        refined.samples[0] = 1;
        refined.occupancy[0] &= !1;
        let mut bytes = vec![0; sdfb_encoded_len(2).unwrap()];
        encode_sdfb_into(grid, &[base.clone(), refined.clone()], &mut bytes).unwrap();
        let view = SdfBrickView::parse(&bytes).unwrap();
        let brick = view.bricks().nth(1).unwrap();
        assert_eq!(brick.key(), refined.key);
        assert_eq!(brick.wall_semantic_id(), base.wall_semantic_id);
        assert_eq!(brick.lumen_semantic_id(), base.lumen_semantic_id);
        assert_eq!(brick.distance_m(0), Some(0.125));
        assert_eq!(brick.occupied(0), Some(false));
        assert_eq!(brick.distance_m(1), Some(-0.25));
        assert_eq!(brick.occupied(1), Some(true));
        bytes[SDFB_HEADER_BYTES + 10] ^= 1;
        assert!(matches!(SdfBrickView::parse(&bytes), Err(SdfBrickError::Checksum)));
        assert_eq!(encode_sdfb_into(grid, &[refined, base.clone()], &mut bytes), Err(SdfBrickError::Order));
        base.lumen_semantic_id = base.wall_semantic_id;
        assert_eq!(encode_sdfb_into(grid, &[base], &mut bytes), Err(SdfBrickError::InvalidSemanticId));
    }

    #[test]
    fn openvdb_activity_is_a_borrowed_independent_z_fastest_mask() {
        let grid = SdfBrickGrid {
            origin_m: [0.0; 3],
            voxel_spacing_m: [0.25; 3],
            base_bricks: [2; 3],
            encoding: SdfSampleEncoding::ScaledI16,
            meters_per_i16: 0.125,
        };
        let mut first = SdfBrickRecord {
            key: SdfBrickKey { refinement_level: 0, coordinate: [0; 3] },
            wall_semantic_id: 1,
            lumen_semantic_id: 2,
            samples: [2; SDFB_SAMPLES],
            occupancy: [0; SDFB_OCCUPANCY_BYTES],
        };
        first.samples[1] = (-2i16) as u16;
        first.occupancy[0] = 1 << 1;
        let mut next = first.clone();
        next.key.coordinate[0] = 1;
        let records = [first, next];
        let mut v2 = vec![0; sdfb_encoded_len(records.len()).unwrap()];
        encode_sdfb_into(grid, &records, &mut v2).unwrap();
        let mut v3 = vec![0; sdfb_openvdb_encoded_len(records.len()).unwrap()];
        let mut masks = [[0; SDFB_OPENVDB_ACTIVE_MASK_BYTES]; 2];
        masks[1][0] = 1 << 1; // Local (0, 0, 1), opposite of X-fastest bit 1.
        encode_sdfb_with_openvdb_active_masks_into(grid, &records, &masks, &mut v3).unwrap();

        assert_eq!(&v2[4..6], &VERSION_V2.to_le_bytes());
        assert_eq!(&v3[4..6], &VERSION_OPENVDB.to_le_bytes());
        for index in 0..records.len() {
            let start_v2 = SDFB_HEADER_BYTES + index * SDFB_RECORD_BYTES;
            let start_v3 = SDFB_HEADER_BYTES + index * SDFB_OPENVDB_RECORD_BYTES;
            assert_eq!(
                &v3[start_v3..start_v3 + SDFB_RECORD_BYTES],
                &v2[start_v2..start_v2 + SDFB_RECORD_BYTES],
            );
        }
        let old = SdfBrickView::parse(&v2).unwrap();
        assert!(!old.has_openvdb_active_masks());
        assert_eq!(old.bricks().next().unwrap().openvdb_active_mask(), None);
        let view = SdfBrickView::parse(&v3).unwrap();
        assert!(view.has_openvdb_active_masks());
        let brick = view.brick(records[1].key).unwrap();
        assert_eq!(brick.occupied(1), Some(true));
        assert_eq!(brick.occupied(64), Some(false));
        assert_eq!(brick.openvdb_active(1, 0, 0), Some(false));
        assert_eq!(brick.openvdb_active(0, 0, 1), Some(true));
        assert_eq!(brick.openvdb_active(8, 0, 0), None);
        let offset = SDFB_HEADER_BYTES + SDFB_OPENVDB_RECORD_BYTES + SDFB_RECORD_BYTES;
        assert_eq!(brick.openvdb_active_mask().unwrap().as_ptr(), v3[offset..].as_ptr());
        v3[offset] ^= 1;
        assert!(matches!(SdfBrickView::parse(&v3), Err(SdfBrickError::Checksum)));
        assert_eq!(
            encode_sdfb_with_openvdb_active_masks_into(grid, &records, &masks[..1], &mut v3),
            Err(SdfBrickError::Length),
        );
    }
}
