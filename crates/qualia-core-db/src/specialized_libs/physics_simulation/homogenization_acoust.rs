//! Acoustic & Electromagnetic Homogenization (CRIM Dielectric Permittivity & Biot Poroacoustics).
//!
//! Evaluates complex dielectric permittivity (for GPR and SAR sensors) and Biot P/S-wave velocities.

/// Complex Refractive Index Method (CRIM) dielectric permittivity evaluation.
///
/// $\sqrt{\varepsilon_{\text{eff}}} = (1 - \phi)\sum \phi_i \sqrt{\varepsilon_i} + \phi S_w \sqrt{\varepsilon_w} + \phi(1 - S_w)\sqrt{\varepsilon_{\text{air}}}$
///
/// `porosity_phi`: Total connected soil porosity $\phi \in [0, 1)$.
/// `saturation_sw`: Volumetric water saturation $S_w \in [0, 1]$.
/// `mineral_fractions`: Relative fractions of solid minerals (must sum to ~1.0).
/// `eps_minerals`: Real relative dielectric permittivity of each solid mineral (e.g. 4.5 for Quartz, 6.0 for Kaolinite).
/// `eps_water`: Relative permittivity of water (typically 80.0 at 20°C).
/// `eps_air`: Relative permittivity of air (1.0).
pub fn crim_dielectric_permittivity(
    porosity_phi: f64,
    saturation_sw: f64,
    mineral_fractions: &[f64],
    eps_minerals: &[f64],
    eps_water: f64,
    eps_air: f64,
) -> f64 {
    let phi = porosity_phi.clamp(0.0, 0.99);
    let sw = saturation_sw.clamp(0.0, 1.0);

    let mut sqrt_eps_solid = 0.0;
    let n = mineral_fractions.len().min(eps_minerals.len());
    for i in 0..n {
        sqrt_eps_solid += mineral_fractions[i].max(0.0) * eps_minerals[i].max(1.0).sqrt();
    }

    let sqrt_eps_w = eps_water.max(1.0).sqrt();
    let sqrt_eps_a = eps_air.max(1.0).sqrt();

    let sqrt_eps_eff = (1.0 - phi) * sqrt_eps_solid
        + phi * sw * sqrt_eps_w
        + phi * (1.0 - sw) * sqrt_eps_a;

    sqrt_eps_eff * sqrt_eps_eff
}

/// Evaluates bulk composite density in kg/m³.
pub fn compute_bulk_density_kg_m3(
    porosity_phi: f64,
    saturation_sw: f64,
    mineral_fractions: &[f64],
    mineral_densities: &[f64],
    water_density: f64,
    air_density: f64,
) -> f64 {
    let phi = porosity_phi.clamp(0.0, 0.99);
    let sw = saturation_sw.clamp(0.0, 1.0);

    let mut solid_density = 0.0;
    let n = mineral_fractions.len().min(mineral_densities.len());
    for i in 0..n {
        solid_density += mineral_fractions[i].max(0.0) * mineral_densities[i].max(0.0);
    }

    (1.0 - phi) * solid_density + phi * sw * water_density + phi * (1.0 - sw) * air_density
}

/// Evaluates Biot poroelastic acoustic P-wave and S-wave phase velocities in m/s.
///
/// $v_P = \sqrt{\frac{K + \frac{4}{3}G}{\rho_{\text{bulk}}}}, \quad v_S = \sqrt{\frac{G}{\rho_{\text{bulk}}}}$
pub fn biot_acoustic_velocities(
    bulk_modulus_k_pa: f64,
    shear_modulus_g_pa: f64,
    bulk_density_kg_m3: f64,
) -> Option<(f64, f64)> {
    if bulk_density_kg_m3 <= 0.0 || bulk_modulus_k_pa < 0.0 || shear_modulus_g_pa < 0.0 {
        return None;
    }

    let p_term = bulk_modulus_k_pa + (4.0 / 3.0) * shear_modulus_g_pa;
    if p_term < 0.0 {
        return None;
    }

    let v_p = (p_term / bulk_density_kg_m3).sqrt();
    let v_s = (shear_modulus_g_pa / bulk_density_kg_m3).sqrt();

    Some((v_p, v_s))
}
