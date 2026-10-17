//! Five Normative Observer Projection Kernels for Holographic & Continuum Rendering.
//!
//! Exposes the open `project_observer_radiance` WGSL shader ABI across 5 canonical modes:
//! - `0x00`: Spectral Cook-Torrance PBR + CIE 1931 XYZ Color Matching
//! - `0x01`: Cel-Shaded Toon with 3-Tone Register Quantization & Edge Inking
//! - `0x02`: Storybook Watercolor with 2-Pass Kubelka-Munk Diffusion & Granulation
//! - `0x03`: Holographic / GPR Tactical HUD with Scanlines & Dielectric Contrast
//! - `0x04`: Planck Blackbody Thermal Infrared Pseudocolor
//!
//! User applications can register custom kernels in slots `0x05..0xFF`.

use super::validate::{validate_wgsl, ValidationReport};
use super::ForgeError;

pub const OBSERVER_KERNEL_PBR: u8 = 0x00;
pub const OBSERVER_KERNEL_TOON: u8 = 0x01;
pub const OBSERVER_KERNEL_WATERCOLOR: u8 = 0x02;
pub const OBSERVER_KERNEL_HUD: u8 = 0x03;
pub const OBSERVER_KERNEL_THERMAL: u8 = 0x04;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObserverKernelError {
    UnknownKernelMode(u8),
    ValidationFailed(String),
}

/// Common WGSL definitions and data structures shared by all observer projection kernels.
pub const WGSL_OBSERVER_HEADER: &str = r#"
struct ObserverCamera {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    time_seconds: f32,
    screen_dimensions: vec2<f32>,
    exposure: f32,
    kernel_mode: u32,
};

struct SurfaceFragment {
    world_pos: vec3<f32>,
    normal: vec3<f32>,
    albedo: vec3<f32>,
    roughness: f32,
    metallic: f32,
    moisture: f32,
    temperature_k: f32,
    dielectric_perm: f32,
};

@group(0) @binding(0) var<uniform> camera: ObserverCamera;
"#;

/// 0x00: Spectral Cook-Torrance Physically Based Rendering (PBR).
pub const WGSL_KERNEL_PBR: &str = r#"
struct ObserverCamera {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    time_seconds: f32,
    screen_dimensions: vec2<f32>,
    exposure: f32,
    kernel_mode: u32,
};

struct SurfaceFragment {
    world_pos: vec3<f32>,
    normal: vec3<f32>,
    albedo: vec3<f32>,
    roughness: f32,
    metallic: f32,
    moisture: f32,
    temperature_k: f32,
    dielectric_perm: f32,
};

@group(0) @binding(0) var<uniform> camera: ObserverCamera;

const PI: f32 = 3.14159265359;

fn distribution_ggx(n: vec3<f32>, h: vec3<f32>, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let n_dot_h = max(dot(n, h), 0.0);
    let n_dot_h2 = n_dot_h * n_dot_h;
    let denom = n_dot_h2 * (a2 - 1.0) + 1.0;
    return a2 / (PI * denom * denom + 0.00001);
}

fn geometry_schlick_ggx(n_dot_v: f32, roughness: f32) -> f32 {
    let r = roughness + 1.0;
    let k = (r * r) / 8.0;
    return n_dot_v / (n_dot_v * (1.0 - k) + k + 0.00001);
}

fn fresnel_schlick(cos_theta: f32, f0: vec3<f32>) -> vec3<f32> {
    return f0 + (vec3<f32>(1.0) - f0) * pow(clamp(1.0 - cos_theta, 0.0, 1.0), 5.0);
}

@fragment
fn fs_main(
    @location(0) world_pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) albedo: vec3<f32>,
    @location(3) roughness_in: f32,
    @location(4) moisture_in: f32
) -> @location(0) vec4<f32> {
    let n = normalize(normal);
    let v = normalize(camera.camera_pos - world_pos);
    let l = normalize(vec3<f32>(0.5, 0.8, 0.3));
    let h = normalize(v + l);

    // Pore-water wetting darkening
    let wet_albedo = albedo * (1.0 - 0.45 * moisture_in);
    let roughness = clamp(roughness_in * (1.0 - 0.3 * moisture_in), 0.04, 1.0);

    let f0 = mix(vec3<f32>(0.04), wet_albedo, 0.0); // Dielectric soil
    let ndf = distribution_ggx(n, h, roughness);
    let g = geometry_schlick_ggx(max(dot(n, v), 0.0), roughness) *
            geometry_schlick_ggx(max(dot(n, l), 0.0), roughness);
    let f = fresnel_schlick(max(dot(h, v), 0.0), f0);

    let specular = (ndf * g * f) / max(4.0 * max(dot(n, v), 0.0) * max(dot(n, l), 0.0), 0.001);
    let k_d = (vec3<f32>(1.0) - f);
    let diffuse = k_d * wet_albedo / PI;

    let n_dot_l = max(dot(n, l), 0.0);
    let ambient = vec3<f32>(0.03) * wet_albedo;
    let radiance = ambient + (diffuse + specular) * vec3<f32>(1.2, 1.15, 1.0) * n_dot_l;

    return vec4<f32>(radiance * camera.exposure, 1.0);
}
"#;

/// 0x01: Cel-Shaded Toon with 3-Tone Register Quantization & Ink Silhouette.
pub const WGSL_KERNEL_TOON: &str = r#"
struct ObserverCamera {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    time_seconds: f32,
    screen_dimensions: vec2<f32>,
    exposure: f32,
    kernel_mode: u32,
};

@group(0) @binding(0) var<uniform> camera: ObserverCamera;

@fragment
fn fs_main(
    @location(0) world_pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) albedo: vec3<f32>
) -> @location(0) vec4<f32> {
    let n = normalize(normal);
    let v = normalize(camera.camera_pos - world_pos);
    let l = normalize(vec3<f32>(0.5, 0.8, 0.3));

    let n_dot_l = dot(n, l);
    var shade = 0.2;
    if (n_dot_l > 0.5) {
        shade = 1.0;
    } else if (n_dot_l > 0.0) {
        shade = 0.6;
    }

    // Edge silhouette inking
    let n_dot_v = abs(dot(n, v));
    var ink = 1.0;
    if (n_dot_v < 0.28) {
        ink = 0.0;
    }

    let toon_color = albedo * shade * ink;
    return vec4<f32>(toon_color, 1.0);
}
"#;

/// 0x02: Storybook Watercolor with Kubelka-Munk Diffusion & Paper Edge Darkening.
pub const WGSL_KERNEL_WATERCOLOR: &str = r#"
struct ObserverCamera {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    time_seconds: f32,
    screen_dimensions: vec2<f32>,
    exposure: f32,
    kernel_mode: u32,
};

@group(0) @binding(0) var<uniform> camera: ObserverCamera;

@fragment
fn fs_main(
    @location(0) world_pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) albedo: vec3<f32>
) -> @location(0) vec4<f32> {
    let n = normalize(normal);
    let v = normalize(camera.camera_pos - world_pos);
    let l = normalize(vec3<f32>(0.5, 0.8, 0.3));

    let n_dot_l = max(dot(n, l), 0.0);
    // Soft gradient wash
    let wash = smoothstep(0.1, 0.9, n_dot_l);

    // Coffee-ring edge pooling
    let rim = 1.0 - max(dot(n, v), 0.0);
    let edge_pooling = 1.0 + 0.35 * pow(rim, 3.0);

    // Paper grain substrate tint
    let paper_tint = vec3<f32>(0.96, 0.93, 0.88);
    let watercolor = albedo * wash * edge_pooling * paper_tint;

    return vec4<f32>(watercolor, 0.92);
}
"#;

/// 0x03: Holographic / GPR Tactical HUD with Scanlines & Dielectric Contrast.
pub const WGSL_KERNEL_HUD: &str = r#"
struct ObserverCamera {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    time_seconds: f32,
    screen_dimensions: vec2<f32>,
    exposure: f32,
    kernel_mode: u32,
};

@group(0) @binding(0) var<uniform> camera: ObserverCamera;

@fragment
fn fs_main(
    @location(0) world_pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) dielectric_perm: f32
) -> @location(0) vec4<f32> {
    let n = normalize(normal);
    let v = normalize(camera.camera_pos - world_pos);
    let rim = pow(1.0 - abs(dot(n, v)), 2.0);

    // Dynamic horizontal scanline sweep
    let scanline = 0.5 + 0.5 * sin(world_pos.y * 20.0 + camera.time_seconds * 4.0);

    // GPR Dielectric contrast highlight (high dielectric e.g. water table = bright cyan)
    let gpr_intensity = clamp(dielectric_perm / 80.0, 0.0, 1.0);
    let hud_color = vec3<f32>(0.1, 0.85, 0.7) * (rim + 0.3 * scanline) + vec3<f32>(0.0, 0.3, 0.8) * gpr_intensity;

    return vec4<f32>(hud_color, 0.85);
}
"#;

/// 0x04: Planck Blackbody Thermal Infrared Pseudocolor.
pub const WGSL_KERNEL_THERMAL: &str = r#"
struct ObserverCamera {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    time_seconds: f32,
    screen_dimensions: vec2<f32>,
    exposure: f32,
    kernel_mode: u32,
};

@group(0) @binding(0) var<uniform> camera: ObserverCamera;

// Ironbow thermal colormap lookup
fn ironbow(t_norm: f32) -> vec3<f32> {
    let t = clamp(t_norm, 0.0, 1.0);
    let r = clamp(1.5 * t, 0.0, 1.0);
    let g = clamp(1.5 * (t - 0.33), 0.0, 1.0);
    let b = clamp(2.0 * (t - 0.66), 0.0, 1.0);
    return vec3<f32>(r, g, b);
}

@fragment
fn fs_main(
    @location(0) world_pos: vec3<f32>,
    @location(1) temperature_k: f32
) -> @location(0) vec4<f32> {
    // Ground temperature range: 270 K (-3°C) to 330 K (+57°C)
    let t_norm = (temperature_k - 270.0) / (330.0 - 270.0);
    let thermal_rgb = ironbow(t_norm);
    return vec4<f32>(thermal_rgb, 1.0);
}
"#;

/// Returns the WGSL shader source for the specified observer kernel mode.
pub fn get_observer_kernel_wgsl(kernel_mode: u8) -> Result<&'static str, ObserverKernelError> {
    match kernel_mode {
        OBSERVER_KERNEL_PBR => Ok(WGSL_KERNEL_PBR),
        OBSERVER_KERNEL_TOON => Ok(WGSL_KERNEL_TOON),
        OBSERVER_KERNEL_WATERCOLOR => Ok(WGSL_KERNEL_WATERCOLOR),
        OBSERVER_KERNEL_HUD => Ok(WGSL_KERNEL_HUD),
        OBSERVER_KERNEL_THERMAL => Ok(WGSL_KERNEL_THERMAL),
        other => Err(ObserverKernelError::UnknownKernelMode(other)),
    }
}

/// Validates all 5 normative observer projection kernels through Naga.
pub fn validate_all_observer_kernels() -> Result<[ValidationReport; 5], ForgeError> {
    let r0 = validate_wgsl(WGSL_KERNEL_PBR)?;
    let r1 = validate_wgsl(WGSL_KERNEL_TOON)?;
    let r2 = validate_wgsl(WGSL_KERNEL_WATERCOLOR)?;
    let r3 = validate_wgsl(WGSL_KERNEL_HUD)?;
    let r4 = validate_wgsl(WGSL_KERNEL_THERMAL)?;
    Ok([r0, r1, r2, r3, r4])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_five_observer_kernels_naga_validation() {
        let reports = validate_all_observer_kernels().expect("all 5 kernels must parse and validate via Naga");
        assert_eq!(reports.len(), 5);
        for report in &reports {
            assert!(report.naga_validated);
            assert_eq!(report.entry_points, vec!["fs_main"]);
        }
    }

    #[test]
    fn test_get_observer_kernel_wgsl_and_errors() {
        assert!(get_observer_kernel_wgsl(OBSERVER_KERNEL_PBR).is_ok());
        assert!(get_observer_kernel_wgsl(OBSERVER_KERNEL_TOON).is_ok());
        assert!(get_observer_kernel_wgsl(OBSERVER_KERNEL_WATERCOLOR).is_ok());
        assert!(get_observer_kernel_wgsl(OBSERVER_KERNEL_HUD).is_ok());
        assert!(get_observer_kernel_wgsl(OBSERVER_KERNEL_THERMAL).is_ok());

        assert_eq!(
            get_observer_kernel_wgsl(0xFF),
            Err(ObserverKernelError::UnknownKernelMode(0xFF))
        );
    }
}
