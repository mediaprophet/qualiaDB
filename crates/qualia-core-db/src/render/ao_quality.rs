//! Bounded screen-space ambient occlusion quality profiles and bilateral filtering.
//!
//! Provides deterministic sampling tap distributions, bilateral depth/normal-aware edge weights
//! to prevent halos, and reconstruction upsampling filters for lower-resolution AO passes.
//! Zero-heap execution contract.

use super::quality_profiles::{QualityTier, RenderQualityProfile};
use std::f32::consts::PI;

/// Maximum number of AO sampling taps per pixel.
pub const MAX_AO_TAPS: usize = 16;

/// Explicit AO quality tiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AoQualityTier {
    Disabled = 0,
    Low = 1,    // Quarter-resolution, 4 taps
    Medium = 2, // Half-resolution, 8 taps
    High = 3,   // Half-resolution, 16 taps + bilateral blur
    Ultra = 4,  // Full-resolution, 16 taps + bilateral blur
}

/// Settings governing AO evaluation and bilateral reconstruction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AoQualitySettings {
    pub tier: AoQualityTier,
    pub tap_count: u32,
    pub radius_world: f32,
    pub bias: f32,
    pub intensity: f32,
    pub depth_threshold: f32,
    pub normal_power: f32,
    pub bilateral_blur_enabled: bool,
}

impl AoQualitySettings {
    /// Derive AO settings deterministically from a RenderQualityProfile.
    pub fn from_profile(profile: &RenderQualityProfile) -> Self {
        if !profile.ambient_occlusion_enabled {
            return Self::disabled();
        }
        match profile.tier {
            QualityTier::Conservative => Self::disabled(),
            QualityTier::Low => Self {
                tier: AoQualityTier::Low,
                tap_count: 4,
                radius_world: 0.5,
                bias: 0.05,
                intensity: 1.0,
                depth_threshold: 0.1,
                normal_power: 4.0,
                bilateral_blur_enabled: false,
            },
            QualityTier::Balanced => Self {
                tier: AoQualityTier::Medium,
                tap_count: 8,
                radius_world: 0.75,
                bias: 0.03,
                intensity: 1.2,
                depth_threshold: 0.08,
                normal_power: 6.0,
                bilateral_blur_enabled: false,
            },
            QualityTier::High => Self {
                tier: AoQualityTier::High,
                tap_count: 16,
                radius_world: 1.0,
                bias: 0.02,
                intensity: 1.5,
                depth_threshold: 0.05,
                normal_power: 8.0,
                bilateral_blur_enabled: true,
            },
            QualityTier::Ultra => Self {
                tier: AoQualityTier::Ultra,
                tap_count: 16,
                radius_world: 1.2,
                bias: 0.015,
                intensity: 1.6,
                depth_threshold: 0.03,
                normal_power: 12.0,
                bilateral_blur_enabled: true,
            },
        }
    }

    pub const fn disabled() -> Self {
        Self {
            tier: AoQualityTier::Disabled,
            tap_count: 0,
            radius_world: 0.0,
            bias: 0.0,
            intensity: 0.0,
            depth_threshold: 0.0,
            normal_power: 1.0,
            bilateral_blur_enabled: false,
        }
    }
}

/// Compute deterministic Fibonacci spiral sample offsets in disk [-1.0, 1.0].
/// Generates (normalized_offset_xy, distance) per tap.
pub fn generate_spiral_taps(
    tap_count: usize,
    rotation_rad: f32,
    out: &mut [([f32; 2], f32); MAX_AO_TAPS],
) -> usize {
    let count = tap_count.min(MAX_AO_TAPS);
    let golden_angle = PI * (3.0 - (5.0f32).sqrt());

    for i in 0..count {
        let t = (i as f32 + 0.5) / (count as f32);
        let radius = t.sqrt();
        let angle = i as f32 * golden_angle + rotation_rad;
        let x = radius * angle.cos();
        let y = radius * angle.sin();
        out[i] = ([x, y], radius);
    }
    count
}

/// Bilateral weight for depth and normal edge preservation.
///
/// Rejects samples across geometric silhouettes (depth discontinuities)
/// and across sharp surface creases (divergent normals) to prevent haloing.
#[inline]
pub fn bilateral_edge_weight(
    center_depth: f32,
    sample_depth: f32,
    depth_threshold: f32,
    center_normal: [f32; 3],
    sample_normal: [f32; 3],
    normal_power: f32,
) -> f32 {
    let depth_diff = (center_depth - sample_depth).abs();
    let depth_weight = (-depth_diff / depth_threshold.max(1e-4)).exp();

    let normal_dot = center_normal[0] * sample_normal[0]
        + center_normal[1] * sample_normal[1]
        + center_normal[2] * sample_normal[2];
    let normal_weight = normal_dot.max(0.0).powf(normal_power);

    depth_weight * normal_weight
}

/// Bilateral 2x2 reconstruction weights for upsampling lower-resolution AO.
///
/// Returns 4 weights for the bilinear quad [top-left, top-right, bottom-left, bottom-right]
/// normalized so their sum is 1.0. If all samples are across a geometric edge,
/// falls back to the nearest depth sample.
pub fn bilateral_upsample_quad_weights(
    center_depth: f32,
    center_normal: [f32; 3],
    quad_depths: [f32; 4],
    quad_normals: [[f32; 3]; 4],
    bilinear_weights: [f32; 4],
    depth_threshold: f32,
    normal_power: f32,
) -> [f32; 4] {
    let mut weights = [0.0f32; 4];
    let mut total_weight = 0.0f32;

    for i in 0..4 {
        let edge_w = bilateral_edge_weight(
            center_depth,
            quad_depths[i],
            depth_threshold,
            center_normal,
            quad_normals[i],
            normal_power,
        );
        let w = bilinear_weights[i] * edge_w;
        weights[i] = w;
        total_weight += w;
    }

    if total_weight > 1e-6 {
        let inv = 1.0 / total_weight;
        [
            weights[0] * inv,
            weights[1] * inv,
            weights[2] * inv,
            weights[3] * inv,
        ]
    } else {
        // Fallback: pick the closest depth sample
        let mut min_diff = f32::MAX;
        let mut best_idx = 0;
        for i in 0..4 {
            let diff = (center_depth - quad_depths[i]).abs();
            if diff < min_diff {
                min_diff = diff;
                best_idx = i;
            }
        }
        let mut fallback = [0.0; 4];
        fallback[best_idx] = 1.0;
        fallback
    }
}

#[cfg(test)]
#[path = "ao_quality_tests.rs"]
mod tests;
