//! Scene graph — the structural half of the engine dev-kit.
//!
//! A [`Scene`] is a tree of [`Node`]s (transform + optional [`Mesh`] + style +
//! children). [`Scene::render`] walks the tree, composes transforms, and drives
//! any [`Renderer`](super::Renderer) backend (Canvas2D today, WebGPU next). This
//! replaces the per-app hand-rolled draw loops (e.g. the physics surface) and the
//! scene-graph role that three.js/Babylon used to play — but in pure Rust, with
//! geometry that a future native backend can hand to QualiaDB's
//! `geometric_algebra` SIMD kernel.

use super::mesh::{Mesh, Transform};
use super::scene::{Camera, Vec3};
use super::{Renderer, ScreenPoint};

/// Draw style for a node's mesh. Colors are CSS strings (theme `--qualia-*`
/// variables work) so the value flows straight to the Canvas2D backend.
#[derive(Clone, Debug, PartialEq)]
pub struct Style {
    /// Edge/line color, or `None` to skip wireframe.
    pub stroke: Option<String>,
    /// Face fill color, or `None` to skip solid faces.
    pub fill: Option<String>,
    /// Vertex point color, or `None` to skip points.
    pub point: Option<String>,
    pub alpha: f64,
    pub width: f64,
    pub point_radius: f64,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            stroke: Some("var(--qualia-accent)".to_string()),
            fill: None,
            point: None,
            alpha: 1.0,
            width: 1.0,
            point_radius: 2.5,
        }
    }
}

impl Style {
    pub fn wire(color: impl Into<String>) -> Self {
        Self {
            stroke: Some(color.into()),
            ..Default::default()
        }
    }
    pub fn solid(fill: impl Into<String>) -> Self {
        Self {
            stroke: None,
            fill: Some(fill.into()),
            ..Default::default()
        }
    }
    pub fn with_alpha(mut self, a: f64) -> Self {
        self.alpha = a;
        self
    }
}

/// A node in the scene graph.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub label: String,
    pub transform: Transform,
    pub mesh: Option<Mesh>,
    pub style: Style,
    pub children: Vec<Node>,
}

impl Default for Node {
    fn default() -> Self {
        Self {
            label: String::new(),
            transform: Transform::default(),
            mesh: None,
            style: Style::default(),
            children: Vec::new(),
        }
    }
}

impl Node {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            ..Default::default()
        }
    }
    pub fn with_mesh(mut self, mesh: Mesh) -> Self {
        self.mesh = Some(mesh);
        self
    }
    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
    pub fn with_transform(mut self, t: Transform) -> Self {
        self.transform = t;
        self
    }
    pub fn at(mut self, position: Vec3) -> Self {
        self.transform.position = position;
        self
    }
    pub fn child(mut self, node: Node) -> Self {
        self.children.push(node);
        self
    }
}

/// A renderable scene: background, camera, and root nodes.
#[derive(Clone, Debug, PartialEq)]
pub struct Scene {
    pub background: String,
    pub camera: Camera,
    pub roots: Vec<Node>,
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            background: "var(--qualia-bg)".to_string(),
            camera: Camera::default(),
            roots: Vec::new(),
        }
    }
}

impl Scene {
    pub fn new(camera: Camera) -> Self {
        Self {
            camera,
            ..Default::default()
        }
    }
    pub fn with_background(mut self, bg: impl Into<String>) -> Self {
        self.background = bg.into();
        self
    }
    pub fn add(mut self, node: Node) -> Self {
        self.roots.push(node);
        self
    }

    /// Render the whole scene into `r`. Sets the camera, clears, walks the tree.
    pub fn render<R: Renderer>(&self, r: &mut R) {
        r.set_camera(self.camera);
        r.clear(&self.background);
        for node in &self.roots {
            render_node(node, &|v| v, r);
        }
    }
}

/// Recursively draw `node`. `world` maps the node's local space into world space
/// (the composition of all ancestor transforms), so child transforms nest
/// correctly without matrix bookkeeping.
fn render_node<R: Renderer>(node: &Node, world: &dyn Fn(Vec3) -> Vec3, r: &mut R) {
    let to_world = |v: Vec3| world(node.transform.apply(v));

    if let Some(mesh) = &node.mesh {
        // Faces (drawn first so wireframe/points sit on top).
        if let Some(fill) = &node.style.fill {
            for face in &mesh.faces {
                let pts: Option<Vec<ScreenPoint>> = face
                    .iter()
                    .map(|&i| r.project(to_world(mesh.vertices[i])))
                    .collect();
                if let Some(pts) = pts {
                    r.fill_polygon(&pts, fill, node.style.alpha);
                }
            }
        }
        // Edges.
        if let Some(stroke) = &node.style.stroke {
            for &(a, b) in &mesh.edges {
                if let (Some(pa), Some(pb)) = (
                    r.project(to_world(mesh.vertices[a])),
                    r.project(to_world(mesh.vertices[b])),
                ) {
                    r.line(pa, pb, stroke, node.style.alpha, node.style.width);
                }
            }
        }
        // Vertex points.
        if let Some(point) = &node.style.point {
            for v in &mesh.vertices {
                if let Some(p) = r.project(to_world(*v)) {
                    r.point(p, node.style.point_radius, point, node.style.alpha);
                }
            }
        }
    }

    for child in &node.children {
        render_node(child, &to_world, r);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_builder_nesting() {
        let scene = Scene::new(Camera::default()).add(
            Node::new("parent")
                .with_mesh(Mesh::cube(1.0))
                .child(Node::new("child").at(Vec3::new(2.0, 0.0, 0.0))),
        );
        assert_eq!(scene.roots.len(), 1);
        assert_eq!(scene.roots[0].children.len(), 1);
        assert_eq!(scene.roots[0].children[0].transform.position.x, 2.0);
    }
}
