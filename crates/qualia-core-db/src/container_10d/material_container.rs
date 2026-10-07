//! Container-level attach/read helpers for the optional MAT1 section.

use super::header::{Container10dHeader, HeaderParseError};
use super::integrity::{compute_whole_file_crc32c, seal_whole_file_crc32c, IntegrityError};
use super::material_section::{
    decode_material_section_into, encode_material_section, material_section_counts, MaterialRecord,
    MaterialSectionError, SubmeshRange,
};
use super::section::{
    encode_container, parse_section_table, AlignmentTier, SectionInput, SectionTableError,
    SectionType,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaterialContainerError {
    Header(HeaderParseError),
    Integrity(IntegrityError),
    Section(SectionTableError),
    Material(MaterialSectionError),
    AlreadyPresent,
    Missing,
    TooManySections,
}

impl From<HeaderParseError> for MaterialContainerError {
    fn from(value: HeaderParseError) -> Self {
        Self::Header(value)
    }
}
impl From<IntegrityError> for MaterialContainerError {
    fn from(value: IntegrityError) -> Self {
        Self::Integrity(value)
    }
}
impl From<SectionTableError> for MaterialContainerError {
    fn from(value: SectionTableError) -> Self {
        Self::Section(value)
    }
}
impl From<MaterialSectionError> for MaterialContainerError {
    fn from(value: MaterialSectionError) -> Self {
        Self::Material(value)
    }
}

/// Add MAT1 to an already sealed container, preserving its existing sections and resealing CRC.
pub fn attach_material_section(
    container: &[u8],
    materials: &[MaterialRecord],
    ranges: &[SubmeshRange],
    mesh_index_count: u32,
) -> Result<Vec<u8>, MaterialContainerError> {
    let header = Container10dHeader::parse(container)?;
    verify_integrity(container, header.header_crc32c)?;
    let descriptors = parse_section_table(container, &header)?;
    if descriptors
        .iter()
        .any(|d| d.typ() == Some(SectionType::Materials))
    {
        return Err(MaterialContainerError::AlreadyPresent);
    }
    if descriptors.len() >= crate::container_10d::header::MAX_SECTION_COUNT as usize {
        return Err(MaterialContainerError::TooManySections);
    }

    let material_len = super::material_section::encoded_len(materials.len(), ranges.len())
        .ok_or(MaterialSectionError::CountLimit)?;
    let mut material_bytes = vec![0u8; material_len];
    let written =
        encode_material_section(materials, ranges, mesh_index_count, &mut material_bytes)?;
    material_bytes.truncate(written);

    // Construction is cold and bounded by the section-count and material-count limits.
    let mut inputs = Vec::with_capacity(descriptors.len() + 1);
    for descriptor in descriptors {
        let start = descriptor.byte_offset as usize;
        let end = start
            .checked_add(descriptor.byte_length as usize)
            .filter(|&end| end <= container.len())
            .ok_or(SectionTableError::OutOfBounds {
                index: 0,
                offset: descriptor.byte_offset,
                length: descriptor.byte_length,
                file_len: container.len(),
            })?;
        inputs.push(SectionInput {
            section_type: descriptor
                .typ()
                .ok_or(SectionTableError::UndefinedSectionType {
                    index: 0,
                    got: descriptor.section_type,
                })?,
            alignment_tier: descriptor
                .tier()
                .ok_or(SectionTableError::UndefinedAlignmentTier {
                    index: 0,
                    got: descriptor.alignment_tier,
                })?,
            stride: descriptor.stride,
            element_count: descriptor.element_count,
            payload: &container[start..end],
        });
    }
    inputs.push(SectionInput {
        section_type: SectionType::Materials,
        alignment_tier: AlignmentTier::CacheLine,
        stride: 0,
        element_count: 0,
        payload: &material_bytes,
    });

    let needed = match encode_container(&header, &inputs, &mut []) {
        Err(SectionTableError::OutputBufferTooSmall { needed, .. }) => needed,
        Ok(size) => size,
        Err(error) => return Err(error.into()),
    };
    let mut output = vec![0u8; needed];
    let written = encode_container(&header, &inputs, &mut output)?;
    output.truncate(written);
    seal_whole_file_crc32c(&mut output);
    Ok(output)
}

/// Read MAT1 into caller-owned buffers after validating the enclosing container and mesh ranges.
pub fn read_material_section_into(
    container: &[u8],
    mesh_index_count: u32,
    materials_out: &mut [MaterialRecord],
    ranges_out: &mut [SubmeshRange],
) -> Result<(usize, usize), MaterialContainerError> {
    let header = Container10dHeader::parse(container)?;
    verify_integrity(container, header.header_crc32c)?;
    let descriptors = parse_section_table(container, &header)?;
    let descriptor = descriptors
        .iter()
        .find(|d| d.typ() == Some(SectionType::Materials))
        .ok_or(MaterialContainerError::Missing)?;
    let start = descriptor.byte_offset as usize;
    let end = start
        .checked_add(descriptor.byte_length as usize)
        .filter(|&end| end <= container.len())
        .ok_or(SectionTableError::OutOfBounds {
            index: 0,
            offset: descriptor.byte_offset,
            length: descriptor.byte_length,
            file_len: container.len(),
        })?;
    Ok(decode_material_section_into(
        &container[start..end],
        mesh_index_count,
        materials_out,
        ranges_out,
    )?)
}

/// Validate the sealed container and MAT1 header, then return bounded material/range counts.
pub fn material_container_counts(
    container: &[u8],
) -> Result<(usize, usize), MaterialContainerError> {
    let header = Container10dHeader::parse(container)?;
    verify_integrity(container, header.header_crc32c)?;
    let descriptors = parse_section_table(container, &header)?;
    let descriptor = descriptors
        .iter()
        .find(|d| d.typ() == Some(SectionType::Materials))
        .ok_or(MaterialContainerError::Missing)?;
    let start = descriptor.byte_offset as usize;
    let end = start
        .checked_add(descriptor.byte_length as usize)
        .filter(|&end| end <= container.len())
        .ok_or(SectionTableError::OutOfBounds {
            index: 0,
            offset: descriptor.byte_offset,
            length: descriptor.byte_length,
            file_len: container.len(),
        })?;
    Ok(material_section_counts(&container[start..end])?)
}

fn verify_integrity(container: &[u8], expected: u32) -> Result<(), IntegrityError> {
    let actual = compute_whole_file_crc32c(container);
    if actual == expected {
        Ok(())
    } else {
        Err(IntegrityError::WholeFileCrcMismatch {
            expected,
            got: actual,
        })
    }
}
