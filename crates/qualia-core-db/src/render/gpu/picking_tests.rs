//! Adapter-backed differential checks for the Tensor10D picking raster pass.

use super::*;
use crate::render::navigation::cpu_pick_node_at_camera;
use crate::render::telemetry::{ObserverStandpoint, STANDPOINT_SPECTATOR};
use crate::tensor::buffer_export::{write_tensor_buffer, TensorBufferHeader};
use crate::tensor::Tensor10D;

fn tensor_bytes(nodes: &[Tensor10D]) -> Vec<u8> {
    let mut bytes = vec![0; TensorBufferHeader::total_bytes(nodes.len())];
    write_tensor_buffer(nodes, &mut bytes).expect("tensor export");
    bytes
}

#[test]
#[serial_test::serial(gpu)]
fn gpu_pick_readback_matches_cpu_oracle_for_overlap_rejection_and_miss() {
    if !crate::wgsl_forge::test_gpu_available() {
        assert!(
            std::env::var_os("QUALIA_REQUIRE_GPU_TESTS").is_none(),
            "QUALIA_REQUIRE_GPU_TESTS is set but no wgpu adapter initialized"
        );
        return;
    }

    let mut renderer = PortalGpu::new_offscreen(800, 600, 0).expect("native offscreen renderer");
    let observer = ObserverStandpoint::new(0, 0, STANDPOINT_SPECTATOR, 1.0, 0.0, 10.0, 1, 0);
    renderer.set_standpoint(observer);
    let camera = renderer.camera_state();
    let forward = crate::render::camera::orbit_forward(camera.yaw, camera.pitch);
    let nearer = [-forward[0] * 0.5, -forward[1] * 0.5, -forward[2] * 0.5];
    let nodes = [
        Tensor10D::default(),
        Tensor10D {
            x: nearer[0],
            y: nearer[1],
            z: nearer[2],
            ..Tensor10D::default()
        },
        Tensor10D {
            t: observer.t_slice + observer.t_window * 2.0,
            ..Tensor10D::default()
        },
        Tensor10D {
            z: camera.target[2] + 8.0,
            ..Tensor10D::default()
        },
    ];
    let bytes = tensor_bytes(&nodes);
    assert_eq!(
        renderer
            .upload_tensor_buffer(&bytes)
            .expect("upload tensor nodes"),
        nodes.len() as u32
    );

    // Fractional pointer coordinates deliberately exercise the shared floor-to-texel rule.
    // At the center, two nodes overlap and the closer index 1 must win. The other center
    // candidates are temporally hidden or behind the camera. The second texel misses all discs.
    for (pointer_x, pointer_y) in [(400.75, 300.25), (407.1, 300.25)] {
        let expected = cpu_pick_node_at_camera(
            &bytes, 800, 600, pointer_x, pointer_y, 0.0, camera, &observer,
        );
        if pointer_x < 401.0 {
            assert_eq!(
                expected,
                Some(1),
                "CPU fixture should select the nearest node"
            );
        } else {
            assert_eq!(expected, None, "CPU fixture should select no node");
        }

        renderer.queue_pick(pointer_x as f32, pointer_y as f32);
        renderer
            .render(0.0, &SystemTelemetry::default())
            .expect("render picking pass");
        let actual = renderer.poll_pick_readback();
        assert_eq!(
            actual, expected,
            "GPU/CPU pick mismatch at ({pointer_x}, {pointer_y})"
        );
    }
}
