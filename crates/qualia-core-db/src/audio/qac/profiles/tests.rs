//! Unit and physics tests for Q05 profiles (HOA, HRIR, Ray RIR).
//! Included into `tests_q03_q05.rs` to satisfy the <= 500 lines per file requirement.

#[test]
fn test_q05_hoa_cardinal_normalization_fuma_rotation_and_domain_guard() {
    // Canonical order-3 ACN/SN3D values along +X, +Y, +Z.
    let y_px = evaluate_acn_sn3d_dir(3, [1.0, 0.0, 0.0]).expect("+X");
    let y_py = evaluate_acn_sn3d_dir(3, [0.0, 1.0, 0.0]).expect("+Y");
    let y_pz = evaluate_acn_sn3d_dir(3, [0.0, 0.0, 1.0]).expect("+Z");
    assert!((y_px[0] - 1.0).abs() < 1e-12); // W
    assert!((y_py[1] - 1.0).abs() < 1e-12); // Y (+Y left)
    assert!((y_pz[2] - 1.0).abs() < 1e-12); // Z (+Z up)
    assert!((y_px[3] - 1.0).abs() < 1e-12); // X (+X front)

    // SN3D <-> N3D exact factor sqrt(2l+1).
    let mut n3d = vec![0.0f32; 16];
    let sn3d_f32: Vec<f32> = y_px.iter().map(|&v| v as f32).collect();
    convert_sn3d_n3d(3, &sn3d_f32, HoaNormalization::Sn3d, HoaNormalization::N3d, &mut n3d).expect("sn3d -> n3d");
    assert!((f64::from(n3d[3]) - 3.0f64.sqrt() * y_px[3]).abs() < 1e-6);
    assert!((f64::from(n3d[15]) - 7.0f64.sqrt() * y_px[15]).abs() < 1e-6);

    // FuMa ordering/normalization preserves 1/sqrt(2) on W and permutes X/Y/Z cleanly.
    let mut fuma = vec![0.0f32; 16];
    acn_sn3d_to_fuma_maxn(3, &sn3d_f32, &mut fuma).expect("fuma");
    assert!((f64::from(fuma[0]) - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-6);
    assert!((f64::from(fuma[1]) - f64::from(sn3d_f32[3])).abs() < 1e-6); // X = ACN 3

    // Active +90 deg Z rotation turns +X directional field into +Y directional field across all 16 channels.
    let q90z = QuaternionXyzw::from_axis_angle([0.0, 0.0, 1.0], PI * 0.5).expect("q90z");
    let mut rot = vec![0.0f32; 16];
    rotate_acn_sn3d_frame(3, &q90z, &sn3d_f32, &mut rot).expect("rotate order 3");
    for (c, &expected) in y_py.iter().enumerate() {
        assert!(
            (f64::from(rot[c]) - expected).abs() < 1e-5,
            "channel {c} mismatch: got {} vs {}",
            rot[c],
            expected
        );
    }

    // FuMa claiming SN3D normalization is incoherent and rejected.
    let mut bad_spec = HoaProfileSpec {
        order: 1,
        ordering: HoaOrdering::FuMa,
        normalization: HoaNormalization::Sn3d,
        direction_convention: HoaDirectionConvention::ListenerToSource,
        units: "Pa",
        sample_rate: RationalRate::new(48_000, 1).expect("sr"),
    };
    assert!(write_hoa_temporal_qac(
        &[&[1.0, 0.0, 0.0, 0.0]],
        &bad_spec,
        &test_clock(),
        &FrameSpec::local_default(),
        &test_provenance(),
        None,
        ProfileLimits::default(),
    )
    .is_err());

    bad_spec.ordering = HoaOrdering::Acn;
    bad_spec.order = 3;
    let qac = write_hoa_temporal_qac(
        &[&sn3d_f32, &rot],
        &bad_spec,
        &test_clock(),
        &FrameSpec::local_default(),
        &test_provenance(),
        None,
        ProfileLimits::default(),
    )
    .expect("write hoa temporal");
    let view = read_hoa_temporal_qac(&qac, 1, ProfileLimits::default()).expect("read hoa");
    assert_eq!(view.domain, HoaDomain::TimeSample);
    assert_eq!(view.chunk.frame_count(), 2);
    let mut f0 = vec![0.0f32; 16];
    view.chunk.read_frame(0, &mut f0).expect("frame 0");
    assert!((f0[3] - 1.0).abs() < 1e-6);

    // Spectral HOA entry (mode 0x0101) cannot be admitted by temporal reader.
    let mut meta = MetadataBuilder::new();
    meta.push_clock(1, &test_clock()).expect("clk");
    meta.push_provenance(3, &test_provenance()).expect("prov");
    meta.push_block(OwnedBlock {
        id: 4,
        schema_type: 201,
        revision: 1,
        critical: true,
        fields: vec![
            OwnedField::crit(1, OwnedValue::Utf8("STFT-complex".to_owned())),
            OwnedField::crit(13, OwnedValue::U32(vec![4])),
        ],
    })
    .expect("s201");
    meta.push_descriptor(
        6,
        &DescriptorSpec {
            clock_id: Some(1),
            frame_id: None,
            provenance_id: 3,
            medium_id: None,
            transform_id: Some(4),
            calibration_id: None,
            quantity: "hoa-spectral",
            units: "complex_pressure_spectrum",
            channels: Some(4),
            layout: Some("frame-channel-bin"),
            scalar: Some(AuxScalar::ComplexF32),
            model_uri: "urn:qac:mode:0x0101:hoa:v1",
            sample_rate: Some(RationalRate::new(100, 1).expect("fr")),
            valid_record_count: None,
            validity_mask_entry: None,
            source_entries: &[],
            extension_metadata: &[],
        },
    )
    .expect("desc");
    let spectral_qac = assemble_qac_v1(
        &meta,
        &[OwnedEntry {
            mode: MODE_STFT_PRESERVATION,
            flags: 3,
            stream_id: 1,
            frame_id: 0,
            start_tick: 0,
            time_count: 1,
            spatial_key: 0,
            record_count: 1,
            stride: 4 * 4 * 8,
            descriptor_id: 6,
            payload: vec![0u8; 4 * 4 * 8],
        }],
        ProfileLimits::default(),
    )
    .expect("assemble spectral");
    assert!(matches!(
        read_hoa_temporal_qac(&spectral_qac, 1, ProfileLimits::default()),
        Err(ProfileError::InvalidDomainConversion(_))
    ));
}

#[test]
fn test_q05_hrir_delay_cadence_and_q4au_v3_lossless_migration() {
    // 1 measurement, 2 receivers, 8 taps, impulse rate 48 kHz vs acquisition cadence 10 Hz.
    let taps: u32 = 8;
    let mut impulses = vec![0.0f32; 16];
    impulses[2] = 1.0; // Left impulse peak at tap 2
    impulses[8 + 3] = 0.5; // Right impulse peak at tap 3
    let delays = [4.5f64, 1.0f64];
    let clk = test_clock();
    let hrir_spec = HrirWriteSpec {
        representation: "HRIR",
        taps,
        receivers: 2,
        measurements: 1,
        impulse_sample_rate: RationalRate::new(48_000, 1).expect("48k"),
        delay_reference: BASE_DELAY_REFERENCE,
        sofa_convention: Some("SimpleFreeFieldHRIR"),
        units: "1",
        delays_samples: Some(&delays),
        source_positions_xyz: Some(&[[1.0, 0.0, 0.0]]),
        receiver_positions_xyz: Some(&[[0.0, 0.09, 0.0], [0.0, -0.09, 0.0]]),
        measurement_cadence: Some((&clk, RationalRate::new(10, 1).expect("10hz"))),
    };
    let qac = write_hrir_qac(
        &impulses,
        &hrir_spec,
        &FrameSpec::local_default(),
        &test_provenance(),
        ProfileLimits::default(),
    )
    .expect("write hrir");
    let view = read_hrir_qac(&qac, 1, ProfileLimits::default()).expect("read hrir");
    assert_eq!(view.chunk.taps(), 8);
    assert_eq!(view.delays_samples.as_deref(), Some(&delays[..]));

    // Delay applied ONCE: fractional delay 4.5 + tap 2 => sample 6.5.
    let t_s = tap_time_seconds(delays[0], 2, view.impulse_sample_rate).expect("t_s");
    assert!((t_s - 6.5 / 48_000.0).abs() < 1e-12);
    let y_interp = sample_delayed_impulse(&impulses[..8], delays[0], 6.5);
    assert!((y_interp - 1.0).abs() < 1e-12);

    let mut conv = vec![0.0f32; 10];
    let mut unit_in = vec![0.0f32; 10];
    unit_in[0] = 1.0;
    convolve_hrir(&unit_in, &impulses[..8], &mut conv).expect("conv");
    assert!((conv[2] - 1.0).abs() < 1e-6);

    // Build synthetic valid Q4AU v3 fixture with SOFA table & migrate losslessly.
    let mut hdr = Q4AuHeaderV3::empty();
    hdr.magic = SPECTRAL_SIDECAR_MAGIC;
    hdr.version = Q4AU_VERSION_V3;
    hdr.transform_kind = TRANSFORM_KIND_HOA_STFT;
    hdr.bin_count = 2;
    hdr.frame_count = 1;
    hdr.sample_rate = 48_000;
    hdr.ambisonic_order = 1;
    hdr.channel_normalization = NORMALIZATION_SN3D_ACN;
    hdr.sofa_fir_taps = SOFA_DEFAULT_FIR_TAPS as u16;
    hdr.sofa_direction_count = 1;
    let spectral_bytes = 2 * 1 * 4 * 4;
    let sofa_off = (Q4AU_V3_HEADER_BYTES + spectral_bytes) as u32;
    hdr.sofa_block_offset = sofa_off;
    hdr.update_crc();

    let mut sofa = SofaHrirEntry {
        azimuth_deg: 90.0,
        elevation_deg: 0.0,
        itd_samples: 12.25,
        left_fir: [0.0; SOFA_DEFAULT_FIR_TAPS],
        right_fir: [0.0; SOFA_DEFAULT_FIR_TAPS],
    };
    sofa.left_fir[5] = 0.75;
    sofa.right_fir[11] = -0.25;

    let mut legacy = Vec::with_capacity(sofa_off as usize + std::mem::size_of::<SofaHrirEntry>());
    legacy.extend_from_slice(&bytemuck::bytes_of(&hdr)[..Q4AU_V3_HEADER_BYTES]);
    legacy.extend_from_slice(&vec![0u8; spectral_bytes]);
    legacy.extend_from_slice(bytemuck::bytes_of(&sofa));

    // Negative Q4AU v3 checks: overlapping SOFA offset, bad CRC, truncated SOFA.
    let mut bad_overlap_hdr = hdr;
    bad_overlap_hdr.sofa_block_offset = Q4AU_V3_HEADER_BYTES as u32;
    bad_overlap_hdr.update_crc();
    let mut bad_overlap = legacy.clone();
    bad_overlap[..Q4AU_V3_HEADER_BYTES].copy_from_slice(&bytemuck::bytes_of(&bad_overlap_hdr)[..Q4AU_V3_HEADER_BYTES]);
    assert!(parse_legacy_q4au_v3(&bad_overlap).is_err());
    assert!(parse_legacy_q4au_v3(&legacy[..legacy.len() - 4]).is_err());
    let mut bad_crc = legacy.clone();
    bad_crc[16] ^= 1;
    assert!(parse_legacy_q4au_v3(&bad_crc).is_err());

    let migrated = migrate_legacy_q4au_v3_hrir(&legacy, ProfileLimits::default()).expect("migrate v3");
    let mview = read_hrir_qac(&migrated, 1, ProfileLimits::default()).expect("read migrated v3");
    assert_eq!(mview.legacy_original_bytes, Some(legacy.as_slice()));
    assert_eq!(mview.delay_reference, LEGACY_DELAY_REFERENCE);
    assert_eq!(mview.delays_samples.expect("delays")[0], f64::from(12.25f32));
    let mut l_out = vec![0.0f32; SOFA_DEFAULT_FIR_TAPS];
    mview.chunk.read_impulse(0, 0, &mut l_out).expect("left fir");
    assert_eq!(l_out[5], 0.75);
}

#[test]
fn test_q05_ray_rir_energy_vs_signed_phase_and_diffraction_validation() {
    let rays = vec![
        RayContributionRecord {
            origin_m: [0.0, 0.0, 0.0],
            distance_m: 3.43,
            direction: [1.0, 0.0, 0.0],
            delay_s: 0.01,
            ray_id: 1,
            band_row: 0,
            material_ref: 0,
            specular_count: 0,
            diffraction_count: 0,
            scattering_coefficient: 0.0,
            doppler_ratio: 1.0,
            validity_bits: RAY_VALID_ALL,
        },
        RayContributionRecord {
            origin_m: [0.0, 0.0, 0.0],
            distance_m: 6.86,
            direction: [0.0, 1.0, 0.0],
            delay_s: 0.02,
            ray_id: 2,
            band_row: 1,
            material_ref: 1,
            specular_count: 1,
            diffraction_count: 1,
            scattering_coefficient: 0.2,
            doppler_ratio: 1.0,
            validity_bits: RAY_VALID_ALL,
        },
    ];
    let bands = [500.0f32, 2000.0f32];
    let energy_vals = RayBandValues::EnergyRatio {
        bands: 2,
        values: vec![0.8, 0.6, 0.2, 0.1],
    };
    let signed_vals = RayBandValues::SignedPressureRatio {
        bands: 2,
        values: vec![0.9, 0.8, -0.5, -0.3],
    };

    // Energy-only rays refuse signed coherent RIR synthesis; signed pressure preserves phase flips.
    assert!(matches!(
        energy_vals.synthesize_signed_rir_impulse(&rays, 0, 1000.0, 40),
        Err(ProfileError::InvalidDomainConversion(_))
    ));
    let ir = signed_vals
        .synthesize_signed_rir_impulse(&rays, 0, 1000.0, 40)
        .expect("signed rir");
    assert!((ir[10] - 0.9).abs() < 1e-6);
    assert!((ir[20] + 0.5).abs() < 1e-6);

    let (r_p, e_r, e_t) = planar_impedance_reflection_energy(413.0, 1200.0).expect("refl");
    assert!((e_r - r_p * r_p).abs() < 1e-12);
    assert!((e_r + e_t - 1.0).abs() < 1e-12);

    // Diffraction count > 0 under DiffractionModel::None must fail closed.
    assert!(write_ray_contributions_qac(
        &rays,
        &bands,
        &signed_vals,
        1.0,
        DiffractionModel::None,
        None,
        None,
        &test_clock(),
        &FrameSpec::local_default(),
        &test_provenance(),
        ProfileLimits::default(),
    )
    .is_err());

    // Out-of-bounds band_row fails closed.
    let mut bad_row = rays.clone();
    bad_row[1].band_row = 9;
    assert!(matches!(
        write_ray_contributions_qac(
            &bad_row,
            &bands,
            &signed_vals,
            1.0,
            DiffractionModel::UtdWedge,
            None,
            None,
            &test_clock(),
            &FrameSpec::local_default(),
            &test_provenance(),
            ProfileLimits::default(),
        ),
        Err(ProfileError::OutOfBoundsIndex(_))
    ));

    let launch = InstantSpec::new(1, 0, 0, 1).expect("launch");
    let qac = write_ray_contributions_qac(
        &rays,
        &bands,
        &signed_vals,
        1.0,
        DiffractionModel::UtdWedge,
        Some(&[1.0, 1.0]),
        Some(&launch),
        &test_clock(),
        &FrameSpec::local_default(),
        &test_provenance(),
        ProfileLimits::default(),
    )
    .expect("write rays");
    let view = read_ray_contributions_qac(&qac, 1, ProfileLimits::default()).expect("read rays");
    assert_eq!(view.gain_quantity, GainQuantity::PressureAmplitudeRatio);
    assert_eq!(view.diffraction_model, DiffractionModel::UtdWedge);
    assert_eq!(view.records.len(), 2);
}
