//! Backend-agnostic immediate-mode 3D line/point rendering.
//!
//! The browser demo currently renders 3D on the CPU into a Canvas 2D context
//! ([`Canvas2dRenderer`]). The native runtime has real `wgpu` compute, and a
//! future `WgpuRenderer` (WebGPU/WebGL) can implement this same [`Renderer`]
//! trait so call sites such as the physics surface stay backend-agnostic and the
//! browser demo can eventually benchmark genuine GPU work.
//!
//! Geometry is submitted in **world space**; each backend is responsible for
//! projection. The CPU backend projects via [`Camera::project`]; a GPU backend
//! would project in a vertex shader.

pub mod graph;
pub mod mesh;
pub mod motion;
pub mod motion_loop;
pub mod qualia;
pub mod scene;

pub mod node_graph;
pub mod spatial_bridge;
pub mod tensor_buffer;

#[cfg(not(target_arch = "wasm32"))]
pub mod scene_to_contract;

#[cfg(target_arch = "wasm32")]
pub mod canvas2d;

#[allow(unused_imports)]
pub use graph::{Node, Scene, Style};
pub use mesh::{Mesh, Transform};
pub use qualia::{build_scene, item_color, ItemState, SceneItem, SceneSource, SemanticScene};
pub use scene::{Camera, ScreenPoint, Vec3};

#[cfg(target_arch = "wasm32")]
pub use canvas2d::Canvas2dRenderer;

/// Ergonomic single-import surface for building scenes:
/// `use crate::render::prelude::*;`
pub mod prelude {
    #[cfg(target_arch = "wasm32")]
    pub use super::canvas2d::Canvas2dRenderer;
    pub use super::graph::{Node, Scene, Style};
    pub use super::mesh::{Mesh, Transform};

    pub use super::qualia::{build_scene, item_color, ItemState, SceneItem, SemanticScene};
    pub use super::scene::{Camera, ScreenPoint, Vec3};
}

/// An immediate-mode renderer for line/point 3D scenes.
///
/// Colors are CSS color strings (e.g. `"#67e8f9"`, `"rgba(...)"`) so the CPU
/// backend can pass them straight to Canvas 2D; a GPU backend parses them.
#[allow(dead_code)]
pub trait Renderer {
    /// Current drawable size in physical pixels.
    fn viewport(&self) -> (f64, f64);

    /// Set the active camera used for [`Renderer::project`].
    fn set_camera(&mut self, camera: Camera);

    /// Clear the frame to a solid background color.
    fn clear(&self, color: &str);

    /// Project a world-space point to screen space (CPU-side cull/fade helper).
    fn project(&self, world: Vec3) -> Option<ScreenPoint>;

    /// Draw a screen-space line segment.
    fn line(&self, a: ScreenPoint, b: ScreenPoint, color: &str, alpha: f64, width: f64);

    /// Draw a filled screen-space disc (billboarded point).
    fn point(&self, p: ScreenPoint, radius: f64, color: &str, alpha: f64);

    /// Fill a screen-space polygon (e.g. a depth-shaded quad).
    fn fill_polygon(&self, points: &[ScreenPoint], color: &str, alpha: f64);
}
