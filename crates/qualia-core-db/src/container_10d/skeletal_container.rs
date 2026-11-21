//! Caller-buffered SKL1 / SKL2 attachment and integrity-checked readback.
use super::header::{Container10dHeader, HeaderParseError, MAX_SECTION_COUNT};
use super::integrity::{compute_whole_file_crc32c, seal_whole_file_crc32c};
use super::mesh_section::{parse_mesh_header, MeshSectionError};
use super::section::{
    encode_container, parse_section_table, AlignmentTier, SectionInput, SectionTableError,
    SectionType,
};
use super::skeletal_section::{
    SkeletalSection, SkeletalSectionError, SkeletalSectionPayload, SKL2_MAGIC,
};

#[derive(Debug, PartialEq, Eq)]
pub enum SkeletalContainerError {
    Header(HeaderParseError),
    Sections(SectionTableError),
    Mesh(MeshSectionError),
    Payload(SkeletalSectionError),
    Integrity,
    AlreadyPresent,
    MissingMesh,
    Missing,
    TooManySections,
    CountLimit,
    InvalidDescriptor,
}

fn checked_header(bytes: &[u8]) -> Result<Container10dHeader, SkeletalContainerError> {
    if bytes.len() > super::GRAPHICS_ASSET_RING_BYTES {
        return Err(SkeletalContainerError::CountLimit);
    }
    let header = Container10dHeader::parse(bytes).map_err(SkeletalContainerError::Header)?;
    if compute_whole_file_crc32c(bytes) != header.header_crc32c {
        return Err(SkeletalContainerError::Integrity);
    }
    Ok(header)
}

/// Parsed skeletal animation payload stored inside `SectionType::SkeletalAnimation`.
pub enum ContainerSkeletalSection<'a> {
    Skl1(SkeletalSection<'a>),
    Skl2(SkeletalSectionPayload),
}

fn find_skeletal_slice(
    bytes: &[u8],
) -> Result<(usize, &[u8]), SkeletalContainerError> {
    let header = checked_header(bytes)?;
    let sections = parse_section_table(bytes, &header).map_err(SkeletalContainerError::Sections)?;
    let section = sections
        .iter()
        .find(|s| s.typ() == Some(SectionType::SkeletalAnimation))
        .ok_or(SkeletalContainerError::Missing)?;
    if section.stride != 0 || section.element_count != 0 {
        return Err(SkeletalContainerError::InvalidDescriptor);
    }
    let mesh = sections
        .iter()
        .find(|s| s.typ() == Some(SectionType::QuantizedMesh))
        .ok_or(SkeletalContainerError::MissingMesh)?;
    let (mesh_header, _) = parse_mesh_header(
        &bytes[mesh.byte_offset as usize..(mesh.byte_offset + mesh.byte_length) as usize],
    )
    .map_err(SkeletalContainerError::Mesh)?;
    let slice =
        &bytes[section.byte_offset as usize..(section.byte_offset + section.byte_length) as usize];
    Ok((mesh_header.vertex_count as usize, slice))
}

/// Read the optional SKL1 section after validating the existing container envelope,
/// whole-file seal, per-section seals and actual QuantizedMesh vertex count.
pub fn read_skeletal_section(bytes: &[u8]) -> Result<SkeletalSection<'_>, SkeletalContainerError> {
    let (vertex_count, slice) = find_skeletal_slice(bytes)?;
    SkeletalSection::parse(slice, vertex_count).map_err(SkeletalContainerError::Payload)
}

/// Read an SKL1 or SKL2 section payload from `SectionType::SkeletalAnimation`.
pub fn read_skeletal_section_payload(
    bytes: &[u8],
) -> Result<ContainerSkeletalSection<'_>, SkeletalContainerError> {
    let (vertex_count, slice) = find_skeletal_slice(bytes)?;
    if slice.starts_with(&SKL2_MAGIC) {
        SkeletalSectionPayload::from_bytes(slice)
            .map(ContainerSkeletalSection::Skl2)
            .map_err(SkeletalContainerError::Payload)
    } else {
        SkeletalSection::parse(slice, vertex_count)
            .map(ContainerSkeletalSection::Skl1)
            .map_err(SkeletalContainerError::Payload)
    }
}

/// Read an SKL2 payload from `SectionType::SkeletalAnimation`, failing if missing or SKL1-only.
pub fn read_skl2_section(
    bytes: &[u8],
) -> Result<SkeletalSectionPayload, SkeletalContainerError> {
    let (_vertex_count, slice) = find_skeletal_slice(bytes)?;
    SkeletalSectionPayload::from_bytes(slice).map_err(SkeletalContainerError::Payload)
}

/// Attach an already encoded SKL1 or SKL2 payload to a sealed container, retaining all
/// existing fields. Output remains untouched on validation or capacity failure.
/// The section writer reports the required capacity in its structured error.
pub fn attach_skeletal_section_into(
    bytes: &[u8],
    payload: &[u8],
    out: &mut [u8],
) -> Result<usize, SkeletalContainerError> {
    let header = checked_header(bytes)?;
    let sections = parse_section_table(bytes, &header).map_err(SkeletalContainerError::Sections)?;
    if sections.len() >= MAX_SECTION_COUNT as usize {
        return Err(SkeletalContainerError::TooManySections);
    }
    if sections
        .iter()
        .any(|s| s.typ() == Some(SectionType::SkeletalAnimation))
    {
        return Err(SkeletalContainerError::AlreadyPresent);
    }
    let mesh = sections
        .iter()
        .find(|s| s.typ() == Some(SectionType::QuantizedMesh))
        .ok_or(SkeletalContainerError::MissingMesh)?;
    let (mesh_header, _) = parse_mesh_header(
        &bytes[mesh.byte_offset as usize..(mesh.byte_offset + mesh.byte_length) as usize],
    )
    .map_err(SkeletalContainerError::Mesh)?;
    if payload.starts_with(&SKL2_MAGIC) {
        SkeletalSectionPayload::from_bytes(payload).map_err(SkeletalContainerError::Payload)?;
    } else {
        SkeletalSection::parse(payload, mesh_header.vertex_count as usize)
            .map_err(SkeletalContainerError::Payload)?;
    }
    let empty = SectionInput {
        section_type: SectionType::Undefined,
        alignment_tier: AlignmentTier::Byte,
        stride: 0,
        element_count: 0,
        payload: &[],
    };
    let mut inputs = [empty; MAX_SECTION_COUNT as usize];
    for (i, section) in sections.iter().enumerate() {
        inputs[i] = SectionInput {
            section_type: section
                .typ()
                .ok_or(SkeletalContainerError::InvalidDescriptor)?,
            alignment_tier: section
                .tier()
                .ok_or(SkeletalContainerError::InvalidDescriptor)?,
            stride: section.stride,
            element_count: section.element_count,
            payload: &bytes[section.byte_offset as usize
                ..(section.byte_offset + section.byte_length) as usize],
        };
    }
    inputs[sections.len()] = SectionInput {
        section_type: SectionType::SkeletalAnimation,
        alignment_tier: AlignmentTier::Word,
        stride: 0,
        element_count: 0,
        payload,
    };
    // Admission before writes also enforces the enclosing execution budget.
    let needed = match encode_container(&header, &inputs[..sections.len() + 1], &mut []) {
        Err(SectionTableError::OutputBufferTooSmall { needed, .. }) => needed,
        Err(error) => return Err(SkeletalContainerError::Sections(error)),
        Ok(needed) => needed,
    };
    if needed > super::GRAPHICS_ASSET_RING_BYTES {
        return Err(SkeletalContainerError::CountLimit);
    }
    let written = encode_container(&header, &inputs[..sections.len() + 1], out)
        .map_err(SkeletalContainerError::Sections)?;
    seal_whole_file_crc32c(&mut out[..written]);
    Ok(written)
}

/// Encode an `SKL2` payload into caller-supplied workspace and attach it to a sealed `.10d`
/// container without extra heap allocation on the hot path.
pub fn attach_skl2_section_into(
    bytes: &[u8],
    payload: &SkeletalSectionPayload,
    scratch: &mut [u8],
    out: &mut [u8],
) -> Result<usize, SkeletalContainerError> {
    let written = payload
        .encode_v2_into(scratch)
        .map_err(SkeletalContainerError::Payload)?;
    attach_skeletal_section_into(bytes, &scratch[..written], out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container_10d::skeletal_section::{
        KinematicSolverRegime, Skl2JointDescriptorV2, SwingTwistConeLimits, SKL2_NONE_INDEX,
        SKL2_VERSION_1, SKL2_VERSION_2,
    };
    use crate::render::assets::Mesh;
    use crate::render::compile_10d::{compile_mesh_to_10d_with_nodes, decode_10d_mesh};

    fn sample_mesh_container() -> Vec<u8> {
        let mesh = Mesh {
            positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            triangles: vec![[0, 1, 2]],
            min: [0.0; 3],
            max: [1.0, 1.0, 0.0],
        };
        compile_mesh_to_10d_with_nodes(&mesh, &[]).unwrap()
    }

    fn sample_skl2() -> SkeletalSectionPayload {
        SkeletalSectionPayload::new(vec![
            Skl2JointDescriptorV2::new(
                101,
                SKL2_NONE_INDEX,
                SwingTwistConeLimits::symmetric(0.4, 0.25),
                KinematicSolverRegime::ArticulatedRigidBodyDca,
                1180.0,
                0.35,
                0.30,
                14.0,
                0,
                1,
                250,
            ),
            Skl2JointDescriptorV2::new(
                102,
                0,
                SwingTwistConeLimits::new(0.7, 0.5, -0.3, 0.3),
                KinematicSolverRegime::CosseratRodContinuum,
                920.0,
                0.24,
                0.22,
                3.1,
                1,
                2,
                215,
            ),
        ])
    }

    #[test]
    fn attach_and_readback_skl2_v2_preserves_mesh_and_descriptors() {
        let original = sample_mesh_container();
        let skl2 = sample_skl2();
        let mut scratch = [0u8; 512];
        let mut output = [0u8; 8192];
        let len = attach_skl2_section_into(&original, &skl2, &mut scratch, &mut output).unwrap();

        match read_skeletal_section_payload(&output[..len]).unwrap() {
            ContainerSkeletalSection::Skl2(decoded) => {
                assert_eq!(decoded.source_version, SKL2_VERSION_2);
                assert_eq!(decoded.joints, skl2.joints);
            }
            ContainerSkeletalSection::Skl1(_) => panic!("expected SKL2 payload"),
        }

        let direct = read_skl2_section(&output[..len]).unwrap();
        assert_eq!(direct.joints, skl2.joints);

        // SKL1 strict parser must reject SKL2 magic cleanly.
        assert!(matches!(
            read_skeletal_section(&output[..len]),
            Err(SkeletalContainerError::Payload(SkeletalSectionError::Header))
        ));

        // Existing mesh remains intact.
        assert_eq!(decode_10d_mesh(&output[..len]).unwrap().triangles, vec![[0, 1, 2]]);
    }

    #[test]
    fn attach_skl2_v1_upgrades_on_readback() {
        let original = sample_mesh_container();
        let skl2 = sample_skl2();
        let v1_bytes = skl2.to_bytes_v1().unwrap();
        let mut output = [0u8; 8192];
        let len = attach_skeletal_section_into(&original, &v1_bytes, &mut output).unwrap();

        let decoded = read_skl2_section(&output[..len]).unwrap();
        assert_eq!(decoded.source_version, SKL2_VERSION_1);
        assert_eq!(decoded.joints.len(), 2);
        assert_eq!(decoded.joints[1].parent_index, 0);
        assert_eq!(decoded.joints[1].swing_twist, skl2.joints[1].swing_twist);
    }

    #[test]
    fn skl2_container_corruption_fails_closed() {
        let original = sample_mesh_container();
        let skl2 = sample_skl2();
        let mut scratch = [0u8; 512];
        let mut output = [0u8; 8192];
        let len = attach_skl2_section_into(&original, &skl2, &mut scratch, &mut output).unwrap();

        output[len - 1] ^= 0x01;
        assert!(matches!(
            read_skl2_section(&output[..len]),
            Err(SkeletalContainerError::Integrity)
        ));
    }
}
