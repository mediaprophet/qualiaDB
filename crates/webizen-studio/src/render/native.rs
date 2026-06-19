//! Native WebGPU renderer using webizen-render crate
//!
//! This module provides a native implementation of the Renderer trait using
//! the WgpuRenderer from the webizen-render crate, enabling GPU-accelerated
//! rendering with Projective Geometric Algebra support.

use crate::render::{Camera, Renderer, ScreenPoint, Vec3};
use webizen_render::{
    Camera as WebCamera, ScreenPoint as WebScreenPoint, Vec3 as WebVec3,
    WgpuRenderer as WebizenWgpuRenderer,
};

#[cfg(not(target_arch = "wasm32"))]
/// Native WebGPU renderer implementation
pub struct NativeRenderer {
    inner: WebizenWgpuRenderer<'static>,
}

#[cfg(not(target_arch = "wasm32"))]
impl NativeRenderer {
    /// Create a new native WebGPU renderer backed by an offscreen target.
    ///
    /// The native studio runs inside a dioxus/webview host, so there is no OS
    /// window to provide a `wgpu::Surface`. We render headless into a texture and
    /// expose the frame via [`NativeRenderer::read_pixels`], to be displayed
    /// through the same CPU frame-buffer path the diffusion compute already uses.
    #[allow(dead_code)]
    pub async fn new(width: u32, height: u32) -> Result<Self, String> {
        let inner = WebizenWgpuRenderer::new_offscreen(width, height).await?;
        Ok(Self { inner })
    }

    /// Resize the offscreen render target.
    #[allow(dead_code)]
    pub fn resize(&mut self, width: u32, height: u32) {
        self.inner.resize(width, height);
    }

    /// Read the current frame back as tightly-packed RGBA8 (`width*height*4` bytes).
    #[allow(dead_code)]
    pub fn read_pixels(&self) -> Option<Vec<u8>> {
        self.inner.read_pixels()
    }

    /// Read the current frame back as PNG-encoded bytes (for webview delivery).
    #[allow(dead_code)]
    pub fn read_png(&self) -> Option<Vec<u8>> {
        self.inner.read_png()
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Renderer for NativeRenderer {
    fn viewport(&self) -> (f64, f64) {
        self.inner.viewport()
    }

    fn set_camera(&mut self, camera: Camera) {
        let render_camera = WebCamera {
            position: WebVec3 {
                x: camera.eye.x,
                y: camera.eye.y,
                z: camera.eye.z,
            },
            target: WebVec3 {
                x: camera.target.x,
                y: camera.target.y,
                z: camera.target.z,
            },
            up: WebVec3 {
                x: camera.world_up.x,
                y: camera.world_up.y,
                z: camera.world_up.z,
            },
            fov: camera.focal_scale,
        };
        self.inner.set_camera(render_camera);
    }

    fn clear(&self, color: &str) {
        self.inner.clear(color);
    }

    fn project(&self, world: Vec3) -> Option<ScreenPoint> {
        let render_world = WebVec3 {
            x: world.x,
            y: world.y,
            z: world.z,
        };
        let result = self.inner.project(render_world);
        result.map(|sp| ScreenPoint {
            x: sp.x,
            y: sp.y,
            depth: 0.0,
        })
    }

    fn line(&self, a: ScreenPoint, b: ScreenPoint, color: &str, alpha: f64, width: f64) {
        let render_a = WebScreenPoint { x: a.x, y: a.y };
        let render_b = WebScreenPoint { x: b.x, y: b.y };
        self.inner.line(render_a, render_b, color, alpha, width);
    }

    fn point(&self, p: ScreenPoint, radius: f64, color: &str, alpha: f64) {
        let render_p = WebScreenPoint { x: p.x, y: p.y };
        self.inner.point(render_p, radius, color, alpha);
    }

    fn fill_polygon(&self, points: &[ScreenPoint], color: &str, alpha: f64) {
        let render_points: Vec<WebScreenPoint> = points
            .iter()
            .map(|sp| WebScreenPoint { x: sp.x, y: sp.y })
            .collect();
        self.inner.fill_polygon(&render_points, color, alpha);
    }
}
