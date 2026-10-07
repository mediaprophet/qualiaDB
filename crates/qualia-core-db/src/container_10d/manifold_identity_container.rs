//! Caller-buffered integration between the `.10d` v2 identity manifest and section envelope.
//!
//! In this envelope binding, `TypedFieldRecord::payload_section_id` is the stable
//! `SectionType` discriminant. This keeps references portable across deterministic section
//! reordering. Ranges are checked against the referenced payload's byte length on read.

use super::header::Container10dHeader;
use super::manifold_identity_v2::{
    decode_manifold_identity_v2, encode_manifold_identity_v2, ManifestV2Error, ManifoldIdentityV2,
    TypedFieldRecord,
};
use super::section::{
    encode_container, parse_section_table, AlignmentTier, SectionDescriptor, SectionInput,
    SectionTableError, SectionType,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifoldContainerError {
    Manifest(ManifestV2Error),
    Sections(SectionTableError),
    MissingIdentityManifest,
    ScratchCapacity {
        needed: usize,
        capacity: usize,
    },
    UnknownPayloadSection {
        field_index: usize,
        section_id: u32,
    },
    PayloadRangeOutOfBounds {
        field_index: usize,
        end: u64,
        length: u32,
    },
}

impl From<ManifestV2Error> for ManifoldContainerError {
    fn from(value: ManifestV2Error) -> Self {
        Self::Manifest(value)
    }
}

impl From<SectionTableError> for ManifoldContainerError {
    fn from(value: SectionTableError) -> Self {
        Self::Sections(value)
    }
}

/// Encode a full `.10d` v2 section envelope containing the identity manifest.
///
/// `manifest_scratch` and `section_scratch` are caller-owned bounded workspace. Every non-manifest
/// section is copied by reference into the temporary section table; the generic envelope writer
/// handles canonical ordering, alignment, CRCs, and final output capacity before returning.
pub fn encode_container_with_manifold_identity<'a>(
    header: &Container10dHeader,
    other_sections: &[SectionInput<'a>],
    identity: &ManifoldIdentityV2,
    fields: &[TypedFieldRecord],
    manifest_scratch: &'a mut [u8],
    section_scratch: &mut [SectionInput<'a>],
    out: &mut [u8],
) -> Result<usize, ManifoldContainerError> {
    if other_sections
        .iter()
        .any(|section| section.section_type == SectionType::ManifoldIdentityV2)
    {
        return Err(ManifoldContainerError::Sections(
            SectionTableError::DuplicateSectionType {
                section_type: SectionType::ManifoldIdentityV2 as u8,
            },
        ));
    }
    let needed_sections =
        other_sections
            .len()
            .checked_add(1)
            .ok_or(ManifoldContainerError::ScratchCapacity {
                needed: usize::MAX,
                capacity: section_scratch.len(),
            })?;
    if section_scratch.len() < needed_sections {
        return Err(ManifoldContainerError::ScratchCapacity {
            needed: needed_sections,
            capacity: section_scratch.len(),
        });
    }
    let manifest_len = encode_manifold_identity_v2(identity, fields, manifest_scratch)?;
    validate_input_ranges(fields, other_sections)?;
    section_scratch[..other_sections.len()].copy_from_slice(other_sections);
    section_scratch[other_sections.len()] = SectionInput {
        section_type: SectionType::ManifoldIdentityV2,
        alignment_tier: AlignmentTier::Page,
        stride: 0,
        element_count: 0,
        payload: &manifest_scratch[..manifest_len],
    };
    Ok(encode_container(
        header,
        &section_scratch[..needed_sections],
        out,
    )?)
}

/// Read the manifest and validate every typed payload reference before publishing decoded fields.
///
/// The function uses `field_scratch` for transactional decode: on any malformed section, unknown
/// target, range error, or insufficient output capacity, `out_fields` remains unchanged.
pub fn decode_container_manifold_identity(
    data: &[u8],
    header: &Container10dHeader,
    supported_extension_kinds: &[u16],
    field_scratch: &mut [TypedFieldRecord],
    out_fields: &mut [TypedFieldRecord],
) -> Result<(ManifoldIdentityV2, usize), ManifoldContainerError> {
    let descriptors = parse_section_table(data, header)?;
    let descriptor = descriptors
        .iter()
        .find(|item| item.typ() == Some(SectionType::ManifoldIdentityV2))
        .ok_or(ManifoldContainerError::MissingIdentityManifest)?;
    let offset = descriptor.byte_offset as usize;
    let end = offset
        .checked_add(descriptor.byte_length as usize)
        .ok_or(ManifoldContainerError::MissingIdentityManifest)?;
    let manifest = data
        .get(offset..end)
        .ok_or(ManifoldContainerError::MissingIdentityManifest)?;
    let (identity, field_count) =
        decode_manifold_identity_v2(manifest, supported_extension_kinds, field_scratch)?;
    if out_fields.len() < field_count {
        return Err(ManifoldContainerError::ScratchCapacity {
            needed: field_count,
            capacity: out_fields.len(),
        });
    }
    validate_payload_ranges(&field_scratch[..field_count], descriptors)?;
    out_fields[..field_count].copy_from_slice(&field_scratch[..field_count]);
    Ok((identity, field_count))
}

fn validate_payload_ranges(
    fields: &[TypedFieldRecord],
    sections: &[SectionDescriptor],
) -> Result<(), ManifoldContainerError> {
    for (field_index, field) in fields.iter().enumerate() {
        if field.payload_section_id == 0 {
            continue;
        }
        let section_type = u8::try_from(field.payload_section_id).ok();
        let section = section_type
            .and_then(SectionType::from_u8)
            .and_then(|kind| {
                (kind != SectionType::ManifoldIdentityV2)
                    .then(|| sections.iter().find(|item| item.typ() == Some(kind)))
                    .flatten()
            })
            .ok_or(ManifoldContainerError::UnknownPayloadSection {
                field_index,
                section_id: field.payload_section_id,
            })?;
        let range_end = field
            .payload_offset
            .checked_add(field.payload_length)
            .ok_or(ManifoldContainerError::PayloadRangeOutOfBounds {
                field_index,
                end: u64::MAX,
                length: section.byte_length,
            })?;
        if range_end > u64::from(section.byte_length) {
            return Err(ManifoldContainerError::PayloadRangeOutOfBounds {
                field_index,
                end: range_end,
                length: section.byte_length,
            });
        }
    }
    Ok(())
}

fn validate_input_ranges(
    fields: &[TypedFieldRecord],
    sections: &[SectionInput<'_>],
) -> Result<(), ManifoldContainerError> {
    for (field_index, field) in fields.iter().enumerate() {
        if field.payload_section_id == 0 {
            continue;
        }
        let section_type = u8::try_from(field.payload_section_id).ok();
        let section = section_type
            .and_then(SectionType::from_u8)
            .and_then(|kind| sections.iter().find(|item| item.section_type == kind))
            .ok_or(ManifoldContainerError::UnknownPayloadSection {
                field_index,
                section_id: field.payload_section_id,
            })?;
        let range_end = field
            .payload_offset
            .checked_add(field.payload_length)
            .ok_or(ManifoldContainerError::PayloadRangeOutOfBounds {
                field_index,
                end: u64::MAX,
                length: u32::try_from(section.payload.len()).unwrap_or(u32::MAX),
            })?;
        if range_end > section.payload.len() as u64 {
            return Err(ManifoldContainerError::PayloadRangeOutOfBounds {
                field_index,
                end: range_end,
                length: u32::try_from(section.payload.len()).unwrap_or(u32::MAX),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container_10d::{IntegrityIndexDigest, StableEntityId};

    fn identity() -> ManifoldIdentityV2 {
        ManifoldIdentityV2 {
            entity_id: StableEntityId([7; 16]),
            profile_id: 1,
            domain_id: 2,
            coordinate_convention_id: 3,
            coordinates: [0.0; 10],
            external_digest: Some(IntegrityIndexDigest([9; 16])),
        }
    }

    #[test]
    fn envelope_round_trip_validates_stable_section_references() {
        let payload = [1u8, 2, 3, 4];
        let other = [SectionInput {
            section_type: SectionType::FieldSidecar,
            alignment_tier: AlignmentTier::Word,
            stride: 0,
            element_count: 0,
            payload: &payload,
        }];
        let fields = [TypedFieldRecord {
            field_id: 1,
            kind: super::super::manifold_identity_v2::FIELD_KIND_ELECTROMAGNETIC,
            flags: 0,
            unit_id: 10,
            payload_section_id: SectionType::FieldSidecar as u32,
            payload_offset: 1,
            payload_length: 3,
        }];
        let header = Container10dHeader::proposed();
        let mut manifest =
            [0u8; super::super::manifold_identity_v2::MANIFOLD_IDENTITY_V2_MAX_BYTES];
        let mut section_scratch = [other[0]; 2];
        let mut bytes = [0u8; 8192];
        let used = encode_container_with_manifold_identity(
            &header,
            &other,
            &identity(),
            &fields,
            &mut manifest,
            &mut section_scratch,
            &mut bytes,
        )
        .unwrap();
        let parsed_header = Container10dHeader::parse(&bytes[..used]).unwrap();
        let mut field_scratch = [TypedFieldRecord::default(); 1];
        let mut out_fields = [TypedFieldRecord::default(); 1];
        let (decoded, count) = decode_container_manifold_identity(
            &bytes[..used],
            &parsed_header,
            &[],
            &mut field_scratch,
            &mut out_fields,
        )
        .unwrap();
        assert_eq!(decoded, identity());
        assert_eq!(count, 1);
        assert_eq!(out_fields[0], fields[0]);
    }

    #[test]
    fn invalid_payload_range_is_rejected_before_envelope_write() {
        let payload = [1u8, 2, 3, 4];
        let other = [SectionInput {
            section_type: SectionType::FieldSidecar,
            alignment_tier: AlignmentTier::Word,
            stride: 0,
            element_count: 0,
            payload: &payload,
        }];
        let fields = [TypedFieldRecord {
            field_id: 1,
            kind: super::super::manifold_identity_v2::FIELD_KIND_ELECTROMAGNETIC,
            flags: 0,
            unit_id: 10,
            payload_section_id: SectionType::FieldSidecar as u32,
            payload_offset: 3,
            payload_length: 2,
        }];
        let header = Container10dHeader::proposed();
        let mut manifest =
            [0u8; super::super::manifold_identity_v2::MANIFOLD_IDENTITY_V2_MAX_BYTES];
        let mut section_scratch = [other[0]; 2];
        let mut bytes = [0xa5u8; 8192];
        assert!(matches!(
            encode_container_with_manifold_identity(
                &header,
                &other,
                &identity(),
                &fields,
                &mut manifest,
                &mut section_scratch,
                &mut bytes,
            ),
            Err(ManifoldContainerError::PayloadRangeOutOfBounds { .. })
        ));
        assert!(bytes.iter().all(|byte| *byte == 0xa5));
    }
}
