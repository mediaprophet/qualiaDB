//! Two-Tier Geometry & Embodiment Reference Codecs (`AREF`, `IOVR`, `INST`).
//!
//! Parallels how `.emf` (`Q42EMF64`), `.qac` (`qac\0`), and `.m64` (`m64\0`)
//! separate domain embodiments from the primary 10D computational manifold:
//!
//! - **Mode A (Archetype Inline Constituency):** A canonical species (`intenteco`)
//!   or made-world thing (`qualia-poiesis`) `.10d` embeds shared geometry, LODs,
//!   rigging (`SKL2`/`SSM1`), and materials (`MAT4`) inline.
//! - **Mode B (Instance / Population Reference):** An individual entity or
//!   multi-instance population `.10d` holds its primary computational state
//!   (`MID2` 10D coordinates `[q, v, w, x, y, z, t, α, μ, σ]`, `EONT` ontogenetic
//!   state, `PROC` 64-bit seed, `MSTR` multi-scale strata) and references the
//!   shared Mode-A Archetype via:
//!   - [`ArchetypeRefRecord`] (`FIELD_KIND_ARCHETYPE_REF = 0x0301`, `AREF`, 80B)
//!   - [`InstanceOverrideHeader`] + [`SsmInstanceCoeff`] (`FIELD_KIND_INSTANCE_OVERRIDES = 0x0302`, `IOVR`)
//!   - [`InstanceBatchHeader`] + [`InstanceBatchRecord`] (`FIELD_KIND_INSTANCE_BATCH = 0x0303`, `INST`, `N * 64B`)
//!
//! All hot encode/decode routines are caller-buffered and allocate zero heap.
//! Binary format tags are 32-bit little-endian discriminants; all ontological
//! identities use 64-bit RDF `q_hash` or 128-bit `StableEntityId` handles.

use bytemuck::{bytes_of, pod_read_unaligned, Pod, Zeroable};

use crate::container_10d::crc32c::crc32c;

/// 32-bit binary format discriminant for [`ArchetypeRefRecord`] (`0x4645_5241` LE).
pub const ARCHETYPE_REF_TAG: [u8; 4] = *b"AREF";
pub const ARCHETYPE_REF_VERSION: u16 = 1;
pub const ARCHETYPE_REF_RECORD_SIZE: usize = 80;

/// 32-bit binary format discriminant for [`InstanceOverrideHeader`] (`0x5256_4F49` LE).
pub const INSTANCE_OVERRIDE_TAG: [u8; 4] = *b"IOVR";
pub const INSTANCE_OVERRIDE_VERSION: u16 = 1;
pub const INSTANCE_OVERRIDE_HEADER_SIZE: usize = 48;
pub const SSM_INSTANCE_COEFF_SIZE: usize = 8;
pub const MAX_INSTANCE_SSM_COEFFS: usize = 256;

/// 32-bit binary format discriminant for [`InstanceBatchHeader`] (`0x5453_4E49` LE).
pub const INSTANCE_BATCH_TAG: [u8; 4] = *b"INST";
pub const INSTANCE_BATCH_VERSION: u16 = 1;
pub const INSTANCE_BATCH_HEADER_SIZE: usize = 16;
pub const INSTANCE_BATCH_RECORD_SIZE: usize = 64;
pub const MAX_INSTANCE_BATCH_RECORDS: usize = 262_144;

/// Override bitflags for [`ArchetypeRefRecord::override_flags`].
pub const OVERRIDE_FLAG_COORDS_10D: u32 = 1 << 0;
pub const OVERRIDE_FLAG_EONT_STATE: u32 = 1 << 1;
pub const OVERRIDE_FLAG_PROC_SEED: u32 = 1 << 2;
pub const OVERRIDE_FLAG_SSM_WEIGHTS: u32 = 1 << 3;
pub const OVERRIDE_FLAG_MATERIAL_VARIANT: u32 = 1 << 4;
pub const OVERRIDE_FLAG_BATCH_NODES: u32 = 1 << 5;
pub const OVERRIDE_FLAG_MULTISCALE_STRATA: u32 = 1 << 6;

/// Cryptographic digest algorithm used in [`ArchetypeRefRecord::digest_alg`].
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArchetypeDigestAlg {
    Blake3_256 = 1,
    Sha256 = 2,
}

impl ArchetypeDigestAlg {
    #[inline]
    pub const fn from_u8(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::Blake3_256),
            2 => Some(Self::Sha256),
            _ => None,
        }
    }
}

/// Target container class referenced by [`ArchetypeRefRecord::target_kind`].
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArchetypeTargetKind {
    /// Standalone Mode-A Archetype `.10d` container.
    Archetype10D = 1,
    /// Named sub-resource entry inside a streamable `.hmc` (`QBDL`) bundle.
    HmcBundleEntry = 2,
    /// Sibling `.emf` (`Q42EMF64`) electromagnetic/radiance volume container.
    EmfSibling = 3,
    /// Sibling `.qac` (`qac\0`) spatial/procedural/AAC acoustic container.
    QacSibling = 4,
    /// Sibling `.m64` (`m64\0`) physical matter/tissue/strata container.
    M64Sibling = 5,
}

impl ArchetypeTargetKind {
    #[inline]
    pub const fn from_u8(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::Archetype10D),
            2 => Some(Self::HmcBundleEntry),
            3 => Some(Self::EmfSibling),
            4 => Some(Self::QacSibling),
            5 => Some(Self::M64Sibling),
            _ => None,
        }
    }
}

/// 80-byte fixed-layout POD record binding an instance or population `.10d`
/// to an external Mode-A Archetype `.10d` or sibling domain container (`FIELD_KIND_ARCHETYPE_REF = 0x0301`).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct ArchetypeRefRecord {
    pub magic: [u8; 4],
    pub version: u16,
    pub digest_alg: u8,
    pub target_kind: u8,
    pub override_flags: u32,
    pub record_crc32c: u32,
    /// Full 256-bit cryptographic digest (`BLAKE3` or `SHA-256`) of the referenced archetype.
    pub archetype_digest: [u8; 32],
    /// 128-bit `StableEntityId` of the canonical archetype.
    pub archetype_entity_id: [u8; 16],
    /// 64-bit `q_hash` of the full UTF-8 RDF species taxon IRI or made-world class IRI.
    pub taxon_or_class_iri_hash: u64,
    /// 64-bit `q_hash` of the sub-resource key within an `.hmc` bundle (`0` if standalone).
    pub subresource_key_hash: u64,
}

const _: () = assert!(core::mem::size_of::<ArchetypeRefRecord>() == ARCHETYPE_REF_RECORD_SIZE);

impl ArchetypeRefRecord {
    /// Construct a new validated [`ArchetypeRefRecord`] and seal its CRC-32C word.
    #[inline]
    pub fn new(
        digest_alg: ArchetypeDigestAlg,
        target_kind: ArchetypeTargetKind,
        override_flags: u32,
        archetype_digest: [u8; 32],
        archetype_entity_id: [u8; 16],
        taxon_or_class_iri_hash: u64,
        subresource_key_hash: u64,
    ) -> Self {
        let mut rec = Self {
            magic: ARCHETYPE_REF_TAG,
            version: ARCHETYPE_REF_VERSION,
            digest_alg: digest_alg as u8,
            target_kind: target_kind as u8,
            override_flags,
            record_crc32c: 0,
            archetype_digest,
            archetype_entity_id,
            taxon_or_class_iri_hash,
            subresource_key_hash,
        };
        rec.record_crc32c = rec.compute_crc32c();
        rec
    }

    /// Compute CRC-32C over all bytes of the record with `record_crc32c` zeroed.
    #[inline]
    pub fn compute_crc32c(&self) -> u32 {
        let mut copy = *self;
        copy.record_crc32c = 0;
        crc32c(bytes_of(&copy))
    }
}

/// 48-byte fixed-layout POD header for single-instance overrides (`FIELD_KIND_INSTANCE_OVERRIDES = 0x0302`).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct InstanceOverrideHeader {
    pub magic: [u8; 4],
    pub version: u16,
    pub coeff_count: u16,
    /// Ontogenetic or operational age in days.
    pub age_days: f32,
    /// Uniform allometric / dimensional scale multiplier (`1.0` = nominal archetype scale).
    pub uniform_scale: f32,
    /// Local 3D PGA $\mathcal{G}_{3,0,1}$ motor `[s, e23, e31, e12, e01, e02, e03, e0123]`.
    pub local_pga_motor: [f32; 8],
}

const _: () =
    assert!(core::mem::size_of::<InstanceOverrideHeader>() == INSTANCE_OVERRIDE_HEADER_SIZE);

/// 8-byte POD record supplying an instance's realized coefficient for one `SSM1` PCA shape mode.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable, Default)]
pub struct SsmInstanceCoeff {
    pub mode_id: u32,
    /// Realized standard-deviation weight $\sigma_i$ within the archetype's `[min_sigma, max_sigma]`.
    pub sigma_weight: f32,
}

const _: () = assert!(core::mem::size_of::<SsmInstanceCoeff>() == SSM_INSTANCE_COEFF_SIZE);

/// 16-byte fixed-layout POD header for a multi-instance population batch (`FIELD_KIND_INSTANCE_BATCH = 0x0303`).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct InstanceBatchHeader {
    pub magic: [u8; 4],
    pub version: u16,
    pub flags: u16,
    pub instance_count: u32,
    pub payload_crc32c: u32,
}

const _: () = assert!(core::mem::size_of::<InstanceBatchHeader>() == INSTANCE_BATCH_HEADER_SIZE);

/// 64-byte cache-line-aligned POD record representing one instance in a multi-instance `.10d`
/// population cloud, indexed 1:1 with `SectionType::Tensor10DNodes` (`[q, v, w, x, y, z, t, α, μ, σ]`).
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct InstanceBatchRecord {
    /// 64-bit entity identity hash (`did:q42:` or RDF IRI `q_hash`).
    pub entity_id_hash: u64,
    /// Deterministic 64-bit procedural seed driving Voronoi micro-surfaces, allometry, and phase.
    pub proc_seed: u64,
    /// Ontogenetic or service age in days.
    pub age_days: f32,
    /// Uniform scale multiplier (`1.0` = nominal archetype size).
    pub uniform_scale: f32,
    /// Ontogenetic stage (`0=Seed/Spore, 1=Juvenile, 2=Adult, 3=Senescent, 4=Deceased`).
    pub stage: u8,
    /// Belnap 4-valued truth/epistemic flags (`0=Unknown, 1=True, 2=False, 3=Both/Paraconsistent`).
    pub epistemic_flags: u8,
    /// Material variant / palette slot selector.
    pub material_variant: u16,
    /// Metabolic energy in permill (`0..=1000`).
    pub energy_permill: u16,
    /// Structural/tissue damage in permill (`0..=1000`).
    pub damage_permill: u16,
    /// Hydration / turgor pressure in permill (`0..=1000`).
    pub hydration_permill: u16,
    /// Mechanical wear / weathering in permill (`0..=1000`).
    pub wear_permill: u16,
    /// Top 4 principal shape component weights quantized in `Q4.11` (`i16 / 2048.0` $\sigma$ units).
    pub ssm_weights_q11: [i16; 4],
    /// Unit PGA rotation rotor `[s, e23, e31, e12]`.
    pub pga_rotor: [f32; 4],
    /// Castagnoli CRC-32C over bytes `0..60`.
    pub record_crc32c: u32,
}

const _: () = assert!(core::mem::size_of::<InstanceBatchRecord>() == INSTANCE_BATCH_RECORD_SIZE);
const _: () = assert!(core::mem::align_of::<InstanceBatchRecord>() == 64);

impl Default for InstanceBatchRecord {
    fn default() -> Self {
        let mut rec = Self {
            entity_id_hash: 0,
            proc_seed: 0,
            age_days: 0.0,
            uniform_scale: 1.0,
            stage: 2,
            epistemic_flags: 1,
            material_variant: 0,
            energy_permill: 1000,
            damage_permill: 0,
            hydration_permill: 1000,
            wear_permill: 0,
            ssm_weights_q11: [0; 4],
            pga_rotor: [1.0, 0.0, 0.0, 0.0],
            record_crc32c: 0,
        };
        rec.record_crc32c = rec.compute_crc32c();
        rec
    }
}

impl InstanceBatchRecord {
    #[inline]
    pub fn compute_crc32c(&self) -> u32 {
        let bytes: &[u8; INSTANCE_BATCH_RECORD_SIZE] = bytemuck::cast_ref(self);
        crc32c(&bytes[..60])
    }

    #[inline]
    pub fn seal_crc32c(&mut self) {
        self.record_crc32c = self.compute_crc32c();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchetypeRefError {
    PayloadTooShort { need: usize, have: usize },
    OutputTooSmall { need: usize, have: usize },
    BadMagic([u8; 4]),
    UnsupportedVersion(u16),
    InvalidDigestAlg(u8),
    InvalidTargetKind(u8),
    ZeroArchetypeDigest,
    ZeroEntityId,
    NonFiniteFloat,
    CountLimitExceeded { got: usize, max: usize },
    LengthMismatch { expected: usize, actual: usize },
    ChecksumMismatch { expected: u32, actual: u32 },
}

impl core::fmt::Display for ArchetypeRefError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for ArchetypeRefError {}

/// Encode an [`ArchetypeRefRecord`] (`AREF`) into a caller-supplied buffer. Zero heap.
pub fn encode_archetype_ref_record(
    record: &ArchetypeRefRecord,
    out: &mut [u8],
) -> Result<usize, ArchetypeRefError> {
    if out.len() < ARCHETYPE_REF_RECORD_SIZE {
        return Err(ArchetypeRefError::OutputTooSmall {
            need: ARCHETYPE_REF_RECORD_SIZE,
            have: out.len(),
        });
    }
    if record.magic != ARCHETYPE_REF_TAG {
        return Err(ArchetypeRefError::BadMagic(record.magic));
    }
    if record.version != ARCHETYPE_REF_VERSION {
        return Err(ArchetypeRefError::UnsupportedVersion(record.version));
    }
    if ArchetypeDigestAlg::from_u8(record.digest_alg).is_none() {
        return Err(ArchetypeRefError::InvalidDigestAlg(record.digest_alg));
    }
    if ArchetypeTargetKind::from_u8(record.target_kind).is_none() {
        return Err(ArchetypeRefError::InvalidTargetKind(record.target_kind));
    }
    if record.archetype_digest.iter().all(|&b| b == 0) {
        return Err(ArchetypeRefError::ZeroArchetypeDigest);
    }
    if record.archetype_entity_id.iter().all(|&b| b == 0) {
        return Err(ArchetypeRefError::ZeroEntityId);
    }
    let mut sealed = *record;
    sealed.record_crc32c = sealed.compute_crc32c();
    out[..ARCHETYPE_REF_RECORD_SIZE].copy_from_slice(bytes_of(&sealed));
    Ok(ARCHETYPE_REF_RECORD_SIZE)
}

/// Decode and verify an [`ArchetypeRefRecord`] (`AREF`) from a byte slice. Zero heap.
pub fn decode_archetype_ref_record(payload: &[u8]) -> Result<ArchetypeRefRecord, ArchetypeRefError> {
    if payload.len() < ARCHETYPE_REF_RECORD_SIZE {
        return Err(ArchetypeRefError::PayloadTooShort {
            need: ARCHETYPE_REF_RECORD_SIZE,
            have: payload.len(),
        });
    }
    if payload.len() != ARCHETYPE_REF_RECORD_SIZE {
        return Err(ArchetypeRefError::LengthMismatch {
            expected: ARCHETYPE_REF_RECORD_SIZE,
            actual: payload.len(),
        });
    }
    let rec: ArchetypeRefRecord = pod_read_unaligned(&payload[..ARCHETYPE_REF_RECORD_SIZE]);
    if rec.magic != ARCHETYPE_REF_TAG {
        return Err(ArchetypeRefError::BadMagic(rec.magic));
    }
    if rec.version != ARCHETYPE_REF_VERSION {
        return Err(ArchetypeRefError::UnsupportedVersion(rec.version));
    }
    if ArchetypeDigestAlg::from_u8(rec.digest_alg).is_none() {
        return Err(ArchetypeRefError::InvalidDigestAlg(rec.digest_alg));
    }
    if ArchetypeTargetKind::from_u8(rec.target_kind).is_none() {
        return Err(ArchetypeRefError::InvalidTargetKind(rec.target_kind));
    }
    if rec.archetype_digest.iter().all(|&b| b == 0) {
        return Err(ArchetypeRefError::ZeroArchetypeDigest);
    }
    if rec.archetype_entity_id.iter().all(|&b| b == 0) {
        return Err(ArchetypeRefError::ZeroEntityId);
    }
    let computed = rec.compute_crc32c();
    if rec.record_crc32c != computed {
        return Err(ArchetypeRefError::ChecksumMismatch {
            expected: rec.record_crc32c,
            actual: computed,
        });
    }
    Ok(rec)
}

/// Compute exact byte length for an `IOVR` instance override payload.
#[inline]
pub const fn instance_override_encoded_len(coeff_count: usize) -> Option<usize> {
    if coeff_count > MAX_INSTANCE_SSM_COEFFS {
        return None;
    }
    Some(INSTANCE_OVERRIDE_HEADER_SIZE + coeff_count * SSM_INSTANCE_COEFF_SIZE)
}

/// Encode an `IOVR` instance override header and SSM1 coefficients into a caller buffer. Zero heap.
pub fn encode_instance_overrides(
    age_days: f32,
    uniform_scale: f32,
    local_pga_motor: [f32; 8],
    coeffs: &[SsmInstanceCoeff],
    out: &mut [u8],
) -> Result<usize, ArchetypeRefError> {
    if coeffs.len() > MAX_INSTANCE_SSM_COEFFS {
        return Err(ArchetypeRefError::CountLimitExceeded {
            got: coeffs.len(),
            max: MAX_INSTANCE_SSM_COEFFS,
        });
    }
    if !age_days.is_finite()
        || !uniform_scale.is_finite()
        || uniform_scale <= 0.0
        || local_pga_motor.iter().any(|v| !v.is_finite())
        || coeffs.iter().any(|c| !c.sigma_weight.is_finite())
    {
        return Err(ArchetypeRefError::NonFiniteFloat);
    }
    let needed = INSTANCE_OVERRIDE_HEADER_SIZE + coeffs.len() * SSM_INSTANCE_COEFF_SIZE;
    if out.len() < needed {
        return Err(ArchetypeRefError::OutputTooSmall {
            need: needed,
            have: out.len(),
        });
    }
    let header = InstanceOverrideHeader {
        magic: INSTANCE_OVERRIDE_TAG,
        version: INSTANCE_OVERRIDE_VERSION,
        coeff_count: coeffs.len() as u16,
        age_days,
        uniform_scale,
        local_pga_motor,
    };
    out[..INSTANCE_OVERRIDE_HEADER_SIZE].copy_from_slice(bytes_of(&header));
    let mut cursor = INSTANCE_OVERRIDE_HEADER_SIZE;
    for coeff in coeffs {
        out[cursor..cursor + SSM_INSTANCE_COEFF_SIZE].copy_from_slice(bytes_of(coeff));
        cursor += SSM_INSTANCE_COEFF_SIZE;
    }
    Ok(needed)
}

/// Decode an `IOVR` instance override payload into a caller-supplied coefficient slice. Zero heap.
pub fn decode_instance_overrides_into(
    payload: &[u8],
    out_coeffs: &mut [SsmInstanceCoeff],
) -> Result<(InstanceOverrideHeader, usize), ArchetypeRefError> {
    if payload.len() < INSTANCE_OVERRIDE_HEADER_SIZE {
        return Err(ArchetypeRefError::PayloadTooShort {
            need: INSTANCE_OVERRIDE_HEADER_SIZE,
            have: payload.len(),
        });
    }
    let header: InstanceOverrideHeader =
        pod_read_unaligned(&payload[..INSTANCE_OVERRIDE_HEADER_SIZE]);
    if header.magic != INSTANCE_OVERRIDE_TAG {
        return Err(ArchetypeRefError::BadMagic(header.magic));
    }
    if header.version != INSTANCE_OVERRIDE_VERSION {
        return Err(ArchetypeRefError::UnsupportedVersion(header.version));
    }
    let count = header.coeff_count as usize;
    if count > MAX_INSTANCE_SSM_COEFFS {
        return Err(ArchetypeRefError::CountLimitExceeded {
            got: count,
            max: MAX_INSTANCE_SSM_COEFFS,
        });
    }
    let expected = INSTANCE_OVERRIDE_HEADER_SIZE + count * SSM_INSTANCE_COEFF_SIZE;
    if payload.len() != expected {
        return Err(ArchetypeRefError::LengthMismatch {
            expected,
            actual: payload.len(),
        });
    }
    if !header.age_days.is_finite()
        || !header.uniform_scale.is_finite()
        || header.uniform_scale <= 0.0
        || header.local_pga_motor.iter().any(|v| !v.is_finite())
    {
        return Err(ArchetypeRefError::NonFiniteFloat);
    }
    if out_coeffs.len() < count {
        return Err(ArchetypeRefError::OutputTooSmall {
            need: count,
            have: out_coeffs.len(),
        });
    }
    for i in 0..count {
        let start = INSTANCE_OVERRIDE_HEADER_SIZE + i * SSM_INSTANCE_COEFF_SIZE;
        let coeff: SsmInstanceCoeff =
            pod_read_unaligned(&payload[start..start + SSM_INSTANCE_COEFF_SIZE]);
        if !coeff.sigma_weight.is_finite() {
            return Err(ArchetypeRefError::NonFiniteFloat);
        }
        out_coeffs[i] = coeff;
    }
    Ok((header, count))
}

/// Compute exact byte length for an `INST` multi-instance batch payload.
#[inline]
pub const fn instance_batch_encoded_len(instance_count: usize) -> Option<usize> {
    if instance_count > MAX_INSTANCE_BATCH_RECORDS {
        return None;
    }
    Some(INSTANCE_BATCH_HEADER_SIZE + instance_count * INSTANCE_BATCH_RECORD_SIZE)
}

/// Encode an `INST` multi-instance batch payload into a caller-supplied buffer. Zero heap.
pub fn encode_instance_batch(
    records: &[InstanceBatchRecord],
    out: &mut [u8],
) -> Result<usize, ArchetypeRefError> {
    if records.len() > MAX_INSTANCE_BATCH_RECORDS {
        return Err(ArchetypeRefError::CountLimitExceeded {
            got: records.len(),
            max: MAX_INSTANCE_BATCH_RECORDS,
        });
    }
    let needed = INSTANCE_BATCH_HEADER_SIZE + records.len() * INSTANCE_BATCH_RECORD_SIZE;
    if out.len() < needed {
        return Err(ArchetypeRefError::OutputTooSmall {
            need: needed,
            have: out.len(),
        });
    }
    let mut cursor = INSTANCE_BATCH_HEADER_SIZE;
    for rec in records {
        if !rec.age_days.is_finite()
            || !rec.uniform_scale.is_finite()
            || rec.uniform_scale <= 0.0
            || rec.pga_rotor.iter().any(|v| !v.is_finite())
        {
            return Err(ArchetypeRefError::NonFiniteFloat);
        }
        let mut sealed = *rec;
        sealed.seal_crc32c();
        out[cursor..cursor + INSTANCE_BATCH_RECORD_SIZE].copy_from_slice(bytes_of(&sealed));
        cursor += INSTANCE_BATCH_RECORD_SIZE;
    }
    let body_crc = crc32c(&out[INSTANCE_BATCH_HEADER_SIZE..needed]);
    let header = InstanceBatchHeader {
        magic: INSTANCE_BATCH_TAG,
        version: INSTANCE_BATCH_VERSION,
        flags: 0,
        instance_count: records.len() as u32,
        payload_crc32c: body_crc,
    };
    out[..INSTANCE_BATCH_HEADER_SIZE].copy_from_slice(bytes_of(&header));
    Ok(needed)
}

/// Decode and verify an `INST` multi-instance batch payload into caller storage. Zero heap.
pub fn decode_instance_batch_into(
    payload: &[u8],
    out_records: &mut [InstanceBatchRecord],
) -> Result<(InstanceBatchHeader, usize), ArchetypeRefError> {
    if payload.len() < INSTANCE_BATCH_HEADER_SIZE {
        return Err(ArchetypeRefError::PayloadTooShort {
            need: INSTANCE_BATCH_HEADER_SIZE,
            have: payload.len(),
        });
    }
    let header: InstanceBatchHeader =
        pod_read_unaligned(&payload[..INSTANCE_BATCH_HEADER_SIZE]);
    if header.magic != INSTANCE_BATCH_TAG {
        return Err(ArchetypeRefError::BadMagic(header.magic));
    }
    if header.version != INSTANCE_BATCH_VERSION {
        return Err(ArchetypeRefError::UnsupportedVersion(header.version));
    }
    let count = header.instance_count as usize;
    if count > MAX_INSTANCE_BATCH_RECORDS {
        return Err(ArchetypeRefError::CountLimitExceeded {
            got: count,
            max: MAX_INSTANCE_BATCH_RECORDS,
        });
    }
    let expected = INSTANCE_BATCH_HEADER_SIZE + count * INSTANCE_BATCH_RECORD_SIZE;
    if payload.len() != expected {
        return Err(ArchetypeRefError::LengthMismatch {
            expected,
            actual: payload.len(),
        });
    }
    if out_records.len() < count {
        return Err(ArchetypeRefError::OutputTooSmall {
            need: count,
            have: out_records.len(),
        });
    }
    let body_crc = crc32c(&payload[INSTANCE_BATCH_HEADER_SIZE..expected]);
    if header.payload_crc32c != body_crc {
        return Err(ArchetypeRefError::ChecksumMismatch {
            expected: header.payload_crc32c,
            actual: body_crc,
        });
    }
    for i in 0..count {
        let start = INSTANCE_BATCH_HEADER_SIZE + i * INSTANCE_BATCH_RECORD_SIZE;
        let rec: InstanceBatchRecord =
            pod_read_unaligned(&payload[start..start + INSTANCE_BATCH_RECORD_SIZE]);
        let rec_crc = rec.compute_crc32c();
        if rec.record_crc32c != rec_crc {
            return Err(ArchetypeRefError::ChecksumMismatch {
                expected: rec.record_crc32c,
                actual: rec_crc,
            });
        }
        out_records[i] = rec;
    }
    Ok((header, count))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archetype_ref_record_roundtrips_and_detects_corruption() {
        let rec = ArchetypeRefRecord::new(
            ArchetypeDigestAlg::Blake3_256,
            ArchetypeTargetKind::Archetype10D,
            OVERRIDE_FLAG_COORDS_10D | OVERRIDE_FLAG_EONT_STATE | OVERRIDE_FLAG_PROC_SEED,
            [0x42; 32],
            [0x11; 16],
            0xCAFE_BABE_1234_5678,
            0,
        );
        let mut buf = [0u8; ARCHETYPE_REF_RECORD_SIZE];
        let written = encode_archetype_ref_record(&rec, &mut buf).unwrap();
        assert_eq!(written, ARCHETYPE_REF_RECORD_SIZE);

        let decoded = decode_archetype_ref_record(&buf).unwrap();
        assert_eq!(decoded, rec);

        buf[40] ^= 0x01;
        assert!(matches!(
            decode_archetype_ref_record(&buf),
            Err(ArchetypeRefError::ChecksumMismatch { .. })
        ));
    }

    #[test]
    fn instance_overrides_and_batch_roundtrip_zero_heap() {
        let coeffs = [
            SsmInstanceCoeff {
                mode_id: 0,
                sigma_weight: 1.25,
            },
            SsmInstanceCoeff {
                mode_id: 1,
                sigma_weight: -0.75,
            },
        ];
        let motor = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let mut iovr_buf = [0u8; 128];
        let n = encode_instance_overrides(365.25, 1.15, motor, &coeffs, &mut iovr_buf).unwrap();
        let mut out_coeffs = [SsmInstanceCoeff::default(); 4];
        let (hdr, count) = decode_instance_overrides_into(&iovr_buf[..n], &mut out_coeffs).unwrap();
        assert_eq!(count, 2);
        assert_eq!(hdr.age_days, 365.25);
        assert_eq!(&out_coeffs[..2], &coeffs);

        let batch = [
            InstanceBatchRecord {
                entity_id_hash: 1001,
                proc_seed: 0x9988_7766_5544_3322,
                age_days: 1200.0,
                uniform_scale: 1.08,
                ..InstanceBatchRecord::default()
            },
            InstanceBatchRecord {
                entity_id_hash: 1002,
                proc_seed: 0x1122_3344_5566_7788,
                age_days: 450.0,
                uniform_scale: 0.85,
                ..InstanceBatchRecord::default()
            },
        ];
        let mut inst_buf = [0u8; 256];
        let inst_len = encode_instance_batch(&batch, &mut inst_buf).unwrap();
        let mut decoded_batch = [InstanceBatchRecord::default(); 2];
        let (bhdr, bcount) =
            decode_instance_batch_into(&inst_buf[..inst_len], &mut decoded_batch).unwrap();
        assert_eq!(bcount, 2);
        assert_eq!(bhdr.instance_count, 2);
        assert_eq!(decoded_batch[0].entity_id_hash, 1001);
        assert_eq!(decoded_batch[1].proc_seed, 0x1122_3344_5566_7788);
    }
}
