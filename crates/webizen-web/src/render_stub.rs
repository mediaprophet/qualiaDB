//! Minimal canvas-2D renderer stub for the web package.
//!
//! This is a tiny stand-alone version of the ideas in webizen-studio/src/render/canvas2d.rs
//! and webizen-render's Renderer trait. The goal for the first slice is just enough
//! to prove that data from a .q42 (or JSON stub) can be turned into pixels in the browser.
//!
//! Long-term we want to share the real implementation:
//! - Move the common Scene / Node / Edge / Face / Tensor10D types into webizen-render
//!   behind a "web" feature (or a separate webizen-scene crate).
//! - Make the canvas2d path compile for wasm32-unknown-unknown.
//! - Add an optional WebGPU path when the browser supports it.
//!
//! For now this is intentionally small and self-contained so we can iterate fast on the
//! single-package + .q42 loading story.

use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

pub struct SimpleCanvasRenderer {
    ctx: CanvasRenderingContext2d,
    width: f64,
    height: f64,
}

impl SimpleCanvasRenderer {
    pub fn new(canvas: HtmlCanvasElement) -> Result<Self, JsValue> {
        let ctx = canvas
            .get_context("2d")?
            .ok_or_else(|| JsValue::from_str("no 2d context"))?
            .dyn_into::<CanvasRenderingContext2d>()?;

        let width = canvas.width() as f64;
        let height = canvas.height() as f64;

        Ok(Self { ctx, width, height })
    }

    pub fn clear(&self, color: &str) {
        self.ctx.set_fill_style(&JsValue::from_str(color));
        self.ctx.fill_rect(0.0, 0.0, self.width, self.height);
    }

    pub fn draw_label(&self, text: &str, x: f64, y: f64, color: &str) {
        self.ctx.set_fill_style(&JsValue::from_str(color));
        self.ctx.set_font("14px Inter, system-ui, sans-serif");
        let _ = self.ctx.fill_text(text, x, y);
    }

    pub fn draw_node(&self, x: f64, y: f64, radius: f64, color: &str, label: Option<&str>) {
        self.ctx.begin_path();
        let _ = self.ctx.arc(x, y, radius, 0.0, std::f64::consts::TAU);
        self.ctx.set_fill_style(&JsValue::from_str(color));
        self.ctx.fill();

        if let Some(l) = label {
            self.draw_label(l, x + radius + 4.0, y + 4.0, "#e0e0e0");
        }
    }

    pub fn draw_edge(&self, x1: f64, y1: f64, x2: f64, y2: f64, color: &str, width: f64) {
        self.ctx.begin_path();
        self.ctx.move_to(x1, y1);
        self.ctx.line_to(x2, y2);
        self.ctx.set_stroke_style(&JsValue::from_str(color));
        self.ctx.set_line_width(width);
        self.ctx.stroke();
    }
}