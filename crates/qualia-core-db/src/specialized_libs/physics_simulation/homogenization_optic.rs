//! Optical & Radiometric Homogenization (Bachmann Wetting Darkening & Kubelka-Munk Diffusion).
//!
//! Evaluates spectral darkening due to capillary pore-water menisci and aggregate pigment diffusion.

/// Evaluates Bachmann spectral reflectance darkening of moist porous soil/minerals.
///
/// $R_{\text{wet}}(\lambda) = R_{\text{dry}}(\lambda) \frac{n_w^2}{n_m^2} \left[1 - S_w\left(1 - \frac{1}{n_w^2}\right)\right]$
///
/// `r_dry`: Dry soil reflectance at target wavelength $\in [0, 1]$.
/// `n_mineral`: Mineral grain refractive index (e.g. 1.54 for Quartz).
/// `n_water`: Pore fluid refractive index (typically 1.333 for water).
/// `saturation_sw`: Volumetric water saturation $S_w \in [0, 1]$.
pub fn bachmann_wetting_reflectance(
    r_dry: f64,
    n_mineral: f64,
    n_water: f64,
    saturation_sw: f64,
) -> f64 {
    if n_mineral <= 0.0 || n_water <= 0.0 {
        return r_dry;
    }

    let sw = saturation_sw.clamp(0.0, 1.0);
    let nw2 = n_water * n_water;
    let nm2 = n_mineral * n_mineral;

    let index_ratio = nw2 / nm2;
    let internal_reflection_factor = 1.0 - sw * (1.0 - (1.0 / nw2));

    (r_dry * index_ratio * internal_reflection_factor).clamp(0.0, 1.0)
}

/// Evaluates Kubelka-Munk diffuse reflectance $R_\infty$ from absorption $K$ and scattering $S$ coefficients.
///
/// $R_\infty = 1 + \frac{K}{S} - \sqrt{\left(\frac{K}{S}\right)^2 + 2\frac{K}{S}}$
pub fn kubelka_munk_reflectance(k_absorption: f64, s_scattering: f64) -> f64 {
    if s_scattering <= 0.0 {
        return 0.0;
    }
    if k_absorption <= 0.0 {
        return 1.0;
    }

    let ratio = k_absorption / s_scattering;
    let r_inf = 1.0 + ratio - (ratio * ratio + 2.0 * ratio).sqrt();
    r_inf.clamp(0.0, 1.0)
}

/// Computes the linear mixture absorption and scattering coefficients for multiple constituents.
///
/// $K_{\text{mix}} = \sum \phi_i K_i, \quad S_{\text{mix}} = \sum \phi_i S_i$
pub fn kubelka_munk_mixture(
    vol_fractions: &[f64],
    k_constituents: &[f64],
    s_constituents: &[f64],
) -> (f64, f64) {
    let mut k_mix = 0.0;
    let mut s_mix = 0.0;
    let n = vol_fractions.len().min(k_constituents.len()).min(s_constituents.len());

    for i in 0..n {
        let phi = vol_fractions[i].max(0.0);
        k_mix += phi * k_constituents[i].max(0.0);
        s_mix += phi * s_constituents[i].max(0.0);
    }

    (k_mix, s_mix)
}
