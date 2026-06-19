//! Convert internal Scene graph to neutral RenderScene contract
//!
//! This module provides the mapping layer between webizen-studio's rich
//! scene graph (Scene/Node/Mesh) and webizen-render's neutral GPU contract
//! (RenderScene/SceneNode/SceneEdge/SceneFace). This allows the studio to
//! drive the headless GPU renderer while keeping the dependency direction
//! correct (studio → render).

use super::graph::{Node, Scene, Style};
use super::scene::{Camera, Vec3};
use webizen_render::scene_contract::{
    RenderScene, SceneCamera, SceneEdge, SceneFace, SceneNode, ScenePoint,
};

/// Convert webizen-studio Camera to RenderScene SceneCamera
impl From<Camera> for SceneCamera {
    fn from(camera: Camera) -> Self {
        SceneCamera {
            position: [camera.eye.x, camera.eye.y, camera.eye.z],
            target: [camera.target.x, camera.target.y, camera.target.z],
            fov: 60.0, // Fixed FOV for now - can be derived from focal_scale
        }
    }
}

/// Convert internal Scene to RenderScene contract
impl From<Scene> for RenderScene {
    fn from(scene: Scene) -> Self {
        let mut render_scene = RenderScene::new();
        render_scene.set_camera(scene.camera.into());

        // Convert CSS variable background to fallback color
        // In production, this would resolve theme variables
        let bg_color = resolve_css_color(&scene.background);
        render_scene.set_background(bg_color);

        // Walk the scene graph and collect all renderable elements
        for root in &scene.roots {
            collect_scene_elements(root, &|v| v, &mut render_scene);
        }

        render_scene
    }
}

/// Recursively collect renderable elements from the scene graph
fn collect_scene_elements(
    node: &Node,
    world_transform: &dyn Fn(Vec3) -> Vec3,
    render_scene: &mut RenderScene,
) {
    let to_world = |v: Vec3| world_transform(node.transform.apply(v));

    if let Some(mesh) = &node.mesh {
        let style = &node.style;

        // Collect faces (filled polygons) - drawn first (background layer)
        if let Some(fill) = &style.fill {
            for face in &mesh.faces {
                let vertices: Vec<ScenePoint> = face
                    .iter()
                    .filter_map(|&i| {
                        let world_pos = to_world(mesh.vertices[i]);
                        // Project to normalized screen space (0..1)
                        // For now, use simple orthographic projection
                        // TODO: Use camera projection from Scene
                        Some(ScenePoint {
                            x: normalize_coordinate(world_pos.x),
                            y: normalize_coordinate(world_pos.y),
                            z: normalize_coordinate(world_pos.z),
                        })
                    })
                    .collect();

                if vertices.len() >= 3 {
                    render_scene.add_face(SceneFace {
                        vertices,
                        color: resolve_css_color(fill),
                        alpha: style.alpha,
                    });
                }
            }
        }

        // Collect edges (lines)
        if let Some(stroke) = &style.stroke {
            for &(a, b) in &mesh.edges {
                let from = to_world(mesh.vertices[a]);
                let to = to_world(mesh.vertices[b]);

                // Semantic shading: derive edge strength from node weights
                let edge_weight = compute_edge_weight(node);
                let (width, alpha) = compute_edge_attributes(edge_weight, style.width, style.alpha);

                render_scene.add_edge(SceneEdge {
                    from: ScenePoint {
                        x: normalize_coordinate(from.x),
                        y: normalize_coordinate(from.y),
                        z: normalize_coordinate(from.z),
                    },
                    to: ScenePoint {
                        x: normalize_coordinate(to.x),
                        y: normalize_coordinate(to.y),
                        z: normalize_coordinate(to.z),
                    },
                    color: resolve_css_color(stroke),
                    width,
                    alpha,
                });
            }
        }

        // Collect vertices as nodes (points) - drawn last (foreground layer)
        if let Some(point) = &style.point {
            for v in &mesh.vertices {
                let world_pos = to_world(*v);

                // Semantic shading: derive weight from transform scale
                let weight = extract_semantic_weight(node);
                let radius = compute_radius_from_weight(weight, style.point_radius);

                // Semantic shading: derive classification from node label
                let classification = extract_classification_from_label(&node.label);
                let color = map_classification_to_color(classification, point);

                // Animation states: derive from node label semantics
                let (is_inferencing, pulse_rate) = extract_animation_state(&node.label);

                render_scene.add_node(SceneNode {
                    id: node.label.clone(),
                    position: ScenePoint {
                        x: normalize_coordinate(world_pos.x),
                        y: normalize_coordinate(world_pos.y),
                        z: normalize_coordinate(world_pos.z),
                    },
                    color,
                    radius,
                    alpha: style.alpha,
                    is_inferencing,
                    pulse_rate,
                    tensor: webizen_render::scene_contract::Tensor10DProjection::default(),
                    epistemic_state: webizen_render::scene_contract::EpistemicState::Collapsed,
                    version: 0.0,
                });
            }
        }
    }

    // Recursively process children
    for child in &node.children {
        collect_scene_elements(child, &to_world, render_scene);
    }
}

/// Extract semantic weight from a node's transform scale.
/// Larger scales indicate more important/central entities.
fn extract_semantic_weight(node: &Node) -> f64 {
    // Use the maximum scale component as the weight indicator
    node.transform
        .scale
        .x
        .max(node.transform.scale.y)
        .max(node.transform.scale.z)
}

/// Compute edge weight from node weight.
/// Edges from higher-weight nodes are considered stronger.
fn compute_edge_weight(node: &Node) -> f64 {
    extract_semantic_weight(node)
}

/// Compute edge width and alpha from edge weight.
/// Stronger edges get thicker lines and higher opacity.
fn compute_edge_attributes(edge_weight: f64, base_width: f64, base_alpha: f64) -> (f64, f64) {
    // Clamp weight to reasonable range
    let clamped_weight = edge_weight.clamp(0.1, 10.0);
    // Linear scaling for width (edges shouldn't get too thick)
    let width = base_width * (1.0 + (clamped_weight - 1.0) * 0.5);
    // Alpha scales with weight (stronger edges are more visible)
    let alpha = base_alpha * clamped_weight.min(1.0);
    (width, alpha.clamp(0.1, 1.0))
}

/// Compute radius from weight using non-linear scaling.
/// Uses square root scaling to prevent outliers from dominating the screen.
fn compute_radius_from_weight(weight: f64, base_radius: f64) -> f64 {
    // Clamp weight to reasonable range (0.1 to 10.0)
    let clamped_weight = weight.clamp(0.1, 10.0);
    // Square root scaling: sqrt(weight) * base_radius
    base_radius * clamped_weight.sqrt()
}

/// Extract classification from node label.
/// Parses semantic type hints from label naming conventions.
fn extract_classification_from_label(label: &str) -> SemanticClassification {
    let label_lower = label.to_lowercase();

    if label_lower.contains("person")
        || label_lower.contains("user")
        || label_lower.contains("agent")
    {
        SemanticClassification::Person
    } else if label_lower.contains("concept")
        || label_lower.contains("idea")
        || label_lower.contains("topic")
    {
        SemanticClassification::Concept
    } else if label_lower.contains("document")
        || label_lower.contains("file")
        || label_lower.contains("resource")
    {
        SemanticClassification::Document
    } else if label_lower.contains("location")
        || label_lower.contains("place")
        || label_lower.contains("spatial")
    {
        SemanticClassification::Location
    } else if label_lower.contains("event")
        || label_lower.contains("action")
        || label_lower.contains("activity")
    {
        SemanticClassification::Event
    } else if label_lower.contains("organization")
        || label_lower.contains("company")
        || label_lower.contains("group")
    {
        SemanticClassification::Organization
    } else {
        SemanticClassification::Generic
    }
}

/// Map semantic classification to color.
/// Returns RGBA color arrays for the GPU shader.
fn map_classification_to_color(
    classification: SemanticClassification,
    fallback_color: &str,
) -> String {
    match classification {
        SemanticClassification::Person => "#3b82f6".to_string(), // Blue
        SemanticClassification::Concept => "#f59e0b".to_string(), // Orange
        SemanticClassification::Document => "#10b981".to_string(), // Green
        SemanticClassification::Location => "#8b5cf6".to_string(), // Purple
        SemanticClassification::Event => "#ef4444".to_string(),  // Red
        SemanticClassification::Organization => "#6366f1".to_string(), // Indigo
        SemanticClassification::Generic => resolve_css_color(fallback_color),
    }
}

/// Extract animation state from node label.
/// Derives inferencing state and pulse rate from semantic hints.
fn extract_animation_state(label: &str) -> (bool, f64) {
    let label_lower = label.to_lowercase();

    // Nodes with "inferencing", "processing", or "active" keywords are animated
    let is_inferencing = label_lower.contains("inferencing")
        || label_lower.contains("processing")
        || label_lower.contains("computing")
        || label_lower.contains("active");

    // Pulse rate based on semantic intensity (higher for critical nodes)
    let pulse_rate = if label_lower.contains("critical") || label_lower.contains("urgent") {
        2.0 // Fast pulse
    } else if label_lower.contains("normal") || label_lower.contains("standard") {
        1.0 // Normal pulse
    } else if is_inferencing {
        0.5 // Slow pulse for inferencing
    } else {
        0.0 // Static
    };

    (is_inferencing, pulse_rate)
}

/// Semantic classification of nodes for color mapping.
#[derive(Debug, Clone, Copy, PartialEq)]
enum SemanticClassification {
    Person,
    Concept,
    Document,
    Location,
    Event,
    Organization,
    Generic,
}

/// Normalize a world coordinate to screen space (0..1)
/// This is a simplified projection - in production, use proper camera projection
fn normalize_coordinate(coord: f64) -> f64 {
    // Simple normalization: map from typical world range (-10..10) to (0..1)
    (coord + 10.0) / 20.0
}

/// Resolve CSS color variables to concrete colors
/// For now, this handles CSS variables and returns a fallback
fn resolve_css_color(color: &str) -> String {
    // Handle CSS custom properties (variables)
    if color.starts_with("var(") {
        // Map common theme variables to fallback colors
        return match color {
            s if s.contains("qualia-accent") => "#67e8f9".to_string(), // Cyan
            s if s.contains("qualia-bg") => "#101820".to_string(),     // Dark slate
            s if s.contains("qualia-primary") => "#3b82f6".to_string(), // Blue
            s if s.contains("qualia-secondary") => "#8b5cf6".to_string(), // Purple
            s if s.contains("qualia-success") => "#10b981".to_string(), // Green
            s if s.contains("qualia-warning") => "#f59e0b".to_string(), // Amber
            s if s.contains("qualia-error") => "#ef4444".to_string(),  // Red
            _ => "#ffffff".to_string(),                                // Fallback to white
        };
    }

    // Return CSS color as-is if it's not a variable
    color.to_string()
}

#[cfg(test)]
mod tests {
    use super::super::graph::Style;
    use super::super::mesh::Mesh;
    use super::*;

    #[test]
    fn test_camera_conversion() {
        let camera = Camera::default();
        let scene_camera = SceneCamera::from(camera);
        assert_eq!(scene_camera.position, [0.0, 8.0, 18.0]);
        assert_eq!(scene_camera.target, [0.0, 0.5, 0.0]);
    }

    #[test]
    fn test_empty_scene_conversion() {
        let scene = Scene::new(Camera::default());
        let render_scene = RenderScene::from(scene);
        assert!(render_scene.is_empty());
        assert_eq!(render_scene.background, "#101820");
    }

    #[test]
    fn test_scene_with_cube_conversion() {
        let scene = Scene::new(Camera::default()).add(
            Node::new("cube")
                .with_mesh(Mesh::cube(1.0))
                .with_style(Style::wire("#67e8f9")),
        );

        let render_scene = RenderScene::from(scene);
        // A cube has edges and vertices
        assert!(!render_scene.is_empty());
        assert!(render_scene.edges.len() > 0);
    }

    #[test]
    fn test_css_color_resolution() {
        assert_eq!(resolve_css_color("var(--qualia-accent)"), "#67e8f9");
        assert_eq!(resolve_css_color("var(--qualia-bg)"), "#101820");
        assert_eq!(resolve_css_color("#ff0000"), "#ff0000");
        assert_eq!(resolve_css_color("rgb(255,0,0)"), "rgb(255,0,0)");
    }

    #[test]
    fn test_coordinate_normalization() {
        // Map -10..10 to 0..1
        assert_eq!(normalize_coordinate(-10.0), 0.0);
        assert_eq!(normalize_coordinate(0.0), 0.5);
        assert_eq!(normalize_coordinate(10.0), 1.0);
    }
}
