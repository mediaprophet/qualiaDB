use super::*;
fn field(key: u16, value: FieldValue<'_>) -> FieldInput<'_> {
    FieldInput {
        key,
        flags: 1,
        value,
    }
}
fn block<'a>(id: u32, fields: &'a [FieldInput<'a>]) -> BlockInput<'a> {
    BlockInput {
        id,
        schema_type: 205,
        revision: 1,
        flags: 1,
        fields,
    }
}
#[test]
fn independently_authored_tlv_bytes_and_tail_are_exact() {
    let values = [0x11223344u32, 0xaabbccdd];
    let fields = [
        field(1, FieldValue::U32(&values)),
        FieldInput {
            key: 3,
            flags: 0,
            value: FieldValue::Utf8("hi"),
        },
    ];
    let blocks = [block(7, &fields)];
    let input = MetadataInput { blocks: &blocks };
    let expected: [u8; 64] = [
        7, 0, 0, 0, 205, 0, 1, 0, 48, 0, 0, 0, 1, 0, 0, 0, 1, 0, 1, 1, 2, 0, 0, 0, 8, 0, 0, 0, 0,
        0, 0, 0, 0x44, 0x33, 0x22, 0x11, 0xdd, 0xcc, 0xbb, 0xaa, 3, 0, 6, 0, 2, 0, 0, 0, 2, 0, 0,
        0, 0, 0, 0, 0, b'h', b'i', 0, 0, 0, 0, 0, 0,
    ];
    assert_eq!(
        input.encoded_len(Limits::default()).unwrap(),
        expected.len()
    );
    let mut out = [0xa5; 75];
    assert_eq!(
        input.encode_into(&mut out, Limits::default()).unwrap(),
        expected
    );
    assert_eq!(&out[64..], &[0xa5; 11]);
    metadata::validate(&out[..64], Limits::default()).unwrap();
}
#[test]
fn all_primitive_slices_use_explicit_little_endian_and_preserve_float_bits() {
    let u32s = [0x01020304u32];
    let u64s = [0x0102030405060708u64];
    let i64s = [-2i64];
    let f32s = [-0.0f32];
    let f64s = [1.0f64];
    let refs = [0x12345678u32];
    let u16s = [0x0102u16];
    let i32s = [-2i32];
    let fields = [
        field(1, FieldValue::U32(&u32s)),
        field(2, FieldValue::U64(&u64s)),
        field(3, FieldValue::I64(&i64s)),
        field(4, FieldValue::F32(&f32s)),
        field(5, FieldValue::F64(&f64s)),
        field(6, FieldValue::Utf8("é")),
        field(7, FieldValue::Bytes(&[0, 255])),
        field(8, FieldValue::MetadataRefs(&refs)),
        field(9, FieldValue::EntryRefs(&refs)),
        field(10, FieldValue::U16(&u16s)),
        field(11, FieldValue::I32(&i32s)),
    ];
    let blocks = [block(1, &fields)];
    let mut out = [0; 512];
    let bytes = MetadataInput { blocks: &blocks }
        .encode_into(&mut out, Limits::default())
        .unwrap();
    metadata::validate(bytes, Limits::default()).unwrap();
    let expected: &[&[u8]] = &[
        &[4, 3, 2, 1],
        &[8, 7, 6, 5, 4, 3, 2, 1],
        &[254, 255, 255, 255, 255, 255, 255, 255],
        &[0, 0, 0, 128],
        &[0, 0, 0, 0, 0, 0, 240, 63],
        &[195, 169],
        &[0, 255],
        &[120, 86, 52, 18],
        &[120, 86, 52, 18],
        &[2, 1],
        &[254, 255, 255, 255],
    ];
    for (i, f) in Blocks::new(bytes).next().unwrap().fields().enumerate() {
        assert_eq!(usize::from(f.type_tag()), i + 1);
        assert_eq!(f.value(), expected[i]);
    }
}
#[test]
fn every_existing_independent_candidate_metadata_fixture_reencodes_exactly() {
    const FIXTURES: &[(&[u8], Family)] = &[
        (
            include_bytes!("fixtures/candidate/qac_candidate-opaque.qac"),
            Family::QacV1Candidate,
        ),
        (
            include_bytes!("fixtures/candidate/qac_candidate-metadata-only.qac"),
            Family::QacV1Candidate,
        ),
        (
            include_bytes!("fixtures/candidate/qac_candidate-aligned4096.qac"),
            Family::QacV1Candidate,
        ),
        (
            include_bytes!("fixtures/candidate/qac_candidate-pcm-signed-zero.qac"),
            Family::QacV1Candidate,
        ),
        (
            include_bytes!("fixtures/candidate/qac_candidate-two-entries.qac"),
            Family::QacV1Candidate,
        ),
        (
            include_bytes!("fixtures/candidate/emf_candidate-opaque.emf"),
            Family::EmfV4Candidate,
        ),
        (
            include_bytes!("fixtures/candidate/emf_candidate-metadata-only.emf"),
            Family::EmfV4Candidate,
        ),
        (
            include_bytes!("fixtures/candidate/emf_candidate-aligned4096.emf"),
            Family::EmfV4Candidate,
        ),
        (
            include_bytes!("fixtures/candidate/emf_candidate-two-entries.emf"),
            Family::EmfV4Candidate,
        ),
    ];
    for (bytes, family) in FIXTURES {
        let view = StructuralView::open(bytes, *family, Limits::default()).unwrap();
        let fields: Vec<Vec<FieldInput<'_>>> = view
            .blocks()
            .map(|b| {
                b.fields()
                    .map(|f| FieldInput {
                        key: f.key(),
                        flags: f.flags(),
                        value: FieldValue::Raw {
                            type_tag: f.type_tag(),
                            element_count: f.element_count(),
                            bytes: f.value(),
                        },
                    })
                    .collect()
            })
            .collect();
        let blocks: Vec<_> = view
            .blocks()
            .zip(&fields)
            .map(|(b, fields)| BlockInput {
                id: b.id(),
                schema_type: b.schema_type(),
                revision: b.revision(),
                flags: b.flags(),
                fields,
            })
            .collect();
        let input = MetadataInput { blocks: &blocks };
        assert_eq!(
            input.encoded_len(Limits::default()).unwrap(),
            view.metadata_bytes().len()
        );
        let mut out = vec![0xa5; view.metadata_bytes().len() + 3];
        let result = input.encode_into(&mut out, Limits::default()).unwrap();
        assert_eq!(result, view.metadata_bytes());
        assert_eq!(&out[view.metadata_bytes().len()..], &[0xa5; 3]);
    }
}
fn rejects(input: MetadataInput<'_>, limits: Limits, expected: Error) {
    let mut out = [0xa5; 256];
    assert_eq!(input.encoded_len(limits), Err(expected));
    assert_eq!(input.encode_into(&mut out, limits).unwrap_err(), expected);
    assert!(out.iter().all(|b| *b == 0xa5));
}
#[test]
fn malformed_primitive_and_late_nonfinite_input_leave_output_unchanged() {
    let bad_values = [
        (FieldValue::F32(&[f32::INFINITY]), Error::Numeric),
        (FieldValue::F64(&[f64::NAN]), Error::Numeric),
        (FieldValue::Utf8("a\0b"), Error::Metadata),
        (
            FieldValue::Raw {
                type_tag: 6,
                element_count: 1,
                bytes: &[255],
            },
            Error::Metadata,
        ),
        (
            FieldValue::Raw {
                type_tag: 4,
                element_count: 1,
                bytes: &[0, 0, 128, 127],
            },
            Error::Numeric,
        ),
        (
            FieldValue::Raw {
                type_tag: 1,
                element_count: 2,
                bytes: &[0; 4],
            },
            Error::Numeric,
        ),
    ];
    for (value, error) in bad_values {
        let fields = [field(1, FieldValue::U32(&[7])), field(2, value)];
        let blocks = [block(1, &fields)];
        rejects(MetadataInput { blocks: &blocks }, Limits::default(), error);
    }
}
#[test]
fn canonical_order_flags_limits_and_short_output_are_atomic() {
    let fields = [
        field(1, FieldValue::Utf8("abc")),
        field(2, FieldValue::U32(&[9])),
    ];
    let good = block(1, &fields);
    for id in [0, 1] {
        let blocks = if id == 0 {
            [block(0, &fields), block(2, &fields)]
        } else {
            [block(1, &fields), block(1, &fields)]
        };
        rejects(
            MetadataInput { blocks: &blocks },
            Limits::default(),
            Error::Metadata,
        );
    }
    let blocks = [block(2, &fields), good];
    rejects(
        MetadataInput { blocks: &blocks },
        Limits::default(),
        Error::Metadata,
    );
    let mut bad_block = good;
    bad_block.flags = 2;
    rejects(
        MetadataInput {
            blocks: &[bad_block],
        },
        Limits::default(),
        Error::Metadata,
    );
    for keys in [[0, 1], [2, 1], [1, 1]] {
        let bad = [
            field(keys[0], FieldValue::U32(&[0])),
            field(keys[1], FieldValue::U32(&[0])),
        ];
        rejects(
            MetadataInput {
                blocks: &[block(1, &bad)],
            },
            Limits::default(),
            Error::Metadata,
        );
    }
    let bad = [FieldInput {
        flags: 2,
        ..fields[0]
    }];
    rejects(
        MetadataInput {
            blocks: &[block(1, &bad)],
        },
        Limits::default(),
        Error::Metadata,
    );
    let blocks = [good];
    let input = MetadataInput { blocks: &blocks };
    for limits in [
        Limits {
            blocks: 0,
            ..Default::default()
        },
        Limits {
            fields_per_block: 1,
            ..Default::default()
        },
        Limits {
            metadata_bytes: 1,
            ..Default::default()
        },
        Limits {
            string_bytes: 2,
            ..Default::default()
        },
    ] {
        rejects(input, limits, Error::Limit);
    }
    let mut small = [0xa5; 16];
    assert_eq!(
        input
            .encode_into(&mut small, Limits::default())
            .unwrap_err(),
        Error::Extent
    );
    assert_eq!(small, [0xa5; 16]);
}
#[test]
fn unknown_tags_and_empty_sequences_are_structural_only() {
    let fields = [
        field(
            1,
            FieldValue::Raw {
                type_tag: 240,
                element_count: 99,
                bytes: &[1, 2, 3],
            },
        ),
        field(2, FieldValue::U64(&[])),
        field(3, FieldValue::Utf8("")),
    ];
    let blocks = [BlockInput {
        schema_type: 0,
        revision: 0,
        ..block(1, &fields)
    }];
    let mut out = [0; 128];
    let result = MetadataInput { blocks: &blocks }
        .encode_into(&mut out, Limits::default())
        .unwrap();
    metadata::validate(result, Limits::default()).unwrap();
    let b = Blocks::new(result).next().unwrap();
    assert_eq!(b.fields().next().unwrap().element_count(), 99);
    let mut tail = [0xa5; 8];
    assert!(MetadataInput { blocks: &[] }
        .encode_into(&mut tail, Limits::default())
        .unwrap()
        .is_empty());
    assert_eq!(tail, [0xa5; 8]);
}
