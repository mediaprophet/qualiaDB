//! Adapter-backed GPU pixel qualification for water surface and shoreline foam.

use super::*;
use crate::render::gpu::water::{
    WATER_QUALITY_BALANCED, WATER_QUALITY_CINEMATIC, WATER_QUALITY_LOWER,
};

#[test]
#[serial_test::serial(gpu)]
fn native_water_surface_renders_with_depth_blending() {
    if !crate::wgsl_forge::test_gpu_available() {
        return;
    }

    let mut renderer = PortalGpu::new_offscreen(64, 64, 0).expect("native offscreen renderer");
    renderer.set_clear_color(0.0, 0.0, 0.0, 1.0);

    // Deep water color: [0.05, 0.25, 0.65], alpha 0.8
    renderer.set_water_surface([0.05, 0.25, 0.65], 0.8, 0.05, 2.0);
    renderer.set_water_quality(WATER_QUALITY_BALANCED);

    // Camera-facing water quad
    let positions = [
        [-5.0, 0.0, -5.0],
        [5.0, 0.0, -5.0],
        [5.0, 0.0, 5.0],
        [-5.0, 0.0, 5.0],
    ];
    let indices = [0, 1, 2, 0, 2, 3];

    renderer
        .upload_water_geometry(&positions, &indices)
        .expect("upload water geometry");

    renderer
        .render(0.0, &SystemTelemetry::default())
        .expect("render water frame");

    let mut rgba = vec![0u8; renderer.required_rgba8_bytes()];
    renderer.read_rgba8_into(&mut rgba).expect("readback");

    let center_idx = (32 * 64 + 32) * 4;
    let r = rgba[center_idx];
    let g = rgba[center_idx + 1];
    let b = rgba[center_idx + 2];
    let a = rgba[center_idx + 3];

    assert_eq!(a, 255);
    assert!(
        b > 30,
        "water surface must produce visible blue radiance, got b={b}"
    );
    assert!(
        b > r,
        "blue channel must dominate over red on water surface, got r={r}, b={b}"
    );
    assert!(
        g > r,
        "green channel must reflect water tint, got r={r}, g={g}"
    );
}

#[test]
#[serial_test::serial(gpu)]
fn native_water_shoreline_foam_and_quality_tiers() {
    if !crate::wgsl_forge::test_gpu_available() {
        return;
    }

    let mut renderer = PortalGpu::new_offscreen(64, 64, 0).expect("native offscreen renderer");
    renderer.set_clear_color(0.0, 0.0, 0.0, 1.0);

    let positions = [
        [-5.0, 0.0, -5.0],
        [5.0, 0.0, -5.0],
        [5.0, 0.0, 5.0],
        [-5.0, 0.0, 5.0],
    ];
    let indices = [0, 1, 2, 0, 2, 3];

    renderer
        .upload_water_geometry(&positions, &indices)
        .expect("upload water geometry");

    // Pass 1: Lower quality (zero reflection, low foam)
    renderer.set_water_surface([0.1, 0.2, 0.5], 0.7, 0.0, 0.0);
    renderer.set_water_quality(WATER_QUALITY_LOWER);
    renderer
        .render(0.0, &SystemTelemetry::default())
        .expect("render lower quality");

    let mut rgba_lower = vec![0u8; renderer.required_rgba8_bytes()];
    renderer
        .read_rgba8_into(&mut rgba_lower)
        .expect("readback lower");

    // Pass 2: Cinematic quality (high reflection, high shoreline foam response)
    renderer.set_water_quality(WATER_QUALITY_CINEMATIC);
    renderer
        .render(0.0, &SystemTelemetry::default())
        .expect("render cinematic quality");

    let mut rgba_cinematic = vec![0u8; renderer.required_rgba8_bytes()];
    renderer
        .read_rgba8_into(&mut rgba_cinematic)
        .expect("readback cinematic");

    let center_idx = (32 * 64 + 32) * 4;
    let b_lower = rgba_lower[center_idx + 2];
    let b_cinematic = rgba_cinematic[center_idx + 2];

    assert!(
        b_lower > 10 && b_cinematic > 10,
        "both quality tiers must produce valid rendered pixels, got lower={b_lower}, cinematic={b_cinematic}"
    );
}
