//! End-to-end integration gate (`W3-05`: `GATE-QAC-SIGNAL-PROFILES` + `[GAP-026]`)
//! exercising `QacStorageView::open`, `audio::qac::metadata` (`200..=210`),
//! `audio::qac::signal` (`0x0100`, `0x0101`, `0x0104`, `Rate::divided_by` GCD),
//! `audio::qac::persistence`, `audio::qac::production_verify`, and all QAC profiles
//! (`0x0000..=0x0001`, `0x0003`, `0x0100..=0x0107`), `numerical_bridge`, and `dsp_bridge`.

use qualia_core_db::audio::qac::metadata::{
    OriginBasis, SignalRegistry, WindowKind, SHAPE_SCHEMA, SIGNAL_SCHEMA, TRANSFORM_SCHEMA,
};
use qualia_core_db::audio::qac::numerical::{
    step_fluid_fdtd_in_place, BiotPoroelasticParams, FluidBoundaryCondition, FluidFdtdConfig,
    FluidFdtdState, FluidMedium,
};
use qualia_core_db::audio::qac::persistence::{
    publish_qac, QacPublicationAdmission, QacPublishOptions,
};
use qualia_core_db::audio::qac::production::{ProductionOptions, StftProductionOptions};
use qualia_core_db::audio::qac::production_verify::{
    verify_hann_stft_production_roundtrip, verify_noclobber_conflict_fails_closed,
    verify_pcm_production_roundtrip, verify_sidecar_link_binding, VerificationError,
};
use qualia_core_db::audio::qac::profiles::{
    bridge_continuum_views_biot_poroelastic, bridge_hoa_view_rotate_and_decode,
    bridge_hrir_view_binaural_convolve, bridge_modal_view_to_bs1770_and_mfcc,
    bridge_perceptual_view_zwicker_sones, bridge_ray_haptic_ultrasound_numerical,
    chunk_to_numerical_elastic_records, chunk_to_numerical_pv_records, evaluate_acn_sn3d_dir,
    evaluate_actuator_command_at, export_fdtd_cell_records, find_ultrasound_summary_entry_index,
    plane_wave_record, read_elastic_stress_qac, read_haptic_qac, read_hoa_temporal_qac,
    read_hrir_qac, read_modal_bank_qac, read_perceptual_features_qac, read_pressure_velocity_qac,
    read_ray_contributions_qac, read_ultrasound_qac, write_elastic_stress_qac, write_haptic_qac,
    write_hoa_temporal_qac, write_hrir_qac, write_modal_bank_qac, write_perceptual_features_qac,
    write_pressure_velocity_qac, write_ray_contributions_qac, write_ultrasound_qac,
    AdmittedEntryKind, AdmittedEnvelope, CauchyStress6, ClockSpec, DiffractionModel,
    ElasticMediumSpec, ElasticStressVelocityRecord, FluidMediumModel, FrameSpec, HapticContactSpec,
    HapticRecord, HapticRole, HoaDirectionConvention, HoaNormalization, HoaOrdering,
    HoaProfileSpec, HrirWriteSpec, InstantSpec, ModalRecord, PerceptualScale, PerceptualWriteSpec,
    PressureVelocityRecord, ProfileError, ProfileLimits, ProvenanceSpec, QacProfileRegistry,
    QuaternionXyzw, RationalRate, RayBandValues, RayContributionRecord, StaggeringOffsetsPV,
    UltrasoundAcquisitionSpec, UltrasoundBeamSummaryRecord, UltrasoundRole, UnitDirection3,
    BASE_DELAY_REFERENCE, ELASTIC_VALID_ALL, HAPTIC_VALID_ALL, PV_VALID_ALL, RAY_VALID_ALL,
    REFERENCE_PRESSURE_AIR_PA, US_VALID_IQ, US_VALID_ISPTA, US_VALID_MI, US_VALID_PR, US_VALID_TI,
};
use qualia_core_db::audio::qac::signal::stft::{StftDocument, StftQuantity, StftWriteBuffers};
use qualia_core_db::audio::qac::signal::writer::{
    PcmDocument, PcmQuantity, PcmSamples, Provenance, ProvenanceRole, SignalRelation, WriteBuffers,
    WriteOptions,
};
use qualia_core_db::audio::qac::signal::{
    admit, admit_qac_mixed_entry, Limits as SignalLimits, QacAcquisitionRegistry, QacMixedEntry,
    Rate, Timing,
};
use qualia_core_db::audio::qac::{QacStorageOptions, QacStorageView};
use qualia_core_db::q42::field_envelope::{Family, Limits as EnvelopeLimits, StructuralView};
use qualia_core_db::q42::field_semantics::{
    DomainRegistry, Limits as SemanticLimits, NodeState, Scratch,
};
use qualia_core_db::sensing::acquisition::storage::Limits as AcquisitionLimits;

fn fixture_clock() -> ClockSpec {
    ClockSpec {
        scale: 0,
        epoch_seconds: 1_700_000_000,
        epoch_nanoseconds: 0,
        ticks_per_second: RationalRate::new(48_000, 1).expect("rate"),
    }
}

fn fixture_provenance() -> ProvenanceSpec<'static> {
    ProvenanceSpec {
        role: 0,
        source_digests: &[],
        algorithm: Some("urn:qac:test:w3-05"),
        algorithm_version: Some("1.0"),
        instrument_uri: Some("urn:qac:instrument:ref"),
        authority_receipt_digest: None,
        lineage: Some("w3-05-gate"),
    }
}

#[test]
fn gate_qac_storage_view_legacy_rejection_and_metadata_schemas_200_to_210() {
    for ver in 1u16..=3u16 {
        let mut legacy = vec![0u8; 64];
        legacy[0..4].copy_from_slice(&0x5541_3451u32.to_le_bytes());
        legacy[4..6].copy_from_slice(&ver.to_le_bytes());
        assert!(QacStorageView::open(&legacy, QacStorageOptions::default()).is_err());
    }
    assert_eq!((TRANSFORM_SCHEMA, SHAPE_SCHEMA, SIGNAL_SCHEMA), (201, 205, 210));
    let (sig_reg, prof_reg, acq_reg) = (SignalRegistry, QacProfileRegistry, QacAcquisitionRegistry::default());
    assert_eq!(sig_reg.descriptor_target_schema(4), Some(200));
    assert_eq!(sig_reg.descriptor_target_schema(5), Some(201));
    assert_eq!(sig_reg.descriptor_target_schema(6), Some(202));
    for schema in 200u16..=210u16 {
        assert!(sig_reg.rules(schema, 1).is_some(), "SignalRegistry missing schema {schema}");
        assert!(acq_reg.rules(schema, 1).is_some(), "QacAcquisitionRegistry missing schema {schema}");
        if ![205, 210].contains(&schema) {
            assert!(prof_reg.rules(schema, 1).is_some(), "QacProfileRegistry missing schema {schema}");
        }
    }
}

#[test]
fn gate_qac_signal_pcm_stft_rate_gcd_persistence_and_production_verify() {
    let large_rate = Rate::new(1_000_000_007 * 999_983, 999_999_999_989).expect("large rate");
    let divided = large_rate.divided_by(1_000_000_007).expect("Rate::divided_by large prime GCD");
    assert_eq!(divided, Rate { numerator: 999_983, denominator: 999_999_999_989 });

    let root = tempfile::tempdir().expect("tempdir");
    let samples = [0.25_f32, -0.5, 0.75, -0.25, 0.5, -0.75, 0.125, -0.125];
    let r48k = Rate::new(48_000, 1).expect("48k");
    let pcm_doc = PcmDocument {
        samples: PcmSamples::F32(&samples),
        timing: Timing { scale: 0, epoch_seconds: 1_700_000_000, epoch_nanoseconds: 0, ticks: r48k, samples: r48k, start_tick: 0 },
        stream_id: 1,
        channels: 1,
        channel_order: "mic-0",
        purpose: "pcm-roundtrip-gate",
        quantity: PcmQuantity::DigitalSample,
        relation: SignalRelation::DigitalUnrelatedToPressure,
        units_per_stored_sample: 1.0,
        stored_offset: 0.0,
        provenance: Provenance { role: ProvenanceRole::Measured, algorithm: None, algorithm_version: None, instrument_uri: Some("urn:qualia:mic:0"), lineage: None },
        original: Some(b"opaque-source-capture-header"),
    };

    let (mut metadata, mut payload, mut file) = ([0u8; 4096], [0u8; 4096], [0u8; 16384]);
    let mut out = vec![0u8; 16384];
    let (mut m, mut e) = ([NodeState::default(); 32], [NodeState::default(); 32]);
    let mut scratch = Scratch { metadata: &mut m, entries: &mut e };
    let (pcm_artifact, records) = verify_pcm_production_roundtrip(
        root.path(),
        &pcm_doc,
        WriteBuffers { metadata: &mut metadata, payload: &mut payload, file: &mut file },
        &mut scratch,
        &mut out,
        ProductionOptions::default(),
    )
    .expect("verify_pcm_production_roundtrip");
    assert_eq!(records, 8);
    assert_eq!(pcm_artifact.admission, QacPublicationAdmission::CurrentStructure);
    let pcm_bytes = &out[..pcm_artifact.byte_length as usize];
    verify_noclobber_conflict_fails_closed(root.path(), pcm_bytes, &mut scratch, ProductionOptions::default())
        .expect("noclobber conflict check");

    let pub_again = publish_qac(root.path(), pcm_bytes, QacPublishOptions::default()).expect("idempotent publish_qac");
    assert_eq!(pub_again, pcm_artifact);

    let view = StructuralView::open(pcm_bytes, Family::QacV1Candidate, EnvelopeLimits::default()).expect("structural view");
    let (mut m2, mut e2) = ([NodeState::default(); 32], [NodeState::default(); 32]);
    let mixed_opaque = admit_qac_mixed_entry(
        &view, 1, &mut Scratch { metadata: &mut m2, entries: &mut e2 },
        SemanticLimits::default(), SignalLimits::default(), AcquisitionLimits::default(),
    )
    .expect("admit opaque 0x0104 entry");
    let QacMixedEntry::Signal(opaque_sig) = mixed_opaque else { panic!("expected opaque signal entry") };
    assert_eq!(opaque_sig.mode(), 0x0104);
    assert_eq!(opaque_sig.as_bytes(), b"opaque-source-capture-header");

    let stft_doc = StftDocument {
        samples: PcmSamples::F32(&samples),
        timing: pcm_doc.timing,
        stream_id: 2,
        channels: 1,
        channel_order: "mic-0",
        fft_size: 4,
        hop: 2,
        pcm_purpose: "retained-pcm",
        stft_purpose: "hann-complex-stft",
        quantity: StftQuantity::DigitalSample,
        relation: SignalRelation::DigitalUnrelatedToPressure,
        pcm_provenance: pcm_doc.provenance,
        stft_provenance: Provenance {
            role: ProvenanceRole::Derived,
            algorithm: Some("urn:etalon:signal:hann-stft-f32"),
            algorithm_version: Some("1"),
            instrument_uri: None,
            lineage: Some("hann-stft-gate"),
        },
        original: Some(b"raw-stft-capture"),
    };
    let (mut mono, mut fft, mut win) = ([0.0f32; 16], [0.0f32; 32], [0.0f32; 16]);
    let mut spectra = [0u8; 4096];
    let mut stft_out = vec![0u8; 16384];
    let stft_pub = verify_hann_stft_production_roundtrip(
        root.path(),
        &stft_doc,
        StftWriteBuffers {
            mono: &mut mono, fft: &mut fft, window: &mut win, spectra: &mut spectra,
            metadata: &mut metadata, payload: &mut payload, file: &mut file,
        },
        &mut scratch,
        &mut stft_out,
        StftProductionOptions::default(),
    )
    .expect("verify_hann_stft_production_roundtrip");
    assert_eq!(stft_pub.window_kind, WindowKind::HannSymmetricF32);
    assert_eq!(stft_pub.origin_basis, OriginBasis::SourceEntryFirstSample);
    assert_eq!(stft_pub.stft_frames, 3);

    let legacy_rel = format!("spectral/audio/{}.q4au", blake3::Hash::from(stft_pub.artifact.blake3).to_hex());
    assert_eq!(
        verify_sidecar_link_binding(1, 0, &stft_pub.artifact.blake3, &legacy_rel).unwrap_err(),
        VerificationError::ForbiddenLegacyExtension
    );

    let stft_storage = QacStorageView::open(&stft_out[..stft_pub.artifact.byte_length as usize], QacStorageOptions::default())
        .expect("open stft storage");
    let stft_sig = admit(
        stft_storage.structure(), stft_pub.stft_entry_index, &mut scratch,
        SemanticLimits::default(), SignalLimits::default(),
    )
    .expect("admit stft signal");
    assert_eq!(stft_sig.mode(), 0x0101);
    let _ = WriteOptions::default();
}

#[test]
fn gate_qac_profiles_all_modes_numerical_bridge_and_dsp_bridge() {
    let (clk, prov, frame, limits) = (fixture_clock(), fixture_provenance(), FrameSpec::local_default(), ProfileLimits::default());
    let sr_48k = RationalRate::new(48_000, 1).expect("48k");

    // 1. Mode 0x0000 (PV) + Mode 0x0001 (Elastic) + FDTD export + Biot poroelastic bridge
    let dir = UnitDirection3::new([1.0, 0.0, 0.0]).expect("unit x");
    let pv_rec: PressureVelocityRecord = plane_wave_record([0.0, 0.0, 0.0], 0.0, 2.0, dir, 500.0, 1.2041, 142_000.0);
    let pv_qac = write_pressure_velocity_qac(
        &[pv_rec], &FluidMediumModel::air_20c(), &StaggeringOffsetsPV::collocated(),
        &clk, sr_48k, &frame, &prov, None, limits,
    )
    .expect("write pv");
    let pv_env = AdmittedEnvelope::open_alloc(&pv_qac, limits).expect("pv env");
    assert_eq!(pv_env.inspect_entry_summary(1).expect("pv summary").kind, AdmittedEntryKind::PressureVelocity);
    let pv_view = read_pressure_velocity_qac(&pv_qac, 1, limits).expect("read pv");
    assert_eq!(chunk_to_numerical_pv_records(&pv_view.records).expect("num pv").len(), 1);

    let fdtd_cfg = FluidFdtdConfig {
        dims: [2, 2, 2], spacing_m: [0.05, 0.05, 0.05], dt_s: 2.0e-5,
        medium: FluidMedium::new(1.2041, 142_000.0, 0.0, 0).expect("air"),
        boundary: FluidBoundaryCondition::Rigid,
    };
    let mut fdtd_state = FluidFdtdState::new(&fdtd_cfg).expect("fdtd state");
    fdtd_state.p[0] = 5.0;
    step_fluid_fdtd_in_place(&fdtd_cfg, &mut fdtd_state).expect("fdtd step");
    let exported = export_fdtd_cell_records(&fdtd_cfg, &fdtd_state).expect("export fdtd");
    assert_eq!((exported.len(), exported[0].validity_bits), (8, PV_VALID_ALL));

    let stress = CauchyStress6 { xx: -300.0, yy: -300.0, zz: -300.0, xy: 25.0, yz: 0.0, xz: 0.0 };
    let el_rec = ElasticStressVelocityRecord {
        velocity_m_s: [0.001, 0.0, 0.0], material_ref: 0,
        stress_xx_yy_zz_xy_yz_xz_pa: stress.to_wire_f32(),
        displacement_m: [1e-6, 0.0, 0.0], validity_bits: ELASTIC_VALID_ALL,
    };
    let el_qac = write_elastic_stress_qac(&[el_rec], &ElasticMediumSpec::steel(), None, &clk, sr_48k, &frame, &prov, None, limits)
        .expect("write elastic");
    let el_view = read_elastic_stress_qac(&el_qac, 1, limits).expect("read elastic");
    assert_eq!(chunk_to_numerical_elastic_records(&el_view.records).expect("num el").len(), 1);

    let biot_params = BiotPoroelasticParams {
        porosity: 0.22, rho_fluid_kg_m3: 1000.0, rho_solid_kg_m3: 2650.0, tortuosity_inf: 1.8,
        permeability_m2: 1e-12, viscosity_pa_s: 1e-3, biot_willis_alpha: 0.78,
        biot_modulus_m_pa: 1.1e10, frame_bulk_modulus_pa: 1.4e10, frame_shear_modulus_pa: 9.5e9,
    };
    let (biot_disp, p_eff) = bridge_continuum_views_biot_poroelastic(&pv_view, &el_view, &biot_params, 6283.185)
        .expect("biot bridge");
    assert!(biot_disp.c_p1_m_s > biot_disp.c_s_m_s && p_eff > 200.0);

    // 2. Mode 0x0102 (HOA) + Mode 0x0103 (HRIR) + dsp_bridge
    let y_px: Vec<f32> = evaluate_acn_sn3d_dir(1, [1.0, 0.0, 0.0]).expect("+X").into_iter().take(4).map(|v| v as f32).collect();
    let hoa_spec = HoaProfileSpec {
        order: 1, ordering: HoaOrdering::Acn, normalization: HoaNormalization::Sn3d,
        direction_convention: HoaDirectionConvention::ListenerToSource, units: "Pa", sample_rate: sr_48k,
    };
    let hoa_qac = write_hoa_temporal_qac(&[&y_px, &y_px], &hoa_spec, &clk, &frame, &prov, None, limits).expect("write hoa");
    let hoa_view = read_hoa_temporal_qac(&hoa_qac, 1, limits).expect("read hoa");
    let speakers = [[1.0f32, 0.0, 0.0], [0.0, 1.0, 0.0], [-1.0, 0.0, 0.0], [0.0, -1.0, 0.0]];
    let spk_out = bridge_hoa_view_rotate_and_decode(&hoa_view, [0.0, 0.0, 0.0, 1.0], &speakers, true).expect("hoa dsp");
    assert_eq!(spk_out.len(), 8);

    let mut impulses = vec![0.0f32; 16];
    impulses[0] = 1.0;
    impulses[8 + 1] = 0.5;
    let hrir_spec = HrirWriteSpec {
        representation: "HRIR", taps: 8, receivers: 2, measurements: 1, impulse_sample_rate: sr_48k,
        delay_reference: BASE_DELAY_REFERENCE, sofa_convention: Some("SimpleFreeFieldHRIR"), units: "1",
        delays_samples: Some(&[2.0, 3.0]), source_positions_xyz: Some(&[[1.0, 0.0, 0.0]]),
        receiver_positions_xyz: Some(&[[0.0, 0.09, 0.0], [0.0, -0.09, 0.0]]), measurement_cadence: None,
    };
    let hrir_qac = write_hrir_qac(&impulses, &hrir_spec, &frame, &prov, limits).expect("write hrir");
    let hrir_view = read_hrir_qac(&hrir_qac, 1, limits).expect("read hrir");
    let (l_out, r_out) = bridge_hrir_view_binaural_convolve(&hrir_view, 0, &[1.0, 0.0]).expect("hrir dsp");
    assert!((l_out[2] - 1.0).abs() < 1e-6 && (r_out[4] - 0.5).abs() < 1e-6);

    // 3. Mode 0x0104 (Modal) + Mode 0x0105 (Perceptual) + dsp_bridge
    let m0 = ModalRecord {
        frequency_hz: 440.0, amplitude_pa: 0.8, decay_rate_s1: 1.5, phase_rad: 0.0,
        mode_shape_index: 0, material_ref: 0, excitation_index: 0,
    };
    let modal_qac = write_modal_bank_qac(&[m0], &[1.0], 1, &[1.0], None, &clk, sr_48k, &prov, limits).expect("write modal");
    let modal_view = read_modal_bank_qac(&modal_qac, 1, limits).expect("read modal");
    let (bs1770, mel_out, mfcc_out) = bridge_modal_view_to_bs1770_and_mfcc(&modal_view, 0, 8_000, 4_000).expect("modal dsp");
    assert!(bs1770.total_blocks >= 1 && mel_out.len() == 8 && mfcc_out.len() == 8);

    let perc_spec = PerceptualWriteSpec {
        algorithm_uri: "urn:qac:zwicker:24bark", algorithm_version: "1", scale: PerceptualScale::DbSpl,
        feature_count: 24, feature_names: "bark1..24", centers_hz: None, filter_coefficients: None,
        reference_pressure_pa: Some(REFERENCE_PRESSURE_AIR_PA), channel_weights: None, gating_profile: None,
        frame_rate: RationalRate::new(100, 1).expect("100hz"),
    };
    let mut bark_frame = [-100.0f32; 24];
    bark_frame[8] = 40.0;
    let perc_qac = write_perceptual_features_qac(&[&bark_frame], &perc_spec, &clk, &prov, None, limits).expect("write perceptual");
    let perc_view = read_perceptual_features_qac(&perc_qac, 1, limits).expect("read perceptual");
    let (zw_report, _) = bridge_perceptual_view_zwicker_sones(&perc_view, 0).expect("zwicker dsp");
    assert!((zw_report.total_loudness_sones - 1.0).abs() < 0.05);

    // 4. Mode 0x0003 (Ray) + Mode 0x0106 (Haptic) + Mode 0x0107 (Ultrasound) + dsp_bridge
    let ray = RayContributionRecord {
        origin_m: [0.0, 0.0, 0.0], distance_m: 3.43, direction: [1.0, 0.0, 0.0], delay_s: 0.01,
        ray_id: 1, band_row: 0, material_ref: 0, specular_count: 0, diffraction_count: 1,
        scattering_coefficient: 0.1, doppler_ratio: 1.0, validity_bits: RAY_VALID_ALL,
    };
    let ray_vals = RayBandValues::SignedPressureRatio { bands: 1, values: vec![0.5] };
    let ray_qac = write_ray_contributions_qac(
        &[ray], &[1000.0], &ray_vals, 1.0, DiffractionModel::UtdWedge, None, None, &clk, &frame, &prov, limits,
    )
    .expect("write ray");
    let ray_view = read_ray_contributions_qac(&ray_qac, 1, limits).expect("read ray");

    let (dev_d, auth_d) = ([0x11u8; 32], [0x22u8; 32]);
    let hap_spec = HapticContactSpec {
        role: HapticRole::ActuatorCommand {
            device_profile_digest: dev_d, authority_receipt_digest: auth_d,
            command_expiry: InstantSpec::new(1, 500, 0, 1).expect("exp"), receptor_model_uri: None,
        },
        contact_frame: FrameSpec::local_default(), contact_area_m2: Some(1.0e-4),
    };
    let hap_rec = HapticRecord {
        contact_position_m: [0.0, 0.0, 0.0], contact_area_m2: 1.0e-4, force_local_xyz_n: [0.0, 0.0, -2.0],
        normal_indentation_m: 0.001, displacement_local_xyz_m: [0.0, 0.0, -0.001],
        carrier_hz: 200.0, validity_bits: HAPTIC_VALID_ALL, contact_ref: 4,
    };
    let hap_qac = write_haptic_qac(&[hap_rec], &hap_spec, None, &clk, sr_48k, &frame, &prov, None, limits).expect("write haptic");
    let hap_view = read_haptic_qac(&hap_qac, 1, limits).expect("read haptic");
    assert!(evaluate_actuator_command_at(
        &hap_view, &InstantSpec::new(1, 100, 0, 1).expect("t=100"), &clk, &|d, a| *d == dev_d && *a == auth_d
    ).is_ok());

    let us_rec = UltrasoundBeamSummaryRecord {
        position_m: [0.0, 0.0, 0.03], center_frequency_hz: 2.0e6, peak_rarefactional_pressure_pa: 1.5e6,
        intensity_ispta_w_m2: 1000.0, mechanical_index: 0.67, thermal_index: 1.0,
        complex_envelope_iq: [0.8, -0.2], axial_velocity_m_s: 0.0, harmonic_ratio_linear: 0.0,
        validity_bits: US_VALID_PR | US_VALID_ISPTA | US_VALID_MI | US_VALID_TI | US_VALID_IQ, acquisition_ref: 4,
    };
    let us_spec = UltrasoundAcquisitionSpec {
        role: UltrasoundRole::BeamSummary, center_frequency_hz: Some(2.0e6), element_positions_m: None,
        channel_map: None, transmit_delays_s: None, demodulation_phase_rad: Some(0.0), prf: None,
        derating_model_uri: None, exposure_context: Some("soft-tissue"), thermal_model_uri: Some("urn:qac:ti:tis-v1"),
        instrument_digest: None, authority_receipt_digest: None, target_frame: None, calibration: None,
    };
    let us_qac = write_ultrasound_qac(&[us_rec], &us_spec, None, &clk, sr_48k, &frame, &prov, None, limits).expect("write ultrasound");
    let us_idx = find_ultrasound_summary_entry_index(&us_qac, limits).expect("us idx");
    let us_view = read_ultrasound_qac(&us_qac, us_idx, limits).expect("read ultrasound");

    let (utd_mag, stress_pa, bio_res, tis_est) =
        bridge_ray_haptic_ultrasound_numerical(&ray_view, &hap_view, &us_view, 343.0).expect("ray/haptic/ultrasound bridge");
    assert!(utd_mag > 0.0 && stress_pa > 10_000.0);
    assert!(bio_res.steady_state_delta_t_kelvin > 0.0 && (tis_est.thermal_index - 1.0).abs() < 1e-9);
    assert!(matches!(QuaternionXyzw::from_xyzw([0.0, 0.0, 0.0, 2.0]), Err(ProfileError::InvalidQuaternion)));
}
