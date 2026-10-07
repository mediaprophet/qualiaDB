use super::*;
use crate::scene_contract::{SceneEdge, SceneFace};

#[test]
fn coarse_texture_level_reduces_cpu_base_to_requested_physical_extent() {
    let source = [
        0, 0, 0, 255, 255, 255, 255, 255, 0, 0, 0, 255, 255, 255, 255, 255, 0, 0, 0, 255, 255, 255,
        255, 255, 0, 0, 0, 255, 255, 255, 255, 255,
    ];
    let (coarse, width, height) =
        coarse_texture_level(&source, 4, 2, 1, TextureMipSemantic::Color).unwrap();
    assert_eq!((width, height), (2, 1));
    assert_eq!(coarse.len(), 8);
    assert!(
        coarse[0] > 128,
        "sRGB reduction should average in linear light"
    );
    assert_eq!(coarse[3], 255);
}

#[test]
fn coarse_alpha_mask_preserves_authored_coverage() {
    let mut source = [255u8; 4 * 4 * 4];
    for pixel in 8..16 {
        source[pixel * 4 + 3] = 0;
    }
    let (coarse, width, height) = coarse_texture_level(
        &source,
        4,
        4,
        1,
        TextureMipSemantic::alpha_mask(0.5).unwrap(),
    )
    .unwrap();
    assert_eq!((width, height), (2, 2));
    let covered = coarse
        .chunks_exact(4)
        .filter(|pixel| pixel[3] >= 128)
        .count();
    assert_eq!(
        covered, 2,
        "coarse alpha coverage should match 8/16 base pixels"
    );
}

#[test]
fn scene_mesh_preserves_css_color_and_alpha() {
    let mut scene = RenderScene::new();
    scene.add_face(SceneFace {
        vertices: vec![
            ScenePoint {
                x: 0.1,
                y: 0.1,
                z: 0.0,
            },
            ScenePoint {
                x: 0.9,
                y: 0.1,
                z: 0.0,
            },
            ScenePoint {
                x: 0.5,
                y: 0.9,
                z: 0.0,
            },
        ],
        color: "#ff0000".to_string(),
        alpha: 0.5,
    });
    scene.add_edge(SceneEdge {
        from: ScenePoint {
            x: 0.0,
            y: 0.0,
            z: 0.1,
        },
        to: ScenePoint {
            x: 1.0,
            y: 1.0,
            z: 0.1,
        },
        color: "#00ff00".to_string(),
        width: 2.0,
        alpha: 0.75,
    });

    let (positions, colors, indices) = scene_mesh(&scene, 100, 100);
    assert_eq!(positions.len(), 7);
    assert_eq!(colors.len(), positions.len());
    assert_eq!(indices.len(), 9);
    assert_eq!(colors[0], [1.0, 0.0, 0.0, 0.5]);
    assert_eq!(colors[3], [0.0, 1.0, 0.0, 0.75]);
}

// ── P9.3 tests ────────────────────────────────────────────────────────

#[test]
fn colour_by_field_is_deterministic() {
    let a = VolumetricRenderer::colour_by_field(0.5, 0.0, 1.0);
    let b = VolumetricRenderer::colour_by_field(0.5, 0.0, 1.0);
    assert_eq!(a, b);
}

#[test]
fn colour_by_field_endpoints() {
    let blue = VolumetricRenderer::colour_by_field(0.0, 0.0, 1.0);
    assert_eq!(blue, [0.0, 0.0, 1.0]);
    let red = VolumetricRenderer::colour_by_field(1.0, 0.0, 1.0);
    assert_eq!(red, [1.0, 0.0, 0.0]);
}

#[test]
fn colour_by_field_midpoint_is_green() {
    let green = VolumetricRenderer::colour_by_field(0.5, 0.0, 1.0);
    assert!((green[0] - 0.0).abs() < 1e-6);
    assert!((green[1] - 1.0).abs() < 1e-6);
    assert!((green[2] - 0.0).abs() < 1e-6);
}

#[test]
fn colour_by_field_degenerate_range() {
    let c = VolumetricRenderer::colour_by_field(42.0, 42.0, 42.0);
    // Degenerate range → t=0.5 → green
    assert!((c[1] - 1.0).abs() < 1e-6);
}

#[test]
fn temporal_scrub_returns_in_window_nodes() {
    use qualia_core_db::tensor::buffer_export::{write_tensor_buffer, TensorBufferHeader};
    use qualia_core_db::tensor::Tensor10D;

    let tensors = [
        Tensor10D::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0),
        Tensor10D::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.5, 1.0, 0.0, 0.0),
        Tensor10D::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0),
        Tensor10D::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 2.0, 1.0, 0.0, 0.0),
    ];
    let mut buf = vec![0u8; TensorBufferHeader::total_bytes(tensors.len())];
    write_tensor_buffer(&tensors, &mut buf).unwrap();

    // Window around t=0.5 with width 0.6 → [0.2, 0.8] → only node 1 (t=0.5)
    let result = VolumetricRenderer::temporal_scrub(&buf, 0.5, 0.6).unwrap();
    assert_eq!(result, vec![1]);
}

#[test]
fn temporal_scrub_empty_window() {
    use qualia_core_db::tensor::buffer_export::{write_tensor_buffer, TensorBufferHeader};
    use qualia_core_db::tensor::Tensor10D;

    let tensors = [
        Tensor10D::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0),
        Tensor10D::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 10.0, 1.0, 0.0, 0.0),
    ];
    let mut buf = vec![0u8; TensorBufferHeader::total_bytes(tensors.len())];
    write_tensor_buffer(&tensors, &mut buf).unwrap();

    let result = VolumetricRenderer::temporal_scrub(&buf, 5.0, 0.0).unwrap();
    assert!(
        result.is_empty(),
        "zero-width window at t=5 should match no nodes"
    );
}

#[test]
fn temporal_scrub_matches_linear_scan_oracle() {
    use qualia_core_db::tensor::buffer_export::{
        read_tensor_at, tensor_node_count, write_tensor_buffer, TensorBufferHeader,
    };
    use qualia_core_db::tensor::Tensor10D;

    let tensors: Vec<Tensor10D> = (0..20)
        .map(|i| {
            Tensor10D::new(
                0.0,
                0.0,
                0.0,
                i as f32 * 0.1,
                0.0,
                0.0,
                i as f32 * 0.3,
                1.0,
                0.0,
                0.0,
            )
        })
        .collect();
    let mut buf = vec![0u8; TensorBufferHeader::total_bytes(tensors.len())];
    write_tensor_buffer(&tensors, &mut buf).unwrap();

    let t_slice = 3.0f32;
    let t_window = 2.0f32;
    let half = t_window * 0.5;
    let lo = t_slice - half;
    let hi = t_slice + half;

    // Linear-scan oracle
    let count = tensor_node_count(&buf).unwrap();
    let mut oracle = Vec::new();
    for i in 0..count {
        let t = read_tensor_at(&buf, i).unwrap();
        if t.t >= lo && t.t <= hi {
            oracle.push(i as u32);
        }
    }

    let result = VolumetricRenderer::temporal_scrub(&buf, t_slice, t_window).unwrap();
    assert_eq!(
        result, oracle,
        "temporal_scrub must match linear-scan oracle"
    );
}
