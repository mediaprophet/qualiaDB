//! Neutral, serializable scene contract — the formal hand-off between the
//! decentralized state/runtime layer and the GPU.
//!
//! Re-exports the canonical lightweight scene primitives from
//! `etalon_render::webizen_core::scene`.

pub use etalon_render::webizen_core::scene::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_camera_defaults() {
        let cam = SceneCamera::default();
        assert_eq!(cam.eye, [0.0, 0.0, 5.0]);
        assert_eq!(cam.target, [0.0, 0.0, 0.0]);
        assert_eq!(cam.up, [0.0, 1.0, 0.0]);
        assert_eq!(cam.fov_degrees, 60.0);
    }

    #[test]
    fn render_scene_construction() {
        let mut scene = RenderScene {
            camera: SceneCamera::default(),
            background: "#0b0e14".to_string(),
            nodes: vec![SceneNode {
                position: [1.0, 2.0, 3.0],
                color: "#67e8f9".to_string(),
                radius: 4.0,
                alpha: 1.0,
            }],
            edges: vec![SceneEdge {
                from: 0,
                to: 0,
                color: "#ffffff".to_string(),
                width: 1.0,
                alpha: 1.0,
            }],
        };
        assert_eq!(scene.background, "#0b0e14");
        assert_eq!(scene.nodes.len(), 1);
        assert_eq!(scene.edges.len(), 1);
        scene.nodes.clear();
        assert!(scene.nodes.is_empty());
    }
}
