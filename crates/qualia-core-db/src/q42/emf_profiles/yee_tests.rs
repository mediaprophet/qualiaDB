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
        ticks_per_second_num: 1_000_000_000,
        ticks_per_second_den: 1,
    }
}

fn test_frame(grid: GridSpec) -> FieldFrame {
    FieldFrame {
        origin_m: [0.0, 0.0, 0.0],
        rotation_xyzw: [0.0, 0.0, 0.0, 1.0],
        grid: Some(grid),
    }
}

fn test_prov() -> FieldProvenance<'static> {
    FieldProvenance {
        role: 2,
        algorithm: Some("fdtd-yee-v1"),
        algorithm_version: Some("1.0"),
        lineage: Some("test-yee"),
    }
}

fn fi<'a>(key: u16, critical: bool, value: V<'a>) -> FieldInput<'a> {
    FieldInput {
        key,
        flags: u8::from(critical),
        value,
    }
}

#[test]
fn phasors_poynting_energy_jones_stokes_and_delay_oracles() {
    // Peak vs RMS ratio and harmonic time convention (-iwt vs +iwt).
    let omega = 2.0e9 * core::f64::consts::TAU;
    let t = 0.25 / 2.0e9; // quarter period: theta = +/- pi/2
    let e_minus = evaluate_instantaneous_phasor(
        [1.0, 0.0, 0.0],
        [0.0, 2.0, 0.0],
        omega,
        t,
        PhasorAmplitudeKind::Peak,
        HarmonicSign::MinusIwt,
    )
    .unwrap();
    // Re((e_re + i e_im) * (-i)) = e_im -> [0, 2, 0]
    assert!(e_minus[0].abs() < 1.0e-12);
    assert!((e_minus[1] - 2.0).abs() < 1.0e-12);

    let e_plus_rms = evaluate_instantaneous_phasor(
        [1.0, 0.0, 0.0],
        [0.0, 2.0, 0.0],
        omega,
        t,
        PhasorAmplitudeKind::Rms,
        HarmonicSign::PlusIwt,
    )
    .unwrap();
    // Re((e_re + i e_im) * (+i)) * sqrt(2) = -sqrt(2) * e_im
    assert!((e_plus_rms[1] + 2.0 * core::f64::consts::SQRT_2).abs() < 1.0e-12);
    assert!((rms_to_peak(peak_to_rms(3.5)) - 3.5).abs() < 1.0e-15);

    assert_eq!(
        evaluate_instantaneous_phasor(
            [1.0; 3],
            [0.0; 3],
            -1.0,
            0.0,
            PhasorAmplitudeKind::Peak,
            HarmonicSign::MinusIwt,
        ),
        Err(Error::Value)
    );

    // Vacuum Poynting vector and isotropic energy equivalence with E/B form.
    let e = [12.0, -5.0, 3.0];
    let b = [1.0e-8, 2.5e-8, -1.5e-8];
    let h = [
        b[0] / VACUUM_PERMEABILITY_H_PER_M,
        b[1] / VACUUM_PERMEABILITY_H_PER_M,
        b[2] / VACUUM_PERMEABILITY_H_PER_M,
    ];
    let s1 = poynting_e_cross_h(e, h);
    let s2 = poynting_from_eb_vacuum(e, b);
    for i in 0..3 {
        assert!((s1[i] - s2[i]).abs() < 1.0e-12);
    }
    let u1 = energy_density_isotropic(e, h, 1.0, 1.0).unwrap();
    let u2 = energy_density_vacuum_eb(e, b).unwrap();
    assert!((u1 - u2).abs() < 1.0e-20);

    // Jones right-circular state [1, i] -> Stokes [2, 0, 0, 2] scaled cleanly.
    let rcp = JonesVector {
        ex_re: 1.0,
        ex_im: 0.0,
        ey_re: 0.0,
        ey_im: 1.0,
    }
    .to_stokes(0.5)
    .unwrap();
    assert!((rcp[0] - 1.0).abs() < 1.0e-12);
    assert!(rcp[1].abs() < 1.0e-12 && rcp[2].abs() < 1.0e-12);
    assert!((rcp[3] - 1.0).abs() < 1.0e-12);

    // Passive linear rotation by +45 deg maps horizontal [1, 1, 0, 0] -> [1, 0, -1, 0] (+45 -> -45 in rotated frame).
    let rot45 = rotate_stokes_passive([1.0, 1.0, 0.0, 0.0], core::f64::consts::FRAC_PI_4).unwrap();
    assert!((rot45[0] - 1.0).abs() < 1.0e-12);
    assert!(rot45[1].abs() < 1.0e-12);
    assert!((rot45[2] + 1.0).abs() < 1.0e-12);

    let delay = optical_path_delay_seconds(1.5, 1.5).unwrap();
    assert!((delay - 2.25 / SPEED_OF_LIGHT_M_PER_S).abs() < 1.0e-18);
}

#[test]
fn yee_record_serialization_and_negative_zero_or_state_rejections() {
    let mut rec = YeeRecord::vacuum([1.0, -2.0, 3.0], [0.01, 0.02, -0.03], 6);
    rec.impressed_current_j_a_per_m2 = [0.2, -0.4, 0.6];
    let mut bytes = [0u8; 128];
    rec.encode(true, &mut bytes).unwrap();
    let parsed = YeeRecord::parse(
        &bytes,
        true,
        YeeParsePolicy {
            expected_constitutive_id: Some(6),
            ..Default::default()
        },
    )
    .unwrap()
    .unwrap();
    assert_eq!(parsed, rec);

    // Absent cell must be bit-for-bit zeros.
    let mut absent = [0u8; 128];
    rec.encode(false, &mut absent).unwrap();
    assert_eq!(
        YeeRecord::parse(&absent, false, YeeParsePolicy::default()).unwrap(),
        None
    );
    absent[10] = 1;
    assert_eq!(
        YeeRecord::parse(&absent, false, YeeParsePolicy::default()),
        Err(Error::Reserved)
    );

    // Non-zero scalar potential bits (even -0.0!) fail when placeholder check is enabled.
    let mut phi_neg_zero = bytes;
    phi_neg_zero[72..80].copy_from_slice(&(-0.0f64).to_le_bytes());
    assert_eq!(
        YeeRecord::parse(&phi_neg_zero, true, YeeParsePolicy::default()),
        Err(Error::Reserved)
    );

    // State record index without auxiliary streams fails closed.
    let mut bad_state = rec;
    bad_state.state_record_index = 0;
    bad_state.encode(true, &mut bytes).unwrap();
    assert_eq!(
        YeeRecord::parse(&bytes, true, YeeParsePolicy::default()),
        Err(Error::Index)
    );

    // Constitutive metadata ID mismatch fails closed.
    let bad_id = YeeRecord::parse(
        &bytes,
        true,
        YeeParsePolicy {
            expected_constitutive_id: Some(99),
            has_auxiliary_state_streams: true,
            ..Default::default()
        },
    );
    assert_eq!(bad_id, Err(Error::Descriptor));
}

#[test]
fn yee_cfl_dispersion_and_periodic_leapfrog_energy_and_loss() {
    let spacing = [0.01, 0.01, 0.01];
    let dt_cfl = yee_cfl_max_dt_seconds(spacing, 1.0, 1.0).unwrap();
    let expected_cfl = 0.01 / (SPEED_OF_LIGHT_M_PER_S * 3.0f64.sqrt());
    assert!((dt_cfl - expected_cfl).abs() < 1.0e-22);
    assert!(yee_check_cfl(0.5 * dt_cfl, spacing, 1.0, 1.0).unwrap() < 0.500001);
    assert_eq!(
        yee_check_cfl(1.01 * dt_cfl, spacing, 1.0, 1.0),
        Err(Error::Time)
    );

    // Verify discrete dispersion formula exactly.
    let dt = 0.1 * dt_cfl;
    let k = [40.0, 0.0, 0.0];
    let omega_disc = yee_discrete_dispersion_omega(k, spacing, dt, 1.0, 1.0).unwrap();
    let lhs = (0.5 * omega_disc * dt).sin().powi(2);
    let rhs = (SPEED_OF_LIGHT_M_PER_S * dt / spacing[0]).powi(2) * (0.5 * k[0] * spacing[0]).sin().powi(2);
    assert!((lhs - rhs).abs() < 1.0e-15);

    // 4x4x4 lossless periodic lattice keeps energy bounded/periodic.
    let dims = [4u32, 4, 4];
    let mut cells = vec![YeeRecord::vacuum([0.0; 3], [0.0; 3], 6); 64];
    for x in 0..4 {
        for y in 0..4 {
            for z in 0..4 {
                let idx = x + 4 * (y + 4 * z);
                let phase = core::f64::consts::TAU * (x as f64) / 4.0;
                cells[idx].electric_field_v_per_m = [0.0, 0.0, phase.sin()];
            }
        }
    }
    let e0 = yee_total_energy_j(&cells, spacing).unwrap();
    assert!(e0 > 0.0);
    for _ in 0..20 {
        yee_step_periodic(&mut cells, dims, spacing, dt).unwrap();
    }
    let e20 = yee_total_energy_j(&cells, spacing).unwrap();
    assert!(((e20 - e0) / e0).abs() < 0.08);

    // Lossy conductivity strictly dissipates electromagnetic energy.
    for c in &mut cells {
        c.conductivity_s_per_m = 0.5;
    }
    let e_loss_start = yee_total_energy_j(&cells, spacing).unwrap();
    for _ in 0..25 {
        yee_step_periodic(&mut cells, dims, spacing, dt).unwrap();
    }
    let e_loss_end = yee_total_energy_j(&cells, spacing).unwrap();
    assert!(e_loss_end < 0.5 * e_loss_start);

    // Unstable time step rejected before stepping.
    assert_eq!(
        yee_step_periodic(&mut cells, dims, spacing, 1.1 * dt_cfl),
        Err(Error::Time)
    );
}

#[test]
fn yee_envelope_roundtrip_0x0102_and_candidate_2_with_material_binding() {
    let grid = GridSpec {
        dimensions: [2, 1, 1],
        brick_dimensions: [2, 1, 1],
        spacing_m: [0.01, 0.01, 0.01],
        indexing: GridIndexing::Linear,
    };
    let mut r0 = YeeRecord::vacuum([10.0, 0.0, 0.0], [0.0, 0.02, 0.0], 6);
    // Crucial check: non-zero impressed current J (bytes 48..72) must NOT be treated as position_m!
    r0.impressed_current_j_a_per_m2 = [99.0, -77.0, 55.0];
    let mut r1 = YeeRecord::vacuum([0.0, -5.0, 0.0], [0.0, 0.0, 0.01], 6);
    r1.material_entry = 7;

    let mut payload = [0u8; 256];
    let n = encode_yee_records(&[r0, r1], None, &mut payload).unwrap();
    assert_eq!(n, 256);

    let offsets_n = [0i64, 0, 0, 1, 1, 1];
    let offsets_d = [1u64, 1, 1, 2, 2, 2];
    let spec = ExtendedModeSpec::Yee {
        constitutive: ConstitutiveSpec {
            model: "vacuum",
            epsilon_infinity_relative: 1.0,
            mu_relative: 1.0,
            conductivity_s_per_m: 0.0,
            delta_epsilon: &[],
            relaxation_time_s: &[],
            resonance_rad_s: &[],
            damping_s1: &[],
        },
        time_offset_num: &offsets_n,
        time_offset_den: &offsets_d,
        material: Some(MaterialSpec {
            container_digest: [0x12; 32],
            table_digest: [0x34; 32],
            table_epoch: 3,
            table_schema_version: "v1",
            material_ids: &[7],
        }),
    };

    for mode_id in [0x0102u16, 2u16] {
        let mut meta_buf = [0u8; 4096];
        let mut staging = [0u8; 8192];
        let mut out = [0u8; 8192];
        let mut md = [NodeState::default(); 32];
        let mut ed = [NodeState::default(); 8];
        let mut scratch = Scratch {
            metadata: &mut md,
            entries: &mut ed,
        };
        let written = write_extended_envelope(
            test_clock(),
            test_frame(grid),
            test_prov(),
            mode_id,
            Some(10),
            0,
            1,
            Some(0),
            2,
            &payload,
            &spec,
            &mut meta_buf,
            &mut staging,
            &mut out,
            &mut scratch,
            FieldWriteLimits::default(),
        )
        .unwrap();

        let view = StructuralView::open(written, Family::EmfV4Candidate, StoreLimits::default()).unwrap();
        let mut reader = ProfileReader::open(&view, &mut scratch, Limits::default(), 65536).unwrap();
        let chunk = reader.chunk(1).unwrap();
        assert_eq!(chunk.yee(0).unwrap().unwrap(), r0);
        assert_eq!(chunk.yee(1).unwrap().unwrap(), r1);
        assert!(chunk.field(0).is_err());
    }
}

#[test]
fn yee_rejects_dispersive_restart_without_aux_and_bad_stagger_or_constitutive_mismatch() {
    let mut payload = [0u8; 128];
    YeeRecord::vacuum([1.0, 0.0, 0.0], [0.0; 3], 6)
        .encode(true, &mut payload)
        .unwrap();

    // 1. Debye constitutive model without aux_entries (26) / boundary_entries (29) fails closed.
    let f_clk = [
        fi(1, true, V::U32(&[0])),
        fi(2, true, V::I64(&[0])),
        fi(3, true, V::U32(&[0])),
        fi(4, true, V::U64(&[1000])),
        fi(5, true, V::U64(&[1])),
    ];
    let f_frm = [
        fi(1, true, V::U32(&[0])),
        fi(3, true, V::F64(&[0.0; 3])),
        fi(4, true, V::F64(&[0.0, 0.0, 0.0, 1.0])),
        fi(5, true, V::F64(&[0.01, 0.01, 0.01])),
        fi(6, true, V::U32(&[1, 1, 1])),
        fi(7, true, V::U32(&[0])),
        fi(8, true, V::Utf8("right-handed-X-forward-Y-left-Z-up")),
    ];
    let f_prv = [fi(1, true, V::U32(&[0]))];
    let f_ext_deb = [
        fi(1, true, V::U32(&[0x0102])),
        fi(2, true, V::U32(&[1])),
        fi(3, true, V::MetadataRefs(&[1])),
        fi(4, true, V::MetadataRefs(&[2])),
        fi(5, true, V::MetadataRefs(&[3])),
        fi(6, true, V::Utf8(YEE_PROFILE)),
        fi(7, true, V::Utf8("linear")),
        fi(8, true, V::U64(&[1, 1, 1])),
        fi(9, true, V::U32(&[1, 1, 1])),
        fi(10, true, V::U32(&[0])),
        fi(11, true, V::I64(&[0])),
        fi(14, true, V::MetadataRefs(&[6])),
    ];
    let f_dsc = [
        fi(2, true, V::MetadataRefs(&[2])),
        fi(3, true, V::MetadataRefs(&[3])),
        fi(7, true, V::Utf8("yee_cartesian_cell")),
        fi(8, true, V::Utf8("mixed:mode-registry")),
        fi(12, true, V::Utf8(YEE_MODEL_URI)),
        fi(18, true, V::MetadataRefs(&[4])),
    ];
    let f_deb = [
        fi(1, true, V::Utf8("debye_isotropic")),
        fi(2, true, V::F64(&[2.0])),
        fi(3, true, V::F64(&[1.0])),
        fi(4, true, V::F64(&[0.0])),
        fi(5, false, V::F64(&[1.5])),
        fi(6, false, V::F64(&[1.0e-11])),
    ];
    let blocks = [
        BlockInput { id: 1, schema_type: 1, revision: 1, flags: 1, fields: &f_clk },
        BlockInput { id: 2, schema_type: 2, revision: 1, flags: 1, fields: &f_frm },
        BlockInput { id: 3, schema_type: 4, revision: 1, flags: 1, fields: &f_prv },
        BlockInput { id: 4, schema_type: 103, revision: 1, flags: 1, fields: &f_ext_deb },
        BlockInput { id: 5, schema_type: 3, revision: 1, flags: 1, fields: &f_dsc },
        BlockInput { id: 6, schema_type: 133, revision: 1, flags: 1, fields: &f_deb },
    ];
    let entry = [EntryInput {
        mode: 0x0102,
        revision: 1,
        flags: 4,
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
    let m_in = MetadataInput { blocks: &blocks };
    let mut mb = vec![0u8; m_in.encoded_len(StoreLimits::default()).unwrap()];
    let meta = m_in.encode_into(&mut mb, StoreLimits::default()).unwrap();
    let e_in = EnvelopeInput {
        family: Family::EmfV4Candidate,
        flags: 0,
        metadata: meta,
        entries: &entry,
    };
    let mut env = vec![0u8; e_in.encoded_len(StoreLimits::default()).unwrap()];
    e_in.encode_into(&mut env, StoreLimits::default()).unwrap();
    let view = StructuralView::open(&env, Family::EmfV4Candidate, StoreLimits::default()).unwrap();
    let mut md = [NodeState::default(); 32];
    let mut ed = [NodeState::default(); 8];
    let mut scratch = Scratch { metadata: &mut md, entries: &mut ed };
    assert_eq!(
        admit_chunk(&view, 1, &mut scratch, Limits::default()).map(|_| ()),
        Err(Error::UnsupportedProfile)
    );

    // 2. Non-zero staggered E offset rejects Time.
    let bad_spec = ExtendedModeSpec::Yee {
        constitutive: ConstitutiveSpec {
            model: "vacuum",
            epsilon_infinity_relative: 1.0,
            mu_relative: 1.0,
            conductivity_s_per_m: 0.0,
            delta_epsilon: &[],
            relaxation_time_s: &[],
            resonance_rad_s: &[],
            damping_s1: &[],
        },
        time_offset_num: &[1, 0, 0, 1, 1, 1],
        time_offset_den: &[2, 2, 2, 2, 2, 2],
        material: None,
    };
    let grid = GridSpec {
        dimensions: [1, 1, 1],
        brick_dimensions: [1, 1, 1],
        spacing_m: [0.01, 0.01, 0.01],
        indexing: GridIndexing::Linear,
    };
    let mut mb2 = [0u8; 4096];
    let mut st2 = [0u8; 8192];
    let mut out2 = [0u8; 8192];
    assert_eq!(
        write_extended_envelope(
            test_clock(),
            test_frame(grid),
            test_prov(),
            0x0102,
            None,
            0,
            0,
            None,
            1,
            &payload,
            &bad_spec,
            &mut mb2,
            &mut st2,
            &mut out2,
            &mut scratch,
            FieldWriteLimits::default(),
        )
        .unwrap_err(),
        Error::Shared(super::super::field_semantics::Error::Time)
    );
}
