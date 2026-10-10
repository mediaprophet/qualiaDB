//! Adapter-backed GPU pixel qualification for environment probe irradiance and specular reflection.

use super::*;
use crate::render::lighting::environment_lighting::{
    EnvironmentProbe, EnvironmentProbeSet, ENVIRONMENT_SPECULAR_LEVELS,
};

#[test]
#[serial_test::serial(gpu)]
fn native_environment_probe_diffuse_irradiance_illuminates_unlit_mesh() {
    if !crate::wgsl_forge::test_gpu_available() {
        return;
    }

    let mut renderer = PortalGpu::new_offscreen(64, 64, 0).expect("native offscreen renderer");
    renderer.set_clear_color(0.0, 0.0, 0.0, 1.0);
    // Extinguish direct sun and ambient so only probe lighting can illuminate the surface.
    renderer.set_lighting(0.0, 1.0, 0.0, 0.0, 0.0);

    // Large camera-facing quad at origin.
    let positions = [
        [-0.8, -0.8, 0.0],
        [0.8, -0.8, 0.0],
        [0.8, 0.8, 0.0],
        [-0.8, 0.8, 0.0],
    ];
    let colors = [[1.0, 1.0, 1.0, 1.0]; 4];
    let indices = [0, 1, 2, 0, 2, 3];

    assert_eq!(
        renderer.upload_mesh_colored(&positions, &colors, &indices),
        2
    );

    // Pass 1: No environment probes bound. Surface should be completely unlit.
    renderer.set_environment_probes(None, [0.0, 0.0, 0.0]);
    renderer
        .render(0.0, &SystemTelemetry::default())
        .expect("render without probes");

    let mut rgba_unlit = vec![0u8; renderer.required_rgba8_bytes()];
    renderer
        .read_rgba8_into(&mut rgba_unlit)
        .expect("readback unlit");

    let center_idx = (32 * 64 + 32) * 4;
    let b_unlit = rgba_unlit[center_idx + 2];
    assert!(
        b_unlit < 5,
        "without probes or direct light, surface must remain dark, got b={b_unlit}"
    );

    // Pass 2: Bind pure-blue environment probe covering the mesh.
    let mut probes = EnvironmentProbeSet::new();
    probes.push(EnvironmentProbe {
        position: [0.0, 0.0, 0.0],
        radius: 10.0,
        irradiance: [0.0, 0.0, 1.0],
        specular_levels: [[0.0, 0.0, 1.0]; ENVIRONMENT_SPECULAR_LEVELS],
        dominant_direction: [0.0, 1.0, 0.0],
        valid: true,
    });
    renderer.set_environment_probes(Some(&probes), [0.0, 0.0, 0.0]);

    renderer
        .render(0.0, &SystemTelemetry::default())
        .expect("render with blue probe");

    let mut rgba_lit = vec![0u8; renderer.required_rgba8_bytes()];
    renderer
        .read_rgba8_into(&mut rgba_lit)
        .expect("readback lit");

    let r_lit = rgba_lit[center_idx];
    let g_lit = rgba_lit[center_idx + 1];
    let b_lit = rgba_lit[center_idx + 2];
    let a_lit = rgba_lit[center_idx + 3];

    assert_eq!(a_lit, 255);
    assert!(
        b_lit > 20,
        "blue irradiance probe must illuminate mesh surface with blue radiance, got b={b_lit}"
    );
    assert!(
        b_lit > r_lit * 2 && b_lit > g_lit * 2,
        "blue channel must strongly dominate under blue probe, got r={r_lit}, g={g_lit}, b={b_lit}"
    );
}

#[test]
#[serial_test::serial(gpu)]
fn native_environment_probe_specular_reflection_on_metallic_mesh() {
    if !crate::wgsl_forge::test_gpu_available() {
        return;
    }

    let mut renderer = PortalGpu::new_offscreen(64, 64, 0).expect("native offscreen renderer");
    renderer.set_clear_color(0.0, 0.0, 0.0, 1.0);
    renderer.set_lighting(0.0, 1.0, 0.0, 0.0, 0.0);

    // Authored metallic material
    let mut material = crate::container_10d::MaterialRecord::legacy_default();
    material.metallic = 1.0;
    material.roughness = 0.05;
    material.base_color = [1.0, 1.0, 1.0, 1.0];

    let positions = [
        [-0.8, -0.8, 0.0],
        [0.8, -0.8, 0.0],
        [0.8, 0.8, 0.0],
        [-0.8, 0.8, 0.0],
    ];
    let colors = [[1.0, 1.0, 1.0, 1.0]; 4];
    let indices = [0, 1, 2, 0, 2, 3];
    let range = crate::container_10d::SubmeshRange {
        first_index: 0,
        index_count: 6,
        material_id: material.id,
        semantic_id: 1,
    };

    renderer
        .upload_mesh_colored_with_frames_and_materials(
            &positions,
            &colors,
            None,
            None,
            &indices,
            &[material],
            &[range],
        )
        .expect("upload metallic mesh");

    // Pure green specular probe
    let mut probes = EnvironmentProbeSet::new();
    probes.push(EnvironmentProbe {
        position: [0.0, 0.0, 0.0],
        radius: 10.0,
        irradiance: [0.0, 0.5, 0.0],
        specular_levels: [[0.0, 1.0, 0.0]; ENVIRONMENT_SPECULAR_LEVELS],
        dominant_direction: [0.0, 1.0, 0.0],
        valid: true,
    });
    renderer.set_environment_probes(Some(&probes), [0.0, 0.0, 0.0]);

    renderer
        .render(0.0, &SystemTelemetry::default())
        .expect("render metallic surface with green probe");

    let mut rgba = vec![0u8; renderer.required_rgba8_bytes()];
    renderer.read_rgba8_into(&mut rgba).expect("readback");

    let center_idx = (32 * 64 + 32) * 4;
    let r = rgba[center_idx];
    let g = rgba[center_idx + 1];
    let b = rgba[center_idx + 2];

    assert!(
        g > 20,
        "green specular probe must reflect on metallic surface, got g={g}"
    );
    assert!(
        g > r * 2 && g > b * 2,
        "green channel must dominate reflected specular radiance, got r={r}, g={g}, b={b}"
    );
}

#[test]
#[serial_test::serial(gpu)]
fn native_environment_probe_distance_falloff_outside_influence_radius() {
    if !crate::wgsl_forge::test_gpu_available() {
        return;
    }

    let mut renderer = PortalGpu::new_offscreen(64, 64, 0).expect("native offscreen renderer");
    renderer.set_clear_color(0.0, 0.0, 0.0, 1.0);
    renderer.set_lighting(0.0, 1.0, 0.0, 0.0, 0.0);

    let positions = [
        [-0.8, -0.8, 0.0],
        [0.8, -0.8, 0.0],
        [0.8, 0.8, 0.0],
        [-0.8, 0.8, 0.0],
    ];
    let colors = [[1.0, 1.0, 1.0, 1.0]; 4];
    let indices = [0, 1, 2, 0, 2, 3];

    assert_eq!(
        renderer.upload_mesh_colored(&positions, &colors, &indices),
        2
    );

    // Place probe far outside influence radius of the mesh at origin
    let mut probes = EnvironmentProbeSet::new();
    probes.push(EnvironmentProbe {
        position: [50.0, 50.0, 50.0],
        radius: 5.0,
        irradiance: [1.0, 1.0, 1.0],
        specular_levels: [[1.0, 1.0, 1.0]; ENVIRONMENT_SPECULAR_LEVELS],
        dominant_direction: [0.0, 1.0, 0.0],
        valid: true,
    });
    renderer.set_environment_probes(Some(&probes), [0.0, 0.0, 0.0]);

    renderer
        .render(0.0, &SystemTelemetry::default())
        .expect("render with distant probe");

    let mut rgba = vec![0u8; renderer.required_rgba8_bytes()];
    renderer.read_rgba8_into(&mut rgba).expect("readback");

    let center_idx = (32 * 64 + 32) * 4;
    let r = rgba[center_idx];
    let g = rgba[center_idx + 1];
    let b = rgba[center_idx + 2];

    assert!(
        r < 5 && g < 5 && b < 5,
        "probe beyond influence radius must evaluate to zero weight, got r={r}, g={g}, b={b}"
    );
}
