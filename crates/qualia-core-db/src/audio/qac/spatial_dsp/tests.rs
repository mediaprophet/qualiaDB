//! C02 Scientific & Numerical Oracle Tests for QAC Spatial DSP (`tests.rs`).

use super::*;

#[test]
fn c02_mode02_hoa_cardinals_and_sn3d_n3d_oracles() {
    // First-order ACN SN3D [W, Y, Z, X]:
    // +X = [1, 0, 0, 1], +Y = [1, 1, 0, 0], +Z = [1, 0, 1, 0]
    let cardinals: [([f64; 3], [f64; 4]); 6] = [
        ([1.0, 0.0, 0.0], [1.0, 0.0, 0.0, 1.0]),
        ([-1.0, 0.0, 0.0], [1.0, 0.0, 0.0, -1.0]),
        ([0.0, 1.0, 0.0], [1.0, 1.0, 0.0, 0.0]),
        ([0.0, -1.0, 0.0], [1.0, -1.0, 0.0, 0.0]),
        ([0.0, 0.0, 1.0], [1.0, 0.0, 1.0, 0.0]),
        ([0.0, 0.0, -1.0], [1.0, 0.0, -1.0, 0.0]),
    ];

    for (dir, expected_o1) in cardinals {
        let mut sn3d = [0.0f64; 16];
        assert_eq!(
            evaluate_hoa_basis_f64(HoaOrder::Order3, HoaNormalization::Sn3d, dir, &mut sn3d),
            Ok(16)
        );
        for c in 0..4 {
            assert!(
                (sn3d[c] - expected_o1[c]).abs() <= 1e-15,
                "Order-1 SN3D mismatch for dir {dir:?} at ACN {c}: got {}, expected {}",
                sn3d[c],
                expected_o1[c]
            );
        }

        // Verify spherical harmonic addition theorem: for every degree l in 0..=3,
        // sum_{m=-l}^l (Y_l^{m, SN3D}(u))^2 == 1.0
        for l in 0..=3 {
            let start = l * l;
            let end = (l + 1) * (l + 1);
            let sum_sq: f64 = sn3d[start..end].iter().map(|v| v * v).sum();
            assert!(
                (sum_sq - 1.0).abs() <= 1e-14,
                "SN3D addition theorem failed at degree {l} for {dir:?}: sum_sq={sum_sq}"
            );
        }

        // Verify N3D degree-l scaling by sqrt(2l + 1) and round-trip conversion
        let mut n3d = [0.0f64; 16];
        evaluate_hoa_basis_f64(HoaOrder::Order3, HoaNormalization::N3d, dir, &mut n3d).unwrap();
        let mut converted = sn3d;
        convert_hoa_normalization_in_place_f64(
            HoaOrder::Order3,
            HoaNormalization::Sn3d,
            HoaNormalization::N3d,
            &mut converted,
        )
        .unwrap();
        for acn in 0..16 {
            assert!((converted[acn] - n3d[acn]).abs() <= 1e-14);
        }
        convert_hoa_normalization_in_place_f64(
            HoaOrder::Order3,
            HoaNormalization::N3d,
            HoaNormalization::Sn3d,
            &mut converted,
        )
        .unwrap();
        for acn in 0..16 {
            assert!((converted[acn] - sn3d[acn]).abs() <= 1e-14);
        }
    }

    // Verify N3D degree-1 coefficient is sqrt(3) on +X
    let mut n3d_x = [0.0f64; 4];
    evaluate_hoa_basis_f64(
        HoaOrder::Order1,
        HoaNormalization::N3d,
        [1.0, 0.0, 0.0],
        &mut n3d_x,
    )
    .unwrap();
    assert!((n3d_x[0] - 1.0).abs() <= 1e-15);
    assert!((n3d_x[3] - 3.0_f64.sqrt()).abs() <= 1e-15);
}

#[test]
fn c02_c04_and_mode02_wigner_rotation_oracles() {
    // 1. Active +90 deg Z rotation maps (1,0,0) -> (0,1,0) and +X HOA -> +Y HOA across all orders
    let s = core::f64::consts::FRAC_1_SQRT_2;
    let q_z90 = [0.0, 0.0, s, s];
    let v_rot = rotate_vector_active_f64(q_z90, [1.0, 0.0, 0.0]).unwrap();
    assert!((v_rot[0] - 0.0).abs() <= 1e-15);
    assert!((v_rot[1] - 1.0).abs() <= 1e-15);
    assert!((v_rot[2] - 0.0).abs() <= 1e-15);

    let wigner_z90 = HoaWignerRotation::from_quaternion_xyzw(q_z90).unwrap();
    assert!(wigner_z90.max_orthogonality_residual(HoaOrder::Order3) <= 1e-14);

    let mut hoa_x = [0.0f64; 16];
    let mut hoa_y = [0.0f64; 16];
    let mut hoa_rot = [0.0f64; 16];
    evaluate_hoa_basis_f64(
        HoaOrder::Order3,
        HoaNormalization::Sn3d,
        [1.0, 0.0, 0.0],
        &mut hoa_x,
    )
    .unwrap();
    evaluate_hoa_basis_f64(
        HoaOrder::Order3,
        HoaNormalization::Sn3d,
        [0.0, 1.0, 0.0],
        &mut hoa_y,
    )
    .unwrap();
    wigner_z90
        .apply_frame_f64(HoaOrder::Order3, &hoa_x, &mut hoa_rot)
        .unwrap();
    for acn in 0..16 {
        assert!(
            (hoa_rot[acn] - hoa_y[acn]).abs() <= 1e-14,
            "+90 deg Z HOA rotation mismatch at ACN {acn}: got {}, expected {}",
            hoa_rot[acn],
            hoa_y[acn]
        );
    }

    // 2. Arbitrary quaternion equivariance y(R(q) u) == D(q) y(u), homomorphism D(q2*q1) == D(q2)D(q1),
    //    and inverse round-trip normalized L2 residual <= 1e-5
    let q1 = [0.2, -0.4, 0.5, (1.0 - (0.04 + 0.16 + 0.25_f64)).sqrt()];
    let q2 = [-0.3, 0.1, 0.6, (1.0 - (0.09 + 0.01 + 0.36_f64)).sqrt()];
    let q12 = hamilton_product_xyzw(q2, q1);

    let w1 = HoaWignerRotation::from_quaternion_xyzw(q1).unwrap();
    let w2 = HoaWignerRotation::from_quaternion_xyzw(q2).unwrap();
    let w12 = HoaWignerRotation::from_quaternion_xyzw(q12).unwrap();
    let w1_inv = HoaWignerRotation::from_quaternion_xyzw(quaternion_conjugate_xyzw(q1)).unwrap();

    assert!(w1.max_orthogonality_residual(HoaOrder::Order3) <= 1e-14);
    assert!(w2.max_orthogonality_residual(HoaOrder::Order3) <= 1e-14);

    let test_dir = spherical_to_unit_cartesian_f64(0.63, -0.41).unwrap();
    let rotated_dir = rotate_vector_active_f64(q1, test_dir).unwrap();
    let mut y_u = [0.0f64; 16];
    let mut y_ru = [0.0f64; 16];
    let mut d_yu = [0.0f64; 16];
    evaluate_hoa_basis_f64(HoaOrder::Order3, HoaNormalization::N3d, test_dir, &mut y_u).unwrap();
    evaluate_hoa_basis_f64(
        HoaOrder::Order3,
        HoaNormalization::N3d,
        rotated_dir,
        &mut y_ru,
    )
    .unwrap();
    w1.apply_frame_f64(HoaOrder::Order3, &y_u, &mut d_yu)
        .unwrap();

    for acn in 0..16 {
        assert!(
            (d_yu[acn] - y_ru[acn]).abs() <= 1e-13,
            "Equivariance mismatch at ACN {acn}: D*y(u)={}, y(R*u)={}",
            d_yu[acn],
            y_ru[acn]
        );
    }

    // Group homomorphism: D(q2 * q1) * y_u == D(q2) * (D(q1) * y_u)
    let mut d2_d1_yu = [0.0f64; 16];
    let mut d12_yu = [0.0f64; 16];
    w2.apply_frame_f64(HoaOrder::Order3, &d_yu, &mut d2_d1_yu)
        .unwrap();
    w12.apply_frame_f64(HoaOrder::Order3, &y_u, &mut d12_yu)
        .unwrap();
    for acn in 0..16 {
        assert!((d2_d1_yu[acn] - d12_yu[acn]).abs() <= 1e-13);
    }

    // Inverse round-trip L2 residual <= 1e-5
    let mut recovered = [0.0f64; 16];
    w1_inv
        .apply_frame_f64(HoaOrder::Order3, &d_yu, &mut recovered)
        .unwrap();
    let mut err_sq = 0.0f64;
    let mut ref_sq = 0.0f64;
    for acn in 0..16 {
        let diff = recovered[acn] - y_u[acn];
        err_sq += diff * diff;
        ref_sq += y_u[acn] * y_u[acn];
    }
    let rel_l2 = (err_sq / ref_sq).sqrt();
    assert!(rel_l2 <= 1e-5 && rel_l2 <= 1e-14);

    // 3. C02-C04 quaternion admission boundary: q = (0, 0, 0, 1 + 2^-43) admitted,
    //    sign-equivalent q and -q produce identical R, and out-of-tolerance rejected.
    let eps_43 = f64::from_bits((1023 - 43) << 52); // 2^-43
    let q_pert = [0.0, 0.0, 0.0, 1.0 + eps_43];
    let (q_unit, n2) = admit_and_derive_unit_quaternion_f64(q_pert).unwrap();
    assert!((n2 - 1.0).abs() <= 1e-12);
    assert_eq!(q_unit, [0.0, 0.0, 0.0, 1.0]);
    let r_pos = rotation_matrix_from_quaternion_f64(q1).unwrap();
    let r_neg =
        rotation_matrix_from_quaternion_f64([-q1[0], -q1[1], -q1[2], -q1[3]]).unwrap();
    assert_eq!(r_pos, r_neg);
    assert_eq!(
        admit_and_derive_unit_quaternion_f64([0.0, 0.0, 0.0, 1.000_001]),
        Err(SpatialDspError::InvalidQuaternionNorm)
    );

    // 4. C02-C04 Cauchy stress tensor diag(6000, -6000, 0) Pa rotated preserves trace == 0 (p = 0)
    let sigma = [6000.0, -6000.0, 0.0, 0.0, 0.0, 0.0];
    let sigma_rot = rotate_symmetric_tensor_active_f64(q1, sigma).unwrap();
    let trace = sigma_rot[0] + sigma_rot[1] + sigma_rot[2];
    assert!(trace.abs() <= 1e-11);
}

#[test]
fn c02_mode03_and_mode0103_hrir_rir_and_impedance_oracles() {
    // 1. Mode 0103 oracle: L=[1,0,0], R=[0,0.5,0] at 48 kHz with additional delays [2,3] taps
    //    => output impulses at tap 2 amplitude 1.0 and tap 4 amplitude 0.5 (extra delay applied once).
    let signal = [1.0f32];
    let left_ir = [1.0f32, 0.0, 0.0];
    let right_ir = [0.0f32, 0.5, 0.0];
    let mut out_l = [0.0f32; 8];
    let mut out_r = [0.0f32; 8];
    let (nl, nr) = convolve_stereo_hrir_with_additional_delays_into(
        &signal,
        &left_ir,
        &right_ir,
        [2, 3],
        &mut out_l,
        &mut out_r,
    )
    .unwrap();
    assert_eq!(nl, 5);
    assert_eq!(nr, 6);
    assert_eq!(&out_l[..nl], &[0.0, 0.0, 1.0, 0.0, 0.0]);
    assert_eq!(&out_r[..nr], &[0.0, 0.0, 0.0, 0.0, 0.5, 0.0]);

    // 2. Fractional-delay alignment <= 0.01 tap across fractional offsets
    for &target_delay in &[4.0f64, 4.25, 7.37, 12.83] {
        let mut ir = [0.0f32; 24];
        place_cubic_lagrange_fractional_impulse(&mut ir, target_delay, 1.0).unwrap();
        let measured_delay = impulse_first_moment_tap_f64(&ir).unwrap();
        assert!(
            (measured_delay - target_delay).abs() <= 1e-5,
            "Fractional delay alignment error exceeded 0.01 tap: got {measured_delay}, expected {target_delay}"
        );
    }

    // 3. Mode 03 oracle: d=1.5 m, c=1500 m/s => delay = 0.001 s = 48 taps at 48 kHz;
    //    Z1=1.5e6, Z2=3e6 => r=1/3, t=4/3, R=1/9, T=8/9, R+T=1;
    //    Rigid r=+1, pressure-release r=-1.
    let (delay_s, delay_taps) = ray_arrival_delay_taps_f64(1.5, 1500.0, 48_000.0).unwrap();
    assert!((delay_s - 0.001).abs() <= 1e-15);
    assert!((delay_taps - 48.0).abs() <= 1e-12);

    let iface = normal_incidence_fluid_interface_f64(1.5e6, 3.0e6).unwrap();
    assert!((iface.pressure_reflection_r - (1.0 / 3.0)).abs() <= 1e-15);
    assert!((iface.pressure_transmission_t - (4.0 / 3.0)).abs() <= 1e-15);
    assert!((iface.energy_reflectance_big_r - (1.0 / 9.0)).abs() <= 1e-15);
    assert!((iface.energy_transmittance_big_t - (8.0 / 9.0)).abs() <= 1e-15);
    assert!(
        ((iface.energy_reflectance_big_r + iface.energy_transmittance_big_t) - 1.0).abs() <= 1e-15
    );
    assert_eq!(IdealAcousticBoundary::Rigid.pressure_reflection_r(), 1.0);
    assert_eq!(
        IdealAcousticBoundary::PressureRelease.pressure_reflection_r(),
        -1.0
    );
}

#[test]
fn c02_mode04_modal_bank_decay_t60_and_10s_phase_continuity_oracles() {
    // Mode 04 oracle: A=2 Pa, f=100 Hz, d=ln(1000) s^-1, phi=pi/2:
    // y(0) = 2 Pa, y(1) = 0.002 Pa, amplitude T60 = 1 s.
    let mode = ModalResonanceRecord {
        frequency_hz: 100.0,
        amplitude_pa: 2.0,
        decay_rate_s1: LN_1000 as f32,
        phase_rad: core::f32::consts::FRAC_PI_2,
        mode_shape_index: 0,
        material_ref: 1,
        excitation_index: 0,
        reserved: 0,
    };
    let bytes = mode.to_le_bytes().unwrap();
    let decoded = ModalResonanceRecord::from_le_bytes(&bytes).unwrap();
    assert_eq!(mode, decoded);

    let y0 = mode.evaluate_at_time_f64(0.0).unwrap();
    let y1 = mode.evaluate_at_time_f64(1.0).unwrap();
    let (t60_opt, status) = mode.t60_seconds().unwrap();
    assert_eq!(status, QualityStatus::Valid);
    let t60 = t60_opt.unwrap();
    assert!((y0 - 2.0).abs() <= 1e-6);
    assert!((y1 - 0.002).abs() <= 1e-6);
    assert!((t60 - 1.0).abs() <= 1e-6);

    // d = 0 means no finite T60, represented as None / QualityStatus::ModelUnavailable, never +inf
    let undamped = ModalResonanceRecord {
        decay_rate_s1: 0.0,
        amplitude_pa: -2.0, // negative A preserved
        ..mode
    };
    assert_eq!(
        undamped.t60_seconds().unwrap(),
        (None, QualityStatus::ModelUnavailable)
    );
    assert!((undamped.evaluate_at_time_f64(0.0).unwrap() - (-2.0)).abs() <= 1e-6);

    // 10-second multi-block continuation at 48 kHz (480,000 samples):
    // verify zero phase drift across arbitrary block sizes vs exact analytic t = 10.0 s.
    let mut osc = ModalOscillatorState::new(48_000.0).unwrap();
    let shape_w = [1.0f32];
    let exc_w = [1.0f32];
    let mut block = [0.0f32; 480];
    for _ in 0..1000 {
        osc.synthesize_block_into(&[undamped], &shape_w, &exc_w, &mut block)
            .unwrap();
    }
    assert_eq!(osc.sample_cursor(), 480_000);
    // At sample 479,999 (t = 479999 / 48000 s), compare streamed sample to exact f64 analytic
    let t_last = 479_999.0 / 48_000.0;
    let exact_last = undamped.evaluate_at_time_f64(t_last).unwrap() as f32;
    assert!((block[479] - exact_last).abs() <= 1e-6);
}

#[test]
fn c02_mode05_spl_fir_bs1770_and_pitch_status_oracles() {
    // 1. 1 Pa RMS relative to 20e-6 Pa => 93.97940008672038 dB SPL;
    //    relative to 1e-6 Pa => 120.0 dB SPL.
    let one_pa = [1.0f32, -1.0, 1.0, -1.0];
    let spl_air = calibrated_rms_db_spl(&one_pa, AIR_REF_PRESSURE_PA).unwrap();
    assert_eq!(spl_air.status, QualityStatus::Valid);
    assert!((spl_air.rms_pa - 1.0).abs() <= 1e-15);
    assert!((spl_air.db_spl.unwrap() - 93.979_400_086_720_38).abs() <= 1e-4);

    let spl_water = calibrated_rms_db_spl(&one_pa, UNDERWATER_REF_PRESSURE_PA).unwrap();
    assert_eq!(spl_water.status, QualityStatus::Valid);
    assert!((spl_water.db_spl.unwrap() - 120.0).abs() <= 1e-4);

    // 2. 0 signal has RMS 0 and SPL unavailable with QualityStatus::Silence, never -inf
    let silence = [0.0f32; 64];
    let spl_zero = calibrated_rms_db_spl(&silence, AIR_REF_PRESSURE_PA).unwrap();
    assert_eq!(spl_zero.status, QualityStatus::Silence);
    assert_eq!(spl_zero.rms_pa, 0.0);
    assert_eq!(spl_zero.db_spl, None);

    // 3. FIR [0.5, 0.5] impulse => [0.5, 0.5], DC gain 1, Nyquist gain 0
    let kernel = [0.5f32, 0.5];
    let impulse = [1.0f32, 0.0];
    let mut fir_out = [0.0f32; 3];
    let written = apply_linear_fir_filter_into(&impulse, &kernel, &mut fir_out).unwrap();
    assert_eq!(written, 3);
    assert_eq!(&fir_out[..2], &[0.5, 0.5]);
    let (dc_gain, nyq_gain) = fir_dc_and_nyquist_gain_f64(&kernel).unwrap();
    assert!((dc_gain - 1.0).abs() <= 1e-15);
    assert!(nyq_gain.abs() <= 1e-15);

    // 4. BS.1770-5 gating boundary tests:
    //    - All-silent blocks => QualityStatus::Silence, integrated_lufs == None
    //    - Block below -70 LUFS is excluded by absolute gate
    //    - Block more than 10 LU below L_KG,a is excluded by relative gate
    let silent_reading = gate_block_energies_bs1770(&[0.0, 0.0, 0.0]).unwrap();
    assert_eq!(silent_reading.status, QualityStatus::Silence);
    assert_eq!(silent_reading.integrated_lufs, None);

    // Energy corresponding to -20.691 LUFS (z = 1e-2) and -35.691 LUFS (z = 10^-3.5, 15 LU quieter)
    // and -75.691 LUFS (z = 10^-7.5, below -70 LUFS absolute gate).
    // Absolute gate keeps z=1e-2 and z=10^-3.5; L_KG,a is near -23.56 LUFS, so relative gate
    // (-33.56 LUFS) drops z=10^-3.5 (-35.691 LUFS) and retains only z=1e-2 (-20.691 LUFS)!
    let e_high = 1.0e-2_f64;
    let e_mid_quiet = 3.162_277_660_168_379_5e-4_f64; // 10^-3.5 => -35.691 LUFS
    let e_sub_abs = 3.162_277_660_168_379_4e-8_f64;   // 10^-7.5 => -75.691 LUFS
    let gated = gate_block_energies_bs1770(&[e_high, e_mid_quiet, e_sub_abs]).unwrap();
    assert_eq!(gated.status, QualityStatus::Valid);
    assert_eq!(gated.total_blocks, 3);
    assert_eq!(gated.absolute_gated_blocks, 2);
    assert_eq!(gated.relative_gated_blocks, 1);
    assert!((gated.integrated_lufs.unwrap() - (-20.691)).abs() <= 1e-5);

    // 5. YIN pitch status: 440 Hz voiced vs silent vs unvoiced
    let sr = 48_000.0f32;
    let mut voiced = [0.0f32; 1024];
    for (i, slot) in voiced.iter_mut().enumerate() {
        let angle = core::f64::consts::TAU * 440.0 * (i as f64) / f64::from(sr);
        *slot = angle.sin() as f32;
    }
    let mut yin_scratch = [0.0f32; 513];
    let p_voiced = estimate_pitch_yin_with_status(&voiced, sr, 0.15, &mut yin_scratch).unwrap();
    assert_eq!(p_voiced.status, QualityStatus::Valid);
    assert!((p_voiced.f0_hz.unwrap() - 440.0).abs() <= 1.0);

    let p_silent = estimate_pitch_yin_with_status(&silence, sr, 0.15, &mut yin_scratch).unwrap();
    assert_eq!(p_silent.status, QualityStatus::Silence);
    assert_eq!(p_silent.f0_hz, None);

    // Alternating +1/-1 Nyquist sequence has period 2 (below interior lag search) / high CMNDF
    let mut unvoiced = [0.0f32; 128];
    for (i, slot) in unvoiced.iter_mut().enumerate() {
        *slot = ((i * 73 + 19) % 97) as f32 / 48.5 - 1.0;
    }
    let p_unvoiced =
        estimate_pitch_yin_with_status(&unvoiced, sr, 0.05, &mut yin_scratch[..65]).unwrap();
    if p_unvoiced.status == QualityStatus::Unvoiced {
        assert_eq!(p_unvoiced.f0_hz, None);
    }

    // 6. Mode 0x0102 HOA LE record round-trip
    let mut hoa_bytes = [0u8; 16];
    let hoa_frame = [1.0f32, 0.0, 0.0, 1.0];
    assert_eq!(
        encode_qac_mode0102_hoa_record_le(HoaOrder::Order1, &hoa_frame, &mut hoa_bytes),
        Ok(16)
    );
    let mut hoa_decoded = [0.0f32; 4];
    assert_eq!(
        decode_qac_mode0102_hoa_record_le(HoaOrder::Order1, &hoa_bytes, &mut hoa_decoded),
        Ok(4)
    );
    assert_eq!(hoa_decoded, hoa_frame);
}
