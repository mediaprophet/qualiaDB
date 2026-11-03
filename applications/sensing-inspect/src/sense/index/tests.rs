use super::*;
use qualia_core_db::{
    container_10d as c,
    sensing::{
        container::pack_sensing_index_field,
        record::{
            CalibrationDeclaration, MeasurementQuality, ProjectionDeclaration,
            ProvenanceDeclaration, MEASUREMENT_INDEX_RECORD_SIZE,
        },
        MeasurementModality, PhysicalFieldSemantics, SemanticId,
    },
};

const KIND: u16 = 0x8101;

#[test]
fn optional_selection_requires_all_three_arguments_and_explicit_registration() {
    assert!(Selection::optional(None, None, None).unwrap().is_none());
    for mask in 1..7 {
        assert!(Selection::optional(
            (mask & 1 != 0).then_some(10),
            (mask & 2 != 0).then_some(0),
            (mask & 4 != 0).then_some(KIND)
        )
        .is_err());
    }
    assert!(Selection::optional(Some(0), Some(0), Some(KIND)).is_err());
    assert!(Selection::optional(Some(10), Some(0), Some(0)).is_err());
    let selection = Selection::optional(Some(10), Some(0), Some(KIND))
        .unwrap()
        .unwrap();
    assert!(selection.require_registered(&[]).is_err());
    assert!(selection.require_registered(&[KIND]).is_ok());
}

fn raw_field() -> c::TypedFieldRecord {
    c::TypedFieldRecord {
        field_id: 9,
        kind: c::FIELD_KIND_ELECTROMAGNETIC,
        flags: 0,
        unit_id: 11,
        payload_section_id: c::SectionType::FieldSidecar as u32,
        payload_offset: 0,
        payload_length: 16,
    }
}

fn physical() -> PhysicalFieldSemantics {
    use c::field_section::{
        FieldDescriptor, FieldRepresentationKind, FieldSupportKind, FieldTypeKind,
    };
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
    fields: &[c::TypedFieldRecord],
    extensions: &[u16],
    out: &mut [u8],
) -> usize {
    let section = c::SectionInput {
        section_type: c::SectionType::FieldSidecar,
        alignment_tier: c::AlignmentTier::Word,
        stride: 0,
        element_count: 0,
        payload,
    };
    let identity = c::ManifoldIdentityV2 {
        entity_id: c::StableEntityId([7; 16]),
        profile_id: 1,
        domain_id: 2,
        coordinate_convention_id: 3,
        coordinates: [0.0; 10],
        external_digest: None,
    };
    c::encode_container_with_manifold_identity_extensions(
        &c::Container10dHeader::proposed(),
        &[section],
        &identity,
        fields,
        extensions,
        &mut [0; 4096],
        &mut [section; 2],
        out,
    )
    .unwrap()
}

fn record(binding: ResolvedFieldPayloadBinding, id: u64) -> MeasurementRecord {
    MeasurementRecord::new(
        id,
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
fn canonical_index_matches_selected_sidecar_and_refuses_different_record_row_or_kind() {
    let mut initial = [0; 8192];
    let size = envelope(&[5; 16], &[raw_field()], &[], &mut initial);
    let mut scratch = [c::TypedFieldRecord::default(); 2];
    let mut fields = [c::TypedFieldRecord::default(); 2];
    let initial_resolver =
        c::TypedPayloadResolver::new(&initial[..size], &[], &mut scratch, &mut fields).unwrap();
    let original_binding =
        ResolvedFieldPayloadBinding::new(physical(), &initial_resolver, [1; 16]).unwrap();
    let original = record(original_binding, 21);
    let mut packed = [0; 16 + MEASUREMENT_INDEX_RECORD_SIZE];
    let index =
        pack_sensing_index_field([1; 16], 10, KIND, &[5; 16], &[original], &mut packed).unwrap();
    let mut bytes = [0; 8192];
    let size = envelope(
        &packed[..index.field_section_bytes],
        &[raw_field(), index.index_field],
        &[KIND],
        &mut bytes,
    );
    let mut scratch = [c::TypedFieldRecord::default(); 2];
    let mut fields = [c::TypedFieldRecord::default(); 2];
    let resolver =
        c::TypedPayloadResolver::new(&bytes[..size], &[KIND], &mut scratch, &mut fields).unwrap();
    let binding = ResolvedFieldPayloadBinding::new(physical(), &resolver, [1; 16]).unwrap();
    let selected = Selection {
        field_id: 10,
        row: 0,
        kind: KIND,
    };
    assert!(selected.verify(&resolver, binding, &original).is_ok());
    assert!(selected
        .verify(&resolver, binding, &record(binding, 22))
        .is_err());
    assert!(Selection { row: 1, ..selected }
        .verify(&resolver, binding, &original)
        .is_err());
    assert!(Selection {
        kind: KIND + 1,
        ..selected
    }
    .verify(&resolver, binding, &original)
    .is_err());
    assert!(Selection {
        field_id: 9,
        ..selected
    }
    .verify(&resolver, binding, &original)
    .is_err());
    let different_container =
        ResolvedFieldPayloadBinding::new(physical(), &resolver, [2; 16]).unwrap();
    assert!(selected
        .verify(&resolver, different_container, &original)
        .is_err());
}
