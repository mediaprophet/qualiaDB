//! Explicit structural retention; never used implicitly by the strict reader.
//!
//! Unknown tags, header flags and descriptor metadata can be inspected/copied
//! byte-exactly, not interpreted or admitted. Unknown outer header or alignment
//! grammar refuses. Semantic edits with unresolved dependencies refuse. This
//! does not negotiate required/optional extensions or validate typed payloads.
use super::{
    table_validation, AlignmentTier, SectionDescriptor, SectionTableError, SectionType,
    SECTION_DESCRIPTOR_SIZE,
};
use crate::container_10d::crc32c::crc32c;
use crate::container_10d::header::{
    Container10dHeader, HeaderParseError, FLAG_DEFAULT_DISPOSITION_REFUSE, HEADER_VERSION,
};
use crate::container_10d::integrity::{
    check_whole_file_crc32c, seal_whole_file_crc32c, IntegrityError,
};

/// Explicit limits apply before checksum traversal or output mutation. The
/// canonical header's independent maximum section count still applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpaquePreservationBudget {
    pub max_bytes: usize,
    pub max_sections: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpaquePreservationError {
    Header(HeaderParseError),
    Integrity(IntegrityError),
    Section(SectionTableError),
    ByteBudget { needed: usize, maximum: usize },
    SectionBudget { needed: usize, maximum: usize },
    OutputTooSmall { needed: usize, have: usize },
    UnsupportedTarget { section_type: u8 },
    MissingTarget { section_type: u8 },
    LengthChangeUnsupported { existing: usize, replacement: usize },
    UnknownHeaderFlags { flags: u16 },
    ReservedDescriptorMetadata { index: usize, reserved16: u16 },
    UnresolvedDependencies { section_type: u8 },
    OpaqueTailBytes { at: usize },
}

impl From<HeaderParseError> for OpaquePreservationError {
    fn from(value: HeaderParseError) -> Self {
        Self::Header(value)
    }
}
impl From<IntegrityError> for OpaquePreservationError {
    fn from(value: IntegrityError) -> Self {
        Self::Integrity(value)
    }
}
impl From<SectionTableError> for OpaquePreservationError {
    fn from(value: SectionTableError) -> Self {
        Self::Section(value)
    }
}
impl std::fmt::Display for OpaquePreservationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "10d opaque preservation: {self:?}")
    }
}
impl std::error::Error for OpaquePreservationError {}

/// Fully structurally checked immutable bytes. This is not a typed-admission
/// token. Private fields prevent callers constructing a view from unchecked data.
#[derive(Debug)]
pub struct OpaqueContainerView<'a> {
    bytes: &'a [u8],
    header: Container10dHeader,
    sections: &'a [SectionDescriptor],
}
impl<'a> OpaqueContainerView<'a> {
    pub fn bytes(&self) -> &'a [u8] {
        self.bytes
    }
    pub fn header(&self) -> &Container10dHeader {
        &self.header
    }
    pub fn sections(&self) -> &'a [SectionDescriptor] {
        self.sections
    }
    /// Raw bytes only; no section semantics are asserted.
    pub fn payload(&self, index: usize) -> Option<&'a [u8]> {
        let d = self.sections.get(index)?;
        Some(&self.bytes[d.byte_offset as usize..d.byte_offset as usize + d.byte_length as usize])
    }
}

/// Validate the complete supported envelope without decoding unknown payloads.
/// Padding follows the canonical reader (interior zero; tail opaque). Nonzero
/// descriptor reserved16 and unknown header flags are retained only here.
/// Returns a typed error for an unaligned descriptor table, never a cast panic.
pub fn inspect_opaque_container(
    data: &[u8],
    budget: OpaquePreservationBudget,
) -> Result<OpaqueContainerView<'_>, OpaquePreservationError> {
    if data.len() > budget.max_bytes {
        return Err(OpaquePreservationError::ByteBudget {
            needed: data.len(),
            maximum: budget.max_bytes,
        });
    }
    let header = Container10dHeader::parse(data)?;
    let count = header.section_count as usize;
    if count > budget.max_sections {
        return Err(OpaquePreservationError::SectionBudget {
            needed: count,
            maximum: budget.max_sections,
        });
    }
    check_whole_file_crc32c(data)?;
    let sections = inspect_table(data, &header)?;
    Ok(OpaqueContainerView {
        bytes: data,
        header,
        sections,
    })
}

fn inspect_table<'a>(
    data: &'a [u8],
    header: &Container10dHeader,
) -> Result<&'a [SectionDescriptor], SectionTableError> {
    if header.section_count == 0 {
        return Ok(&[]);
    }
    // Header parser established pointer/count bounds; checked_add also covers
    // a 32-bit target near usize::MAX before any slicing or metadata arithmetic.
    let start = header.section_table_offset as usize;
    let end = start
        .checked_add(header.section_count as usize * SECTION_DESCRIPTOR_SIZE)
        .ok_or(SectionTableError::BadSectionTablePointer {
            offset: header.section_table_offset,
            count: header.section_count,
        })?;
    let table = data
        .get(start..end)
        .ok_or(SectionTableError::InputTooShort {
            got: data.len(),
            need: end,
        })?;
    let descriptors = table_validation::borrow_table(table)?;
    for (index, d) in descriptors.iter().enumerate() {
        if d.section_type == 0 {
            return Err(SectionTableError::UndefinedSectionType { index, got: 0 });
        }
        if descriptors[..index]
            .iter()
            .any(|old| old.section_type == d.section_type)
        {
            return Err(SectionTableError::DuplicateSectionType {
                section_type: d.section_type,
            });
        }
        if d.typ() == Some(SectionType::ManifoldIdentityV2) && header.version < HEADER_VERSION {
            return Err(SectionTableError::SectionRequiresVersion {
                section_type: d.section_type,
                minimum_version: HEADER_VERSION,
                actual_version: header.version,
            });
        }
        let tier = AlignmentTier::from_u8(d.alignment_tier).ok_or(
            SectionTableError::UndefinedAlignmentTier {
                index,
                got: d.alignment_tier,
            },
        )?;
        let offset = d.byte_offset as usize;
        let length = d.byte_length as usize;
        if offset % tier.to_bytes() != 0 {
            return Err(SectionTableError::MisalignedSection {
                index,
                offset: d.byte_offset,
                tier,
            });
        }
        let bounds_error = || SectionTableError::OutOfBounds {
            index,
            offset: d.byte_offset,
            length: d.byte_length,
            file_len: data.len(),
        };
        let payload_end = offset.checked_add(length).ok_or_else(bounds_error)?;
        let payload = data.get(offset..payload_end).ok_or_else(bounds_error)?;
        table_validation::metadata_range(d, index, start, end)?;
        if d.stride != 0
            && u64::from(d.stride) * u64::from(d.element_count) != u64::from(d.byte_length)
        {
            return Err(SectionTableError::StrideInconsistentDescriptor {
                index,
                stride: d.stride,
                element_count: d.element_count,
                byte_length: d.byte_length,
            });
        }
        let actual = crc32c(payload);
        if actual != d.crc32c {
            return Err(SectionTableError::CrcMismatch {
                index,
                section_type: d.section_type,
                expected: d.crc32c,
                got: actual,
            });
        }
        for (previous, old) in descriptors[..index].iter().enumerate() {
            // Every earlier descriptor has already passed checked bounds.
            if length != 0
                && old.byte_length != 0
                && offset < old.byte_offset as usize + old.byte_length as usize
                && (old.byte_offset as usize) < payload_end
            {
                return Err(SectionTableError::OverlappingSections {
                    index_a: previous,
                    index_b: index,
                });
            }
        }
    }
    table_validation::padding(data, descriptors, start, end)?;
    Ok(descriptors)
}

fn capacity(out: &[u8], needed: usize) -> Result<(), OpaquePreservationError> {
    if out.len() < needed {
        Err(OpaquePreservationError::OutputTooSmall {
            needed,
            have: out.len(),
        })
    } else {
        Ok(())
    }
}

/// Copy the complete structurally checked envelope byte-exactly. All checks
/// finish before output mutation; output suffix is untouched. Safe Rust borrows
/// exclude overlapping source/output. This grants no authority over unknowns.
pub fn copy_opaque_container_into(
    data: &[u8],
    budget: OpaquePreservationBudget,
    out: &mut [u8],
) -> Result<usize, OpaquePreservationError> {
    let view = inspect_opaque_container(data, budget)?;
    capacity(out, view.bytes.len())?;
    out[..data.len()].copy_from_slice(data);
    Ok(data.len())
}

/// Replace one implemented section's raw bytes at the same offset and length,
/// retaining every unrelated byte. Typed payload correctness and known semantic
/// dependency admission remain the caller's job; this is not a material/mesh
/// validator. Refuses changed bytes if any unknown flags, reserved metadata or
/// unimplemented section could hide a dependency. Byte-identical no-ops may copy
/// structurally checked unknowns. Section addition/relocation is unsupported.
/// Every error precedes mutation; only payload and its two CRC layers change.
pub fn replace_known_section_bytes_preserving_layout(
    data: &[u8],
    section_type: SectionType,
    replacement: &[u8],
    budget: OpaquePreservationBudget,
    out: &mut [u8],
) -> Result<usize, OpaquePreservationError> {
    let view = inspect_opaque_container(data, budget)?;
    if !section_type.is_implemented() {
        return Err(OpaquePreservationError::UnsupportedTarget {
            section_type: section_type as u8,
        });
    }
    let index = view
        .sections
        .iter()
        .position(|d| d.typ() == Some(section_type))
        .ok_or(OpaquePreservationError::MissingTarget {
            section_type: section_type as u8,
        })?;
    let target = view.sections[index];
    if replacement.len() != target.byte_length as usize {
        return Err(OpaquePreservationError::LengthChangeUnsupported {
            existing: target.byte_length as usize,
            replacement: replacement.len(),
        });
    }
    let unchanged = view.payload(index) == Some(replacement);
    if !unchanged {
        let flags = view.header.flags & !FLAG_DEFAULT_DISPOSITION_REFUSE;
        if flags != 0 {
            return Err(OpaquePreservationError::UnknownHeaderFlags { flags });
        }
        for (index, d) in view.sections.iter().enumerate() {
            if d.reserved16 != 0 {
                return Err(OpaquePreservationError::ReservedDescriptorMetadata {
                    index,
                    reserved16: d.reserved16,
                });
            }
            if !d.typ().is_some_and(SectionType::is_implemented) {
                return Err(OpaquePreservationError::UnresolvedDependencies {
                    section_type: d.section_type,
                });
            }
        }
        // The strict table grammar deliberately does not interpret trailing
        // bytes. Retain them for copies, but do not assume they are independent
        // of an edited section. Zero tail padding carries no additional bytes.
        let occupied_end = view
            .sections
            .iter()
            .filter(|d| d.byte_length != 0)
            .map(|d| d.byte_offset as usize + d.byte_length as usize)
            .max()
            .unwrap_or(0)
            .max(
                view.header.section_table_offset as usize
                    + view.sections.len() * SECTION_DESCRIPTOR_SIZE,
            )
            .max(crate::container_10d::header::HEADER_BYTE_SIZE);
        if let Some(relative) = data[occupied_end..].iter().position(|b| *b != 0) {
            return Err(OpaquePreservationError::OpaqueTailBytes {
                at: occupied_end + relative,
            });
        }
    }
    capacity(out, data.len())?;
    // Commit point: all validation, policy and capacity failures are above.
    out[..data.len()].copy_from_slice(data);
    if !unchanged {
        let offset = target.byte_offset as usize;
        out[offset..offset + replacement.len()].copy_from_slice(replacement);
        let crc_offset = view.header.section_table_offset as usize
            + index * SECTION_DESCRIPTOR_SIZE
            + std::mem::offset_of!(SectionDescriptor, crc32c);
        out[crc_offset..crc_offset + 4].copy_from_slice(&crc32c(replacement).to_le_bytes());
        seal_whole_file_crc32c(&mut out[..data.len()]);
    }
    Ok(data.len())
}
