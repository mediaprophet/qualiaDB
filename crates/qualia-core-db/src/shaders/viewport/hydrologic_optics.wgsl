// Hydrologic Optics, Radiative Transfer & Dielectric BSDF (`hydrologic_optics.wgsl`)
//
// Bridges high-frequency WebGPU rendering with continuous 10D Electromagnetic Field (EMF)
// transport, CASE-2 water optics, and solid dielectrics (soda-lime glass):
// - Multi-spectral Inherent Optical Properties (IOP)
// - Cauchy / Sellmeier chromatic dispersion
// - Henyey-Greenstein anisotropic Mie particulate scattering
// - Beer-Lambert volume extinction & continuous EMF phase shift
// - Volumetric in-scattering (veiling substrate in turbid post-mining clay waters)
// - Brewster-angle polarization window

struct DielectricVolumeIop {
    ior_spectrum: vec3<f32>,       // [n_red, n_green, n_blue] chromatic dispersion
    _pad0: f32,
    absorption_coeff: vec3<f32>,   // a(λ) in m⁻¹
    _pad1: f32,
    rayleigh_scattering: vec3<f32>,// b_r(λ) in m⁻¹ (molecular ∝ λ⁻⁴)
    _pad2: f32,
    mie_scattering: vec4<f32>,     // [b_r, b_g, b_b, g] in m⁻¹ + Henyey-Greenstein g ∈ [-1, 1]
};

const PI: f32 = 3.14159265359;
const TWO_PI: f32 = 6.28318530718;

// Wavelength standards in micrometers for Cauchy dispersion: Red (0.650 μm), Green (0.532 μm), Blue (0.450 μm).
const LAMBDA_RGB_UM: vec3<f32> = vec3<f32>(0.650, 0.532, 0.450);
// Wavelength standards in meters for EMF phase shifts.
const LAMBDA_RGB_M: vec3<f32> = vec3<f32>(650.0e-9, 532.0e-9, 450.0e-9);

// Cauchy dispersion: n(λ) = A + B / λ² (λ in micrometers).
fn cauchy_dispersion(A: f32, B: f32) -> vec3<f32> {
    let inv_sq = 1.0 / (LAMBDA_RGB_UM * LAMBDA_RGB_UM);
    return vec3<f32>(A + B * inv_sq.x, A + B * inv_sq.y, A + B * inv_sq.z);
}

// Henyey-Greenstein particulate scattering phase function p(θ).
// For forward-peaked suspended silt/clay, g ≈ 0.85–0.92.
fn henyey_greenstein(cos_theta: f32, g: f32) -> f32 {
    let g2 = g * g;
    let denom = 1.0 + g2 - 2.0 * g * cos_theta;
    return (1.0 - g2) / (4.0 * PI * pow(max(denom, 0.0001), 1.5));
}

// Total beam attenuation coefficient c(λ) = a(λ) + b_r(λ) + b_m(λ).
fn beam_attenuation_coeff(iop: DielectricVolumeIop) -> vec3<f32> {
    return iop.absorption_coeff + iop.rayleigh_scattering + iop.mie_scattering.xyz;
}

// Beer-Lambert volume transmittance T(d) = exp(-c(λ) * d).
fn beer_lambert_transmittance(c: vec3<f32>, distance_m: f32) -> vec3<f32> {
    return exp(-c * max(distance_m, 0.0));
}

// Continuous EMF phase shift Δϕ = (2π / λ) * n(λ) * d (in radians, modulo 2π).
fn emf_phase_shift(ior: vec3<f32>, distance_m: f32) -> vec3<f32> {
    let phi = (TWO_PI / LAMBDA_RGB_M) * ior * distance_m;
    return fract(phi / TWO_PI) * TWO_PI;
}

// Wavelength-dependent Schlick Fresnel reflectance with chromatic dispersion.
fn spectral_schlick_fresnel(cos_theta: f32, ior: vec3<f32>) -> vec3<f32> {
    // Air-to-dielectric base reflectance F0 = ((n - 1) / (n + 1))²
    let f0 = ((ior - vec3<f32>(1.0)) / (ior + vec3<f32>(1.0))) * ((ior - vec3<f32>(1.0)) / (ior + vec3<f32>(1.0)));
    let omc = clamp(1.0 - cos_theta, 0.0, 1.0);
    let omc5 = pow(omc, 5.0);
    return f0 + (vec3<f32>(1.0) - f0) * omc5;
}

// Volumetric Subsurface In-Scattering (Backscatter Veil).
// High turbidity (e.g., gold-mining colloidal clay) scatters sunlight back toward the viewer,
// creating an opaque milky ochre veil that obscures the substrate.
fn volumetric_subsurface_backscatter(
    iop: DielectricVolumeIop,
    view_dir: vec3<f32>,
    sun_dir: vec3<f32>,
    water_depth: f32,
    sun_radiance: vec3<f32>,
) -> vec3<f32> {
    let cos_theta = clamp(dot(-view_dir, sun_dir), -1.0, 1.0);
    let phase = henyey_greenstein(cos_theta, iop.mie_scattering.w);
    let c = beam_attenuation_coeff(iop);

    // Radiative transfer in-scattering integration over water depth d:
    // L_in = (b * phase * E_sun / c) * (1 - exp(-c * d))
    let scatter_albedo = select(vec3<f32>(0.0), iop.mie_scattering.xyz / max(c, vec3<f32>(0.0001)), c > vec3<f32>(0.00001));
    let depth_factor = vec3<f32>(1.0) - exp(-c * max(water_depth, 0.0));

    return sun_radiance * scatter_albedo * phase * depth_factor * PI;
}

// Substrate (riverbed/ocean floor) visibility attenuation factor.
// Evaluates direct two-way path extinction combined with the veil penalty.
fn substrate_visibility_factor(
    iop: DielectricVolumeIop,
    water_depth: f32,
) -> f32 {
    let c = beam_attenuation_coeff(iop);
    let avg_c = (c.y + c.z) * 0.5;
    let direct_two_way = exp(-2.0 * avg_c * max(water_depth, 0.0));
    let veil_penalty = 1.0 / (1.0 + iop.mie_scattering.y * water_depth * 1.8);
    return clamp(direct_two_way * veil_penalty, 0.0, 1.0);
}
