//! Bounded portable environment lighting.
//!
//! Probes contain already-filtered irradiance and specular radiance. Evaluation only blends a
//! fixed number of caller-owned probes, so the render hot path does not allocate or depend on a
//! backend-specific texture/sampler implementation. A missing or unusable probe set fails closed
//! to the caller's direct-lighting result.

use super::{
    dot3, evaluate_stylized_diffuse, fresnel_schlick, normalize3, SurfaceParameters,
    StylizedLightingParams,
};

/// Maximum number of probes visited by one environment-lighting evaluation.
pub const MAX_ENVIRONMENT_PROBES: usize = 4;

/// Number of roughness-filtered specular levels stored per probe.
pub const ENVIRONMENT_SPECULAR_LEVELS: usize = 5;

/// A caller-owned, prefiltered environment probe.
///
/// `irradiance` is the integrated diffuse environment response. `specular_levels` are filtered
/// radiance values ordered from mirror-like (`0`) to fully rough (`4`). `dominant_direction` is
/// used only by stylized diffuse evaluation; it does not turn irradiance into a directional-light
/// substitute in the physical path.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EnvironmentProbe {
    pub position: [f32; 3],
    /// Influence radius in world units. Values outside this sphere do not contribute.
    pub radius: f32,
    pub irradiance: [f32; 3],
    pub specular_levels: [[f32; 3]; ENVIRONMENT_SPECULAR_LEVELS],
    pub dominant_direction: [f32; 3],
    /// An invalid probe is ignored without affecting deterministic probe ordering.
    pub valid: bool,
}

impl Default for EnvironmentProbe {
    fn default() -> Self {
        Self {
            position: [0.0; 3],
            radius: 0.0,
            irradiance: [0.0; 3],
            specular_levels: [[0.0; 3]; ENVIRONMENT_SPECULAR_LEVELS],
            dominant_direction: [0.0, 1.0, 0.0],
            valid: false,
        }
    }
}

/// Fixed-capacity probe storage. Construction is cold; evaluation is a bounded read-only path.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EnvironmentProbeSet {
    probes: [EnvironmentProbe; MAX_ENVIRONMENT_PROBES],
    count: u8,
}

impl Default for EnvironmentProbeSet {
    fn default() -> Self {
        Self {
            probes: [EnvironmentProbe::default(); MAX_ENVIRONMENT_PROBES],
            count: 0,
        }
    }
}

impl EnvironmentProbeSet {
    pub const fn new() -> Self {
        Self {
            probes: [EnvironmentProbe {
                position: [0.0; 3],
                radius: 0.0,
                irradiance: [0.0; 3],
                specular_levels: [[0.0; 3]; ENVIRONMENT_SPECULAR_LEVELS],
                dominant_direction: [0.0, 1.0, 0.0],
                valid: false,
            }; MAX_ENVIRONMENT_PROBES],
            count: 0,
        }
    }

    /// Append a probe, returning `false` once the deterministic capacity is full.
    pub fn push(&mut self, probe: EnvironmentProbe) -> bool {
        let index = self.count as usize;
        if index >= MAX_ENVIRONMENT_PROBES {
            return false;
        }
        self.probes[index] = probe;
        self.count += 1;
        true
    }

    pub const fn len(&self) -> usize {
        self.count as usize
    }

    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub fn as_slice(&self) -> &[EnvironmentProbe] {
        &self.probes[..self.count as usize]
    }
}

/// Result of one environment evaluation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EnvironmentLightingResult {
    /// Environment response, or the supplied direct-lighting fallback when no probe is usable.
    pub radiance: [f32; 3],
    /// Diffuse probe contribution before it is added to `specular`.
    pub diffuse: [f32; 3],
    /// Roughness-filtered specular probe contribution.
    pub specular: [f32; 3],
    pub used_probe: bool,
    pub contributing_probes: u8,
}

#[inline]
fn finite_rgb(value: [f32; 3]) -> bool {
    value.into_iter().all(f32::is_finite)
}

#[inline]
fn lerp_rgb(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

#[inline]
fn sample_specular_levels(
    levels: &[[f32; 3]; ENVIRONMENT_SPECULAR_LEVELS],
    roughness: f32,
) -> [f32; 3] {
    let scaled = roughness.clamp(0.0, 1.0) * (ENVIRONMENT_SPECULAR_LEVELS - 1) as f32;
    let lower = scaled.floor() as usize;
    let upper = (lower + 1).min(ENVIRONMENT_SPECULAR_LEVELS - 1);
    lerp_rgb(levels[lower], levels[upper], scaled - lower as f32)
}

#[inline]
fn stylized_specular_factor(n_dot_v: f32, params: &StylizedLightingParams) -> f32 {
    let softness = params.specular_softness.clamp(0.001, 0.5);
    let lower = params.specular_threshold - softness;
    let upper = params.specular_threshold + softness;
    let t = ((n_dot_v - lower) / (upper - lower)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[inline]
fn probe_weight(position: [f32; 3], probe: &EnvironmentProbe) -> f32 {
    if !probe.valid
        || probe.radius <= 0.0
        || !probe.radius.is_finite()
        || !position.into_iter().all(f32::is_finite)
        || !probe.position.into_iter().all(f32::is_finite)
    {
        return 0.0;
    }
    let delta = [
        position[0] - probe.position[0],
        position[1] - probe.position[1],
        position[2] - probe.position[2],
    ];
    let distance_sq = dot3(delta, delta);
    if !distance_sq.is_finite() {
        return 0.0;
    }
    let radius_sq = probe.radius * probe.radius;
    if distance_sq >= radius_sq {
        return 0.0;
    }
    let coverage = (1.0 - distance_sq / radius_sq).clamp(0.0, 1.0);
    // The inverse-square term keeps nearby probes authoritative; coverage provides a smooth,
    // bounded edge and avoids discontinuities when a point crosses a probe boundary.
    (1.0 / (distance_sq + 1.0)) * coverage * coverage
}

/// Evaluate bounded irradiance/specular probe lighting with an explicit direct-light fallback.
///
/// A result with `used_probe == false` is not an empty black frame: `radiance` is exactly
/// `direct_fallback`. The caller can therefore use this API before backend probe resources are
/// available without adding a second branch in its platform renderer. No allocations occur.
pub fn evaluate_environment_lighting(
    surface: &SurfaceParameters,
    position: [f32; 3],
    normal: [f32; 3],
    view_dir: [f32; 3],
    probes: Option<&EnvironmentProbeSet>,
    direct_fallback: [f32; 3],
    stylized: Option<&StylizedLightingParams>,
) -> EnvironmentLightingResult {
    let Some(probes) = probes else {
        return fallback_result(direct_fallback);
    };
    if probes.is_empty() {
        return fallback_result(direct_fallback);
    }
    if !finite_rgb(surface.base_color)
        || !finite_rgb(surface.emissive)
        || !surface.metallic.is_finite()
        || !surface.roughness.is_finite()
        || !surface.ambient_occlusion.is_finite()
    {
        return fallback_result(direct_fallback);
    }

    let n = normalize3(normal);
    let v = normalize3(view_dir);
    let n_dot_v = dot3(n, v).clamp(0.0, 1.0);
    let f0 = [
        (0.04 * (1.0 - surface.metallic) + surface.base_color[0] * surface.metallic)
            .clamp(0.0, 1.0),
        (0.04 * (1.0 - surface.metallic) + surface.base_color[1] * surface.metallic)
            .clamp(0.0, 1.0),
        (0.04 * (1.0 - surface.metallic) + surface.base_color[2] * surface.metallic)
            .clamp(0.0, 1.0),
    ];
    let ao = surface.ambient_occlusion.clamp(0.0, 1.0);
    let roughness = surface.roughness.clamp(0.0, 1.0);
    let stylized_specular = stylized.map(|params| stylized_specular_factor(n_dot_v, params));

    let mut total_weight = 0.0;
    let mut diffuse_accum = [0.0; 3];
    let mut specular_accum = [0.0; 3];
    let mut contributing_probes = 0u8;

    for probe in probes.as_slice() {
        let weight = probe_weight(position, probe);
        if weight <= 0.0 || !finite_rgb(probe.irradiance) {
            continue;
        }
        let prefiltered = sample_specular_levels(&probe.specular_levels, roughness);
        if !finite_rgb(prefiltered) {
            continue;
        }
        let dominant = normalize3(probe.dominant_direction);
        let n_dot_l = dot3(n, dominant);
        let diffuse_style = stylized
            .map(|params| evaluate_stylized_diffuse(n_dot_l, params))
            .unwrap_or(1.0);
        let fresnel = fresnel_schlick(n_dot_v, f0);
        let specular_style = stylized_specular.unwrap_or(1.0);

        for c in 0..3 {
            let diffuse = (1.0 - surface.metallic).max(0.0)
                * surface.base_color[c]
                * probe.irradiance[c]
                * (diffuse_style / std::f32::consts::PI)
                * ao;
            diffuse_accum[c] += diffuse * weight;
            specular_accum[c] += prefiltered[c] * fresnel[c] * specular_style * weight;
        }
        total_weight += weight;
        contributing_probes += 1;
    }

    if total_weight <= f32::EPSILON || !total_weight.is_finite() {
        return fallback_result(direct_fallback);
    }
    let inv_weight = 1.0 / total_weight;
    let mut diffuse = [0.0; 3];
    let mut specular = [0.0; 3];
    let mut radiance = [0.0; 3];
    for c in 0..3 {
        diffuse[c] = diffuse_accum[c] * inv_weight;
        specular[c] = specular_accum[c] * inv_weight;
        radiance[c] = diffuse[c] + specular[c] + surface.emissive[c];
    }
    EnvironmentLightingResult {
        radiance,
        diffuse,
        specular,
        used_probe: true,
        contributing_probes,
    }
}

#[inline]
fn fallback_result(direct_fallback: [f32; 3]) -> EnvironmentLightingResult {
    EnvironmentLightingResult {
        radiance: direct_fallback,
        diffuse: [0.0; 3],
        specular: [0.0; 3],
        used_probe: false,
        contributing_probes: 0,
    }
}
