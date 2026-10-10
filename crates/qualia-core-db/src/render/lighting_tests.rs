use super::*;

#[test]
fn distance_attenuation_smoothly_falls_to_zero_at_radius() {
    let radius = 10.0;
    let at_origin = distance_attenuation(0.0, radius);
    assert!((at_origin - 1.0).abs() < 1e-4);

    let at_half = distance_attenuation(25.0, radius);
    assert!(at_half > 0.0 && at_half < at_origin);

    let at_radius = distance_attenuation(100.0, radius);
    assert_eq!(at_radius, 0.0);

    let beyond_radius = distance_attenuation(200.0, radius);
    assert_eq!(beyond_radius, 0.0);
}

#[test]
fn spot_angular_attenuation_respects_cone_boundaries() {
    let cos_inner = 0.9;
    let cos_outer = 0.7;

    // Inside inner cone -> full intensity
    let inside = spot_angular_attenuation(0.95, cos_inner, cos_outer);
    assert_eq!(inside, 1.0);

    // Outside outer cone -> zero intensity
    let outside = spot_angular_attenuation(0.65, cos_inner, cos_outer);
    assert_eq!(outside, 0.0);

    // Mid-cone -> intermediate smooth value
    let mid = spot_angular_attenuation(0.8, cos_inner, cos_outer);
    assert!(mid > 0.0 && mid < 1.0);
}

#[test]
fn pcf_kernel_weight_averages_correctly() {
    let all_lit = [1.0; 9];
    assert_eq!(pcf_kernel_weight_sum(&all_lit), 1.0);

    let half_shadow = [1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
    assert!((pcf_kernel_weight_sum(&half_shadow) - (3.0 / 9.0)).abs() < 1e-6);
}

#[test]
fn stylized_diffuse_bands_quantize_smoothly() {
    let params = StylizedLightingParams {
        diffuse_bands: 2,
        band_softness: 0.01,
        ..Default::default()
    };

    // Low N.L should map to lower band
    let dark = evaluate_stylized_diffuse(-0.5, &params);
    assert!(dark <= 0.5);

    // High N.L should map to upper band
    let bright = evaluate_stylized_diffuse(0.9, &params);
    assert!(bright >= 0.5);
}

#[test]
fn stylized_rim_lighting_activates_at_glancing_angles() {
    let params = StylizedLightingParams {
        rim_intensity: 1.5,
        rim_exponent: 2.0,
        rim_tint: [1.0, 0.5, 0.2],
        ..Default::default()
    };

    // Direct view (n_dot_v = 1.0) -> zero rim
    let center = evaluate_rim_lighting(1.0, 1.0, &params);
    assert_eq!(center, [0.0; 3]);

    // Silhouette edge (n_dot_v = 0.0) -> full rim
    let edge = evaluate_rim_lighting(0.0, 1.0, &params);
    assert!((edge[0] - 1.5).abs() < 1e-4);
    assert!((edge[1] - 0.75).abs() < 1e-4);
}

#[test]
fn pbr_fragment_radiance_conserves_energy_and_scales_with_shadow() {
    let surface = SurfaceParameters {
        base_color: [0.8, 0.8, 0.8],
        metallic: 0.0,
        roughness: 0.3,
        reflectance_f0: 0.04,
        emissive: [0.0; 3],
        ambient_occlusion: 1.0,
    };
    let sun = DirectionalLight {
        direction: [0.0, -1.0, 0.0],
        illuminance_lux: 10.0,
        color_linear: [1.0, 1.0, 1.0],
        cast_shadows: true,
    };

    let lit = evaluate_fragment_radiance(
        &surface,
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 1.0],
        Some(&sun),
        1.0,
        [0.0; 3],
        None,
    );
    assert!(lit[0] > 0.0);

    let shadowed = evaluate_fragment_radiance(
        &surface,
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 1.0],
        Some(&sun),
        0.0,
        [0.0; 3],
        None,
    );
    assert_eq!(shadowed, [0.0; 3]);
}

fn test_surface() -> SurfaceParameters {
    SurfaceParameters {
        base_color: [0.8, 0.6, 0.4],
        metallic: 0.0,
        roughness: 0.25,
        reflectance_f0: 0.04,
        emissive: [0.0; 3],
        ambient_occlusion: 1.0,
    }
}

fn test_probe() -> EnvironmentProbe {
    EnvironmentProbe {
        position: [0.0, 0.0, 0.0],
        radius: 10.0,
        irradiance: [2.0, 2.0, 2.0],
        specular_levels: [
            [4.0, 4.0, 4.0],
            [3.0, 3.0, 3.0],
            [2.0, 2.0, 2.0],
            [1.0, 1.0, 1.0],
            [0.5, 0.5, 0.5],
        ],
        dominant_direction: [0.0, 1.0, 0.0],
        valid: true,
    }
}

#[test]
fn environment_lighting_falls_back_to_direct_when_probes_are_unavailable() {
    let surface = test_surface();
    let direct = [0.25, 0.5, 0.75];
    let result = evaluate_environment_lighting(
        &surface,
        [0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 1.0],
        None,
        direct,
        None,
    );
    assert_eq!(result.radiance, direct);
    assert!(!result.used_probe);
    assert_eq!(result.contributing_probes, 0);

    let empty = EnvironmentProbeSet::new();
    let result = evaluate_environment_lighting(
        &surface,
        [0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 1.0],
        Some(&empty),
        direct,
        None,
    );
    assert_eq!(result.radiance, direct);
    assert!(!result.used_probe);
}

#[test]
fn environment_lighting_is_bounded_and_roughness_aware() {
    let surface = test_surface();
    let mut probes = EnvironmentProbeSet::new();
    assert!(probes.push(test_probe()));
    for _ in 1..MAX_ENVIRONMENT_PROBES {
        assert!(probes.push(test_probe()));
    }
    assert!(!probes.push(test_probe()));
    assert_eq!(probes.len(), MAX_ENVIRONMENT_PROBES);

    let smooth = evaluate_environment_lighting(
        &surface,
        [0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 1.0],
        Some(&probes),
        [0.0; 3],
        None,
    );
    let rough_surface = SurfaceParameters {
        roughness: 1.0,
        ..surface
    };
    let rough = evaluate_environment_lighting(
        &rough_surface,
        [0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 1.0],
        Some(&probes),
        [0.0; 3],
        None,
    );
    assert!(smooth.used_probe && rough.used_probe);
    assert_eq!(smooth.contributing_probes as usize, MAX_ENVIRONMENT_PROBES);
    assert!(smooth.specular[0] > rough.specular[0]);
    assert_eq!(smooth.diffuse, rough.diffuse);
}

#[test]
fn environment_lighting_keeps_stylized_diffuse_and_specular_controls() {
    let surface = test_surface();
    let mut probes = EnvironmentProbeSet::new();
    assert!(probes.push(test_probe()));
    let stylized = StylizedLightingParams {
        diffuse_bands: 2,
        band_softness: 0.01,
        specular_threshold: 0.99,
        specular_softness: 0.001,
        ..Default::default()
    };
    let result = evaluate_environment_lighting(
        &surface,
        [0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.1, 1.0],
        Some(&probes),
        [0.0; 3],
        Some(&stylized),
    );
    assert!(result.used_probe);
    assert!(result.diffuse[0] >= 0.0);
    assert_eq!(result.specular, [0.0; 3]);
}

#[test]
fn environment_gpu_contract_packs_source_probes_and_falls_back_without_them() {
    assert_eq!(
        std::mem::size_of::<EnvironmentLightingGpu>(),
        ENVIRONMENT_LIGHTING_GPU_SIZE
    );
    let fallback = EnvironmentLightingGpu::fallback([0.25, 0.5, 0.75]);
    assert!(!fallback.has_usable_probes());
    assert_eq!(fallback.metadata[0], 0);
    assert_eq!(fallback.direct_fallback[..3], [0.25, 0.5, 0.75]);

    let mut probes = EnvironmentProbeSet::new();
    assert!(probes.push(test_probe()));
    let gpu = probes.to_gpu([0.0; 3]);
    assert!(gpu.has_usable_probes());
    assert_eq!(gpu.metadata[..2], [1, 1]);
    assert_eq!(gpu.probes[0].position_radius, [0.0, 0.0, 0.0, 10.0]);
    assert_eq!(gpu.probes[0].dominant_valid[3], 1.0);
}
