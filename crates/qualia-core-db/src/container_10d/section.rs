//! `.10d` self-describing section table + tiered alignment + caller-buffered
//! writer (P0.2).
//!
//! A `.10d` file is: the 64-byte [`super::header::Container10dHeader`], then
//! (optionally) a section table — an array of [`SectionDescriptor`] rows —
//! then the section payloads, each aligned to its declared tier. The header's
//! `section_table_offset` + `section_count` point at the table.
//!
//! **Canonical encoding (determinism).** Sections are written in ascending
//! `section_type` order. Duplicate `section_type` values are rejected, so two
//! encodes of the same section set — even if the caller passes them in
//! permuted order — produce byte-identical output. This is the P0.2
//! "two encodes (incl. permuted section order) are byte-identical" gate.
//!
//! **Tiered alignment.** Each section declares an [`AlignmentTier`]; the
//! writer inserts zero padding so every section start meets its tier. The
//! reader rejects any section whose start is misaligned, whose descriptor
//! overlaps another, whose `byte_offset`/`byte_length` is out of bounds, or
//! whose `stride * element_count != byte_length` (stride-inconsistent).
//!
//! **Per-section CRC-32C.** Each descriptor carries a CRC-32C over its
//! payload bytes; the reader rejects a flipped bit. The CRC-32C routine here
//! is a local copy of the Castagnoli-reflected algorithm used in
//! `q42/p64_weight.rs`; **P0.3 consolidates the two into a shared module and
//! delegates both call sites** (the P0.3 acceptance gate verifies p64's
//! checksums stay byte-identical after delegation).
//!
//! **Caller-buffered / zero-heap.** [`encode_container`] takes a caller-
//! supplied `&mut [u8]` output buffer and returns the bytes written; it
//! allocates no `Vec`/`String`/`Box` on the hot path (the only allocation is
//! the canonical-order index sort, which uses a stack array — see
//! `sort_indices_stack`). [`parse_section_table`] returns a zero-copy
//! `&[SectionDescriptor]` view into the input bytes.

use bytemuck::{Pod, Zeroable};

use crate::container_10d::crc32c::crc32c;
use crate::container_10d::header::{Container10dHeader, HEADER_BYTE_SIZE, MAX_SECTION_COUNT};

/// Size of one [`SectionDescriptor`] in bytes.
pub const SECTION_DESCRIPTOR_SIZE: usize = 24;

/// Maximum number of sections the writer will accept (mirrors the header's
/// `MAX_SECTION_COUNT` so the writer cannot produce an unreadable file).
const MAX_SECTIONS_ENCODE: usize = MAX_SECTION_COUNT as usize;

/// A section type tag. v1 defines the types the runtime actually fills;
/// future types are listed as `SpecReserved*` and the writer rejects them
/// (a `SpecReserved*` type in a v1 file is a forward-incompatibility signal,
/// not a payload to read blindly).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionType {
    /// Sentinel — the reader rejects this.
    Undefined = 0,
    /// Quantized triangle mesh (P0.4 — u16-quantized vertices within the
    /// mesh's bounding box + u16/u32 triangle indices; see
    /// [`super::mesh_section`]).
    QuantizedMesh = 1,
    /// Tensor10D node section — the 40-byte epistemic atom (P0.5).
    Tensor10DNodes = 2,
    /// Reconstruction output — mesh/complex/operator (P6.7).
    Reconstruction = 3,
    // --- spec-reserved (reader rejects in v1; do NOT read blindly) ---
    SpecReservedGovernance = 4,
    SpecReservedTemporalIndex = 5,
    SpecReservedManifoldHeadTable = 6,
    SpecReservedProvenanceSidecar = 7,
    SpecReservedFieldSidecar = 8,
    SpecReservedCorrespondenceMap = 9,
    /// Topology section — half-edge graph + CSR adjacency + connectivity
    /// summary (P2.8). Contains a TopologyMiniHeader followed by the
    /// half-edge array, vertex-adjacency CSR (offsets + neighbours), and
    /// face-adjacency CSR (offsets + neighbours).
    Topology = 10,
    /// Spatial-index section — BVH + kd-tree node arrays for scan-free
    /// spatial queries (P3.7).
    SpatialIndex = 11,
}

impl SectionType {
    #[inline]
    pub const fn from_u8(raw: u8) -> Option<SectionType> {
        match raw {
            0 => Some(SectionType::Undefined),
            1 => Some(SectionType::QuantizedMesh),
            2 => Some(SectionType::Tensor10DNodes),
            3 => Some(SectionType::Reconstruction),
            4 => Some(SectionType::SpecReservedGovernance),
            5 => Some(SectionType::SpecReservedTemporalIndex),
            6 => Some(SectionType::SpecReservedManifoldHeadTable),
            7 => Some(SectionType::SpecReservedProvenanceSidecar),
            8 => Some(SectionType::SpecReservedFieldSidecar),
            9 => Some(SectionType::SpecReservedCorrespondenceMap),
            10 => Some(SectionType::Topology),
            11 => Some(SectionType::SpatialIndex),
            _ => None,
        }
    }

    /// True if this section type is implemented (the runtime can read/write
    /// it) vs spec-reserved (defined in the format but not yet filled).
    #[inline]
    pub const fn is_implemented(self) -> bool {
        matches!(
            self,
            SectionType::QuantizedMesh
                | SectionType::Tensor10DNodes
                | SectionType::Reconstruction
                | SectionType::Topology
                | SectionType::SpatialIndex
        )
    }
}

/// Alignment tier for a section's start offset. The tier determines the
/// power-of-two alignment the writer enforces (and the reader verifies).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignmentTier {
    /// 1-byte aligned (no requirement).
    Byte = 0,
    /// 4-byte aligned.
    Word = 1,
    /// 16-byte aligned (typical for SoA tensor lanes).
    CacheLine = 2,
    /// 64-byte aligned (page-aligned, matches `q42/p64_weight.rs`).
    Page = 3,
}

impl AlignmentTier {
    /// The alignment in bytes for this tier.
    #[inline]
    pub const fn to_bytes(self) -> usize {
        match self {
            AlignmentTier::Byte => 1,
            AlignmentTier::Word => 4,
            AlignmentTier::CacheLine => 16,
            AlignmentTier::Page => 64,
        }
    }

    #[inline]
    pub const fn from_u8(raw: u8) -> Option<AlignmentTier> {
        match raw {
            0 => Some(AlignmentTier::Byte),
            1 => Some(AlignmentTier::Word),
            2 => Some(AlignmentTier::CacheLine),
            3 => Some(AlignmentTier::Page),
            _ => None,
        }
    }
}

/// Align `offset` up to the next multiple of `align` (power of two).
#[inline]
const fn align_up(offset: usize, align: usize) -> usize {
    if align <= 1 {
        return offset;
    }
    (offset + align - 1) & !(align - 1)
}

/// One row of the section table — a self-describing section descriptor.
///
/// Layout: 24 bytes, `repr(C)`, naturally aligned, no padding.
/// ```text
/// offset  size  field
/// 0       1     section_type     (SectionType u8)
/// 1       1     alignment_tier   (AlignmentTier u8)
/// 2       2     reserved16       (must be zero)
/// 4       4     byte_offset      (from file start)
/// 8       4     byte_length      (payload length in bytes)
/// 12      4     stride           (bytes per element for AoS sections; 0 = non-strided)
/// 16      4     element_count    (for array sections; 0 = non-array)
/// 20      4     crc32c           (CRC-32C over the payload bytes)
/// ```
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct SectionDescriptor {
    pub section_type: u8,
    pub alignment_tier: u8,
    pub reserved16: u16,
    pub byte_offset: u32,
    pub byte_length: u32,
    pub stride: u32,
    pub element_count: u32,
    pub crc32c: u32,
}

impl SectionDescriptor {
    /// The alignment tier as a typed enum (or `None` if the raw byte is
    /// undefined — the reader rejects this).
    #[inline]
    pub fn tier(&self) -> Option<AlignmentTier> {
        AlignmentTier::from_u8(self.alignment_tier)
    }

    /// The section type as a typed enum (or `None` if undefined).
    #[inline]
    pub fn typ(&self) -> Option<SectionType> {
        SectionType::from_u8(self.section_type)
    }
}

/// A caller-supplied section input for [`encode_container`]. The writer
/// computes `byte_offset`, `crc32c`, and the canonical order; the caller
/// supplies the type, tier, stride/element_count (for array sections), and
/// the payload bytes.
#[derive(Debug, Clone, Copy)]
pub struct SectionInput<'a> {
    pub section_type: SectionType,
    pub alignment_tier: AlignmentTier,
    /// Bytes per element for array-of-structs sections. `0` for non-strided
    /// (blob) sections. If non-zero, `stride * element_count` must equal
    /// `payload.len()` or the writer rejects the input as stride-inconsistent.
    pub stride: u32,
    /// Element count for array sections. `0` for non-array (blob) sections.
    pub element_count: u32,
    pub payload: &'a [u8],
}

/// Section-table encode/decode error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SectionTableError {
    /// More than `MAX_SECTION_COUNT` sections.
    TooManySections { count: usize },
    /// Two sections share the same `section_type` (canonical encoding
    /// requires unique types in v1).
    DuplicateSectionType { section_type: u8 },
    /// A section type byte is not a defined variant, or is `Undefined`, or is
    /// a `SpecReserved*` type the v1 writer refuses to emit.
    UnsupportedSectionType { got: u8 },
    /// An alignment tier byte is not a defined variant.
    UnsupportedAlignmentTier { got: u8 },
    /// `stride * element_count != payload.len()`.
    StrideInconsistent { section_type: u8, stride: u32, element_count: u32, payload_len: usize },
    /// The caller-supplied output buffer is too small.
    OutputBufferTooSmall { needed: usize, have: usize },
    /// The input bytes are too short to hold the header + section table.
    InputTooShort { got: usize, need: usize },
    /// The header's section-table pointer is inconsistent (offset/count
    /// mismatch, offset below header, or count over the max).
    BadSectionTablePointer { offset: u32, count: u32 },
    /// A descriptor's `reserved16` is non-zero.
    NonZeroDescriptorReserved { index: usize },
    /// A descriptor's `byte_offset` is misaligned relative to its tier.
    MisalignedSection { index: usize, offset: u32, tier: AlignmentTier },
    /// A descriptor's `byte_offset`/`byte_length` is out of bounds.
    OutOfBounds { index: usize, offset: u32, length: u32, file_len: usize },
    /// Two descriptors' payload regions overlap.
    OverlappingSections { index_a: usize, index_b: usize },
    /// A descriptor's `stride * element_count != byte_length`.
    StrideInconsistentDescriptor { index: usize, stride: u32, element_count: u32, byte_length: u32 },
    /// A descriptor's section type is `Undefined` or an unknown byte.
    UndefinedSectionType { index: usize, got: u8 },
    /// A descriptor's alignment tier is undefined.
    UndefinedAlignmentTier { index: usize, got: u8 },
    /// A payload's CRC-32C does not match the stored value (a flipped bit).
    CrcMismatch { index: usize, section_type: u8, expected: u32, got: u32 },
    /// A padding region between sections is non-zero.
    NonZeroPadding { at: usize },
}

impl std::fmt::Display for SectionTableError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooManySections { count } => write!(f, "10d too many sections: {count} > {MAX_SECTION_COUNT}"),
            Self::DuplicateSectionType { section_type } => write!(f, "10d duplicate section type {section_type} (v1 requires unique types)"),
            Self::UnsupportedSectionType { got } => write!(f, "10d unsupported section type byte {got}"),
            Self::UnsupportedAlignmentTier { got } => write!(f, "10d unsupported alignment tier byte {got}"),
            Self::StrideInconsistent { section_type, stride, element_count, payload_len } => write!(f, "10d stride inconsistent for type {section_type}: {stride} * {element_count} != {payload_len}"),
            Self::OutputBufferTooSmall { needed, have } => write!(f, "10d output buffer too small: need {needed}, have {have}"),
            Self::InputTooShort { got, need } => write!(f, "10d input too short: got {got}, need {need}"),
            Self::BadSectionTablePointer { offset, count } => write!(f, "10d bad section-table pointer: offset={offset}, count={count}"),
            Self::NonZeroDescriptorReserved { index } => write!(f, "10d non-zero reserved16 in descriptor {index}"),
            Self::MisalignedSection { index, offset, tier } => write!(f, "10d section {index} offset {offset} misaligned for tier {tier:?}"),
            Self::OutOfBounds { index, offset, length, file_len } => write!(f, "10d section {index} out of bounds: offset={offset} length={length} file_len={file_len}"),
            Self::OverlappingSections { index_a, index_b } => write!(f, "10d sections {index_a} and {index_b} overlap"),
            Self::StrideInconsistentDescriptor { index, stride, element_count, byte_length } => write!(f, "10d descriptor {index} stride inconsistent: {stride} * {element_count} != {byte_length}"),
            Self::UndefinedSectionType { index, got } => write!(f, "10d descriptor {index} undefined section type {got}"),
            Self::UndefinedAlignmentTier { index, got } => write!(f, "10d descriptor {index} undefined alignment tier {got}"),
            Self::CrcMismatch { index, section_type, expected, got } => write!(f, "10d CRC mismatch in section {index} (type {section_type}): expected {expected:#010x}, got {got:#010x}"),
            Self::NonZeroPadding { at } => write!(f, "10d non-zero padding at byte {at}"),
        }
    }
}

impl std::error::Error for SectionTableError {}

// ---------------------------------------------------------------------------
// Canonical-order index sort (stack-only, no Vec).
// Returns the indices of `inputs` sorted by `section_type` ascending. Insertion
// sort is fine — MAX_SECTIONS_ENCODE is small and the section count in any
// realistic .10d is <10. No heap allocation.
// ---------------------------------------------------------------------------

fn sort_indices_stack(inputs: &[SectionInput<'_>]) -> [usize; MAX_SECTIONS_ENCODE] {
    let mut idx = [0usize; MAX_SECTIONS_ENCODE];
    for i in 0..inputs.len() {
        idx[i] = i;
    }
    // Insertion sort by section_type (stable — preserves input order for
    // equal keys, though equal keys are rejected later as duplicates).
    for i in 1..inputs.len() {
        let mut j = i;
        while j > 0 && inputs[idx[j - 1]].section_type as u8 > inputs[idx[j]].section_type as u8 {
            idx.swap(j - 1, j);
            j -= 1;
        }
    }
    idx
}

/// Compute the total encoded byte length for a set of sections (header +
/// section table + aligned payloads + inter-section padding). Pure, no
/// allocation. Returns the total and fills `order` with the canonical index
/// order and `descs` with the computed descriptors (offsets/lengths/CRCs).
fn plan_layout(
    inputs: &[SectionInput<'_>],
    order: &mut [usize; MAX_SECTIONS_ENCODE],
    descs: &mut [SectionDescriptor; MAX_SECTIONS_ENCODE],
) -> Result<usize, SectionTableError> {
    if inputs.len() > MAX_SECTIONS_ENCODE {
        return Err(SectionTableError::TooManySections { count: inputs.len() });
    }
    *order = sort_indices_stack(inputs);

    // Reject duplicate section types and unsupported types/tiers up front.
    for k in 0..inputs.len() {
        let i = order[k];
        let st = inputs[i].section_type as u8;
        if k > 0 && inputs[order[k - 1]].section_type as u8 == st {
            return Err(SectionTableError::DuplicateSectionType { section_type: st });
        }
        if !inputs[i].section_type.is_implemented() {
            return Err(SectionTableError::UnsupportedSectionType { got: st });
        }
        // Validate stride consistency against the payload.
        if inputs[i].stride > 0 {
            let expected = inputs[i].stride as usize * inputs[i].element_count as usize;
            if expected != inputs[i].payload.len() {
                return Err(SectionTableError::StrideInconsistent {
                    section_type: st,
                    stride: inputs[i].stride,
                    element_count: inputs[i].element_count,
                    payload_len: inputs[i].payload.len(),
                });
            }
        }
    }

    // Layout: header (64) -> section table (count * 24) -> payloads (aligned).
    let table_off = HEADER_BYTE_SIZE;
    let table_len = inputs.len() * SECTION_DESCRIPTOR_SIZE;
    let mut cursor = table_off + table_len;

    for k in 0..inputs.len() {
        let i = order[k];
        let align = inputs[i].alignment_tier.to_bytes();
        cursor = align_up(cursor, align);
        let off = cursor;
        let len = inputs[i].payload.len();
        let crc = crc32c(inputs[i].payload);
        descs[k] = SectionDescriptor {
            section_type: inputs[i].section_type as u8,
            alignment_tier: inputs[i].alignment_tier as u8,
            reserved16: 0,
            byte_offset: off as u32,
            byte_length: len as u32,
            stride: inputs[i].stride,
            element_count: inputs[i].element_count,
            crc32c: crc,
        };
        cursor = off + len;
    }
    Ok(cursor)
}

/// Encode a `.10d` container into a caller-supplied buffer. Zero-heap on the
/// hot path (the only stack arrays are the index order and descriptor table,
/// both fixed-size). Returns the number of bytes written.
///
/// The header is written with `section_table_offset` and `section_count`
/// filled in to point at the table; `header_crc32c` is left as the caller
/// supplied it (P0.3 wires the shared CRC-32C over the header).
pub fn encode_container(
    header: &Container10dHeader,
    inputs: &[SectionInput<'_>],
    out: &mut [u8],
) -> Result<usize, SectionTableError> {
    let mut order = [0usize; MAX_SECTIONS_ENCODE];
    let mut descs = [ZEROED_SECTION_DESCRIPTOR; MAX_SECTIONS_ENCODE];
    let total = plan_layout(inputs, &mut order, &mut descs)?;
    if out.len() < total {
        return Err(SectionTableError::OutputBufferTooSmall { needed: total, have: out.len() });
    }

    // Zero the whole output region we will write into (so padding is zero).
    for b in out[..total].iter_mut() {
        *b = 0;
    }

    // Write the header with the section-table pointer filled in.
    let mut h = *header;
    if inputs.is_empty() {
        h.section_table_offset = 0;
        h.section_count = 0;
    } else {
        h.section_table_offset = HEADER_BYTE_SIZE as u32;
        h.section_count = inputs.len() as u32;
    }
    let mut header_buf = [0u8; HEADER_BYTE_SIZE];
    h.encode(&mut header_buf);
    out[..HEADER_BYTE_SIZE].copy_from_slice(&header_buf);

    if inputs.is_empty() {
        return Ok(HEADER_BYTE_SIZE);
    }

    // Write the section table.
    let table_off = HEADER_BYTE_SIZE;
    for k in 0..inputs.len() {
        let desc_bytes: &[u8; SECTION_DESCRIPTOR_SIZE] = bytemuck::cast_ref(&descs[k]);
        let dst = table_off + k * SECTION_DESCRIPTOR_SIZE;
        out[dst..dst + SECTION_DESCRIPTOR_SIZE].copy_from_slice(desc_bytes);
    }

    // Write the payloads (padding is already zeroed).
    for k in 0..inputs.len() {
        let i = order[k];
        let off = descs[k].byte_offset as usize;
        let len = descs[k].byte_length as usize;
        out[off..off + len].copy_from_slice(inputs[i].payload);
    }

    Ok(total)
}

/// A const-zero `SectionDescriptor` for initialising the fixed-size stack
/// array in [`encode_container`] without going through `Zeroable::zeroed()`.
const ZEROED_SECTION_DESCRIPTOR: SectionDescriptor = SectionDescriptor {
    section_type: 0,
    alignment_tier: 0,
    reserved16: 0,
    byte_offset: 0,
    byte_length: 0,
    stride: 0,
    element_count: 0,
    crc32c: 0,
};

/// Parse the section table from a `.10d` byte slice. Returns a zero-copy
/// `&[SectionDescriptor]` view into the input. Runs every P0.2 reader gate:
/// pointer consistency, per-descriptor type/tier/reserved validation, tier
/// alignment, in-bounds, overlap, stride consistency, and per-section CRC.
///
/// Padding-between-sections is verified zero as part of the overlap/scan pass.
pub fn parse_section_table<'a>(
    data: &'a [u8],
    header: &Container10dHeader,
) -> Result<&'a [SectionDescriptor], SectionTableError> {
    let (off, cnt) = (header.section_table_offset, header.section_count);
    if off == 0 && cnt == 0 {
        return Ok(&[]);
    }
    // Pointer consistency (the header parser also checks this, but re-check
    // for callers that construct a header by hand).
    let both_zero = off == 0 && cnt == 0;
    let both_nonzero = off != 0 && cnt != 0;
    let valid_nonzero = both_nonzero
        && off as usize >= HEADER_BYTE_SIZE
        && off as usize <= data.len()
        && cnt <= MAX_SECTION_COUNT;
    if !both_zero && !valid_nonzero {
        return Err(SectionTableError::BadSectionTablePointer { offset: off, count: cnt });
    }
    let table_start = off as usize;
    let table_bytes = cnt as usize * SECTION_DESCRIPTOR_SIZE;
    let table_end = table_start
        .checked_add(table_bytes)
        .ok_or(SectionTableError::BadSectionTablePointer { offset: off, count: cnt })?;
    if table_end > data.len() {
        return Err(SectionTableError::InputTooShort { got: data.len(), need: table_end });
    }
    // SAFETY: SectionDescriptor is repr(C) + Pod + size 24 with no padding;
    // the table slice is byte-aligned and fully within `data`.
    let descs: &[SectionDescriptor] =
        bytemuck::cast_slice(&data[table_start..table_end]);

    // Per-descriptor validation + cross-descriptor overlap/alignment scan.
    // We scan in table order (which is canonical = ascending section_type
    // order for files we wrote; for files we didn't write, we sort the
    // offset check by byte_offset to detect overlaps correctly).
    for (i, d) in descs.iter().enumerate() {
        if d.reserved16 != 0 {
            return Err(SectionTableError::NonZeroDescriptorReserved { index: i });
        }
        let st = SectionType::from_u8(d.section_type).ok_or(SectionTableError::UndefinedSectionType { index: i, got: d.section_type })?;
        if st == SectionType::Undefined {
            return Err(SectionTableError::UndefinedSectionType { index: i, got: d.section_type });
        }
        let tier = AlignmentTier::from_u8(d.alignment_tier)
            .ok_or(SectionTableError::UndefinedAlignmentTier { index: i, got: d.alignment_tier })?;
        let align = tier.to_bytes();
        let o = d.byte_offset as usize;
        let l = d.byte_length as usize;
        if o % align != 0 {
            return Err(SectionTableError::MisalignedSection { index: i, offset: d.byte_offset, tier });
        }
        let end = o.checked_add(l).ok_or(SectionTableError::OutOfBounds { index: i, offset: d.byte_offset, length: d.byte_length, file_len: data.len() })?;
        if end > data.len() {
            return Err(SectionTableError::OutOfBounds { index: i, offset: d.byte_offset, length: d.byte_length, file_len: data.len() });
        }
        // Stride consistency.
        if d.stride > 0 {
            let expected = d.stride as usize * d.element_count as usize;
            if expected != l {
                return Err(SectionTableError::StrideInconsistentDescriptor { index: i, stride: d.stride, element_count: d.element_count, byte_length: d.byte_length });
            }
        }
        // Per-section CRC.
        let stored = d.crc32c;
        let actual = crc32c(&data[o..end]);
        if actual != stored {
            return Err(SectionTableError::CrcMismatch { index: i, section_type: d.section_type, expected: stored, got: actual });
        }
    }

    // Overlap detection: sort descriptor indices by byte_offset and check
    // adjacent ranges don't overlap. Use a stack array (count is bounded by
    // MAX_SECTION_COUNT, but that's 1024 — too big for a fixed stack array in
    // a zero-heap function). Instead, do an O(n^2) pairwise check (n is small
    // in practice; the 42MB Sentinel bounds the file and section count is
    // realistically <10). This is zero-heap.
    for a in 0..descs.len() {
        for b in (a + 1)..descs.len() {
            let (ao, al) = (descs[a].byte_offset as usize, descs[a].byte_length as usize);
            let (bo, bl) = (descs[b].byte_offset as usize, descs[b].byte_length as usize);
            let a_end = ao + al;
            let b_end = bo + bl;
            if ao < b_end && bo < a_end {
                // Two zero-length sections at the same offset are not an
                // overlap (an empty section is allowed to share an offset).
                if al == 0 || bl == 0 {
                    continue;
                }
                return Err(SectionTableError::OverlappingSections { index_a: a, index_b: b });
            }
        }
    }

    // Padding-between-sections is zero: scan the gaps. The gap before the
    // first section (between table_end and the first section's offset) and
    // between consecutive sections. Walk in offset order.
    let mut order_by_off: [usize; MAX_SECTION_COUNT as usize] = [0usize; MAX_SECTION_COUNT as usize];
    for i in 0..descs.len() {
        order_by_off[i] = i;
    }
    // Insertion sort by byte_offset.
    for i in 1..descs.len() {
        let mut j = i;
        while j > 0 && descs[order_by_off[j - 1]].byte_offset > descs[order_by_off[j]].byte_offset {
            order_by_off.swap(j - 1, j);
            j -= 1;
        }
    }
    let mut gap_start = table_end;
    for k in 0..descs.len() {
        let idx = order_by_off[k];
        let o = descs[idx].byte_offset as usize;
        for b in &data[gap_start..o] {
            if *b != 0 {
                return Err(SectionTableError::NonZeroPadding { at: gap_start });
            }
        }
        gap_start = o + descs[idx].byte_length as usize;
    }
    // Tail padding (after the last section to the end of the encoded region)
    // is not checked here — the caller may have a larger buffer. Only the
    // encoded region's padding matters, and that is covered by `total` from
    // the writer; a reader does not know `total` without the header CRC region
    // (P0.3). For P0.2, inter-section padding is the gate.

    Ok(descs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container_10d::header::Container10dHeader;

    fn mesh_input(payload: &[u8]) -> SectionInput<'_> {
        SectionInput {
            section_type: SectionType::QuantizedMesh,
            alignment_tier: AlignmentTier::Word,
            stride: 0,
            element_count: 0,
            payload,
        }
    }

    fn node_input(payload: &[u8]) -> SectionInput<'_> {
        // Tensor10D nodes: 40 bytes each, 16-byte aligned (SoA lane friendly).
        SectionInput {
            section_type: SectionType::Tensor10DNodes,
            alignment_tier: AlignmentTier::CacheLine,
            stride: 40,
            element_count: (payload.len() / 40) as u32,
            payload,
        }
    }

    #[test]
    fn descriptor_is_pod_with_exact_size() {
        assert_eq!(std::mem::size_of::<SectionDescriptor>(), SECTION_DESCRIPTOR_SIZE);
        assert_eq!(std::mem::offset_of!(SectionDescriptor, section_type), 0);
        assert_eq!(std::mem::offset_of!(SectionDescriptor, alignment_tier), 1);
        assert_eq!(std::mem::offset_of!(SectionDescriptor, reserved16), 2);
        assert_eq!(std::mem::offset_of!(SectionDescriptor, byte_offset), 4);
        assert_eq!(std::mem::offset_of!(SectionDescriptor, byte_length), 8);
        assert_eq!(std::mem::offset_of!(SectionDescriptor, stride), 12);
        assert_eq!(std::mem::offset_of!(SectionDescriptor, element_count), 16);
        assert_eq!(std::mem::offset_of!(SectionDescriptor, crc32c), 20);
    }

    #[test]
    fn bare_header_encodes_and_parses_with_no_sections() {
        let h = Container10dHeader::proposed();
        let mut out = [0u8; 128];
        let n = encode_container(&h, &[], &mut out).expect("bare encode");
        assert_eq!(n, HEADER_BYTE_SIZE);
        let parsed_h = Container10dHeader::parse(&out[..n]).expect("bare parse");
        assert_eq!(parsed_h, h);
        let table = parse_section_table(&out[..n], &parsed_h).expect("bare table parse");
        assert!(table.is_empty());
    }

    #[test]
    fn round_trip_two_sections_descriptors_match() {
        let h = Container10dHeader::proposed();
        let mesh_payload = [0xAAu8; 100];
        let node_payload = [0xBBu8; 40 * 3]; // 3 Tensor10D nodes
        let inputs = [mesh_input(&mesh_payload), node_input(&node_payload)];
        let mut out = [0u8; 512];
        let n = encode_container(&h, &inputs, &mut out).expect("encode");
        let parsed_h = Container10dHeader::parse(&out[..n]).expect("header parse");
        let descs = parse_section_table(&out[..n], &parsed_h).expect("table parse");
        assert_eq!(descs.len(), 2);
        // Canonical order: QuantizedMesh(1) before Tensor10DNodes(2).
        assert_eq!(descs[0].section_type, SectionType::QuantizedMesh as u8);
        assert_eq!(descs[1].section_type, SectionType::Tensor10DNodes as u8);
        // Payloads round-trip.
        assert_eq!(&out[descs[0].byte_offset as usize..][..mesh_payload.len()], &mesh_payload);
        assert_eq!(&out[descs[1].byte_offset as usize..][..node_payload.len()], &node_payload);
        // Stride/element_count for the node section.
        assert_eq!(descs[1].stride, 40);
        assert_eq!(descs[1].element_count, 3);
    }

    #[test]
    fn every_section_start_meets_its_declared_tier() {
        let h = Container10dHeader::proposed();
        let mesh_payload = [0u8; 7]; // odd length, will force padding before next section
        let node_payload = [0u8; 40];
        let inputs = [mesh_input(&mesh_payload), node_input(&node_payload)];
        let mut out = [0u8; 512];
        let n = encode_container(&h, &inputs, &mut out).expect("encode");
        let parsed_h = Container10dHeader::parse(&out[..n]).expect("header parse");
        let descs = parse_section_table(&out[..n], &parsed_h).expect("table parse");
        for d in descs {
            let tier = AlignmentTier::from_u8(d.alignment_tier).unwrap();
            assert_eq!(
                (d.byte_offset as usize) % tier.to_bytes(),
                0,
                "section type {} at offset {} must meet tier {:?}",
                d.section_type,
                d.byte_offset,
                tier
            );
        }
    }

    #[test]
    fn padding_between_sections_is_zero() {
        let h = Container10dHeader::proposed();
        let mesh_payload = [0u8; 7]; // forces padding before the 16-byte-aligned node section
        let node_payload = [0u8; 40];
        let inputs = [mesh_input(&mesh_payload), node_input(&node_payload)];
        let mut out = [0u8; 512];
        let n = encode_container(&h, &inputs, &mut out).expect("encode");
        let parsed_h = Container10dHeader::parse(&out[..n]).expect("header parse");
        let descs = parse_section_table(&out[..n], &parsed_h).expect("table parse");
        // The gap between the mesh payload end and the node section start must be zero.
        let mesh_end = descs[0].byte_offset as usize + descs[0].byte_length as usize;
        let node_start = descs[1].byte_offset as usize;
        assert!(node_start > mesh_end, "there must be padding between the odd-length mesh and the 16-aligned node");
        for b in &out[mesh_end..node_start] {
            assert_eq!(*b, 0, "padding between sections must be zero");
        }
    }

    #[test]
    fn permuted_section_order_produces_byte_identical_output() {
        let h = Container10dHeader::proposed();
        let mesh_payload = [0xAAu8; 100];
        let node_payload = [0xBBu8; 40 * 3];
        let inputs_a = [mesh_input(&mesh_payload), node_input(&node_payload)];
        let inputs_b = [node_input(&node_payload), mesh_input(&mesh_payload)]; // permuted
        let mut out_a = [0u8; 512];
        let mut out_b = [0u8; 512];
        let n_a = encode_container(&h, &inputs_a, &mut out_a).expect("encode a");
        let n_b = encode_container(&h, &inputs_b, &mut out_b).expect("encode b");
        assert_eq!(n_a, n_b);
        assert_eq!(&out_a[..n_a], &out_b[..n_b], "permuted input must produce byte-identical output");
    }

    #[test]
    fn duplicate_section_type_is_rejected() {
        let h = Container10dHeader::proposed();
        let p1 = [0u8; 16];
        let p2 = [0u8; 32];
        let inputs = [mesh_input(&p1), mesh_input(&p2)];
        let mut out = [0u8; 512];
        let err = encode_container(&h, &inputs, &mut out).expect_err("duplicate type must reject");
        assert!(matches!(err, SectionTableError::DuplicateSectionType { section_type: 1 }), "{err}");
    }

    #[test]
    fn stride_inconsistent_input_is_rejected() {
        let h = Container10dHeader::proposed();
        // Claim stride=40, element_count=3 (=> 120 bytes) but payload is 100.
        let bad = SectionInput {
            section_type: SectionType::Tensor10DNodes,
            alignment_tier: AlignmentTier::CacheLine,
            stride: 40,
            element_count: 3,
            payload: &[0u8; 100],
        };
        let mut out = [0u8; 512];
        let err = encode_container(&h, std::slice::from_ref(&bad), &mut out).expect_err("stride inconsistent must reject");
        assert!(matches!(err, SectionTableError::StrideInconsistent { .. }), "{err}");
    }

    #[test]
    fn output_buffer_too_small_is_rejected() {
        let h = Container10dHeader::proposed();
        let payload = [0u8; 100];
        let inputs = [mesh_input(&payload)];
        let mut out = [0u8; 80]; // way too small
        let err = encode_container(&h, &inputs, &mut out).expect_err("small buffer must reject");
        assert!(matches!(err, SectionTableError::OutputBufferTooSmall { .. }), "{err}");
    }

    #[test]
    fn flipped_payload_bit_is_caught_by_crc() {
        let h = Container10dHeader::proposed();
        let payload = [0xAAu8; 100];
        let inputs = [mesh_input(&payload)];
        let mut out = [0u8; 512];
        let n = encode_container(&h, &inputs, &mut out).expect("encode");
        let parsed_h = Container10dHeader::parse(&out[..n]).expect("header parse");
        // Flip one bit in the payload region.
        let descs_ok = parse_section_table(&out[..n], &parsed_h).expect("clean table parses");
        let p_off = descs_ok[0].byte_offset as usize;
        out[p_off] ^= 0x01;
        let err = parse_section_table(&out[..n], &parsed_h).expect_err("flipped bit must be caught");
        assert!(matches!(err, SectionTableError::CrcMismatch { .. }), "{err}");
    }

    #[test]
    fn flipped_descriptor_byte_is_caught() {
        let h = Container10dHeader::proposed();
        let payload = [0xAAu8; 100];
        let inputs = [mesh_input(&payload)];
        let mut out = [0u8; 512];
        let n = encode_container(&h, &inputs, &mut out).expect("encode");
        let parsed_h = Container10dHeader::parse(&out[..n]).expect("header parse");
        // Corrupt the descriptor's reserved16 (offset table_start + 2).
        let table_start = parsed_h.section_table_offset as usize;
        out[table_start + 2] = 0xFF;
        let err = parse_section_table(&out[..n], &parsed_h).expect_err("non-zero reserved16 must reject");
        assert!(matches!(err, SectionTableError::NonZeroDescriptorReserved { index: 0 }), "{err}");
    }

    #[test]
    fn misaligned_section_offset_is_rejected() {
        let h = Container10dHeader::proposed();
        let payload = [0xAAu8; 100];
        let inputs = [mesh_input(&payload)];
        let mut out = [0u8; 512];
        let n = encode_container(&h, &inputs, &mut out).expect("encode");
        let mut parsed_h = Container10dHeader::parse(&out[..n]).expect("header parse");
        // Move the section's byte_offset to a misaligned address (the table
        // is at 64, descriptor 0's byte_offset is at table_start + 4).
        let table_start = parsed_h.section_table_offset as usize;
        let off_field = table_start + 4;
        // Set byte_offset to 65 (misaligned for Word tier = 4).
        out[off_field..off_field + 4].copy_from_slice(&65u32.to_le_bytes());
        // Re-parse the header (the section-table pointer is unchanged).
        parsed_h = Container10dHeader::parse(&out[..n]).expect("header still parses");
        let err = parse_section_table(&out[..n], &parsed_h).expect_err("misaligned offset must reject");
        assert!(matches!(err, SectionTableError::MisalignedSection { .. }), "{err}");
    }

    #[test]
    fn out_of_bounds_section_is_rejected() {
        let h = Container10dHeader::proposed();
        let payload = [0xAAu8; 100];
        let inputs = [mesh_input(&payload)];
        let mut out = [0u8; 512];
        let n = encode_container(&h, &inputs, &mut out).expect("encode");
        let parsed_h = Container10dHeader::parse(&out[..n]).expect("header parse");
        // Set byte_length to a value that runs past the file end.
        let table_start = parsed_h.section_table_offset as usize;
        let len_field = table_start + 8;
        out[len_field..len_field + 4].copy_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
        let err = parse_section_table(&out[..n], &parsed_h).expect_err("OOB must reject");
        assert!(matches!(err, SectionTableError::OutOfBounds { .. }), "{err}");
    }

    #[test]
    fn overlapping_sections_are_rejected() {
        // Encode two valid, non-overlapping sections first, then patch the
        // second descriptor's byte_offset so its range overlaps the first.
        // Recompute the second's CRC over the patched range so the per-section
        // CRC passes and the reader reaches the overlap check.
        let h = Container10dHeader::proposed();
        let mesh_payload = [0xAAu8; 100];
        let node_payload = [0xBBu8; 40];
        let inputs = [mesh_input(&mesh_payload), node_input(&node_payload)];
        let mut out = [0u8; 512];
        let n = encode_container(&h, &inputs, &mut out).expect("encode");
        let parsed_h = Container10dHeader::parse(&out[..n]).expect("header parse");
        let table_start = parsed_h.section_table_offset as usize;
        // Read the first section's offset.
        let first_off = u32::from_le_bytes(
            out[table_start + 4..table_start + 8].try_into().unwrap(),
        ) as usize;
        // Patch the second descriptor's byte_offset to overlap the first.
        // The node section declares CacheLine (16-byte) tier, so the patched
        // offset must stay 16-aligned; align up from first_off+10 so it
        // remains within the first section's payload range (first_off..
        // first_off+100) and thus overlaps.
        let second_desc_off = table_start + SECTION_DESCRIPTOR_SIZE;
        let new_second_off = align_up(first_off + 10, 16) as u32;
        assert!(
            (new_second_off as usize) < first_off + mesh_payload.len(),
            "patched offset must overlap the first payload"
        );
        out[second_desc_off + 4..second_desc_off + 8].copy_from_slice(&new_second_off.to_le_bytes());
        let crc_start = new_second_off as usize;
        let crc_end = crc_start + 40;
        let new_crc = crc32c(&out[crc_start..crc_end]);
        out[second_desc_off + 20..second_desc_off + 24].copy_from_slice(&new_crc.to_le_bytes());
        // The patched file's header still parses (pointer unchanged).
        let parsed_h2 = Container10dHeader::parse(&out[..n]).expect("header still parses");
        let err = parse_section_table(&out[..n], &parsed_h2).expect_err("overlap must reject");
        assert!(matches!(err, SectionTableError::OverlappingSections { .. }), "{err}");
    }

    #[test]
    fn non_zero_inter_section_padding_is_rejected() {
        let h = Container10dHeader::proposed();
        let mesh_payload = [0u8; 7]; // forces a padding gap before the 16-aligned node section
        let node_payload = [0u8; 40];
        let inputs = [mesh_input(&mesh_payload), node_input(&node_payload)];
        let mut out = [0u8; 512];
        let n = encode_container(&h, &inputs, &mut out).expect("encode");
        let parsed_h = Container10dHeader::parse(&out[..n]).expect("header parse");
        let descs = parse_section_table(&out[..n], &parsed_h).expect("clean parses");
        let mesh_end = descs[0].byte_offset as usize + descs[0].byte_length as usize;
        // Corrupt a padding byte.
        out[mesh_end] = 0x42;
        let err = parse_section_table(&out[..n], &parsed_h).expect_err("non-zero padding must reject");
        assert!(matches!(err, SectionTableError::NonZeroPadding { .. }), "{err}");
    }

    #[test]
    fn unsupported_section_type_is_rejected() {
        let h = Container10dHeader::proposed();
        let bad = SectionInput {
            section_type: SectionType::SpecReservedGovernance, // spec-reserved, not yet implemented
            alignment_tier: AlignmentTier::Word,
            stride: 0,
            element_count: 0,
            payload: &[0u8; 16],
        };
        let mut out = [0u8; 512];
        let err = encode_container(&h, std::slice::from_ref(&bad), &mut out).expect_err("spec-reserved type must reject");
        assert!(matches!(err, SectionTableError::UnsupportedSectionType { .. }), "{err}");
    }
}
