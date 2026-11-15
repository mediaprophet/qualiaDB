//! Comprehensive unit tests for `q42::science_validation` (`N04` independent physical invariant
//! verifier across C02-C01..C02-C09, EMF Modes 0..7, and QAC Modes 00..07).

use crate::q42::science_validation::cross_modal_registration::{
    compose_hamilton_quaternions_xyzw, evaluate_exact_sample_instant, transform_local_to_parent,
    verify_anatomical_region_registration, verify_and_derive_quaternion_rotation,
    verify_cross_modal_clock_join, verify_material_and_anatomy_coupling,
    verify_stationary_proper_rotation_matrix, MaterialDomainFamily, RationalClockDescriptor,
    RationalSeconds, RegistrationValidationError,
};
use crate::q42::science_validation::passivity_conservation::{
    evaluate_acoustic_interface_oracle, evaluate_em_mode0_instantaneous_oracle,
    evaluate_em_wavelength_m, evaluate_qac_elastic_mode01_oracle,
    evaluate_qac_fluid_mode00_oracle, verify_acoustic_energy_conservation,
    verify_b32_scalar_budget, verify_b32_vector_budget, verify_f64_closed_form_budget,
    verify_grid_convergence_order, verify_n_term_reduction_budget,
    verify_poynting_conservation_balance, verify_scattering_matrix_passivity,
    verify_yee_cfl_and_dispersion, ConservationValidationError, C02_AMPLITUDE_SCALES_B32,
    C02_AMPLITUDE_SCALES_F64, VACUUM_IMPEDANCE_OHM, VACUUM_PERMITTIVITY_F_PER_M,
};
use crate::q42::science_validation::specialized_oracles::{
    convert_and_verify_iq_carrier_bridge, convert_kev_to_joules, convert_mw_per_cm2_to_w_per_m2,
    convert_spectral_density_hz_to_wavelength_m, evaluate_absorbed_dose_gy,
    evaluate_bioelectric_dipole_potential_v, evaluate_equivalent_dose_sv,
    evaluate_haptic_mechanical_oracle, evaluate_ionizing_beer_lambert_oracle,
    evaluate_qac_db_spl_oracle, evaluate_qac_modal_oscillator_oracle,
    evaluate_ultrasound_mode07_oracle, HapticRole, SpecializedOracleError,
};
use crate::q42::science_validation::stokes_mueller::{
    apply_mueller_matrix, deterministic_transverse_basis, jones_to_mueller_matrix,
    jones_vector_to_stokes, rotate_stokes_transverse_basis,
    vacuum_or_homogeneous_propagation_delay_s, verify_jones_to_mueller_equivalence,
    verify_mueller_realizability, verify_stokes_realizability, Complex64,
    PolarimetricValidationError, VACUUM_SPEED_OF_LIGHT_M_PER_S,
};
use crate::q42::science_validation::uncertainty_ece::{
    apply_or_verify_affine_calibration, evaluate_ece_and_brier, evaluate_gaussian_nll,
    mahalanobis_nees_3d, rotate_covariance_3x3, verify_covariance_3x3_spd,
    verify_gaussian_coverage_factor, verify_half_open_calibration_window,
    verify_uncertainty_descriptor, CalibrationStage, UncertaintyDescriptor, UncertaintyKind,
    UncertaintyValidationError,
};

#[test]
fn test_stokes_realizability_and_mode1_oracles_across_scales() {
    let zero_state = verify_stokes_realizability([0.0, 0.0, 0.0, 0.0]).expect("zero Stokes valid");
    assert_eq!(zero_state.degree_of_polarization, 0.0);

    let over_err = verify_stokes_realizability([1.0, 1.001, 0.0, 0.0])
        .expect_err("over-polarized Stokes must reject");
    assert!(matches!(over_err, PolarimetricValidationError::OverPolarizedState { .. }));

    let neg_err = verify_stokes_realizability([-1.0e-6, 0.0, 0.0, 0.0])
        .expect_err("negative intensity must reject");
    assert!(matches!(neg_err, PolarimetricValidationError::NegativeTotalIntensity { .. }));

    let inv_sqrt2 = 1.0 / core::f64::consts::SQRT_2;
    for &scale in &C02_AMPLITUDE_SCALES_F64 {
        let amp = scale.sqrt();
        let ex = Complex64::new(amp * inv_sqrt2, 0.0);
        let ey = Complex64::new(0.0, amp * inv_sqrt2);
        let stokes = jones_vector_to_stokes(ex, ey);
        let verified = verify_stokes_realizability(stokes).expect("Jones-derived Stokes valid");
        verify_f64_closed_form_budget(verified.stokes[0], scale, scale).unwrap();
        verify_f64_closed_form_budget(verified.stokes[1], 0.0, scale).unwrap();
        verify_f64_closed_form_budget(verified.stokes[2], 0.0, scale).unwrap();
        verify_f64_closed_form_budget(verified.stokes[3], -scale, scale).unwrap();
        verify_f64_closed_form_budget(verified.degree_of_polarization, 1.0, 1.0).unwrap();

        let rotated = rotate_stokes_transverse_basis(
            [scale, scale, 0.0, 0.0],
            core::f64::consts::FRAC_PI_4,
        );
        verify_f64_closed_form_budget(rotated[0], scale, scale).unwrap();
        verify_f64_closed_form_budget(rotated[1], 0.0, scale).unwrap();
        verify_f64_closed_form_budget(rotated[2], -scale, scale).unwrap();
        verify_f64_closed_form_budget(rotated[3], 0.0, scale).unwrap();
    }

    let (e1, e2) = deterministic_transverse_basis([0.0, 0.0, 1.0]).unwrap();
    assert_eq!(e1, [1.0, 0.0, 0.0]);
    assert_eq!(e2, [0.0, 1.0, 0.0]);

    let delay = vacuum_or_homogeneous_propagation_delay_s(0.299_792_458, 1.0).unwrap();
    verify_f64_closed_form_budget(delay, 1.0e-9, 1.0e-9).unwrap();
}

#[test]
fn test_mueller_cloude_realizability_and_jones_equivalence() {
    let m_id = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
    let rep_id = verify_mueller_realizability(&m_id, true).expect("identity Mueller valid");
    verify_f64_closed_form_budget(rep_id.cloude_trace, 1.0, 1.0).unwrap();
    verify_f64_closed_form_budget(rep_id.cloude_eigenvalues[3], 1.0, 1.0).unwrap();
    verify_f64_closed_form_budget(rep_id.depolarization_index, 1.0, 1.0).unwrap();

    let j_pol_x = [
        [Complex64::ONE, Complex64::ZERO],
        [Complex64::ZERO, Complex64::ZERO],
    ];
    let m_pol_x = [
        [0.5, 0.5, 0.0, 0.0],
        [0.5, 0.5, 0.0, 0.0],
        [0.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0, 0.0],
    ];
    verify_jones_to_mueller_equivalence(&m_pol_x, &j_pol_x, 1.0).unwrap();
    let rep_pol = verify_mueller_realizability(&m_pol_x, true).expect("x polarizer valid");
    verify_f64_closed_form_budget(rep_pol.max_transmittance_gain, 1.0, 1.0).unwrap();
    verify_f64_closed_form_budget(rep_pol.cloude_eigenvalues[3], 0.5, 1.0).unwrap();

    for &scale in &C02_AMPLITUDE_SCALES_F64 {
        let out_unpol = apply_mueller_matrix(&m_pol_x, [2.0 * scale, 0.0, 0.0, 0.0]);
        verify_f64_closed_form_budget(out_unpol[0], scale, 2.0 * scale).unwrap();
        verify_f64_closed_form_budget(out_unpol[1], scale, 2.0 * scale).unwrap();

        let out_blocked = apply_mueller_matrix(&m_pol_x, [scale, -scale, 0.0, 0.0]);
        verify_f64_closed_form_budget(out_blocked[0], 0.0, 2.0 * scale).unwrap();

        let out_passed = apply_mueller_matrix(&m_pol_x, [scale, scale, 0.0, 0.0]);
        verify_f64_closed_form_budget(out_passed[0], scale, 2.0 * scale).unwrap();
    }

    let j_qwp = [
        [Complex64::ONE, Complex64::ZERO],
        [Complex64::ZERO, Complex64::I],
    ];
    let m_qwp = jones_to_mueller_matrix(&j_qwp);
    let rep_qwp = verify_mueller_realizability(&m_qwp, true).expect("QWP Mueller valid");
    verify_f64_closed_form_budget(rep_qwp.cloude_eigenvalues[0], 0.0, 1.0).unwrap();
    verify_f64_closed_form_budget(rep_qwp.cloude_eigenvalues[3], 1.0, 1.0).unwrap();

    let m_unphysical = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 2.0, 0.0, 0.0],
        [0.0, 0.0, 2.0, 0.0],
        [0.0, 0.0, 0.0, 2.0],
    ];
    let cloude_err = verify_mueller_realizability(&m_unphysical, false)
        .expect_err("over-polarizing Mueller matrix must fail Cloude PSD");
    assert!(matches!(cloude_err, PolarimetricValidationError::CloudeNegativeEigenvalue { .. }));

    let m_active = [
        [1.2, 0.0, 0.0, 0.0],
        [0.0, 1.2, 0.0, 0.0],
        [0.0, 0.0, 1.2, 0.0],
        [0.0, 0.0, 0.0, 1.2],
    ];
    let active_err = verify_mueller_realizability(&m_active, true)
        .expect_err("amplifying Mueller matrix must fail passive gate");
    assert!(matches!(active_err, PolarimetricValidationError::ActiveGainViolation { .. }));
}

#[test]
fn test_em_mode0_and_mode2_passivity_conservation_oracles() {
    for &scale in &C02_AMPLITUDE_SCALES_F64 {
        let e = [scale, 0.0, 0.0];
        let b = [0.0, scale / VACUUM_SPEED_OF_LIGHT_M_PER_S, 0.0];
        let res = evaluate_em_mode0_instantaneous_oracle(e, b, 1.0, 1.0).unwrap();
        let q_s = (scale * scale) / VACUUM_IMPEDANCE_OHM;
        let q_u = VACUUM_PERMITTIVITY_F_PER_M * (scale * scale);
        verify_f64_closed_form_budget(res.poynting_w_per_m2[2], q_s, q_s).unwrap();
        verify_f64_closed_form_budget(res.peak_phasor_mean_poynting_w_per_m2[2], 0.5 * q_s, q_s).unwrap();
        verify_f64_closed_form_budget(res.energy_density_j_per_m3, q_u, q_u).unwrap();

        let e_rot = [0.0, scale, 0.0];
        let b_rot = [-scale / VACUUM_SPEED_OF_LIGHT_M_PER_S, 0.0, 0.0];
        let res_rot = evaluate_em_mode0_instantaneous_oracle(e_rot, b_rot, 1.0, 1.0).unwrap();
        verify_f64_closed_form_budget(res_rot.poynting_w_per_m2[2], q_s, q_s).unwrap();

        verify_poynting_conservation_balance(-0.5 * q_s, 0.2 * q_s, 0.3 * q_s, q_s).unwrap();
    }

    let loss_err = verify_poynting_conservation_balance(1.0, 0.0, -1.0, 1.0)
        .expect_err("negative ohmic dissipation must reject");
    assert!(matches!(loss_err, ConservationValidationError::PassivityOrConservationViolation { .. }));

    let static_err = evaluate_em_wavelength_m(0.0, 1.0, 1.0)
        .expect_err("static field wavelength must reject");
    assert!(matches!(static_err, ConservationValidationError::StaticFieldWavelengthForbidden { .. }));

    let dx = [1.0 / 64.0; 3];
    let cfl_max = 1.0 / (VACUUM_SPEED_OF_LIGHT_M_PER_S * (3.0 * 64.0 * 64.0_f64).sqrt());
    let (computed_cfl, omega_num) = verify_yee_cfl_and_dispersion(
        dx, 0.5 * cfl_max, 1.0, 1.0, [2.0 * core::f64::consts::PI, 0.0, 0.0],
    ).unwrap();
    verify_f64_closed_form_budget(computed_cfl, cfl_max, cfl_max).unwrap();
    assert!(omega_num > 0.0);

    let cfl_err = verify_yee_cfl_and_dispersion(
        dx, 1.01 * cfl_max, 1.0, 1.0, [2.0 * core::f64::consts::PI, 0.0, 0.0],
    ).expect_err("dt = 1.01 * CFL must reject");
    assert!(matches!(cfl_err, ConservationValidationError::CflStabilityViolation { .. }));
}

#[test]
fn test_qac_continuum_interface_scattering_and_convergence_oracles() {
    for &scale in &C02_AMPLITUDE_SCALES_B32 {
        let p = 1500.0 * scale;
        let v = [0.001 * scale, 0.0, 0.0];
        let res = evaluate_qac_fluid_mode00_oracle(1000.0, 2.25e9, p, v, true).unwrap();
        verify_b32_scalar_budget(res.sound_speed_m_per_s, 1500.0, 1500.0).unwrap();
        verify_b32_scalar_budget(res.characteristic_impedance_rayl, 1.5e6, 1.5e6).unwrap();
        let s_i = 1.5 * scale * scale;
        let s_e = 0.001 * scale * scale;
        verify_b32_vector_budget(&res.instantaneous_intensity_w_per_m2, &[s_i, 0.0, 0.0], s_i).unwrap();
        verify_b32_scalar_budget(res.cycle_mean_intensity_w_per_m2[0], 0.5 * s_i, s_i).unwrap();
        verify_b32_scalar_budget(res.instantaneous_energy_density_j_per_m3, s_e, s_e).unwrap();
        verify_acoustic_energy_conservation(-0.4 * s_i, 0.1 * s_i, 0.3 * s_i, s_i).unwrap();
    }

    assert_eq!(
        evaluate_qac_fluid_mode00_oracle(1000.0, 2.25e9, 1500.0, [0.001, 0.0, 0.0], false),
        Err(ConservationValidationError::NonCollocatedPressureVelocityRejected)
    );

    let el = evaluate_qac_elastic_mode01_oracle(
        2000.0, 8.0e9, 3.0e9, [1.0e-6, -1.0e-6, 0.0, 0.0, 0.0, 0.0],
    ).unwrap();
    verify_f64_closed_form_budget(el.lame_lambda_pa, 6.0e9, 6.0e9).unwrap();
    verify_f64_closed_form_budget(el.compressional_speed_cp_m_per_s, 2449.489742783178, 2500.0).unwrap();
    verify_f64_closed_form_budget(el.shear_speed_cs_m_per_s, 1224.744871391589, 1250.0).unwrap();
    verify_f64_closed_form_budget(el.stress_xx_yy_zz_xy_yz_xz_pa[0], 6000.0, 6000.0).unwrap();
    verify_f64_closed_form_budget(el.stress_xx_yy_zz_xy_yz_xz_pa[1], -6000.0, 6000.0).unwrap();
    verify_f64_closed_form_budget(el.mechanical_pressure_pa, 0.0, 6000.0).unwrap();

    let (r, t, cap_r, cap_t) = evaluate_acoustic_interface_oracle(1.5e6, 3.0e6).unwrap();
    verify_f64_closed_form_budget(r, 1.0 / 3.0, 1.0).unwrap();
    verify_f64_closed_form_budget(t, 4.0 / 3.0, 1.0).unwrap();
    verify_f64_closed_form_budget(cap_r, 1.0 / 9.0, 1.0).unwrap();
    verify_f64_closed_form_budget(cap_t, 8.0 / 9.0, 1.0).unwrap();

    verify_n_term_reduction_budget(8.0, 8.0, 8, 8.0).unwrap();

    let mut s_passive = [[Complex64::ZERO; 4]; 4];
    s_passive[0][0] = Complex64::new(1.0 / 3.0, 0.0);
    s_passive[0][1] = Complex64::new((8.0f64 / 9.0).sqrt(), 0.0);
    s_passive[1][0] = s_passive[0][1];
    s_passive[1][1] = Complex64::new(-1.0 / 3.0, 0.0);
    let norm = verify_scattering_matrix_passivity(&s_passive, 2).unwrap();
    verify_f64_closed_form_budget(norm, 1.0, 1.0).unwrap();

    let mut s_active = s_passive;
    s_active[0][0] = Complex64::new(1.1, 0.0);
    assert!(matches!(
        verify_scattering_matrix_passivity(&s_active, 2),
        Err(ConservationValidationError::ScatteringPassivityViolation { .. })
    ));

    let p_order = verify_grid_convergence_order(1.0 / 32.0, 0.008, 1.0 / 64.0, 0.002, 1.8).unwrap();
    verify_f64_closed_form_budget(p_order, 2.0, 2.0).unwrap();
    assert!(matches!(
        verify_grid_convergence_order(1.0 / 32.0, 0.008, 1.0 / 64.0, 0.004, 1.8),
        Err(ConservationValidationError::ConvergenceOrderBelowMinimum { .. })
    ));
}

#[test]
fn test_uncertainty_calibration_mahalanobis_and_ece_oracles() {
    verify_gaussian_coverage_factor(1.0, 0.682_689_492_137_085_9, 1.0e-12).unwrap();
    verify_gaussian_coverage_factor(1.959_963_984_540_054, 0.95, 1.0e-12).unwrap();
    verify_gaussian_coverage_factor(2.0, 0.954_499_736_103_641_6, 1.0e-12).unwrap();
    verify_gaussian_coverage_factor(3.0, 0.997_300_203_936_739_8, 1.0e-12).unwrap();

    assert!(matches!(
        verify_gaussian_coverage_factor(1.0, 0.95, 1.0e-4),
        Err(UncertaintyValidationError::GaussianCoverageMismatch { .. })
    ));

    let valid_desc = UncertaintyDescriptor {
        target_quantity: "absorbed_dose",
        target_unit: "Gy",
        kind: UncertaintyKind::ExpandedAbsolute {
            coverage_factor_k: 2.0,
            confidence_probability: 0.954_499_74,
        },
        value: 0.01,
        record_has_mixed_units: true,
        broadcast_across_all_fields: false,
    };
    verify_uncertainty_descriptor(&valid_desc, 1.0e-6).unwrap();
    assert_eq!(
        verify_uncertainty_descriptor(&UncertaintyDescriptor { target_unit: "", ..valid_desc }, 1.0e-6),
        Err(UncertaintyValidationError::MissingTargetQuantityOrUnit)
    );
    assert_eq!(
        verify_uncertainty_descriptor(&UncertaintyDescriptor { broadcast_across_all_fields: true, ..valid_desc }, 1.0e-6),
        Err(UncertaintyValidationError::MixedUnitScalarBroadcastForbidden)
    );

    let (i_cal, stage_i) = apply_or_verify_affine_calibration(3.0, CalibrationStage::RawCounts, 1.0, 0.0, 2.0, 0.0).unwrap();
    let (q_cal, stage_q) = apply_or_verify_affine_calibration(4.0, CalibrationStage::RawCounts, 1.0, 0.0, 2.0, 0.0).unwrap();
    assert_eq!((i_cal, q_cal), (6.0, 8.0));
    let (i_reopen, _) = apply_or_verify_affine_calibration(i_cal, stage_i, 1.0, 0.0, 2.0, 0.0).unwrap();
    let (q_reopen, _) = apply_or_verify_affine_calibration(q_cal, stage_q, 1.0, 0.0, 2.0, 0.0).unwrap();
    assert_eq!((i_reopen, q_reopen), (6.0, 8.0));

    assert!(matches!(
        apply_or_verify_affine_calibration(6.0, CalibrationStage::CalibratedPhysical, 2.0, 0.0, 2.0, 0.0),
        Err(UncertaintyValidationError::DoubleCalibrationForbidden { .. })
    ));

    verify_half_open_calibration_window(100, 100, 200).unwrap();
    verify_half_open_calibration_window(199, 100, 200).unwrap();
    assert!(verify_half_open_calibration_window(99, 100, 200).is_err());
    assert!(verify_half_open_calibration_window(200, 100, 200).is_err());

    let cov = [[4.0, 1.0, 0.0], [1.0, 2.0, 0.0], [0.0, 0.0, 1.0]];
    let rot_z90 = [[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]];
    let (tr, det) = verify_covariance_3x3_spd(&cov).unwrap();
    assert_eq!((tr, det), (7.0, 7.0));
    let cov_rot = rotate_covariance_3x3(&cov, &rot_z90).unwrap();
    verify_f64_closed_form_budget(cov_rot[0][0], 2.0, 7.0).unwrap();
    verify_f64_closed_form_budget(cov_rot[1][1], 4.0, 7.0).unwrap();

    let nees = mahalanobis_nees_3d([2.0, 0.0, 0.0], [0.0, 0.0, 0.0], &[[4.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]).unwrap();
    verify_f64_closed_form_budget(nees, 1.0, 1.0).unwrap();

    let probs = [0.25, 0.25, 0.25, 0.25, 0.75, 0.75, 0.75, 0.75];
    let outcomes = [true, false, false, false, true, true, true, false];
    let cal_rep = evaluate_ece_and_brier(&probs, &outcomes, 4, 1.0e-12).unwrap();
    verify_f64_closed_form_budget(cal_rep.expected_calibration_error, 0.0, 1.0).unwrap();
    verify_f64_closed_form_budget(cal_rep.brier_score, 0.1875, 1.0).unwrap();

    let nll = evaluate_gaussian_nll(&[0.0], &[0.0], &[1.0]).unwrap();
    verify_f64_closed_form_budget(nll, 0.5 * (2.0 * core::f64::consts::PI).ln(), 1.0).unwrap();
}

#[test]
fn test_specialized_e06_q07_and_c02_c08_oracles_across_scales() {
    for &scale in &C02_AMPLITUDE_SCALES_F64 {
        let rad = evaluate_ionizing_beer_lambert_oracle(1000.0, 0.01, 0.2, 100.0 * scale, false).unwrap();
        verify_f64_closed_form_budget(rad.linear_attenuation_per_m, 10.0, 10.0).unwrap();
        verify_f64_closed_form_budget(rad.transmitted_fluence_per_m2, 13.533_528_323_661_27 * scale, 100.0 * scale).unwrap();

        let dose = evaluate_absorbed_dose_gy(0.002 * scale, 0.01).unwrap();
        verify_f64_closed_form_budget(dose, 0.2 * scale, 0.2 * scale).unwrap();

        let e_j = convert_kev_to_joules(100.0 * scale).unwrap();
        let ref_j = 1.602_176_634e-14 * scale;
        verify_f64_closed_form_budget(e_j, ref_j, ref_j).unwrap();

        let ref_phi = 7.957_747_154_594_767e-8 * scale;
        let phi_z = evaluate_bioelectric_dipole_potential_v([0.0, 0.0, 1.0e-8 * scale], [0.0, 0.0, 0.0], Some([0.0, 0.0, 0.1]), true, 1.0).unwrap();
        let phi_x = evaluate_bioelectric_dipole_potential_v([0.0, 0.0, 1.0e-8 * scale], [0.0, 0.0, 0.0], Some([0.1, 0.0, 0.0]), true, 1.0).unwrap();
        let phi_nz = evaluate_bioelectric_dipole_potential_v([0.0, 0.0, 1.0e-8 * scale], [0.0, 0.0, 0.0], Some([0.0, 0.0, -0.1]), true, 1.0).unwrap();
        verify_f64_closed_form_budget(phi_z, ref_phi, ref_phi).unwrap();
        verify_f64_closed_form_budget(phi_x, 0.0, ref_phi).unwrap();
        verify_f64_closed_form_budget(phi_nz, -ref_phi, ref_phi).unwrap();
    }

    assert_eq!(evaluate_ionizing_beer_lambert_oracle(1000.0, 0.01, 0.2, 100.0, true), Err(SpecializedOracleError::DoseFromAttenuationAloneForbidden));
    assert_eq!(evaluate_equivalent_dose_sv(0.2, 1.0, None), Err(SpecializedOracleError::EquivalentDoseWithoutWeightingModelForbidden));
    assert!(matches!(evaluate_bioelectric_dipole_potential_v([0.0, 0.0, 1.0e-8], [0.0, 0.0, 0.0], Some([0.0, 0.0, 0.0]), true, 1.0), Err(SpecializedOracleError::SingularBioelectricDipoleAtSource { .. })));
    assert_eq!(evaluate_bioelectric_dipole_potential_v([0.0, 0.0, 1.0e-8], [0.0, 0.0, 0.0], None, false, 1.0), Err(SpecializedOracleError::MissingBioelectricObservationReference));

    let rot_z90 = [[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]];
    let hap = evaluate_haptic_mechanical_oracle(2.0, 1.0e-4, 1000.0, 0.002, [2.0, 0.0, 0.0], &rot_z90, HapticRole::ActuatorCommand, true, true, 99, 100).unwrap();
    verify_b32_scalar_budget(hap.contact_pressure_pa, 20_000.0, 20_000.0).unwrap();
    verify_b32_scalar_budget(hap.restoring_force_n, -2.0, 2.0).unwrap();
    verify_b32_vector_budget(&hap.parent_force_n, &[0.0, 2.0, 0.0], 2.0).unwrap();

    assert!(matches!(evaluate_haptic_mechanical_oracle(2.0, 1.0e-4, 1000.0, 0.002, [2.0, 0.0, 0.0], &rot_z90, HapticRole::ActuatorCommand, true, true, 100, 100), Err(SpecializedOracleError::HapticCommandUnauthorizedOrExpired { .. })));
    assert!(matches!(evaluate_haptic_mechanical_oracle(2.0, 1.0e-4, 1000.0, 0.002, [2.0, 0.0, 0.0], &rot_z90, HapticRole::Measurement, true, true, 50, 100), Err(SpecializedOracleError::HapticCommandUnauthorizedOrExpired { .. })));

    let us = evaluate_ultrasound_mode07_oracle(1500.0, 0.015, 20.0e6, 4.0e6, 1.0e6, 0.15, None, None, false).unwrap();
    verify_f64_closed_form_budget(us.round_trip_tof_s, 20.0e-6, 20.0e-6).unwrap();
    verify_f64_closed_form_budget(us.round_trip_samples, 400.0, 400.0).unwrap();
    verify_f64_closed_form_budget(us.mechanical_index, 0.5, 0.5).unwrap();

    let us_dop = evaluate_ultrasound_mode07_oracle(1500.0, 0.015, 20.0e6, 2.0e6, 1.0e6, 0.15, None, None, false).unwrap();
    verify_f64_closed_form_budget(us_dop.doppler_shift_hz, 400.0, 400.0).unwrap();
    assert_eq!(convert_mw_per_cm2_to_w_per_m2(1.0).unwrap(), 10.0);
    assert_eq!(evaluate_ultrasound_mode07_oracle(1500.0, 0.015, 20.0e6, 4.0e6, 1.0e6, 0.15, Some(0.8), None, false), Err(SpecializedOracleError::ThermalIndexInferenceWithoutModelForbidden));

    let z_emf = Complex64::new(0.0, 1.0);
    convert_and_verify_iq_carrier_bridge(z_emf, z_emf.conj(), 1000.0, 0.00025, 0.0).unwrap();
    assert!(matches!(convert_and_verify_iq_carrier_bridge(z_emf, z_emf, 1000.0, 0.00025, 0.0), Err(SpecializedOracleError::UnconvertedIqCarrierSignMismatch { .. })));

    let (lam1, fl1) = convert_spectral_density_hz_to_wavelength_m(1.0e12, 1.0e-12, false).unwrap();
    let (lam2, _) = convert_spectral_density_hz_to_wavelength_m(2.0e12, 1.0e-12, false).unwrap();
    assert!(lam2 < lam1 && fl1 > 0.0);
    assert_eq!(convert_spectral_density_hz_to_wavelength_m(1.0e12, 0.5, true), Err(SpecializedOracleError::ReflectanceJacobianForbidden));

    let (y0, t60) = evaluate_qac_modal_oscillator_oracle(2.0, 100.0, 1000.0f64.ln(), core::f64::consts::FRAC_PI_2, 0.0).unwrap();
    let (y1, _) = evaluate_qac_modal_oscillator_oracle(2.0, 100.0, 1000.0f64.ln(), core::f64::consts::FRAC_PI_2, 1.0).unwrap();
    verify_b32_scalar_budget(y0, 2.0, 2.0).unwrap();
    verify_b32_scalar_budget(y1, 0.002, 2.0).unwrap();
    verify_b32_scalar_budget(t60.unwrap(), 1.0, 1.0).unwrap();
    assert_eq!(evaluate_qac_modal_oscillator_oracle(2.0, 100.0, 0.0, 0.0, 0.0).unwrap().1, None);

    let spl_94 = evaluate_qac_db_spl_oracle(1.0, 20.0e-6).unwrap().unwrap();
    verify_f64_closed_form_budget(spl_94, 93.979_400_086_720_38, 100.0).unwrap();
    assert_eq!(evaluate_qac_db_spl_oracle(0.0, 20.0e-6).unwrap(), None);
}

#[test]
fn test_cross_modal_registration_quaternions_clocks_and_anatomy() {
    let eps_q = [0.0, 0.0, 0.0, 1.0 + 2.0f64.powi(-43)];
    let ver_q = verify_and_derive_quaternion_rotation(eps_q).unwrap();
    assert_eq!(ver_q.raw_xyzw, eps_q);
    verify_f64_closed_form_budget(ver_q.rotation_matrix[0][0], 1.0, 1.0).unwrap();

    assert!(matches!(verify_and_derive_quaternion_rotation([0.0, 0.0, 0.0, 1.001]), Err(RegistrationValidationError::QuaternionNormOutOfTolerance { .. })));

    let s = 1.0 / core::f64::consts::SQRT_2;
    let q_z90 = [0.0, 0.0, s, s];
    let rot_z90 = verify_and_derive_quaternion_rotation(q_z90).unwrap();
    let neg_rot_z90 = verify_and_derive_quaternion_rotation([0.0, 0.0, -s, -s]).unwrap();
    assert_eq!(rot_z90.rotation_matrix, neg_rot_z90.rotation_matrix);
    let (pos_p, vec_p) = transform_local_to_parent([10.0, 20.0, 30.0], &rot_z90.rotation_matrix, [1.0, 0.0, 0.0], [1.0, 0.0, 0.0]);
    verify_f64_closed_form_budget(pos_p[0], 10.0, 30.0).unwrap();
    verify_f64_closed_form_budget(pos_p[1], 21.0, 30.0).unwrap();
    verify_f64_closed_form_budget(vec_p[0], 0.0, 1.0).unwrap();
    verify_f64_closed_form_budget(vec_p[1], 1.0, 1.0).unwrap();

    let q_z180 = compose_hamilton_quaternions_xyzw(q_z90, q_z90);
    let rot_z180 = verify_and_derive_quaternion_rotation(q_z180).unwrap();
    let (_, vec_180) = transform_local_to_parent([0.0, 0.0, 0.0], &rot_z180.rotation_matrix, [0.0, 0.0, 0.0], [1.0, 0.0, 0.0]);
    verify_f64_closed_form_budget(vec_180[0], -1.0, 1.0).unwrap();
    verify_f64_closed_form_budget(vec_180[1], 0.0, 1.0).unwrap();

    let reflection = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, -1.0]];
    assert!(matches!(verify_stationary_proper_rotation_matrix(&reflection, false, false), Err(RegistrationValidationError::ImproperOrNonOrthogonalFrame { .. })));

    let clk_96k = RationalClockDescriptor {
        origin_id: 42, scale: 1, epoch_seconds: 1_700_000_000, epoch_nanoseconds: 0,
        ticks_per_second_num: 96_000, ticks_per_second_den: 1,
    };
    assert_eq!(evaluate_exact_sample_instant(&clk_96k, 0, 1, 48_000, 1, 0, 1).unwrap(), RationalSeconds { num: 1, den: 48_000 });
    assert_eq!(evaluate_exact_sample_instant(&clk_96k, 0, 48_000, 48_000, 1, 0, 1).unwrap(), RationalSeconds { num: 1, den: 1 });
    assert_eq!(evaluate_exact_sample_instant(&clk_96k, 0, 1001, 48_000_000, 1001, 0, 1).unwrap(), RationalSeconds { num: 1_002_001, den: 48_000_000 });

    let clk_other = RationalClockDescriptor { origin_id: 99, ..clk_96k };
    assert!(matches!(verify_cross_modal_clock_join(&clk_96k, &clk_other, false, 1.0e-9, 1.0e6, Some(0.1)), Err(RegistrationValidationError::IncompatibleClockOrigins { .. })));

    let obj_dig = [0xAAu8; 32];
    let tbl_dig = [0xBBu8; 32];
    assert!(verify_material_and_anatomy_coupling(MaterialDomainFamily::EmfV4 { explicit_vacuum: false }, 0, &obj_dig, &obj_dig, &tbl_dig, &tbl_dig, 1, 1, true, true).is_err());
    verify_material_and_anatomy_coupling(MaterialDomainFamily::EmfV4 { explicit_vacuum: true }, 0, &obj_dig, &obj_dig, &tbl_dig, &tbl_dig, 1, 1, true, true).unwrap();
    verify_material_and_anatomy_coupling(MaterialDomainFamily::QacV1 { has_declared_uniform_medium: true }, 0, &obj_dig, &obj_dig, &tbl_dig, &tbl_dig, 1, 1, true, true).unwrap();

    verify_anatomical_region_registration([0.0, 0.0, 0.05], [-0.1, -0.1, -0.1], [0.1, 0.1, 0.1], [0.0, 0.0, 1.0]).unwrap();
    assert!(verify_anatomical_region_registration([0.5, 0.0, 0.0], [-0.1, -0.1, -0.1], [0.1, 0.1, 0.1], [0.0, 0.0, 1.0]).is_err());
}
