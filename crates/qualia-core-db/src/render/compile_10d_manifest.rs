//! Opt-in attachment of identity-v2 metadata to an already compiled `.10d` asset.
//!
//! Existing compiler outputs remain byte-identical unless callers explicitly invoke this
//! adapter. It preserves every existing section payload and descriptor contract, adds the
//! identity section, validates all field references, and seals the resulting whole-file CRC.

use crate::container_10d::header::{Container10dHeader, MAX_SECTION_COUNT};
use crate::container_10d::integrity::{compute_whole_file_crc32c, seal_whole_file_crc32c};
use crate::container_10d::manifold_identity_container::{
    encode_container_with_manifold_identity_extensions, ManifoldContainerError,
};
use crate::container_10d::manifold_identity_v2::{ManifoldIdentityV2, TypedFieldRecord};
use crate::container_10d::section::{
    parse_section_table, AlignmentTier, SectionInput, SectionTableError, SectionType,
};
use crate::render::assets::Mesh;
use crate::render::compile_10d::{compile_mesh_to_10d, Compile10dError};

#[derive(Debug, PartialEq, Eq)]
pub enum CompileWithIdentityError {
    Compile(Compile10dError),
    Attach(ManifestAttachError),
}

/// Compile a mesh using the established legacy pipeline, then explicitly attach the v2 identity
/// section. Existing `compile_mesh_to_10d` functions and their byte output remain unchanged.
pub fn compile_mesh_to_10d_with_identity(
    mesh: &Mesh,
    identity: &ManifoldIdentityV2,
    fields: &[TypedFieldRecord],
    supported_extension_kinds: &[u16],
) -> Result<Vec<u8>, CompileWithIdentityError> {
    let container = compile_mesh_to_10d(mesh).map_err(CompileWithIdentityError::Compile)?;
    let mut output = [];
    let needed = match attach_with_workspace(
        &container,
        identity,
        fields,
        supported_extension_kinds,
        &mut output,
    ) {
        Ok(written) => written,
        Err(ManifestAttachError::Manifest(ManifoldContainerError::Sections(
            SectionTableError::OutputBufferTooSmall { needed, .. },
        ))) => needed,
        Err(error) => return Err(CompileWithIdentityError::Attach(error)),
    };
    let mut output = vec![0u8; needed];
    let written = attach_with_workspace(
        &container,
        identity,
        fields,
        supported_extension_kinds,
        &mut output,
    )
    .map_err(CompileWithIdentityError::Attach)?;
    output.truncate(written);
    Ok(output)
}

fn attach_with_workspace(
    container: &[u8],
    identity: &ManifoldIdentityV2,
    fields: &[TypedFieldRecord],
    supported_extension_kinds: &[u16],
    out: &mut [u8],
) -> Result<usize, ManifestAttachError> {
    let mut manifest_scratch =
        [0u8; crate::container_10d::manifold_identity_v2::MANIFOLD_IDENTITY_V2_MAX_BYTES];
    let placeholder = SectionInput {
        section_type: SectionType::QuantizedMesh,
        alignment_tier: AlignmentTier::Word,
        stride: 0,
        element_count: 0,
        payload: &[],
    };
    let mut section_scratch = [placeholder; MAX_SECTION_COUNT as usize];
    attach_manifold_identity_v2_into(
        container,
        identity,
        fields,
        supported_extension_kinds,
        &mut manifest_scratch,
        &mut section_scratch,
        out,
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestAttachError {
    BadHeader,
    WholeFileCrcMismatch { expected: u32, actual: u32 },
    Section(SectionTableError),
    Manifest(ManifoldContainerError),
    IdentityAlreadyPresent,
    ScratchCapacity { needed: usize, capacity: usize },
    InvalidSectionRange { section_type: u8 },
}

impl From<SectionTableError> for ManifestAttachError {
    fn from(value: SectionTableError) -> Self {
        Self::Section(value)
    }
}

impl From<ManifoldContainerError> for ManifestAttachError {
    fn from(value: ManifoldContainerError) -> Self {
        Self::Manifest(value)
    }
}

/// Attach an identity-v2 manifest to a verified compiled container using caller-owned buffers.
///
/// `section_scratch` needs one slot for each existing section. The underlying envelope writer
/// validates output capacity before writing, so errors leave `out` unchanged. A source container
/// that already has an identity-v2 section is rejected instead of silently replacing identity.
pub fn attach_manifold_identity_v2_into<'a>(
    container: &'a [u8],
    identity: &ManifoldIdentityV2,
    fields: &[TypedFieldRecord],
    supported_extension_kinds: &[u16],
    manifest_scratch: &'a mut [u8],
    section_scratch: &mut [SectionInput<'a>],
    out: &mut [u8],
) -> Result<usize, ManifestAttachError> {
    let header =
        Container10dHeader::parse(container).map_err(|_| ManifestAttachError::BadHeader)?;
    if container.len() < 56 {
        return Err(ManifestAttachError::BadHeader);
    }
    let expected = u32::from_le_bytes(
        container[52..56]
            .try_into()
            .map_err(|_| ManifestAttachError::BadHeader)?,
    );
    let actual = compute_whole_file_crc32c(container);
    if actual != expected {
        return Err(ManifestAttachError::WholeFileCrcMismatch { expected, actual });
    }

    let descriptors = parse_section_table(container, &header)?;
    if descriptors
        .iter()
        .any(|descriptor| descriptor.typ() == Some(SectionType::ManifoldIdentityV2))
    {
        return Err(ManifestAttachError::IdentityAlreadyPresent);
    }
    let needed_sections =
        descriptors
            .len()
            .checked_add(1)
            .ok_or(ManifestAttachError::ScratchCapacity {
                needed: usize::MAX,
                capacity: section_scratch.len(),
            })?;
    if needed_sections > MAX_SECTION_COUNT as usize {
        return Err(ManifestAttachError::ScratchCapacity {
            needed: needed_sections,
            capacity: MAX_SECTION_COUNT as usize,
        });
    }
    if section_scratch.len() < needed_sections {
        return Err(ManifestAttachError::ScratchCapacity {
            needed: needed_sections,
            capacity: section_scratch.len(),
        });
    }

    let placeholder = SectionInput {
        section_type: SectionType::QuantizedMesh,
        alignment_tier: AlignmentTier::Word,
        stride: 0,
        element_count: 0,
        payload: &[],
    };
    let mut existing_sections = [placeholder; MAX_SECTION_COUNT as usize];
    for (slot, descriptor) in existing_sections.iter_mut().zip(descriptors.iter()) {
        let section_type = descriptor
            .typ()
            .ok_or(ManifestAttachError::InvalidSectionRange {
                section_type: descriptor.section_type,
            })?;
        let alignment_tier = descriptor
            .tier()
            .ok_or(ManifestAttachError::InvalidSectionRange {
                section_type: descriptor.section_type,
            })?;
        let start = descriptor.byte_offset as usize;
        let end = start.checked_add(descriptor.byte_length as usize).ok_or(
            ManifestAttachError::InvalidSectionRange {
                section_type: descriptor.section_type,
            },
        )?;
        let payload =
            container
                .get(start..end)
                .ok_or(ManifestAttachError::InvalidSectionRange {
                    section_type: descriptor.section_type,
                })?;
        *slot = SectionInput {
            section_type,
            alignment_tier,
            stride: descriptor.stride,
            element_count: descriptor.element_count,
            payload,
        };
    }

    let written = encode_container_with_manifold_identity_extensions(
        &header,
        &existing_sections[..descriptors.len()],
        identity,
        fields,
        supported_extension_kinds,
        manifest_scratch,
        section_scratch,
        out,
    )?;
    seal_whole_file_crc32c(&mut out[..written]);
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container_10d::integrity::{seal_whole_file_crc32c, verify_whole_file_crc32c};
    use crate::container_10d::manifold_identity_container::decode_container_manifold_identity;
    use crate::container_10d::manifold_identity_v2::{
        IntegrityIndexDigest, StableEntityId, FIELD_KIND_ELECTROMAGNETIC,
        MANIFOLD_IDENTITY_V2_MAX_BYTES,
    };
    use crate::container_10d::section::{encode_container, SectionInput};

    fn base_container() -> Vec<u8> {
        let header = Container10dHeader::proposed();
        let payload = [3u8, 1, 4, 1, 5, 9];
        let inputs = [SectionInput {
            section_type: SectionType::FieldSidecar,
            alignment_tier: AlignmentTier::Word,
            stride: 0,
            element_count: 0,
            payload: &payload,
        }];
        let mut bytes = [0; 4096];
        let length = encode_container(&header, &inputs, &mut bytes).unwrap();
        seal_whole_file_crc32c(&mut bytes[..length]);
        bytes[..length].to_vec()
    }

    fn sample_identity() -> ManifoldIdentityV2 {
        ManifoldIdentityV2 {
            entity_id: StableEntityId(*b"compile-q42-0001"),
            profile_id: 1,
            domain_id: 2,
            coordinate_convention_id: 3,
            coordinates: [0.0; 10],
            external_digest: Some(IntegrityIndexDigest([0xA5; 16])),
        }
    }

    #[test]
    fn attachment_preserves_payload_and_emits_sealed_resolvable_identity() {
        let source_storage = base_container();
        let source = source_storage.as_slice();
        let fields = [TypedFieldRecord {
            field_id: 1,
            kind: FIELD_KIND_ELECTROMAGNETIC,
            flags: 0,
            unit_id: 42,
            payload_section_id: SectionType::FieldSidecar as u32,
            payload_offset: 1,
            payload_length: 4,
        }];
        let mut manifest = [0; MANIFOLD_IDENTITY_V2_MAX_BYTES];
        let mut sections = [SectionInput {
            section_type: SectionType::FieldSidecar,
            alignment_tier: AlignmentTier::Word,
            stride: 0,
            element_count: 0,
            payload: &[],
        }; 2];
        let mut output = [0; 8192];
        let written = attach_manifold_identity_v2_into(
            source,
            &sample_identity(),
            &fields,
            &[],
            &mut manifest,
            &mut sections,
            &mut output,
        )
        .unwrap();

        let output_header = Container10dHeader::parse(&output[..written]).unwrap();
        verify_whole_file_crc32c(&mut output[..written]).unwrap();
        let descriptors = parse_section_table(&output[..written], &output_header).unwrap();
        let preserved = descriptors
            .iter()
            .find(|descriptor| descriptor.typ() == Some(SectionType::FieldSidecar))
            .unwrap();
        let start = preserved.byte_offset as usize;
        assert_eq!(&output[start..start + 6], &[3, 1, 4, 1, 5, 9]);

        let mut field_scratch = [TypedFieldRecord::default(); 1];
        let mut decoded_fields = [TypedFieldRecord::default(); 1];
        let (decoded, count) = decode_container_manifold_identity(
            &output[..written],
            &output_header,
            &[],
            &mut field_scratch,
            &mut decoded_fields,
        )
        .unwrap();
        assert_eq!(decoded, sample_identity());
        assert_eq!(count, 1);
        assert_eq!(decoded_fields, fields);
    }

    #[test]
    fn attachment_rejects_bad_source_crc_without_touching_output() {
        let mut source = base_container();
        source[64] ^= 0x40;
        let mut manifest = [0; MANIFOLD_IDENTITY_V2_MAX_BYTES];
        let mut sections = [SectionInput {
            section_type: SectionType::FieldSidecar,
            alignment_tier: AlignmentTier::Word,
            stride: 0,
            element_count: 0,
            payload: &[],
        }; 2];
        let mut output = [0xA5; 8192];
        assert!(matches!(
            attach_manifold_identity_v2_into(
                &source,
                &sample_identity(),
                &[],
                &[],
                &mut manifest,
                &mut sections,
                &mut output,
            ),
            Err(ManifestAttachError::WholeFileCrcMismatch { .. })
        ));
        assert!(output.iter().all(|byte| *byte == 0xA5));
    }
}
