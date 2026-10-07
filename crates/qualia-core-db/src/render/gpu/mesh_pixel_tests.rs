//! Independent CPU coverage/depth oracle for controlled native offscreen mesh frames.

use super::*;

#[derive(Clone, Copy, Debug, PartialEq)]
struct ScreenVertex {
    x: f64,
    y: f64,
    depth: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Coverage {
    Inside { depth: f64 },
    Outside,
    Edge,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum ExpectedPixel {
    Clear,
    FarBlue,
    NearRed,
    Edge,
}

fn project_triangle(
    points: [[f32; 3]; 3],
    view_projection: [[f32; 4]; 4],
    width: u32,
    height: u32,
) -> [ScreenVertex; 3] {
    points.map(|point| {
        let clip = [
            view_projection[0][0] * point[0]
                + view_projection[1][0] * point[1]
                + view_projection[2][0] * point[2]
                + view_projection[3][0],
            view_projection[0][1] * point[0]
                + view_projection[1][1] * point[1]
                + view_projection[2][1] * point[2]
                + view_projection[3][1],
            view_projection[0][2] * point[0]
                + view_projection[1][2] * point[1]
                + view_projection[2][2] * point[2]
                + view_projection[3][2],
            view_projection[0][3] * point[0]
                + view_projection[1][3] * point[1]
                + view_projection[2][3] * point[2]
                + view_projection[3][3],
        ];
        assert!(
            clip[3].is_finite() && clip[3] > 0.0,
            "fixture must be in front of camera"
        );
        let inv_w = 1.0 / f64::from(clip[3]);
        ScreenVertex {
            x: (f64::from(clip[0]) * inv_w * 0.5 + 0.5) * f64::from(width),
            y: (0.5 - f64::from(clip[1]) * inv_w * 0.5) * f64::from(height),
            depth: f64::from(clip[2]) * inv_w,
        }
    })
}

fn triangle_weights(triangle: [ScreenVertex; 3], x: f64, y: f64) -> Option<[f64; 3]> {
    let [a, b, c] = triangle;
    let edge = |p: ScreenVertex, q: ScreenVertex, x: f64, y: f64| {
        (q.x - p.x) * (y - p.y) - (q.y - p.y) * (x - p.x)
    };
    let area = edge(a, b, c.x, c.y);
    if !area.is_finite() || area.abs() < 1e-12 {
        return None;
    }
    Some([
        edge(b, c, x, y) / area,
        edge(c, a, x, y) / area,
        edge(a, b, x, y) / area,
    ])
}

fn classify(triangle: [ScreenVertex; 3], x: f64, y: f64) -> Coverage {
    const EDGE_GUARD: f64 = 0.06;
    let Some(weights) = triangle_weights(triangle, x, y) else {
        return Coverage::Outside;
    };
    let min_weight = weights.into_iter().fold(f64::INFINITY, f64::min);
    if min_weight < -EDGE_GUARD {
        return Coverage::Outside;
    }
    if min_weight <= EDGE_GUARD {
        return Coverage::Edge;
    }
    Coverage::Inside {
        depth: weights[0] * triangle[0].depth
            + weights[1] * triangle[1].depth
            + weights[2] * triangle[2].depth,
    }
}

fn expected_pixel(
    far: [ScreenVertex; 3],
    near: [ScreenVertex; 3],
    x: f64,
    y: f64,
) -> ExpectedPixel {
    let far_coverage = classify(far, x, y);
    let near_coverage = classify(near, x, y);
    match (far_coverage, near_coverage) {
        (Coverage::Inside { depth: far_depth }, Coverage::Inside { depth: near_depth }) => {
            if near_depth < far_depth {
                ExpectedPixel::NearRed
            } else {
                ExpectedPixel::FarBlue
            }
        }
        (Coverage::Inside { .. }, Coverage::Outside) => ExpectedPixel::FarBlue,
        (Coverage::Outside, Coverage::Inside { .. }) => ExpectedPixel::NearRed,
        (Coverage::Outside, Coverage::Outside) => ExpectedPixel::Clear,
        _ => ExpectedPixel::Edge,
    }
}

#[test]
#[serial_test::serial(gpu)]
fn native_mesh_frame_matches_independent_cpu_coverage_and_depth_oracle() {
    if !crate::wgsl_forge::test_gpu_available() {
        assert!(
            std::env::var_os("QUALIA_REQUIRE_GPU_TESTS").is_none(),
            "QUALIA_REQUIRE_GPU_TESTS is set but no wgpu adapter initialized"
        );
        return;
    }

    const WIDTH: u32 = 96;
    const HEIGHT: u32 = 96;
    let mut renderer =
        PortalGpu::new_offscreen(WIDTH, HEIGHT, 0).expect("native offscreen renderer");
    renderer.set_clear_color(0.0, 0.0, 0.0, 1.0);

    let far = [[-0.8, -0.65, -0.5], [0.8, -0.65, -0.5], [0.0, 0.8, -0.5]];
    let near = [[-0.35, -0.3, 0.5], [0.35, -0.3, 0.5], [0.0, 0.4, 0.5]];
    let positions = [far[0], far[1], far[2], near[0], near[1], near[2]];
    let colors = [
        [0.0, 0.0, 1.0, 1.0],
        [0.0, 0.0, 1.0, 1.0],
        [0.0, 0.0, 1.0, 1.0],
        [1.0, 0.0, 0.0, 1.0],
        [1.0, 0.0, 0.0, 1.0],
        [1.0, 0.0, 0.0, 1.0],
    ];
    assert_eq!(
        renderer.upload_mesh_colored(&positions, &colors, &[0, 1, 2, 3, 4, 5]),
        2
    );
    renderer
        .render(0.0, &SystemTelemetry::default())
        .expect("offscreen mesh draw");

    let mut rgba = vec![0u8; renderer.required_rgba8_bytes()];
    renderer
        .read_rgba8_into(&mut rgba)
        .expect("offscreen readback");
    let camera = renderer.camera_state();
    let view_projection = camera
        .to_uniform(WIDTH as f32 / HEIGHT as f32, false)
        .view_projection;
    let far_screen = project_triangle(far, view_projection, WIDTH, HEIGHT);
    let near_screen = project_triangle(near, view_projection, WIDTH, HEIGHT);
    let mut checked = [0usize; 3]; // clear, far blue, near red

    // Ignore a narrow barycentric band around triangle edges, where API-specific top-left
    // ownership and subpixel fixed-point precision are intentionally outside this oracle.
    for y in (1..HEIGHT - 1).step_by(2) {
        for x in (1..WIDTH - 1).step_by(2) {
            let expected = expected_pixel(
                far_screen,
                near_screen,
                f64::from(x) + 0.5,
                f64::from(y) + 0.5,
            );
            if expected == ExpectedPixel::Edge {
                continue;
            }
            let offset = ((y * WIDTH + x) * 4) as usize;
            let pixel = &rgba[offset..offset + 4];
            match expected {
                ExpectedPixel::Clear => {
                    checked[0] += 1;
                    assert_eq!(pixel, [0, 0, 0, 255], "unexpected geometry at ({x}, {y})");
                }
                ExpectedPixel::FarBlue => {
                    checked[1] += 1;
                    assert!(
                        pixel[2] > pixel[0].saturating_mul(2),
                        "CPU oracle expected far-blue ownership at ({x}, {y}), got {pixel:?}"
                    );
                }
                ExpectedPixel::NearRed => {
                    checked[2] += 1;
                    assert!(
                        pixel[0] > pixel[2].saturating_mul(2),
                        "CPU depth oracle expected near-red ownership at ({x}, {y}), got {pixel:?}"
                    );
                }
                ExpectedPixel::Edge => unreachable!(),
            }
        }
    }

    assert!(
        checked[0] > 500,
        "oracle did not cover enough clear pixels: {checked:?}"
    );
    assert!(
        checked[1] > 20,
        "oracle did not cover enough far-only pixels: {checked:?}"
    );
    assert!(
        checked[2] > 20,
        "oracle did not cover enough near/depth pixels: {checked:?}"
    );
}

#[test]
#[serial_test::serial(gpu)]
fn native_screen_space_ao_only_reduces_ambient_receiver_pixels() {
    if !crate::wgsl_forge::test_gpu_available() {
        assert!(
            std::env::var_os("QUALIA_REQUIRE_GPU_TESTS").is_none(),
            "QUALIA_REQUIRE_GPU_TESTS is set but no wgpu adapter initialized"
        );
        return;
    }

    let mut renderer = PortalGpu::new_offscreen(160, 160, 0).expect("native offscreen renderer");
    renderer.set_clear_color(0.0, 0.0, 0.0, 1.0);
    renderer.set_lighting(0.0, 0.0, 1.0, 0.0, 1.0);
    renderer.set_screen_space_ao_enabled(false);
    renderer.set_screen_space_ao(2.0, 1.0, 0.0);
    renderer.set_screen_space_ao_sample_count(12);

    // A broad receiver plane sits behind a small, front-facing raised patch. Screen-space
    // neighborhood samples across the patch silhouette should lower the receiver's ambient
    // visibility while leaving the diagnostic scene otherwise identical.
    let positions = [
        [-1.5, -1.5, -0.5],
        [1.5, -1.5, -0.5],
        [1.5, 1.5, -0.5],
        [-1.5, 1.5, -0.5],
        [-0.25, -0.25, 0.2],
        [0.25, -0.25, 0.2],
        [0.25, 0.25, 0.2],
        [-0.25, 0.25, 0.2],
    ];
    let colors = [[1.0, 1.0, 1.0, 1.0]; 8];
    let indices = [0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7];
    assert_eq!(
        renderer.upload_mesh_colored(&positions, &colors, &indices),
        4
    );

    renderer
        .render(0.0, &SystemTelemetry::default())
        .expect("ambient-only baseline frame");
    let mut without_ao = vec![0u8; renderer.required_rgba8_bytes()];
    renderer
        .read_rgba8_into(&mut without_ao)
        .expect("baseline pixel readback");

    assert!(
        renderer.screen_space_ao_available(),
        "AO target admission required"
    );
    renderer.set_screen_space_ao_enabled(true);
    for samples in [4, 8, 12] {
        renderer.set_screen_space_ao_sample_count(samples);
        renderer
            .render(0.0, &SystemTelemetry::default())
            .expect("AO receiver frame");
        let mut with_ao = vec![0u8; renderer.required_rgba8_bytes()];
        renderer
            .read_rgba8_into(&mut with_ao)
            .expect("AO pixel readback");

        let mut darkened_pixels = 0usize;
        let mut largest_channel_drop = 0u8;
        for (pixel_index, (base, ao)) in without_ao
            .chunks_exact(4)
            .zip(with_ao.chunks_exact(4))
            .enumerate()
        {
            for channel in 0..3 {
                assert!(
                    ao[channel] <= base[channel].saturating_add(1),
                    "{samples}-tap AO must not brighten ambient-only pixel {pixel_index}: baseline={base:?}, AO={ao:?}"
                );
                largest_channel_drop =
                    largest_channel_drop.max(base[channel].saturating_sub(ao[channel]));
            }
            if (0..3).any(|channel| base[channel].saturating_sub(ao[channel]) >= 2) {
                darkened_pixels += 1;
            }
            assert_eq!(
                ao[3], base[3],
                "{samples}-tap AO must preserve alpha coverage at pixel {pixel_index}"
            );
        }

        assert!(
            darkened_pixels > 0,
            "{samples}-tap AO produced no visible receiver darkening"
        );
        assert!(
            largest_channel_drop >= 2,
            "{samples}-tap AO effect fell below the 8-bit diagnostic floor: max drop {largest_channel_drop}"
        );
    }
}

#[test]
#[serial_test::serial(gpu)]
fn native_base_color_texture_uses_uv_and_srgb_residency() {
    if !crate::wgsl_forge::test_gpu_available() {
        assert!(
            std::env::var_os("QUALIA_REQUIRE_GPU_TESTS").is_none(),
            "QUALIA_REQUIRE_GPU_TESTS is set but no wgpu adapter initialized"
        );
        return;
    }

    let mut renderer = PortalGpu::new_offscreen(64, 64, 0).expect("native offscreen renderer");
    renderer.set_clear_color(0.0, 0.0, 0.0, 1.0);
    renderer.set_lighting(0.0, 0.0, 1.0, 0.0, 1.0);
    renderer.set_screen_space_ao_enabled(false);

    // UV x=0.25 samples the centre of texel zero. The second texel is deliberately green so
    // that a missing/wrong UV stream or sampler address breaks the red-only pixel assertion.
    let digest = [0xA5; 32];
    renderer
        .upload_resident_texture_rgba8(
            digest,
            TextureColorSpace::Srgb,
            2,
            1,
            &[128, 0, 0, 255, 0, 255, 0, 255],
        )
        .expect("resident sRGB base-colour texture");
    assert!(
        renderer
            .resident_texture_binding(&digest, TextureColorSpace::Srgb)
            .is_some(),
        "texture residency must publish the uploaded binding"
    );

    let mut material = crate::container_10d::MaterialRecord::legacy_default();
    material.base_color_texture = digest;
    let range = crate::container_10d::SubmeshRange {
        first_index: 0,
        index_count: 6,
        material_id: material.id,
        semantic_id: 0,
    };
    let positions = [
        [-0.8, -0.8, 0.0],
        [0.8, -0.8, 0.0],
        [0.8, 0.8, 0.0],
        [-0.8, 0.8, 0.0],
    ];
    let colors = [[1.0, 1.0, 1.0, 1.0]; 4];
    let uv0 = [[0.25, 0.5]; 4];
    assert_eq!(
        renderer
            .upload_mesh_colored_with_frames_and_materials_and_uv0(
                &positions,
                &colors,
                None,
                None,
                Some(&uv0),
                &[0, 1, 2, 0, 2, 3],
                &[material],
                &[range],
            )
            .expect("upload textured MAT1 mesh"),
        2
    );
    renderer
        .render(0.0, &SystemTelemetry::default())
        .expect("textured mesh draw");
    let mut rgba = vec![0u8; renderer.required_rgba8_bytes()];
    renderer
        .read_rgba8_into(&mut rgba)
        .expect("textured frame readback");
    let offset = ((32 * 64 + 32) * 4) as usize;
    let pixel = &rgba[offset..offset + 4];

    // A 128 sRGB texel decodes to ~0.216 linear. This shader's diagnostic ambient term is
    // base * intensity (without a Lambertian 1/π factor), so the red channel should land near
    // 55/255; allow backend rounding while staying well below a linear 128/255 interpretation.
    assert!(
        (35..=70).contains(&pixel[0]),
        "sRGB decode + base-colour sampling expected red near 55, got {pixel:?}"
    );
    assert!(
        pixel[1] <= 2 && pixel[2] <= 2,
        "UV-selected red texel should not sample the green neighbour or white fallback: {pixel:?}"
    );
    assert_eq!(pixel[3], 255, "opaque base-colour texture preserves alpha");
}
