//! CPU rasterizing [`Renderer`] backed by an HTML Canvas 2D context.
//!
//! This is the reference/software backend. It projects geometry on the CPU via
//! [`Camera::project`] and strokes/fills into a `CanvasRenderingContext2d`. A
//! future `WgpuRenderer` implements the same [`Renderer`] trait against WebGPU.

use super::scene::{Camera, ScreenPoint, Vec3};
use super::Renderer;
use wasm_bindgen::JsValue;
use web_sys::CanvasRenderingContext2d;

pub struct Canvas2dRenderer {
    ctx: CanvasRenderingContext2d,
    viewport: (f64, f64),
    camera: Camera,
}

impl Canvas2dRenderer {
    /// Wrap an existing 2D context. The context handle is a ref-counted JS value
    /// and may be cloned by the caller, so backend-specific extras (text, custom
    /// fills) can still be issued against the same canvas.
    pub fn new(ctx: CanvasRenderingContext2d, viewport: (f64, f64)) -> Self {
        Self {
            ctx,
            viewport,
            camera: Camera::default(),
        }
    }
}

impl Renderer for Canvas2dRenderer {
    fn viewport(&self) -> (f64, f64) {
        self.viewport
    }

    fn set_camera(&mut self, camera: Camera) {
        self.camera = camera;
    }

    fn clear(&self, color: &str) {
        self.ctx.set_global_alpha(1.0);
        self.ctx.set_fill_style(&JsValue::from_str(color));
        self.ctx
            .fill_rect(0.0, 0.0, self.viewport.0, self.viewport.1);
    }

    fn project(&self, world: Vec3) -> Option<ScreenPoint> {
        self.camera.project(world, self.viewport)
    }

    fn line(&self, a: ScreenPoint, b: ScreenPoint, color: &str, alpha: f64, width: f64) {
        self.ctx.begin_path();
        self.ctx.set_global_alpha(alpha.clamp(0.04, 1.0));
        self.ctx.set_line_width(width);
        self.ctx.set_stroke_style(&JsValue::from_str(color));
        self.ctx.move_to(a.x, a.y);
        self.ctx.line_to(b.x, b.y);
        self.ctx.stroke();
    }

    fn point(&self, p: ScreenPoint, radius: f64, color: &str, alpha: f64) {
        self.ctx.begin_path();
        self.ctx.set_global_alpha(alpha.clamp(0.08, 1.0));
        self.ctx.set_fill_style(&JsValue::from_str(color));
        let _ = self.ctx.arc(p.x, p.y, radius, 0.0, std::f64::consts::TAU);
        self.ctx.fill();
    }

    fn fill_polygon(&self, points: &[ScreenPoint], color: &str, alpha: f64) {
        if points.len() < 3 {
            return;
        }
        self.ctx.begin_path();
        self.ctx.set_global_alpha(alpha.clamp(0.0, 1.0));
        self.ctx.set_fill_style(&JsValue::from_str(color));
        self.ctx.move_to(points[0].x, points[0].y);
        for p in &points[1..] {
            self.ctx.line_to(p.x, p.y);
        }
        self.ctx.close_path();
        self.ctx.fill();
    }
}
