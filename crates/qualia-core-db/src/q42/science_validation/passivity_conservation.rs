//! Passivity, energy conservation, and C02-C09 numerical budget verifiers.
//!
//! Implements independent first-principles `f64` evaluators for:
//! - C02-C09 `f64` small closed-form budget: $|y - y_{\text{ref}}| \le 64 \cdot 2^{-52} Q$.
//! - C02-C09 $N$-term reduction / DFT budget: $16\,\gamma_N \sum_{i=1}^N |t_i|$ with $\gamma_N = \frac{N u}{1 - N u}, u = 2^{-53}$.
//! - C02-C09 QAC `B32` scalar budget: $|y - y_{\text{ref}}| \le 8\,u_{32} S + 2\times 10^{-6} |y_{\text{ref}}|, u_{32} = 2^{-24}$,
//!   and vector relative $L_2 \le 5\times 10^{-6}$.
//! - Electromagnetic Mode 0 (Poynting flux, energy density, static wavelength guard) and Mode 2
//!   (3D Cartesian Yee CFL bound and exact discrete dispersion relation).
//! - Acoustic Mode 00 (fluid intensity/energy), Mode 01 (6-component Cauchy elasticity), and
//!   Mode 03 (impedance reflection/transmission energy conservation $R + T = 1$).
//! - Multi-port scattering matrix passivity ($\|\mathbf{S}\|_2 \le 1$) and grid-refinement
//!   observed convergence order $p = \frac{\ln(E_1/E_2)}{\ln(h_1/h_2)} \ge 1.8$.

use crate::q42::science_validation::stokes_mueller::{
    hermitian_4x4_eigenvalues, scaled_hypot3, Complex64, VACUUM_SPEED_OF_LIGHT_M_PER_S,
};

/// IEEE-754 `f64` unit roundoff $u = 2^{-53}$ (`0.5 * f64::EPSILON`).
pub const F64_UNIT_ROUNDOFF: f64 = 0.5 * f64::EPSILON;

/// IEEE-754 `f64` machine epsilon $\varepsilon = 2^{-52}$.
pub const F64_EPSILON: f64 = f64::EPSILON;

/// IEEE-754 `f32` unit roundoff $u_{32} = 2^{-24}$ (`0.5 * f32::EPSILON as f64`).
pub const F32_UNIT_ROUNDOFF: f64 = 0.5 * (f32::EPSILON as f64);

/// Exact SI magnetic constant (vacuum permeability) $\mu_0 = 4\pi \times 10^{-7}\text{ H/m}$
/// consistent with $c_0 = 299\,792\,458\text{ m/s}$ and $Z_0 = \mu_0 c_0$.
pub const VACUUM_PERMEABILITY_H_PER_M: f64 = 4.0e-7 * core::f64::consts::PI;

/// Characteristic impedance of vacuum $Z_0 = \mu_0 c_0$ ($\Omega$).
pub const VACUUM_IMPEDANCE_OHM: f64 =
    VACUUM_PERMEABILITY_H_PER_M * VACUUM_SPEED_OF_LIGHT_M_PER_S;

/// Vacuum electric permittivity $\varepsilon_0 = \frac{1}{\mu_0 c_0^2}$ ($\text{F/m}$).
pub const VACUUM_PERMITTIVITY_F_PER_M: f64 = 1.0
    / (VACUUM_PERMEABILITY_H_PER_M
        * VACUUM_SPEED_OF_LIGHT_M_PER_S
        * VACUUM_SPEED_OF_LIGHT_M_PER_S);

/// Standard C02 amplitude scaling factors for linear-domain invariance checks.
pub const C02_AMPLITUDE_SCALES_F64: [f64; 3] = [1.0e-6, 1.0, 1.0e6];

/// Standard C02 QAC `B32` amplitude scaling factors.
pub const C02_AMPLITUDE_SCALES_B32: [f64; 3] = [1.0e-3, 1.0, 1.0e3];

/// Detailed audit record for a numerical budget comparison.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BudgetEvaluation {
    pub actual: f64,
    pub reference: f64,
    pub scale: f64,
    pub abs_error: f64,
    pub rel_error: f64,
    pub budget: f64,
}

/// Errors emitted by passivity, conservation, and numerical budget verifiers.
#[derive(Debug, Clone, PartialEq)]
pub enum ConservationValidationError {
    /// Non-finite input or reference value encountered.
    NonFiniteValue { context: &'static str, value: f64 },
    /// Characteristic scale $Q$ or $S$ is non-positive or non-finite.
    InvalidCharacteristicScale { scale: f64 },
    /// Reduction term count $N$ is zero or $N u \ge 1$.
    InvalidReductionCount { n: usize },
    /// Scalar error exceeded the C02 numerical budget.
    BudgetExceeded(BudgetEvaluation),
    /// Vector relative $L_2$ error exceeded the QAC `B32` threshold ($5\times 10^{-6}$).
    VectorRelativeL2Exceeded {
        rel_l2_error: f64,
        max_allowed: f64,
        l2_diff: f64,
        l2_ref: f64,
    },
    /// Static/aperiodic field ($f_c \le 0$) cannot be evaluated for wavelength $c/f$.
    StaticFieldWavelengthForbidden { carrier_hz: f64 },
    /// Invalid constitutive parameters ($\varepsilon_r \le 0$, $\mu_r \le 0$, or $\sigma < 0$).
    InvalidConstitutiveParameters {
        epsilon_r: f64,
        mu_r: f64,
        sigma_s_per_m: f64,
    },
    /// Invalid fluid or elastic medium parameters ($\rho \le 0$, $K \le 0$, or $\mu < 0$).
    InvalidMediumParameters {
        density_kg_per_m3: f64,
        bulk_modulus_pa: f64,
        shear_modulus_pa: f64,
    },
    /// Multiplying non-collocated pressure and velocity without explicit interpolation is forbidden.
    NonCollocatedPressureVelocityRejected,
    /// Conservation law residual $\nabla\cdot\mathbf{F} + \partial u/\partial t + w_{\text{loss}}$ exceeded budget,
    /// or medium exhibited active energy generation ($w_{\text{loss}} < 0$).
    PassivityOrConservationViolation {
        residual: f64,
        dissipated_power_w_per_m3: f64,
        budget: f64,
    },
    /// Yee or leapfrog timestep violated the 3D Courant-Friedrichs-Lewy (CFL) stability bound.
    CflStabilityViolation {
        dt_s: f64,
        cfl_max_dt_s: f64,
        courant_ratio: f64,
    },
    /// Scattering matrix violated passivity ($\|\mathbf{S}\|_2 > 1 + 64\,\varepsilon$).
    ScatteringPassivityViolation {
        max_singular_value_sq: f64,
        tolerance_sq: f64,
    },
    /// Grid refinement failed to achieve the minimum observed convergence order $p \ge 1.8$.
    ConvergenceOrderBelowMinimum {
        observed_order: f64,
        minimum_order: f64,
        coarse_error: f64,
        fine_error: f64,
    },
}

/// Evaluate the C02-C09 small closed-form `f64` budget:
/// $|y - y_{\text{ref}}| \le 64 \cdot 2^{-52} Q$.
pub fn verify_f64_closed_form_budget(
    actual: f64,
    reference: f64,
    scale_q: f64,
) -> Result<BudgetEvaluation, ConservationValidationError> {
    if !actual.is_finite() {
        return Err(ConservationValidationError::NonFiniteValue {
            context: "actual",
            value: actual,
        });
    }
    if !reference.is_finite() {
        return Err(ConservationValidationError::NonFiniteValue {
            context: "reference",
            value: reference,
        });
    }
    if !scale_q.is_finite() || scale_q <= 0.0 {
        return Err(ConservationValidationError::InvalidCharacteristicScale { scale: scale_q });
    }
    let abs_error = (actual - reference).abs();
    let rel_error = if reference == 0.0 {
        abs_error / scale_q
    } else {
        abs_error / reference.abs()
    };
    let budget = 64.0 * F64_EPSILON * scale_q;
    let eval = BudgetEvaluation {
        actual,
        reference,
        scale: scale_q,
        abs_error,
        rel_error,
        budget,
    };
    if abs_error > budget {
        return Err(ConservationValidationError::BudgetExceeded(eval));
    }
    Ok(eval)
}

/// Compute Higham's rounding constant $\gamma_N = \frac{N u}{1 - N u}$ with $u = 2^{-53}$.
pub fn gamma_n_f64(n: usize) -> Result<f64, ConservationValidationError> {
    if n == 0 {
        return Err(ConservationValidationError::InvalidReductionCount { n });
    }
    let nu = (n as f64) * F64_UNIT_ROUNDOFF;
    if !nu.is_finite() || nu >= 1.0 {
        return Err(ConservationValidationError::InvalidReductionCount { n });
    }
    Ok(nu / (1.0 - nu))
}

/// Evaluate the C02-C09 $N$-term reduction / DFT budget:
/// $|y - y_{\text{ref}}| \le 16\,\gamma_N \sum_{i=1}^N |t_i|$.
pub fn verify_n_term_reduction_budget(
    actual: f64,
    reference: f64,
    n_terms: usize,
    sum_abs_terms: f64,
) -> Result<BudgetEvaluation, ConservationValidationError> {
    if !actual.is_finite() || !reference.is_finite() {
        return Err(ConservationValidationError::NonFiniteValue {
            context: "reduction_value",
            value: if !actual.is_finite() { actual } else { reference },
        });
    }
    if !sum_abs_terms.is_finite() || sum_abs_terms <= 0.0 {
        return Err(ConservationValidationError::InvalidCharacteristicScale {
            scale: sum_abs_terms,
        });
    }
    let gamma = gamma_n_f64(n_terms)?;
    let budget = 16.0 * gamma * sum_abs_terms;
    let abs_error = (actual - reference).abs();
    let rel_error = if reference == 0.0 {
        abs_error / sum_abs_terms
    } else {
        abs_error / reference.abs()
    };
    let eval = BudgetEvaluation {
        actual,
        reference,
        scale: sum_abs_terms,
        abs_error,
        rel_error,
        budget,
    };
    if abs_error > budget {
        return Err(ConservationValidationError::BudgetExceeded(eval));
    }
    Ok(eval)
}

/// Evaluate the C02-C09 QAC `B32` scalar budget:
/// $|y - y_{\text{ref}}| \le 8\,u_{32} S + 2\times 10^{-6} |y_{\text{ref}}|, \quad u_{32} = 2^{-24}$.
pub fn verify_b32_scalar_budget(
    actual: f64,
    reference: f64,
    scale_s: f64,
) -> Result<BudgetEvaluation, ConservationValidationError> {
    if !actual.is_finite() || !reference.is_finite() {
        return Err(ConservationValidationError::NonFiniteValue {
            context: "b32_scalar",
            value: if !actual.is_finite() { actual } else { reference },
        });
    }
    if !scale_s.is_finite() || scale_s <= 0.0 {
        return Err(ConservationValidationError::InvalidCharacteristicScale { scale: scale_s });
    }
    let budget = 8.0 * F32_UNIT_ROUNDOFF * scale_s + 2.0e-6 * reference.abs();
    let abs_error = (actual - reference).abs();
    let rel_error = if reference == 0.0 {
        abs_error / scale_s
    } else {
        abs_error / reference.abs()
    };
    let eval = BudgetEvaluation {
        actual,
        reference,
        scale: scale_s,
        abs_error,
        rel_error,
        budget,
    };
    if abs_error > budget {
        return Err(ConservationValidationError::BudgetExceeded(eval));
    }
    Ok(eval)
}

/// Evaluate the C02-C09 QAC `B32` vector budget:
/// per-component `B32` scalar check plus relative $L_2 \le 5\times 10^{-6}$ when $\|\mathbf{y}_{\text{ref}}\|_2 > 0$.
pub fn verify_b32_vector_budget(
    actual: &[f64],
    reference: &[f64],
    scale_s: f64,
) -> Result<f64, ConservationValidationError> {
    if actual.len() != reference.len() || actual.is_empty() {
        return Err(ConservationValidationError::InvalidReductionCount { n: actual.len() });
    }
    let mut diff_sq = 0.0f64;
    let mut ref_sq = 0.0f64;
    for (&a, &r) in actual.iter().zip(reference.iter()) {
        verify_b32_scalar_budget(a, r, scale_s)?;
        let d = a - r;
        diff_sq += d * d;
        ref_sq += r * r;
    }
    let l2_diff = diff_sq.sqrt();
    let l2_ref = ref_sq.sqrt();
    let rel_l2 = if l2_ref > 0.0 {
        l2_diff / l2_ref
    } else {
        l2_diff / scale_s
    };
    if l2_ref > 0.0 && rel_l2 > 5.0e-6 {
        return Err(ConservationValidationError::VectorRelativeL2Exceeded {
            rel_l2_error: rel_l2,
            max_allowed: 5.0e-6,
            l2_diff,
            l2_ref,
        });
    }
    Ok(rel_l2)
}

/// Instantaneous electromagnetic Mode 0 oracle output (vacuum or isotropic lossless medium).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EmMode0OracleResult {
    /// Instantaneous Poynting vector $\mathbf{S} = \mathbf{E} \times \mathbf{H}$ ($\text{W/m}^2$).
    pub poynting_w_per_m2: [f64; 3],
    /// Peak-phasor time-averaged Poynting vector $\langle\mathbf{S}\rangle = \frac{1}{2}\mathbf{S}$ ($\text{W/m}^2$).
    pub peak_phasor_mean_poynting_w_per_m2: [f64; 3],
    /// Instantaneous electromagnetic energy density $u = \frac{1}{2}(\varepsilon \mathbf{E}^2 + \mu^{-1} \mathbf{B}^2)$ ($\text{J/m}^3$).
    pub energy_density_j_per_m3: f64,
}

/// Evaluate the independent C02 Mode 0 instantaneous $\mathbf{E}/\mathbf{B}$ oracle in an
/// isotropic medium ($\varepsilon_r > 0, \mu_r > 0$).
pub fn evaluate_em_mode0_instantaneous_oracle(
    e_v_per_m: [f64; 3],
    b_tesla: [f64; 3],
    epsilon_r: f64,
    mu_r: f64,
) -> Result<EmMode0OracleResult, ConservationValidationError> {
    for &v in e_v_per_m.iter().chain(b_tesla.iter()) {
        if !v.is_finite() {
            return Err(ConservationValidationError::NonFiniteValue {
                context: "em_mode0_field",
                value: v,
            });
        }
    }
    if !epsilon_r.is_finite() || epsilon_r <= 0.0 || !mu_r.is_finite() || mu_r <= 0.0 {
        return Err(ConservationValidationError::InvalidConstitutiveParameters {
            epsilon_r,
            mu_r,
            sigma_s_per_m: 0.0,
        });
    }
    let mu = VACUUM_PERMEABILITY_H_PER_M * mu_r;
    let eps = VACUUM_PERMITTIVITY_F_PER_M * epsilon_r;
    let ex_b = [
        e_v_per_m[1] * b_tesla[2] - e_v_per_m[2] * b_tesla[1],
        e_v_per_m[2] * b_tesla[0] - e_v_per_m[0] * b_tesla[2],
        e_v_per_m[0] * b_tesla[1] - e_v_per_m[1] * b_tesla[0],
    ];
    let poynting = [ex_b[0] / mu, ex_b[1] / mu, ex_b[2] / mu];
    let peak_mean = [0.5 * poynting[0], 0.5 * poynting[1], 0.5 * poynting[2]];
    let e_sq = e_v_per_m[0] * e_v_per_m[0]
        + e_v_per_m[1] * e_v_per_m[1]
        + e_v_per_m[2] * e_v_per_m[2];
    let b_sq =
        b_tesla[0] * b_tesla[0] + b_tesla[1] * b_tesla[1] + b_tesla[2] * b_tesla[2];
    let u = 0.5 * (eps * e_sq + b_sq / mu);

    Ok(EmMode0OracleResult {
        poynting_w_per_m2: poynting,
        peak_phasor_mean_poynting_w_per_m2: peak_mean,
        energy_density_j_per_m3: u,
    })
}

/// Evaluate phase wavelength $\lambda = \frac{c_0}{f \sqrt{\varepsilon_r \mu_r}}$, strictly
/// rejecting static/aperiodic fields ($f \le 0$) per C02-C02 and Mode 0.
pub fn evaluate_em_wavelength_m(
    carrier_hz: f64,
    epsilon_r: f64,
    mu_r: f64,
) -> Result<f64, ConservationValidationError> {
    if !carrier_hz.is_finite() || carrier_hz <= 0.0 {
        return Err(ConservationValidationError::StaticFieldWavelengthForbidden { carrier_hz });
    }
    if !epsilon_r.is_finite() || epsilon_r <= 0.0 || !mu_r.is_finite() || mu_r <= 0.0 {
        return Err(ConservationValidationError::InvalidConstitutiveParameters {
            epsilon_r,
            mu_r,
            sigma_s_per_m: 0.0,
        });
    }
    let phase_speed = VACUUM_SPEED_OF_LIGHT_M_PER_S / (epsilon_r * mu_r).sqrt();
    Ok(phase_speed / carrier_hz)
}

/// Verify Poynting energy conservation and passivity balance:
/// $\nabla\cdot\mathbf{S} + \frac{\partial u}{\partial t} = -w_{\text{loss}}$ with $w_{\text{loss}} = \sigma \|\mathbf{E}\|^2 \ge 0$.
pub fn verify_poynting_conservation_balance(
    div_s_w_per_m3: f64,
    du_dt_w_per_m3: f64,
    ohmic_loss_w_per_m3: f64,
    power_scale_q: f64,
) -> Result<f64, ConservationValidationError> {
    if !div_s_w_per_m3.is_finite()
        || !du_dt_w_per_m3.is_finite()
        || !ohmic_loss_w_per_m3.is_finite()
    {
        return Err(ConservationValidationError::NonFiniteValue {
            context: "poynting_balance",
            value: div_s_w_per_m3 + du_dt_w_per_m3 + ohmic_loss_w_per_m3,
        });
    }
    if !power_scale_q.is_finite() || power_scale_q <= 0.0 {
        return Err(ConservationValidationError::InvalidCharacteristicScale {
            scale: power_scale_q,
        });
    }
    let budget = 64.0 * F64_EPSILON * power_scale_q;
    if ohmic_loss_w_per_m3 < -budget {
        return Err(
            ConservationValidationError::PassivityOrConservationViolation {
                residual: div_s_w_per_m3 + du_dt_w_per_m3 + ohmic_loss_w_per_m3,
                dissipated_power_w_per_m3: ohmic_loss_w_per_m3,
                budget,
            },
        );
    }
    let residual = (div_s_w_per_m3 + du_dt_w_per_m3 + ohmic_loss_w_per_m3).abs();
    if residual > budget {
        return Err(
            ConservationValidationError::PassivityOrConservationViolation {
                residual,
                dissipated_power_w_per_m3: ohmic_loss_w_per_m3,
                budget,
            },
        );
    }
    Ok(residual)
}

/// Verify Cartesian 3D Yee CFL stability (C02 Mode 2):
/// $\Delta t \le \frac{1}{v \sqrt{\Delta x^{-2} + \Delta y^{-2} + \Delta z^{-2}}}$ with $v = \frac{c_0}{\sqrt{\varepsilon_r \mu_r}}$,
/// and evaluate the exact discrete dispersion RHS $(v\Delta t)^2 \sum_{j=0}^2 \frac{\sin^2(k_j \Delta x_j / 2)}{\Delta x_j^2}$.
pub fn verify_yee_cfl_and_dispersion(
    dx_m: [f64; 3],
    dt_s: f64,
    epsilon_r: f64,
    mu_r: f64,
    wavevector_rad_per_m: [f64; 3],
) -> Result<(f64, f64), ConservationValidationError> {
    if !epsilon_r.is_finite() || epsilon_r <= 0.0 || !mu_r.is_finite() || mu_r <= 0.0 {
        return Err(ConservationValidationError::InvalidConstitutiveParameters {
            epsilon_r,
            mu_r,
            sigma_s_per_m: 0.0,
        });
    }
    for &d in &dx_m {
        if !d.is_finite() || d <= 0.0 {
            return Err(ConservationValidationError::InvalidCharacteristicScale { scale: d });
        }
    }
    if !dt_s.is_finite() || dt_s <= 0.0 {
        return Err(ConservationValidationError::InvalidCharacteristicScale { scale: dt_s });
    }
    let v = VACUUM_SPEED_OF_LIGHT_M_PER_S / (epsilon_r * mu_r).sqrt();
    let inv_h_norm = (1.0 / (dx_m[0] * dx_m[0])
        + 1.0 / (dx_m[1] * dx_m[1])
        + 1.0 / (dx_m[2] * dx_m[2]))
        .sqrt();
    let cfl_max_dt = 1.0 / (v * inv_h_norm);
    let courant_ratio = dt_s / cfl_max_dt;
    if courant_ratio > 1.0 + 64.0 * F64_EPSILON {
        return Err(ConservationValidationError::CflStabilityViolation {
            dt_s,
            cfl_max_dt_s: cfl_max_dt,
            courant_ratio,
        });
    }

    let mut spatial_sum = 0.0f64;
    for j in 0..3 {
        let s = (0.5 * wavevector_rad_per_m[j] * dx_m[j]).sin();
        spatial_sum += (s * s) / (dx_m[j] * dx_m[j]);
    }
    let rhs = (v * dt_s) * (v * dt_s) * spatial_sum;
    let omega_num = (2.0 / dt_s) * rhs.clamp(0.0, 1.0).sqrt().asin();
    Ok((cfl_max_dt, omega_num))
}

/// Independent QAC Mode 00 fluid acoustic oracle result.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QacFluidMode00OracleResult {
    pub sound_speed_m_per_s: f64,
    pub characteristic_impedance_rayl: f64,
    pub instantaneous_intensity_w_per_m2: [f64; 3],
    pub cycle_mean_intensity_w_per_m2: [f64; 3],
    pub instantaneous_energy_density_j_per_m3: f64,
}

/// Evaluate QAC Mode 00 fluid acoustic perturbation oracle (C02 Mode 00):
/// $c = \sqrt{K/\rho}$, $Z = \sqrt{\rho K}$, $\mathbf{I} = p\mathbf{v}$, $e = \frac{1}{2}\rho\|\mathbf{v}\|^2 + \frac{p^2}{2K}$.
/// Rejects non-collocated $p$ and $\mathbf{v}$ when `collocated` is false.
pub fn evaluate_qac_fluid_mode00_oracle(
    density_kg_per_m3: f64,
    bulk_modulus_pa: f64,
    pressure_pa: f64,
    velocity_m_per_s: [f64; 3],
    collocated: bool,
) -> Result<QacFluidMode00OracleResult, ConservationValidationError> {
    if !collocated {
        return Err(ConservationValidationError::NonCollocatedPressureVelocityRejected);
    }
    if !density_kg_per_m3.is_finite()
        || density_kg_per_m3 <= 0.0
        || !bulk_modulus_pa.is_finite()
        || bulk_modulus_pa <= 0.0
    {
        return Err(ConservationValidationError::InvalidMediumParameters {
            density_kg_per_m3,
            bulk_modulus_pa,
            shear_modulus_pa: 0.0,
        });
    }
    if !pressure_pa.is_finite() || velocity_m_per_s.iter().any(|v| !v.is_finite()) {
        return Err(ConservationValidationError::NonFiniteValue {
            context: "qac_mode00",
            value: pressure_pa,
        });
    }
    let c = (bulk_modulus_pa / density_kg_per_m3).sqrt();
    let z = (density_kg_per_m3 * bulk_modulus_pa).sqrt();
    let intensity = [
        pressure_pa * velocity_m_per_s[0],
        pressure_pa * velocity_m_per_s[1],
        pressure_pa * velocity_m_per_s[2],
    ];
    let mean_intensity = [0.5 * intensity[0], 0.5 * intensity[1], 0.5 * intensity[2]];
    let v_norm = scaled_hypot3(velocity_m_per_s[0], velocity_m_per_s[1], velocity_m_per_s[2]);
    let energy = 0.5 * density_kg_per_m3 * (v_norm * v_norm)
        + 0.5 * (pressure_pa * pressure_pa) / bulk_modulus_pa;

    Ok(QacFluidMode00OracleResult {
        sound_speed_m_per_s: c,
        characteristic_impedance_rayl: z,
        instantaneous_intensity_w_per_m2: intensity,
        cycle_mean_intensity_w_per_m2: mean_intensity,
        instantaneous_energy_density_j_per_m3: energy,
    })
}

/// Independent QAC Mode 01 isotropic elastic solid oracle result.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QacElasticMode01OracleResult {
    pub lame_lambda_pa: f64,
    pub compressional_speed_cp_m_per_s: f64,
    pub shear_speed_cs_m_per_s: f64,
    /// Six-component Cauchy stress $[\sigma_{xx}, \sigma_{yy}, \sigma_{zz}, \sigma_{xy}, \sigma_{yz}, \sigma_{xz}]$ ($\text{Pa}$, tensile-positive).
    pub stress_xx_yy_zz_xy_yz_xz_pa: [f64; 6],
    /// Mechanical pressure $p = -\frac{1}{3}(\sigma_{xx} + \sigma_{yy} + \sigma_{zz})$ ($\text{Pa}$).
    pub mechanical_pressure_pa: f64,
}

/// Evaluate QAC Mode 01 small-strain isotropic elasticity oracle (C02 Mode 01):
/// $\lambda = K - \frac{2}{3}\mu$, $c_P = \sqrt{\frac{\lambda + 2\mu}{\rho}}$, $c_S = \sqrt{\frac{\mu}{\rho}}$,
/// $\boldsymbol{\sigma} = \lambda\operatorname{tr}(\boldsymbol{\varepsilon})\mathbf{I} + 2\mu\boldsymbol{\varepsilon}$,
/// $p = -\frac{1}{3}\operatorname{tr}(\boldsymbol{\sigma})$.
pub fn evaluate_qac_elastic_mode01_oracle(
    density_kg_per_m3: f64,
    bulk_modulus_pa: f64,
    shear_modulus_pa: f64,
    tensor_strain_xx_yy_zz_xy_yz_xz: [f64; 6],
) -> Result<QacElasticMode01OracleResult, ConservationValidationError> {
    if !density_kg_per_m3.is_finite()
        || density_kg_per_m3 <= 0.0
        || !bulk_modulus_pa.is_finite()
        || bulk_modulus_pa <= 0.0
        || !shear_modulus_pa.is_finite()
        || shear_modulus_pa < 0.0
    {
        return Err(ConservationValidationError::InvalidMediumParameters {
            density_kg_per_m3,
            bulk_modulus_pa,
            shear_modulus_pa,
        });
    }
    let lambda = bulk_modulus_pa - (2.0 / 3.0) * shear_modulus_pa;
    let cp = ((lambda + 2.0 * shear_modulus_pa) / density_kg_per_m3).sqrt();
    let cs = (shear_modulus_pa / density_kg_per_m3).sqrt();
    let tr_eps = tensor_strain_xx_yy_zz_xy_yz_xz[0]
        + tensor_strain_xx_yy_zz_xy_yz_xz[1]
        + tensor_strain_xx_yy_zz_xy_yz_xz[2];
    let two_mu = 2.0 * shear_modulus_pa;
    let stress = [
        lambda * tr_eps + two_mu * tensor_strain_xx_yy_zz_xy_yz_xz[0],
        lambda * tr_eps + two_mu * tensor_strain_xx_yy_zz_xy_yz_xz[1],
        lambda * tr_eps + two_mu * tensor_strain_xx_yy_zz_xy_yz_xz[2],
        two_mu * tensor_strain_xx_yy_zz_xy_yz_xz[3],
        two_mu * tensor_strain_xx_yy_zz_xy_yz_xz[4],
        two_mu * tensor_strain_xx_yy_zz_xy_yz_xz[5],
    ];
    let pressure = -(stress[0] + stress[1] + stress[2]) / 3.0;
    Ok(QacElasticMode01OracleResult {
        lame_lambda_pa: lambda,
        compressional_speed_cp_m_per_s: cp,
        shear_speed_cs_m_per_s: cs,
        stress_xx_yy_zz_xy_yz_xz_pa: stress,
        mechanical_pressure_pa: pressure,
    })
}

/// Evaluate QAC Mode 03 normal-incidence planar acoustic interface coefficients:
/// $r = \frac{Z_2 - Z_1}{Z_2 + Z_1}$, $t = \frac{2 Z_2}{Z_2 + Z_1}$,
/// energy reflectance $R = r^2$, energy transmittance $T = \frac{Z_1}{Z_2} t^2$,
/// verifying energy conservation $R + T = 1$.
pub fn evaluate_acoustic_interface_oracle(
    z1_rayl: f64,
    z2_rayl: f64,
) -> Result<(f64, f64, f64, f64), ConservationValidationError> {
    if !z1_rayl.is_finite() || z1_rayl <= 0.0 || !z2_rayl.is_finite() || z2_rayl <= 0.0 {
        return Err(ConservationValidationError::InvalidCharacteristicScale {
            scale: z1_rayl.min(z2_rayl),
        });
    }
    let sum = z1_rayl + z2_rayl;
    let r = (z2_rayl - z1_rayl) / sum;
    let t = (2.0 * z2_rayl) / sum;
    let cap_r = r * r;
    let cap_t = (z1_rayl / z2_rayl) * (t * t);
    verify_f64_closed_form_budget(cap_r + cap_t, 1.0, 1.0)?;
    Ok((r, t, cap_r, cap_t))
}

/// Verify Acoustic energy flux conservation and passivity:
/// $\nabla\cdot\mathbf{I} + \frac{\partial e}{\partial t} = -w_{\text{loss}}$ with $w_{\text{loss}} \ge 0$.
pub fn verify_acoustic_energy_conservation(
    div_i_w_per_m3: f64,
    de_dt_w_per_m3: f64,
    viscous_loss_w_per_m3: f64,
    scale_s: f64,
) -> Result<f64, ConservationValidationError> {
    verify_poynting_conservation_balance(
        div_i_w_per_m3,
        de_dt_w_per_m3,
        viscous_loss_w_per_m3,
        scale_s,
    )
}

/// Verify passivity of a multi-port complex scattering matrix $\mathbf{S}$ (up to $4\times 4$):
/// $\|\mathbf{S}\|_2 \le 1 + 64\,\varepsilon \iff \lambda_{\max}(\mathbf{S}^\dagger\mathbf{S}) \le (1 + 64\,\varepsilon)^2$.
pub fn verify_scattering_matrix_passivity(
    s_matrix: &[[Complex64; 4]; 4],
    active_ports: usize,
) -> Result<f64, ConservationValidationError> {
    if active_ports == 0 || active_ports > 4 {
        return Err(ConservationValidationError::InvalidReductionCount { n: active_ports });
    }
    let mut gram = [[Complex64::ZERO; 4]; 4];
    for i in 0..active_ports {
        for j in 0..active_ports {
            let mut sum = Complex64::ZERO;
            for k in 0..active_ports {
                let s_ki = s_matrix[k][i];
                let s_kj = s_matrix[k][j];
                if !s_ki.re.is_finite()
                    || !s_ki.im.is_finite()
                    || !s_kj.re.is_finite()
                    || !s_kj.im.is_finite()
                {
                    return Err(ConservationValidationError::NonFiniteValue {
                        context: "scattering_matrix",
                        value: f64::NAN,
                    });
                }
                sum = sum.add(s_ki.conj().mul(s_kj));
            }
            gram[i][j] = sum;
        }
    }
    let eigs = hermitian_4x4_eigenvalues(&gram);
    let max_eig = eigs[3];
    let tol_sq = (1.0 + 64.0 * F64_EPSILON) * (1.0 + 64.0 * F64_EPSILON);
    if max_eig > tol_sq {
        return Err(ConservationValidationError::ScatteringPassivityViolation {
            max_singular_value_sq: max_eig,
            tolerance_sq: tol_sq,
        });
    }
    Ok(max_eig.max(0.0).sqrt())
}

/// Verify observed grid-refinement convergence order $p = \frac{\ln(E_1 / E_2)}{\ln(h_1 / h_2)} \ge p_{\min}$
/// across successive grid spacings $h_1 > h_2$ with positive errors $E_1 > E_2 > 0$.
pub fn verify_grid_convergence_order(
    h_coarse: f64,
    error_coarse: f64,
    h_fine: f64,
    error_fine: f64,
    min_order: f64,
) -> Result<f64, ConservationValidationError> {
    if !h_coarse.is_finite()
        || !h_fine.is_finite()
        || h_coarse <= h_fine
        || h_fine <= 0.0
        || !error_coarse.is_finite()
        || !error_fine.is_finite()
        || error_coarse <= 0.0
        || error_fine <= 0.0
    {
        return Err(ConservationValidationError::InvalidCharacteristicScale {
            scale: error_fine,
        });
    }
    let observed_order = (error_coarse / error_fine).ln() / (h_coarse / h_fine).ln();
    if observed_order < min_order {
        return Err(ConservationValidationError::ConvergenceOrderBelowMinimum {
            observed_order,
            minimum_order: min_order,
            coarse_error: error_coarse,
            fine_error: error_fine,
        });
    }
    Ok(observed_order)
}
