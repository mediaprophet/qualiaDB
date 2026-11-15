//! Minimal canvas-2D renderer stub for the web package.
//!
//! This is a tiny stand-alone version of the ideas in webizen-studio/src/render/canvas2d.rs
//! and webizen-render's Renderer trait. The goal for the first slice is just enough
//! to prove that data from a .q42 (or JSON stub) can be turned into pixels in the browser.

use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

pub struct SimpleCanvasRenderer {
    ctx: CanvasRenderingContext2d,
    width: f64,
    height: f64,
}

impl SimpleCanvasRenderer {
    /// Creates a new instance.
    pub fn new(canvas: HtmlCanvasElement) -> Result<Self, JsValue> {
        let ctx = canvas
            .get_context("2d")?
            .ok_or_else(|| JsValue::from_str("no 2d context"))?
            .dyn_into::<CanvasRenderingContext2d>()?;

        let width = canvas.width() as f64;
        let height = canvas.height() as f64;

        Ok(Self { ctx, width, height })
    }

    /// Clears.
    pub fn clear(&self, color: &str) {
        self.ctx.set_fill_style_str(color);
        self.ctx.fill_rect(0.0, 0.0, self.width, self.height);
    }

    /// Draws label.
    pub fn draw_label(&self, text: &str, x: f64, y: f64, color: &str) {
        self.ctx.set_fill_style_str(color);
        self.ctx.set_font("14px Inter, system-ui, sans-serif");
        let _ = self.ctx.fill_text(text, x, y);
    }

    /// Draws node.
    pub fn draw_node(&self, x: f64, y: f64, radius: f64, color: &str, label: Option<&str>) {
        self.ctx.begin_path();
        let _ = self.ctx.arc(x, y, radius, 0.0, std::f64::consts::TAU);
        self.ctx.set_fill_style_str(color);
        self.ctx.fill();

        if let Some(l) = label {
            self.draw_label(l, x + radius + 4.0, y + 4.0, "#e0e0e0");
        }
    }

    /// Draws edge.
    pub fn draw_edge(&self, x1: f64, y1: f64, x2: f64, y2: f64, color: &str, width: f64) {
        self.ctx.begin_path();
        self.ctx.move_to(x1, y1);
        self.ctx.line_to(x2, y2);
        self.ctx.set_stroke_style_str(color);
        self.ctx.set_line_width(width);
        self.ctx.stroke();
    }

    /// Projects a 3D scene point into 2D canvas coordinates using yaw rotation and perspective depth.
    pub fn project_point(&self, p: &webizen_render::ScenePoint, yaw: f64) -> (f64, f64) {
        let (wx, wy, wz) = if p.x >= 0.0 && p.x <= 1.0 && p.y >= 0.0 && p.y <= 1.0 {
            ((p.x - 0.5) * 2.0, (0.5 - p.y) * 2.0, p.z)
        } else {
            (p.x, p.y, p.z)
        };
        let cx = yaw.cos();
        let sx = yaw.sin();
        let xr = wx * cx + wz * sx;
        let zr = -wx * sx + wz * cx;
        let depth = (1.0 / (1.0 + zr * 0.35)).clamp(0.2, 1.0);
        let scale = 0.42 * self.width.min(self.height) * depth;
        let px = self.width * 0.5 + xr * scale;
        let py = self.height * 0.5 - wy * scale;
        (px, py)
    }

    /// Fills a 2D polygon with specified color and opacity.
    pub fn fill_polygon(&self, points: &[(f64, f64)], color: &str, alpha: f64) {
        if points.len() < 3 {
            return;
        }
        self.ctx.begin_path();
        self.ctx.set_global_alpha(alpha.clamp(0.0, 1.0));
        self.ctx.set_fill_style_str(color);
        self.ctx.move_to(points[0].0, points[0].1);
        for p in &points[1..] {
            self.ctx.line_to(p.0, p.1);
        }
        self.ctx.close_path();
        self.ctx.fill();
        self.ctx.set_global_alpha(1.0);
    }

    /// Renders a full `RenderScene` (faces, edges, nodes) onto the 2D canvas.
    pub fn render_scene(&self, scene: &webizen_render::RenderScene, yaw: f64) {
        // Draw faces (filled polygons)
        for face in &scene.faces {
            if face.vertices.len() < 3 {
                continue;
            }
            let pts: Vec<(f64, f64)> = face
                .vertices
                .iter()
                .map(|v| self.project_point(v, yaw))
                .collect();
            self.fill_polygon(&pts, &face.color, face.alpha);
        }

        // Draw edges (projected lines)
        for edge in &scene.edges {
            let (x1, y1) = self.project_point(&edge.from, yaw);
            let (x2, y2) = self.project_point(&edge.to, yaw);
            self.ctx.set_global_alpha(edge.alpha.clamp(0.04, 1.0));
            self.draw_edge(x1, y1, x2, y2, &edge.color, edge.width.max(1.0));
            self.ctx.set_global_alpha(1.0);
        }

        // Draw nodes (projected circles with optional labels)
        for node in &scene.nodes {
            let (x, y) = self.project_point(&node.position, yaw);
            let label = if !node.id.is_empty() {
                Some(node.id.as_str())
            } else {
                None
            };
            let radius = if node.radius > 0.0 { node.radius } else { 4.0 };
            self.ctx.set_global_alpha(node.alpha.clamp(0.08, 1.0));
            self.draw_node(x, y, radius, &node.color, label);
            self.ctx.set_global_alpha(1.0);
        }
    }
}
