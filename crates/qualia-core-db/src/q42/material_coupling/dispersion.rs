//! Kramers-Kronig causality & passivity verification, Brillouin dispersive energy density,
//! complex wave propagation parameters, and instantaneous vs phasor Poynting/energy relations.

use std::f64::consts::PI;

use super::constitutive::{
    Complex64, ConstitutiveError, ConstitutiveModel, C_0_M_PER_S, EPSILON_0_F_PER_M, MU_0_H_PER_M,
};

/// Errors detected during Kramers-Kronig causality, passivity, or dispersive energy checks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DispersionError {
    Constitutive(ConstitutiveError),
    EmptyOrInvalidFrequencyGrid,
    PassivityViolation {
        omega_rad_s: f64,
        omega_im_epsilon: f64,
        omega_im_mu: f64,
    },
    KramersKronigMismatch {
        omega_rad_s: f64,
        analytic_re_chi: f64,
        reconstructed_re_chi: f64,
        abs_error: f64,
        tolerance: f64,
    },
    NegativeDispersiveEnergyDensity {
        omega_rad_s: f64,
        electric_term: f64,
        magnetic_term: f64,
        total_j_per_m3: f64,
    },
    ZeroFrequencyWavelengthRejected,
    NonFiniteFieldComponent,
}

impl From<ConstitutiveError> for DispersionError {
    fn from(e: ConstitutiveError) -> Self {
        Self::Constitutive(e)
    }
}

/// Summary report of Kramers-Kronig causality and passivity verification across a frequency grid.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KramersKronigReport {
    pub grid_points_checked: usize,
    pub min_omega_im_eps_r: f64,
    pub min_omega_im_mu_r: f64,
    pub max_electric_kk_abs_error: f64,
    pub max_magnetic_kk_abs_error: f64,
    pub passivity_satisfied: bool,
    pub causality_satisfied: bool,
}

/// Verify thermodynamic passivity ($\omega\operatorname{Im}\varepsilon(\omega) \ge 0$ and
/// $\omega\operatorname{Im}\mu(\omega) \ge 0$) at every point on a strictly positive frequency grid.
pub fn verify_passivity_on_grid(
    model: &ConstitutiveModel,
    omega_grid_rad_s: &[f64],
) -> Result<(f64, f64), DispersionError> {
    model.validate()?;
    if omega_grid_rad_s.is_empty() {
        return Err(DispersionError::EmptyOrInvalidFrequencyGrid);
    }
    let mut min_e = f64::INFINITY;
    let mut min_m = f64::INFINITY;
    let mut prev = 0.0;
    for &w in omega_grid_rad_s {
        if !w.is_finite() || w <= prev {
            return Err(DispersionError::EmptyOrInvalidFrequencyGrid);
        }
        prev = w;
        let eps_r = model.relative_permittivity(w)?;
        let mu_r = model.relative_permeability(w)?;
        let w_im_e = w * eps_r.im;
        let w_im_m = w * mu_r.im;
        if w_im_e < -1e-15 || w_im_m < -1e-15 {
            return Err(DispersionError::PassivityViolation {
                omega_rad_s: w,
                omega_im_epsilon: w_im_e * EPSILON_0_F_PER_M,
                omega_im_mu: w_im_m * MU_0_H_PER_M,
            });
        }
        min_e = min_e.min(w_im_e);
        min_m = min_m.min(w_im_m);
    }
    Ok((min_e, min_m))
}

/// Reconstruct $\operatorname{Re}\chi(\omega)$ from $\operatorname{Im}\chi(\omega')$ via the
/// Kramers-Kronig Cauchy principal value integral over $[\omega_{\min}, \omega_{\max}]$
/// using analytical singularity subtraction:
/// $$\operatorname{Re}\chi(\omega) = \frac{2}{\pi}\int_{\omega_{\min}}^{\omega_{\max}} \frac{\omega'\operatorname{Im}\chi(\omega') - \omega\operatorname{Im}\chi(\omega)}{\omega'^2 - \omega^2}\,d\omega' + \frac{\operatorname{Im}\chi(\omega)}{\pi}\ln\left|\frac{\omega_{\max}-\omega}{\omega_{\max}+\omega}\cdot\frac{\omega_{\min}+\omega}{\omega_{\min}-\omega}\right|$$
pub fn kramers_kronig_re_from_im<F>(
    omega_target: f64,
    omega_min: f64,
    omega_max: f64,
    quadrature_panels: usize,
    im_chi_fn: F,
) -> Result<f64, DispersionError>
where
    F: Fn(f64) -> Result<f64, ConstitutiveError>,
{
    if !omega_target.is_finite()
        || !omega_min.is_finite()
        || !omega_max.is_finite()
        || omega_min <= 0.0
        || omega_target <= omega_min
        || omega_target >= omega_max
        || quadrature_panels < 16
    {
        return Err(DispersionError::EmptyOrInvalidFrequencyGrid);
    }

    let im_target = im_chi_fn(omega_target)?;
    let w_sq = omega_target * omega_target;
    let h = (omega_max - omega_min) / (quadrature_panels as f64);

    // Midpoint composite rule with explicit singularity subtraction avoids division by zero.
    let mut integral = 0.0;
    for i in 0..quadrature_panels {
        let wp = omega_min + (i as f64 + 0.5) * h;
        let diff = wp * wp - w_sq;
        let integrand = if diff.abs() < 1e-12 * w_sq {
            // L'Hopital limit: d/d(wp) [wp * Im(chi(wp))] / (2 * wp) via symmetric finite difference
            let delta = (1e-6 * omega_target).max(1e-9);
            let im_plus = im_chi_fn(wp + delta)?;
            let im_minus = im_chi_fn((wp - delta).max(0.5 * omega_min))?;
            let d_im = (im_plus - im_minus) / (2.0 * delta);
            (im_target + omega_target * d_im) / (2.0 * omega_target)
        } else {
            let im_wp = im_chi_fn(wp)?;
            (wp * im_wp - omega_target * im_target) / diff
        };
        integral += integrand * h;
    }

    let log_arg = ((omega_max - omega_target) / (omega_max + omega_target))
        * ((omega_min + omega_target) / (omega_target - omega_min));
    let singular_term = (im_target / PI) * log_arg.abs().ln();

    Ok((2.0 / PI) * integral + singular_term)
}

/// Reconstruct $\operatorname{Im}\chi(\omega)$ from $\operatorname{Re}\chi(\omega')$ via the inverse
/// Kramers-Kronig Cauchy principal value integral over $[\omega_{\min}, \omega_{\max}]$:
/// $$\operatorname{Im}\chi(\omega) = -\frac{2\omega}{\pi}\mathcal{P}\int_{\omega_{\min}}^{\omega_{\max}} \frac{\operatorname{Re}\chi(\omega') - \operatorname{Re}\chi(\omega)}{\omega'^2 - \omega^2}\,d\omega' - \frac{\operatorname{Re}\chi(\omega)}{\pi}\ln\left|\frac{\omega_{\max}-\omega}{\omega_{\max}+\omega}\cdot\frac{\omega_{\min}+\omega}{\omega_{\min}-\omega}\right|$$
pub fn kramers_kronig_im_from_re<F>(
    omega_target: f64,
    omega_min: f64,
    omega_max: f64,
    quadrature_panels: usize,
    re_chi_fn: F,
) -> Result<f64, DispersionError>
where
    F: Fn(f64) -> Result<f64, ConstitutiveError>,
{
    if !omega_target.is_finite()
        || !omega_min.is_finite()
        || !omega_max.is_finite()
        || omega_min <= 0.0
        || omega_target <= omega_min
        || omega_target >= omega_max
        || quadrature_panels < 16
    {
        return Err(DispersionError::EmptyOrInvalidFrequencyGrid);
    }

    let re_target = re_chi_fn(omega_target)?;
    let w_sq = omega_target * omega_target;
    let h = (omega_max - omega_min) / (quadrature_panels as f64);

    let mut integral = 0.0;
    for i in 0..quadrature_panels {
        let wp = omega_min + (i as f64 + 0.5) * h;
        let diff = wp * wp - w_sq;
        let integrand = if diff.abs() < 1e-12 * w_sq {
            let delta = (1e-6 * omega_target).max(1e-9);
            let re_plus = re_chi_fn(wp + delta)?;
            let re_minus = re_chi_fn((wp - delta).max(0.5 * omega_min))?;
            let d_re = (re_plus - re_minus) / (2.0 * delta);
            d_re / (2.0 * omega_target)
        } else {
            let re_wp = re_chi_fn(wp)?;
            (re_wp - re_target) / diff
        };
        integral += integrand * h;
    }

    let log_arg = ((omega_max - omega_target) / (omega_max + omega_target))
        * ((omega_min + omega_target) / (omega_target - omega_min));
    let singular_term = (re_target / PI) * log_arg.abs().ln();

    Ok(-(2.0 * omega_target / PI) * integral - singular_term)
}

/// Perform a full Kramers-Kronig causality and passivity verification on `model` across
/// interior test frequencies `eval_omegas_rad_s` using integration domain `[omega_min, omega_max]`.
pub fn verify_kramers_kronig_consistency(
    model: &ConstitutiveModel,
    eval_omegas_rad_s: &[f64],
    omega_min: f64,
    omega_max: f64,
    quadrature_panels: usize,
    abs_tolerance: f64,
) -> Result<KramersKronigReport, DispersionError> {
    let (min_e, min_m) = verify_passivity_on_grid(model, eval_omegas_rad_s)?;
    let mut max_e_err = 0.0f64;
    let mut max_m_err = 0.0f64;

    for &w in eval_omegas_rad_s {
        let chi_e = model.bound_electric_susceptibility(w)?;
        let recon_re_e = kramers_kronig_re_from_im(
            w,
            omega_min,
            omega_max,
            quadrature_panels,
            |wp| Ok(model.bound_electric_susceptibility(wp)?.im),
        )?;
        let err_e = (chi_e.re - recon_re_e).abs();
        if err_e > abs_tolerance {
            return Err(DispersionError::KramersKronigMismatch {
                omega_rad_s: w,
                analytic_re_chi: chi_e.re,
                reconstructed_re_chi: recon_re_e,
                abs_error: err_e,
                tolerance: abs_tolerance,
            });
        }
        max_e_err = max_e_err.max(err_e);

        let chi_m = model.bound_magnetic_susceptibility(w)?;
        let recon_re_m = kramers_kronig_re_from_im(
            w,
            omega_min,
            omega_max,
            quadrature_panels,
            |wp| Ok(model.bound_magnetic_susceptibility(wp)?.im),
        )?;
        let err_m = (chi_m.re - recon_re_m).abs();
        if err_m > abs_tolerance {
            return Err(DispersionError::KramersKronigMismatch {
                omega_rad_s: w,
                analytic_re_chi: chi_m.re,
                reconstructed_re_chi: recon_re_m,
                abs_error: err_m,
                tolerance: abs_tolerance,
            });
        }
        max_m_err = max_m_err.max(err_m);
    }

    Ok(KramersKronigReport {
        grid_points_checked: eval_omegas_rad_s.len(),
        min_omega_im_eps_r: min_e,
        min_omega_im_mu_r: min_m,
        max_electric_kk_abs_error: max_e_err,
        max_magnetic_kk_abs_error: max_m_err,
        passivity_satisfied: true,
        causality_satisfied: true,
    })
}

/// Evaluate Brillouin time-averaged stored dispersive electromagnetic energy density (in $\text{J}\cdot\text{m}^{-3}$)
/// for peak-phasor electric field amplitude $|\mathbf{E}_{\text{peak}}|$ ($\text{V}\cdot\text{m}^{-1}$)
/// and magnetic field amplitude $|\mathbf{H}_{\text{peak}}|$ ($\text{A}\cdot\text{m}^{-1}$):
/// $$u_{\text{Brillouin}}(\omega) = \frac{1}{4}\frac{\partial(\omega\operatorname{Re}\varepsilon(\omega))}{\partial\omega}|\mathbf{E}_{\text{peak}}|^2 + \frac{1}{4}\frac{\partial(\omega\operatorname{Re}\mu(\omega))}{\partial\omega}|\mathbf{H}_{\text{peak}}|^2$$
pub fn evaluate_brillouin_energy_density_j_per_m3(
    model: &ConstitutiveModel,
    omega_rad_s: f64,
    e_peak_norm_sq: f64,
    h_peak_norm_sq: f64,
) -> Result<f64, DispersionError> {
    if !e_peak_norm_sq.is_finite()
        || !h_peak_norm_sq.is_finite()
        || e_peak_norm_sq < 0.0
        || h_peak_norm_sq < 0.0
    {
        return Err(DispersionError::NonFiniteFieldComponent);
    }
    let de = model.d_omega_re_epsilon_d_omega(omega_rad_s)?;
    let dm = model.d_omega_re_mu_d_omega(omega_rad_s)?;
    let u_e = 0.25 * de * e_peak_norm_sq;
    let u_m = 0.25 * dm * h_peak_norm_sq;
    let total = u_e + u_m;
    if u_e < 0.0 || u_m < 0.0 || total < 0.0 {
        return Err(DispersionError::NegativeDispersiveEnergyDensity {
            omega_rad_s,
            electric_term: u_e,
            magnetic_term: u_m,
            total_j_per_m3: total,
        });
    }
    Ok(total)
}

/// Wave propagation quantities at angular frequency $\omega > 0$ in a homogeneous isotropic medium.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EmWavePropagation {
    pub omega_rad_s: f64,
    pub complex_refractive_index: Complex64,
    pub complex_impedance_ohms: Complex64,
    pub complex_wavenumber_rad_per_m: Complex64,
    pub phase_velocity_m_per_s: f64,
    pub attenuation_neper_per_m: f64,
    pub wavelength_m: f64,
}

/// Compute wave impedance, refractive index, wavenumber, phase speed, attenuation, and wavelength.
/// Rejects zero or negative frequency per C02-C02 ("never evaluate $c/f$ at zero").
pub fn evaluate_wave_propagation(
    model: &ConstitutiveModel,
    omega_rad_s: f64,
) -> Result<EmWavePropagation, DispersionError> {
    if !omega_rad_s.is_finite() || omega_rad_s <= 0.0 {
        return Err(DispersionError::ZeroFrequencyWavelengthRejected);
    }
    let eps = model.permittivity_f_per_m(omega_rad_s)?;
    let mu = model.permeability_h_per_m(omega_rad_s)?;
    let eps_r = model.relative_permittivity(omega_rad_s)?;
    let mu_r = model.relative_permeability(omega_rad_s)?;

    let n_complex = eps_r.mul(mu_r).sqrt();
    let z_complex = mu
        .div(eps)
        .ok_or(DispersionError::Constitutive(
            ConstitutiveError::NonFiniteParameter,
        ))?
        .sqrt();
    let k_complex = n_complex.scale(omega_rad_s / C_0_M_PER_S);
    let re_k = k_complex.re.max(1e-30);
    let phase_velocity_m_per_s = omega_rad_s / re_k;
    let wavelength_m = (2.0 * PI) / re_k;

    Ok(EmWavePropagation {
        omega_rad_s,
        complex_refractive_index: n_complex,
        complex_impedance_ohms: z_complex,
        complex_wavenumber_rad_per_m: k_complex,
        phase_velocity_m_per_s,
        attenuation_neper_per_m: k_complex.im,
        wavelength_m,
    })
}

/// Representation profile for harmonic or instantaneous field observations (C02-C02 / Mode 0).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmAmplitudeConvention {
    InstantaneousSnapshot,
    PeakPhasor,
    RmsPhasor,
}

/// Instantaneous or cycle-averaged Poynting flux $\mathbf{S}$ ($\text{W}\cdot\text{m}^{-2}$) and
/// nondispersive energy density $u$ ($\text{J}\cdot\text{m}^{-3}$) for EMF Mode 0 (`0x0100`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InstantaneousOrPhasorFieldState {
    pub poynting_w_per_m2: [f64; 3],
    pub energy_density_j_per_m3: f64,
    pub magnetic_field_h_a_per_m: [f64; 3],
}

/// Evaluate $\mathbf{H} = \mathbf{B}/\mu$, Poynting flux $\mathbf{S}$, and energy density $u$
/// from $\mathbf{E}$ ($\text{V}\cdot\text{m}^{-1}$) and $\mathbf{B}$ ($\text{T}$) under an explicit
/// `EmAmplitudeConvention` (PeakPhasor includes $1/2$, RmsPhasor and InstantaneousSnapshot do not).
pub fn evaluate_isotropic_eb_state(
    e_v_per_m: [f64; 3],
    b_tesla: [f64; 3],
    epsilon_relative: f64,
    mu_relative: f64,
    convention: EmAmplitudeConvention,
) -> Result<InstantaneousOrPhasorFieldState, DispersionError> {
    if e_v_per_m.iter().any(|x| !x.is_finite())
        || b_tesla.iter().any(|x| !x.is_finite())
        || !epsilon_relative.is_finite()
        || !mu_relative.is_finite()
        || epsilon_relative <= 0.0
        || mu_relative <= 0.0
    {
        return Err(DispersionError::NonFiniteFieldComponent);
    }
    let eps = EPSILON_0_F_PER_M * epsilon_relative;
    let mu = MU_0_H_PER_M * mu_relative;
    let h = [b_tesla[0] / mu, b_tesla[1] / mu, b_tesla[2] / mu];
    let cross = [
        e_v_per_m[1] * h[2] - e_v_per_m[2] * h[1],
        e_v_per_m[2] * h[0] - e_v_per_m[0] * h[2],
        e_v_per_m[0] * h[1] - e_v_per_m[1] * h[0],
    ];
    let e_sq = e_v_per_m[0] * e_v_per_m[0]
        + e_v_per_m[1] * e_v_per_m[1]
        + e_v_per_m[2] * e_v_per_m[2];
    let h_sq = h[0] * h[0] + h[1] * h[1] + h[2] * h[2];

    let factor = match convention {
        EmAmplitudeConvention::InstantaneousSnapshot | EmAmplitudeConvention::RmsPhasor => 1.0,
        EmAmplitudeConvention::PeakPhasor => 0.5,
    };

    let poynting_w_per_m2 = [cross[0] * factor, cross[1] * factor, cross[2] * factor];
    let energy_density_j_per_m3 = factor * 0.5 * (eps * e_sq + mu * h_sq);

    Ok(InstantaneousOrPhasorFieldState {
        poynting_w_per_m2,
        energy_density_j_per_m3,
        magnetic_field_h_a_per_m: h,
    })
}
