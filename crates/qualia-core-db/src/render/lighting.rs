//! Physically-based and stylized cinematic lighting evaluation.
//!
//! Provides radiometric evaluation (lux for directional, lumens for point/spot), GGX
//! specular BRDF, cascaded PCF shadow filtering, slope/normal-offset bias, and stylized
//! toon/ramp/rim controls sharing the same underlying material parameters. Zero-heap hot path.

use std::f32::consts::PI;

/// Portable bounded environment-probe evaluation and fallback policy.
#[path = "environment_lighting.rs"]
pub mod environment_lighting;
pub use environment_lighting::*;

/// Maximum number of local (point/spot) lights evaluated per fragment/cluster.
pub const MAX_LOCAL_LIGHTS: usize = 8;

/// Directional light source (sun/moon) defined in lux (lm/m^2).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DirectionalLight {
    pub direction: [f32; 3],
    /// Surface illuminance in lux.
    pub illuminance_lux: f32,
    pub color_linear: [f32; 3],
    pub cast_shadows: bool,
}

/// Point light source defined in luminous flux (lumens).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointLight {
    pub position: [f32; 3],
    pub flux_lumens: f32,
    pub color_linear: [f32; 3],
    pub radius: f32,
}

/// Spot light source with smooth inner/outer angular falloff.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpotLight {
    pub position: [f32; 3],
    pub direction: [f32; 3],
    pub flux_lumens: f32,
    pub color_linear: [f32; 3],
    pub inner_cone_cos: f32,
    pub outer_cone_cos: f32,
    pub radius: f32,
}

/// Authored stylized art direction parameters sharing standard PBR geometry and materials.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StylizedLightingParams {
    /// Number of discrete diffuse bands (e.g. 2 for classic cel-shade, 0 for continuous).
    pub diffuse_bands: u8,
    /// Smoothstep window around diffuse band thresholds [0.0 = sharp, 0.5 = soft].
    pub band_softness: f32,
    /// Fresnel rim power [typically 2.0 - 5.0].
    pub rim_exponent: f32,
    pub rim_intensity: f32,
    pub rim_tint: [f32; 3],
    /// Quantized specular highlight threshold in N.H [e.g. 0.95].
    pub specular_threshold: f32,
    pub specular_softness: f32,
}

impl Default for StylizedLightingParams {
    fn default() -> Self {
        Self {
            diffuse_bands: 0,
            band_softness: 0.05,
            rim_exponent: 3.0,
            rim_intensity: 0.0,
            rim_tint: [1.0, 1.0, 1.0],
            specular_threshold: 0.9,
            specular_softness: 0.05,
        }
    }
}

/// Surface parameters for evaluation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceParameters {
    pub base_color: [f32; 3],
    pub metallic: f32,
    pub roughness: f32,
    pub reflectance_f0: f32,
    pub emissive: [f32; 3],
    pub ambient_occlusion: f32,
}

// ---------------------------------------------------------------------------
// Math helpers
// ---------------------------------------------------------------------------

#[inline]
pub fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[inline]
pub fn normalize3(v: [f32; 3]) -> [f32; 3] {
    let len_sq = dot3(v, v);
    if len_sq > 1e-12 {
        let inv = 1.0 / len_sq.sqrt();
        [v[0] * inv, v[1] * inv, v[2] * inv]
    } else {
        [0.0, 1.0, 0.0]
    }
}

/// Smooth windowed inverse-square distance attenuation.
pub fn distance_attenuation(distance_sq: f32, radius: f32) -> f32 {
    if radius <= 0.0 {
        return 0.0;
    }
    let inv_sq = 1.0 / (distance_sq + 1.0);
    let ratio = distance_sq / (radius * radius);
    let smooth = (1.0 - ratio * ratio).clamp(0.0, 1.0);
    inv_sq * (smooth * smooth)
}

/// Smooth angular falloff for spot lights between inner and outer cone cosines.
pub fn spot_angular_attenuation(cos_theta: f32, cos_inner: f32, cos_outer: f32) -> f32 {
    if cos_inner <= cos_outer {
        return 0.0;
    }
    let t = ((cos_theta - cos_outer) / (cos_inner - cos_outer)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t) // smoothstep
}

// ---------------------------------------------------------------------------
// PBR Specular & Diffuse BRDF (GGX / Smith / Schlick)
// ---------------------------------------------------------------------------

/// Trowbridge-Reitz GGX Normal Distribution Function.
pub fn distribution_ggx(n_dot_h: f32, roughness: f32) -> f32 {
    let alpha = roughness * roughness;
    let alpha_sq = alpha * alpha;
    let n_dot_h_sq = n_dot_h * n_dot_h;
    let denom = n_dot_h_sq * (alpha_sq - 1.0) + 1.0;
    alpha_sq / (PI * denom * denom).max(1e-7)
}

/// Smith Joint GGX Visibility function (Heitz 2014).
pub fn visibility_smith_ggx(n_dot_v: f32, n_dot_l: f32, roughness: f32) -> f32 {
    let alpha = roughness * roughness;
    let ggx_v = n_dot_l * (n_dot_v * (1.0 - alpha) + alpha);
    let ggx_l = n_dot_v * (n_dot_l * (1.0 - alpha) + alpha);
    0.5 / (ggx_v + ggx_l).max(1e-7)
}

/// Schlick approximation for Fresnel reflection.
pub fn fresnel_schlick(v_dot_h: f32, f0: [f32; 3]) -> [f32; 3] {
    let f = (1.0 - v_dot_h.clamp(0.0, 1.0)).powi(5);
    [
        f0[0] + (1.0 - f0[0]) * f,
        f0[1] + (1.0 - f0[1]) * f,
        f0[2] + (1.0 - f0[2]) * f,
    ]
}

// ---------------------------------------------------------------------------
// Shadow & Bias Math
// ---------------------------------------------------------------------------

/// Normal-offset bias world position to prevent shadow acne and self-shadowing.
pub fn apply_normal_offset_bias(
    position: [f32; 3],
    normal: [f32; 3],
    n_dot_l: f32,
    texel_size_world: f32,
) -> [f32; 3] {
    let bias = (1.0 - n_dot_l.clamp(0.0, 1.0)) * texel_size_world;
    [
        position[0] + normal[0] * bias,
        position[1] + normal[1] * bias,
        position[2] + normal[2] * bias,
    ]
}

/// Percentage-Closer Filtering (3x3 kernel) oracle weights for shadow maps.
pub const PCF_3X3_OFFSETS: [[f32; 2]; 9] = [
    [-1.0, -1.0],
    [0.0, -1.0],
    [1.0, -1.0],
    [-1.0, 0.0],
    [0.0, 0.0],
    [1.0, 0.0],
    [-1.0, 1.0],
    [0.0, 1.0],
    [1.0, 1.0],
];

pub fn pcf_kernel_weight_sum(samples: &[f32; 9]) -> f32 {
    samples.iter().sum::<f32>() / 9.0
}

// ---------------------------------------------------------------------------
// Stylized Lighting Evaluation
// ---------------------------------------------------------------------------

/// Evaluate stylized quantized diffuse factor from N.L.
pub fn evaluate_stylized_diffuse(n_dot_l: f32, params: &StylizedLightingParams) -> f32 {
    let half_lambert = (n_dot_l * 0.5 + 0.5).clamp(0.0, 1.0);
    if params.diffuse_bands < 2 {
        return half_lambert;
    }
    let bands = params.diffuse_bands as f32;
    let scaled = half_lambert * bands;
    let band_idx = scaled.floor();
    let frac = scaled - band_idx;
    let softness = params.band_softness.clamp(0.001, 0.5);
    let transition = ((frac - (1.0 - softness)) / softness).clamp(0.0, 1.0);
    (band_idx + transition) / bands
}

/// Evaluate Fresnel rim / silhouette contour lighting.
pub fn evaluate_rim_lighting(
    n_dot_v: f32,
    n_dot_l: f32,
    params: &StylizedLightingParams,
) -> [f32; 3] {
    if params.rim_intensity <= 0.0 {
        return [0.0; 3];
    }
    let fresnel = (1.0 - n_dot_v.clamp(0.0, 1.0)).powf(params.rim_exponent);
    let light_mask = (n_dot_l * 0.5 + 0.5).clamp(0.0, 1.0);
    let factor = fresnel * light_mask * params.rim_intensity;
    [
        params.rim_tint[0] * factor,
        params.rim_tint[1] * factor,
        params.rim_tint[2] * factor,
    ]
}

// ---------------------------------------------------------------------------
// Complete Fragment Evaluator
// ---------------------------------------------------------------------------

/// Evaluate physical or stylized fragment radiance in linear color space.
pub fn evaluate_fragment_radiance(
    surface: &SurfaceParameters,
    normal: [f32; 3],
    view_dir: [f32; 3],
    directional_light: Option<&DirectionalLight>,
    shadow_visibility: f32,
    ambient_probe_irradiance: [f32; 3],
    stylized: Option<&StylizedLightingParams>,
) -> [f32; 3] {
    let n = normalize3(normal);
    let v = normalize3(view_dir);
    let n_dot_v = dot3(n, v).clamp(1e-4, 1.0);

    let f0 = [
        0.04 * (1.0 - surface.metallic) + surface.base_color[0] * surface.metallic,
        0.04 * (1.0 - surface.metallic) + surface.base_color[1] * surface.metallic,
        0.04 * (1.0 - surface.metallic) + surface.base_color[2] * surface.metallic,
    ];

    let mut direct_accum = [0.0f32; 3];

    if let Some(sun) = directional_light {
        let l = normalize3([-sun.direction[0], -sun.direction[1], -sun.direction[2]]);
        let h = normalize3([v[0] + l[0], v[1] + l[1], v[2] + l[2]]);
        let n_dot_l = dot3(n, l);

        if n_dot_l > 0.0 {
            let n_dot_h = dot3(n, h).clamp(0.0, 1.0);
            let v_dot_h = dot3(v, h).clamp(0.0, 1.0);

            let (diffuse_factor, specular_term) = match stylized {
                Some(st) if st.diffuse_bands >= 2 => {
                    let d = evaluate_stylized_diffuse(n_dot_l, st);
                    let spec = if n_dot_h > st.specular_threshold {
                        1.0
                    } else {
                        0.0
                    };
                    (d, [spec * f0[0], spec * f0[1], spec * f0[2]])
                }
                _ => {
                    let d = n_dot_l / PI;
                    let d_term = distribution_ggx(n_dot_h, surface.roughness);
                    let v_term = visibility_smith_ggx(n_dot_v, n_dot_l, surface.roughness);
                    let f_term = fresnel_schlick(v_dot_h, f0);
                    let spec_scale = d_term * v_term;
                    (
                        d,
                        [
                            f_term[0] * spec_scale,
                            f_term[1] * spec_scale,
                            f_term[2] * spec_scale,
                        ],
                    )
                }
            };

            let illuminance = sun.illuminance_lux;
            let eff_shadow = if sun.cast_shadows {
                shadow_visibility
            } else {
                1.0
            };

            for c in 0..3 {
                let diff = (1.0 - surface.metallic) * surface.base_color[c] * diffuse_factor;
                direct_accum[c] +=
                    (diff + specular_term[c]) * sun.color_linear[c] * illuminance * eff_shadow;
            }
        }

        if let Some(st) = stylized {
            let rim = evaluate_rim_lighting(n_dot_v, n_dot_l.max(0.0), st);
            for c in 0..3 {
                direct_accum[c] += rim[c];
            }
        }
    }

    let ao = surface.ambient_occlusion.clamp(0.0, 1.0);
    let mut color = [0.0f32; 3];
    for c in 0..3 {
        let ambient = surface.base_color[c] * ambient_probe_irradiance[c] * ao;
        color[c] = direct_accum[c] + ambient + surface.emissive[c];
    }
    color
}

#[cfg(test)]
#[path = "lighting_tests.rs"]
mod tests;
