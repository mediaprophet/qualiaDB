//! Container-level attach/read helpers for the UV01 vertex-coordinate section.

use super::header::{Container10dHeader, HeaderParseError, MAX_SECTION_COUNT};
use super::integrity::{compute_whole_file_crc32c, seal_whole_file_crc32c, IntegrityError};
use super::section::{
    encode_container, parse_section_table, AlignmentTier, SectionInput, SectionTableError,
    SectionType,
};
use super::texture_coordinate_section::{
    decode_texture_coordinates_into, encode_texture_coordinate_section,
    texture_coordinate_section_count, TextureCoordinateSectionError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextureCoordinateContainerError {
    Header(HeaderParseError),
    Integrity(IntegrityError),
    Section(SectionTableError),
    Payload(TextureCoordinateSectionError),
    AlreadyPresent,
    Missing,
    TooManySections,
    CountMismatch { expected: usize, got: usize },
}

impl From<HeaderParseError> for TextureCoordinateContainerError {
    fn from(value: HeaderParseError) -> Self {
        Self::Header(value)
    }
}
impl From<IntegrityError> for TextureCoordinateContainerError {
    fn from(value: IntegrityError) -> Self {
        Self::Integrity(value)
    }
}
impl From<SectionTableError> for TextureCoordinateContainerError {
    fn from(value: SectionTableError) -> Self {
        Self::Section(value)
    }
}
impl From<TextureCoordinateSectionError> for TextureCoordinateContainerError {
    fn from(value: TextureCoordinateSectionError) -> Self {
        Self::Payload(value)
    }
}

/// Add UV01 to a sealed `.10d` container and reseal whole-file integrity.
pub fn attach_texture_coordinate_section(
    container: &[u8],
    coordinates: &[[f32; 2]],
) -> Result<Vec<u8>, TextureCoordinateContainerError> {
    let header = Container10dHeader::parse(container)?;
    verify_integrity(container, header.header_crc32c)?;
    let descriptors = parse_section_table(container, &header)?;
    if descriptors
        .iter()
        .any(|descriptor| descriptor.typ() == Some(SectionType::TextureCoordinates))
    {
        return Err(TextureCoordinateContainerError::AlreadyPresent);
    }
    if descriptors.len() >= MAX_SECTION_COUNT as usize {
        return Err(TextureCoordinateContainerError::TooManySections);
    }
    let coordinate_len = super::texture_coordinate_section::encoded_len(coordinates.len()).ok_or(
        TextureCoordinateSectionError::CountMismatch {
            expected: 1,
            got: coordinates.len(),
        },
    )?;
    let mut coordinate_bytes = vec![0; coordinate_len];
    let written = encode_texture_coordinate_section(coordinates, &mut coordinate_bytes)?;
    coordinate_bytes.truncate(written);

    let mut inputs = Vec::with_capacity(descriptors.len() + 1);
    for (index, descriptor) in descriptors.iter().enumerate() {
        let start = descriptor.byte_offset as usize;
        let end = start
            .checked_add(descriptor.byte_length as usize)
            .filter(|&end| end <= container.len())
            .ok_or(SectionTableError::OutOfBounds {
                index,
                offset: descriptor.byte_offset,
                length: descriptor.byte_length,
                file_len: container.len(),
            })?;
        inputs.push(SectionInput {
            section_type: descriptor
                .typ()
                .ok_or(SectionTableError::UndefinedSectionType {
                    index,
                    got: descriptor.section_type,
                })?,
            alignment_tier: descriptor
                .tier()
                .ok_or(SectionTableError::UndefinedAlignmentTier {
                    index,
                    got: descriptor.alignment_tier,
                })?,
            stride: descriptor.stride,
            element_count: descriptor.element_count,
            payload: &container[start..end],
        });
    }
    inputs.push(SectionInput {
        section_type: SectionType::TextureCoordinates,
        alignment_tier: AlignmentTier::Word,
        // The payload includes its own UV01 header, so it is a blob rather than a raw strided
        // array in the outer table. The inner header supplies and validates the vertex count.
        stride: 0,
        element_count: 0,
        payload: &coordinate_bytes,
    });
    let needed = match encode_container(&header, &inputs, &mut []) {
        Err(SectionTableError::OutputBufferTooSmall { needed, .. }) => needed,
        Ok(size) => size,
        Err(error) => return Err(error.into()),
    };
    let mut output = vec![0; needed];
    let written = encode_container(&header, &inputs, &mut output)?;
    output.truncate(written);
    seal_whole_file_crc32c(&mut output);
    Ok(output)
}

/// Read a validated UV01 section into a caller-owned vertex buffer.
pub fn read_texture_coordinate_section_into(
    container: &[u8],
    expected_vertex_count: usize,
    out: &mut [[f32; 2]],
) -> Result<usize, TextureCoordinateContainerError> {
    let header = Container10dHeader::parse(container)?;
    verify_integrity(container, header.header_crc32c)?;
    let descriptors = parse_section_table(container, &header)?;
    let descriptor = descriptors
        .iter()
        .find(|descriptor| descriptor.typ() == Some(SectionType::TextureCoordinates))
        .ok_or(TextureCoordinateContainerError::Missing)?;
    if descriptor.stride != 0 || descriptor.element_count != 0 {
        return Err(SectionTableError::StrideInconsistentDescriptor {
            index: 0,
            stride: descriptor.stride,
            element_count: descriptor.element_count,
            byte_length: descriptor.byte_length,
        }
        .into());
    }
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
    let payload = &container[start..end];
    let count = texture_coordinate_section_count(payload)?;
    if count != expected_vertex_count {
        return Err(TextureCoordinateContainerError::CountMismatch {
            expected: expected_vertex_count,
            got: count,
        });
    }
    Ok(decode_texture_coordinates_into(
        payload,
        expected_vertex_count,
        out,
    )?)
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
