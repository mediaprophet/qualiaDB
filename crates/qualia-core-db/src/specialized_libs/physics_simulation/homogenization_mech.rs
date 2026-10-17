//! Continuum Mechanics Homogenization (Voigt-Reuss-Hill Elasticity & Drucker-Prager Yield).
//!
//! Evaluates macroscopic effective elastic moduli and geotechnical shear failure
//! from constituent volume fractions without heap allocation.

/// Voigt-Reuss-Hill elastic homogenization result.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VrhElasticProperties {
    /// Effective bulk modulus K in Pascals.
    pub bulk_modulus_k_pa: f64,
    /// Effective shear modulus G in Pascals.
    pub shear_modulus_g_pa: f64,
    /// Effective Young's modulus E in Pascals.
    pub youngs_modulus_e_pa: f64,
    /// Effective Poisson's ratio ν.
    pub poissons_ratio_nu: f64,
}

/// Computes Voigt-Reuss-Hill bounds and average for an arbitrary constituent mixture.
///
/// `vol_fractions`: Volume fractions $\phi_i$ of each constituent (must sum to ~1.0).
/// `k_constituents`: Bulk moduli $K_i$ in Pa for each constituent.
/// `g_constituents`: Shear moduli $G_i$ in Pa for each constituent.
pub fn compute_vrh_elasticity(
    vol_fractions: &[f64],
    k_constituents: &[f64],
    g_constituents: &[f64],
) -> Option<VrhElasticProperties> {
    let n = vol_fractions.len();
    if n == 0 || n != k_constituents.len() || n != g_constituents.len() {
        return None;
    }

    let mut k_voigt = 0.0;
    let mut g_voigt = 0.0;
    let mut k_reuss_inv = 0.0;
    let mut g_reuss_inv = 0.0;

    for i in 0..n {
        let phi = vol_fractions[i];
        let k = k_constituents[i];
        let g = g_constituents[i];

        if phi < 0.0 || k <= 0.0 || g <= 0.0 {
            return None;
        }

        k_voigt += phi * k;
        g_voigt += phi * g;
        k_reuss_inv += phi / k;
        g_reuss_inv += phi / g;
    }

    if k_reuss_inv <= 0.0 || g_reuss_inv <= 0.0 {
        return None;
    }

    let k_reuss = 1.0 / k_reuss_inv;
    let g_reuss = 1.0 / g_reuss_inv;

    let k_vrh = 0.5 * (k_voigt + k_reuss);
    let g_vrh = 0.5 * (g_voigt + g_reuss);

    // Derived isotropic moduli
    let e_vrh = (9.0 * k_vrh * g_vrh) / (3.0 * k_vrh + g_vrh);
    let nu_vrh = (3.0 * k_vrh - 2.0 * g_vrh) / (2.0 * (3.0 * k_vrh + g_vrh));

    Some(VrhElasticProperties {
        bulk_modulus_k_pa: k_vrh,
        shear_modulus_g_pa: g_vrh,
        youngs_modulus_e_pa: e_vrh,
        poissons_ratio_nu: nu_vrh,
    })
}

/// Evaluates Drucker-Prager yield stress envelope for soil & rock mechanics.
///
/// $\sigma_y = \frac{6 c \cos\theta}{\sqrt{3}(3 - \sin\theta)} + \frac{2 \sin\theta}{\sqrt{3}(3 - \sin\theta)} I_1$
///
/// `cohesion_c_pa`: Mohr-Coulomb cohesion $c$ in Pascals.
/// `friction_angle_rad`: Angle of internal friction $\theta$ in radians.
/// `first_stress_invariant_i1_pa`: Trace of Cauchy stress tensor $I_1 = \sigma_{xx} + \sigma_{yy} + \sigma_{zz}$.
#[inline]
pub fn drucker_prager_yield_stress(
    cohesion_c_pa: f64,
    friction_angle_rad: f64,
    first_stress_invariant_i1_pa: f64,
) -> f64 {
    let sin_theta = friction_angle_rad.sin();
    let cos_theta = friction_angle_rad.cos();
    let sqrt3 = 3.0f64.sqrt();
    let denom = sqrt3 * (3.0 - sin_theta);

    if denom.abs() < 1e-9 {
        return f64::MAX;
    }

    let a = (6.0 * cohesion_c_pa * cos_theta) / denom;
    let b = (2.0 * sin_theta) / denom;

    a + b * first_stress_invariant_i1_pa
}

/// Evaluates whether the second invariant of deviatoric stress $\sqrt{J_2}$ exceeds the Drucker-Prager failure envelope.
#[inline]
pub fn check_drucker_prager_failure(
    cauchy_stress: [f64; 6], // [s_xx, s_yy, s_zz, tau_xy, tau_yz, tau_zx]
    cohesion_c_pa: f64,
    friction_angle_rad: f64,
) -> bool {
    let i1 = cauchy_stress[0] + cauchy_stress[1] + cauchy_stress[2];
    let mean_p = i1 / 3.0;

    let s_xx = cauchy_stress[0] - mean_p;
    let s_yy = cauchy_stress[1] - mean_p;
    let s_zz = cauchy_stress[2] - mean_p;
    let t_xy = cauchy_stress[3];
    let t_yz = cauchy_stress[4];
    let t_zx = cauchy_stress[5];

    // J_2 = 0.5 * (s_xx^2 + s_yy^2 + s_zz^2) + tau_xy^2 + tau_yz^2 + tau_zx^2
    let j2 = 0.5 * (s_xx * s_xx + s_yy * s_yy + s_zz * s_zz)
        + t_xy * t_xy
        + t_yz * t_yz
        + t_zx * t_zx;
    let sqrt_j2 = j2.sqrt();

    let yield_limit = drucker_prager_yield_stress(cohesion_c_pa, friction_angle_rad, i1);
    sqrt_j2 >= yield_limit
}
