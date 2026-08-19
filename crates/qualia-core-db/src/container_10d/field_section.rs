//! `.10d` Field section encoder — ontology reserved (T32).
//!
//! The Field section carries field declarations (pressure, temperature,
//! EMF, etc.) into the `.10d` container so that "fields live on the
//! manifold" is a byte-inseparable claim, not just a graph convention.
//!
//! ## Design
//!
//! The section starts with a [`FieldSectionHeader`] followed by zero or
//! more [`FieldDescriptor`] entries. Each descriptor names a field and
//! records its type, unit IRI, support, and representation. The actual
//! field sample data (grid values, mesh values, etc.) is NOT stored in
//! this section — it lives in a separate data section (or out-of-band).
//! This section is the **ontology table**: it declares what fields exist
//! and their metadata, not the samples themselves.
//!
//! ## Binary layout (v0 — ontology only, no sample bytes)
//!
//! ```text
//! ┌─────────────────────────────────────────────┐
//! │ FieldSectionHeader (16 bytes)               │
//! │   magic: u32        = 0x4649454C ("FIEL")   │
//! │   version: u16      = 0                     │
//! │   field_count: u16                          │
//! │   reserved: u32     = 0                     │
//! ├─────────────────────────────────────────────┤
//! │ FieldDescriptor[0] (32 bytes each)          │
//! │   name_hash: u64     (FNV-1a of field name) │
//! │   unit_hash: u64     (FNV-1a of unit IRI)   │
//! │   type_kind: u8      (0=scalar,1=vector,...)│
//! │   support: u8        (0=region,1=point,...) │
//! │   representation: u8 (0=grid,1=mesh,...)    │
//! │   reserved: u8       = 0                    │
//! │   reserved: u32      = 0                    │
//! │   sample_offset: u64 (into data section)    │
//! ├─────────────────────────────────────────────┤
//! │ FieldDescriptor[1] ...                      │
//! │ ...                                         │
//! └─────────────────────────────────────────────┘
//! ```
//!
//! **Status: v0 — ontology only.** No sample bytes are encoded yet.
//! The `sample_offset` field is reserved for future use (always 0 in v0).
//! This is the "ontology reserved, no bytes yet" state from the plan.
//!
//! Reference: `docs/vibescript-full-impl-PLAN.md` §8.5 T32.

use bytemuck::{Pod, Zeroable};

/// Magic bytes for the Field section: "FIEL" in little-endian.
pub const FIELD_SECTION_MAGIC: u32 = 0x4649454C;

/// Current version of the Field section encoding.
pub const FIELD_SECTION_VERSION: u16 = 0;

/// Size of the FieldSectionHeader in bytes (u32 + u16 + u16 + u32 = 12).
pub const FIELD_SECTION_HEADER_SIZE: usize = 12;

/// Size of one FieldDescriptor in bytes.
pub const FIELD_DESCRIPTOR_SIZE: usize = 32;

/// Maximum fields per section (limited by u16 field_count).
pub const MAX_FIELDS_PER_SECTION: usize = 65535;

/// Field type kind — what values the field holds.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldTypeKind {
    /// Scalar field (e.g. pressure, temperature).
    Scalar = 0,
    /// Vector field (e.g. velocity, E-field).
    Vector = 1,
    /// Tensor field (e.g. stress tensor).
    Tensor = 2,
    /// Spectral field (e.g. EMF spectrum).
    Spectral = 3,
}

impl FieldTypeKind {
    pub const fn from_u8(raw: u8) -> Option<Self> {
        match raw {
            0 => Some(Self::Scalar),
            1 => Some(Self::Vector),
            2 => Some(Self::Tensor),
            3 => Some(Self::Spectral),
            _ => None,
        }
    }
}

/// Field support — where the field is defined.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldSupportKind {
    /// Region-wide field (continuous across a region).
    Region = 0,
    /// Point field (defined at specific points).
    Point = 1,
    /// Continuant-bound field (attached to a continuant).
    Continuant = 2,
    /// Stream field (time-varying stream of values).
    Stream = 3,
}

impl FieldSupportKind {
    pub const fn from_u8(raw: u8) -> Option<Self> {
        match raw {
            0 => Some(Self::Region),
            1 => Some(Self::Point),
            2 => Some(Self::Continuant),
            3 => Some(Self::Stream),
            _ => None,
        }
    }
}

/// Field representation — how the field is stored.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldRepresentationKind {
    /// Grid representation (regular grid of samples).
    Grid = 0,
    /// Mesh representation (values on mesh nodes/faces).
    Mesh = 1,
    /// Particle representation (values on particles).
    Particles = 2,
    /// Analytic representation (closed-form expression).
    Analytic = 3,
    /// Sampled representation (irregular samples).
    Sampled = 4,
}

impl FieldRepresentationKind {
    pub const fn from_u8(raw: u8) -> Option<Self> {
        match raw {
            0 => Some(Self::Grid),
            1 => Some(Self::Mesh),
            2 => Some(Self::Particles),
            3 => Some(Self::Analytic),
            4 => Some(Self::Sampled),
            _ => None,
        }
    }
}

/// Header for the Field section.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct FieldSectionHeader {
    /// Magic bytes: 0x4649454C ("FIEL").
    pub magic: u32,
    /// Encoding version (currently 0).
    pub version: u16,
    /// Number of FieldDescriptor entries following the header.
    pub field_count: u16,
    /// Reserved for future use (must be 0 in v0).
    pub reserved: u32,
}

impl Default for FieldSectionHeader {
    fn default() -> Self {
        Self {
            magic: FIELD_SECTION_MAGIC,
            version: FIELD_SECTION_VERSION,
            field_count: 0,
            reserved: 0,
        }
    }
}

/// A single field descriptor — one entry in the Field section's ontology table.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct FieldDescriptor {
    /// FNV-1a hash of the field name (e.g. "pressure_ambient").
    pub name_hash: u64,
    /// FNV-1a hash of the unit IRI (e.g. "qudt:KiloPascal").
    pub unit_hash: u64,
    /// Field type kind (see FieldTypeKind).
    pub type_kind: u8,
    /// Field support kind (see FieldSupportKind).
    pub support: u8,
    /// Field representation kind (see FieldRepresentationKind).
    pub representation: u8,
    /// Reserved (must be 0 in v0).
    pub reserved: u8,
    /// Reserved (must be 0 in v0).
    pub reserved2: u32,
    /// Offset into the data section where samples begin (0 in v0 — no samples yet).
    pub sample_offset: u64,
}

impl Default for FieldDescriptor {
    fn default() -> Self {
        Self {
            name_hash: 0,
            unit_hash: 0,
            type_kind: FieldTypeKind::Scalar as u8,
            support: FieldSupportKind::Region as u8,
            representation: FieldRepresentationKind::Grid as u8,
            reserved: 0,
            reserved2: 0,
            sample_offset: 0,
        }
    }
}

/// Errors from the Field section encoder/decoder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldSectionError {
    /// Buffer too small for the header.
    BufferTooSmallForHeader,
    /// Buffer too small for the declared number of descriptors.
    BufferTooSmallForDescriptors,
    /// Magic bytes don't match.
    BadMagic(u32),
    /// Unsupported version.
    UnsupportedVersion(u16),
    /// Invalid type kind.
    InvalidTypeKind(u8),
    /// Invalid support kind.
    InvalidSupportKind(u8),
    /// Invalid representation kind.
    InvalidRepresentationKind(u8),
    /// Too many fields.
    TooManyFields(usize),
}

impl std::fmt::Display for FieldSectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BufferTooSmallForHeader => write!(f, "buffer too small for field section header"),
            Self::BufferTooSmallForDescriptors => write!(f, "buffer too small for field descriptors"),
            Self::BadMagic(m) => write!(f, "bad field section magic: 0x{m:08X}"),
            Self::UnsupportedVersion(v) => write!(f, "unsupported field section version: {v}"),
            Self::InvalidTypeKind(k) => write!(f, "invalid field type kind: {k}"),
            Self::InvalidSupportKind(k) => write!(f, "invalid field support kind: {k}"),
            Self::InvalidRepresentationKind(k) => write!(f, "invalid field representation kind: {k}"),
            Self::TooManyFields(n) => write!(f, "too many fields: {n} (max {MAX_FIELDS_PER_SECTION})"),
        }
    }
}

impl std::error::Error for FieldSectionError {}

/// A field entry to encode into the Field section.
#[derive(Debug, Clone)]
pub struct FieldEntry {
    /// FNV-1a hash of the field name.
    pub name_hash: u64,
    /// FNV-1a hash of the unit IRI (0 if no unit).
    pub unit_hash: u64,
    /// Field type kind.
    pub type_kind: FieldTypeKind,
    /// Field support kind.
    pub support: FieldSupportKind,
    /// Field representation kind.
    pub representation: FieldRepresentationKind,
}

impl FieldEntry {
    pub fn new(
        name_hash: u64,
        unit_hash: u64,
        type_kind: FieldTypeKind,
        support: FieldSupportKind,
        representation: FieldRepresentationKind,
    ) -> Self {
        Self { name_hash, unit_hash, type_kind, support, representation }
    }

    fn to_descriptor(&self) -> FieldDescriptor {
        FieldDescriptor {
            name_hash: self.name_hash,
            unit_hash: self.unit_hash,
            type_kind: self.type_kind as u8,
            support: self.support as u8,
            representation: self.representation as u8,
            reserved: 0,
            reserved2: 0,
            sample_offset: 0, // v0: no samples yet
        }
    }
}

/// Encode a list of field entries into a `.10d` Field section.
///
/// Returns the number of bytes written, or an error if the buffer is
/// too small or the entries are invalid.
pub fn encode_field_section(
    entries: &[FieldEntry],
    out: &mut [u8],
) -> Result<usize, FieldSectionError> {
    if entries.len() > MAX_FIELDS_PER_SECTION {
        return Err(FieldSectionError::TooManyFields(entries.len()));
    }
    let needed = FIELD_SECTION_HEADER_SIZE + entries.len() * FIELD_DESCRIPTOR_SIZE;
    if out.len() < needed {
        return Err(FieldSectionError::BufferTooSmallForHeader);
    }

    // Write header.
    let header = FieldSectionHeader {
        magic: FIELD_SECTION_MAGIC,
        version: FIELD_SECTION_VERSION,
        field_count: entries.len() as u16,
        reserved: 0,
    };
    let header_bytes = bytemuck::bytes_of(&header);
    out[..FIELD_SECTION_HEADER_SIZE].copy_from_slice(header_bytes);

    // Write descriptors.
    let mut offset = FIELD_SECTION_HEADER_SIZE;
    for entry in entries {
        let desc = entry.to_descriptor();
        let desc_bytes = bytemuck::bytes_of(&desc);
        out[offset..offset + FIELD_DESCRIPTOR_SIZE].copy_from_slice(desc_bytes);
        offset += FIELD_DESCRIPTOR_SIZE;
    }

    Ok(offset)
}

/// Decode a `.10d` Field section from bytes.
///
/// Returns the header and a slice of field descriptors (zero-copy view
/// into the input bytes).
pub fn decode_field_section(
    bytes: &[u8],
) -> Result<(FieldSectionHeader, Vec<FieldDescriptor>), FieldSectionError> {
    if bytes.len() < FIELD_SECTION_HEADER_SIZE {
        return Err(FieldSectionError::BufferTooSmallForHeader);
    }

    // Copy header bytes into a stack array for alignment-safe reading.
    let mut header_buf = [0u8; FIELD_SECTION_HEADER_SIZE];
    header_buf.copy_from_slice(&bytes[..FIELD_SECTION_HEADER_SIZE]);
    let header: FieldSectionHeader = *bytemuck::from_bytes(&header_buf);

    if header.magic != FIELD_SECTION_MAGIC {
        return Err(FieldSectionError::BadMagic(header.magic));
    }
    if header.version != FIELD_SECTION_VERSION {
        return Err(FieldSectionError::UnsupportedVersion(header.version));
    }

    let count = header.field_count as usize;
    let needed = FIELD_SECTION_HEADER_SIZE + count * FIELD_DESCRIPTOR_SIZE;
    if bytes.len() < needed {
        return Err(FieldSectionError::BufferTooSmallForDescriptors);
    }

    // Read descriptors one by one (alignment-safe).
    let mut descriptors = Vec::with_capacity(count);
    let mut offset = FIELD_SECTION_HEADER_SIZE;
    for _ in 0..count {
        let mut desc_buf = [0u8; FIELD_DESCRIPTOR_SIZE];
        desc_buf.copy_from_slice(&bytes[offset..offset + FIELD_DESCRIPTOR_SIZE]);
        let desc: FieldDescriptor = *bytemuck::from_bytes(&desc_buf);
        descriptors.push(desc);
        offset += FIELD_DESCRIPTOR_SIZE;
    }

    // Validate each descriptor's enum fields.
    for desc in &descriptors {
        if FieldTypeKind::from_u8(desc.type_kind).is_none() {
            return Err(FieldSectionError::InvalidTypeKind(desc.type_kind));
        }
        if FieldSupportKind::from_u8(desc.support).is_none() {
            return Err(FieldSectionError::InvalidSupportKind(desc.support));
        }
        if FieldRepresentationKind::from_u8(desc.representation).is_none() {
            return Err(FieldSectionError::InvalidRepresentationKind(desc.representation));
        }
    }

    Ok((header, descriptors))
}

/// Compute the byte size needed to encode the given number of fields.
pub fn field_section_size(field_count: usize) -> usize {
    FIELD_SECTION_HEADER_SIZE + field_count * FIELD_DESCRIPTOR_SIZE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_field_section_round_trip() {
        let entries: Vec<FieldEntry> = vec![];
        let size = field_section_size(0);
        assert_eq!(size, FIELD_SECTION_HEADER_SIZE);
        let mut buf = vec![0u8; size];
        let written = encode_field_section(&entries, &mut buf).unwrap();
        assert_eq!(written, FIELD_SECTION_HEADER_SIZE);
        let (header, descs) = decode_field_section(&buf).unwrap();
        assert_eq!(header.magic, FIELD_SECTION_MAGIC);
        assert_eq!(header.version, FIELD_SECTION_VERSION);
        assert_eq!(header.field_count, 0);
        assert!(descs.is_empty());
    }

    #[test]
    fn single_field_round_trip() {
        let entries = vec![FieldEntry::new(
            0x1234567890ABCDEF, // name_hash
            0xFEDCBA0987654321, // unit_hash
            FieldTypeKind::Scalar,
            FieldSupportKind::Region,
            FieldRepresentationKind::Grid,
        )];
        let size = field_section_size(1);
        assert_eq!(size, FIELD_SECTION_HEADER_SIZE + FIELD_DESCRIPTOR_SIZE);
        let mut buf = vec![0u8; size];
        encode_field_section(&entries, &mut buf).unwrap();
        let (header, descs) = decode_field_section(&buf).unwrap();
        assert_eq!(header.field_count, 1);
        assert_eq!(descs.len(), 1);
        assert_eq!(descs[0].name_hash, 0x1234567890ABCDEF);
        assert_eq!(descs[0].unit_hash, 0xFEDCBA0987654321);
        assert_eq!(descs[0].type_kind, FieldTypeKind::Scalar as u8);
        assert_eq!(descs[0].support, FieldSupportKind::Region as u8);
        assert_eq!(descs[0].representation, FieldRepresentationKind::Grid as u8);
        assert_eq!(descs[0].sample_offset, 0); // v0: no samples
    }

    #[test]
    fn multiple_fields_round_trip() {
        let entries = vec![
            FieldEntry::new(100, 200, FieldTypeKind::Scalar, FieldSupportKind::Region, FieldRepresentationKind::Grid),
            FieldEntry::new(101, 201, FieldTypeKind::Vector, FieldSupportKind::Point, FieldRepresentationKind::Mesh),
            FieldEntry::new(102, 202, FieldTypeKind::Spectral, FieldSupportKind::Stream, FieldRepresentationKind::Sampled),
        ];
        let size = field_section_size(3);
        let mut buf = vec![0u8; size];
        encode_field_section(&entries, &mut buf).unwrap();
        let (header, descs) = decode_field_section(&buf).unwrap();
        assert_eq!(header.field_count, 3);
        assert_eq!(descs.len(), 3);
        assert_eq!(descs[0].name_hash, 100);
        assert_eq!(descs[1].type_kind, FieldTypeKind::Vector as u8);
        assert_eq!(descs[2].representation, FieldRepresentationKind::Sampled as u8);
    }

    #[test]
    fn bad_magic_rejected() {
        let mut buf = vec![0u8; FIELD_SECTION_HEADER_SIZE];
        let header = FieldSectionHeader {
            magic: 0xDEADBEEF,
            version: FIELD_SECTION_VERSION,
            field_count: 0,
            reserved: 0,
        };
        buf[..FIELD_SECTION_HEADER_SIZE].copy_from_slice(bytemuck::bytes_of(&header));
        let err = decode_field_section(&buf).unwrap_err();
        assert!(matches!(err, FieldSectionError::BadMagic(0xDEADBEEF)));
    }

    #[test]
    fn unsupported_version_rejected() {
        let mut buf = vec![0u8; FIELD_SECTION_HEADER_SIZE];
        let header = FieldSectionHeader {
            magic: FIELD_SECTION_MAGIC,
            version: 99,
            field_count: 0,
            reserved: 0,
        };
        buf[..FIELD_SECTION_HEADER_SIZE].copy_from_slice(bytemuck::bytes_of(&header));
        let err = decode_field_section(&buf).unwrap_err();
        assert!(matches!(err, FieldSectionError::UnsupportedVersion(99)));
    }

    #[test]
    fn buffer_too_small_rejected() {
        let buf = [0u8; 8]; // too small for header (12 bytes)
        let err = decode_field_section(&buf).unwrap_err();
        assert!(matches!(err, FieldSectionError::BufferTooSmallForHeader));
    }

    #[test]
    fn buffer_too_small_for_descriptors_rejected() {
        let mut buf = vec![0u8; FIELD_SECTION_HEADER_SIZE + 16]; // room for header + half a descriptor
        let header = FieldSectionHeader {
            magic: FIELD_SECTION_MAGIC,
            version: FIELD_SECTION_VERSION,
            field_count: 1, // claims 1 descriptor but buffer is too small
            reserved: 0,
        };
        buf[..FIELD_SECTION_HEADER_SIZE].copy_from_slice(bytemuck::bytes_of(&header));
        let err = decode_field_section(&buf).unwrap_err();
        assert!(matches!(err, FieldSectionError::BufferTooSmallForDescriptors));
    }

    #[test]
    fn invalid_type_kind_rejected() {
        let size = field_section_size(1);
        let mut buf = vec![0u8; size];
        let entries = vec![FieldEntry::new(1, 2, FieldTypeKind::Scalar, FieldSupportKind::Region, FieldRepresentationKind::Grid)];
        encode_field_section(&entries, &mut buf).unwrap();
        // Corrupt the type_kind byte. Within FieldDescriptor, type_kind is at
        // offset 16 (after name_hash:u64 + unit_hash:u64). In the buffer, that's
        // FIELD_SECTION_HEADER_SIZE + 16.
        buf[FIELD_SECTION_HEADER_SIZE + 16] = 99;
        let err = decode_field_section(&buf).unwrap_err();
        assert!(matches!(err, FieldSectionError::InvalidTypeKind(99)));
    }

    #[test]
    fn field_section_size_calculation() {
        assert_eq!(field_section_size(0), 12);
        assert_eq!(field_section_size(1), 44);
        assert_eq!(field_section_size(10), 12 + 320);
    }

    #[test]
    fn header_default_is_valid() {
        let h = FieldSectionHeader::default();
        assert_eq!(h.magic, FIELD_SECTION_MAGIC);
        assert_eq!(h.version, FIELD_SECTION_VERSION);
        assert_eq!(h.field_count, 0);
        assert_eq!(h.reserved, 0);
    }

    #[test]
    fn descriptor_default_is_valid() {
        let d = FieldDescriptor::default();
        assert_eq!(d.type_kind, FieldTypeKind::Scalar as u8);
        assert_eq!(d.support, FieldSupportKind::Region as u8);
        assert_eq!(d.representation, FieldRepresentationKind::Grid as u8);
        assert_eq!(d.sample_offset, 0);
    }
}
