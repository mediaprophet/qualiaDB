//! Caller-buffered authoring of a minimal sensing envelope.
use super::super::record::MeasurementRecord;
use super::{
    RegistrationError, SensingContainerError, SensingIndexProfile, MAX_SENSING_PROFILE_EXTENSIONS,
};
use crate::container_10d::{
    encode_container_with_manifold_identity_extensions,
    manifold_identity_container::ManifoldContainerError,
    manifold_identity_v2::MANIFOLD_IDENTITY_V2_MAX_FIELDS, AlignmentTier, Container10dHeader,
    ManifoldIdentityV2, SectionInput, SectionType, TypedFieldRecord,
};

#[derive(Debug)]
pub enum SensingAssemblyError {
    DeclarationMismatch,
    Capacity,
    Index(SensingContainerError),
    Registration(RegistrationError),
    Envelope(ManifoldContainerError),
}

/// Assemble a new envelope containing FieldSidecar and MID2 sections. All raw
/// fields must refer to the supplied prefix; this does not rewrite an existing
/// container or preserve other sections. Identity, namespaces and metadata are
/// explicit caller declarations, not verified scientific or policy claims.
/// Legacy FIEL descriptors are not authored here; their semantic pairing stays
/// with the caller, as in `PhysicalFieldSemantics::from_existing`.
///
/// Scratch is caller-owned and may change on failure. The existing checked
/// envelope writer validates before changing final output. Slice capacities
/// are the caller-selected byte/entry budgets; this function allocates nothing.
#[allow(clippy::too_many_arguments)]
pub fn encode_sensing_container<'a>(
    profile: SensingIndexProfile,
    header: &Container10dHeader,
    identity: &ManifoldIdentityV2,
    container_id: [u8; 16],
    index_field_id: u64,
    raw_prefix: &[u8],
    raw_fields: &[TypedFieldRecord],
    records: &[MeasurementRecord],
    other_extensions: &[u16],
    payload_scratch: &'a mut [u8],
    manifest_scratch: &'a mut [u8],
    field_scratch: &mut [TypedFieldRecord],
    section_scratch: &mut [SectionInput<'a>],
    output: &mut [u8],
) -> Result<usize, SensingAssemblyError> {
    let count = raw_fields
        .len()
        .checked_add(1)
        .ok_or(SensingAssemblyError::Capacity)?;
    if count > MANIFOLD_IDENTITY_V2_MAX_FIELDS
        || field_scratch.len() < count
        || section_scratch.len() < 2
    {
        return Err(SensingAssemblyError::Capacity);
    }
    for (i, field) in raw_fields.iter().enumerate() {
        if field.field_id == 0
            || field.field_id == index_field_id
            || field.kind == profile.kind()
            || field.flags != 0
            || field.payload_section_id != SectionType::FieldSidecar as u32
            || field.payload_length == 0
            || field
                .payload_offset
                .checked_add(field.payload_length)
                .filter(|end| *end <= raw_prefix.len() as u64)
                .is_none()
            || raw_fields[..i]
                .iter()
                .any(|prior| prior.field_id == field.field_id)
        {
            return Err(SensingAssemblyError::DeclarationMismatch);
        }
    }
    for record in records {
        let physical = record.physical();
        let raw = record.raw_payload();
        let Some(field) = raw_fields
            .iter()
            .find(|field| field.field_id == physical.field_id())
        else {
            return Err(SensingAssemblyError::DeclarationMismatch);
        };
        if field.kind != physical.typed_field_kind()
            || field.unit_id != physical.typed_unit_id().value().unwrap_or(0)
            || field.payload_section_id != raw.section_id()
            || field.payload_offset != raw.offset()
            || field.payload_length != raw.length()
        {
            return Err(SensingAssemblyError::DeclarationMismatch);
        }
    }
    let mut extensions = [0u16; MAX_SENSING_PROFILE_EXTENSIONS];
    let extension_count = profile
        .write_supported_kinds(other_extensions, &mut extensions)
        .map_err(SensingAssemblyError::Registration)?;
    let packed = profile
        .pack(
            container_id,
            index_field_id,
            raw_prefix,
            records,
            payload_scratch,
        )
        .map_err(SensingAssemblyError::Index)?;
    field_scratch[..raw_fields.len()].copy_from_slice(raw_fields);
    field_scratch[raw_fields.len()] = packed.index_field;
    let section = SectionInput {
        section_type: SectionType::FieldSidecar,
        alignment_tier: AlignmentTier::Word,
        stride: 0,
        element_count: 0,
        payload: &payload_scratch[..packed.field_section_bytes],
    };
    encode_container_with_manifold_identity_extensions(
        header,
        &[section],
        identity,
        &field_scratch[..count],
        &extensions[..extension_count],
        manifest_scratch,
        section_scratch,
        output,
    )
    .map_err(SensingAssemblyError::Envelope)
}
