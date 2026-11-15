use super::*;

fn sample_identity() -> ManifoldIdentityV2 {
    ManifoldIdentityV2 {
        entity_id: StableEntityId(*b"entity-q42-00001"),
        profile_id: 7,
        domain_id: 11,
        coordinate_convention_id: 13,
        coordinates: [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 0.25, 0.5, 0.75],
        external_digest: Some(IntegrityIndexDigest(*b"digest-index-001")),
    }
}

fn sample_fields() -> [TypedFieldRecord; 3] {
    [
        TypedFieldRecord {
            field_id: 10,
            kind: FIELD_KIND_SCALAR,
            flags: 0,
            unit_id: 101,
            payload_section_id: 4,
            payload_offset: 0,
            payload_length: 64,
        },
        TypedFieldRecord {
            field_id: 20,
            kind: FIELD_KIND_VECTOR,
            flags: 0,
            unit_id: 202,
            payload_section_id: 5,
            payload_offset: 16,
            payload_length: 96,
        },
        TypedFieldRecord {
            field_id: 30,
            kind: FIELD_KIND_ELECTROMAGNETIC,
            flags: 0,
            unit_id: 303,
            payload_section_id: 6,
            payload_offset: 0,
            payload_length: 256,
        },
    ]
}

#[test]
fn identity_and_typed_fields_round_trip_deterministically() {
    let identity = sample_identity();
    let fields = sample_fields();
    let size = encoded_len(fields.len()).unwrap();
    let mut first = [0u8; MANIFOLD_IDENTITY_V2_MAX_BYTES];
    let mut second = [0u8; MANIFOLD_IDENTITY_V2_MAX_BYTES];
    let written = encode_manifold_identity_v2(&identity, &fields, &mut first).unwrap();
    let written_again = encode_manifold_identity_v2(&identity, &fields, &mut second).unwrap();
    assert_eq!(written, size);
    assert_eq!(written_again, written);
    assert_eq!(&first[..written], &second[..written]);

    let mut decoded_fields = [TypedFieldRecord::default(); 3];
    let (decoded_identity, count) =
        decode_manifold_identity_v2(&first[..written], &[], &mut decoded_fields).unwrap();
    assert_eq!(count, fields.len());
    assert_eq!(decoded_identity, identity);
    assert_eq!(decoded_fields, fields);
    assert_eq!(
        MANIFOLD_IDENTITY_V2_AXIS_ORDER,
        ["q", "v", "w", "x", "y", "z", "t", "α", "μ", "σ"]
    );
}

#[test]
fn stable_identity_is_independent_of_optional_digest() {
    let identity = sample_identity();
    let digest = identity.external_digest.unwrap();
    assert_ne!(identity.entity_id.0, digest.0);
    let mut replay_state = identity;
    replay_state.coordinates[3] += 1.0;
    replay_state.external_digest = None;
    assert!(identity.resolves_same_entity(&replay_state));
    assert!(!identity.has_same_manifold_address(&replay_state));

    let mut unrelated_entity = identity;
    unrelated_entity.entity_id = StableEntityId(*b"entity-q42-00002");
    assert!(!identity.resolves_same_entity(&unrelated_entity));

    let mut bytes = [0u8; MANIFOLD_IDENTITY_V2_MAX_BYTES];
    let written = encode_manifold_identity_v2(&identity, &[], &mut bytes).unwrap();
    let mut no_fields = [];
    let (decoded, count) =
        decode_manifold_identity_v2(&bytes[..written], &[], &mut no_fields).unwrap();
    assert_eq!(count, 0);
    assert_eq!(decoded.entity_id, identity.entity_id);
    assert_eq!(decoded.external_digest, Some(digest));
}

#[test]
fn emf_is_one_typed_field_and_registered_extensions_are_supported() {
    const FIELD_KIND_CUSTOM_SPECTRAL: u16 = 0x8001;
    let identity = sample_identity();
    let fields = [
        TypedFieldRecord {
            field_id: 1,
            kind: FIELD_KIND_ELECTROMAGNETIC,
            flags: 0,
            unit_id: 8,
            payload_section_id: 2,
            payload_offset: 0,
            payload_length: 32,
        },
        TypedFieldRecord {
            field_id: 2,
            kind: FIELD_KIND_CUSTOM_SPECTRAL,
            flags: 0,
            unit_id: 9,
            payload_section_id: 3,
            payload_offset: 0,
            payload_length: 48,
        },
    ];
    let mut bytes = [0u8; MANIFOLD_IDENTITY_V2_MAX_BYTES];
    assert_eq!(
        encode_manifold_identity_v2(&identity, &fields, &mut bytes),
        Err(ManifestV2Error::UnsupportedFieldKind(
            FIELD_KIND_CUSTOM_SPECTRAL
        ))
    );
    let written = encode_manifold_identity_v2_with_extensions(
        &identity,
        &fields,
        &[FIELD_KIND_CUSTOM_SPECTRAL],
        &mut bytes,
    )
    .unwrap();
    let mut decoded = [TypedFieldRecord::default(); 2];
    assert_eq!(
        decode_manifold_identity_v2(&bytes[..written], &[], &mut decoded),
        Err(ManifestV2Error::UnsupportedFieldKind(
            FIELD_KIND_CUSTOM_SPECTRAL
        ))
    );
    let (_, count) = decode_manifold_identity_v2(
        &bytes[..written],
        &[FIELD_KIND_CUSTOM_SPECTRAL],
        &mut decoded,
    )
    .unwrap();
    assert_eq!(count, 2);
    assert_eq!(decoded[0].kind, FIELD_KIND_ELECTROMAGNETIC);
    assert_eq!(decoded[1].kind, FIELD_KIND_CUSTOM_SPECTRAL);
}

#[test]
fn malformed_or_truncated_sections_fail_closed_without_partial_output() {
    let identity = sample_identity();
    let fields = sample_fields();
    let mut bytes = [0u8; MANIFOLD_IDENTITY_V2_MAX_BYTES];
    let written = encode_manifold_identity_v2(&identity, &fields, &mut bytes).unwrap();
    let mut decoded = [TypedFieldRecord {
        field_id: 999,
        ..TypedFieldRecord::default()
    }; 3];
    assert!(matches!(
        decode_manifold_identity_v2(&bytes[..written - 1], &[], &mut decoded),
        Err(ManifestV2Error::BadTotalLength { .. })
    ));
    assert!(decoded.iter().all(|field| field.field_id == 999));

    let mut bad_version = bytes;
    put_u16(&mut bad_version, 4, 3);
    assert_eq!(
        decode_manifold_identity_v2(&bad_version[..written], &[], &mut decoded),
        Err(ManifestV2Error::UnsupportedVersion(3))
    );

    let mut bad_reserved = bytes;
    bad_reserved[MANIFOLD_IDENTITY_V2_HEADER_SIZE + 24] = 1;
    assert_eq!(
        decode_manifold_identity_v2(&bad_reserved[..written], &[], &mut decoded),
        Err(ManifestV2Error::NonZeroReserved { field_index: 0 })
    );
}

#[test]
fn bounds_and_invalid_input_do_not_modify_output() {
    let identity = sample_identity();
    let fields = sample_fields();
    let mut too_small = [0xA5u8; 32];
    assert!(matches!(
        encode_manifold_identity_v2(&identity, &fields, &mut too_small),
        Err(ManifestV2Error::OutputTooSmall { .. })
    ));
    assert!(too_small.iter().all(|byte| *byte == 0xA5));
    assert!(matches!(
        encoded_len(MANIFOLD_IDENTITY_V2_MAX_FIELDS + 1),
        Err(ManifestV2Error::TooManyFields { .. })
    ));

    let mut bad_identity = identity;
    bad_identity.coordinates[8] = f32::NAN;
    let mut output = [0x5Au8; MANIFOLD_IDENTITY_V2_MAX_BYTES];
    assert_eq!(
        encode_manifold_identity_v2(&bad_identity, &fields, &mut output),
        Err(ManifestV2Error::NonFiniteCoordinate { axis: 8 })
    );
    assert!(output.iter().all(|byte| *byte == 0x5A));
}

#[test]
fn signed_zero_has_one_canonical_address_encoding() {
    let mut identity = sample_identity();
    identity.coordinates[4] = -0.0;
    let mut bytes = [0u8; MANIFOLD_IDENTITY_V2_MAX_BYTES];
    let written = encode_manifold_identity_v2(&identity, &[], &mut bytes).unwrap();
    assert_eq!(&bytes[56 + 4 * 4..56 + 5 * 4], &0u32.to_le_bytes());

    let mut decoded_fields = [];
    let (decoded, _) =
        decode_manifold_identity_v2(&bytes[..written], &[], &mut decoded_fields).unwrap();
    assert_eq!(decoded.coordinates[4].to_bits(), 0);
    assert!(identity.has_same_manifold_address(&decoded));

    let mut noncanonical = bytes;
    noncanonical[56 + 4 * 4..56 + 5 * 4].copy_from_slice(&(-0.0f32).to_bits().to_le_bytes());
    assert_eq!(
        decode_manifold_identity_v2(&noncanonical[..written], &[], &mut decoded_fields),
        Err(ManifestV2Error::NonCanonicalCoordinate { axis: 4 })
    );
}
