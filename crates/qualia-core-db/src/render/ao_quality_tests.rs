use super::*;
use crate::render::quality_profiles::{QualityTier, RenderQualityProfile};

#[test]
fn ao_quality_derives_correctly_from_profiles() {
    let mut profile = RenderQualityProfile {
        tier: QualityTier::High,
        render_scale_bps: 10_000,
        shadows_enabled: true,
        shadow_map_dimension: 2048,
        shadow_cascade_count: 2,
        ambient_occlusion_enabled: true,
        ao_scale_bps: 5_000,
        bloom_enabled: true,
        bloom_levels: 3,
        texture_residency_bytes: 1024 * 1024 * 1024,
        hdr_enabled: true,
        volume_projection_enabled: false,
    };

    let settings = AoQualitySettings::from_profile(&profile);
    assert_eq!(settings.tier, AoQualityTier::High);
    assert_eq!(settings.tap_count, 16);
    assert!(settings.bilateral_blur_enabled);

    // Disabled when profile has AO disabled
    profile.ambient_occlusion_enabled = false;
    let disabled = AoQualitySettings::from_profile(&profile);
    assert_eq!(disabled.tier, AoQualityTier::Disabled);
    assert_eq!(disabled.tap_count, 0);
}

#[test]
fn spiral_taps_distribute_evenly_within_unit_disk() {
    let mut taps = [([0.0, 0.0], 0.0); MAX_AO_TAPS];
    let count = generate_spiral_taps(8, 0.0, &mut taps);
    assert_eq!(count, 8);

    for (xy, r) in &taps[..count] {
        assert!(*r >= 0.0 && *r <= 1.0);
        let dist = (xy[0] * xy[0] + xy[1] * xy[1]).sqrt();
        assert!((dist - *r).abs() < 1e-4);
    }
}

#[test]
fn bilateral_edge_weight_rejects_depth_discontinuities() {
    let center_normal = [0.0, 1.0, 0.0];
    let sample_normal = [0.0, 1.0, 0.0];

    // Smooth surface (depth difference = 0)
    let w_smooth = bilateral_edge_weight(5.0, 5.0, 0.1, center_normal, sample_normal, 4.0);
    assert!((w_smooth - 1.0).abs() < 1e-4);

    // Edge discontinuity (depth difference = 1.0 >> 0.1)
    let w_edge = bilateral_edge_weight(5.0, 6.0, 0.1, center_normal, sample_normal, 4.0);
    assert!(w_edge < 0.001);
}

#[test]
fn bilateral_edge_weight_rejects_opposing_normals() {
    let center_normal = [0.0, 1.0, 0.0];
    let opposing_normal = [0.0, -1.0, 0.0];

    let w = bilateral_edge_weight(5.0, 5.0, 0.1, center_normal, opposing_normal, 4.0);
    assert_eq!(w, 0.0);
}

#[test]
fn bilateral_upsample_quad_weights_normalize_to_one() {
    let center_depth = 5.0;
    let center_normal = [0.0, 1.0, 0.0];
    let quad_depths = [5.0, 5.02, 4.98, 5.01];
    let quad_normals = [[0.0, 1.0, 0.0]; 4];
    let bilinear_weights = [0.25; 4];

    let weights = bilateral_upsample_quad_weights(
        center_depth,
        center_normal,
        quad_depths,
        quad_normals,
        bilinear_weights,
        0.1,
        4.0,
    );

    let sum: f32 = weights.iter().sum();
    assert!((sum - 1.0).abs() < 1e-4);
}
