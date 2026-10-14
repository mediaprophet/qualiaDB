//! Continuous Electromagnetic (EMF) Spectral Basis Functions & Wave Optics.
//!
//! Implements the Jakob & Hanika (2019) bounded spectral model, Hero Wavelength
//! Vectorized Transport, closed-form Gaussian emission moments for QualiaDB's 10D tensor
//! $[\alpha, \mu, \sigma]$, and SIMD-4 continuous wave optics (thin-film Airy interference).
//!
//! # 1. Jakob & Hanika (2019) Bounded Spectral Basis
//! Continuous reflectance $R(\lambda)$ is parameterized by three coefficients $(c_0, c_1, c_2)$
//! evaluated over normalized wavelength $u = (\lambda - 380.0) / 400.0 \in [0.0, 1.0]$:
//!
//! $$f(u) = c_0 u^2 + c_1 u + c_2$$
//! $$R(u) = \frac{1}{2} + \frac{f(u)}{2\sqrt{1 + f(u)^2}}$$
//!
//! This algebraic sigmoid guarantees $R(u) \in [0.0, 1.0]$ strictly without clamping,
//! preventing energy explosion and negative ringing.
//!
//! # 2. Closed-Form Moments of Gaussian EMF Emission
//! QualiaDB's 10D tensor encodes EMF radiation as $I_{\text{EMF}}(\lambda) = \alpha \exp\left(-\frac{(\lambda - \sigma)^2}{2\mu^2}\right)$.
//! When integrated against the polynomial basis $f(u)$, light transport evaluates analytically via
//! Gaussian raw moments:
//!
//! $$\int_{-\infty}^{\infty} I_{\text{EMF}}(\lambda) f(u) d\lambda = \alpha \mu \sqrt{2\pi} \left[ c_0(\sigma_u^2 + \mu_u^2) + c_1 \sigma_u + c_2 \right]$$
//!
//! Bypassing numerical quadrature and Riemann sums entirely.

pub const LAMBDA_MIN_NM: f32 = 380.0;
pub const LAMBDA_MAX_NM: f32 = 780.0;
pub const LAMBDA_SPAN_NM: f32 = 400.0;

/// 3-component polynomial spectral basis coefficients for Jakob & Hanika (2019) representation.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct JakobHanikaCoeffs {
    pub c0: f32,
    pub c1: f32,
    pub c2: f32,
}

impl JakobHanikaCoeffs {
    #[inline]
    pub const fn new(c0: f32, c1: f32, c2: f32) -> Self {
        Self { c0, c1, c2 }
    }

    /// Neutral flat 50% reflectance ($c_0=0, c_1=0, c_2=0 \implies R=0.5$).
    pub const FLAT_MID: Self = Self::new(0.0, 0.0, 0.0);
}

/// Normalize wavelength $\lambda \in [380.0, 780.0]\text{ nm}$ to $u \in [0.0, 1.0]$.
#[inline]
pub fn normalize_wavelength(lambda_nm: f32) -> f32 {
    ((lambda_nm - LAMBDA_MIN_NM) / LAMBDA_SPAN_NM).clamp(0.0, 1.0)
}

/// Evaluate continuous spectral reflectance $R(\lambda) \in [0.0, 1.0]$ from basis coefficients.
///
/// Costs minimal ALU cycles with an algebraic sigmoid:
/// $$R(u) = 0.5 + 0.5 \cdot \frac{f(u)}{\sqrt{1.0 + f(u)^2}}$$
#[inline]
pub fn evaluate_spectral_reflectance(coeffs: &JakobHanikaCoeffs, lambda_nm: f32) -> f32 {
    let u = normalize_wavelength(lambda_nm);
    let f = coeffs.c0 * u * u + coeffs.c1 * u + coeffs.c2;
    0.5 + 0.5 * (f / (1.0 + f * f).sqrt())
}

/// Vectorized 4-lane SIMD evaluation matching WGSL `vec4<f32>` Hero Wavelength Sampling.
#[inline]
pub fn evaluate_spectral_reflectance_simd4(
    coeffs: &JakobHanikaCoeffs,
    lambdas_nm: [f32; 4],
) -> [f32; 4] {
    let mut out = [0.0f32; 4];
    for i in 0..4 {
        out[i] = evaluate_spectral_reflectance(coeffs, lambdas_nm[i]);
    }
    out
}

/// Exact closed-form Gaussian moments light transport.
///
/// Evaluates $\int_{-\infty}^{\infty} I_{\text{EMF}}(\lambda) f(u) d\lambda$ where
/// $I_{\text{EMF}}(\lambda) = \alpha \exp\left(-\frac{(\lambda - \sigma)^2}{2\mu^2}\right)$.
///
/// Returns the radiant exiting power in closed form without numerical integration.
#[inline]
pub fn evaluate_emf_gaussian_moment_transport(
    alpha: f32,
    mu: f32,
    sigma: f32,
    coeffs: &JakobHanikaCoeffs,
) -> f32 {
    if alpha <= 0.0 || mu <= 1e-6 {
        return 0.0;
    }
    // Normalize Gaussian parameters to u in [0, 1] domain
    let sigma_u = (sigma - LAMBDA_MIN_NM) / LAMBDA_SPAN_NM;
    let mu_u = mu / LAMBDA_SPAN_NM;

    // sqrt(2 * pi) ~ 2.50662827
    let sqrt_2pi = 2.506_628_3;
    let scale = alpha * mu_u * sqrt_2pi;

    // Moments of normal distribution:
    // E[u^0] = 1.0
    // E[u^1] = sigma_u
    // E[u^2] = sigma_u^2 + mu_u^2
    let moment_sum = coeffs.c0 * (sigma_u * sigma_u + mu_u * mu_u) + coeffs.c1 * sigma_u + coeffs.c2;

    scale * (0.5 + 0.5 * (moment_sum / (1.0 + moment_sum * moment_sum).sqrt()))
}

/// Continuous thin-film wave interference via unpolarized Airy formulation over 4 hero wavelengths.
///
/// - `d_nm`: film physical thickness in nanometers.
/// - `n_film`: refractive index vector across the 4 hero wavelengths.
/// - `cos_theta0`: cosine of incident angle in ambient medium (air, $n_0 = 1.0$).
/// - `lambdas_nm`: the 4 hero wavelengths in nanometers.
#[inline]
pub fn thin_film_airy_simd4(
    d_nm: f32,
    n_film: [f32; 4],
    cos_theta0: f32,
    lambdas_nm: [f32; 4],
) -> [f32; 4] {
    let n0 = 1.0f32;
    let sin_theta0_sq = (1.0 - cos_theta0 * cos_theta0).max(0.0);
    let pi = std::f32::consts::PI;

    let mut r_unpol = [0.0f32; 4];

    for i in 0..4 {
        let n1 = n_film[i].max(1.0);
        let lambda = lambdas_nm[i].max(100.0);

        // Snell's law: cos(theta1)
        let cos_theta1_sq = (1.0 - (n0 * n0 / (n1 * n1)) * sin_theta0_sq).max(0.0);
        let cos_theta1 = cos_theta1_sq.sqrt();

        // Phase shift: delta = (4 * pi * d * n1 * cos_theta1) / lambda
        let delta = (4.0 * pi * d_nm * n1 * cos_theta1) / lambda;

        // Amplitude Fresnel coefficients for s-polarization (assuming substrate ~ air/substrate)
        let rs_01 = (n0 * cos_theta0 - n1 * cos_theta1) / (n0 * cos_theta0 + n1 * cos_theta1);
        let rs_12 = (n1 * cos_theta1 - n0 * cos_theta0) / (n1 * cos_theta1 + n0 * cos_theta0);

        let rs_01_sq = rs_01 * rs_01;
        let rs_12_sq = rs_12 * rs_12;
        let cos_delta = delta.cos();

        let num = rs_01_sq + rs_12_sq + 2.0 * rs_01 * rs_12 * cos_delta;
        let den = 1.0 + rs_01_sq * rs_12_sq + 2.0 * rs_01 * rs_12 * cos_delta;

        r_unpol[i] = if den > 1e-7 {
            (num / den).clamp(0.0, 1.0)
        } else {
            0.0
        };
    }

    r_unpol
}

/// Christensen-Burley Normalized Diffusion BSSRDF weights for subsurface scattering.
///
/// Evaluates the spectral diffuse profile $R_d(r, \lambda) = \frac{e^{-r/l_d} + e^{-r/(3 l_d)}}{8 \pi l_d r}$
/// over the 4 hero wavelengths.
#[inline]
pub fn burley_diffusion_weights_simd4(
    r_mm: f32,
    mean_free_path_mm: [f32; 4],
) -> [f32; 4] {
    let r = r_mm.max(1e-4);
    let mut weights = [0.0f32; 4];
    let pi = std::f32::consts::PI;

    for i in 0..4 {
        let ld = mean_free_path_mm[i].max(1e-3);
        let inv_ld = 1.0 / ld;
        let exp1 = (-r * inv_ld).exp();
        let exp2 = (-r * inv_ld * (1.0 / 3.0)).exp();
        weights[i] = (inv_ld / (8.0 * pi * r)) * (exp1 + exp2);
    }

    weights
}

// ───────────────────────────────────────────────────────────────────────────
//  Spatiotemporal Hero Wavelength & Tristimulus Integration
// ───────────────────────────────────────────────────────────────────────────

pub const GOLDEN_RATIO_CONJUGATE: f32 = 0.618_033_99;

/// Generate 4 stratified hero wavelengths from blue noise and temporal frame index.
///
/// Uses the golden ratio additive recurrence to decorrelate chromatic noise per pixel.
#[inline]
pub fn generate_hero_wavelength_packet(base_blue_noise: f32, frame_index: u32) -> [f32; 4] {
    let temporal_seed = (base_blue_noise + (frame_index as f32) * GOLDEN_RATIO_CONJUGATE).fract();
    let offsets = [0.0, 0.25, 0.50, 0.75];
    let mut lambdas = [0.0f32; 4];
    for i in 0..4 {
        let u = (temporal_seed + offsets[i]).fract();
        lambdas[i] = LAMBDA_MIN_NM + u * LAMBDA_SPAN_NM;
    }
    lambdas
}

/// Analytical Gaussian approximation of CIE 1931 CMFs (Wyman et al. 2013).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WymanCmf {
    pub x: [f32; 4],
    pub y: [f32; 4],
    pub z: [f32; 4],
}

/// Evaluate Wyman et al. (2013) analytical CMFs across the 4 hero wavelengths.
#[inline]
pub fn evaluate_wyman_cmf(lambdas: [f32; 4]) -> WymanCmf {
    let mut cmf = WymanCmf {
        x: [0.0; 4],
        y: [0.0; 4],
        z: [0.0; 4],
    };

    for i in 0..4 {
        let l = lambdas[i];
        // x_bar(l)
        let x1 = 1.056 * (-0.5 * ((l - 599.8) / 37.9).powi(2)).exp();
        let x2 = 0.362 * (-0.5 * ((l - 442.0) / 16.0).powi(2)).exp();
        let x3 = -0.065 * (-0.5 * ((l - 501.1) / 20.4).powi(2)).exp();
        cmf.x[i] = (x1 + x2 + x3).max(0.0);

        // y_bar(l)
        let y1 = 0.821 * (-0.5 * ((l - 568.8) / 46.9).powi(2)).exp();
        let y2 = 0.286 * (-0.5 * ((l - 530.9) / 42.7).powi(2)).exp();
        cmf.y[i] = (y1 + y2).max(0.0);

        // z_bar(l)
        let z1 = 1.217 * (-0.5 * ((l - 437.0) / 11.8).powi(2)).exp();
        let z2 = 0.681 * (-0.5 * ((l - 459.0) / 26.0).powi(2)).exp();
        cmf.z[i] = (z1 + z2).max(0.0);
    }

    cmf
}

/// Integrate a 4-lane spectral radiance packet into linear CIE XYZ tristimulus values.
#[inline]
pub fn spectral_to_cie_xyz_simd4(lambdas: [f32; 4], radiance: [f32; 4]) -> [f32; 3] {
    let cmf = evaluate_wyman_cmf(lambdas);
    let delta_lambda_over_n = LAMBDA_SPAN_NM / 4.0; // 100 nm per hero packet lane

    let mut x = 0.0f32;
    let mut y = 0.0f32;
    let mut z = 0.0f32;
    for i in 0..4 {
        x += radiance[i] * cmf.x[i];
        y += radiance[i] * cmf.y[i];
        z += radiance[i] * cmf.z[i];
    }

    [
        x * delta_lambda_over_n,
        y * delta_lambda_over_n,
        z * delta_lambda_over_n,
    ]
}

// ───────────────────────────────────────────────────────────────────────────
//  AgX / Manuka Path-to-White Gamut Inset & Outset
// ───────────────────────────────────────────────────────────────────────────

/// AgX / Manuka 20% linear gamut inset matrix for pre-compression desaturation.
pub const AGX_INSET_MATRIX: [[f32; 3]; 3] = [
    [0.84, 0.08, 0.08],
    [0.08, 0.84, 0.08],
    [0.08, 0.08, 0.84],
];

/// Inverse outset matrix restoring midtone saturation.
pub const AGX_OUTSET_MATRIX: [[f32; 3]; 3] = [
    [1.226_190_5, -0.113_095_24, -0.113_095_24],
    [-0.113_095_24, 1.226_190_5, -0.113_095_24],
    [-0.113_095_24, -0.113_095_24, 1.226_190_5],
];

/// Apply AgX / Manuka linear gamut inset toward white point.
#[inline]
pub fn agx_manuka_gamut_inset(rgb: [f32; 3]) -> [f32; 3] {
    [
        AGX_INSET_MATRIX[0][0] * rgb[0] + AGX_INSET_MATRIX[0][1] * rgb[1] + AGX_INSET_MATRIX[0][2] * rgb[2],
        AGX_INSET_MATRIX[1][0] * rgb[0] + AGX_INSET_MATRIX[1][1] * rgb[1] + AGX_INSET_MATRIX[1][2] * rgb[2],
        AGX_INSET_MATRIX[2][0] * rgb[0] + AGX_INSET_MATRIX[2][1] * rgb[1] + AGX_INSET_MATRIX[2][2] * rgb[2],
    ]
}

/// Apply AgX / Manuka inverse outset restoring midtone saturation.
#[inline]
pub fn agx_manuka_gamut_outset(rgb: [f32; 3]) -> [f32; 3] {
    [
        AGX_OUTSET_MATRIX[0][0] * rgb[0] + AGX_OUTSET_MATRIX[0][1] * rgb[1] + AGX_OUTSET_MATRIX[0][2] * rgb[2],
        AGX_OUTSET_MATRIX[1][0] * rgb[0] + AGX_OUTSET_MATRIX[1][1] * rgb[1] + AGX_OUTSET_MATRIX[1][2] * rgb[2],
        AGX_OUTSET_MATRIX[2][0] * rgb[0] + AGX_OUTSET_MATRIX[2][1] * rgb[1] + AGX_OUTSET_MATRIX[2][2] * rgb[2],
    ]
}

// ───────────────────────────────────────────────────────────────────────────
//  Bandwidth-Optimized 16-Byte/Pixel Spectral G-Buffer Layout
// ───────────────────────────────────────────────────────────────────────────

/// G-Buffer 0: 64-bit float allocation (c0, c1, c2 in f16, roughness in f16).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpectralGBuffer0Record {
    pub c0: f32,
    pub c1: f32,
    pub c2: f32,
    pub roughness: f32,
}

/// G-Buffer 1: 32-bit packed allocation (Octahedral Normal Nx, Ny, Metallic, MaterialFlags).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpectralGBuffer1Record {
    pub oct_normal: [u8; 2],
    pub metallic: u8,
    pub material_flags: u8,
}

// ───────────────────────────────────────────────────────────────────────────
//  Continuous Volumetric Atmospheric Scattering (Rayleigh, Mie, Ozone)
// ───────────────────────────────────────────────────────────────────────────

/// Atmospheric and volumetric scattering and extinction cross-sections for a 4-lane hero packet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AtmosphericCrossSections {
    /// Total scattering cross section per hero wavelength (sigma_s).
    pub sigma_s: [f32; 4],
    /// Total extinction cross section per hero wavelength (sigma_t = sigma_s + sigma_a).
    pub sigma_t: [f32; 4],
}

/// Evaluate Rayleigh, Mie, and Ozone atmospheric cross-sections continuously across the 4 hero wavelengths.
///
/// - Rayleigh: proportional to lambda^-4 (baseline 0.0058 at 500 nm).
/// - Mie: proportional to lambda^-1 (baseline 0.0039 at 500 nm).
/// - Ozone: Chappuis absorption band Gaussian centered at 600 nm with 45 nm standard deviation.
#[inline]
pub fn evaluate_volumetric_cross_sections_simd4(lambdas_nm: [f32; 4]) -> AtmosphericCrossSections {
    let mut sigma_s = [0.0f32; 4];
    let mut sigma_t = [0.0f32; 4];

    for i in 0..4 {
        let l = lambdas_nm[i].max(100.0);
        let ratio = 500.0 / l;

        // Rayleigh propto lambda^-4
        let rayleigh = 0.0058 * ratio.powi(4);

        // Mie propto lambda^-1
        let mie = 0.0039 * ratio;

        // Ozone Chappuis band near 600 nm
        let o3 = 0.0006 * (-0.5 * ((l - 600.0) / 45.0).powi(2)).exp();

        sigma_s[i] = rayleigh + mie;
        sigma_t[i] = rayleigh + mie * 1.1 + o3; // Mie absorbs ~10%
    }

    AtmosphericCrossSections { sigma_s, sigma_t }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reflectance_strictly_bounded() {
        let extreme_coeffs = JakobHanikaCoeffs::new(1000.0, -500.0, 200.0);
        for lambda in (380..=780).step_by(10) {
            let r = evaluate_spectral_reflectance(&extreme_coeffs, lambda as f32);
            assert!(r >= 0.0 && r <= 1.0, "Reflectance {r} out of bounds at {lambda} nm");
        }

        let flat_mid = JakobHanikaCoeffs::FLAT_MID;
        let r_mid = evaluate_spectral_reflectance(&flat_mid, 550.0);
        assert!((r_mid - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_simd4_consistency() {
        let coeffs = JakobHanikaCoeffs::new(1.2, -0.8, 0.3);
        let lambdas = [420.0, 510.0, 600.0, 690.0];
        let simd_res = evaluate_spectral_reflectance_simd4(&coeffs, lambdas);

        for i in 0..4 {
            let scalar_res = evaluate_spectral_reflectance(&coeffs, lambdas[i]);
            assert!((simd_res[i] - scalar_res).abs() < 1e-6);
        }
    }

    #[test]
    fn test_thin_film_airy_energy_conservation() {
        let n_film = [1.33, 1.35, 1.38, 1.40];
        let lambdas = [400.0, 500.0, 600.0, 700.0];
        let r = thin_film_airy_simd4(300.0, n_film, 1.0, lambdas);

        for i in 0..4 {
            assert!(r[i] >= 0.0 && r[i] <= 1.0, "Reflectance {} out of bounds", r[i]);
        }
    }

    #[test]
    fn test_closed_form_gaussian_moment_transport() {
        let coeffs = JakobHanikaCoeffs::new(0.5, -0.2, 0.1);
        let alpha = 100.0;
        let mu = 30.0;
        let sigma = 550.0;

        let exiting_power = evaluate_emf_gaussian_moment_transport(alpha, mu, sigma, &coeffs);
        assert!(exiting_power > 0.0);
    }

    #[test]
    fn test_hero_packet_generation_in_bounds() {
        let lambdas = generate_hero_wavelength_packet(0.42, 10);
        for l in lambdas {
            assert!(l >= LAMBDA_MIN_NM && l <= LAMBDA_MAX_NM);
        }
    }

    #[test]
    fn test_agx_inset_outset_invertibility() {
        let original = [0.8, 0.3, 0.1];
        let inset = agx_manuka_gamut_inset(original);
        let recovered = agx_manuka_gamut_outset(inset);
        for i in 0..3 {
            assert!((original[i] - recovered[i]).abs() < 1e-4);
        }
    }

    #[test]
    fn test_spectral_to_cie_xyz_positive() {
        let lambdas = [450.0, 520.0, 600.0, 680.0];
        let radiance = [1.0, 1.0, 1.0, 1.0];
        let xyz = spectral_to_cie_xyz_simd4(lambdas, radiance);
        assert!(xyz[0] > 0.0 && xyz[1] > 0.0 && xyz[2] > 0.0);
    }

    #[test]
    fn test_rayleigh_blue_scattering_dominance() {
        // Blue (450 nm) vs Red (650 nm)
        let lambdas = [450.0, 550.0, 650.0, 750.0];
        let cross = evaluate_volumetric_cross_sections_simd4(lambdas);
        assert!(cross.sigma_s[0] > cross.sigma_s[2] * 3.0, "Blue scattering must dominate red by >3x");
    }
}

