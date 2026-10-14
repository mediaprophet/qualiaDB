//! Screen-Space Visual Angle Error (SSVAE), Retinal Slip & 4-Tier Virtualization Classifier.
//!
//! Grounded in human visual perception (1 arcminute = 60 Pixels Per Degree) and modern
//! computer graphics virtualized geometry systems (Nanite, Meshlets, Octahedral Impostors, 3DGS).
//!
//! # Problem Statement: The 2x2 GPU Quad Overdraw Disaster
//!
//! GPUs rasterize in $2 \times 2$ pixel quads to compute finite-difference texture coordinates
//! ($dFdx, dFdy$). When triangles project smaller than a pixel (sub-pixel triangles), the entire
//! quad is still shaded, discarding up to 75–90% of computed fragments. Furthermore, dense vertex
//! transformations choke the geometry pipe.
//!
//! # Solution: 4-Tier Decoupled Representation
//!
//! Geometry is virtualized and selected dynamically based on effective screen-space radius $R_{\text{eff}}$:
//!
//! - **Tier 0: Hero Mesh** ($R_{\text{eff}} \ge 256$ px): Full-resolution mesh or fine displacement.
//! - **Tier 1: Meshlet Cluster** ($64 \le R_{\text{eff}} < 256$ px): Fixed-size clusters (~128 tris) with cone culling.
//! - **Tier 2: Octahedral Impostor** ($8 \le R_{\text{eff}} < 64$ px): 2-triangle quad sampling `.10d` Section 19 (`IMPO`).
//! - **Tier 3: Hierarchical Surfel / Splat** ($1 \le R_{\text{eff}} < 8$ px): Point/disk primitive with scale and normal.
//! - **Tier 4: Subpixel Culled** ($R_{\text{eff}} < 1$ px): Below Nyquist limit, culled to protect rasterizer.
//!
//! # Retinal Slip & Contrast Sensitivity
//!
//! Human visual acuity drops sharply under angular motion (retinal slip). When the camera or object
//! rotates rapidly, high spatial frequencies are imperceptible. The engine calculates:
//! $$\text{PPD}_{\text{eff}} = \frac{\text{PPD}_{\text{static}}}{1.0 + k_{\text{slip}} \cdot \|\omega\|}$$
//! and scales the effective screen radius, seamlessly promoting objects into cheaper impostor or
//! surfel tiers during fast motion.

pub const STATIC_HUMAN_PPD: f32 = 60.0;
pub const DEFAULT_RETINAL_SLIP_COEFF: f32 = 0.05; // Drop factor per deg/s
pub const SUBPIXEL_CULL_THRESHOLD_PX: f32 = 1.0;

/// The 4-tier virtualized geometry classification.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VirtualizationTier {
    /// Full-detail hero mesh ($R_{\text{eff}} \ge 256$ px).
    HeroMesh = 0,
    /// Pre-computed cluster meshlets of ~128 triangles ($64 \le R_{\text{eff}} < 256$ px).
    MeshletCluster = 1,
    /// 2-triangle camera-facing quad sampling Hemi-Octahedral atlas ($8 \le R_{\text{eff}} < 64$ px).
    OctahedralImpostor = 2,
    /// Hierarchical oriented surfel / 3D Gaussian splat ($1 \le R_{\text{eff}} < 8$ px).
    HierarchicalSurfel = 3,
    /// Sub-pixel culled ($R_{\text{eff}} < 1$ px).
    SubpixelCulled = 4,
}

/// Evaluation metrics produced for an instance or cluster during culling/LOD selection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenSpaceMetric {
    /// Geometric projected screen radius in pixels without motion blur.
    pub raw_screen_radius_px: f32,
    /// Motion-compensated effective screen radius in pixels.
    pub effective_screen_radius_px: f32,
    /// Visual angle subtended by bounding sphere in degrees.
    pub visual_angle_deg: f32,
    /// Motion-degraded effective Pixels Per Degree (PPD).
    pub effective_ppd: f32,
    /// Acuity retention factor $[0.0, 1.0]$.
    pub acuity_factor: f32,
    /// Recommended virtualization tier.
    pub tier: VirtualizationTier,
    /// Estimated GPU quad rasterization efficiency $[0.0, 1.0]$ if rendered as raw triangles.
    pub estimated_quad_efficiency: f32,
}

/// Calculate projected screen radius of a bounding sphere. Zero heap.
///
/// $$R_{\text{screen}} = \frac{r \cdot H_{\text{screen}}}{2 \cdot z \cdot \tan(\text{FOV}_y / 2)}$$
#[inline]
pub fn calculate_screen_radius_px(
    sphere_center_world: [f32; 3],
    sphere_radius_world: f32,
    camera_pos_world: [f32; 3],
    camera_forward_world: [f32; 3],
    fov_y_rad: f32,
    screen_height_px: f32,
) -> f32 {
    let to_center = [
        sphere_center_world[0] - camera_pos_world[0],
        sphere_center_world[1] - camera_pos_world[1],
        sphere_center_world[2] - camera_pos_world[2],
    ];
    // Longitudinal depth along camera forward axis
    let z = to_center[0] * camera_forward_world[0]
        + to_center[1] * camera_forward_world[1]
        + to_center[2] * camera_forward_world[2];

    if z <= 0.001 {
        // Behind or intersecting camera plane: fail open with full screen height
        return screen_height_px;
    }

    let tan_half_fov = (fov_y_rad * 0.5).tan();
    if tan_half_fov <= 1e-6 {
        return screen_height_px;
    }

    let radius_px = (sphere_radius_world * screen_height_px) / (2.0 * z * tan_half_fov);
    radius_px.max(0.0)
}

/// Calculate effective PPD and acuity retention factor under angular retinal slip.
///
/// Angular speed $\omega$ is in radians/second.
#[inline]
pub fn calculate_retinal_slip_acuity(
    angular_speed_rad_s: f32,
    slip_coeff: f32,
) -> (f32, f32) {
    let speed_deg_s = angular_speed_rad_s.abs() * (180.0 / std::f32::consts::PI);
    let effective_ppd = STATIC_HUMAN_PPD / (1.0 + slip_coeff * speed_deg_s);
    let acuity_factor = (effective_ppd / STATIC_HUMAN_PPD).clamp(0.01, 1.0);
    (effective_ppd, acuity_factor)
}

/// Classify an entity or cluster into its optimal virtualization tier.
///
/// Operates in zero-heap with pure arithmetic.
pub fn classify_screen_metric(
    sphere_center_world: [f32; 3],
    sphere_radius_world: f32,
    camera_pos_world: [f32; 3],
    camera_forward_world: [f32; 3],
    fov_y_rad: f32,
    screen_height_px: f32,
    camera_angular_speed_rad_s: f32,
    slip_coeff: f32,
) -> ScreenSpaceMetric {
    let raw_radius_px = calculate_screen_radius_px(
        sphere_center_world,
        sphere_radius_world,
        camera_pos_world,
        camera_forward_world,
        fov_y_rad,
        screen_height_px,
    );

    let (effective_ppd, acuity_factor) =
        calculate_retinal_slip_acuity(camera_angular_speed_rad_s, slip_coeff);

    let effective_radius_px = raw_radius_px * acuity_factor;

    // Subtended visual angle in degrees: 2 * arctan(r / dist)
    let to_center = [
        sphere_center_world[0] - camera_pos_world[0],
        sphere_center_world[1] - camera_pos_world[1],
        sphere_center_world[2] - camera_pos_world[2],
    ];
    let dist = (to_center[0] * to_center[0]
        + to_center[1] * to_center[1]
        + to_center[2] * to_center[2])
        .sqrt()
        .max(1e-4);
    let visual_angle_deg = (2.0 * (sphere_radius_world / dist).atan()) * (180.0 / std::f32::consts::PI);

    let tier = if effective_radius_px < SUBPIXEL_CULL_THRESHOLD_PX {
        VirtualizationTier::SubpixelCulled
    } else if effective_radius_px < 8.0 {
        VirtualizationTier::HierarchicalSurfel
    } else if effective_radius_px < 64.0 {
        VirtualizationTier::OctahedralImpostor
    } else if effective_radius_px < 256.0 {
        VirtualizationTier::MeshletCluster
    } else {
        VirtualizationTier::HeroMesh
    };

    // Estimated quad efficiency if rendered as a triangle mesh:
    // If diameter < 4 px, each triangle occupies << 1 quad, resulting in heavy waste.
    let diameter_px = raw_radius_px * 2.0;
    let estimated_quad_efficiency = if diameter_px >= 32.0 {
        1.0
    } else if diameter_px <= 1.0 {
        0.05
    } else {
        (diameter_px / 32.0).clamp(0.05, 1.0)
    };

    ScreenSpaceMetric {
        raw_screen_radius_px: raw_radius_px,
        effective_screen_radius_px: effective_radius_px,
        visual_angle_deg,
        effective_ppd,
        acuity_factor,
        tier,
        estimated_quad_efficiency,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hero_mesh_classification() {
        // Large object close to camera -> HeroMesh
        let metric = classify_screen_metric(
            [0.0, 0.0, 5.0],
            2.0,
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            std::f32::consts::FRAC_PI_4, // 45 deg FOV
            1080.0,
            0.0,
            DEFAULT_RETINAL_SLIP_COEFF,
        );
        assert_eq!(metric.tier, VirtualizationTier::HeroMesh);
        assert!(metric.raw_screen_radius_px >= 256.0);
        assert_eq!(metric.acuity_factor, 1.0);
    }

    #[test]
    fn test_impostor_tier_classification() {
        // Object far away -> OctahedralImpostor
        let metric = classify_screen_metric(
            [0.0, 0.0, 100.0],
            1.5,
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            std::f32::consts::FRAC_PI_4,
            1080.0,
            0.0,
            DEFAULT_RETINAL_SLIP_COEFF,
        );
        assert_eq!(metric.tier, VirtualizationTier::OctahedralImpostor);
        assert!(metric.raw_screen_radius_px >= 8.0 && metric.raw_screen_radius_px < 64.0);
    }

    #[test]
    fn test_surfel_tier_classification() {
        // Object very far away -> HierarchicalSurfel
        let metric = classify_screen_metric(
            [0.0, 0.0, 400.0],
            1.0,
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            std::f32::consts::FRAC_PI_4,
            1080.0,
            0.0,
            DEFAULT_RETINAL_SLIP_COEFF,
        );
        assert_eq!(metric.tier, VirtualizationTier::HierarchicalSurfel);
        assert!(metric.raw_screen_radius_px >= 1.0 && metric.raw_screen_radius_px < 8.0);
    }

    #[test]
    fn test_retinal_slip_motion_demotion() {
        // Stationary: would be MeshletCluster (e.g. 100 px)
        let static_metric = classify_screen_metric(
            [0.0, 0.0, 25.0],
            1.0,
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            std::f32::consts::FRAC_PI_4,
            1080.0,
            0.0,
            DEFAULT_RETINAL_SLIP_COEFF,
        );
        assert_eq!(static_metric.tier, VirtualizationTier::MeshletCluster);

        // High camera angular speed (e.g. 5 rad/s ~ 286 deg/s whip pan):
        // Acuity drops heavily -> demoted to OctahedralImpostor!
        let motion_metric = classify_screen_metric(
            [0.0, 0.0, 25.0],
            1.0,
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            std::f32::consts::FRAC_PI_4,
            1080.0,
            5.0,
            DEFAULT_RETINAL_SLIP_COEFF,
        );
        assert!(motion_metric.acuity_factor < 0.2);
        assert_eq!(motion_metric.tier, VirtualizationTier::OctahedralImpostor);
    }
}
