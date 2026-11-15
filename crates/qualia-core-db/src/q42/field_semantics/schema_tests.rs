use super::*;
#[test]
fn shared_base_temporal_grid_and_instant_admit() {
    assert_eq!(result(&good(), &[row()]), Ok(()));
    let mut blocks = good();
    let desc = &mut blocks[3];
    desc.fields.insert(0, n32(1, 8, 1));
    desc.fields.push(n64(13, 2, 48000));
    desc.fields.push(n64(14, 2, 1));
    assert_eq!(
        result(
            &blocks,
            &[EntryInput {
                flags: 6,
                stream_id: 1,
                start_tick: -2,
                time_count: 1,
                ..row()
            }]
        ),
        Ok(())
    );
    let mut blocks = good();
    change(&mut blocks, 2, 7).value = 1u32.to_le_bytes().to_vec();
    assert_eq!(
        result(
            &blocks,
            &[EntryInput {
                spatial_key: 0,
                ..row()
            }]
        ),
        Ok(())
    );
    assert_eq!(result(&blocks, &[row()]), Err(Error::Frame));
    let preserved =
        include_bytes!("../field_envelope/fixtures/candidate/qac_candidate-metadata-only.qac");
    assert_eq!(
        result_with(preserved, &SharedOnly, Limits::default()),
        Ok(())
    );
}
#[test]
fn exact_rules_required_critical_rational_and_frame_bounds() {
    let mut b = good();
    change(&mut b, 1, 1).critical = false;
    assert_eq!(result(&b, &[row()]), Err(Error::RequiredCritical));
    let mut b = good();
    change(&mut b, 1, 1).tag = 11;
    assert_eq!(result(&b, &[row()]), Err(Error::FieldType));
    let mut b = good();
    b[0].fields.remove(2);
    assert_eq!(result(&b, &[row()]), Err(Error::MissingField));
    for (schema, key, value) in [
        (1, 4, 0u64),
        (1, 4, 1_000_000_000_000_001),
        (1, 5, 2),
        (5, 3, 2),
        (5, 4, 0),
        (5, 4, 1_000_000_000_001),
    ] {
        let mut b = good();
        change(&mut b, schema, key).value = value.to_le_bytes().to_vec();
        assert_eq!(result(&b, &[row()]), Err(Error::Time));
    }
    let mut b = good();
    change(&mut b, 1, 3).value = 1_000_000_000u32.to_le_bytes().to_vec();
    assert_eq!(result(&b, &[row()]), Err(Error::Time));
    let mut b = good();
    change(&mut b, 2, 4).value = [0.0f64, 0.0, 0.0, 1.01]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect();
    assert_eq!(result(&b, &[row()]), Err(Error::Frame));
    let mut b = good();
    change(&mut b, 2, 4).value = [0.0f64, 0.0, 0.0, 1.0 + 2f64.powi(-43)]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect();
    assert_eq!(result(&b, &[row()]), Ok(()));
    let mut b = good();
    b[1].fields.retain(|f| f.key != 5);
    assert_eq!(result(&b, &[row()]), Err(Error::Frame));
    let mut b = good();
    change(&mut b, 3, 12).value = "urn:bad\u{0085}".as_bytes().to_vec();
    change(&mut b, 3, 12).count = 9;
    assert_eq!(result(&b, &[row()]), Err(Error::Value));
}
#[test]
fn typed_references_and_directory_coherence_fail_closed() {
    let mut b = good();
    change(&mut b, 3, 3).value = 1u32.to_le_bytes().to_vec();
    assert_eq!(result(&b, &[row()]), Err(Error::ReferenceSchema));
    let mut b = good();
    change(&mut b, 5, 1).value = 0u32.to_le_bytes().to_vec();
    assert_eq!(result(&b, &[row()]), Err(Error::Reference));
    assert_eq!(
        result(
            &good(),
            &[EntryInput {
                descriptor_id: 2,
                ..row()
            }]
        ),
        Err(Error::ReferenceSchema)
    );
    assert_eq!(
        result(
            &good(),
            &[EntryInput {
                flags: 0,
                spatial_key: 0,
                frame_id: 0,
                ..row()
            }]
        ),
        Err(Error::Frame)
    );
    assert_eq!(
        result(
            &good(),
            &[EntryInput {
                spatial_key: 24,
                ..row()
            }]
        ),
        Err(Error::Frame)
    );
    assert_eq!(
        result(
            &good(),
            &[EntryInput {
                flags: 6,
                stream_id: 1,
                time_count: 1,
                ..row()
            }]
        ),
        Err(Error::MissingField)
    );
    let mut b = good();
    b[3].fields.insert(0, n32(1, 8, 1));
    assert_eq!(result(&b, &[row()]), Err(Error::Time));
    let mut b = good();
    b[3].fields.push(f(18, 8, 1, 6u32.to_le_bytes().to_vec()));
    let mut unknown = block(6, 777, vec![]);
    unknown.critical = false;
    b.push(unknown);
    assert_eq!(result(&b, &[row()]), Err(Error::UnsupportedDomain));
    let mut b = good();
    b[3].fields.insert(2, n32(4, 8, 3));
    assert_eq!(result(&b, &[row()]), Err(Error::UnsupportedDomain));
}
#[test]
fn unknown_critical_information_and_unimplemented_domain_hooks() {
    let mut b = good();
    b[0].revision = 2;
    assert_eq!(result(&b, &[row()]), Err(Error::UnknownCriticalSchema));
    let mut b = good();
    b[0].fields.push(n32(99, 1, 0));
    assert_eq!(result(&b, &[row()]), Err(Error::UnknownCriticalField));
    let mut b = good();
    b[0].fields.push(f(99, 250, 3, b"raw".to_vec()));
    assert_eq!(result(&b, &[row()]), Err(Error::UnknownCriticalType));
    let mut b = good();
    let mut unknown = block(6, 777, vec![]);
    unknown.critical = false;
    b.push(unknown);
    assert_eq!(result(&b, &[row()]), Ok(()));
    let fixture = include_bytes!("../field_envelope/fixtures/candidate/qac_candidate-opaque.qac");
    assert_eq!(
        result_with(fixture, &SharedOnly, Limits::default()),
        Err(Error::UnknownCriticalSchema)
    );
    struct Domain;
    impl DomainRegistry for Domain {
        fn rules(&self, schema: u16, revision: u16) -> Option<&'static [FieldRule]> {
            static RULES: &[FieldRule] = &[
                FieldRule {
                    key: 1,
                    type_tag: 6,
                    count: None,
                    required: true,
                },
                FieldRule {
                    key: 2,
                    type_tag: 6,
                    count: None,
                    required: true,
                },
                FieldRule {
                    key: 14,
                    type_tag: 2,
                    count: Some(1),
                    required: true,
                },
            ];
            if schema == 205 && revision == 1 {
                Some(RULES)
            } else {
                None
            }
        }
        fn validate_block(&self, block: Block<'_>, _: &Context<'_, '_>) -> Result<(), Error> {
            if scalar64(block, 14)? != 11 {
                return Err(Error::Value);
            }
            Ok(())
        }
    }
    assert_eq!(result_with(fixture, &Domain, Limits::default()), Ok(()));
}

#[test]
fn exact_array_counts_and_conditional_critical_fields() {
    let mut b = good();
    let origin = change(&mut b, 2, 3);
    origin.count = 2;
    origin.value.truncate(16);
    assert_eq!(result(&b, &[row()]), Err(Error::FieldType));
    let mut b = good();
    change(&mut b, 2, 5).critical = false;
    assert_eq!(result(&b, &[row()]), Err(Error::RequiredCritical));
    let mut b = good();
    b[3].fields.insert(0, n32(1, 8, 1));
    b[3].fields[0].critical = false;
    b[3].fields.push(n64(13, 2, 48000));
    b[3].fields.push(n64(14, 2, 1));
    assert_eq!(
        result(
            &b,
            &[EntryInput {
                flags: 6,
                stream_id: 1,
                time_count: 1,
                ..row()
            }]
        ),
        Err(Error::RequiredCritical)
    );
    let mut b = good();
    let mut extra = block(6, 777, vec![n32(1, 1, 0)]);
    extra.critical = false;
    b.push(extra);
    assert_eq!(result(&b, &[row()]), Err(Error::UnknownCriticalField));
}
