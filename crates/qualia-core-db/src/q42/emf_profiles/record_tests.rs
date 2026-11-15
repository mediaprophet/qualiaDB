use super::*;
#[test]
fn static_fields_keep_absent_carrier_and_original_signed_bits() {
    let mut payload = field_payload(0);
    put(&mut payload, 8, -0.0);
    let encoded = encode(&metadata(0, false), &[row(0, &payload, 1, false)]);
    let view =
        StructuralView::open(&encoded, Family::EmfV4Candidate, StoreLimits::default()).unwrap();
    let chunk = admit(&view).unwrap();
    let record = chunk.field(0).unwrap().unwrap();
    assert_eq!(record.electric_v_per_m()[0], 1.0);
    assert_eq!(record.electric_v_per_m()[1].to_bits(), (-0.0f64).to_bits());
    assert_eq!(record.carrier_hz(), None);
    assert_eq!(record.stored_poynting_w_per_m2(), None);
    assert_eq!(record.confidence(), None);
    assert_eq!(view.verify_chunk(1).unwrap(), payload);
    assert_eq!(chunk.field(1).unwrap_err(), Error::Index);
    assert_eq!(chunk.ray(0).unwrap_err(), Error::WrongMode);
}
#[test]
fn field_optional_lanes_flags_and_temporal_order_are_exact() {
    let mut payload = field_payload(-2);
    payload.extend_from_slice(&field_payload(0));
    assert!(check(0, &payload, 2, true, &metadata(0, true)).is_ok());
    payload[200..208].copy_from_slice(&1i64.to_le_bytes());
    assert_eq!(
        check(0, &payload, 2, true, &metadata(0, true)),
        Err(Error::Time)
    );
    for flags in [2u32, 16] {
        let mut p = field_payload(0);
        p[120..124].copy_from_slice(&flags.to_le_bytes());
        assert!(check(0, &p, 1, false, &metadata(0, false)).is_err());
    }
    let mut p = field_payload(0);
    put(&mut p, 80, 1e30);
    p[120..124].copy_from_slice(&1u32.to_le_bytes());
    assert!(check(0, &p, 1, false, &metadata(0, false)).is_ok());
    put(&mut p, 80, 0.0);
    assert_eq!(
        check(0, &p, 1, false, &metadata(0, false)),
        Err(Error::Value)
    );
    let mut p = field_payload(0);
    put(&mut p, 80, -0.0);
    assert_eq!(
        check(0, &p, 1, false, &metadata(0, false)),
        Err(Error::Reserved)
    );
    let mut p = field_payload(0);
    put(&mut p, 0, f64::NAN);
    assert_eq!(
        check(0, &p, 1, false, &metadata(0, false)),
        Err(Error::NonFinite)
    );
}
#[test]
fn ray_physical_stokes_direction_and_full_spectrum_values() {
    let good = ray_payload();
    assert!(check(1, &good, 1, false, &metadata(1, false)).is_ok());
    for hz in [1e-100, 30.0, 2.402e9, 6e14, 3e20, 1e100] {
        let mut p = good.clone();
        put(&mut p, 80, hz);
        assert!(check(1, &p, 1, false, &metadata(1, false)).is_ok());
    }
    for (at, value) in [
        (40, 2.0),
        (48, -1.0),
        (56, 1.001),
        (80, 0.0),
        (88, -1.0),
        (96, -1.0),
        (112, std::f64::consts::FRAC_PI_2),
    ] {
        let mut p = good.clone();
        if at == 56 {
            put(&mut p, 72, 0.0);
        }
        put(&mut p, at, value);
        assert!(check(1, &p, 1, false, &metadata(1, false)).is_err());
    }
    let mut p = good.clone();
    put(&mut p, 48, f64::MAX);
    put(&mut p, 56, f64::MAX);
    put(&mut p, 72, 0.0);
    assert!(check(1, &p, 1, false, &metadata(1, false)).is_ok());
    let mut p = good;
    put(&mut p, 56, 1.0);
    put(&mut p, 72, 0.0);
    assert!(check(1, &p, 1, false, &metadata(1, false)).is_ok());
}
#[test]
fn spectra_bind_physical_axis_values_temperature_and_padding() {
    let p = spectrum_payload(0);
    let encoded = encode(&metadata(3, false), &[row(3, &p, 1, false)]);
    let view =
        StructuralView::open(&encoded, Family::EmfV4Candidate, StoreLimits::default()).unwrap();
    let chunk = admit(&view).unwrap();
    let record = chunk.spectrum(0).unwrap().unwrap();
    assert_eq!(
        record.values().collect::<Vec<_>>(),
        vec![1e-12, 2e-12, 3e-12, 4e-12]
    );
    assert_eq!(chunk.spectral_axis().unwrap().coordinate_count(), 4);
    for (at, value) in [(64, -1.0), (72, f64::INFINITY), (40, 300.0)] {
        let mut p = p.clone();
        put(&mut p, at, value);
        assert!(check(3, &p, 1, false, &metadata(3, false)).is_err());
    }
    let mut bad = p.clone();
    bad[127] = 1;
    assert_eq!(
        check(3, &bad, 1, false, &metadata(3, false)),
        Err(Error::Reserved)
    );
    let mut bad = p.clone();
    bad[36..40].copy_from_slice(&99u32.to_le_bytes());
    assert_eq!(
        check(3, &bad, 1, false, &metadata(3, false)),
        Err(Error::Descriptor)
    );
    let mut warm = p;
    put(&mut warm, 40, 300.0);
    warm[48..52].copy_from_slice(&1u32.to_le_bytes());
    assert!(check(3, &warm, 1, false, &metadata(3, false)).is_ok());
}

#[test]
fn ray_getters_and_optional_field_observations_keep_exact_units_and_bits() {
    let payload = ray_payload();
    let encoded = encode(&metadata(1, false), &[row(1, &payload, 1, false)]);
    let view =
        StructuralView::open(&encoded, Family::EmfV4Candidate, StoreLimits::default()).unwrap();
    let ray = admit(&view).unwrap().ray(0).unwrap().unwrap();
    assert_eq!(ray.direction(), [0.0, 0.0, 1.0]);
    assert_eq!(ray.stokes(), [1.0, 0.0, 0.0, -1.0]);
    assert_eq!(ray.carrier_hz(), 2.4e9);
    assert_eq!(ray.geometric_path_m(), 0.299792458);
    let mut payload = field_payload(0);
    put(&mut payload, 80, 2.402e9);
    put(&mut payload, 112, 0.00265);
    payload[120..124].copy_from_slice(&13u32.to_le_bytes());
    payload[124..128].copy_from_slice(&0.75f32.to_le_bytes());
    let encoded = encode(&metadata(0, false), &[row(0, &payload, 1, false)]);
    let view =
        StructuralView::open(&encoded, Family::EmfV4Candidate, StoreLimits::default()).unwrap();
    let record = admit(&view).unwrap().field(0).unwrap().unwrap();
    assert_eq!(record.carrier_hz(), Some(2.402e9));
    assert_eq!(record.stored_poynting_w_per_m2(), Some([0.0, 0.0, 0.00265]));
    assert_eq!(record.confidence(), Some(0.75));
}
