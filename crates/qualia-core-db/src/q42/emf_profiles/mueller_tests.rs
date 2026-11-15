use super::super::field_envelope::{
    BlockInput, EntryInput, EnvelopeInput, Family, FieldInput, FieldValue as V,
    Limits as StoreLimits, MetadataInput, StructuralView,
};
use super::super::field_semantics::NodeState;
use super::*;

fn test_clock() -> FieldClock {
    FieldClock {
        scale: 0,
        epoch_seconds: 1_700_000_000,
        epoch_nanoseconds: 0,
        ticks_per_second_num: 1000,
        ticks_per_second_den: 1,
    }
}

fn test_frame() -> FieldFrame {
    FieldFrame {
        origin_m: [0.0, 0.0, 0.0],
        rotation_xyzw: [0.0, 0.0, 0.0, 1.0],
        grid: None,
    }
}

fn test_prov() -> FieldProvenance<'static> {
    FieldProvenance {
        role: 1,
        algorithm: Some("mueller-spectral-suite"),
        algorithm_version: Some("1.0"),
        lineage: Some("test-e04"),
    }
}

fn make_spectrum_payload(tick: i64, bins: &[f64], stride: usize) -> Vec<u8> {
    let mut buf = vec![0u8; stride];
    buf[24..32].copy_from_slice(&tick.to_le_bytes());
    buf[36..40].copy_from_slice(&6u32.to_le_bytes());
    for (i, &val) in bins.iter().enumerate() {
        let off = 64 + i * 8;
        buf[off..off + 8].copy_from_slice(&val.to_le_bytes());
    }
    buf
}

#[test]
fn band_edges_24_and_28_precision_profiles_and_density_conversions() {
    let mut e24 = [0.0f64; BAND_COUNT_24 + 1];
    assert_eq!(build_log_band_edges(1.0e9, 1.0e15, &mut e24).unwrap(), 24);
    assert_eq!(validate_band_edges(&e24).unwrap(), 24);

    let mut e28 = [0.0f64; BAND_COUNT_28 + 1];
    assert_eq!(build_log_band_edges(400.0e-9, 700.0e-9, &mut e28).unwrap(), 28);
    assert_eq!(validate_band_edges(&e28).unwrap(), 28);

    // Non-monotonic or non-positive band edge must reject.
    let mut bad = e24;
    bad[5] = bad[4];
    assert_eq!(validate_band_edges(&bad), Err(Error::Value));
    bad[0] = 0.0;
    assert_eq!(validate_band_edges(&bad), Err(Error::Value));

    // Precision profiles.
    let q = SpectralPrecisionProfile::QuantizedScaledI32 {
        scale: 1.0e-6,
        offset: 0.0,
    };
    let enc = q.encode_value(0.123_456_7).unwrap();
    assert!((enc - 0.123_457).abs() < 1.0e-12);
    assert_eq!(
        SpectralPrecisionProfile::Float32.encode_value(1.5).unwrap(),
        1.5
    );

    // Jacobian conversions: S_f -> S_omega, S_q = f * S_f, and roundtrip S_lambda <-> S_f.
    let s_f = 2.5e-18;
    let f_hz = 5.0e14;
    let s_omega = convert_density_hz(s_f, f_hz, DensityTarget::AngularFrequencyRadS).unwrap();
    assert!((s_omega * core::f64::consts::TAU - s_f).abs() < 1.0e-30);
    let s_q = convert_density_hz(s_f, f_hz, DensityTarget::LogFrequencyNatural).unwrap();
    assert!((s_q - f_hz * s_f).abs() / s_q < 1.0e-12);
    let s_lam = convert_density_hz(s_f, f_hz, DensityTarget::VacuumWavelengthMetres).unwrap();
    let lambda = super::spectrum::VACUUM_SPEED_OF_LIGHT_M_PER_S / f_hz;
    let s_f_back = convert_density_wavelength_to_hz(s_lam, lambda).unwrap();
    assert!(((s_f_back - s_f) / s_f).abs() < 1.0e-12);

    // Response weighting.
    let mut weighted = [0.0f64; 3];
    apply_spectral_response(&[1.0, 2.0, 3.0], &[0.5, 0.25, 2.0], &mut weighted).unwrap();
    assert_eq!(weighted, [0.5, 0.5, 6.0]);
}

#[test]
fn spectrum_24_and_28_band_envelopes_with_response_calibration_and_integration() {
    let mut e24 = [0.0f64; 25];
    build_log_band_edges(1.0e9, 1.0e12, &mut e24).unwrap();
    let bins24 = [1.0e-12f64; 24];
    let stride24 = 64 + 24 * 8;
    let payload24 = make_spectrum_payload(10, &bins24, stride24);

    let spec24 = ExtendedModeSpec::Spectrum {
        axis: SpectralAxisSpec {
            axis: "frequency_hz",
            sample_semantics: "bin_average",
            physical_quantity: "spectral_irradiance",
            unit: "W m^-2 Hz^-1",
            coordinates: &e24,
            sigma_interpretation: "none",
            sigma_bounds: None,
        },
        bins: 24,
        response_cal: Some(CalibrationSpec {
            instrument_identifier: "radiometer-24ch",
            content_digest: [0x24; 32],
            quantity_unit: "V/W",
            scale: 1.0,
            offset: 0.0,
            uncertainty: Some((0.02, "relative_1sigma")),
            validity_ticks: Some([0, 100]),
        }),
    };
    let mut mb = [0u8; 4096];
    let mut st = [0u8; 8192];
    let mut out = [0u8; 8192];
    let mut md = [NodeState::default(); 32];
    let mut ed = [NodeState::default(); 8];
    let mut scratch = Scratch { metadata: &mut md, entries: &mut ed };
    let bytes24 = write_extended_envelope(
        test_clock(),
        test_frame(),
        test_prov(),
        0x0103,
        Some(10),
        10,
        1,
        None,
        1,
        &payload24,
        &spec24,
        &mut mb,
        &mut st,
        &mut out,
        &mut scratch,
        FieldWriteLimits::default(),
    )
    .unwrap();
    let v24 = StructuralView::open(bytes24, Family::EmfV4Candidate, StoreLimits::default()).unwrap();
    let mut r24 = ProfileReader::open(&v24, &mut scratch, Limits::default(), 65536).unwrap();
    let c24 = r24.chunk(1).unwrap();
    let srec = c24.spectrum(0).unwrap().unwrap();
    let vals: Vec<f64> = srec.values().collect();
    assert_eq!(vals.len(), 24);
    let total = integrate_spectrum_over_axis(c24.spectral_axis().unwrap(), &vals).unwrap();
    let expected_total = 1.0e-12 * (1.0e12 - 1.0e9);
    assert!(((total - expected_total) / expected_total).abs() < 1.0e-12);

    // 28-band roundtrip also includes non-64-aligned byte padding check (`28 * 8 = 224` -> stride 320).
    let mut e28 = [0.0f64; 29];
    build_log_band_edges(400.0e-9, 700.0e-9, &mut e28).unwrap();
    let bins28 = [2.0e-9f64; 28];
    let stride28 = 64 + 256;
    let payload28 = make_spectrum_payload(10, &bins28, stride28);
    let spec28 = ExtendedModeSpec::Spectrum {
        axis: SpectralAxisSpec {
            axis: "wavelength_m",
            sample_semantics: "bin_average",
            physical_quantity: "spectral_radiance",
            unit: "W sr^-1 m^-2 m^-1",
            coordinates: &e28,
            sigma_interpretation: "none",
            sigma_bounds: None,
        },
        bins: 28,
        response_cal: None,
    };
    let mut out28 = [0u8; 8192];
    let bytes28 = write_extended_envelope(
        test_clock(),
        test_frame(),
        test_prov(),
        3,
        Some(10),
        10,
        1,
        None,
        1,
        &payload28,
        &spec28,
        &mut mb,
        &mut st,
        &mut out28,
        &mut scratch,
        FieldWriteLimits::default(),
    )
    .unwrap();
    let v28 = StructuralView::open(bytes28, Family::EmfV4Candidate, StoreLimits::default()).unwrap();
    let mut r28 = ProfileReader::open(&v28, &mut scratch, Limits::default(), 65536).unwrap();
    assert_eq!(r28.chunk(1).unwrap().spectrum(0).unwrap().unwrap().values().count(), 28);
}

#[test]
fn spectrum_rejects_invalid_response_reference() {
    let f_clk = [
        FieldInput { key: 1, flags: 1, value: V::U32(&[0]) },
        FieldInput { key: 2, flags: 1, value: V::I64(&[0]) },
        FieldInput { key: 3, flags: 1, value: V::U32(&[0]) },
        FieldInput { key: 4, flags: 1, value: V::U64(&[1000]) },
        FieldInput { key: 5, flags: 1, value: V::U64(&[1]) },
    ];
    let f_frm = [
        FieldInput { key: 1, flags: 1, value: V::U32(&[0]) },
        FieldInput { key: 3, flags: 1, value: V::F64(&[0.0; 3]) },
        FieldInput { key: 4, flags: 1, value: V::F64(&[0.0, 0.0, 0.0, 1.0]) },
        FieldInput { key: 8, flags: 1, value: V::Utf8("right-handed-X-forward-Y-left-Z-up") },
    ];
    let f_prv = [FieldInput { key: 1, flags: 1, value: V::U32(&[0]) }];
    let f_ext = [
        FieldInput { key: 1, flags: 1, value: V::U32(&[0x0103]) },
        FieldInput { key: 2, flags: 1, value: V::U32(&[1]) },
        FieldInput { key: 3, flags: 1, value: V::MetadataRefs(&[1]) },
        FieldInput { key: 4, flags: 1, value: V::MetadataRefs(&[2]) },
        FieldInput { key: 5, flags: 1, value: V::MetadataRefs(&[3]) },
        FieldInput { key: 6, flags: 1, value: V::Utf8(SPECTRUM_PROFILE) },
        FieldInput { key: 7, flags: 1, value: V::Utf8("linear") },
        FieldInput { key: 8, flags: 1, value: V::U64(&[1, 1, 1]) },
        FieldInput { key: 9, flags: 1, value: V::U32(&[1, 1, 1]) },
        FieldInput { key: 10, flags: 1, value: V::U32(&[0]) },
        FieldInput { key: 11, flags: 1, value: V::I64(&[0]) },
        FieldInput { key: 12, flags: 1, value: V::MetadataRefs(&[6]) },
        FieldInput { key: 19, flags: 1, value: V::U32(&[2]) },
    ];
    let f_dsc = [
        FieldInput { key: 2, flags: 0, value: V::MetadataRefs(&[2]) },
        FieldInput { key: 3, flags: 1, value: V::MetadataRefs(&[3]) },
        FieldInput { key: 7, flags: 1, value: V::Utf8("spectral_irradiance") },
        FieldInput { key: 8, flags: 1, value: V::Utf8("W m^-2 Hz^-1") },
        FieldInput { key: 12, flags: 1, value: V::Utf8(SPECTRUM_MODEL_URI) },
        FieldInput { key: 18, flags: 1, value: V::MetadataRefs(&[4]) },
    ];
    // Points key 7 (response) to Provenance block id=3 instead of Calibration 107 / response schema 3.
    let f_ax = [
        FieldInput { key: 1, flags: 1, value: V::Utf8("frequency_hz") },
        FieldInput { key: 2, flags: 1, value: V::Utf8("point_density") },
        FieldInput { key: 3, flags: 1, value: V::Utf8("spectral_irradiance") },
        FieldInput { key: 4, flags: 1, value: V::Utf8("W m^-2 Hz^-1") },
        FieldInput { key: 5, flags: 1, value: V::F64(&[1.0e9, 2.0e9]) },
        FieldInput { key: 6, flags: 1, value: V::Utf8("explicit") },
        FieldInput { key: 7, flags: 0, value: V::MetadataRefs(&[3]) },
        FieldInput { key: 8, flags: 1, value: V::Utf8("none") },
    ];
    let blocks = [
        BlockInput { id: 1, schema_type: 1, revision: 1, flags: 1, fields: &f_clk },
        BlockInput { id: 2, schema_type: 2, revision: 1, flags: 1, fields: &f_frm },
        BlockInput { id: 3, schema_type: 4, revision: 1, flags: 1, fields: &f_prv },
        BlockInput { id: 4, schema_type: 103, revision: 1, flags: 1, fields: &f_ext },
        BlockInput { id: 5, schema_type: 3, revision: 1, flags: 1, fields: &f_dsc },
        BlockInput { id: 6, schema_type: 132, revision: 1, flags: 1, fields: &f_ax },
    ];
    let payload = make_spectrum_payload(0, &[1.0, 2.0], 128);
    let m_in = MetadataInput { blocks: &blocks };
    let mut mb = vec![0u8; m_in.encoded_len(StoreLimits::default()).unwrap()];
    let meta = m_in.encode_into(&mut mb, StoreLimits::default()).unwrap();
    let entry = [EntryInput {
        mode: 0x0103,
        revision: 1,
        flags: 0,
        stream_id: 0,
        frame_id: 2,
        start_tick: 0,
        time_count: 0,
        spatial_key: 0,
        record_count: 1,
        stride: 128,
        codec: 0,
        descriptor_id: 5,
        payload: &payload,
    }];
    let e_in = EnvelopeInput { family: Family::EmfV4Candidate, flags: 0, metadata: meta, entries: &entry };
    let mut env = vec![0u8; e_in.encoded_len(StoreLimits::default()).unwrap()];
    e_in.encode_into(&mut env, StoreLimits::default()).unwrap();
    let view = StructuralView::open(&env, Family::EmfV4Candidate, StoreLimits::default()).unwrap();
    let mut md = [NodeState::default(); 32];
    let mut ed = [NodeState::default(); 8];
    let mut scratch = Scratch { metadata: &mut md, entries: &mut ed };
    assert!(admit_chunk(&view, 1, &mut scratch, Limits::default()).is_err());
}

#[test]
fn jones_to_mueller_passive_rotation_and_cloude_realizability_oracle() {
    // 1. Horizontal polarizer and Quarter-Wave Plate Jones -> Mueller.
    let h_pol = jones_to_mueller([[1.0, 0.0], [0.0, 0.0]], [[0.0; 2]; 2]).unwrap();
    assert!((h_pol[0][0] - 0.5).abs() < 1.0e-12);
    assert!((h_pol[0][1] - 0.5).abs() < 1.0e-12);
    assert!((h_pol[1][0] - 0.5).abs() < 1.0e-12);
    assert!((h_pol[1][1] - 0.5).abs() < 1.0e-12);

    let qwp = jones_to_mueller([[1.0, 0.0], [0.0, 0.0]], [[0.0, 0.0], [0.0, 1.0]]).unwrap();
    assert!((qwp[2][3] + 1.0).abs() < 1.0e-12 || (qwp[2][3] - 1.0).abs() < 1.0e-12);
    // Applying QWP to +45 linear polarized [1, 0, 1, 0] produces circular [1, 0, 0, +/-1].
    let circ = apply_mueller(&qwp, [1.0, 0.0, 1.0, 0.0]).unwrap();
    assert!((circ[0] - 1.0).abs() < 1.0e-12);
    assert!(circ[1].abs() < 1.0e-12 && circ[2].abs() < 1.0e-12);
    assert!((circ[3].abs() - 1.0).abs() < 1.0e-12);

    // Passive basis rotation of horizontal polarizer by pi/4 extinguishes [1, 0, +1, 0] and transmits [1, 0, -1, 0].
    let rot_pol = rotate_mueller_passive(h_pol, core::f64::consts::FRAC_PI_4, core::f64::consts::FRAC_PI_4).unwrap();
    let pass = apply_mueller(&rot_pol, [1.0, 0.0, -1.0, 0.0]).unwrap();
    assert!((pass[0] - 1.0).abs() < 1.0e-12);
    let block = apply_mueller(&rot_pol, [1.0, 0.0, 1.0, 0.0]).unwrap();
    assert!(block[0].abs() < 1.0e-12);

    // 2. Cloude eigenvalues: pure rank-1 matrices yield [M00, 0, 0, 0]; isotropic depolarizer analytical match.
    let evals_qwp = verify_mueller_physical_realizability(&qwp).unwrap();
    assert!((evals_qwp[0] - 1.0).abs() < 1.0e-12);
    assert!(evals_qwp[1].abs() < 1.0e-12 && evals_qwp[3].abs() < 1.0e-12);

    let iso_depol = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 0.5, 0.0, 0.0],
        [0.0, 0.0, 0.5, 0.0],
        [0.0, 0.0, 0.0, 0.5],
    ];
    let ev_iso = verify_mueller_physical_realizability(&iso_depol).unwrap();
    assert!((ev_iso[0] - 0.625).abs() < 1.0e-12);
    assert!((ev_iso[1] - 0.125).abs() < 1.0e-12);
    assert!((ev_iso[2] - 0.125).abs() < 1.0e-12);
    assert!((ev_iso[3] - 0.125).abs() < 1.0e-12);

    // 3. Unphysical matrices passing naive row/column checks must fail Cloude PSD!
    let unphys_a = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, -0.9, 0.0, 0.0],
        [0.0, 0.0, -0.9, 0.0],
        [0.0, 0.0, 0.0, -0.9],
    ];
    assert_eq!(verify_mueller_physical_realizability(&unphys_a), Err(Error::Value));

    let unphys_b = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 0.8, 0.0, 0.0],
        [0.0, 0.0, 0.8, 0.0],
        [0.0, 0.0, 0.0, -0.8],
    ];
    assert_eq!(verify_mueller_physical_realizability(&unphys_b), Err(Error::Value));
}

#[test]
fn mueller_0x0104_record_and_envelope_roundtrip_plus_m00_rejection() {
    let qwp = jones_to_mueller([[1.0, 0.0], [0.0, 0.0]], [[0.0, 0.0], [0.0, 1.0]]).unwrap();
    let rec = MuellerRecord {
        position_m: [1.0, 2.0, 3.0],
        incident_direction: [0.0, 0.0, 1.0],
        outgoing_direction: [0.0, 1.0, 0.0],
        frequency_hz: 5.0e14,
        material_entry: 12,
        flags: 0,
        mueller_matrix_m: qwp,
    };
    let mut payload = [0u8; 256];
    assert_eq!(encode_mueller_records(&[rec], None, &mut payload).unwrap(), 256);

    // Reject M00 <= 0 on raw parse.
    let mut bad_m00 = payload;
    bad_m00[128..136].copy_from_slice(&0.0f64.to_le_bytes());
    assert_eq!(MuellerRecord::parse(&bad_m00, true, true), Err(Error::Value));

    // Reject unphysical Cloude matrix inside MuellerRecord::parse when require_cloude_realizable=true.
    let mut bad_cloude = payload;
    for r in 0..4 {
        for c in 0..4 {
            let val: f64 = if r == 0 && c == 0 {
                1.0
            } else if r == c {
                -0.6
            } else {
                0.0
            };
            let off = 128 + (r * 4 + c) * 8;
            bad_cloude[off..off + 8].copy_from_slice(&val.to_le_bytes());
        }
    }
    assert_eq!(MuellerRecord::parse(&bad_cloude, true, true), Err(Error::Value));

    let spec = ExtendedModeSpec::Mueller {
        evidence: ModelEvidenceSpec {
            model_identifier_version: "ray-optics-mueller-v1",
            domain_applicability: "optical_scattering",
            parameter_source_digest: [0x99; 32],
            numerical_convention: "I_Q_U_V_forward_backward",
            epistemic_class: "model_estimate",
            uncertainty: Some("5%"),
            external_reference: None,
            qualification_receipt_id: None,
            particle_species: None,
            integration_ticks: None,
            weighting_derating_model: None,
        },
        units: "sr^-1",
        material: Some(MaterialSpec {
            container_digest: [0x55; 32],
            table_digest: [0x66; 32],
            table_epoch: 1,
            table_schema_version: "v1",
            material_ids: &[12],
        }),
    };
    let mut mb = [0u8; 4096];
    let mut st = [0u8; 8192];
    let mut out = [0u8; 8192];
    let mut md = [NodeState::default(); 32];
    let mut ed = [NodeState::default(); 8];
    let mut scratch = Scratch { metadata: &mut md, entries: &mut ed };
    let bytes = write_extended_envelope(
        test_clock(),
        test_frame(),
        test_prov(),
        0x0104,
        None,
        0,
        0,
        None,
        1,
        &payload,
        &spec,
        &mut mb,
        &mut st,
        &mut out,
        &mut scratch,
        FieldWriteLimits::default(),
    )
    .unwrap();
    let view = StructuralView::open(bytes, Family::EmfV4Candidate, StoreLimits::default()).unwrap();
    let mut reader = ProfileReader::open(&view, &mut scratch, Limits::default(), 65536).unwrap();
    let chunk = reader.chunk(1).unwrap();
    let parsed = chunk.mueller(0).unwrap().unwrap();
    assert_eq!(parsed, rec);
    let out_stokes = parsed.apply_stokes([2.0, 0.0, 2.0, 0.0]).unwrap();
    assert!((out_stokes[0] - 2.0).abs() < 1.0e-12);
    assert!((out_stokes[3].abs() - 2.0).abs() < 1.0e-12);
}
