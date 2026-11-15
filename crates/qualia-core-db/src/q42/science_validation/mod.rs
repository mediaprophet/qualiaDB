//! Independent physical invariant, passivity, conservation, uncertainty, and cross-modal
//! registration verifiers for the EMF v4 and QAC v1 scientific evolution programme
//! (Packet `N04` — `independent_science_reviewer`, Gates `G-SCIENCE` & `G-GPU`).
//!
//! All reference expectations in this module hierarchy are evaluated from closed-form
//! first-principles equations in independent `f64` and checked `i128` rational arithmetic
//! without delegating to candidate solver implementations.
//!
//! ## Submodules
//! - [`stokes_mueller`]: Stokes physical realizability (overflow-safe scaled 3D hypot, degree of
//!   polarization, non-clamping over-polarization rejection), Jones-to-Mueller exact equivalence,
//!   and $4\times 4$ Cloude Hermitian coherency matrix positive-semidefiniteness & passivity.
//! - [`passivity_conservation`]: C02-C09 numerical budget evaluators (`f64` closed-form,
//!   $N$-term Higham $\gamma_N$ reduction, QAC `B32` scalar & vector $L_2$), electromagnetic
//!   Poynting balance, 3D Cartesian Yee CFL & discrete dispersion, acoustic fluid/elastic/interface
//!   conservation, multi-port scattering matrix passivity ($\|\mathbf{S}\|_2 \le 1$), and
//!   grid-refinement convergence order ($p \ge 1.8$).
//! - [`uncertainty_ece`]: C02-C05 uncertainty descriptor validation, Gaussian coverage factor
//!   $k \leftrightarrow p$ consistency via positive-term Kummer $\operatorname{erf}(k/\sqrt{2})$,
//!   one-shot affine calibration idempotence, half-open calibration windows, $3\times 3$ SPD
//!   covariance rotation $\mathbf{R}\boldsymbol{\Sigma}\mathbf{R}^T$, Mahalanobis NEES, ECE,
//!   Brier score, and Gaussian NLL.
//! - [`specialized_oracles`]: Mode `0x0106` (EMF ionizing radiation Beer-Lambert, independent
//!   absorbed dose, $\text{keV}\to\text{J}$, XCOM dose-inference prohibition), Mode `0x0107`
//!   (EMF bioelectric dipole forward potential & singularity/reference guards), Mode `0x0106`
//!   (QAC haptic contact pressure, Hooke restoring force, role/expiry guards), Mode `0x0107`
//!   (QAC ultrasound round-trip ToF, derated Mechanical Index, monostatic Doppler, TI model guard),
//!   and C02-C08 I/Q carrier conjugation bridge.
//! - [`cross_modal_registration`]: Ordered `f64` quaternion norm admission, derived orthogonal
//!   Hamilton rotations, proper rotation determinant/orthogonality gates, exact `i128` rational
//!   clock synchronization, material `0` EMF/QAC disambiguation, and `.10d`/`.m64` anatomical AABB
//!   and surface-normal invariants.

pub mod cross_modal_registration;
pub mod passivity_conservation;
pub mod specialized_oracles;
pub mod stokes_mueller;
pub mod uncertainty_ece;

#[cfg(test)]
mod tests;

pub use cross_modal_registration::{
    compose_hamilton_quaternions_xyzw, evaluate_exact_sample_instant, ordered_quaternion_norm_sq,
    transform_local_to_parent, verify_anatomical_region_registration,
    verify_and_derive_quaternion_rotation, verify_cross_modal_clock_join,
    verify_material_and_anatomy_coupling, verify_stationary_proper_rotation_matrix,
    MaterialDomainFamily, RationalClockDescriptor, RationalSeconds, RegistrationValidationError,
    VerifiedQuaternionRotation, FRAME_ADMISSION_TOLERANCE,
};
pub use passivity_conservation::{
    evaluate_acoustic_interface_oracle, evaluate_em_mode0_instantaneous_oracle,
    evaluate_em_wavelength_m, evaluate_qac_elastic_mode01_oracle,
    evaluate_qac_fluid_mode00_oracle, gamma_n_f64, verify_acoustic_energy_conservation,
    verify_b32_scalar_budget, verify_b32_vector_budget, verify_f64_closed_form_budget,
    verify_grid_convergence_order, verify_n_term_reduction_budget,
    verify_poynting_conservation_balance, verify_scattering_matrix_passivity,
    verify_yee_cfl_and_dispersion, BudgetEvaluation, ConservationValidationError,
    EmMode0OracleResult, QacElasticMode01OracleResult, QacFluidMode00OracleResult,
    C02_AMPLITUDE_SCALES_B32, C02_AMPLITUDE_SCALES_F64, F32_UNIT_ROUNDOFF, F64_EPSILON,
    F64_UNIT_ROUNDOFF, VACUUM_IMPEDANCE_OHM, VACUUM_PERMEABILITY_H_PER_M,
    VACUUM_PERMITTIVITY_F_PER_M,
};
pub use specialized_oracles::{
    convert_and_verify_iq_carrier_bridge, convert_kev_to_joules, convert_mw_per_cm2_to_w_per_m2,
    convert_spectral_density_hz_to_wavelength_m, evaluate_absorbed_dose_gy,
    evaluate_bioelectric_dipole_potential_v, evaluate_equivalent_dose_sv,
    evaluate_haptic_mechanical_oracle, evaluate_ionizing_beer_lambert_oracle,
    evaluate_qac_db_spl_oracle, evaluate_qac_modal_oscillator_oracle,
    evaluate_ultrasound_mode07_oracle, reconstruct_emf_negative_carrier_rf,
    reconstruct_qac_positive_carrier_rf, HapticMechanicalOracleResult, HapticRole,
    IonizingRadiationOracleResult, SpecializedOracleError, UltrasoundOracleResult,
    ELEMENTARY_CHARGE_J_PER_EV,
};
pub use stokes_mueller::{
    apply_mueller_matrix, compute_cloude_coherency_matrix, deterministic_transverse_basis,
    hermitian_4x4_eigenvalues, jones_to_mueller_matrix, jones_vector_to_stokes,
    rotate_stokes_transverse_basis, scaled_hypot3, vacuum_or_homogeneous_propagation_delay_s,
    verify_jones_to_mueller_equivalence, verify_mueller_realizability, verify_stokes_realizability,
    Complex64, PolarimetricValidationError, VerifiedMuellerReport, VerifiedStokesState,
    C02_ROUNDING_FACTOR, F64_EPS, VACUUM_SPEED_OF_LIGHT_M_PER_S,
};
pub use uncertainty_ece::{
    apply_or_verify_affine_calibration, erf_f64, evaluate_ece_and_brier, evaluate_gaussian_nll,
    gaussian_coverage_probability, mahalanobis_nees_3d, rotate_covariance_3x3,
    verify_covariance_3x3_spd, verify_gaussian_coverage_factor,
    verify_half_open_calibration_window, verify_uncertainty_descriptor, CalibrationMetricsReport,
    CalibrationStage, UncertaintyDescriptor, UncertaintyKind, UncertaintyValidationError,
};

/// Execute a consolidated reference validation pass across all five `N04` submodules
/// (`stokes_mueller`, `passivity_conservation`, `uncertainty_ece`, `specialized_oracles`,
/// and `cross_modal_registration`).
pub fn run_all_science_validations() -> Result<(), String> {
    let stokes = jones_vector_to_stokes(Complex64::ONE, Complex64::I);
    verify_stokes_realizability(stokes).map_err(|e| format!("{e:?}"))?;
    let j_id = [[Complex64::ONE, Complex64::ZERO], [Complex64::ZERO, Complex64::ONE]];
    let m_id = jones_to_mueller_matrix(&j_id);
    verify_mueller_realizability(&m_id, true).map_err(|e| format!("{e:?}"))?;
    verify_jones_to_mueller_equivalence(&m_id, &j_id, 1.0).map_err(|e| format!("{e:?}"))?;
    evaluate_em_mode0_instantaneous_oracle(
        [1.0, 0.0, 0.0],
        [0.0, 1.0 / VACUUM_SPEED_OF_LIGHT_M_PER_S, 0.0],
        1.0,
        1.0,
    )
    .map_err(|e| format!("{e:?}"))?;
    verify_poynting_conservation_balance(-0.5, 0.2, 0.3, 1.0).map_err(|e| format!("{e:?}"))?;
    verify_yee_cfl_and_dispersion([1e-3; 3], 1e-12, 1.0, 1.0, [10.0, 0.0, 0.0])
        .map_err(|e| format!("{e:?}"))?;
    verify_gaussian_coverage_factor(2.0, 0.954_499_736_103_641_6, 1e-12)
        .map_err(|e| format!("{e:?}"))?;
    evaluate_ionizing_beer_lambert_oracle(1000.0, 0.01, 0.2, 100.0, false)
        .map_err(|e| format!("{e:?}"))?;
    evaluate_bioelectric_dipole_potential_v(
        [0.0, 0.0, 1e-8],
        [0.0; 3],
        Some([0.0, 0.0, 0.1]),
        true,
        1.0,
    )
    .map_err(|e| format!("{e:?}"))?;
    verify_and_derive_quaternion_rotation([0.0, 0.0, 0.0, 1.0]).map_err(|e| format!("{e:?}"))?;
    Ok(())
}
