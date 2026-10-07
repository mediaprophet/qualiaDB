//! `.10d` v2 manifold identity and typed-field manifest codec.
//!
//! The `MID2` section is integrated with the v2 outer `.10d` section table and
//! sits beside independently versioned mesh, node, topology, material, and
//! field payloads. Existing v1 containers remain readable and are never
//! rewritten unless a caller explicitly opts into the v2 section.
//!
//! The identity record keeps three concepts distinct:
//!
//! - `StableEntityId` is an opaque continuity handle supplied by a trusted,
//!   global identity authority. The authority MUST guarantee global uniqueness
//!   and collision-check issuance; this codec never derives it from content.
//! - Tensor10D coordinates are explicit identity-bearing address/state under
//!   the declared profile, domain, and coordinate convention. Their fixed
//!   order is `[q, v, w, x, y, z, t, α, μ, σ]`, matching `Tensor10D`.
//! - `IntegrityIndexDigest` is an optional external digest/index hint. It is
//!   not an entity identifier, is not computed here, and is not used to resolve
//!   identity.
//!
//! Field records are typed references into other sections of the enclosing
//! container. Built-in field kinds include electromagnetic field (EMF) as one
//! peer among scalar, vector, tensor, and spectral fields. Extension encoding
//! and decoding both require an explicit allow-list; unknown kinds fail closed.
//! The owning field schema validates kind-specific payload semantics after
//! this envelope layer checks section identity and byte ranges.
//! Encoding is little-endian, deterministic, caller-buffered, allocation-free,
//! and requires field records in ascending stable `field_id` order.

/// Exact axis order for the ten `coordinates` lanes in this section version.
pub const MANIFOLD_IDENTITY_V2_AXIS_ORDER: [&str; 10] =
    ["q", "v", "w", "x", "y", "z", "t", "α", "μ", "σ"];

pub const MANIFOLD_IDENTITY_V2_MAGIC: [u8; 4] = *b"MID2";
pub const MANIFOLD_IDENTITY_V2_VERSION: u16 = 2;
pub const MANIFOLD_IDENTITY_V2_HEADER_SIZE: usize = 112;
pub const MANIFOLD_IDENTITY_V2_FIELD_RECORD_SIZE: usize = 48;
pub const MANIFOLD_IDENTITY_V2_MAX_FIELDS: usize = 256;
pub const MANIFOLD_IDENTITY_V2_MAX_BYTES: usize = MANIFOLD_IDENTITY_V2_HEADER_SIZE
    + MANIFOLD_IDENTITY_V2_MAX_FIELDS * MANIFOLD_IDENTITY_V2_FIELD_RECORD_SIZE;

const FLAG_HAS_EXTERNAL_DIGEST: u8 = 1;

pub const FIELD_KIND_SCALAR: u16 = 0x0001;
pub const FIELD_KIND_VECTOR: u16 = 0x0002;
pub const FIELD_KIND_TENSOR: u16 = 0x0003;
pub const FIELD_KIND_SPECTRAL: u16 = 0x0004;
/// Electromagnetic field data. This is a typed field kind, not a manifold or
/// container format of its own.
pub const FIELD_KIND_ELECTROMAGNETIC: u16 = 0x0100;
pub const BUILTIN_FIELD_KINDS: [u16; 5] = [
    FIELD_KIND_SCALAR,
    FIELD_KIND_VECTOR,
    FIELD_KIND_TENSOR,
    FIELD_KIND_SPECTRAL,
    FIELD_KIND_ELECTROMAGNETIC,
];

/// Opaque stable continuity identity, globally unique under the trusted
/// issuing authority's contract. Its bytes are never interpreted as a digest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StableEntityId(pub [u8; 16]);

/// Optional externally computed digest/index aid. It is kept in a distinct
/// type and field so it cannot become the entity identity by codec convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IntegrityIndexDigest(pub [u8; 16]);

/// Manifold identity/profile declaration for one stable entity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ManifoldIdentityV2 {
    pub entity_id: StableEntityId,
    /// Stable profile registry identifier; not the entity identity.
    pub profile_id: u64,
    /// Declared manifold/domain registry identifier.
    pub domain_id: u64,
    /// Declared interpretation and unit convention registry identifier.
    pub coordinate_convention_id: u64,
    /// Values in [`MANIFOLD_IDENTITY_V2_AXIS_ORDER`].
    pub coordinates: [f32; 10],
    /// Optional digest supplied by the enclosing asset/index pipeline.
    pub external_digest: Option<IntegrityIndexDigest>,
}

impl ManifoldIdentityV2 {
    /// Canonical entity resolution uses the stable entity handle only. Edits
    /// and replay frames for the same entity retain this handle even when the
    /// manifold address/state changes. Splits or merges create new handles and
    /// record lineage in an appropriate provenance section.
    pub fn resolves_same_entity(&self, other: &Self) -> bool {
        self.entity_id == other.entity_id
    }

    /// Compare the declared manifold address/state independently of entity
    /// continuity or digest. Coordinates use canonical numeric equality;
    /// the codec normalizes either signed zero to positive zero.
    pub fn has_same_manifold_address(&self, other: &Self) -> bool {
        self.profile_id == other.profile_id
            && self.domain_id == other.domain_id
            && self.coordinate_convention_id == other.coordinate_convention_id
            && self
                .coordinates
                .iter()
                .zip(other.coordinates.iter())
                .all(|(left, right)| left == right)
    }
}

/// Typed reference to a payload in another section of the outer `.10d`
/// envelope. A zero `payload_section_id` is allowed only for an empty
/// declaration that has no attached sample payload yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypedFieldRecord {
    /// Stable field declaration identifier, unique and ascending per manifest.
    pub field_id: u64,
    /// Built-in kind or an extension kind admitted by the decoder allow-list.
    pub kind: u16,
    /// Reserved for future field semantics; must be zero in v2.
    pub flags: u16,
    /// Unit/convention registry identifier; zero means unitless or unspecified.
    pub unit_id: u64,
    /// Section identifier in the outer `.10d` envelope.
    pub payload_section_id: u32,
    /// Byte range within the referenced section.
    pub payload_offset: u64,
    pub payload_length: u64,
}

impl Default for TypedFieldRecord {
    fn default() -> Self {
        Self {
            field_id: 0,
            kind: 0,
            flags: 0,
            unit_id: 0,
            payload_section_id: 0,
            payload_offset: 0,
            payload_length: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestV2Error {
    TooShort { got: usize, need: usize },
    BadMagic,
    UnsupportedVersion(u16),
    BadHeaderSize(u16),
    BadTotalLength { declared: u32, actual: usize },
    TooManyFields { got: usize, max: usize },
    CoordinateCount(u8),
    UnknownFlags(u8),
    NonZeroAbsentDigest,
    ZeroEntityId,
    ZeroDeclarationId { field: &'static str },
    NonFiniteCoordinate { axis: usize },
    NonCanonicalCoordinate { axis: usize },
    OutputTooSmall { needed: usize, have: usize },
    InvalidFieldId,
    InvalidFieldKind,
    UnsupportedFieldKind(u16),
    InvalidFieldFlags(u16),
    FieldsNotStrictlySorted,
    NonZeroReserved { field_index: usize },
    InvalidPayloadReference { field_index: usize },
    PayloadRangeOverflow { field_index: usize },
    LengthOverflow,
}

/// Encode built-in field kinds into a caller-owned buffer. Extension kinds
/// require the explicit allow-list form so successful writes are readable by
/// the same registered schema.
pub fn encode_manifold_identity_v2(
    identity: &ManifoldIdentityV2,
    fields: &[TypedFieldRecord],
    out: &mut [u8],
) -> Result<usize, ManifestV2Error> {
    encode_manifold_identity_v2_with_extensions(identity, fields, &[], out)
}

/// Encode with an explicit extension-kind allow-list. Inputs must already use
/// canonical ascending `field_id` order; sorting is intentionally not
/// performed because this boundary must not allocate or rewrite caller data.
/// Validation and size checks finish before output is modified.
pub fn encode_manifold_identity_v2_with_extensions(
    identity: &ManifoldIdentityV2,
    fields: &[TypedFieldRecord],
    supported_extension_kinds: &[u16],
    out: &mut [u8],
) -> Result<usize, ManifestV2Error> {
    validate_identity(identity)?;
    let needed = encoded_len(fields.len())?;
    if out.len() < needed {
        return Err(ManifestV2Error::OutputTooSmall {
            needed,
            have: out.len(),
        });
    }
    validate_fields(fields, supported_extension_kinds)?;

    let mut header = [0u8; MANIFOLD_IDENTITY_V2_HEADER_SIZE];
    header[0..4].copy_from_slice(&MANIFOLD_IDENTITY_V2_MAGIC);
    put_u16(&mut header, 4, MANIFOLD_IDENTITY_V2_VERSION);
    put_u16(&mut header, 6, MANIFOLD_IDENTITY_V2_HEADER_SIZE as u16);
    put_u32(&mut header, 8, needed as u32);
    put_u16(&mut header, 12, fields.len() as u16);
    header[14] = MANIFOLD_IDENTITY_V2_AXIS_ORDER.len() as u8;
    if let Some(digest) = identity.external_digest {
        header[15] = FLAG_HAS_EXTERNAL_DIGEST;
        header[96..112].copy_from_slice(&digest.0);
    }
    header[16..32].copy_from_slice(&identity.entity_id.0);
    put_u64(&mut header, 32, identity.profile_id);
    put_u64(&mut header, 40, identity.domain_id);
    put_u64(&mut header, 48, identity.coordinate_convention_id);
    for (index, value) in identity.coordinates.iter().enumerate() {
        put_u32(
            &mut header,
            56 + index * 4,
            if *value == 0.0 { 0 } else { value.to_bits() },
        );
    }
    out[..MANIFOLD_IDENTITY_V2_HEADER_SIZE].copy_from_slice(&header);

    let mut offset = MANIFOLD_IDENTITY_V2_HEADER_SIZE;
    for field in fields {
        encode_field_record(
            field,
            &mut out[offset..offset + MANIFOLD_IDENTITY_V2_FIELD_RECORD_SIZE],
        );
        offset += MANIFOLD_IDENTITY_V2_FIELD_RECORD_SIZE;
    }
    Ok(needed)
}

/// Decode a v2 manifest without allocating. Parsed field records are copied to
/// caller storage only after the entire section has passed validation. Built-in
/// kinds are always recognized; extensions require explicit allow-list entries.
pub fn decode_manifold_identity_v2(
    bytes: &[u8],
    supported_extension_kinds: &[u16],
    out_fields: &mut [TypedFieldRecord],
) -> Result<(ManifoldIdentityV2, usize), ManifestV2Error> {
    if bytes.len() < MANIFOLD_IDENTITY_V2_HEADER_SIZE {
        return Err(ManifestV2Error::TooShort {
            got: bytes.len(),
            need: MANIFOLD_IDENTITY_V2_HEADER_SIZE,
        });
    }
    if bytes[0..4] != MANIFOLD_IDENTITY_V2_MAGIC {
        return Err(ManifestV2Error::BadMagic);
    }
    let version = get_u16(bytes, 4);
    if version != MANIFOLD_IDENTITY_V2_VERSION {
        return Err(ManifestV2Error::UnsupportedVersion(version));
    }
    let header_size = get_u16(bytes, 6);
    if header_size as usize != MANIFOLD_IDENTITY_V2_HEADER_SIZE {
        return Err(ManifestV2Error::BadHeaderSize(header_size));
    }
    let declared = get_u32(bytes, 8);
    let field_count = get_u16(bytes, 12) as usize;
    let needed = encoded_len(field_count)?;
    if declared as usize != bytes.len() || needed != bytes.len() {
        return Err(ManifestV2Error::BadTotalLength {
            declared,
            actual: bytes.len(),
        });
    }
    if bytes[14] as usize != MANIFOLD_IDENTITY_V2_AXIS_ORDER.len() {
        return Err(ManifestV2Error::CoordinateCount(bytes[14]));
    }
    let flags = bytes[15];
    if flags & !FLAG_HAS_EXTERNAL_DIGEST != 0 {
        return Err(ManifestV2Error::UnknownFlags(flags));
    }
    let external_digest = if flags & FLAG_HAS_EXTERNAL_DIGEST != 0 {
        let mut digest = [0u8; 16];
        digest.copy_from_slice(&bytes[96..112]);
        Some(IntegrityIndexDigest(digest))
    } else {
        if bytes[96..112].iter().any(|byte| *byte != 0) {
            return Err(ManifestV2Error::NonZeroAbsentDigest);
        }
        None
    };

    let mut entity_bytes = [0u8; 16];
    entity_bytes.copy_from_slice(&bytes[16..32]);
    let mut coordinates = [0.0f32; 10];
    for (index, coordinate) in coordinates.iter_mut().enumerate() {
        *coordinate = f32::from_bits(get_u32(bytes, 56 + index * 4));
        if *coordinate == 0.0 && coordinate.to_bits() != 0 {
            return Err(ManifestV2Error::NonCanonicalCoordinate { axis: index });
        }
    }
    let identity = ManifoldIdentityV2 {
        entity_id: StableEntityId(entity_bytes),
        profile_id: get_u64(bytes, 32),
        domain_id: get_u64(bytes, 40),
        coordinate_convention_id: get_u64(bytes, 48),
        coordinates,
        external_digest,
    };
    validate_identity(&identity)?;

    if out_fields.len() < field_count {
        return Err(ManifestV2Error::OutputTooSmall {
            needed: field_count,
            have: out_fields.len(),
        });
    }
    let records = &bytes[MANIFOLD_IDENTITY_V2_HEADER_SIZE..];
    validate_encoded_fields(records, field_count, supported_extension_kinds)?;
    for (index, target) in out_fields.iter_mut().take(field_count).enumerate() {
        let start = index * MANIFOLD_IDENTITY_V2_FIELD_RECORD_SIZE;
        *target =
            decode_field_record(&records[start..start + MANIFOLD_IDENTITY_V2_FIELD_RECORD_SIZE]);
    }
    Ok((identity, field_count))
}

/// Checked exact encoded length, bounded by the v2 manifest's field limit.
pub fn encoded_len(field_count: usize) -> Result<usize, ManifestV2Error> {
    if field_count > MANIFOLD_IDENTITY_V2_MAX_FIELDS {
        return Err(ManifestV2Error::TooManyFields {
            got: field_count,
            max: MANIFOLD_IDENTITY_V2_MAX_FIELDS,
        });
    }
    field_count
        .checked_mul(MANIFOLD_IDENTITY_V2_FIELD_RECORD_SIZE)
        .and_then(|n| n.checked_add(MANIFOLD_IDENTITY_V2_HEADER_SIZE))
        .filter(|n| *n <= MANIFOLD_IDENTITY_V2_MAX_BYTES && *n <= u32::MAX as usize)
        .ok_or(ManifestV2Error::LengthOverflow)
}

fn validate_identity(identity: &ManifoldIdentityV2) -> Result<(), ManifestV2Error> {
    if identity.entity_id.0.iter().all(|byte| *byte == 0) {
        return Err(ManifestV2Error::ZeroEntityId);
    }
    for (field, value) in [
        ("profile_id", identity.profile_id),
        ("domain_id", identity.domain_id),
        (
            "coordinate_convention_id",
            identity.coordinate_convention_id,
        ),
    ] {
        if value == 0 {
            return Err(ManifestV2Error::ZeroDeclarationId { field });
        }
    }
    for (axis, value) in identity.coordinates.iter().enumerate() {
        if !value.is_finite() {
            return Err(ManifestV2Error::NonFiniteCoordinate { axis });
        }
    }
    Ok(())
}

fn validate_fields(
    fields: &[TypedFieldRecord],
    supported_extension_kinds: &[u16],
) -> Result<(), ManifestV2Error> {
    if fields.len() > MANIFOLD_IDENTITY_V2_MAX_FIELDS {
        return Err(ManifestV2Error::TooManyFields {
            got: fields.len(),
            max: MANIFOLD_IDENTITY_V2_MAX_FIELDS,
        });
    }
    let mut previous = 0u64;
    for (index, field) in fields.iter().enumerate() {
        if field.field_id == 0 {
            return Err(ManifestV2Error::InvalidFieldId);
        }
        if index != 0 && field.field_id <= previous {
            return Err(ManifestV2Error::FieldsNotStrictlySorted);
        }
        previous = field.field_id;
        if field.kind == 0 {
            return Err(ManifestV2Error::InvalidFieldKind);
        }
        if !is_known_kind(field.kind, supported_extension_kinds) {
            return Err(ManifestV2Error::UnsupportedFieldKind(field.kind));
        }
        if field.flags != 0 {
            return Err(ManifestV2Error::InvalidFieldFlags(field.flags));
        }
        validate_payload_reference(field, index)?;
    }
    Ok(())
}

fn validate_encoded_fields(
    records: &[u8],
    field_count: usize,
    extensions: &[u16],
) -> Result<(), ManifestV2Error> {
    let mut previous = 0u64;
    for index in 0..field_count {
        let start = index * MANIFOLD_IDENTITY_V2_FIELD_RECORD_SIZE;
        let record = &records[start..start + MANIFOLD_IDENTITY_V2_FIELD_RECORD_SIZE];
        let field = decode_field_record(record);
        if field.field_id == 0 {
            return Err(ManifestV2Error::InvalidFieldId);
        }
        if index != 0 && field.field_id <= previous {
            return Err(ManifestV2Error::FieldsNotStrictlySorted);
        }
        previous = field.field_id;
        if field.kind == 0 {
            return Err(ManifestV2Error::InvalidFieldKind);
        }
        if !is_known_kind(field.kind, extensions) {
            return Err(ManifestV2Error::UnsupportedFieldKind(field.kind));
        }
        if field.flags != 0 {
            return Err(ManifestV2Error::InvalidFieldFlags(field.flags));
        }
        if get_u32(record, 24) != 0 || get_u32(record, 44) != 0 {
            return Err(ManifestV2Error::NonZeroReserved { field_index: index });
        }
        validate_payload_reference(&field, index)?;
    }
    Ok(())
}

fn validate_payload_reference(
    field: &TypedFieldRecord,
    index: usize,
) -> Result<(), ManifestV2Error> {
    if field.payload_section_id == 0 {
        if field.payload_offset != 0 || field.payload_length != 0 {
            return Err(ManifestV2Error::InvalidPayloadReference { field_index: index });
        }
    } else if field
        .payload_offset
        .checked_add(field.payload_length)
        .is_none()
    {
        return Err(ManifestV2Error::PayloadRangeOverflow { field_index: index });
    }
    Ok(())
}

fn is_known_kind(kind: u16, extensions: &[u16]) -> bool {
    BUILTIN_FIELD_KINDS.contains(&kind) || extensions.contains(&kind)
}

fn encode_field_record(field: &TypedFieldRecord, out: &mut [u8]) {
    put_u64(out, 0, field.field_id);
    put_u16(out, 8, field.kind);
    put_u16(out, 10, field.flags);
    put_u64(out, 12, field.unit_id);
    put_u32(out, 20, field.payload_section_id);
    put_u32(out, 24, 0);
    put_u64(out, 28, field.payload_offset);
    put_u64(out, 36, field.payload_length);
    put_u32(out, 44, 0);
}

fn decode_field_record(bytes: &[u8]) -> TypedFieldRecord {
    TypedFieldRecord {
        field_id: get_u64(bytes, 0),
        kind: get_u16(bytes, 8),
        flags: get_u16(bytes, 10),
        unit_id: get_u64(bytes, 12),
        payload_section_id: get_u32(bytes, 20),
        payload_offset: get_u64(bytes, 28),
        payload_length: get_u64(bytes, 36),
    }
}

fn put_u16(out: &mut [u8], offset: usize, value: u16) {
    out[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn put_u32(out: &mut [u8], offset: usize, value: u32) {
    out[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_u64(out: &mut [u8], offset: usize, value: u64) {
    out[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn get_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

fn get_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

fn get_u64(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
        bytes[offset + 4],
        bytes[offset + 5],
        bytes[offset + 6],
        bytes[offset + 7],
    ])
}

#[cfg(test)]
#[path = "manifold_identity_v2_tests.rs"]
mod tests;
