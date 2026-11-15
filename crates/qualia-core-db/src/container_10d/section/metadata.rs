//! Original descriptor metadata checks, shared by full and selective readers.
//! No payload, padding or whole-file checksum claim is made by this function.
use super::{
    AlignmentTier, HEADER_BYTE_SIZE, SectionDescriptor, SectionTableError as E, SectionType,
};
pub fn validate_descriptor_metadata(
    d: &SectionDescriptor,
    index: usize,
    version: u16,
    extent: u64,
    table_start: u64,
    table_end: u64,
) -> Result<(), E> {
    if d.reserved16 != 0 {
        return Err(E::NonZeroDescriptorReserved { index });
    }
    let st = SectionType::from_u8(d.section_type).ok_or(E::UndefinedSectionType {
        index,
        got: d.section_type,
    })?;
    if st == SectionType::Undefined {
        return Err(E::UndefinedSectionType {
            index,
            got: d.section_type,
        });
    }
    if st == SectionType::ManifoldIdentityV2
        && version < crate::container_10d::header::HEADER_VERSION
    {
        return Err(E::SectionRequiresVersion {
            section_type: d.section_type,
            minimum_version: crate::container_10d::header::HEADER_VERSION,
            actual_version: version,
        });
    }
    let tier = AlignmentTier::from_u8(d.alignment_tier).ok_or(E::UndefinedAlignmentTier {
        index,
        got: d.alignment_tier,
    })?;
    let start = u64::from(d.byte_offset);
    let length = u64::from(d.byte_length);
    let end = start + length;
    if start % tier.to_bytes() as u64 != 0 {
        return Err(E::MisalignedSection {
            index,
            offset: d.byte_offset,
            tier,
        });
    }
    if end > extent {
        return Err(E::OutOfBounds {
            index,
            offset: d.byte_offset,
            length: d.byte_length,
            file_len: usize::try_from(extent).unwrap_or(usize::MAX),
        });
    }
    if length != 0 && (start < HEADER_BYTE_SIZE as u64 || (start < table_end && end > table_start))
    {
        return Err(E::SectionOverlapsMetadata { index });
    }
    if d.stride > 0 && u64::from(d.stride) * u64::from(d.element_count) != length {
        return Err(E::StrideInconsistentDescriptor {
            index,
            stride: d.stride,
            element_count: d.element_count,
            byte_length: d.byte_length,
        });
    }
    Ok(())
}
