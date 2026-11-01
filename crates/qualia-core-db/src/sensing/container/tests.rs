use super::*;
use crate::container_10d::{
    encode_container_with_manifold_identity_extensions,
    field_section::{FieldDescriptor, FieldRepresentationKind, FieldSupportKind, FieldTypeKind},
    typed_payloads::TypedPayloadResolver,
    AlignmentTier, Container10dHeader, ManifoldIdentityV2, SectionInput, SectionType,
    StableEntityId, TypedFieldRecord, FIELD_KIND_ELECTROMAGNETIC,
};
use crate::sensing::{
    record::{
        CalibrationDeclaration, MeasurementQuality, MeasurementRecord, ProjectionDeclaration,
        ProvenanceDeclaration, ResolvedFieldPayloadBinding, MEASUREMENT_INDEX_RECORD_SIZE,
    },
    MeasurementModality, PhysicalFieldSemantics, SemanticId,
};

const KIND: u16 = 0x8101;
const CONTAINER: [u8; 16] = [1; 16];

#[test]
fn minimal_sensing_envelope_round_trip_uses_existing_codec() {
    let profile = SensingIndexProfile::native_v1();
    let identity = ManifoldIdentityV2 {
        entity_id: StableEntityId([7; 16]),
        profile_id: 1,
        domain_id: 2,
        coordinate_convention_id: 3,
        coordinates: [0.0; 10],
        external_digest: None,
    };
    let mut payload_scratch = [0; 16 + MEASUREMENT_INDEX_RECORD_SIZE];
    let mut manifest_scratch = [0; 4096];
    let mut field_scratch = [TypedFieldRecord::default(); 2];
    let mut section_scratch = [SectionInput {
        section_type: SectionType::FieldSidecar,
        alignment_tier: AlignmentTier::Word,
        stride: 0,
        element_count: 0,
        payload: &[],
    }; 2];
    let mut out = [0; 8192];
    let original = record();
    let used = encode_sensing_container(
        profile,
        &Container10dHeader::proposed(),
        &identity,
        CONTAINER,
        10,
        &[5; 16],
        &[raw_field()],
        &[original],
        &[],
        &mut payload_scratch,
        &mut manifest_scratch,
        &mut field_scratch,
        &mut section_scratch,
        &mut out,
    )
    .unwrap();
    let mut scratch = [TypedFieldRecord::default(); 2];
    let mut fields = [TypedFieldRecord::default(); 2];
    let resolver = profile
        .resolve(&out[..used], &[], &mut scratch, &mut fields)
        .unwrap();
    let view = profile.open(&resolver, 10, CONTAINER).unwrap();
    assert_eq!(view.record(0, physical()).unwrap(), original);
    assert_eq!(
        view.resolve_record(0, physical()).unwrap().payload(),
        &[5; 16]
    );
}

#[test]
fn minimal_authoring_refuses_declaration_mismatch_without_output_changes() {
    let profile = SensingIndexProfile::native_v1();
    let identity = ManifoldIdentityV2 {
        entity_id: StableEntityId([7; 16]),
        profile_id: 1,
        domain_id: 2,
        coordinate_convention_id: 3,
        coordinates: [0.0; 10],
        external_digest: None,
    };
    let mut raw = raw_field();
    raw.unit_id += 1;
    let mut payload_scratch = [0; 16 + MEASUREMENT_INDEX_RECORD_SIZE];
    let mut manifest_scratch = [0; 4096];
    let mut field_scratch = [TypedFieldRecord::default(); 2];
    let mut section_scratch = [SectionInput {
        section_type: SectionType::FieldSidecar,
        alignment_tier: AlignmentTier::Word,
        stride: 0,
        element_count: 0,
        payload: &[],
    }; 2];
    let mut out = [0xaa; 8192];
    assert!(matches!(
        encode_sensing_container(
            profile,
            &Container10dHeader::proposed(),
            &identity,
            CONTAINER,
            10,
            &[5; 16],
            &[raw],
            &[record()],
            &[],
            &mut payload_scratch,
            &mut manifest_scratch,
            &mut field_scratch,
            &mut section_scratch,
            &mut out
        ),
        Err(SensingAssemblyError::DeclarationMismatch)
    ));
    assert!(out.iter().all(|byte| *byte == 0xaa));
}

#[test]
fn native_profile_packs_resolves_and_decodes_without_global_registration() {
    let profile = SensingIndexProfile::native_v1();
    let original = record();
    let mut packed = [0; 16 + MEASUREMENT_INDEX_RECORD_SIZE];
    let result = profile
        .pack(CONTAINER, 10, &[5; 16], &[original], &mut packed)
        .unwrap();
    let mut data = [0; 8192];
    let used = envelope(
        &packed[..result.field_section_bytes],
        &[raw_field(), result.index_field],
        &[profile.kind()],
        &mut data,
    );
    let mut scratch = [TypedFieldRecord::default(); 2];
    let mut fields = [TypedFieldRecord::default(); 2];
    let resolver = profile
        .resolve(&data[..used], &[], &mut scratch, &mut fields)
        .unwrap();
    let view = profile.open(&resolver, 10, CONTAINER).unwrap();
    assert_eq!(view.record(0, physical()).unwrap(), original);
    let resolved = view.resolve_record(0, physical()).unwrap();
    assert_eq!(resolved.record(), original);
    assert_eq!(resolved.binding().raw_payload(), original.raw_payload());
    assert_eq!(resolved.payload(), &[5; 16]);
    assert_eq!(
        resolved.payload().as_ptr(),
        resolver.field(9).unwrap().unwrap().bytes.as_ptr()
    );
}

fn raw_field() -> TypedFieldRecord {
    TypedFieldRecord {
        field_id: 9,
        kind: FIELD_KIND_ELECTROMAGNETIC,
        flags: 0,
        unit_id: 11,
        payload_section_id: SectionType::FieldSidecar as u32,
        payload_offset: 0,
        payload_length: 16,
    }
}

fn physical() -> PhysicalFieldSemantics {
    PhysicalFieldSemantics::from_existing(
        &FieldDescriptor {
            name_hash: 22,
            unit_hash: 33,
            type_kind: FieldTypeKind::Spectral as u8,
            support: FieldSupportKind::Stream as u8,
            representation: FieldRepresentationKind::Sampled as u8,
            reserved: 0,
            reserved2: 0,
            sample_offset: 0,
        },
        &raw_field(),
        MeasurementModality::Electromagnetic,
        SemanticId::new(13).unwrap(),
    )
    .unwrap()
}

fn envelope(
    payload: &[u8],
    fields: &[TypedFieldRecord],
    extensions: &[u16],
    out: &mut [u8],
) -> usize {
    let section = SectionInput {
        section_type: SectionType::FieldSidecar,
        alignment_tier: AlignmentTier::Word,
        stride: 0,
        element_count: 0,
        payload,
    };
    let identity = ManifoldIdentityV2 {
        entity_id: StableEntityId([7; 16]),
        profile_id: 1,
        domain_id: 2,
        coordinate_convention_id: 3,
        coordinates: [0.0; 10],
        external_digest: None,
    };
    encode_container_with_manifold_identity_extensions(
        &Container10dHeader::proposed(),
        &[section],
        &identity,
        fields,
        extensions,
        &mut [0u8; 4096],
        &mut [section; 2],
        out,
    )
    .unwrap()
}

fn record() -> MeasurementRecord {
    let mut data = [0; 8192];
    let used = envelope(&[5; 16], &[raw_field()], &[], &mut data);
    let mut scratch = [TypedFieldRecord::default(); 2];
    let mut fields = [TypedFieldRecord::default(); 2];
    let resolver =
        TypedPayloadResolver::new(&data[..used], &[], &mut scratch, &mut fields).unwrap();
    let binding = ResolvedFieldPayloadBinding::new(physical(), &resolver, CONTAINER).unwrap();
    MeasurementRecord::new(
        21,
        SemanticId::Unknown,
        binding,
        ProvenanceDeclaration::Unknown,
        CalibrationDeclaration::Missing,
        MeasurementQuality::Unknown,
        None,
        ProjectionDeclaration::Unknown,
    )
    .unwrap()
}

#[test]
fn index_round_trip_uses_registered_extension_and_same_envelope_binding() {
    let original = record();
    let mut packed = [0; 16 + MEASUREMENT_INDEX_RECORD_SIZE];
    let result =
        pack_sensing_index_field(CONTAINER, 10, KIND, &[5; 16], &[original], &mut packed).unwrap();
    let mut data = [0; 8192];
    let used = envelope(
        &packed[..result.field_section_bytes],
        &[raw_field(), result.index_field],
        &[KIND],
        &mut data,
    );
    let mut scratch = [TypedFieldRecord::default(); 2];
    let mut fields = [TypedFieldRecord::default(); 2];
    assert!(TypedPayloadResolver::new(&data[..used], &[], &mut scratch, &mut fields).is_err());
    let resolver =
        TypedPayloadResolver::new(&data[..used], &[KIND], &mut scratch, &mut fields).unwrap();
    let view = SensingIndexView::open(&resolver, 10, KIND, CONTAINER).unwrap();
    assert_eq!(view.count(), 1);
    assert_eq!(view.record(0, physical()).unwrap(), original);
    assert!(matches!(
        view.record(1, physical()),
        Err(SensingContainerError::Range)
    ));
    assert!(matches!(
        SensingIndexView::open(&resolver, 10, KIND, [9; 16])
            .unwrap()
            .record(0, physical()),
        Err(SensingContainerError::Record(_))
    ));
}

#[test]
fn pack_refusals_leave_output_unchanged() {
    let original = record();
    let mut out = [0xaa; 1024];
    assert!(matches!(
        pack_sensing_index_field(CONTAINER, 10, KIND, &[5; 15], &[original], &mut out),
        Err(SensingContainerError::IndexPayloadOverlap)
    ));
    assert!(matches!(
        pack_sensing_index_field([2; 16], 10, KIND, &[5; 16], &[original], &mut out),
        Err(SensingContainerError::ContainerMismatch)
    ));
    assert!(matches!(
        pack_sensing_index_field(
            CONTAINER,
            10,
            KIND,
            &[5; 16],
            &[original, original],
            &mut out
        ),
        Err(SensingContainerError::DuplicateRecordId)
    ));
    assert!(out.iter().all(|byte| *byte == 0xaa));
}

#[test]
fn malformed_row_and_self_referencing_raw_payload_are_refused() {
    let original = record();
    let mut packed = [0; 16 + MEASUREMENT_INDEX_RECORD_SIZE];
    let result =
        pack_sensing_index_field(CONTAINER, 10, KIND, &[5; 16], &[original], &mut packed).unwrap();
    packed[16] ^= 1;
    let mut data = [0; 8192];
    let used = envelope(
        &packed,
        &[raw_field(), result.index_field],
        &[KIND],
        &mut data,
    );
    let mut scratch = [TypedFieldRecord::default(); 2];
    let mut fields = [TypedFieldRecord::default(); 2];
    let resolver =
        TypedPayloadResolver::new(&data[..used], &[KIND], &mut scratch, &mut fields).unwrap();
    assert!(matches!(
        SensingIndexView::open(&resolver, 10, KIND, CONTAINER),
        Err(SensingContainerError::InvalidIndex)
    ));
    assert!(matches!(
        pack_sensing_index_field(CONTAINER, 9, KIND, &[5; 16], &[original], &mut packed),
        Err(SensingContainerError::IndexPayloadOverlap)
    ));
}

#[test]
fn decoded_raw_payload_cannot_overlap_index_rows() {
    let mut packed = [0; 16 + MEASUREMENT_INDEX_RECORD_SIZE];
    let result =
        pack_sensing_index_field(CONTAINER, 10, KIND, &[5; 16], &[record()], &mut packed).unwrap();
    // Raw-payload offset is at byte 98 of the canonical measurement row.
    packed[16 + 98..16 + 106].copy_from_slice(&16u64.to_le_bytes());
    let mut raw = raw_field();
    raw.payload_offset = 16;
    let mut data = [0; 8192];
    let used = envelope(&packed, &[raw, result.index_field], &[KIND], &mut data);
    let mut scratch = [TypedFieldRecord::default(); 2];
    let mut fields = [TypedFieldRecord::default(); 2];
    let resolver =
        TypedPayloadResolver::new(&data[..used], &[KIND], &mut scratch, &mut fields).unwrap();
    let view = SensingIndexView::open(&resolver, 10, KIND, CONTAINER).unwrap();
    assert!(matches!(
        view.record(0, physical()),
        Err(SensingContainerError::IndexPayloadOverlap)
    ));
}
