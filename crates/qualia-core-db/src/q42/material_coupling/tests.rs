//! Comprehensive unit test suite for `q42::material_coupling` (Packet M01).

use std::f64::consts::{FRAC_1_SQRT_2, PI};

use super::acoustic_coupling::{
    evaluate_fluid_solid_oblique_partition, evaluate_planar_interface_scattering,
    resolve_emf_material_entry, resolve_qac_material_ref, AcousticCouplingError, CauchyStress6,
    ContinuumMedium, EmfMaterialZeroPolicy, OrthotropicElasticMedium, PowerLawAcousticAttenuation,
    ResolvedEmfMaterial, TensorStrain6,
};
use super::constitutive::{
    ActiveHamiltonRotation, AnisotropicConstitutiveTensor3x3, Complex64, ConstitutiveError,
    ConstitutiveModel, DebyePole, LorentzPole, C_0_M_PER_S, EPSILON_0_F_PER_M,
    ROTATION_ORTHOGONALITY_BUDGET, Z_0_OHMS,
};
use super::dispersion::{
    evaluate_brillouin_energy_density_j_per_m3, evaluate_isotropic_eb_state,
    evaluate_wave_propagation, kramers_kronig_im_from_re, verify_kramers_kronig_consistency,
    DispersionError, EmAmplitudeConvention,
};
use super::transduction::{
    convert_iq_carrier_convention, evaluate_radar_sonar_normal_interface,
    evaluate_thermoelastic_stress, reconstruct_real_passband_sample, IqCarrierConvention,
    PhotoacousticParameters, PiezoelectricConstitutiveModel, TransductionError,
};

#[test]
fn kramers_kronig_debye_and_lorentz_causality_and_passivity() {
    let model = ConstitutiveModel {
        epsilon_infinity_relative: 2.2,
        mu_infinity_relative: 1.0,
        conductivity_s_per_m: 1e-4,
        electric_debye_poles: vec![DebyePole {
            delta_chi: 1.5,
            tau_s: 1.0e-9,
        }],
        electric_lorentz_poles: vec![LorentzPole {
            delta_chi: 0.8,
            omega_0_rad_s: 5.0e9,
            gamma_rad_s: 1.0e9,
        }],
        magnetic_debye_poles: vec![DebyePole {
            delta_chi: 0.4,
            tau_s: 2.0e-9,
        }],
        magnetic_lorentz_poles: vec![],
    };

    let eval_omegas = [2.0e8, 5.0e8, 1.0e9, 2.5e9];
    let report = verify_kramers_kronig_consistency(
        &model,
        &eval_omegas,
        1.0e5,
        2.0e11,
        20_000,
        2.5e-2,
    )
    .expect("Kramers-Kronig passivity and Hilbert integral consistency");

    assert!(report.passivity_satisfied);
    assert!(report.causality_satisfied);
    assert!(report.min_omega_im_eps_r > 0.0);
    assert!(report.min_omega_im_mu_r > 0.0);
    assert!(report.max_electric_kk_abs_error < 2.5e-2);
    assert!(report.max_magnetic_kk_abs_error < 2.5e-2);

    // Also verify inverse Kramers-Kronig (Im from Re) on the Debye pole at omega = 1/tau = 1e9 rad/s
    let target_w = 1.0e9;
    let debye_only = ConstitutiveModel {
        epsilon_infinity_relative: 1.0,
        mu_infinity_relative: 1.0,
        conductivity_s_per_m: 0.0,
        electric_debye_poles: vec![DebyePole {
            delta_chi: 2.0,
            tau_s: 1.0e-9,
        }],
        electric_lorentz_poles: vec![],
        magnetic_debye_poles: vec![],
        magnetic_lorentz_poles: vec![],
    };
    let exact_chi = debye_only.bound_electric_susceptibility(target_w).unwrap();
    let recon_im = kramers_kronig_im_from_re(target_w, 1.0e5, 1.0e11, 16_000, |wp| {
        Ok(debye_only.bound_electric_susceptibility(wp)?.re)
    })
    .unwrap();
    assert!((exact_chi.im - recon_im).abs() < 2.0e-2);

    // Verify Brillouin stored dispersive energy density is strictly positive in normal dispersion regime
    let u_brillouin =
        evaluate_brillouin_energy_density_j_per_m3(&debye_only, 5.0e8, 100.0, 1.0).unwrap();
    assert!(u_brillouin > 0.0);

    // Reject negative oscillator strength (active/non-passive medium)
    let bad_model = ConstitutiveModel {
        electric_debye_poles: vec![DebyePole {
            delta_chi: -0.5,
            tau_s: 1.0e-9,
        }],
        ..ConstitutiveModel::vacuum()
    };
    assert!(matches!(
        bad_model.validate(),
        Err(ConstitutiveError::InvalidDebyePole { index: 0, .. })
    ));

    // Reject zero-frequency wavelength calculation per C02-C02
    assert_eq!(
        evaluate_wave_propagation(&ConstitutiveModel::vacuum(), 0.0).err(),
        Some(DispersionError::ZeroFrequencyWavelengthRejected)
    );
}

#[test]
fn c02_c04_quaternion_admission_sign_invariance_and_mode0_oracle() {
    // 1. Test q = (0, 0, 0, 1 + 2^-43) per C02-C04
    let eps_43 = 2.0f64.powi(-43);
    let raw_q = [0.0, 0.0, 0.0, 1.0 + eps_43];
    let rot = ActiveHamiltonRotation::admit(raw_q).expect("within 1e-12 norm-sq admission");
    // Raw bytes preserved identically
    assert_eq!(rot.raw_xyzw[3].to_bits(), (1.0 + eps_43).to_bits());
    assert!((rot.raw_norm_sq - 1.0).abs() <= 1e-12);
    // Derived rotation matrix is identity within 64 * 2^-52
    assert!(rot.orthogonality_residual() <= ROTATION_ORTHOGONALITY_BUDGET);
    assert!((rot.determinant() - 1.0).abs() <= ROTATION_ORTHOGONALITY_BUDGET);
    for i in 0..3 {
        for j in 0..3 {
            let expected = if i == j { 1.0 } else { 0.0 };
            assert!((rot.matrix[i][j] - expected).abs() <= ROTATION_ORTHOGONALITY_BUDGET);
        }
    }

    // 2. Test sign-equivalent q and -q produce identical rotation matrices and tensor transforms
    let q_pos = ActiveHamiltonRotation::admit([0.0, 0.0, FRAC_1_SQRT_2, FRAC_1_SQRT_2]).unwrap();
    let q_neg = ActiveHamiltonRotation::admit([0.0, 0.0, -FRAC_1_SQRT_2, -FRAC_1_SQRT_2]).unwrap();
    for i in 0..3 {
        for j in 0..3 {
            assert!((q_pos.matrix[i][j] - q_neg.matrix[i][j]).abs() <= 1e-15);
        }
    }

    // 3. +90 deg Z rotation maps (1, 0, 0) -> (0, 1, 0)
    let v_rot = q_pos.rotate_vector([1.0, 0.0, 0.0]);
    assert!((v_rot[0] - 0.0).abs() <= 64.0 * f64::EPSILON);
    assert!((v_rot[1] - 1.0).abs() <= 64.0 * f64::EPSILON);
    assert!((v_rot[2] - 0.0).abs() <= 64.0 * f64::EPSILON);

    // 4. Reject quaternion outside 1e-12 squared-norm tolerance
    assert!(matches!(
        ActiveHamiltonRotation::admit([0.0, 0.0, 0.0, 1.0 + 1e-6]),
        Err(ConstitutiveError::QuaternionNormOutOfBounds { .. })
    ));

    // 5. Mode 0 oracle: E=(1,0,0) V/m, B=(0, 1/c, 0) T in vacuum
    let e0 = [1.0, 0.0, 0.0];
    let b0 = [0.0, 1.0 / C_0_M_PER_S, 0.0];
    let snap = evaluate_isotropic_eb_state(
        e0,
        b0,
        1.0,
        1.0,
        EmAmplitudeConvention::InstantaneousSnapshot,
    )
    .unwrap();
    let expected_s = 1.0 / Z_0_OHMS;
    assert!((snap.poynting_w_per_m2[2] - expected_s).abs() <= 1e-9);
    assert!((snap.energy_density_j_per_m3 - EPSILON_0_F_PER_M).abs() <= 1e-22);

    // Peak phasor average is half S; RMS phasor gives same average as snapshot
    let peak =
        evaluate_isotropic_eb_state(e0, b0, 1.0, 1.0, EmAmplitudeConvention::PeakPhasor).unwrap();
    let rms =
        evaluate_isotropic_eb_state(e0, b0, 1.0, 1.0, EmAmplitudeConvention::RmsPhasor).unwrap();
    assert!((peak.poynting_w_per_m2[2] - 0.5 * snap.poynting_w_per_m2[2]).abs() <= 1e-15);
    assert!((rms.poynting_w_per_m2[2] - snap.poynting_w_per_m2[2]).abs() <= 1e-15);

    // Rotate +90 deg about Z: E -> +y, B -> -x, S stays +z
    let e_rot = q_pos.rotate_vector(e0);
    let b_rot = q_pos.rotate_vector(b0);
    assert!((e_rot[1] - 1.0).abs() <= 64.0 * f64::EPSILON);
    assert!((b_rot[0] + 1.0 / C_0_M_PER_S).abs() <= 64.0 * f64::EPSILON);
    let snap_rot = evaluate_isotropic_eb_state(
        e_rot,
        b_rot,
        1.0,
        1.0,
        EmAmplitudeConvention::InstantaneousSnapshot,
    )
    .unwrap();
    assert!((snap_rot.poynting_w_per_m2[2] - snap.poynting_w_per_m2[2]).abs() <= 1e-15);

    // 6. Anisotropic 3x3 tensor rotation preserves trace and symmetry
    let aniso = AnisotropicConstitutiveTensor3x3::from_principal_axes([
        Complex64::new(4.0, 0.2),
        Complex64::new(2.0, 0.1),
        Complex64::new(3.0, 0.15),
    ])
    .unwrap();
    let aniso_rot = aniso.transform_to_parent(&q_pos);
    aniso_rot.validate_passivity_and_symmetry(1e-12).unwrap();
    assert!((aniso_rot.trace().re - aniso.trace().re).abs() <= 1e-14);
    assert!((aniso_rot.trace().im - aniso.trace().im).abs() <= 1e-14);
    // +90 deg Z swaps xx (4.0) and yy (2.0)
    assert!((aniso_rot.components[0][0].re - 2.0).abs() <= 1e-14);
    assert!((aniso_rot.components[1][1].re - 4.0).abs() <= 1e-14);
}

#[test]
fn acoustic_continuum_oracles_impedance_scattering_and_namespace_separation() {
    // QAC Mode 00 oracle: water rho=1000, K=2.25e9 => c=1500, Z=1.5e6
    let water = ContinuumMedium::new_fluid(1000.0, 2.25e9, 101_325.0).unwrap();
    assert!((water.longitudinal_speed_m_per_s() - 1500.0).abs() <= 1e-12);
    assert!((water.characteristic_impedance_rayl() - 1.5e6).abs() <= 1e-9);
    assert_eq!(water.shear_speed_m_per_s(), 0.0);

    // QAC Mode 01 oracle: solid rho=2000, K=8e9, mu=3e9 => lambda=6e9, cP=sqrt(6e6), cS=sqrt(1.5e6)
    let solid = ContinuumMedium::new_isotropic_solid(2000.0, 8.0e9, 3.0e9, 101_325.0).unwrap();
    assert!((solid.lame_lambda_pa() - 6.0e9).abs() <= 1e-6);
    assert!((solid.longitudinal_speed_m_per_s() - 6.0e6_f64.sqrt()).abs() <= 1e-9);
    assert!((solid.shear_speed_m_per_s() - 1.5e6_f64.sqrt()).abs() <= 1e-9);

    // Strain diag(1e-6, -1e-6, 0) => sigma diag(6000, -6000, 0) Pa and pressure = 0
    let strain_dev = TensorStrain6 {
        xx_yy_zz_xy_yz_xz: [1e-6, -1e-6, 0.0, 0.0, 0.0, 0.0],
    };
    let stress_dev = CauchyStress6::from_isotropic_strain(&solid, &strain_dev);
    assert!((stress_dev.xx_yy_zz_xy_yz_xz_pa[0] - 6000.0).abs() <= 1e-9);
    assert!((stress_dev.xx_yy_zz_xy_yz_xz_pa[1] + 6000.0).abs() <= 1e-9);
    assert!((stress_dev.xx_yy_zz_xy_yz_xz_pa[2] - 0.0).abs() <= 1e-9);
    assert!((stress_dev.mechanical_pressure_pa() - 0.0).abs() <= 1e-12);

    // Rotate stress by +90 deg about Z: trace and pressure remain 0, xx and yy swap signs
    let q_z90 = ActiveHamiltonRotation::admit([0.0, 0.0, FRAC_1_SQRT_2, FRAC_1_SQRT_2]).unwrap();
    let stress_rot = stress_dev.rotate_to_parent(&q_z90);
    assert!((stress_rot.xx_yy_zz_xy_yz_xz_pa[0] + 6000.0).abs() <= 1e-9);
    assert!((stress_rot.xx_yy_zz_xy_yz_xz_pa[1] - 6000.0).abs() <= 1e-9);
    assert!((stress_rot.mechanical_pressure_pa() - 0.0).abs() <= 1e-12);

    // Hydrostatic stress diag(-3, -3, -3) Pa gives p = +3 Pa
    let hydro = CauchyStress6 {
        xx_yy_zz_xy_yz_xz_pa: [-3.0, -3.0, -3.0, 0.0, 0.0, 0.0],
    };
    assert!((hydro.mechanical_pressure_pa() - 3.0).abs() <= 1e-15);

    // Auxetic solid with negative Lame lambda (e.g. K=1e9, mu=2e9 => lambda = -1/3 e9) is admissible per C02-C06
    let auxetic = ContinuumMedium::new_isotropic_solid(1500.0, 1.0e9, 2.0e9, 0.0).unwrap();
    assert!(auxetic.lame_lambda_pa() < 0.0);
    assert!(auxetic.longitudinal_speed_m_per_s() > auxetic.shear_speed_m_per_s());

    // QAC Mode 03 oracle: Z1=1.5e6, Z2=3.0e6 => r=1/3, t=4/3, R_E=1/9, T_E=8/9, R_E+T_E=1
    let medium_z2 = ContinuumMedium::new_fluid(2000.0, 4.5e9, 101_325.0).unwrap();
    assert!((medium_z2.characteristic_impedance_rayl() - 3.0e6).abs() <= 1e-9);
    let scat = evaluate_planar_interface_scattering(&water, &medium_z2, 0.0).unwrap();
    assert!((scat.pressure_reflection - (1.0 / 3.0)).abs() <= 1e-12);
    assert!((scat.pressure_transmission - (4.0 / 3.0)).abs() <= 1e-12);
    assert!((scat.energy_reflection - (1.0 / 9.0)).abs() <= 1e-12);
    assert!((scat.energy_transmission - (8.0 / 9.0)).abs() <= 1e-12);
    assert!((scat.energy_reflection + scat.energy_transmission - 1.0).abs() <= 1e-15);

    // Oblique fluid-solid wave partitioning preserves total energy R_P + T_P + T_SV = 1
    let part = evaluate_fluid_solid_oblique_partition(&water, &solid, 0.35).unwrap();
    let total_e =
        part.energy_reflection_p + part.energy_transmission_p + part.energy_transmission_sv;
    assert!((total_e - 1.0).abs() <= 1e-14);
    assert!(part.energy_transmission_sv > 0.0);

    // Power-law attenuation & Kramers-Kronig phase velocity dispersion
    let atten = PowerLawAcousticAttenuation {
        omega_0_rad_s: 2.0 * PI * 1.0e6,
        c_0_m_per_s: 1500.0,
        alpha_0_np_per_m_rad_s_y: 1.0e-6,
        exponent_y: 1.0,
    };
    let c_low = atten.phase_velocity_m_per_s(2.0 * PI * 0.5e6).unwrap();
    let c_high = atten.phase_velocity_m_per_s(2.0 * PI * 2.0e6).unwrap();
    // Causal absorbing medium has monotonically increasing phase speed with frequency for y=1
    assert!(c_high > c_low);

    // Orthotropic elastic medium validation
    let ortho = OrthotropicElasticMedium {
        density_kg_per_m3: 2500.0,
        c11: 40e9,
        c22: 35e9,
        c33: 30e9,
        c12: 10e9,
        c13: 8e9,
        c23: 9e9,
        c44: 12e9,
        c55: 11e9,
        c66: 13e9,
    };
    ortho.validate().unwrap();

    // C02-C06 namespace separation: EMF material_entry == 0 vs QAC material_ref == 0
    assert_eq!(
        resolve_emf_material_entry(0, 4, EmfMaterialZeroPolicy::UnboundRejected),
        Err(AcousticCouplingError::EmfMaterialZeroUnbound)
    );
    assert_eq!(
        resolve_emf_material_entry(0, 4, EmfMaterialZeroPolicy::ExplicitVacuumAdmitted),
        Ok(ResolvedEmfMaterial::ExplicitVacuum)
    );
    assert_eq!(
        resolve_qac_material_ref(0, None, &[water]),
        Err(AcousticCouplingError::QacUniformMediumNotDeclared)
    );
    assert_eq!(
        resolve_qac_material_ref(0, Some(&water), &[]),
        Ok(&water)
    );
    // Reject rho = 0 vacuum substitution in ContinuumMedium
    assert_eq!(
        ContinuumMedium::new_fluid(0.0, 2.25e9, 0.0),
        Err(AcousticCouplingError::InvalidDensity { rho_kg_per_m3: 0.0 })
    );
}

#[test]
fn cross_domain_photoacoustic_piezoelectric_thermoelastic_and_iq_carrier_conjugation() {
    // 1. Photoacoustic coupling: soft tissue / water-like phantom
    let pa = PhotoacousticParameters {
        optical_absorption_per_m: 100.0,       // 1 cm^-1
        thermal_expansion_per_k: 2.1e-4,       // K^-1
        specific_heat_cp_j_per_kg_k: 4184.0,   // J/(kg K)
        density_kg_per_m3: 1000.0,
        sound_speed_m_per_s: 1500.0,
        thermal_diffusivity_m2_per_s: 1.4e-7,
        characteristic_length_m: 1.0e-3,       // 1 mm -> tau_st = 666.7 ns
    };
    let resp = pa
        .evaluate_confined_excitation(200.0, 10.0e-9) // 20 mJ/cm^2 = 200 J/m^2, 10 ns laser pulse
        .expect("confined photoacoustic excitation");
    let expected_gamma = (2.1e-4 * 1500.0 * 1500.0) / 4184.0;
    assert!((resp.gruneisen_parameter - expected_gamma).abs() <= 1e-12);
    assert!((resp.initial_pressure_pa - expected_gamma * 20_000.0).abs() <= 1e-9);
    assert!(resp.acoustic_energy_density_j_per_m3 < resp.absorbed_energy_density_j_per_m3);

    // Reject pulse duration exceeding stress confinement time (e.g. 1 ms pulse > 666.7 ns)
    assert!(matches!(
        pa.evaluate_confined_excitation(200.0, 1.0e-3),
        Err(TransductionError::StressConfinementViolated { .. })
    ));

    // 2. Piezoelectric coupling: quartz / PZT-like crystal
    let mut c_e = [[0.0; 6]; 6];
    for m in 0..6 {
        c_e[m][m] = 100.0e9;
    }
    let eps_s = [
        [100.0 * EPSILON_0_F_PER_M, 0.0, 0.0],
        [0.0, 100.0 * EPSILON_0_F_PER_M, 0.0],
        [0.0, 0.0, 100.0 * EPSILON_0_F_PER_M],
    ];
    let mut piezo_e = [[0.0; 6]; 3];
    piezo_e[2][2] = 5.0; // e_33 = 5 C/m^2 -> k_33^2 = 25 / (1e11 * 8.854e-10) = 0.282 < 1
    let piezo = PiezoelectricConstitutiveModel {
        stiffness_c_e_pa: c_e,
        permittivity_eps_s_f_per_m: eps_s,
        piezo_e_c_per_m2: piezo_e,
    };
    let k_sq = piezo.validate().expect("stable electromechanical coupling");
    assert!(k_sq > 0.28 && k_sq < 0.29);

    let strain = TensorStrain6 {
        xx_yy_zz_xy_yz_xz: [0.0, 0.0, 1.0e-5, 0.0, 0.0, 0.0],
    };
    let e_field = [0.0, 0.0, 1.0e4];
    let p_state = piezo.evaluate_coupled_state(&strain, e_field).unwrap();
    // Cross terms cancel in 0.5*(T*S + D*E) == u_mech + u_elec
    assert!(
        (p_state.total_internal_energy_j_per_m3
            - (p_state.mechanical_energy_j_per_m3 + p_state.dielectric_energy_j_per_m3))
            .abs()
            <= 1e-12
    );
    assert!(piezo.open_circuit_stiffened_modulus_pa(2, 2) > c_e[2][2]);

    // Reject unstable electromechanical coupling k^2 >= 1
    let mut bad_piezo = piezo;
    bad_piezo.piezo_e_c_per_m2[2][2] = 50.0;
    assert!(matches!(
        bad_piezo.validate(),
        Err(TransductionError::ElectromechanicalCouplingUnstable { .. })
    ));

    // 3. Duhamel-Neumann thermo-elastic stress
    let solid = ContinuumMedium::new_isotropic_solid(2700.0, 50.0e9, 30.0e9, 0.0).unwrap();
    let zero_strain = TensorStrain6::default();
    let th_stress = evaluate_thermoelastic_stress(&solid, &zero_strain, 1.0e-5, 10.0).unwrap();
    // Constrained heating (strain=0, dT=+10 K) produces compressive stress: -3 * K * alpha_L * dT = -1.5e7 Pa
    assert!((th_stress.xx_yy_zz_xy_yz_xz_pa[0] + 1.5e7).abs() <= 1e-6);
    assert!((th_stress.mechanical_pressure_pa() - 1.5e7).abs() <= 1e-6);

    // 4. Radar/Sonar co-located normal interface energy conservation
    let air_em = ConstitutiveModel::vacuum();
    let water_em = ConstitutiveModel {
        epsilon_infinity_relative: 81.0,
        ..ConstitutiveModel::vacuum()
    };
    let air_ac = ContinuumMedium::new_fluid(1.2, 1.42e5, 101_325.0).unwrap();
    let water_ac = ContinuumMedium::new_fluid(1000.0, 2.25e9, 101_325.0).unwrap();
    let rs = evaluate_radar_sonar_normal_interface(
        &air_em,
        &air_ac,
        &water_em,
        &water_ac,
        2.0 * PI * 1.0e9,
    )
    .unwrap();
    // For n1=1, n2=9: r_em = (1 - 9)/(1 + 9) = -0.8 => R_EM = 0.64, T_EM = 0.36
    assert!((rs.em_reflection_amplitude.re + 0.8).abs() <= 1e-12);
    assert!((rs.em_power_reflection + rs.em_power_transmission - 1.0).abs() <= 1e-12);
    assert!(
        (rs.acoustic.energy_reflection + rs.acoustic.energy_transmission - 1.0).abs() <= 1e-12
    );

    // 5. C02-C08 I/Q carrier convention conjugation test
    // At quarter-cycle (2*pi*fc*dt = pi/2, phi0 = 0):
    // I=1, Q=0 -> cosine(pi/2) = 0; I=0, Q=1 -> +1 in EMF, -1 in QAC without conjugation
    let fc = 1000.0;
    let dt_quarter = 0.25 / fc;
    let z_i = Complex64::new(1.0, 0.0);
    let z_q = Complex64::new(0.0, 1.0);
    assert!(
        reconstruct_real_passband_sample(
            z_i,
            fc,
            0.0,
            dt_quarter,
            IqCarrierConvention::EmfNegativeCarrier
        )
        .abs()
            <= 1e-15
    );
    let emf_q_quarter = reconstruct_real_passband_sample(
        z_q,
        fc,
        0.0,
        dt_quarter,
        IqCarrierConvention::EmfNegativeCarrier,
    );
    let qac_unconverted_quarter = reconstruct_real_passband_sample(
        z_q,
        fc,
        0.0,
        dt_quarter,
        IqCarrierConvention::QacPositiveCarrier,
    );
    assert!((emf_q_quarter - 1.0).abs() <= 1e-15);
    assert!((qac_unconverted_quarter + 1.0).abs() <= 1e-15);
    // Unconverted Q has opposite sign!
    assert_ne!(emf_q_quarter > 0.0, qac_unconverted_quarter > 0.0);

    // Converted sample z_QAC = conj(z_EMF) with phase offset conversion matches real waveform at all times
    let z_arbitrary = Complex64::new(0.75, -1.25);
    let z_qac = convert_iq_carrier_convention(
        z_arbitrary,
        IqCarrierConvention::EmfNegativeCarrier,
        0.3,
        IqCarrierConvention::QacPositiveCarrier,
        -0.4,
    )
    .unwrap();
    for step in 0..16 {
        let t = (step as f64) * 0.000_07;
        let w_emf = reconstruct_real_passband_sample(
            z_arbitrary,
            fc,
            0.3,
            t,
            IqCarrierConvention::EmfNegativeCarrier,
        );
        let w_qac = reconstruct_real_passband_sample(
            z_qac,
            fc,
            -0.4,
            t,
            IqCarrierConvention::QacPositiveCarrier,
        );
        assert!((w_emf - w_qac).abs() <= 1e-14);
    }
}
