//! GPU integration and sanity tests for PortalGpu.

use super::super::{
    padded_bytes_per_row, portal_fixed_buffer_bytes, PortalGpu, TemporalProducerViews,
    TemporalSubmissionState,
};
use crate::render::telemetry::SystemTelemetry;
use crate::tensor::buffer_export::{write_tensor_buffer, TensorBufferHeader};
use crate::tensor::Tensor10D;

#[test]
fn portal_fixed_buffer_residency_covers_all_persistent_uniforms() {
    assert_eq!(portal_fixed_buffer_bytes(), Some(640));
}

#[test]
fn offscreen_size_contract_is_caller_buffered() {
    assert_eq!(padded_bytes_per_row(1), wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    assert_eq!(padded_bytes_per_row(64), 256);
    assert_eq!(padded_bytes_per_row(65), 512);
}

#[test]
#[serial_test::serial(gpu)]
fn temporal_owner_is_optional_and_resize_resets_history() {
    if !crate::wgsl_forge::test_gpu_available() {
        return;
    }
    let mut renderer = PortalGpu::new_offscreen(32, 24, 0).expect("offscreen renderer");
    // VRAM admission may intentionally decline the optional temporal owner. In that case
    // the renderer remains valid and the host submission seam must fail closed.
    if renderer.temporal_resolve_gpu().is_none() {
        assert!(!renderer.temporal_history_valid());
        return;
    }
    assert!(!renderer.temporal_history_valid());
    assert!(renderer.temporal_reset_pending());
    assert_eq!(
        renderer.temporal_submission_state(),
        TemporalSubmissionState::Idle
    );
    let availability = renderer.temporal_producer_availability();
    assert!(availability.scene_color);
    assert!(availability.linear_depth);
    assert!(!availability.motion_vectors);
    assert!(!availability.reactive_mask);
    renderer.temporal_reset_pending = false;
    renderer.set_camera(0.2, 0.3, 4.0);
    assert!(renderer.temporal_reset_pending());
    renderer.invalidate_temporal_history();
    assert!(!renderer.temporal_history_valid());
    assert_eq!(
        renderer.temporal_submission_state(),
        TemporalSubmissionState::Idle
    );
    assert!(renderer.temporal_reset_pending());
    assert_eq!(renderer.resize(48, 40).expect("resize"), (48, 40));
    assert!(!renderer.temporal_history_valid());
    assert!(renderer.temporal_reset_pending());
    assert_eq!(renderer.temporal_resolve_gpu().map(|owner| owner.extent()), Some((48, 40)));
}

#[test]
#[serial_test::serial(gpu)]
fn temporal_render_submits_only_with_real_host_producer_views() {
    if !crate::wgsl_forge::test_gpu_available() {
        return;
    }
    let mut renderer = PortalGpu::new_offscreen(32, 24, 0).expect("offscreen renderer");
    if renderer.temporal_resolve_gpu().is_none() {
        return;
    }

    // These are host-owned attachments. The renderer receives their views but never creates
    // or fills them, so the integration test exercises the typed producer seam directly.
    let motion_texture = renderer.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("test-host-motion-vectors"),
        size: wgpu::Extent3d {
            width: 32,
            height: 24,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rg16Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let reactive_texture = renderer.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("test-host-reactive-mask"),
        size: wgpu::Extent3d {
            width: 32,
            height: 24,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let motion_view = motion_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let reactive_view = reactive_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let schedule = crate::render::frame_graph::TemporalOutputSchedule {
        enabled: true,
        reset_history: true,
        reads_history: false,
        publish_history: true,
        final_output: true,
    };
    let missing_producers = TemporalProducerViews {
        extent: (32, 24),
        current_linear_depth: None,
        motion_vectors: None,
        reactive_mask: None,
    };
    assert!(!renderer
        .render_with_temporal(
            0.0,
            &SystemTelemetry::default(),
            schedule,
            missing_producers,
        )
        .expect("ordinary fallback render"));
    assert_eq!(
        renderer.temporal_submission_state(),
        TemporalSubmissionState::Idle
    );
    let producers = TemporalProducerViews {
        extent: (32, 24),
        current_linear_depth: None,
        motion_vectors: Some(&motion_view),
        reactive_mask: Some(&reactive_view),
    };

    assert!(renderer
        .render_with_temporal(0.0, &SystemTelemetry::default(), schedule, producers)
        .expect("temporal render"));
    assert!(renderer.temporal_history_valid());
    assert_eq!(
        renderer.temporal_submission_state(),
        TemporalSubmissionState::OutputReady
    );
    assert!(!renderer.temporal_reset_pending());
}

#[test]
#[serial_test::serial(gpu)]
fn native_offscreen_renders_tensor_and_mesh_on_shared_gpu() {
    if !crate::wgsl_forge::test_gpu_available() {
        return;
    }
    let mut renderer =
        PortalGpu::new_offscreen(96, 96, 256).expect("native offscreen renderer");

    let tensors = [
        Tensor10D::ground_truth(0.0, 0.0, -0.35, 0.0, 0.0, 0.0, 1.0, 0.0, 0.2),
        Tensor10D::ground_truth(0.0, 0.0, 0.35, 0.0, 0.1, 0.0, 1.0, 0.0, 0.8),
    ];
    let mut tensor_bytes = vec![0u8; TensorBufferHeader::total_bytes(tensors.len())];
    write_tensor_buffer(&tensors, &mut tensor_bytes).expect("tensor export");
    assert_eq!(
        renderer
            .upload_tensor_buffer(&tensor_bytes)
            .expect("tensor upload"),
        2
    );
    assert_eq!(
        renderer.upload_mesh(
            &[[-0.6, -0.5, 0.2], [0.6, -0.5, 0.2], [0.0, 0.6, 0.2]],
            &[0, 1, 2],
        ),
        1
    );

    renderer
        .render(0.25, &SystemTelemetry::default())
        .expect("offscreen draw");
    let mut rgba = vec![0u8; renderer.required_rgba8_bytes()];
    assert_eq!(
        renderer
            .read_rgba8_into(&mut rgba)
            .expect("offscreen readback"),
        rgba.len()
    );
    let clear_rgb = crate::render::output::pbr_neutral_v1_srgb([0.03, 0.05, 0.08])
        .map(|channel| (channel * 255.0).round() as u8);
    let clear_rgba = [clear_rgb[0], clear_rgb[1], clear_rgb[2], 255];
    assert!(
        rgba.chunks_exact(4)
            .any(|px| px != clear_rgba && px[3] != 0),
        "expected projected tensor, mesh, or ambient pixels over the clear colour"
    );
}

#[test]
#[serial_test::serial(gpu)]
fn tensor_upload_does_not_enable_ambient_particle_field() {
    if !crate::wgsl_forge::test_gpu_available() {
        return;
    }
    let mut renderer = PortalGpu::new_offscreen(64, 64, 0).expect("native offscreen renderer");
    assert!(
        !renderer.ambient_enabled,
        "ambient particle field must default off"
    );

    let tensors = [Tensor10D::ground_truth(
        0.0, 0.0, -0.35, 0.0, 0.0, 0.0, 1.0, 0.0, 0.2,
    )];
    let mut tensor_bytes = vec![0u8; TensorBufferHeader::total_bytes(tensors.len())];
    write_tensor_buffer(&tensors, &mut tensor_bytes).expect("tensor export");
    assert_eq!(
        renderer
            .upload_tensor_buffer(&tensor_bytes)
            .expect("tensor upload"),
        1
    );

    // Regression: uploading tensor nodes used to force-enable the ambient
    // particle field, resurfacing the debug point cloud after the host had
    // disabled it (and again on GPU re-init re-upload). The field is an
    // explicit presentation choice — `set_ambient_enabled` only.
    assert!(
        !renderer.ambient_enabled,
        "tensor upload must not enable the ambient particle field"
    );
    assert_eq!(
        renderer.tensor_node_count(),
        1,
        "tensor nodes stay resident for semantic picking"
    );

    renderer.set_ambient_enabled(true);
    assert!(renderer.ambient_enabled);
    renderer.set_ambient_enabled(false);
    assert!(!renderer.ambient_enabled);
}

#[test]
#[serial_test::serial(gpu)]
fn native_offscreen_mesh_occlusion_and_non_additive_blend() {
    if !crate::wgsl_forge::test_gpu_available() {
        return;
    }
    let mut renderer = PortalGpu::new_offscreen(64, 64, 0).expect("native offscreen renderer");

    // Two overlapping triangles:
    // Far triangle: z = -0.5, blue [0.0, 0.0, 1.0, 1.0]
    // Near triangle: z = 0.5, red [1.0, 0.0, 0.0, 1.0]
    let positions = [
        [-0.5, -0.5, -0.5],
        [0.5, -0.5, -0.5],
        [0.0, 0.5, -0.5],
        [-0.5, -0.5, 0.5],
        [0.5, -0.5, 0.5],
        [0.0, 0.5, 0.5],
    ];
    let colors = [
        [0.0, 0.0, 1.0, 1.0],
        [0.0, 0.0, 1.0, 1.0],
        [0.0, 0.0, 1.0, 1.0],
        [1.0, 0.0, 0.0, 1.0],
        [1.0, 0.0, 0.0, 1.0],
        [1.0, 0.0, 0.0, 1.0],
    ];
    let indices = [0, 1, 2, 3, 4, 5];

    assert_eq!(
        renderer.upload_mesh_colored(&positions, &colors, &indices),
        2
    );

    renderer
        .render(0.0, &SystemTelemetry::default())
        .expect("offscreen draw");

    let mut rgba = vec![0u8; renderer.required_rgba8_bytes()];
    assert_eq!(
        renderer.read_rgba8_into(&mut rgba).expect("readback"),
        rgba.len()
    );

    // Center pixel should be red (near triangle occludes far blue triangle).
    // It must NOT be additive magenta (R > 100, B > 100).
    let center_idx = (32 * 64 + 32) * 4;
    let r = rgba[center_idx];
    let _g = rgba[center_idx + 1];
    let b = rgba[center_idx + 2];
    let a = rgba[center_idx + 3];

    assert_eq!(a, 255);
    assert!(r > 50, "near red surface should be visible, got r={r}");
    assert!(
        r > b * 2,
        "red should dominate blue (no additive bleed), got r={r}, b={b}"
    );
}

#[test]
#[serial_test::serial(gpu)]
fn native_offscreen_zero_light_intensity_is_black() {
    if !crate::wgsl_forge::test_gpu_available() {
        return;
    }
    let mut renderer = PortalGpu::new_offscreen(64, 64, 0).expect("native offscreen renderer");
    renderer.set_lighting(0.0, 0.0, 1.0, 0.0, 0.0);
    renderer.upload_mesh_colored(
        &[[-0.5, -0.5, 0.5], [0.5, -0.5, 0.5], [0.0, 0.5, 0.5]],
        &[[1.0, 1.0, 1.0, 1.0]; 3],
        &[0, 1, 2],
    );
    renderer
        .render(0.0, &SystemTelemetry::default())
        .expect("offscreen draw");

    let mut rgba = vec![0u8; renderer.required_rgba8_bytes()];
    renderer.read_rgba8_into(&mut rgba).expect("readback");
    let center_idx = (32 * 64 + 32) * 4;
    let pixel = &rgba[center_idx..center_idx + 4];
    assert_eq!(pixel[3], 255);
    assert!(
        pixel[..3].iter().all(|channel| *channel <= 2),
        "zero sun and ambient intensity should produce black, got {pixel:?}"
    );
}

#[test]
#[serial_test::serial(gpu)]
fn native_offscreen_empty_viewport_stays_black_across_resize() {
    if !crate::wgsl_forge::test_gpu_available() {
        assert!(
            std::env::var_os("QUALIA_REQUIRE_GPU_TESTS").is_none(),
            "QUALIA_REQUIRE_GPU_TESTS is set but no wgpu adapter initialized"
        );
        return;
    }
    let mut renderer = PortalGpu::new_offscreen(32, 24, 0).expect("native offscreen renderer");
    renderer.set_clear_color(0.0, 0.0, 0.0, 1.0);

    for (width, height) in [(32, 24), (65, 31), (1, 1)] {
        assert_eq!(renderer.resize(width, height), Ok((width, height)));
        renderer
            .render(0.0, &SystemTelemetry::default())
            .expect("empty offscreen frame");

        let mut rgba = vec![0u8; renderer.required_rgba8_bytes()];
        assert_eq!(
            renderer
                .read_rgba8_into(&mut rgba)
                .expect("offscreen readback"),
            rgba.len()
        );
        assert!(
            rgba.chunks_exact(4).all(|pixel| pixel == [0, 0, 0, 255]),
            "empty viewport should clear to opaque black at {width}x{height}"
        );
    }

    // Zero-sized surface notifications are transient and must retain the last
    // valid render extent rather than replacing attachments with 0xN textures.
    assert_eq!(renderer.resize(0, 48), Ok((1, 1)));
    renderer
        .render(0.0, &SystemTelemetry::default())
        .expect("render after zero-sized resize notification");
    let mut rgba = vec![0u8; renderer.required_rgba8_bytes()];
    renderer
        .read_rgba8_into(&mut rgba)
        .expect("post-resize readback");
    assert_eq!(rgba, [0, 0, 0, 255]);
}

#[test]
#[serial_test::serial(gpu)]
fn camera_history_tracks_frames_and_resets_on_cut_and_resize() {
    if !crate::wgsl_forge::test_gpu_available() {
        return;
    }
    let mut renderer = PortalGpu::new_offscreen(32, 24, 0).expect("native offscreen renderer");
    assert!(renderer.previous_camera_view_projection().is_none());

    let curr_vp = renderer.current_camera_view_projection();
    assert_ne!(curr_vp.cols[0][0], 0.0);

    renderer
        .render(0.0, &SystemTelemetry::default())
        .expect("render frame 1");

    let prev_vp = renderer.previous_camera_view_projection().expect("latched prev vp");
    assert_eq!(prev_vp, curr_vp);

    renderer.invalidate_temporal_history();
    assert!(renderer.previous_camera_view_projection().is_none());

    renderer
        .render(0.0, &SystemTelemetry::default())
        .expect("render frame 2");
    assert!(renderer.previous_camera_view_projection().is_some());

    assert_eq!(renderer.resize(64, 48), Ok((64, 48)));
    assert!(renderer.previous_camera_view_projection().is_none());
}

