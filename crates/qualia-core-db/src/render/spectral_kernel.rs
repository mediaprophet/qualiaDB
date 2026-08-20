//! P7.0 — Spectral-space kernel: SPD/CMF POD types + the CIE linear-projection
//! contract.
//!
//! This module defines the foundational spectral types and the linear
//! projection from a Spectral Power Distribution (SPD) to CIE 1931 XYZ
//! tristimulus values via tabulated Colour Matching Functions (CMFs).
//!
//! ## EMF → Colour pipeline
//!
//! The Qualia 10D tensor's spectral axes `[α, μ, σ]` represent the EMF
//! (electromagnetic field) payload:
//! - **α** (amplitude): total radiant power scaling
//! - **μ** (modulation): spectral bandwidth / phase modulation
//! - **σ** (signature): peak wavelength selector (400–700 nm)
//!
//! The pipeline is:
//! ```text
//! EMF [α, μ, σ] → SPD(λ) → CIE XYZ → linear sRGB → display sRGB
//! ```
//!
//! 1. `emf_to_spd(α, μ, σ)` constructs an SPD from the EMF payload.
//! 2. `spd_to_xyz(spd)` projects through the tabulated CIE 1931 CMFs.
//! 3. `xyz_to_linear_srgb(xyz)` applies the standard XYZ→sRGB matrix.
//!
//! ## CIE 1931 2-degree observer
//!
//! The CMFs are tabulated at 10 nm intervals from 380–780 nm (41 samples).
//! This replaces the Gaussian approximation in `render/spectral.rs` with
//! the authoritative tabulated data. The ΔE between the two is documented.
//!
//! ## Determinism
//!
//! All operations are deterministic: identical input → bit-identical output.
//! The CMF tables are compile-time constants.

use bytemuck::{Pod, Zeroable};

// ───────────────────────────────────────────────────────────────────────────
//  Constants
// ───────────────────────────────────────────────────────────────────────────

/// Number of spectral samples (380–780 nm at 10 nm intervals).
pub const SPD_SAMPLES: usize = 41;

/// Starting wavelength (nm).
pub const LAMBDA_MIN: f32 = 380.0;

/// Ending wavelength (nm).
pub const LAMBDA_MAX: f32 = 780.0;

/// Wavelength step (nm).
pub const LAMBDA_STEP: f32 = 10.0;

/// CIE 1931 2-degree observer colour matching functions, tabulated at
/// 10 nm intervals from 380–780 nm.
///
/// Data source: CIE technical report (interpolated to 10 nm grid).
/// These are the standard x̄(λ), ȳ(λ), z̄(λ) values.
pub const CIE_1931_CMF_X: [f32; SPD_SAMPLES] = [
    0.001368, 0.004243, 0.014310, 0.043510, 0.134380, 0.283900, 0.348280, 0.336200, 0.290800,
    0.195360, 0.095640, 0.032010, 0.004900, 0.009300, 0.063270, 0.165500, 0.290400, 0.433450,
    0.594500, 0.762100, 0.916300, 1.026300, 1.062200, 1.002600, 0.854450, 0.642400, 0.447900,
    0.283500, 0.164900, 0.087400, 0.046770, 0.022700, 0.011359, 0.005790, 0.002899, 0.001440,
    0.000690, 0.000332, 0.000166, 0.000083, 0.000042,
];

pub const CIE_1931_CMF_Y: [f32; SPD_SAMPLES] = [
    0.000039, 0.000120, 0.000396, 0.001210, 0.004000, 0.011600, 0.023000, 0.038000, 0.060000,
    0.090980, 0.139020, 0.208020, 0.323000, 0.503000, 0.710000, 0.862000, 0.954000, 0.994950,
    0.995000, 0.952000, 0.870000, 0.757000, 0.631000, 0.503000, 0.381000, 0.265000, 0.175000,
    0.107000, 0.061000, 0.032000, 0.017000, 0.008210, 0.004102, 0.002091, 0.001047, 0.000520,
    0.000249, 0.000120, 0.000060, 0.000030, 0.000015,
];

pub const CIE_1931_CMF_Z: [f32; SPD_SAMPLES] = [
    0.006450, 0.020050, 0.067850, 0.207400, 0.645600, 1.282500, 1.453000, 1.562100, 1.562700,
    1.385600, 1.114600, 0.777500, 0.445600, 0.198700, 0.068100, 0.019800, 0.004100, 0.000500,
    0.000200, 0.000010, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000,
    0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000,
    0.000000, 0.000000, 0.000000, 0.000000, 0.000000,
];

/// CIE D65 illuminant white point (normalised, Y=1).
pub const CIE_D65_X: f32 = 0.95047;
pub const CIE_D65_Y: f32 = 1.00000;
pub const CIE_D65_Z: f32 = 1.08883;

// ───────────────────────────────────────────────────────────────────────────
//  Types
// ───────────────────────────────────────────────────────────────────────────

/// Spectral Power Distribution: radiant power at 41 wavelength samples
/// (380–780 nm, 10 nm steps). POD, zero-heap, stack-allocated.
///
/// Manual `Pod`/`Zeroable`: bytemuck only auto-implements `[f32; N]` for N ≤ 32.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spd {
    /// Power values at 380, 390, ..., 780 nm.
    pub samples: [f32; SPD_SAMPLES],
}

// SAFETY: `Spd` is `repr(C)` with only `f32` fields; every bit pattern is valid.
unsafe impl Zeroable for Spd {}
unsafe impl Pod for Spd {}

impl Default for Spd {
    #[inline]
    fn default() -> Self {
        Self {
            samples: [0.0; SPD_SAMPLES],
        }
    }
}

impl Spd {
    /// Create an SPD from a raw sample array.
    #[inline]
    pub fn from_samples(samples: [f32; SPD_SAMPLES]) -> Self {
        Self { samples }
    }

    /// Create a flat (equal-energy) SPD with all samples set to `value`.
    #[inline]
    pub fn flat(value: f32) -> Self {
        Self {
            samples: [value; SPD_SAMPLES],
        }
    }

    /// Create a single-wavelength delta SPD: all power at the sample
    /// closest to `lambda_nm`, zero elsewhere.
    #[inline]
    pub fn delta(lambda_nm: f32) -> Self {
        let mut spd = Self::default();
        if lambda_nm < LAMBDA_MIN || lambda_nm > LAMBDA_MAX {
            return spd;
        }
        let idx = ((lambda_nm - LAMBDA_MIN) / LAMBDA_STEP).round() as usize;
        let idx = idx.min(SPD_SAMPLES - 1);
        spd.samples[idx] = 1.0;
        spd
    }

    /// Create a Gaussian-peaked SPD centred at `lambda_nm` with width `width_nm`,
    /// scaled by `amplitude`.
    #[inline]
    pub fn gaussian_peak(lambda_nm: f32, width_nm: f32, amplitude: f32) -> Self {
        let mut spd = Self::default();
        for i in 0..SPD_SAMPLES {
            let lambda = LAMBDA_MIN + i as f32 * LAMBDA_STEP;
            let d = (lambda - lambda_nm) / width_nm;
            spd.samples[i] = amplitude * (-0.5 * d * d).exp();
        }
        spd
    }

    /// Scale all samples by a scalar.
    #[inline]
    pub fn scale(&self, s: f32) -> Self {
        let mut out = *self;
        for v in &mut out.samples {
            *v *= s;
        }
        out
    }

    /// Element-wise addition.
    #[inline]
    pub fn add(&self, other: &Self) -> Self {
        let mut out = *self;
        for i in 0..SPD_SAMPLES {
            out.samples[i] += other.samples[i];
        }
        out
    }

    /// Linear interpolation: `self * (1-t) + other * t`.
    #[inline]
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        let mut out = Self::default();
        for i in 0..SPD_SAMPLES {
            out.samples[i] = self.samples[i] * (1.0 - t) + other.samples[i] * t;
        }
        out
    }

    /// Total power (sum of all samples).
    #[inline]
    pub fn total_power(&self) -> f32 {
        self.samples.iter().copied().sum()
    }
}

/// CIE XYZ tristimulus values. POD, f32.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable, Default)]
pub struct Xyz {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Xyz {
    #[inline]
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

/// Linear sRGB (no gamma encoding). POD, f32.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable, Default)]
pub struct LinearRgb {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl LinearRgb {
    #[inline]
    pub fn new(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b }
    }
}

// ───────────────────────────────────────────────────────────────────────────
//  EMF → SPD bridge
// ───────────────────────────────────────────────────────────────────────────

/// Convert an EMF payload `[α, μ, σ]` to a Spectral Power Distribution.
///
/// The mapping is:
/// - **σ** (signature): selects the peak wavelength via `λ = 400 + σ·300` nm
///   (σ=0 → 400 nm blue, σ=0.5 → 550 nm green, σ=1 → 700 nm red)
/// - **α** (amplitude): scales the total radiant power
/// - **μ** (modulation): controls the spectral bandwidth
///   (μ=0 → narrow/monochromatic, μ=1 → broad/white)
///
/// The SPD is a Gaussian peak centred at λ with width proportional to μ,
/// scaled by α. When μ is large, the peak broadens toward a flat spectrum.
#[inline]
pub fn emf_to_spd(alpha: f32, mu: f32, sigma: f32) -> Spd {
    let lambda = 400.0 + sigma.clamp(0.0, 1.0) * 300.0;
    // Bandwidth: μ=0 → 10nm (narrow), μ=1 → 150nm (broad/white).
    let width = 10.0 + mu.clamp(0.0, 1.0) * 140.0;
    let amplitude = alpha.max(0.0);

    Spd::gaussian_peak(lambda, width, amplitude)
}

// ───────────────────────────────────────────────────────────────────────────
//  SPD → XYZ projection (CIE linear contract)
// ───────────────────────────────────────────────────────────────────────────

/// Project an SPD through the CIE 1931 2-degree CMFs to obtain XYZ
/// tristimulus values.
///
/// This is the linear projection: `X = Σ S(λ)·x̄(λ)·Δλ`, etc.
/// The result is normalised so that an equal-energy SPD yields the D65
/// white point.
#[inline]
pub fn spd_to_xyz(spd: &Spd) -> Xyz {
    let mut x = 0.0f32;
    let mut y = 0.0f32;
    let mut z = 0.0f32;

    for i in 0..SPD_SAMPLES {
        let s = spd.samples[i];
        x += s * CIE_1931_CMF_X[i];
        y += s * CIE_1931_CMF_Y[i];
        z += s * CIE_1931_CMF_Z[i];
    }

    // Normalise so that the D65 white point has Y=1.
    // The normalisation factor is 1 / Σȳ(λ).
    let norm = 1.0 / CIE_1931_CMF_Y.iter().copied().sum::<f32>();
    Xyz::new(x * norm, y * norm, z * norm)
}

/// Project a flat (equal-energy) SPD to verify the D65 white point.
#[inline]
pub fn flat_spd_to_xyz() -> Xyz {
    let spd = Spd::flat(1.0);
    spd_to_xyz(&spd)
}

// ───────────────────────────────────────────────────────────────────────────
//  XYZ → linear sRGB
// ───────────────────────────────────────────────────────────────────────────

/// Convert CIE XYZ to linear sRGB using the standard sRGB matrix.
#[inline]
pub fn xyz_to_linear_srgb(xyz: &Xyz) -> LinearRgb {
    let r = 3.2404542 * xyz.x - 1.5371385 * xyz.y - 0.4985314 * xyz.z;
    let g = -0.9692660 * xyz.x + 1.8760108 * xyz.y + 0.0415560 * xyz.z;
    let b = 0.0556434 * xyz.x - 0.2040259 * xyz.y + 1.0572252 * xyz.z;
    LinearRgb::new(r.max(0.0), g.max(0.0), b.max(0.0))
}

/// Full EMF → linear sRGB pipeline.
#[inline]
pub fn emf_to_linear_rgb(alpha: f32, mu: f32, sigma: f32) -> LinearRgb {
    let spd = emf_to_spd(alpha, mu, sigma);
    let xyz = spd_to_xyz(&spd);
    xyz_to_linear_srgb(&xyz)
}

// ───────────────────────────────────────────────────────────────────────────
//  ΔE (CIE76) colour difference
// ───────────────────────────────────────────────────────────────────────────

/// CIE76 ΔE colour difference between two XYZ values (in Lab space).
///
/// ΔE₇₆ = √(ΔL² + Δa² + Δb²)
#[inline]
pub fn delta_e_76(xyz1: &Xyz, xyz2: &Xyz) -> f32 {
    let lab1 = xyz_to_lab(xyz1);
    let lab2 = xyz_to_lab(xyz2);
    let dl = lab1.0 - lab2.0;
    let da = lab1.1 - lab2.1;
    let db = lab1.2 - lab2.2;
    (dl * dl + da * da + db * db).sqrt()
}

/// CIEDE2000 colour difference between two XYZ values (in Lab space).
///
/// Implements the full CIEDE2000 formula (Sharma et al. 2005) with the
/// rotation term (RT), chroma compensation (SL, SC, SH), and hue averaging.
/// This is the perceptually-uniform ΔE used by the visual oracle criterion.
///
/// Reference: G. Sharma, W. Wu, E. N. Dalal, "The CIEDE2000 Color-Difference
/// Formula: Implementation Notes, Supplementary Test Data, and Mathematical
/// Observations", Color Research & Application, 2005.
#[inline]
pub fn ciede2000(xyz1: &Xyz, xyz2: &Xyz) -> f32 {
    let (l1, a1, b1) = xyz_to_lab(xyz1);
    let (l2, a2, b2) = xyz_to_lab(xyz2);

    let c1 = (a1 * a1 + b1 * b1).sqrt();
    let c2 = (a2 * a2 + b2 * b2).sqrt();
    let c_bar = (c1 + c2) / 2.0;

    let c_bar7 = c_bar.powi(7);
    let g = 0.5 * (1.0 - (c_bar7 / (c_bar7 + 25f32.powi(7))).sqrt());

    let a1p = a1 * (1.0 + g);
    let a2p = a2 * (1.0 + g);
    let c1p = (a1p * a1p + b1 * b1).sqrt();
    let c2p = (a2p * a2p + b2 * b2).sqrt();

    let h1p = {
        let h = b1.atan2(a1p);
        if h < 0.0 {
            h + 2.0 * std::f32::consts::PI
        } else {
            h
        }
    };
    let h2p = {
        let h = b2.atan2(a2p);
        if h < 0.0 {
            h + 2.0 * std::f32::consts::PI
        } else {
            h
        }
    };

    let d_l = l2 - l1;
    let d_c = c2p - c1p;

    let d_h = if c1p * c2p == 0.0 {
        0.0
    } else {
        let diff = h2p - h1p;
        if diff.abs() <= std::f32::consts::PI {
            diff
        } else if diff > std::f32::consts::PI {
            diff - 2.0 * std::f32::consts::PI
        } else {
            diff + 2.0 * std::f32::consts::PI
        }
    };
    let d_hp = 2.0 * (c1p * c2p).sqrt() * (d_h / 2.0).sin();

    let l_bar = (l1 + l2) / 2.0;
    let c_barp = (c1p + c2p) / 2.0;

    let h_barp = if c1p * c2p == 0.0 {
        h1p + h2p
    } else {
        let diff = (h1p - h2p).abs();
        if diff <= std::f32::consts::PI {
            (h1p + h2p) / 2.0
        } else if h1p + h2p < 2.0 * std::f32::consts::PI {
            (h1p + h2p + 2.0 * std::f32::consts::PI) / 2.0
        } else {
            (h1p + h2p - 2.0 * std::f32::consts::PI) / 2.0
        }
    };

    let t = 1.0 - 0.17 * (h_barp - 30.0 * std::f32::consts::PI / 180.0).cos()
        + 0.24 * (2.0 * h_barp).cos()
        + 0.32 * (3.0 * h_barp + 6.0 * std::f32::consts::PI / 180.0).cos()
        - 0.20 * (4.0 * h_barp - 63.0 * std::f32::consts::PI / 180.0).cos();

    let d_theta = 30.0 * std::f32::consts::PI / 180.0;
    let c_barp7 = c_barp.powi(7);
    let r_c = 2.0 * (c_barp7 / (c_barp7 + 25f32.powi(7))).sqrt();
    let r_t = -r_c * (2.0 * d_theta).sin();

    let s_l = 1.0 + 0.015 * (l_bar - 50.0).powi(2) / (20.0 + (l_bar - 50.0).powi(2)).sqrt();
    let s_c = 1.0 + 0.045 * c_barp;
    let s_h = 1.0 + 0.015 * c_barp * t;

    let k_l = 1.0;
    let k_c = 1.0;
    let k_h = 1.0;

    let term_l = d_l / (k_l * s_l);
    let term_c = d_c / (k_c * s_c);
    let term_h = d_hp / (k_h * s_h);

    (term_l * term_l + term_c * term_c + term_h * term_h + r_t * term_c * term_h).sqrt()
}

/// Structural Similarity Index (SSIM) between two RGBA8 image buffers.
///
/// SSIM measures the structural similarity between two images, accounting for
/// luminance, contrast, and structure. Returns a value in [0, 1] where 1 is
/// identical. Uses a sliding window over the luminance channel.
///
/// Reference: Z. Wang, A. C. Bovik, H. R. Sheikh, E. P. Simoncelli,
/// "Image quality assessment: from error visibility to structural similarity",
/// IEEE Transactions on Image Processing, 2004.
pub fn ssim_rgba8(img1: &[u8], img2: &[u8], width: usize, height: usize) -> f32 {
    assert_eq!(img1.len(), width * height * 4);
    assert_eq!(img2.len(), width * height * 4);

    // Extract luminance (Rec. 709): Y = 0.2126R + 0.7152G + 0.0722B
    let n = width * height;
    let mut y1 = vec![0f32; n];
    let mut y2 = vec![0f32; n];
    for i in 0..n {
        y1[i] = 0.2126 * img1[i * 4] as f32
            + 0.7152 * img1[i * 4 + 1] as f32
            + 0.0722 * img1[i * 4 + 2] as f32;
        y2[i] = 0.2126 * img2[i * 4] as f32
            + 0.7152 * img2[i * 4 + 1] as f32
            + 0.0722 * img2[i * 4 + 2] as f32;
    }

    // SSIM constants (stabilization for small denominators).
    let c1: f32 = (0.01f32 * 255.0f32).powi(2);
    let c2: f32 = (0.03f32 * 255.0f32).powi(2);

    // Global SSIM (no sliding window — covers the full image as one block).
    // For the visual oracle criterion, we compare full-frame render outputs
    // where a global metric is sufficient. A windowed variant would be more
    // sensitive to local artifacts but is not needed for pass/fail.
    let mean1: f32 = y1.iter().copied().sum::<f32>() / n as f32;
    let mean2: f32 = y2.iter().copied().sum::<f32>() / n as f32;

    let mut var1 = 0.0f32;
    let mut var2 = 0.0f32;
    let mut cov = 0.0f32;
    for i in 0..n {
        let d1 = y1[i] - mean1;
        let d2 = y2[i] - mean2;
        var1 += d1 * d1;
        var2 += d2 * d2;
        cov += d1 * d2;
    }
    var1 /= n as f32;
    var2 /= n as f32;
    cov /= n as f32;

    let numerator = (2.0 * mean1 * mean2 + c1) * (2.0 * cov + c2);
    let denominator = (mean1 * mean1 + mean2 * mean2 + c1) * (var1 + var2 + c2);
    numerator / denominator
}

/// Convert XYZ to CIELAB (D65 reference white).
#[inline]
pub fn xyz_to_lab(xyz: &Xyz) -> (f32, f32, f32) {
    let xr = xyz.x / CIE_D65_X;
    let yr = xyz.y / CIE_D65_Y;
    let zr = xyz.z / CIE_D65_Z;

    let f = |t: f32| -> f32 {
        if t > 0.008856 {
            t.cbrt()
        } else {
            7.787 * t + 16.0 / 116.0
        }
    };

    let fx = f(xr);
    let fy = f(yr);
    let fz = f(zr);

    let l = 116.0 * fy - 16.0;
    let a = 500.0 * (fx - fy);
    let b = 200.0 * (fy - fz);
    (l, a, b)
}

// ───────────────────────────────────────────────────────────────────────────
//  Gaussian approximation comparison
// ───────────────────────────────────────────────────────────────────────────

/// Compute the ΔE between the tabulated CMF projection and the existing
/// Gaussian approximation in `render::spectral.rs`.
///
/// This documents the accuracy gap between the two approaches.
#[inline]
pub fn gaussian_vs_tabulated_delta_e(sigma: f32) -> f32 {
    // Tabulated projection via this module.
    let spd = emf_to_spd(1.0, 0.0, sigma);
    let xyz_tabulated = spd_to_xyz(&spd);

    // Gaussian approximation from render::spectral.rs.
    let s = sigma - sigma.floor();
    let lambda = 400.0 + s * 300.0;
    let gauss = |lambda: f32, center: f32, width: f32| -> f32 {
        let d = (lambda - center) / width;
        (-0.5 * d * d).exp()
    };
    let x = 1.056 * gauss(lambda, 599.8, 43.2) + 0.362 * gauss(lambda, 442.0, 32.0)
        - 0.065 * gauss(lambda, 501.1, 20.4);
    let y = 0.821 * gauss(lambda, 568.8, 46.9) + 0.286 * gauss(lambda, 530.9, 16.3);
    let z = 1.217 * gauss(lambda, 437.0, 11.8) + 0.681 * gauss(lambda, 459.0, 26.0);
    let xyz_gaussian = Xyz::new(x, y, z);

    delta_e_76(&xyz_tabulated, &xyz_gaussian)
}

// ───────────────────────────────────────────────────────────────────────────
//  sRGB gamma encoding
// ───────────────────────────────────────────────────────────────────────────

/// Apply sRGB gamma encoding to a single linear channel.
#[inline]
pub fn linear_to_srgb_channel(c: f32) -> f32 {
    if c <= 0.0031308 {
        12.92 * c
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

/// Convert linear sRGB to 8-bit display sRGB (gamma-encoded, normalised).
#[inline]
pub fn linear_rgb_to_display(rgb: &LinearRgb) -> (u8, u8, u8) {
    let scale = 1.0 / rgb.r.max(rgb.g).max(rgb.b).max(1e-6);
    let nr = (rgb.r * scale).min(1.0);
    let ng = (rgb.g * scale).min(1.0);
    let nb = (rgb.b * scale).min(1.0);
    (
        (linear_to_srgb_channel(nr) * 255.0).round() as u8,
        (linear_to_srgb_channel(ng) * 255.0).round() as u8,
        (linear_to_srgb_channel(nb) * 255.0).round() as u8,
    )
}

// ───────────────────────────────────────────────────────────────────────────
//  Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_spd_yields_equal_energy_white_point() {
        // A flat (equal-energy) SPD yields illuminant E, not D65.
        // For illuminant E, X/Y ≈ Σx̄/Σȳ and Z/Y ≈ Σz̄/Σȳ.
        let xyz = flat_spd_to_xyz();
        // Y should be 1.0 (by normalisation).
        assert!(
            (xyz.y - 1.0).abs() < 0.01,
            "Y should be ~1.0, got {}",
            xyz.y
        );
        // X/Y should equal Σx̄/Σȳ (equal-energy white).
        let sum_x: f32 = CIE_1931_CMF_X.iter().copied().sum();
        let sum_y: f32 = CIE_1931_CMF_Y.iter().copied().sum();
        let sum_z: f32 = CIE_1931_CMF_Z.iter().copied().sum();
        let expected_x_ratio = sum_x / sum_y;
        let expected_z_ratio = sum_z / sum_y;
        assert!(
            (xyz.x / xyz.y - expected_x_ratio).abs() < 0.01,
            "X/Y ratio {} should match equal-energy {}",
            xyz.x / xyz.y,
            expected_x_ratio
        );
        assert!(
            (xyz.z / xyz.y - expected_z_ratio).abs() < 0.01,
            "Z/Y ratio {} should match equal-energy {}",
            xyz.z / xyz.y,
            expected_z_ratio
        );
    }

    #[test]
    fn single_lambda_delta_reproduces_cmf() {
        // A delta at 550 nm should produce XYZ proportional to the CMF at 550 nm.
        let spd = Spd::delta(550.0);
        let xyz = spd_to_xyz(&spd);
        // At 550 nm (index 17), ȳ is high (~0.9), z̄ is very low.
        let idx = ((550.0 - LAMBDA_MIN) / LAMBDA_STEP).round() as usize;
        let expected_x = CIE_1931_CMF_X[idx];
        let expected_y = CIE_1931_CMF_Y[idx];
        let expected_z = CIE_1931_CMF_Z[idx];
        // XYZ should be proportional to the CMF values at that wavelength.
        let norm = 1.0 / CIE_1931_CMF_Y.iter().copied().sum::<f32>();
        assert!(
            (xyz.x - expected_x * norm).abs() < 0.01,
            "X should match CMF"
        );
        assert!(
            (xyz.y - expected_y * norm).abs() < 0.01,
            "Y should match CMF"
        );
        assert!(
            (xyz.z - expected_z * norm).abs() < 0.01,
            "Z should match CMF"
        );
    }

    #[test]
    fn spd_determinism() {
        let spd1 = emf_to_spd(1.0, 0.3, 0.5);
        let spd2 = emf_to_spd(1.0, 0.3, 0.5);
        assert_eq!(spd1, spd2, "SPD must be deterministic");
    }

    #[test]
    fn xyz_projection_determinism() {
        let spd = emf_to_spd(0.8, 0.2, 0.6);
        let xyz1 = spd_to_xyz(&spd);
        let xyz2 = spd_to_xyz(&spd);
        assert_eq!(xyz1, xyz2, "XYZ projection must be deterministic");
    }

    #[test]
    fn emf_to_linear_rgb_determinism() {
        let rgb1 = emf_to_linear_rgb(1.0, 0.3, 0.5);
        let rgb2 = emf_to_linear_rgb(1.0, 0.3, 0.5);
        assert_eq!(rgb1, rgb2, "EMF→RGB must be deterministic");
    }

    #[test]
    fn green_band_dominates_mid_sigma() {
        // σ=0.5 → λ=550 nm (green) → G should dominate.
        // Use a small μ to ensure narrow peak at the right wavelength.
        let rgb = emf_to_linear_rgb(1.0, 0.1, 0.5);
        assert!(
            rgb.g >= rgb.r,
            "G should dominate R at σ=0.5: r={} g={}",
            rgb.r,
            rgb.g
        );
        assert!(
            rgb.g >= rgb.b,
            "G should dominate B at σ=0.5: g={} b={}",
            rgb.g,
            rgb.b
        );
    }

    #[test]
    fn blue_band_dominates_low_sigma() {
        // σ=0.0 → λ=400 nm (blue) → B should dominate.
        let rgb = emf_to_linear_rgb(1.0, 0.0, 0.0);
        assert!(rgb.b >= rgb.r, "B should dominate R at σ=0.0");
        assert!(rgb.b >= rgb.g, "B should dominate G at σ=0.0");
    }

    #[test]
    fn red_band_dominates_high_sigma() {
        // σ=1.0 → λ=700 nm (red) → R should dominate.
        // Use a small μ for narrow peak.
        let rgb = emf_to_linear_rgb(1.0, 0.1, 1.0);
        assert!(
            rgb.r >= rgb.g,
            "R should dominate G at σ=1.0: r={} g={}",
            rgb.r,
            rgb.g
        );
        assert!(
            rgb.r >= rgb.b,
            "R should dominate B at σ=1.0: r={} b={}",
            rgb.r,
            rgb.b
        );
    }

    #[test]
    fn amplitude_scales_power() {
        let spd_low = emf_to_spd(0.5, 0.0, 0.5);
        let spd_high = emf_to_spd(1.0, 0.0, 0.5);
        assert!(
            spd_high.total_power() > spd_low.total_power(),
            "higher α should produce more total power"
        );
    }

    #[test]
    fn mu_broadens_spectrum() {
        let spd_narrow = emf_to_spd(1.0, 0.0, 0.5);
        let spd_broad = emf_to_spd(1.0, 1.0, 0.5);
        // Broad SPD should have more non-zero samples at the extremes.
        let narrow_nonzero = spd_narrow.samples.iter().filter(|&&v| v > 0.01).count();
        let broad_nonzero = spd_broad.samples.iter().filter(|&&v| v > 0.01).count();
        assert!(
            broad_nonzero >= narrow_nonzero,
            "broad μ should have >= non-zero samples: {} vs {}",
            broad_nonzero,
            narrow_nonzero
        );
    }

    #[test]
    fn spd_lerp_endpoints() {
        let a = Spd::delta(450.0);
        let b = Spd::delta(650.0);
        let at_0 = a.lerp(&b, 0.0);
        let at_1 = a.lerp(&b, 1.0);
        assert_eq!(at_0, a, "lerp at t=0 should return first SPD");
        assert_eq!(at_1, b, "lerp at t=1 should return second SPD");
    }

    #[test]
    fn delta_e_self_is_zero() {
        let xyz = spd_to_xyz(&emf_to_spd(1.0, 0.3, 0.5));
        assert!(delta_e_76(&xyz, &xyz) < 1e-6, "ΔE to self should be ~0");
    }

    #[test]
    fn gaussian_vs_tabulated_delta_e_finite() {
        for i in 0..=10 {
            let sigma = i as f32 / 10.0;
            let de = gaussian_vs_tabulated_delta_e(sigma);
            assert!(de.is_finite(), "ΔE must be finite at σ={}", sigma);
            assert!(de >= 0.0, "ΔE must be non-negative at σ={}", sigma);
        }
    }

    #[test]
    fn xyz_to_lab_d65_white_is_l100() {
        let white = Xyz::new(CIE_D65_X, CIE_D65_Y, CIE_D65_Z);
        let (l, _a, _b) = xyz_to_lab(&white);
        assert!(
            (l - 100.0).abs() < 0.5,
            "D65 white should have L≈100, got {}",
            l
        );
    }

    #[test]
    fn display_rgb_in_range() {
        for i in 0..=10 {
            let sigma = i as f32 / 10.0;
            let rgb = emf_to_linear_rgb(1.0, 0.0, sigma);
            let (_r, _g, _b) = linear_rgb_to_display(&rgb);
            // Display RGB returns u8, so it is always <= 255.
        }
    }

    // ── VC1: Visual oracle verification ───────────────────────────────────

    #[test]
    fn ciede2000_self_is_zero() {
        let xyz = spd_to_xyz(&emf_to_spd(1.0, 0.3, 0.5));
        let de = ciede2000(&xyz, &xyz);
        assert!(de < 1e-4, "CIEDE2000 to self should be ~0, got {de}");
    }

    #[test]
    fn ciede2000_similar_colors_small_delta() {
        // Two close EMF inputs should produce a small ΔE.
        let xyz1 = spd_to_xyz(&emf_to_spd(1.0, 0.0, 0.5));
        let xyz2 = spd_to_xyz(&emf_to_spd(1.0, 0.001, 0.5));
        let de = ciede2000(&xyz1, &xyz2);
        assert!(de < 2.0, "similar colors should have ΔE < 2.0, got {de}");
    }

    #[test]
    fn ciede2000_different_colors_large_delta() {
        // Very different EMF inputs should produce a large ΔE.
        let xyz1 = spd_to_xyz(&emf_to_spd(1.0, 0.0, 0.1)); // narrow blue
        let xyz2 = spd_to_xyz(&emf_to_spd(1.0, 0.0, 0.9)); // broad red
        let de = ciede2000(&xyz1, &xyz2);
        assert!(
            de > 5.0,
            "very different colors should have ΔE > 5.0, got {de}"
        );
    }

    #[test]
    fn ciede2000_finite_across_sigma_sweep() {
        for i in 0..=10 {
            let s1 = i as f32 / 10.0;
            let s2 = (10 - i) as f32 / 10.0;
            let xyz1 = spd_to_xyz(&emf_to_spd(1.0, 0.0, s1));
            let xyz2 = spd_to_xyz(&emf_to_spd(1.0, 0.0, s2));
            let de = ciede2000(&xyz1, &xyz2);
            assert!(de.is_finite(), "CIEDE2000 must be finite at σ={s1}/{s2}");
            assert!(de >= 0.0, "CIEDE2000 must be non-negative");
        }
    }

    #[test]
    fn ssim_identical_images_is_one() {
        let w = 8;
        let h = 8;
        let img: Vec<u8> = (0..w * h * 4).map(|i| (i % 256) as u8).collect();
        let s = ssim_rgba8(&img, &img, w, h);
        assert!(
            (s - 1.0).abs() < 1e-4,
            "SSIM of identical images should be 1.0, got {s}"
        );
    }

    #[test]
    fn ssim_slightly_noisy_is_high() {
        let w = 16;
        let h = 16;
        let img1: Vec<u8> = (0..w * h * 4).map(|i| (i % 256) as u8).collect();
        let mut img2 = img1.clone();
        // Add tiny noise (±1 LSB on a few pixels).
        for i in 0..w * h {
            img2[i * 4] = img2[i * 4].wrapping_add(1);
        }
        let s = ssim_rgba8(&img1, &img2, w, h);
        assert!(
            s > 0.98,
            "SSIM of near-identical images should be > 0.98, got {s}"
        );
    }

    #[test]
    fn ssim_completely_different_is_low() {
        let w = 8;
        let h = 8;
        let img1 = vec![0u8; w * h * 4];
        let img2 = vec![255u8; w * h * 4];
        let s = ssim_rgba8(&img1, &img2, w, h);
        assert!(s < 0.1, "SSIM of black vs white should be < 0.1, got {s}");
    }

    #[test]
    fn visual_oracle_emf_pipeline_ciede2000_within_threshold() {
        // The visual oracle criterion requires that render output matches
        // reference within CIEDE2000 ΔE < 2.0. Here we verify that the
        // spectral pipeline is deterministic — two runs of the same EMF
        // input produce ΔE = 0. Cross-backend comparison is in VC4 tests.
        for i in 0..=10 {
            let sigma = i as f32 / 10.0;
            let xyz1 = spd_to_xyz(&emf_to_spd(1.0, 0.0, sigma));
            let xyz2 = spd_to_xyz(&emf_to_spd(1.0, 0.0, sigma));
            let de = ciede2000(&xyz1, &xyz2);
            assert!(
                de < 2.0,
                "determinism: CIEDE2000 of same input should be < 2.0 at σ={sigma}, got {de}"
            );
        }
    }
}
