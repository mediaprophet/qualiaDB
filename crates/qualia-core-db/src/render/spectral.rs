//! σ → CIE 1931 XYZ → linear sRGB — CPU oracle for canvas2d fallback and cold-path export.
//!
//! WGSL twin: `shaders/viewport/spectral.wgsl`. HDR GPU path stays linear; display paths
//! apply sRGB gamma encode via `sigma_to_display_rgb`.

#[inline]
fn fract_sigma(sigma: f32) -> f32 {
    sigma - sigma.floor()
}

#[inline]
pub fn sigma_to_cie_xyz(sigma: f32) -> [f32; 3] {
    let s = fract_sigma(sigma);
    let lambda = 400.0 + s * 300.0;

    let gauss = |lambda: f32, center: f32, width: f32| -> f32 {
        let d = (lambda - center) / width;
        (-0.5 * d * d).exp()
    };

    let x1 = 1.056 * gauss(lambda, 599.8, 43.2);
    let x2 = 0.362 * gauss(lambda, 442.0, 32.0);
    let x3 = -0.065 * gauss(lambda, 501.1, 20.4);
    let x = x1 + x2 + x3;

    let y1 = 0.821 * gauss(lambda, 568.8, 46.9);
    let y2 = 0.286 * gauss(lambda, 530.9, 16.3);
    let y = y1 + y2;

    let z1 = 1.217 * gauss(lambda, 437.0, 11.8);
    let z2 = 0.681 * gauss(lambda, 459.0, 26.0);
    let z = z1 + z2;

    [x, y, z]
}

#[inline]
pub fn xyz_to_linear_srgb(xyz: [f32; 3]) -> [f32; 3] {
    let r = 3.2404542 * xyz[0] - 1.5371385 * xyz[1] - 0.4985314 * xyz[2];
    let g = -0.9692660 * xyz[0] + 1.8760108 * xyz[1] + 0.0415560 * xyz[2];
    let b = 0.0556434 * xyz[0] - 0.2040259 * xyz[1] + 1.0572252 * xyz[2];
    [r.max(0.0), g.max(0.0), b.max(0.0)]
}

#[inline]
pub fn sigma_to_linear_rgb(sigma: f32) -> [f32; 3] {
    xyz_to_linear_srgb(sigma_to_cie_xyz(sigma))
}

#[inline]
fn linear_to_srgb_channel(c: f32) -> f32 {
    if c <= 0.0031308 {
        12.92 * c
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

/// sRGB 8-bit for canvas2d / HUD (gamma-encoded display).
#[inline]
pub fn sigma_to_display_rgb(sigma: f32) -> (u8, u8, u8) {
    let linear = sigma_to_linear_rgb(sigma);
    let scale = 1.0 / linear.iter().copied().fold(0.0_f32, f32::max).max(1e-6);
    let norm = [linear[0] * scale, linear[1] * scale, linear[2] * scale];
    (
        (linear_to_srgb_channel(norm[0].min(1.0)) * 255.0).round() as u8,
        (linear_to_srgb_channel(norm[1].min(1.0)) * 255.0).round() as u8,
        (linear_to_srgb_channel(norm[2].min(1.0)) * 255.0).round() as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_sigma_same_linear_rgb() {
        let a = sigma_to_linear_rgb(0.42);
        let b = sigma_to_linear_rgb(0.42);
        assert_eq!(a, b);
    }

    #[test]
    fn sigma_wraps_via_fract() {
        let a = sigma_to_linear_rgb(0.25);
        let b = sigma_to_linear_rgb(1.25);
        assert_eq!(a, b);
    }

    #[test]
    fn green_band_dominates_mid_sigma() {
        let rgb = sigma_to_linear_rgb(0.5);
        assert!(rgb[1] >= rgb[0]);
        assert!(rgb[1] >= rgb[2]);
    }

    #[test]
    fn linear_components_non_negative() {
        for i in 0..=10 {
            let s = i as f32 / 10.0;
            let rgb = sigma_to_linear_rgb(s);
            assert!(rgb[0] >= 0.0 && rgb[1] >= 0.0 && rgb[2] >= 0.0);
        }
    }
}