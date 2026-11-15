//! Neutral GPU-facing scene contract.
//!
//! Re-exports the canonical serde-serializable scene contract from
//! `etalon_render::webizen_core::scene_contract`.

pub use etalon_render::webizen_core::scene_contract::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scene_point_serialization() {
        let point = ScenePoint {
            x: 0.5,
            y: 0.3,
            z: 0.0,
        };
        let serialized = serde_json::to_string(&point).unwrap();
        let deserialized: ScenePoint = serde_json::from_str(&serialized).unwrap();
        assert_eq!(point, deserialized);
    }

    #[test]
    fn test_scene_node_serialization() {
        let node = SceneNode {
            id: "test-node".to_string(),
            position: ScenePoint {
                x: 0.5,
                y: 0.3,
                z: 0.0,
            },
            color: "#ff0000".to_string(),
            radius: 5.0,
            alpha: 1.0,
            is_inferencing: false,
            pulse_rate: 0.0,
            ..SceneNode::default()
        };
        let serialized = serde_json::to_string(&node).unwrap();
        let deserialized: SceneNode = serde_json::from_str(&serialized).unwrap();
        assert_eq!(node.color, deserialized.color);
        assert_eq!(node.radius, deserialized.radius);
        assert_eq!(node.is_inferencing, deserialized.is_inferencing);
        assert_eq!(node.id, deserialized.id);
    }

    #[test]
    fn test_render_scene_serialization() {
        let scene = RenderScene {
            nodes: vec![SceneNode {
                id: "test-node".to_string(),
                position: ScenePoint {
                    x: 0.5,
                    y: 0.3,
                    z: 0.0,
                },
                color: "#ff0000".to_string(),
                radius: 5.0,
                alpha: 1.0,
                is_inferencing: false,
                pulse_rate: 0.0,
                ..SceneNode::default()
            }],
            edges: vec![],
            faces: vec![],
            camera: SceneCamera::default(),
            background: "#101820".to_string(),
            ..RenderScene::default()
        };
        let serialized = serde_json::to_string(&scene).unwrap();
        let deserialized: RenderScene = serde_json::from_str(&serialized).unwrap();
        assert_eq!(scene.nodes.len(), deserialized.nodes.len());
        assert_eq!(scene.background, deserialized.background);
    }

    #[test]
    fn test_render_scene_builder() {
        let mut scene = RenderScene::new();
        scene.set_background("#202030");
        assert_eq!(scene.background, "#202030");
        assert!(scene.is_empty());

        scene.add_node(SceneNode {
            id: "test-node".to_string(),
            position: ScenePoint {
                x: 0.5,
                y: 0.5,
                z: 0.0,
            },
            color: "#00ff00".to_string(),
            radius: 3.0,
            alpha: 0.8,
            is_inferencing: false,
            pulse_rate: 0.0,
            ..SceneNode::default()
        });
        assert!(!scene.is_empty());
        assert_eq!(scene.element_count(), 1);
    }

    #[cfg(feature = "qualia")]
    #[test]
    fn spectral_color_uses_engine_oracle() {
        let tensor = Tensor10DProjection {
            sigma: 0.42,
            ..Tensor10DProjection::default()
        };
        let (r, g, b) = qualia_core_db::render::spectral::sigma_to_display_rgb(0.42);
        assert_eq!(tensor.spectral_to_color(), format!("rgb({r}, {g}, {b})"));
    }
}
