//! Unit and physics tests for Q06 (Modal, Perceptual + Q4AU v2) & Q07 (Haptic, Ultrasound).

use std::f64::consts::PI;

use super::envelope::*;
use super::haptic::*;
use super::modal::*;
use super::perceptual::*;
use super::tests_q03_q05::{test_clock, test_provenance};
use super::types::*;
use super::ultrasound::*;
use crate::audio::audio_spectral_sheet::{bake_spectral_v2_from_samples, v2_sidecar_size};

#[test]
fn test_q06_modal_t60_factor_of_two_d0_silence_and_opaque_0104_disambiguation() {
    // 32-byte wire layout + exact amplitude T60 = ln(1000)/d (never ln(1000)/(2*d)).
    assert_eq!(MODAL_RECORD_STRIDE, 32);
    let d = 2.302585092994046f32; // ln(10), so -60 dB occurs at exact T60 = 3.0 s!
    let m1 = ModalRecord {
        frequency_hz: 440.0,
        amplitude_pa: 1.0,
        decay_rate_s1: d,
        phase_rad: 0.0,
        mode_shape_index: 0,
        material_ref: 0,
        excitation_index: 0,
    };
    let t60 = m1.t60_seconds().expect("valid").expect("finite t60");
    assert!((t60 - 3.0).abs() < 1e-5, "T60 must be 3.0 s, got {t60}");
    let amp_at_t60 = (f64::from(m1.amplitude_pa) * (-f64::from(d) * t60).exp()).abs();
    assert!((amp_at_t60 - 1.0e-3).abs() < 1e-6);
    // Factor-of-2 bug check: t = T60/2 gives 10^(-3/2) ~= 0.03162, NOT 0.001!
    let amp_at_half = (f64::from(m1.amplitude_pa) * (-f64::from(d) * (0.5 * t60)).exp()).abs();
    assert!((amp_at_half - 10.0f64.powf(-1.5)).abs() < 1e-5);
    assert!(m1.damping_ratio().expect("zeta") > 0.0);

    // Undamped mode d=0 -> Ok(None), never +inf.
    let m0 = ModalRecord {
        frequency_hz: 220.0,
        amplitude_pa: 0.5,
        decay_rate_s1: 0.0,
        phase_rad: 0.5 * PI as f32,
        mode_shape_index: 1,
        material_ref: 0,
        excitation_index: 1,
    };
    assert_eq!(m0.t60_seconds().expect("valid d=0"), None);

    // Mode 3 explicitly Silence -> wire masked bytes zero, excluded from synthesis.
    let m_silent = ModalRecord {
        frequency_hz: 0.0,
        amplitude_pa: 0.0,
        decay_rate_s1: 0.0,
        phase_rad: 0.0,
        mode_shape_index: 0,
        material_ref: 0,
        excitation_index: 0,
    };
    let modes = [m1, m0, m_silent];
    let statuses = [SampleStatus::Valid, SampleStatus::Valid, SampleStatus::Silence];
    let mode_shapes = [1.0f32, -0.5, 0.25, 0.75]; // 2 shapes x 2 DOFs
    let excitations = [2.0f32, 0.5];

    let qac = write_modal_bank_qac(
        &modes,
        &mode_shapes,
        2,
        &excitations,
        Some(&statuses),
        &test_clock(),
        RationalRate::new(48_000, 1).expect("sr"),
        &test_provenance(),
        ProfileLimits::default(),
    )
    .expect("write modal");
    let view = read_modal_bank_qac(&qac, 1, ProfileLimits::default()).expect("read modal");
    assert_eq!(view.records.len(), 3);
    assert_eq!(view.shape_dofs, 2);

    let y_synth = synthesize_modal_sample(&modes, Some(&statuses), &mode_shapes, 2, 1, &excitations, 0.001)
        .expect("synth");
    let y_manual = -0.5 * 2.0 * m1.evaluate_at(0.001).expect("m1")
        + 0.75 * 0.5 * m0.evaluate_at(0.001).expect("m0");
    assert!((y_synth - y_manual).abs() < 1e-6);

    // Out-of-bounds mode_shape_index or excitation_index fails closed.
    let bad_mode = [ModalRecord {
        mode_shape_index: 99,
        ..m1
    }];
    assert!(matches!(
        write_modal_bank_qac(
            &bad_mode,
            &mode_shapes,
            2,
            &excitations,
            None,
            &test_clock(),
            RationalRate::new(48_000, 1).expect("sr"),
            &test_provenance(),
            ProfileLimits::default(),
        ),
        Err(ProfileError::OutOfBoundsIndex(_))
    ));

    // Mode 0x0104 opaque_source_preservation (stride=1, no Schema 206) must be rejected by read_modal_bank_qac.
    let mut meta = MetadataBuilder::new();
    meta.push_provenance(3, &test_provenance()).expect("prov");
    meta.push_descriptor(
        6,
        &DescriptorSpec {
            clock_id: None,
            frame_id: None,
            provenance_id: 3,
            medium_id: None,
            transform_id: None,
            calibration_id: None,
            quantity: "opaque-original",
            units: "1",
            channels: Some(1),
            layout: Some("byte"),
            scalar: Some(AuxScalar::U8),
            model_uri: "urn:qac:mode:0x0104:opaque-original:v1",
            sample_rate: None,
            valid_record_count: None,
            validity_mask_entry: None,
            source_entries: &[],
            extension_metadata: &[],
        },
    )
    .expect("desc");
    let opaque_0104 = assemble_qac_v1(
        &meta,
        &[OwnedEntry {
            mode: MODE_OPAQUE_SOURCE,
            flags: 1,
            stream_id: 0,
            frame_id: 0,
            start_tick: 0,
            time_count: 0,
            spatial_key: 0,
            record_count: 32,
            stride: 1,
            descriptor_id: 6,
            payload: vec![0u8; 32],
        }],
        ProfileLimits::default(),
    )
    .expect("opaque 0104");
    assert!(read_modal_bank_qac(&opaque_0104, 1, ProfileLimits::default()).is_err());
}

#[test]
fn test_q06_perceptual_scales_gammatone_bs1770_and_q4au_v2_migration() {
    // ERB / Bark / HTK Mel exact identities.
    assert!((erb_bandwidth_hz(1000.0) - 24.7 * 5.37).abs() < 1e-9);
    assert!(hz_to_erb_number(4000.0) > hz_to_erb_number(1000.0));
    assert!(hz_to_bark(4000.0) > hz_to_bark(1000.0));
    let mel = hz_to_mel_htk(1000.0);
    assert!((mel_to_hz_htk(mel) - 1000.0).abs() < 1e-9);

    // 4th-order Gammatone FIR filterbank unity gain at fc.
    let ir_f32 = gammatone_fir_impulse(1000.0, 48_000.0, 1024).expect("gammatone");
    let ir_f64: Vec<f64> = ir_f32.iter().map(|&v| f64::from(v)).collect();
    let gain_fc = dtft_magnitude(&ir_f64, 1000.0 / 48_000.0);
    assert!((gain_fc - 1.0).abs() < 1e-4, "DTFT gain at fc = {gain_fc}");

    // dB-SPL reference pressure & exact zero -> SampleStatus::Silence (not -inf).
    assert!((rms_pa_to_db_spl(REFERENCE_PRESSURE_AIR_PA, REFERENCE_PRESSURE_AIR_PA).expect("0 dB") - 0.0).abs() < 1e-12);
    assert_eq!(
        rms_pa_to_db_spl(0.0, REFERENCE_PRESSURE_AIR_PA),
        Err(SampleStatus::Silence)
    );

    // ITU-R BS.1770 two-stage gating: quiet block below -10 LU relative threshold excluded;
    // below -70 LKFS excluded as Silence.
    let silence_blk = [0.0f64];
    let very_quiet = [1e-9f64]; // -0.691 - 90 = -90.691 LKFS < -70 LKFS
    assert_eq!(
        bs1770_integrated_lufs(&[&silence_blk, &very_quiet], &[1.0]),
        Err(SampleStatus::Silence)
    );
    let loud = [0.05f64]; // ~ -13.7 LKFS
    let quiet_rel = [0.0005f64]; // ~ -33.7 LKFS (< -23.7 relative threshold -> gated out)
    let lufs_gated = bs1770_integrated_lufs(&[&loud, &quiet_rel], &[1.0]).expect("gated lufs");
    let lufs_loud_only = bs1770_integrated_lufs(&[&loud], &[1.0]).expect("loud lufs");
    assert!((lufs_gated - lufs_loud_only).abs() < 1e-12);

    // Perceptual QAC v1 requires reference_pressure_pa for DbSpl and gating_profile for Lufs.
    let bad_db_spec = PerceptualWriteSpec {
        algorithm_uri: "urn:qac:alg:spl:v1",
        algorithm_version: "1",
        scale: PerceptualScale::DbSpl,
        feature_count: 1,
        feature_names: "band0",
        centers_hz: Some(&[1000.0]),
        filter_coefficients: Some(&ir_f32[..64]),
        reference_pressure_pa: None,
        channel_weights: None,
        gating_profile: None,
        frame_rate: RationalRate::new(100, 1).expect("fr"),
    };
    assert!(write_perceptual_features_qac(
        &[&[60.0]],
        &bad_db_spec,
        &test_clock(),
        &test_provenance(),
        None,
        ProfileLimits::default(),
    )
    .is_err());

    let ok_lufs_spec = PerceptualWriteSpec {
        scale: PerceptualScale::Lufs,
        reference_pressure_pa: None,
        channel_weights: Some(&[1.0]),
        gating_profile: Some("ITU-R BS.1770-5"),
        ..bad_db_spec
    };
    let lufs_qac = write_perceptual_features_qac(
        &[&[-14.2], &[-15.0]],
        &ok_lufs_spec,
        &test_clock(),
        &test_provenance(),
        None,
        ProfileLimits::default(),
    )
    .expect("write lufs");
    let lufs_view = read_perceptual_features_qac(&lufs_qac, 1, ProfileLimits::default()).expect("read lufs");
    assert_eq!(lufs_view.scale, PerceptualScale::Lufs);
    assert_eq!(lufs_view.gating_profile, Some("ITU-R BS.1770-5"));
    assert_eq!(lufs_view.centers_hz.as_deref(), Some(&[1000.0f32][..]));

    // Lossless Q4AU v2 mel/MFCC migration retaining exact opaque original + plane0 + mel + MFCC.
    const FRAME: usize = 128;
    const HOP: usize = 64;
    let samples: Vec<f32> = (0..FRAME * 4)
        .map(|i| (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / 16_000.0).sin())
        .collect();
    let frame_count = (samples.len() - FRAME) / HOP + 1;
    let bins = FRAME / 2 + 1;
    let mut v2_buf = vec![0u8; v2_sidecar_size(frame_count, bins, 16, 8)];
    bake_spectral_v2_from_samples(&samples, FRAME, HOP, 16_000, 16, 8, &mut v2_buf).expect("bake v2");
    let migrated_v2 = migrate_legacy_q4au_v2_features(&v2_buf, ProfileLimits::default()).expect("migrate v2");

    // Verify opaque entry 1 matches original bytes, entry 3 is mel (16 bands), entry 4 is MFCC (8 bands).
    let env = AdmittedEnvelope::open_alloc(&migrated_v2, ProfileLimits::default()).expect("open migrated v2");
    assert_eq!(env.verify_chunk(1).expect("orig v2"), v2_buf.as_slice());
    let mel_view = read_perceptual_features_qac(&migrated_v2, 3, ProfileLimits::default()).expect("read mel");
    let mfcc_view = read_perceptual_features_qac(&migrated_v2, 4, ProfileLimits::default()).expect("read mfcc");
    assert_eq!(mel_view.scale, PerceptualScale::Mel);
    assert_eq!(mfcc_view.scale, PerceptualScale::Mfcc);
    assert_eq!(mel_view.chunk.frame_count(), frame_count);
}

#[test]
fn test_q07_haptic_action_convention_passive_storage_vs_expiry_actuation_gate() {
    let f_local = [10.0_f32, 0.0, 0.0];
    let q90z = QuaternionXyzw::from_axis_angle([0.0, 0.0, 1.0], PI * 0.5).expect("q90z");
    let f_parent = rotate_force_to_parent(f_local, q90z, ForceActionConvention::ForceOnBody).expect("rot f");
    assert!((f_parent[1] - 10.0).abs() < 1e-12);
    let tau = torque_about_origin_n_m([0.0, 0.0, 0.2], [5.0, 0.0, 0.0]).expect("torque");
    assert!((tau[1] - 1.0).abs() < 1e-12); // (0,0,0.2) x (5,0,0) = (0, 1.0, 0)
    assert!((contact_pressure_pa(2.0, 1e-4).expect("p") - 20_000.0).abs() < 1e-6);
    assert!((hooke_spring_force_n(500.0, 0.004, 0.5).expect("hooke") - 2.5).abs() < 1e-12);

    // Explicit sign inversion between ForceOnBody and ForceOnDevice.
    let on_body = ForceActionConvention::ForceOnDevice.to_force_on_body([0.0, 0.0, -3.5]);
    assert_eq!(on_body, [0.0, 0.0, 3.5]);

    let clk = test_clock();
    let rec = HapticRecord {
        contact_position_m: [0.0, 0.0, 0.0],
        contact_area_m2: 1.0e-4,
        force_local_xyz_n: [0.0, 0.0, 2.5],
        normal_indentation_m: 0.002,
        displacement_local_xyz_m: [0.0, 0.0, -0.002],
        carrier_hz: 200.0,
        validity_bits: HAPTIC_VALID_ALL,
        contact_ref: 4,
    };
    let sr = RationalRate::new(1000, 1).expect("1k");

    // 1. Passive Measurement & ReceptorModel roles inspect cleanly, but evaluate_actuator_command_at rejects both!
    let meas_spec = HapticContactSpec {
        role: HapticRole::Measurement {
            receptor_model_uri: Some("urn:qac:receptor:sai-v1".to_owned()),
        },
        contact_frame: FrameSpec::local_default(),
        contact_area_m2: Some(1.0e-4),
    };
    let meas_qac = write_haptic_qac(
        &[rec],
        &meas_spec,
        Some(&[0.75]),
        &clk,
        sr,
        &FrameSpec::local_default(),
        &test_provenance(),
        None,
        ProfileLimits::default(),
    )
    .expect("write meas haptic");
    let meas_doc = read_haptic_qac(&meas_qac, 1, ProfileLimits::default()).expect("read meas haptic");
    let now_early = InstantSpec::new(1, 50, 0, 1).expect("t=50");
    assert!(matches!(
        evaluate_actuator_command_at(&meas_doc, &now_early, &clk, &|_, _| true),
        Err(ProfileError::ActuationAuthority(_))
    ));

    // 2. ActuatorCommand role with exact rational expiry [tick 100 + 1/3] -> read_haptic_qac succeeds passively,
    // while evaluate_actuator_command_at enforces authority digest, clock scale, and strict now < expiry.
    let dev_digest = [0x11u8; 32];
    let auth_digest = [0x22u8; 32];
    let expiry = InstantSpec::new(1, 100, 1, 3).expect("expiry");
    let cmd_spec = HapticContactSpec {
        role: HapticRole::ActuatorCommand {
            device_profile_digest: dev_digest,
            authority_receipt_digest: auth_digest,
            command_expiry: expiry,
            receptor_model_uri: None,
        },
        contact_frame: FrameSpec::local_default(),
        contact_area_m2: Some(1.0e-4),
    };
    let cmd_qac = write_haptic_qac(
        &[rec],
        &cmd_spec,
        None,
        &clk,
        sr,
        &FrameSpec::local_default(),
        &test_provenance(),
        None,
        ProfileLimits::default(),
    )
    .expect("write cmd haptic");
    let cmd_doc = read_haptic_qac(&cmd_qac, 1, ProfileLimits::default()).expect("read cmd passive");

    let valid_auth = |d: &[u8; 32], a: &[u8; 32]| *d == dev_digest && *a == auth_digest;
    assert!(evaluate_actuator_command_at(&cmd_doc, &InstantSpec::new(1, 100, 1, 4).expect("t<exp"), &clk, &valid_auth).is_ok());
    assert!(matches!(
        evaluate_actuator_command_at(&cmd_doc, &expiry, &clk, &valid_auth),
        Err(ProfileError::ActuationAuthority(_))
    ));
    assert!(matches!(
        evaluate_actuator_command_at(&cmd_doc, &InstantSpec::new(1, 100, 1, 2).expect("t>exp"), &clk, &valid_auth),
        Err(ProfileError::ActuationAuthority(_))
    ));
    assert!(matches!(
        evaluate_actuator_command_at(&cmd_doc, &now_early, &clk, &|_, _| false),
        Err(ProfileError::ActuationAuthority(_))
    ));
    let mut bad_clk = clk;
    bad_clk.scale = 1;
    assert!(matches!(
        evaluate_actuator_command_at(&cmd_doc, &now_early, &bad_clk, &valid_auth),
        Err(ProfileError::ClockScaleMismatch)
    ));
}

#[test]
fn test_q07_ultrasound_raw_rf_iq_mi_doppler_high_mi_preserve_and_ti_guard() {
    // 1. Standard FDA derated MI formula.
    let mi = mechanical_index_derated(2.0e6, 4.0e6, 2.0, 0.3).expect("mi");
    let expected_mi = (2.0 * 10.0f64.powf(-(0.3 * 2.0 * 4.0) / 20.0)) / 4.0f64.sqrt();
    assert!((mi - expected_mi).abs() < 1e-12);

    // 2. Positive-carrier RF <-> IQ complex envelope equivalence: +Q sign check.
    let fc = 2.5e6;
    let fs = 40.0e6; // 16 samples/cycle; 32 samples = 2 full carrier periods!
    let phi0 = 0.4;
    let rf: Vec<f64> = (0..32)
        .map(|n| modulate_positive_carrier_rf(1.25, -0.75, fc, phi0, (n as f64) / fs))
        .collect();
    let iq = demodulate_positive_carrier_iq(&rf, fs, fc, phi0).expect("demod iq");
    assert!((iq[0] - 1.25).abs() < 1e-10, "I mismatch: {}", iq[0]);
    assert!((iq[1] + 0.75).abs() < 1e-10, "Q mismatch: {}", iq[1]);

    // 3. Doppler velocity & roundtrip ToF delay.
    let v_dop = doppler_axial_velocity_m_s(2_000.0, 2.5e6, 1540.0).expect("doppler");
    assert!((v_dop - 1540.0 * 2_000.0 / 5.0e6).abs() < 1e-12);
    let tof = element_roundtrip_delay_s([0.0, 0.0, 0.0], [0.003, 0.0, 0.0], [0.0, 0.0, 0.004], 1540.0)
        .expect("tof");
    assert!((tof - (0.004 + 0.005) / 1540.0).abs() < 1e-12);

    // 4. Write passive high-MI ultrasound container WITHOUT thermal context when US_VALID_TI == 0 ->
    // MUST succeed without clinical MI/ISPTA veto; preserves raw RF + geometry + delays!
    let rf_f32: Vec<f32> = rf.iter().map(|&v| v as f32).collect();
    let elem_pos = [[-0.001f32, 0.0, 0.0], [0.0, 0.0, 0.0], [0.001, 0.0, 0.0]];
    let ch_map = [0u32, 1, 2];
    let delays = [1.2e-7, 0.0, 1.2e-7];
    let raw_payload = RawUltrasoundPayload::RawRfF32 {
        channels: 1,
        sample_rate: RationalRate::new(40_000_000, 1).expect("40m"),
        samples_interleaved: &rf_f32,
    };
    let mut rec = UltrasoundBeamSummaryRecord {
        position_m: [0.0, 0.0, 0.02],
        center_frequency_hz: fc as f32,
        peak_rarefactional_pressure_pa: 3.8e6,
        intensity_ispta_w_m2: 12_000.0, // > 720 mW/cm^2 diagnostic limit, stored faithfully as physics
        mechanical_index: 2.4,
        thermal_index: 0.0,
        complex_envelope_iq: [1.25, -0.75],
        axial_velocity_m_s: v_dop as f32,
        harmonic_ratio_linear: 0.15,
        validity_bits: US_VALID_PR | US_VALID_ISPTA | US_VALID_MI | US_VALID_IQ | US_VALID_AXIAL_VEL | US_VALID_HARMONIC,
        acquisition_ref: 4,
    };
    let acq_spec = UltrasoundAcquisitionSpec {
        role: UltrasoundRole::BeamSummary,
        center_frequency_hz: Some(fc),
        element_positions_m: Some(&elem_pos),
        channel_map: Some(&ch_map),
        transmit_delays_s: Some(&delays),
        demodulation_phase_rad: Some(phi0),
        prf: Some(RationalRate::new(4000, 1).expect("prf")),
        derating_model_uri: Some("urn:qac:ultrasound:derating:fda-0.3db"),
        exposure_context: None,
        thermal_model_uri: None,
        instrument_digest: Some([0x33u8; 32]),
        authority_receipt_digest: None,
        target_frame: None,
        calibration: None,
    };
    let qac = write_ultrasound_qac(
        &[rec],
        &acq_spec,
        Some(&raw_payload),
        &test_clock(),
        RationalRate::new(60, 1).expect("fps"),
        &FrameSpec::local_default(),
        &test_provenance(),
        None,
        ProfileLimits::default(),
    )
    .expect("high-MI passive ultrasound must succeed");
    let sum_idx = find_ultrasound_summary_entry_index(&qac, ProfileLimits::default()).expect("summary idx");
    let view = read_ultrasound_qac(&qac, sum_idx, ProfileLimits::default()).expect("read us");
    assert_eq!(view.records.get(0).expect("r0").mechanical_index, 2.4);
    assert_eq!(view.channel_map.as_deref(), Some(&ch_map[..]));
    assert!(view.raw_source_bytes.is_some());

    // 5. Setting US_VALID_TI without thermal_model_URI and exposure_context must fail closed!
    rec.validity_bits |= US_VALID_TI;
    rec.thermal_index = 1.1;
    assert!(matches!(
        write_ultrasound_qac(
            &[rec],
            &acq_spec,
            None,
            &test_clock(),
            RationalRate::new(60, 1).expect("fps"),
            &FrameSpec::local_default(),
            &test_provenance(),
            None,
            ProfileLimits::default(),
        ),
        Err(ProfileError::MissingThermalModel(_))
    ));
}
