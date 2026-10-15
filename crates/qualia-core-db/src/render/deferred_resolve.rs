//! Deferred Resolve Kernel — CPU Reference Oracle & WebGPU Validation.
//!
//! Provides the mathematical reference oracle for the Deferred Resolve compute pass:
//! - Jakob & Hanika (2019) 6-ALU cycle continuous EMF spectral basis evaluation.
//! - Wyman et al. (2013) analytic Color Matching Functions (CMFs) for 4 hero wavelengths.
//! - Decoupled light loop accumulation in 4 FMAs per light.
//! - Projection from spectral radiance to CIE 1931 XYZ and conversion to linear sRGB.
//! - Zero-heap execution on caller-supplied data.

use std::f32::consts::PI;

/// Frame and camera parameters required for deferred resolve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeferredCameraParams {
    pub camera_pos: [f32; 3],
    pub viewport_dims: [f32; 2],
    pub temporal_jitter_w: f32,
}

/// Light source parameters matching the GPU Light struct.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResolvedLight {
    pub position: [f32; 3],
    pub radius: f32,
    pub direction: [f32; 3],
    pub spot_inner_cos: f32,
    pub spot_outer_cos: f32,
    pub radiance_flux: [f32; 3],
    pub illuminance_lux: f32,
    pub light_type: u32, // 0 = Directional, 1 = Point, 2 = Spot
    pub is_active: bool,
}

/// G-Buffer pixel inputs for the resolve pass.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GBufferPixel {
    pub spectral_coeffs: [f32; 3], // c0, c1, c2
    pub roughness: f32,
    pub normal: [f32; 3],
    pub metallic: f32,
    pub world_pos: [f32; 3],
    pub depth: f32,
}

/// Generate 4 Hero Wavelengths in nm from spatiotemporal blue noise.
#[inline]
pub fn generate_hero_wavelengths(blue_noise: f32, jitter_w: f32) -> [f32; 4] {
    let seed = (blue_noise + jitter_w).fract();
    let lambda_0 = 380.0 + seed * 400.0;
    let base_u = (lambda_0 - 380.0) / 400.0;
    let offsets = [0.0f32, 0.25, 0.5, 0.75];
    let mut lambdas = [0.0f32; 4];
    for i in 0..4 {
        lambdas[i] = 380.0 + (base_u + offsets[i]).fract() * 400.0;
    }
    lambdas
}

/// Jakob & Hanika (2019) 6-ALU cycle bounded spectral basis evaluation.
/// Guaranteed physical reflectance in [0.0, 1.0] without negative ringing.
#[inline]
pub fn evaluate_spectral_reflectance(coeffs: [f32; 3], u: [f32; 4]) -> [f32; 4] {
    let mut out = [0.0f32; 4];
    for i in 0..4 {
        let ui = u[i];
        let f = coeffs[0] * ui * ui + coeffs[1] * ui + coeffs[2];
        let denom = (1.0 + f * f).sqrt();
        out[i] = 0.5 + 0.5 * (f / denom);
    }
    out
}

/// Wyman et al. (2013) analytic Gaussian sum CMF evaluations for 4 hero wavelengths.
#[inline]
pub fn evaluate_cmf_xyz(lambda: [f32; 4]) -> ([f32; 4], [f32; 4], [f32; 4]) {
    let mut cmf_x = [0.0f32; 4];
    let mut cmf_y = [0.0f32; 4];
    let mut cmf_z = [0.0f32; 4];

    for i in 0..4 {
        let l = lambda[i];
        let x1 = 1.056 * (-0.5 * ((l - 599.8) / 43.2).powi(2)).exp();
        let x2 = 0.362 * (-0.5 * ((l - 442.0) / 32.0).powi(2)).exp();
        let x3 = -0.065 * (-0.5 * ((l - 501.1) / 20.4).powi(2)).exp();
        cmf_x[i] = (x1 + x2 + x3).max(0.0);

        let y1 = 0.821 * (-0.5 * ((l - 568.8) / 46.9).powi(2)).exp();
        let y2 = 0.286 * (-0.5 * ((l - 530.9) / 16.3).powi(2)).exp();
        cmf_y[i] = (y1 + y2).max(0.0);

        let z1 = 1.217 * (-0.5 * ((l - 437.0) / 11.8).powi(2)).exp();
        let z2 = 0.681 * (-0.5 * ((l - 459.0) / 26.0).powi(2)).exp();
        cmf_z[i] = (z1 + z2).max(0.0);
    }

    (cmf_x, cmf_y, cmf_z)
}

/// Convert CIE 1931 XYZ to linear sRGB primaries (D65 white point).
#[inline]
pub fn xyz_to_linear_srgb(xyz: [f32; 3]) -> [f32; 3] {
    let r = 3.2404542 * xyz[0] - 1.5371385 * xyz[1] - 0.4985314 * xyz[2];
    let g = -0.9692660 * xyz[0] + 1.8760108 * xyz[1] + 0.0415560 * xyz[2];
    let b = 0.0556434 * xyz[0] - 0.2040259 * xyz[1] + 1.0572252 * xyz[2];
    [r.max(0.0), g.max(0.0), b.max(0.0)]
}

/// Evaluate a single pixel resolve through the decoupled spectral CMF pipeline.
/// Zero heap allocations.
pub fn evaluate_deferred_resolve_pixel(
    camera: &DeferredCameraParams,
    pixel: &GBufferPixel,
    blue_noise: f32,
    lights: &[ResolvedLight],
) -> [f32; 3] {
    // 1. Sky/background early out
    if pixel.depth <= 0.0 || pixel.depth >= 1.0 {
        return [0.02, 0.02, 0.04];
    }

    // 2. Generate 4 Hero Wavelengths and normalized u
    let lambdas = generate_hero_wavelengths(blue_noise, camera.temporal_jitter_w);
    let mut u = [0.0f32; 4];
    for i in 0..4 {
        u[i] = (lambdas[i] - 380.0) / 400.0;
    }

    // 3. Evaluate Jakob-Hanika spectral reflectance
    let hero_albedo = evaluate_spectral_reflectance(pixel.spectral_coeffs, u);

    // 4. Precompute CMFs (Decoupled from light loop)
    let (cmf_x, cmf_y, cmf_z) = evaluate_cmf_xyz(lambdas);

    // 5. Normal, view direction, roughness
    let n = pixel.normal;
    let mut v = [
        camera.camera_pos[0] - pixel.world_pos[0],
        camera.camera_pos[1] - pixel.world_pos[1],
        camera.camera_pos[2] - pixel.world_pos[2],
    ];
    let v_len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-5);
    v[0] /= v_len;
    v[1] /= v_len;
    v[2] /= v_len;

    let roughness = pixel.roughness.clamp(0.04, 1.0);
    let metallic = pixel.metallic.clamp(0.0, 1.0);

    // 6. Light Loop — Accumulates Hero Spectral Radiance
    let mut hero_radiance = [0.0f32; 4];

    for light in lights {
        if !light.is_active {
            continue;
        }

        let (l_dir, atten) = match light.light_type {
            0 => {
                // Directional
                let d = [
                    -light.direction[0],
                    -light.direction[1],
                    -light.direction[2],
                ];
                let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt().max(1e-5);
                ([d[0] / len, d[1] / len, d[2] / len], light.illuminance_lux)
            }
            _ => {
                // Point or Spot
                let to_l = [
                    light.position[0] - pixel.world_pos[0],
                    light.position[1] - pixel.world_pos[1],
                    light.position[2] - pixel.world_pos[2],
                ];
                let dist_sq = to_l[0] * to_l[0] + to_l[1] * to_l[1] + to_l[2] * to_l[2];
                let dist = dist_sq.sqrt().max(1e-5);
                let dir = [to_l[0] / dist, to_l[1] / dist, to_l[2] / dist];
                let d_over_r = dist / light.radius.max(1e-4);
                let smooth_att = (1.0 - d_over_r * d_over_r).clamp(0.0, 1.0);
                let mut att = (smooth_att * smooth_att) / dist_sq.max(0.01) * light.illuminance_lux;

                if light.light_type == 2 {
                    let spot_cos = -(dir[0] * light.direction[0]
                        + dir[1] * light.direction[1]
                        + dir[2] * light.direction[2]);
                    let inner = light.spot_inner_cos;
                    let outer = light.spot_outer_cos;
                    let spot_att = ((spot_cos - outer) / (inner - outer).max(1e-4)).clamp(0.0, 1.0);
                    att *= spot_att * spot_att;
                }
                (dir, att)
            }
        };

        let n_dot_l = (n[0] * l_dir[0] + n[1] * l_dir[1] + n[2] * l_dir[2]).max(0.0);
        if n_dot_l <= 0.0 {
            continue;
        }

        let mut h = [l_dir[0] + v[0], l_dir[1] + v[1], l_dir[2] + v[2]];
        let h_len = (h[0] * h[0] + h[1] * h[1] + h[2] * h[2]).sqrt().max(1e-5);
        h[0] /= h_len;
        h[1] /= h_len;
        h[2] /= h_len;

        let n_dot_h = (n[0] * h[0] + n[1] * h[1] + n[2] * h[2]).max(0.0);
        let n_dot_v = (n[0] * v[0] + n[1] * v[1] + n[2] * v[2]).max(1e-5);
        let v_dot_h = (v[0] * h[0] + v[1] * h[1] + v[2] * h[2]).max(0.0);

        // Diffuse Lambertian in spectral space
        let inv_pi = 1.0 / PI;

        // Cook-Torrance Specular
        let a = roughness * roughness;
        let a2 = a * a;
        let denom_d = n_dot_h * n_dot_h * (a2 - 1.0) + 1.0;
        let d_term = a2 / (PI * denom_d * denom_d).max(1e-7);

        let k = (roughness + 1.0) * (roughness + 1.0) / 8.0;
        let g1_l = n_dot_l / (n_dot_l * (1.0 - k) + k);
        let g1_v = n_dot_v / (n_dot_v * (1.0 - k) + k);
        let g_term = g1_l * g1_v;

        let spec_scalar = (d_term * g_term) / (4.0 * n_dot_l * n_dot_v).max(1e-7);

        // 4 FMAs per light
        for i in 0..4 {
            let diff_spectral = hero_albedo[i] * inv_pi * (1.0 - metallic);
            let f0 = 0.04 * (1.0 - metallic) + hero_albedo[i] * metallic;
            let fresnel = f0 + (1.0 - f0) * (1.0 - v_dot_h).powi(5);
            let bsdf = (diff_spectral + fresnel * spec_scalar) * n_dot_l * atten;
            hero_radiance[i] += light.radiance_flux[0] * bsdf;
        }
    }

    // 7. Project Accumulated Hero Spectral Radiance to CIE 1931 XYZ
    let mut x = 0.0f32;
    let mut y = 0.0f32;
    let mut z = 0.0f32;
    for i in 0..4 {
        x += hero_radiance[i] * cmf_x[i];
        y += hero_radiance[i] * cmf_y[i];
        z += hero_radiance[i] * cmf_z[i];
    }
    x *= 0.25;
    y *= 0.25;
    z *= 0.25;

    // 8. Convert to Linear sRGB
    xyz_to_linear_srgb([x, y, z])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hero_wavelengths_are_equispaced_in_spectral_unit_cube() {
        let lambdas = generate_hero_wavelengths(0.35, 0.0);
        for l in lambdas {
            assert!(l >= 380.0 && l <= 780.0);
        }
        // Verify 100nm spacing across the 400nm visible band
        let u0 = (lambdas[0] - 380.0) / 400.0;
        let u1 = (lambdas[1] - 380.0) / 400.0;
        let diff = (u1 - u0).abs();
        assert!((diff - 0.25).abs() < 1e-4 || (diff - 0.75).abs() < 1e-4);
    }

    #[test]
    fn spectral_reflectance_stays_bounded() {
        let coeffs = [0.1, -0.4, 0.8];
        let u = [0.0, 0.25, 0.5, 0.75];
        let r = evaluate_spectral_reflectance(coeffs, u);
        for val in r {
            assert!(val >= 0.0 && val <= 1.0);
        }
    }

    #[test]
    fn cmf_projection_produces_finite_xyz() {
        let lambdas = [450.0, 550.0, 650.0, 700.0];
        let (x, y, z) = evaluate_cmf_xyz(lambdas);
        for i in 0..4 {
            assert!(x[i] >= 0.0);
            assert!(y[i] >= 0.0);
            assert!(z[i] >= 0.0);
        }
    }

    #[test]
    fn deferred_resolve_evaluates_pbr_pixel() {
        let camera = DeferredCameraParams {
            camera_pos: [0.0, 0.0, 5.0],
            viewport_dims: [1920.0, 1080.0],
            temporal_jitter_w: 0.0,
        };
        let pixel = GBufferPixel {
            spectral_coeffs: [0.0, 0.0, 0.0], // Neutral 0.5 reflectance
            roughness: 0.5,
            normal: [0.0, 0.0, 1.0],
            metallic: 0.0,
            world_pos: [0.0, 0.0, 0.0],
            depth: 0.5,
        };
        let lights = [ResolvedLight {
            position: [0.0, 0.0, 10.0],
            radius: 20.0,
            direction: [0.0, 0.0, -1.0],
            spot_inner_cos: 0.9,
            spot_outer_cos: 0.7,
            radiance_flux: [1.0, 1.0, 1.0],
            illuminance_lux: 1000.0,
            light_type: 0, // Directional
            is_active: true,
        }];

        let srgb = evaluate_deferred_resolve_pixel(&camera, &pixel, 0.5, &lights);
        assert!(srgb[0] > 0.0);
        assert!(srgb[1] > 0.0);
        assert!(srgb[2] > 0.0);
        assert!(srgb[0].is_finite());
    }

    #[test]
    fn naga_parses_deferred_resolve_wgsl() {
        let source = crate::shaders::viewport::DEFERRED_RESOLVE_WGSL;
        let parsed = naga::front::wgsl::parse_str(source);
        assert!(
            parsed.is_ok(),
            "DEFERRED_RESOLVE_WGSL must be valid WGSL: {:?}",
            parsed.err()
        );
    }
}
