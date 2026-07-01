//! Semantic scene → volumetric GPU contract → pick positions.
//!
//! Uses the canonical `webizen-render` / `PortalGpu` stack — no external 3D engines.

use std::f64::consts::PI;

use webizen_render::scene_contract::{
    EpistemicState, RenderScene, SceneCamera, SceneNode, ScenePoint, Tensor10DProjection,
};
use webizen_studio::render::qualia::{ItemState, SemanticScene};
use webizen_studio::render::item_color;

/// Golden-angle sphere layout for semantic graph nodes.
fn sphere_position(index: usize, total: usize) -> (f64, f64, f64) {
    let n = total.max(1) as f64;
    let i = index as f64;
    let radius = 4.0 + (i / n) * 2.5;
    let theta = i * PI * (3.0 - 5.0_f64.sqrt());
    let y = 1.0 - (2.0 * i + 1.0) / n;
    let ring = (1.0 - y * y).max(0.0).sqrt();
    let x = radius * ring * theta.cos();
    let z = radius * ring * theta.sin();
    (x, y * radius * 0.6, z)
}

fn map_epistemic(state: ItemState) -> EpistemicState {
    match state {
        ItemState::Default | ItemState::Active => EpistemicState::Collapsed,
        ItemState::Highlighted => EpistemicState::Sandbox,
        ItemState::Alert => EpistemicState::Pending,
    }
}

fn inferencing_from_state(state: ItemState) -> bool {
    matches!(state, ItemState::Highlighted | ItemState::Alert)
}

/// Build a volumetric [`RenderScene`] from a QualiaDB semantic projection.
pub fn semantic_to_render_scene(semantic: &SemanticScene) -> RenderScene {
    let mut scene = RenderScene::new();
    scene.set_background("#0a1122");
    scene.set_camera(SceneCamera {
        position: [0.0, 6.0, 14.0],
        target: [0.0, 0.0, 0.0],
        fov: 55.0,
    });

    let total = semantic.items.len().max(1);
    for (idx, item) in semantic.items.iter().enumerate() {
        let (x, y, z) = sphere_position(idx, total);
        let color = item_color(item.state, item.intensity);
        let radius = 6.0 + item.intensity * 10.0;
        let pulse = match item.state {
            ItemState::Highlighted => 1.2,
            ItemState::Alert => 2.0,
            _ => 0.0,
        };

        scene.add_node(SceneNode {
            id: item.id.clone(),
            position: ScenePoint {
                x: 0.5,
                y: 0.5,
                z: 0.0,
            },
            color,
            radius,
            alpha: 0.55 + item.intensity * 0.4,
            is_inferencing: inferencing_from_state(item.state),
            pulse_rate: pulse,
            tensor: Tensor10DProjection {
                q: if matches!(item.state, ItemState::Alert) {
                    0.35
                } else {
                    0.0
                },
                v: idx as f64,
                w: 1.0,
                x,
                y,
                z,
                t: item.intensity,
                alpha: item.intensity,
                mu: 0.0,
                sigma: item.intensity,
            },
            epistemic_state: map_epistemic(item.state),
            version: item.intensity,
        });
    }

    // Connect sequential neighbors to show relational structure.
    for idx in 0..semantic.items.len().saturating_sub(1) {
        let a = sphere_position(idx, total);
        let b = sphere_position(idx + 1, total);
        scene.add_edge(webizen_render::scene_contract::SceneEdge {
            from: ScenePoint {
                x: a.0,
                y: a.1,
                z: a.2,
            },
            to: ScenePoint {
                x: b.0,
                y: b.1,
                z: b.2,
            },
            color: "rgba(245, 158, 11, 0.35)".to_string(),
            alpha: 0.4,
            width: 1.5,
        });
    }

    scene
}

/// Project node world positions to screen pixels for CPU-side picking.
pub fn compute_pick_positions(
    scene: &RenderScene,
    width: u32,
    height: u32,
) -> Vec<(String, f64, f64, f64)> {
    let w = width.max(1) as f64;
    let h = height.max(1) as f64;
    let cam = scene.camera;
    let dx = cam.position[0] - cam.target[0];
    let dy = cam.position[1] - cam.target[1];
    let dz = cam.position[2] - cam.target[2];
    let dist = (dx * dx + dy * dy + dz * dz).sqrt().max(1.0);

    scene
        .nodes
        .iter()
        .map(|node| {
            let wx = node.tensor.x;
            let wy = node.tensor.y;
            let wz = node.tensor.z;
            // Perspective-ish screen mapping aligned with volumetric camera orbit.
            let sx = 0.5 + (wx / dist) * 0.45;
            let sy = 0.5 - (wy / dist) * 0.45;
            let depth_scale = 1.0 + (wz / dist) * 0.15;
            let px = sx.clamp(0.05, 0.95) * w;
            let py = sy.clamp(0.05, 0.95) * h;
            let radius = node.radius * depth_scale;
            (node.id.clone(), px, py, radius)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use webizen_studio::render::qualia::SceneItem;

    #[test]
    fn semantic_scene_produces_nodes_and_picks() {
        let semantic = SemanticScene {
            items: vec![
                SceneItem {
                    id: "alpha".into(),
                    state: ItemState::Active,
                    intensity: 0.8,
                    provenance: None,
                    reasons: vec![],
                },
                SceneItem {
                    id: "beta".into(),
                    state: ItemState::Highlighted,
                    intensity: 0.5,
                    provenance: None,
                    reasons: vec![],
                },
            ],
            explanations: vec![],
        };
        let scene = semantic_to_render_scene(&semantic);
        assert_eq!(scene.nodes.len(), 2);
        assert_eq!(scene.edges.len(), 1);
        let picks = compute_pick_positions(&scene, 800, 600);
        assert_eq!(picks.len(), 2);
    }
}