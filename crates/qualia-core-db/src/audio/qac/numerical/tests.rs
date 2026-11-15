//! Comprehensive unit tests for QAC continuum numerical solvers, room acoustics,
//! Helmholtz frequency-domain solvers, Kramers-Kronig attenuation-dispersion causality,
//! and WGSL compute kernel adapters.

use core::f64::consts::PI;
use super::{
    assert_b32_scalar_budget,
    attenuation_kk::{MultiRelaxationMedium, PowerLawAttenuation},
    elastic_virieux::{
        decode_elastic_stress_velocity_records, encode_elastic_stress_velocity_records,
        step_elastic_virieux_in_place, CauchyStress6, ElasticBoundaryCondition, ElasticMedium,
        ElasticStressVelocityRecord, ElasticVirieuxConfig, ElasticVirieuxState,
        HamiltonQuaternionXyzw, NormalIncidenceInterface, TensorStrain6,
        ELASTIC_STRESS_VELOCITY_RECORD_STRIDE, ESV_DEFINED_VALIDITY_MASK, ESV_VALID_VELOCITY,
    },
    fdtd_fluid::{
        collocate_fluid_cell, decode_pressure_velocity_records, encode_pressure_velocity_records,
        evaluate_direct_energy_and_intensity, step_fluid_fdtd_in_place, FluidBoundaryCondition,
        FluidFdtdConfig, FluidFdtdState, FluidMedium, NumericalError, PressureVelocityRecord,
        StaggeredFluidOffsets, PRESSURE_VELOCITY_RECORD_STRIDE, PV_DEFINED_VALIDITY_MASK,
        PV_VALID_POSITION,
    },
    helmholtz::{
        apply_helmholtz_operator, helmholtz_greens_function_3d,
        rectangular_cavity_eigenfrequency_hz, rectangular_cavity_eigenmode_value,
        solve_helmholtz_bicgstab, step_helmholtz_jacobi, Complex64, HelmholtzBoundary,
        HelmholtzConfig, HelmholtzStencil, HelmholtzWorkspace,
    },
    room_acoustics::{
        analyze_room_impulse_response, schroeder_backward_integration_db, RoomGeometry,
        SurfacePatch,
    },
};

#[test]
fn c02_mode00_water_algebra_and_scale_factors() {
    let water = FluidMedium::new(1000.0, 2.25e9, 0.0, 0).expect("valid water medium");
    assert!((water.sound_speed_m_s() - 1500.0).abs() <= 1e-12);
    assert!((water.characteristic_impedance_rayl() - 1.5e6).abs() <= 1e-9);

    for &scale in &[1e-3_f64, 1.0_f64, 1e3_f64] {
        let p_peak = 1500.0 * scale;
        let v_peak = [0.001 * scale, 0.0, 0.0];
        let i_expected = 1.5 * scale * scale;
        let e_expected = 0.001 * scale * scale;
        let i_mean_expected = 0.75 * scale * scale;

        let (e_actual, i_actual) = evaluate_direct_energy_and_intensity(
            &water,
            &StaggeredFluidOffsets::collocated(),
            p_peak,
            v_peak,
        )
        .expect("collocated evaluation succeeds");

        assert!((i_actual[0] - i_expected).abs() <= 1e-14 * i_expected);
        assert_eq!(i_actual[1], 0.0);
        assert_eq!(i_actual[2], 0.0);
        assert!((e_actual - e_expected).abs() <= 1e-14 * e_expected);

        let i_mean = FluidMedium::peak_phasor_mean_intensity_w_m2(p_peak, v_peak, 0.0);
        assert!((i_mean[0] - i_mean_expected).abs() <= 1e-14 * i_mean_expected);

        let rec = PressureVelocityRecord::from_collocated_state(
            [0.0, 0.0, 0.0],
            p_peak as f32,
            [v_peak[0] as f32, 0.0, 0.0],
            &water,
        )
        .expect("valid f32 record");
        assert_b32_scalar_budget(rec.intensity_w_m2[0] as f64, i_expected, i_expected);
        assert_b32_scalar_budget(rec.energy_density_j_m3 as f64, e_expected, e_expected);
    }

    let err = evaluate_direct_energy_and_intensity(
        &water,
        &StaggeredFluidOffsets::canonical_staggered(),
        1500.0,
        [0.001, 0.0, 0.0],
    )
    .expect_err("staggered offsets must reject direct product");
    assert_eq!(err, NumericalError::NonCollocatedFieldProduct);
}

#[test]
fn c02_mode00_counterpropagating_waves_and_wire_record() {
    let water = FluidMedium::new(1000.0, 2.25e9, 0.0, 1).unwrap();
    let z = water.characteristic_impedance_rayl();
    let p0 = 1500.0;
    let v0 = p0 / z;

    let n_samples = 64usize;
    let mut sum_intensity = 0.0f64;
    let mut sum_energy = 0.0f64;
    for it in 0..n_samples {
        let wt = 2.0 * PI * (it as f64) / (n_samples as f64);
        for ix in 0..n_samples {
            let kx = 2.0 * PI * (ix as f64) / (n_samples as f64);
            let p_fwd = p0 * (kx - wt).cos();
            let p_bwd = p0 * (kx + wt).cos();
            let p = p_fwd + p_bwd;
            let vx = v0 * (kx - wt).cos() - v0 * (kx + wt).cos();
            sum_intensity += FluidMedium::instantaneous_intensity_w_m2(p, [vx, 0.0, 0.0])[0];
            sum_energy += water.energy_density_j_m3(p, [vx, 0.0, 0.0]);
        }
    }
    let count = (n_samples * n_samples) as f64;
    let mean_intensity = sum_intensity / count;
    let mean_energy = sum_energy / count;
    assert!(mean_intensity.abs() <= 1e-14);
    assert!((mean_energy - 0.001).abs() <= 1e-14);

    let rec = PressureVelocityRecord::from_collocated_state(
        [1.0, -2.0, 3.5],
        1500.0,
        [0.001, 0.0, 0.0],
        &water,
    )
    .unwrap();
    let mut bytes = rec.to_wire_bytes().unwrap();
    assert_eq!(bytes.len(), PRESSURE_VELOCITY_RECORD_STRIDE);
    let decoded = PressureVelocityRecord::from_wire_bytes(&bytes).unwrap();
    assert_eq!(decoded, rec);

    let batch_bytes = encode_pressure_velocity_records(&[rec, rec]).unwrap();
    let batch_dec = decode_pressure_velocity_records(&batch_bytes).unwrap();
    assert_eq!(batch_dec, vec![rec, rec]);

    bytes[60] = 1;
    assert!(PressureVelocityRecord::from_wire_bytes(&bytes).is_err());

    let mut bad_rec = rec;
    bad_rec.validity_bits = PV_VALID_POSITION;
    assert!(bad_rec.validate().is_err());
    bad_rec.validity_bits = PV_DEFINED_VALIDITY_MASK | (1 << 31);
    assert!(bad_rec.validate().is_err());
}

#[test]
fn c02_mode00_fdtd_courant_gate_convergence_and_bitwise_restart() {
    let water = FluidMedium::new(1000.0, 2.25e9, 0.0, 0).unwrap();
    let c = water.sound_speed_m_s();

    let dx = 1.0_f64 / 32.0;
    let cfl_max_dt = 1.0 / (c * (3.0 / (dx * dx)).sqrt());
    let bad_cfg = FluidFdtdConfig {
        dims: [32, 4, 4],
        spacing_m: [dx, dx, dx],
        dt_s: 1.01 * cfl_max_dt,
        medium: water,
        boundary: FluidBoundaryCondition::Periodic,
    };
    let mut dummy_state = FluidFdtdState {
        step_index: 0,
        p: vec![0.0; 32 * 16],
        vx: vec![0.0; 32 * 16],
        vy: vec![0.0; 32 * 16],
        vz: vec![0.0; 32 * 16],
        prev_vx: vec![0.0; 32 * 16],
        prev_vy: vec![0.0; 32 * 16],
        prev_vz: vec![0.0; 32 * 16],
        cpml_psi: Vec::new(),
    };
    assert!(matches!(
        step_fluid_fdtd_in_place(&bad_cfg, &mut dummy_state),
        Err(NumericalError::CourantViolation { .. })
    ));

    let z = water.characteristic_impedance_rayl();
    let length_m = 1.0;
    let k_wave = 2.0 * PI / length_m;
    let omega = c * k_wave;
    let p_amp = 1500.0;
    let v_amp = p_amp / z;

    let mut errors = Vec::new();
    for &nx in &[16usize, 32usize, 64usize] {
        let h = length_m / (nx as f64);
        let max_dt = 1.0 / (c * (3.0 / (h * h)).sqrt());
        let dt = 0.5 * max_dt;
        let cfg = FluidFdtdConfig {
            dims: [nx, 2, 2],
            spacing_m: [h, h, h],
            dt_s: dt,
            medium: water,
            boundary: FluidBoundaryCondition::Periodic,
        };
        let mut state = FluidFdtdState::new(&cfg).unwrap();
        let idx = |ix: usize, iy: usize, iz: usize| (ix * 2 + iy) * 2 + iz;

        for ix in 0..nx {
            let x_p = (ix as f64) * h;
            let x_v = (ix as f64 + 0.5) * h;
            let p_val = p_amp * (k_wave * x_p).cos();
            let vx_val = v_amp * (k_wave * x_v - omega * 0.5 * dt).cos();
            let prev_vx_val = v_amp * (k_wave * x_v + omega * 0.5 * dt).cos();
            for iy in 0..2 {
                for iz in 0..2 {
                    let c_idx = idx(ix, iy, iz);
                    state.p[c_idx] = p_val;
                    state.vx[c_idx] = vx_val;
                    state.prev_vx[c_idx] = prev_vx_val;
                }
            }
        }

        let total_steps = nx;
        let half_steps = total_steps / 2;
        for _ in 0..half_steps {
            step_fluid_fdtd_in_place(&cfg, &mut state).unwrap();
        }

        let restart_bytes = state.encode_restart_bytes();
        let mut restarted = FluidFdtdState::decode_restart_bytes(&cfg, &restart_bytes).unwrap();

        for _ in half_steps..total_steps {
            step_fluid_fdtd_in_place(&cfg, &mut state).unwrap();
            step_fluid_fdtd_in_place(&cfg, &mut restarted).unwrap();
        }

        assert_eq!(state.step_index, restarted.step_index);
        for (a, b) in state.p.iter().zip(restarted.p.iter()) {
            assert_eq!(a.to_bits(), b.to_bits());
        }
        for (a, b) in state.vx.iter().zip(restarted.vx.iter()) {
            assert_eq!(a.to_bits(), b.to_bits());
        }

        let (p_c, v_c, e_c, i_c) = collocate_fluid_cell(
            &cfg,
            &state.p,
            (&state.prev_vx, &state.prev_vy, &state.prev_vz),
            (&state.vx, &state.vy, &state.vz),
            [0, 0, 0],
        )
        .unwrap();
        assert!(p_c.is_finite() && v_c[0].is_finite() && e_c >= 0.0 && i_c[0].is_finite());

        let t_final = (total_steps as f64) * dt;
        let mut err_sq = 0.0f64;
        let mut ref_sq = 0.0f64;
        for ix in 0..nx {
            let x = (ix as f64) * h;
            let p_ref = p_amp * (k_wave * x - omega * t_final).cos();
            let diff = state.p[idx(ix, 0, 0)] - p_ref;
            err_sq += diff * diff;
            ref_sq += p_ref * p_ref;
        }
        errors.push((err_sq / ref_sq).sqrt());
    }

    let order_16_32 = (errors[0] / errors[1]).log2();
    let order_32_64 = (errors[1] / errors[2]).log2();
    assert!(order_16_32 >= 1.8 && order_32_64 >= 1.8);
    assert!(errors[2] <= 0.01);
}

#[test]
fn c02_mode00_boundaries_and_wgsl_forge_f32_adapter() {
    let water = FluidMedium::new(1000.0, 2.25e9, 0.0, 0).unwrap();
    let h = 0.05;
    let dt = 0.4 * (h / (water.sound_speed_m_s() * 3.0_f64.sqrt()));

    for boundary in [
        FluidBoundaryCondition::Rigid,
        FluidBoundaryCondition::PressureRelease,
        FluidBoundaryCondition::EngquistMajdaFirstOrder,
        FluidBoundaryCondition::Cpml {
            layers: 2,
            sigma_max: 500.0,
            kappa_max: 2.0,
            alpha_max: 10.0,
        },
    ] {
        let cfg = FluidFdtdConfig {
            dims: [10, 8, 8],
            spacing_m: [h, h, h],
            dt_s: dt,
            medium: water,
            boundary,
        };
        let mut state = FluidFdtdState::new(&cfg).unwrap();
        state.p[(5 * 8 + 4) * 8 + 4] = 100.0;
        for _ in 0..8 {
            step_fluid_fdtd_in_place(&cfg, &mut state).unwrap();
        }
        if matches!(boundary, FluidBoundaryCondition::PressureRelease) {
            assert_eq!(state.p[0], 0.0);
        }
        let bytes = state.encode_restart_bytes();
        let restored = FluidFdtdState::decode_restart_bytes(&cfg, &bytes).unwrap();
        assert_eq!(restored, state);
    }

    #[cfg(feature = "gpu-runtime")]
    {
        let cfg_periodic = FluidFdtdConfig {
            dims: [6, 6, 6],
            spacing_m: [h, h, h],
            dt_s: dt,
            medium: water,
            boundary: FluidBoundaryCondition::Periodic,
        };
        let mut state_f64 = FluidFdtdState::new(&cfg_periodic).unwrap();
        state_f64.p[(3 * 6 + 3) * 6 + 3] = 250.0;
        let gpu_params = super::fdtd_fluid::fluid_config_to_gpu_params(&cfg_periodic).unwrap();
        let mut flat_f32 = super::fdtd_fluid::pack_fluid_fdtd_state_f32(&state_f64);
        super::fdtd_fluid::acoustic_fluid_fdtd_step_cpu(&mut flat_f32, &gpu_params).unwrap();
        super::fdtd_fluid::unpack_fluid_fdtd_state_f32(&flat_f32, &mut state_f64).unwrap();
        assert_eq!(state_f64.step_index, 1);
    }
}

#[test]
fn c02_mode01_elastic_solid_stress_strain_and_virieux() {
    let solid = ElasticMedium::new_solid(2000.0, 8.0e9, 3.0e9, 1).unwrap();
    assert!((solid.lame_lambda_pa() - 6.0e9).abs() <= 1e-6);
    assert!((solid.p_wave_speed_m_s() - 6.0e6_f64.sqrt()).abs() <= 1e-12);
    assert!((solid.s_wave_speed_m_s() - 1.5e6_f64.sqrt()).abs() <= 1e-12);

    let strain_dev = TensorStrain6::new([1e-6, -1e-6, 0.0, 0.0, 0.0, 0.0]);
    let stress_dev = solid.stress_from_tensor_strain(&strain_dev);
    assert!((stress_dev.components[0] - 6000.0).abs() <= 1e-9);
    assert!((stress_dev.components[1] + 6000.0).abs() <= 1e-9);
    assert_eq!(stress_dev.components[2], 0.0);
    assert!(stress_dev.mechanical_pressure_pa().abs() <= 1e-12);

    let stress_hydro = CauchyStress6::new([-3.0, -3.0, -3.0, 0.0, 0.0, 0.0]);
    assert!((stress_hydro.mechanical_pressure_pa() - 3.0).abs() <= 1e-15);

    let voigt = [0.0, 0.0, 0.0, 2e-6, 0.0, 0.0];
    let tensor_from_voigt = TensorStrain6::from_engineering_voigt(voigt);
    assert!((tensor_from_voigt.components[3] - 1e-6).abs() <= 1e-20);
    let shear_stress = solid.stress_from_tensor_strain(&tensor_from_voigt);
    assert!((shear_stress.components[3] - 6000.0).abs() <= 1e-9);

    assert!(ElasticMedium::new_solid(0.0, 8.0e9, 3.0e9, 0).is_err());

    let arb_stress = CauchyStress6::new([100.0, 200.0, 300.0, 40.0, 50.0, 60.0]);
    let free_x = arb_stress.project_free_surface_axis(0);
    assert_eq!(free_x.traction_vector([1.0, 0.0, 0.0]), [0.0, 0.0, 0.0]);
    let fluid_z = arb_stress.project_fluid_interface_axis(2, 1500.0);
    assert_eq!(fluid_z.traction_vector([0.0, 0.0, 1.0]), [0.0, 0.0, -1500.0]);

    let rec = ElasticStressVelocityRecord {
        velocity_m_s: [0.01, -0.02, 0.03],
        material_ref: 1,
        stress_xx_yy_zz_xy_yz_xz_pa: [6000.0, -6000.0, 0.0, 0.0, 0.0, 0.0],
        displacement_m: [1e-6, -1e-6, 0.0],
        validity_bits: ESV_DEFINED_VALIDITY_MASK,
    };
    let mut wire = rec.to_wire_bytes().unwrap();
    assert_eq!(wire.len(), ELASTIC_STRESS_VELOCITY_RECORD_STRIDE);
    let dec = ElasticStressVelocityRecord::from_wire_bytes(&wire).unwrap();
    assert_eq!(dec, rec);
    assert!((dec.mechanical_pressure_pa()).abs() <= 1e-12);

    let batch_wire = encode_elastic_stress_velocity_records(&[rec, rec]).unwrap();
    let batch_dec = decode_elastic_stress_velocity_records(&batch_wire).unwrap();
    assert_eq!(batch_dec, vec![rec, rec]);

    wire[63] = 0xFF;
    assert!(ElasticStressVelocityRecord::from_wire_bytes(&wire).is_err());

    let mut bad_esv = rec;
    bad_esv.validity_bits = ESV_VALID_VELOCITY;
    assert!(bad_esv.validate().is_err());

    let h = 0.1_f64;
    let max_dt = 1.0 / (solid.p_wave_speed_m_s() * (3.0 / (h * h)).sqrt());
    let bad_vcfg = ElasticVirieuxConfig {
        dims: [6, 6, 6],
        spacing_m: [h, h, h],
        dt_s: 1.05 * max_dt,
        medium: solid,
        boundary: ElasticBoundaryCondition::Periodic,
    };
    assert!(ElasticVirieuxState::new(&bad_vcfg).is_err());

    let vcfg = ElasticVirieuxConfig {
        dims: [8, 8, 8],
        spacing_m: [h, h, h],
        dt_s: 0.5 * max_dt,
        medium: solid,
        boundary: ElasticBoundaryCondition::FreeSurface,
    };
    let mut vstate = ElasticVirieuxState::new(&vcfg).unwrap();
    vstate.vx[(4 * 8 + 4) * 8 + 4] = 0.01;
    for _ in 0..6 {
        step_elastic_virieux_in_place(&vcfg, &mut vstate).unwrap();
    }
    assert_eq!(vstate.sxx[0], 0.0);

    #[cfg(feature = "gpu-runtime")]
    {
        let gpu_eparams = super::elastic_virieux::elastic_config_to_gpu_params(&vcfg).unwrap();
        let mut flat_e = super::elastic_virieux::pack_elastic_virieux_state_f32(&vstate);
        super::elastic_virieux::elastic_virieux_fdtd_step_cpu(&mut flat_e, &gpu_eparams).unwrap();
        super::elastic_virieux::unpack_elastic_virieux_state_f32(&flat_e, &mut vstate).unwrap();
        assert_eq!(vstate.step_index, 7);
    }
}

#[test]
fn c02_c04_hamilton_quaternion_and_stress_invariance() {
    let s = (0.25 * PI).sin();
    let c = (0.25 * PI).cos();
    let q_z90 = HamiltonQuaternionXyzw::admit([0.0, 0.0, s, c]).unwrap();
    let v_rot = q_z90.rotate_vector([1.0, 0.0, 0.0]);
    assert!(v_rot[0].abs() <= 1e-15);
    assert!((v_rot[1] - 1.0).abs() <= 1e-15);
    assert!(v_rot[2].abs() <= 1e-15);

    let q_neg = HamiltonQuaternionXyzw::admit([0.0, 0.0, -s, -c]).unwrap();
    assert_eq!(q_z90.rotation_matrix(), q_neg.rotation_matrix());

    let eps_w = 1.0 + 2.0_f64.powi(-43);
    let raw_q = [0.0, 0.0, 0.0, eps_w];
    let q_pert = HamiltonQuaternionXyzw::admit(raw_q).unwrap();
    assert_eq!(q_pert.raw_xyzw[3].to_bits(), eps_w.to_bits());
    assert!(q_pert.orthogonality_residual() <= 64.0 * 2.0_f64.powi(-52));
    assert_eq!(
        q_pert.rotation_matrix(),
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
    );

    assert!(HamiltonQuaternionXyzw::admit([0.0, 0.0, 0.0, 1.0 + 1e-9]).is_err());

    let stress = CauchyStress6::new([6000.0, -6000.0, 0.0, 1200.0, -800.0, 450.0]);
    let q_arb = HamiltonQuaternionXyzw::admit([0.2, -0.4, 0.5, (1.0 - 0.45_f64).sqrt()]).unwrap();
    let rotated = q_arb.rotate_stress(&stress);
    let (i1_a, i2_a, i3_a) = stress.principal_invariants();
    let (i1_b, i2_b, i3_b) = rotated.principal_invariants();
    assert!((i1_a - i1_b).abs() <= 1e-9);
    assert!((i2_a - i2_b).abs() <= 1e-5);
    assert!((i3_a - i3_b).abs() <= 1e-2);
    assert!((stress.mechanical_pressure_pa() - rotated.mechanical_pressure_pa()).abs() <= 1e-10);
}

#[test]
fn c02_mode03_normal_incidence_impedance_oracle() {
    let iface = NormalIncidenceInterface::from_impedances(1.5e6, 3.0e6).unwrap();
    assert!((iface.pressure_reflection - 1.0 / 3.0).abs() <= 1e-15);
    assert!((iface.pressure_transmission - 4.0 / 3.0).abs() <= 1e-15);
    assert!(iface.pressure_transmission > 1.0);
    assert!((iface.energy_reflectance - 1.0 / 9.0).abs() <= 1e-15);
    assert!((iface.energy_transmittance - 8.0 / 9.0).abs() <= 1e-15);
    assert!((iface.energy_reflectance + iface.energy_transmittance - 1.0).abs() <= 1e-15);

    let rigid = NormalIncidenceInterface::rigid_limit();
    assert_eq!(rigid.pressure_reflection, 1.0);
    assert_eq!(rigid.energy_reflectance, 1.0);
    let soft = NormalIncidenceInterface::pressure_release_limit();
    assert_eq!(soft.pressure_reflection, -1.0);
    assert_eq!(soft.energy_reflectance, 1.0);
}

#[test]
fn helmholtz_3d_greens_function_eigenmode_and_bicgstab() {
    let k = Complex64::new(2.0 * PI, 0.05);
    assert!(helmholtz_greens_function_3d(0.0, k).is_err());
    let g = helmholtz_greens_function_3d(0.5, k).unwrap();
    let expected_re = -(-0.05 * 0.5_f64).exp() / (4.0 * PI * 0.5);
    assert!((g.re - expected_re).abs() <= 1e-15);
    assert!(g.im.abs() <= 1e-15);

    let f111 = rectangular_cavity_eigenfrequency_hz([1.0, 1.0, 1.0], [1, 1, 1], 343.0).unwrap();
    assert!((f111 - 0.5 * 343.0 * 3.0_f64.sqrt()).abs() <= 1e-12);
    let mode_center =
        rectangular_cavity_eigenmode_value([0.5, 0.5, 0.5], [1.0, 1.0, 1.0], [1, 1, 1], false);
    assert!((mode_center - 1.0).abs() <= 1e-15);

    for stencil in [
        HelmholtzStencil::SevenPoint,
        HelmholtzStencil::TwentySevenPointIsotropic,
    ] {
        let cfg = HelmholtzConfig {
            dims: [8, 8, 8],
            spacing_m: [0.1, 0.1, 0.1],
            omega_rad_s: 2.0 * PI * 200.0,
            phase_speed_m_s: 343.0,
            attenuation_np_m: 0.2,
            stencil,
            boundary: HelmholtzBoundary::DirichletZero,
        };
        let n = cfg.node_count();
        let mut rhs = vec![Complex64::ZERO; n];
        rhs[(4 * 8 + 4) * 8 + 4] = Complex64::new(-1.0, 0.0);
        let mut sol = vec![Complex64::ZERO; n];
        let mut ws = HelmholtzWorkspace::new(n);

        for _ in 0..4 {
            step_helmholtz_jacobi(&cfg, &rhs, &mut sol, &mut ws.r, 0.5).unwrap();
        }
        let report = solve_helmholtz_bicgstab(&cfg, &rhs, &mut sol, &mut ws, 200, 1e-10).unwrap();
        assert!(report.relative_residual <= 1e-10);

        let mut ax = vec![Complex64::ZERO; n];
        apply_helmholtz_operator(&cfg, &sol, &mut ax).unwrap();
        let max_err = ax
            .iter()
            .zip(rhs.iter())
            .map(|(a, b)| a.sub(*b).abs())
            .fold(0.0_f64, f64::max);
        assert!(max_err <= 1e-9);
    }
}

#[test]
fn room_acoustics_sabine_eyring_millington_and_schroeder_t60() {
    let room = RoomGeometry {
        volume_m3: 200.0,
        sound_speed_m_s: 343.0,
        air_attenuation_m_inv: 0.001,
        surfaces: vec![
            SurfacePatch {
                area_m2: 120.0,
                absorption_coeff: 0.25,
            },
            SurfacePatch {
                area_m2: 100.0,
                absorption_coeff: 0.40,
            },
        ],
    };
    let t_sabine = room.sabine_rt60_s().unwrap();
    let t_eyring = room.eyring_rt60_s().unwrap();
    let t_millington = room.millington_rt60_s().unwrap();
    assert!(t_sabine > t_eyring && t_eyring > t_millington && t_millington > 0.0);

    let anechoic_patch_room = RoomGeometry {
        volume_m3: 100.0,
        sound_speed_m_s: 343.0,
        air_attenuation_m_inv: 0.0,
        surfaces: vec![SurfacePatch {
            area_m2: 50.0,
            absorption_coeff: 1.0,
        }],
    };
    assert!(anechoic_patch_room.sabine_rt60_s().is_ok());
    assert!(anechoic_patch_room.eyring_rt60_s().is_err());
    assert!(anechoic_patch_room.millington_rt60_s().is_err());

    let fs = 4000.0;
    let target_t60 = 1.0;
    let d = 1000.0_f64.ln() / target_t60;
    let n_samples = 8000usize;
    let rir: Vec<f64> = (0..n_samples)
        .map(|n| (-d * (n as f64) / fs).exp())
        .collect();
    let report = analyze_room_impulse_response(&rir, fs).unwrap();
    assert!((report.edt_s - target_t60).abs() <= 2e-3);
    assert!((report.t20_s - target_t60).abs() <= 2e-3);
    assert!((report.t30_s - target_t60).abs() <= 5e-3);
    assert!(report.d50_ratio > 0.0 && report.d50_ratio < 1.0);
    assert!(report.centre_time_s > 0.0);

    assert!(schroeder_backward_integration_db(&[0.0; 64]).is_err());
}

#[test]
fn kramers_kronig_power_law_and_multi_relaxation_causality() {
    let w1 = 2.0 * PI * 1_000.0;
    let w2 = 2.0 * PI * 10_000.0;
    for &y in &[0.6_f64, 1.0_f64, 1.5_f64] {
        let model = PowerLawAttenuation::new(1e-7, y, w1, 1500.0).unwrap();
        let c1 = model.phase_speed_m_s(w1).unwrap();
        let c2 = model.phase_speed_m_s(w2).unwrap();
        assert!((c1 - 1500.0).abs() <= 1e-12);
        assert!(c2 > c1);
        assert!(model.attenuation_np_m(w2).unwrap() > model.attenuation_np_m(w1).unwrap());
    }

    for medium in [
        MultiRelaxationMedium::standard_atmosphere_20c_50rh(),
        MultiRelaxationMedium::standard_seawater_10c_35ppt(),
    ] {
        let w_low = 2.0 * PI * 500.0;
        let w_high = 2.0 * PI * 50_000.0;
        let exact_shift = medium.slowness_shift_s_m(w_high, w_low).unwrap();
        let quad_shift = medium
            .numerical_kk_slowness_shift_quadrature(w_high, w_low, 4096)
            .unwrap();
        let rel_err = (exact_shift - quad_shift).abs() / exact_shift.abs();
        assert!(rel_err <= 2e-3);
        assert!(medium.phase_speed_m_s(w_high).unwrap() > medium.phase_speed_m_s(w_low).unwrap());
    }
}
