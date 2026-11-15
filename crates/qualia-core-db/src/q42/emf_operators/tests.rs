//! Unit Tests for `q42::emf_operators` (C02 Oracles, Yee FDTD, Vector Calculus, ADE, and GPU Parity).

use super::dispersion_ade::{DebyePole, DrudePole, LorentzPole};
use super::gpu_parity::{
    c02_preview_f32_budget, c02_reduction_f64_budget, c02_small_op_f64_budget, lossy_flags,
    project_mode0_to_display_voxel64, verify_cpu_f64_vs_gpu_f32_yee_parity,
    EmfMode0InstantaneousRecord128,
};
use super::poynting_energy::{
    admit_stokes_vector, convert_iq_emf_to_qac_conjugated, dft_forward_unscaled,
    instantaneous_poynting_and_energy, jones_to_stokes_candidate_v1, mode0_carrier_wavelength_m,
    mode6_photon_attenuation_and_independent_dose, mode7_quasi_static_dipole_potential_v,
    mueller_apply, mueller_ideal_linear_polarizer_x, peak_phasor_time_average_poynting,
    ray_geometric_delay_s, reconstruct_rf_emf_negative_carrier,
    reconstruct_rf_qac_positive_carrier, rms_phasor_time_average_poynting,
    rotate_eb_hamilton_xyzw, rotate_stokes_basis_passive,
    spectral_density_wavelength_midpoint_integral, verify_discrete_poynting_power_balance,
};
use super::vector_calculus::{
    spectral_plane_wave_curl, verify_discrete_charge_conservation, verify_div_curl_zero_2nd,
    verify_stokes_circulation_cell, yee_curl_e_2nd, yee_curl_e_4th,
};
use super::yee_fdtd::{
    admit_yee_timestep, run_yee_periodic_plane_wave_fixture, yee_cfl_limit_3d,
    yee_step_leapfrog_in_place, YeeCheckpointState, YeeMaterialParams, YeeSolverProfile,
};
use super::{
    BoundaryCondition, EmfOperatorError, Grid3dSpec, C0_M_PER_S, EPS0_REF_F_PER_M, Z0_REF_OHM,
};

#[test]
fn test_mode0_instantaneous_poynting_energy_phasor_and_rotation() {
    let mat = YeeMaterialParams::vacuum();
    for &scale in &[1e-6_f64, 1.0_f64, 1e6_f64] {
        let e = [scale, 0.0, 0.0];
        let b = [0.0, scale / C0_M_PER_S, 0.0];
        let (s, u) = instantaneous_poynting_and_energy(e, b, &mat).expect("mode0 S and u");

        let q_s = (scale * scale) / Z0_REF_OHM;
        let q_u = EPS0_REF_F_PER_M * (scale * scale);
        assert!((s[0] - 0.0).abs() <= c02_small_op_f64_budget(q_s));
        assert!((s[1] - 0.0).abs() <= c02_small_op_f64_budget(q_s));
        assert!((s[2] - q_s).abs() <= c02_small_op_f64_budget(q_s));
        assert!((u - q_u).abs() <= c02_small_op_f64_budget(q_u));
    }

    // Peak phasor vs RMS phasor average Poynting equivalence
    let e_peak = [(1.0, 0.0), (0.0, 0.0), (0.0, 0.0)];
    let h_peak = [(0.0, 0.0), (1.0 / Z0_REF_OHM, 0.0), (0.0, 0.0)];
    let inv_sqrt2 = 1.0 / 2.0_f64.sqrt();
    let e_rms = [(inv_sqrt2, 0.0), (0.0, 0.0), (0.0, 0.0)];
    let h_rms = [(0.0, 0.0), (inv_sqrt2 / Z0_REF_OHM, 0.0), (0.0, 0.0)];
    let s_peak_avg = peak_phasor_time_average_poynting(e_peak, h_peak).unwrap();
    let s_rms_avg = rms_phasor_time_average_poynting(e_rms, h_rms).unwrap();
    let q_s = 1.0 / Z0_REF_OHM;
    assert!((s_peak_avg[2] - 0.5 * q_s).abs() <= c02_small_op_f64_budget(q_s));
    assert!((s_peak_avg[2] - s_rms_avg[2]).abs() <= c02_small_op_f64_budget(q_s));

    // Static carrier wavelength rejection
    assert_eq!(
        mode0_carrier_wavelength_m(0.0, false),
        Err(EmfOperatorError::StaticCarrierNoWavelength)
    );

    // +90 degree Z active Hamilton rotation: E=(1,0,0)->(0,1,0), B=(0,1/c,0)->(-1/c,0,0), S stays +z
    let s45 = core::f64::consts::FRAC_1_SQRT_2;
    let q_z90 = [0.0, 0.0, s45, s45];
    let (e_rot, b_rot, _) =
        rotate_eb_hamilton_xyzw(q_z90, [1.0, 0.0, 0.0], [0.0, 1.0 / C0_M_PER_S, 0.0]).unwrap();
    assert!((e_rot[0] - 0.0).abs() <= c02_small_op_f64_budget(1.0));
    assert!((e_rot[1] - 1.0).abs() <= c02_small_op_f64_budget(1.0));
    assert!((b_rot[0] + 1.0 / C0_M_PER_S).abs() <= c02_small_op_f64_budget(1.0 / C0_M_PER_S));
    assert!((b_rot[1] - 0.0).abs() <= c02_small_op_f64_budget(1.0 / C0_M_PER_S));
    let (s_rot, _) = instantaneous_poynting_and_energy(e_rot, b_rot, &mat).unwrap();
    assert!((s_rot[2] - 1.0 / Z0_REF_OHM).abs() <= c02_small_op_f64_budget(1.0 / Z0_REF_OHM));

    // C02-C04 perturbed identity quaternion q = (0, 0, 0, 1 + 2^-43)
    let q_pert = [0.0, 0.0, 0.0, 1.0 + 2.0_f64.powi(-43)];
    let (_, _, r_id) = rotate_eb_hamilton_xyzw(q_pert, [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]).unwrap();
    assert!((r_id[0][0] - 1.0).abs() <= c02_small_op_f64_budget(1.0));
    assert!((r_id[1][1] - 1.0).abs() <= c02_small_op_f64_budget(1.0));
    assert!((r_id[2][2] - 1.0).abs() <= c02_small_op_f64_budget(1.0));
}

#[test]
fn test_mode1_polarimetric_ray_stokes_basis_rotation_and_delay() {
    let inv_sqrt2 = core::f64::consts::FRAC_1_SQRT_2;
    let stokes_circ = jones_to_stokes_candidate_v1((inv_sqrt2, 0.0), (0.0, inv_sqrt2)).unwrap();
    let b1 = c02_small_op_f64_budget(1.0);
    assert!((stokes_circ[0] - 1.0).abs() <= b1);
    assert!((stokes_circ[1] - 0.0).abs() <= b1);
    assert!((stokes_circ[2] - 0.0).abs() <= b1);
    assert!((stokes_circ[3] - (-1.0)).abs() <= b1);

    let stokes_rot =
        rotate_stokes_basis_passive([1.0, 1.0, 0.0, 0.0], core::f64::consts::FRAC_PI_4).unwrap();
    assert!((stokes_rot[0] - 1.0).abs() <= b1);
    assert!((stokes_rot[1] - 0.0).abs() <= b1);
    assert!((stokes_rot[2] - (-1.0)).abs() <= b1);
    assert!((stokes_rot[3] - 0.0).abs() <= b1);

    let delay_s = ray_geometric_delay_s(0.299_792_458, &YeeMaterialParams::vacuum()).unwrap();
    assert!((delay_s - 1e-9).abs() <= c02_small_op_f64_budget(1e-9));

    assert!(admit_stokes_vector([0.0, 0.0, 0.0, 0.0]).is_ok());
    assert!(matches!(
        admit_stokes_vector([1.0, 1.001, 0.0, 0.0]),
        Err(EmfOperatorError::InvalidPolarimetricState { .. })
    ));
}

#[test]
fn test_mode2_yee_cfl_gate_dispersion_convergence_and_bitwise_restart() {
    let mat = YeeMaterialParams::vacuum();
    let grid32 = Grid3dSpec {
        nx: 32,
        ny: 32,
        nz: 32,
        dx: 1.0 / 32.0,
        dy: 1.0 / 32.0,
        dz: 1.0 / 32.0,
    };
    let cfl = yee_cfl_limit_3d(&grid32, &mat).unwrap();
    assert!( matches!(
        admit_yee_timestep(&grid32, &mat, 1.01 * cfl),
        Err(EmfOperatorError::CflViolation { .. })
    ));

    let r32 = run_yee_periodic_plane_wave_fixture(32, 1.0, 0.5, &mat).unwrap();
    let r64 = run_yee_periodic_plane_wave_fixture(64, 1.0, 0.5, &mat).unwrap();
    let r128 = run_yee_periodic_plane_wave_fixture(128, 1.0, 0.5, &mat).unwrap();

    // One-step discrete eigenmode frequency matches analytical discrete dispersion within 1e-10
    assert!(r32.one_step_rel_error <= 1e-10, "r32={:?}", r32);
    assert!(r64.one_step_rel_error <= 1e-10, "r64={:?}", r64);
    assert!(r128.one_step_rel_error <= 1e-10, "r128={:?}", r128);

    // Continuum phase convergence order >= 1.8 across N=32->64 and N=64->128
    let order_32_64 = (r32.continuum_phase_error_rad / r64.continuum_phase_error_rad).log2();
    let order_64_128 = (r64.continuum_phase_error_rad / r128.continuum_phase_error_rad).log2();
    assert!(order_32_64 >= 1.8, "order_32_64={order_32_64}");
    assert!(order_64_128 >= 1.8, "order_64_128={order_64_128}");
    assert!(
        r128.continuum_phase_error_rad <= 0.002,
        "fine phase err={}",
        r128.continuum_phase_error_rad
    );
    assert!(
        r128.relative_energy_oscillation <= 0.005,
        "energy osc={}",
        r128.relative_energy_oscillation
    );

    // Deterministic bitwise-identical checkpoint & restart verification
    let small_grid = Grid3dSpec {
        nx: 16,
        ny: 2,
        nz: 2,
        dx: 1.0 / 16.0,
        dy: 1.0 / 16.0,
        dz: 1.0 / 16.0,
    };
    let dt_small = 0.5 * yee_cfl_limit_3d(&small_grid, &mat).unwrap();
    let n_cells = small_grid.validate().unwrap();
    let mut state_uninterrupted = YeeCheckpointState {
        profile: YeeSolverProfile::LosslessPeriodicNoPhiV1,
        boundary: BoundaryCondition::Periodic,
        grid: small_grid,
        material: mat,
        dt: dt_small,
        step_index: 0,
        has_complete_boundary_state: true,
        has_complete_auxiliary_state: true,
        phi_unavailable_declared: true,
        ex: vec![0.0; n_cells],
        ey: (0..n_cells)
            .map(|i| ((i as f64) * 0.25).cos())
            .collect(),
        ez: vec![0.0; n_cells],
        hx: vec![0.0; n_cells],
        hy: vec![0.0; n_cells],
        hz: (0..n_cells)
            .map(|i| ((i as f64) * 0.25).sin() / Z0_REF_OHM)
            .collect(),
        jx: vec![0.0; n_cells],
        jy: vec![0.0; n_cells],
        jz: vec![0.0; n_cells],
        state_blake3: [0u8; 32],
    };
    state_uninterrupted.seal();
    let mut state_restarted = state_uninterrupted.clone();

    state_uninterrupted.advance_steps(10).unwrap();
    state_restarted.advance_steps(5).unwrap();
    let saved_checkpoint = state_restarted.clone();
    let mut restored_checkpoint = saved_checkpoint;
    restored_checkpoint.admit_for_restart().unwrap();
    restored_checkpoint.advance_steps(5).unwrap();
    assert!(state_uninterrupted.is_bitwise_identical_to(&restored_checkpoint));

    // Incomplete restart state must reject
    let mut bad_restart = restored_checkpoint.clone();
    bad_restart.has_complete_boundary_state = false;
    assert!(matches!(
        bad_restart.admit_for_restart(),
        Err(EmfOperatorError::IncompleteRestartState { .. })
    ));
}

#[test]
fn test_vector_calculus_identities_gauss_charge_conservation_and_4th_order() {
    let grid = Grid3dSpec {
        nx: 16,
        ny: 16,
        nz: 16,
        dx: 1.0 / 16.0,
        dy: 1.0 / 16.0,
        dz: 1.0 / 16.0,
    };
    let n = grid.validate().unwrap();
    let mut ex = vec![0.0_f64; n];
    let mut ey = vec![0.0_f64; n];
    let mut ez = vec![0.0_f64; n];
    let mut hx = vec![0.0_f64; n];
    let mut hy = vec![0.0_f64; n];
    let mut hz = vec![0.0_f64; n];

    let two_pi = 2.0 * core::f64::consts::PI;
    for iz in 0..grid.nz {
        for iy in 0..grid.ny {
            for ix in 0..grid.nx {
                let c = grid.idx(ix, iy, iz);
                let x = (ix as f64) * grid.dx;
                let y = (iy as f64) * grid.dy;
                let z = (iz as f64) * grid.dz;
                ex[c] = (two_pi * x).sin() * (two_pi * y).cos();
                ey[c] = (two_pi * y).sin() * (two_pi * z).cos();
                ez[c] = (two_pi * z).sin() * (two_pi * x).cos();
                hx[c] = (two_pi * (x + y)).cos();
                hy[c] = (two_pi * (y + z)).sin();
                hz[c] = (two_pi * (z + x)).cos();
            }
        }
    }

    let mut cx = vec![0.0_f64; n];
    let mut cy = vec![0.0_f64; n];
    let mut cz = vec![0.0_f64; n];
    let mut div_scratch = vec![0.0_f64; n];

    let (max_div_curl_e, max_div_curl_h) = verify_div_curl_zero_2nd(
        &grid,
        BoundaryCondition::Periodic,
        &ex,
        &ey,
        &ez,
        &hx,
        &hy,
        &hz,
        &mut cx,
        &mut cy,
        &mut cz,
        &mut div_scratch,
    )
    .unwrap();
    assert!(max_div_curl_e <= 1e-12, "max_div_curl_e={max_div_curl_e}");
    assert!(max_div_curl_h <= 1e-12, "max_div_curl_h={max_div_curl_h}");

    // Stokes theorem cell circulation vs surface flux
    yee_curl_e_2nd(
        &grid,
        BoundaryCondition::Periodic,
        &ex,
        &ey,
        &ez,
        &mut cx,
        &mut cy,
        &mut cz,
    )
    .unwrap();
    let stokes_res = verify_stokes_circulation_cell(
        &grid,
        BoundaryCondition::Periodic,
        &ex,
        &ey,
        &ez,
        &cx,
        &cy,
        &cz,
        3,
        5,
        7,
    )
    .unwrap();
    assert!(stokes_res.iter().all(|&r| r <= 1e-15));

    // Discrete charge conservation and Poynting power balance across a driven Yee step
    let mat = YeeMaterialParams::vacuum();
    let dt = 0.5 * yee_cfl_limit_3d(&grid, &mat).unwrap();
    let eps = mat.epsilon_f_per_m();
    let ex_n = ex.clone();
    let ey_n = ey.clone();
    let ez_n = ez.clone();
    let hx_half = hx.clone();
    let hy_half = hy.clone();
    let hz_half = hz.clone();
    let jx_half: Vec<f64> = (0..n).map(|i| 1e-4 * ((i as f64) * 0.1).sin()).collect();
    let jy_half: Vec<f64> = (0..n).map(|i| 1e-4 * ((i as f64) * 0.2).cos()).collect();
    let jz_half: Vec<f64> = (0..n).map(|i| -1e-4 * ((i as f64) * 0.15).sin()).collect();

    yee_step_leapfrog_in_place(
        &grid,
        YeeSolverProfile::LosslessPeriodicNoPhiV1,
        BoundaryCondition::Periodic,
        &mat,
        dt,
        &mut ex,
        &mut ey,
        &mut ez,
        &mut hx,
        &mut hy,
        &mut hz,
        &jx_half,
        &jy_half,
        &jz_half,
        &mut cx,
        &mut cy,
        &mut cz,
    )
    .unwrap();

    let dx_n: Vec<f64> = ex_n.iter().map(|&v| eps * v).collect();
    let dy_n: Vec<f64> = ey_n.iter().map(|&v| eps * v).collect();
    let dz_n: Vec<f64> = ez_n.iter().map(|&v| eps * v).collect();
    let dx_np1: Vec<f64> = ex.iter().map(|&v| eps * v).collect();
    let dy_np1: Vec<f64> = ey.iter().map(|&v| eps * v).collect();
    let dz_np1: Vec<f64> = ez.iter().map(|&v| eps * v).collect();
    let mut s1 = vec![0.0_f64; n];
    let mut s2 = vec![0.0_f64; n];
    let mut s3 = vec![0.0_f64; n];

    let charge_res = verify_discrete_charge_conservation(
        &grid,
        BoundaryCondition::Periodic,
        dt,
        &dx_n,
        &dy_n,
        &dz_n,
        &dx_np1,
        &dy_np1,
        &dz_np1,
        &jx_half,
        &jy_half,
        &jz_half,
        &mut s1,
        &mut s2,
        &mut s3,
    )
    .unwrap();
    assert!(charge_res <= 1e-13, "charge_res={charge_res}");

    let poynting_res = verify_discrete_poynting_power_balance(
        &grid,
        BoundaryCondition::Periodic,
        &mat,
        dt,
        &ex_n,
        &ey_n,
        &ez_n,
        &ex,
        &ey,
        &ez,
        &hx_half,
        &hy_half,
        &hz_half,
        &jx_half,
        &jy_half,
        &jz_half,
        &mut cx,
        &mut cy,
        &mut cz,
    )
    .unwrap();
    assert!(poynting_res <= 1e-13, "poynting_res={poynting_res}");

    // 4th-order vs 2nd-order curl accuracy against spectral plane-wave oracle
    for iz in 0..grid.nz {
        for iy in 0..grid.ny {
            for ix in 0..grid.nx {
                let c = grid.idx(ix, iy, iz);
                let x = (ix as f64) * grid.dx;
                ex[c] = 0.0;
                ey[c] = (two_pi * x).cos();
                ez[c] = 0.0;
            }
        }
    }
    let mut cz_2nd = vec![0.0_f64; n];
    let mut cz_4th = vec![0.0_f64; n];
    yee_curl_e_2nd(
        &grid,
        BoundaryCondition::Periodic,
        &ex,
        &ey,
        &ez,
        &mut cx,
        &mut cy,
        &mut cz_2nd,
    )
    .unwrap();
    yee_curl_e_4th(
        &grid,
        BoundaryCondition::Periodic,
        &ex,
        &ey,
        &ez,
        &mut cx,
        &mut cy,
        &mut cz_4th,
    )
    .unwrap();

    let mut err_2nd = 0.0_f64;
    let mut err_4th = 0.0_f64;
    for ix in 0..grid.nx {
        let x_face = ((ix as f64) + 0.5) * grid.dx;
        let exact = spectral_plane_wave_curl([two_pi, 0.0, 0.0], [0.0, 1.0, 0.0], [x_face, 0.0, 0.0], 0.0)[2];
        let c = grid.idx(ix, 0, 0);
        err_2nd = err_2nd.max((cz_2nd[c] - exact).abs());
        err_4th = err_4th.max((cz_4th[c] - exact).abs());
    }
    assert!(err_4th < 0.05 * err_2nd, "err_4th={err_4th}, err_2nd={err_2nd}");
}

#[test]
fn test_dispersion_ade_debye_drude_lorentz_transfer_and_energy() {
    let debye = DebyePole {
        delta_epsilon: 2.0,
        tau_s: 1e-10,
    };
    let omega = 1e10_f64;
    let (re_c, im_c) = debye.continuous_susceptibility(omega).unwrap();
    let (re_d1, im_d1) = debye.discrete_susceptibility(omega, 1e-12).unwrap();
    let (re_d2, im_d2) = debye.discrete_susceptibility(omega, 5e-13).unwrap();
    let err1 = (re_d1 - re_c).hypot(im_d1 - im_c);
    let err2 = (re_d2 - re_c).hypot(im_d2 - im_c);
    assert!(err2 < 0.26 * err1);

    let mut px = [0.0_f64];
    let mut py = [0.0_f64];
    let mut pz = [0.0_f64];
    let mut jpx = [0.0_f64];
    let mut jpy = [0.0_f64];
    let mut jpz = [0.0_f64];
    debye
        .step_ade_in_place(
            1e-11,
            &[0.0],
            &[0.0],
            &[0.0],
            &[100.0],
            &[0.0],
            &[0.0],
            &mut px,
            &mut py,
            &mut pz,
            &mut jpx,
            &mut jpy,
            &mut jpz,
        )
        .unwrap();
    assert!(px[0] > 0.0 && jpx[0] > 0.0);
    assert!(debye.stored_energy_density([px[0], 0.0, 0.0]).unwrap() > 0.0);

    let drude = DrudePole {
        omega_p_rad_s: 1e15,
        gamma_rad_s: 1e13,
    };
    let (dr_re, dr_im) = drude.continuous_susceptibility(2e15).unwrap();
    assert!(dr_re < 0.0 && dr_im > 0.0);

    let lorentz = LorentzPole {
        delta_epsilon: 1.5,
        omega_0_rad_s: 1e14,
        gamma_rad_s: 1e12,
    };
    let (l_c_re, l_c_im) = lorentz.continuous_susceptibility(5e13).unwrap();
    let (l_d_re, l_d_im) = lorentz.discrete_susceptibility(5e13, 1e-16).unwrap();
    assert!((l_c_re - l_d_re).hypot(l_c_im - l_d_im) < 1e-4);
}

#[test]
fn test_mode3_4_5_6_7_and_gpu_parity_oracles() {
    // Mode 3: Spectral Jacobian integral & midpoint convergence
    let (analytic, mid64) =
        spectral_density_wavelength_midpoint_integral(1e12, 2e12, 1e-12, 64).unwrap();
    let (_, mid128) =
        spectral_density_wavelength_midpoint_integral(1e12, 2e12, 1e-12, 128).unwrap();
    let (_, mid256) =
        spectral_density_wavelength_midpoint_integral(1e12, 2e12, 1e-12, 256).unwrap();
    assert!((analytic - 1.0).abs() <= c02_small_op_f64_budget(1.0));
    let e64 = (mid64 - 1.0).abs();
    let e128 = (mid128 - 1.0).abs();
    let e256 = (mid256 - 1.0).abs();
    assert!((3.5..=4.5).contains(&(e64 / e128)));
    assert!((3.5..=4.5).contains(&(e128 / e256)));
    assert!(e256 <= 1e-5);

    // Mode 4: Ideal horizontal Mueller polarizer
    let m_pol = mueller_ideal_linear_polarizer_x();
    assert_eq!(mueller_apply(&m_pol, [2.0, 0.0, 0.0, 0.0]).unwrap(), [1.0, 1.0, 0.0, 0.0]);
    assert_eq!(mueller_apply(&m_pol, [1.0, -1.0, 0.0, 0.0]).unwrap(), [0.0, 0.0, 0.0, 0.0]);
    assert_eq!(mueller_apply(&m_pol, [1.0, 1.0, 0.0, 0.0]).unwrap(), [1.0, 1.0, 0.0, 0.0]);

    // Mode 5: N=8 complex DFT and EMF/QAC carrier conjugation
    let mut z = [(0.0_f64, 0.0_f64); 8];
    for (n, item) in z.iter_mut().enumerate() {
        let ang = 2.0 * core::f64::consts::PI * (n as f64) / 8.0;
        *item = (ang.cos(), ang.sin());
    }
    let mut bins = [(0.0_f64, 0.0_f64); 8];
    dft_forward_unscaled(&z, None, &mut bins).unwrap();
    let dft_budget = c02_reduction_f64_budget(8, 8.0);
    for (k, &(re, im)) in bins.iter().enumerate() {
        let exp_re = if k == 1 { 8.0 } else { 0.0 };
        let err = (re - exp_re).hypot(im);
        assert!(err <= dft_budget, "bin {k} err={err} > {dft_budget}");
    }
    let iq_emf = (0.6_f64, -0.8_f64);
    let iq_qac = convert_iq_emf_to_qac_conjugated(iq_emf);
    let rf_emf = reconstruct_rf_emf_negative_carrier(iq_emf, 10_000.0, 2.5e-5, 0.3);
    let rf_qac = reconstruct_rf_qac_positive_carrier(iq_qac, 10_000.0, 2.5e-5, 0.3);
    assert!((rf_emf - rf_qac).abs() <= c02_small_op_f64_budget(1.0));

    // Mode 6: Beer-Lambert photon attenuation & independent dose
    let (mu, flu_out, dose_gy, e_j) = mode6_photon_attenuation_and_independent_dose(
        1000.0, 0.01, 0.2, 100.0, 0.002, 0.01, 1e5,
    )
    .unwrap();
    assert!((mu - 10.0).abs() <= c02_small_op_f64_budget(10.0));
    assert!((flu_out - 13.533_528_323_661_27).abs() <= c02_small_op_f64_budget(100.0));
    assert!((dose_gy - 0.2).abs() <= c02_small_op_f64_budget(0.2));
    assert!((e_j - 1.602_176_634e-14).abs() <= c02_small_op_f64_budget(1.602_176_634e-14));

    // Mode 7: Quasi-static bioelectric dipole potential
    let q_phi = 7.957_747_154_594_767e-8;
    let phi_ax =
        mode7_quasi_static_dipole_potential_v([0.0, 0.0, 1e-8], [0.0; 3], [0.0, 0.0, 0.1], 1.0)
            .unwrap();
    let phi_eq =
        mode7_quasi_static_dipole_potential_v([0.0, 0.0, 1e-8], [0.0; 3], [0.1, 0.0, 0.0], 1.0)
            .unwrap();
    let phi_rev =
        mode7_quasi_static_dipole_potential_v([0.0, 0.0, 1e-8], [0.0; 3], [0.0, 0.0, -0.1], 1.0)
            .unwrap();
    assert!((phi_ax - q_phi).abs() <= c02_small_op_f64_budget(q_phi));
    assert!((phi_eq - 0.0).abs() <= c02_small_op_f64_budget(q_phi));
    assert!((phi_rev + q_phi).abs() <= c02_small_op_f64_budget(q_phi));
    assert_eq!(
        mode7_quasi_static_dipole_potential_v([0.0, 0.0, 1e-8], [0.0; 3], [0.0; 3], 1.0),
        Err(EmfOperatorError::SingularDipoleSourceEvaluation)
    );

    // GPU/CPU display projection & f32 reference kernel parity
    let rec0 = EmfMode0InstantaneousRecord128 {
        electric_field_v_per_m: [1.0, 0.0, 0.0],
        magnetic_field_tesla: [0.0, 1.0 / C0_M_PER_S, 0.0],
        local_position_m: [10.0, 20.0, 30.0],
        record_tick: 42,
        carrier_hz: 5e14,
        coherence_phase_rad: 0.5,
        stored_poynting_w_per_m2: [0.0, 0.0, 1.0 / Z0_REF_OHM],
        flags: 0b1011,
        confidence: 1.0,
    };
    let proj = project_mode0_to_display_voxel64(&rec0, &YeeMaterialParams::vacuum()).unwrap();
    assert_ne!(
        proj.lossy_flags & lossy_flags::DISPLAY_SIGMA_DERIVED_NON_AUTHORITATIVE,
        0
    );
    assert!(proj.max_s_abs_error <= c02_preview_f32_budget(1.0 / Z0_REF_OHM));

    let parity = verify_cpu_f64_vs_gpu_f32_yee_parity(32, 4).unwrap();
    assert!(parity.max_e_abs_diff <= parity.e_budget);
    assert!(parity.max_h_abs_diff <= parity.h_budget);
}
