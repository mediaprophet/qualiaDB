use super::*;
#[test]
fn unknown_profiles_and_unqualified_physics_fail_closed() {
    let p = field_payload(0);
    for (key, value) in [
        (6, s(6, "legacy_display_field")),
        (7, s(7, "hilbert3")),
    ] {
        let mut m = metadata(0, false);
        replace(&mut m, 4, key, value);
        assert!(matches!(
            check(0, &p, 1, false, &m),
            Err(Error::UnsupportedProfile)
        ));
    }
    for (key, value) in [
        (13, n32(13, 8, 3)),
        (14, n32(14, 8, 3)),
        (30, n32(30, 8, 3)),
    ] {
        let mut m = metadata(0, false);
        replace(&mut m, 4, key, value);
        assert!(matches!(
            check(0, &p, 1, false, &m),
            Err(Error::Shared(semantics::Error::ReferenceSchema))
        ));
    }
    let mut m = metadata(0, false);
    replace(&mut m, 5, 12, s(12, "urn:unimplemented:constitutive"));
    assert_eq!(check(0, &p, 1, false, &m), Err(Error::Descriptor));
    let mut m = metadata(3, false);
    replace(&mut m, 6, 8, s(8, "legacy_display_400_700nm"));
    assert_eq!(
        check(3, &spectrum_payload(0), 1, false, &m),
        Err(Error::UnsupportedProfile)
    );
}
#[test]
fn shapes_frames_clocks_and_quantity_units_are_bound() {
    let p = field_payload(-2);
    let mut m = metadata(0, true);
    replace(&mut m, 5, 13, n64(13, 2, 1000));
    assert_eq!(check(0, &p, 1, true, &m), Err(Error::Time));
    let mut m = metadata(0, true);
    replace(&mut m, 4, 11, n64(11, 3, 3));
    assert_eq!(check(0, &p, 1, true, &m), Err(Error::Time));
    let mut m = metadata(0, false);
    replace(&mut m, 5, 7, s(7, "spectral_irradiance"));
    assert_eq!(
        check(0, &field_payload(0), 1, false, &m),
        Err(Error::Descriptor)
    );
    let mut m = metadata(3, false);
    replace(&mut m, 4, 19, n32(19, 1, 3));
    assert!(check(3, &spectrum_payload(0), 1, false, &m).is_err());
    let mut p = field_payload(0);
    p.resize(64, 0);
    assert_eq!(
        check(0, &p, 1, false, &metadata(0, false)),
        Err(Error::Shape)
    );
}
#[test]
fn whole_payload_digest_and_budgets_precede_typed_access() {
    let p = field_payload(0);
    let mut encoded = encode(&metadata(0, false), &[row(0, &p, 1, false)]);
    let view =
        StructuralView::open(&encoded, Family::EmfV4Candidate, StoreLimits::default()).unwrap();
    let at = view.entry(1).unwrap().payload_offset() as usize;
    encoded[at] ^= 1;
    let view =
        StructuralView::open(&encoded, Family::EmfV4Candidate, StoreLimits::default()).unwrap();
    assert_eq!(
        admit(&view).unwrap_err(),
        Error::Storage(super::super::super::field_envelope::Error::PayloadDigest)
    );
    let encoded = encode(&metadata(0, false), &[row(0, &p, 1, false)]);
    let view =
        StructuralView::open(&encoded, Family::EmfV4Candidate, StoreLimits::default()).unwrap();
    let mut m = [semantics::NodeState::default(); 8];
    let mut e = [semantics::NodeState::default(); 1];
    for limits in [
        Limits {
            records: 0,
            ..Limits::default()
        },
        Limits {
            payload_work: 0,
            ..Limits::default()
        },
    ] {
        assert_eq!(
            admit_chunk(
                &view,
                1,
                &mut Scratch {
                    metadata: &mut m,
                    entries: &mut e
                },
                limits
            )
            .unwrap_err(),
            Error::Limit
        );
    }
}

#[test]
fn explicit_point_shapes_frame_identity_and_tick_overflow_are_checked() {
    let p = field_payload(0);
    let mut m = metadata(0, false);
    let mut frame = m[1].clone();
    frame.id = 7;
    m.push(frame);
    replace(&mut m, 4, 4, n32(4, 8, 7));
    assert_eq!(check(0, &p, 1, false, &m), Err(Error::Descriptor));
    let mut m = metadata(0, false);
    replace(
        &mut m,
        4,
        8,
        f(
            8,
            2,
            3,
            &[2u64, 1, 1]
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .collect::<Vec<_>>(),
        ),
    );
    assert_eq!(check(0, &p, 1, false, &m), Err(Error::UnsupportedProfile));
    let mut p = field_payload(i64::MAX);
    p.extend_from_slice(&field_payload(0));
    let m = metadata(0, true);
    let r = EntryInput {
        start_tick: i64::MAX,
        ..row(0, &p, 2, true)
    };
    let encoded = encode(&m, &[r]);
    let view =
        StructuralView::open(&encoded, Family::EmfV4Candidate, StoreLimits::default()).unwrap();
    assert_eq!(admit(&view).unwrap_err(), Error::Time);
    let encoded = encode(&metadata(0, false), &[row(0, &field_payload(0), 1, false)]);
    let mut unaligned = vec![0];
    unaligned.extend_from_slice(&encoded);
    let view = StructuralView::open(
        &unaligned[1..],
        Family::EmfV4Candidate,
        StoreLimits::default(),
    )
    .unwrap();
    assert!(admit(&view).is_ok());
}

#[test]
fn admitted_metadata_survives_nonidentity_frames_and_both_stokes_units() {
    for units in ["W m^-2", "W sr^-1 m^-2"] {
        let mut m = metadata(1, false);
        replace(
            &mut m,
            2,
            4,
            numbers(
                4,
                &[
                    0.0,
                    0.0,
                    std::f64::consts::FRAC_1_SQRT_2,
                    std::f64::consts::FRAC_1_SQRT_2,
                ],
            ),
        );
        replace(&mut m, 5, 8, s(8, units));
        replace(&mut m, 6, 3, s(3, units));
        let mut payload = ray_payload();
        put(&mut payload, 0, 2.0);
        put(&mut payload, 8, 3.0);
        put(&mut payload, 16, 4.0);
        put(&mut payload, 24, 1.0);
        put(&mut payload, 40, 0.0);
        let encoded = encode(&m, &[row(1, &payload, 1, false)]);
        let view =
            StructuralView::open(&encoded, Family::EmfV4Candidate, StoreLimits::default()).unwrap();
        let chunk = admit(&view).unwrap();
        assert_eq!(chunk.entry_index(), 1);
        assert_eq!(chunk.descriptor_id(), 5);
        assert_eq!(chunk.frame_id(), 2);
        assert_eq!(chunk.clock_id(), 1);
        assert_eq!(chunk.profile(), RAY_PROFILE);
        assert_eq!(chunk.quantity(), "polarimetric_ray");
        assert_eq!(chunk.units(), units);
        assert_eq!(chunk.model_uri(), RAY_MODEL_URI);
        assert_eq!(chunk.start_tick(), None);
        assert_eq!(chunk.temporal_step_ticks(), None);
        assert_eq!(chunk.verified_stored_bytes(), payload);
        let ray = chunk.ray(0).unwrap().unwrap();
        assert_eq!(ray.position_m(), [2.0, 3.0, 4.0]);
        assert_eq!(ray.direction(), [1.0, 0.0, 0.0]);
    }
    let payload = field_payload(-2);
    let encoded = encode(&metadata(0, true), &[row(0, &payload, 1, true)]);
    let view =
        StructuralView::open(&encoded, Family::EmfV4Candidate, StoreLimits::default()).unwrap();
    let chunk = admit(&view).unwrap();
    assert_eq!(chunk.start_tick(), Some(-2));
    assert_eq!(chunk.temporal_step_ticks(), Some(2));
    assert_eq!(chunk.units(), "mixed:mode-registry");
    assert_eq!(chunk.model_uri(), FIELD_MODEL_URI);
}
