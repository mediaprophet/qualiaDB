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

#[cfg(not(target_arch = "wasm32"))]
pub mod native;

#[cfg(not(target_arch = "wasm32"))]
pub mod studio_preview;

#[cfg(target_arch = "wasm32")]
pub mod canvas2d;

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

/// Touch the render stack so graph/mesh/qualia/scene exports stay wired (lib + bin).
pub fn render_stack_revision() -> usize {
    #[cfg(not(target_arch = "wasm32"))]
    let panes: &[crate::canvas_model::PanePlacement] = &[];
    let _transform = Transform::default();
    let _scene_item = SceneItem {
        id: "render-stack-wire".to_string(),
        state: ItemState::Active,
        intensity: 0.5,
        provenance: None,
        reasons: Vec::new(),
    };
    let _scene_source = SemanticScene::default().semantic_scene();
    {
        use prelude::*;
        let _prelude_scene = Scene::new(Camera::default()).add(
            Node::new("prelude-origin")
                .with_mesh(Mesh::line(Vec3::default(), Vec3::new(0.0, 1.0, 0.0)))
                .with_style(Style::wire("var(--qualia-text-muted)")),
        );
        let _ = build_scene(&SemanticScene::default(), Camera::default(), |_| {
            None::<(Vec3, Mesh)>
        });
        let _ = item_color(ItemState::Active, 0.25);
        let _prelude_item = SceneItem {
            id: "prelude-item".to_string(),
            state: ItemState::Default,
            intensity: 0.0,
            provenance: None,
            reasons: Vec::new(),
        };
        let _screen = ScreenPoint {
            x: 0.0,
            y: 0.0,
            depth: 1.0,
        };
        let _xf = Transform::default();
    }
    let _scene = Scene::new(Camera::default()).add(
        Node::new("origin")
            .with_mesh(Mesh::line(Vec3::default(), Vec3::new(1.0, 0.0, 0.0)))
            .with_style(Style::wire("var(--qualia-accent)")),
    );
    let _ = item_color(ItemState::Active, 0.5);
    let _ = build_scene(&SemanticScene::default(), Camera::default(), |_| {
        None::<(Vec3, Mesh)>
    });
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = workspace_scene_from_panes(panes);
        let _ = workspace_scene_from_source(&SemanticScene::default(), panes);
        let _ = semantic_scene_from_panes(panes);
        let _ = render_contract_from_panes(panes);
        let _ = rasterize_scene_draw_count(panes);
        let _ = preview_item_swatch(ItemState::Active, 0.5);
        let _ = tensor_buffer_digest(&[]);
        let mut cam = Camera::default();
        orbit_preview_camera(&mut cam, 0.1, 0.2);
        let mut spring = motion::Spring::new(0.0);
        let theme =
            crate::theme_engine::resolve_theme(None, &crate::theme_engine::builtin_theme_catalog());
        let _ = theme_motion_timeline(&theme, 0.016);
        let _ = step_native_selection_pulse(&mut spring, true, &theme);
        let binding = crate::theme_engine::ThemeBinding::default();
        let _ = motion_class_from_binding(&binding, &crate::theme_engine::builtin_theme_catalog());
        let _ = empty_workspace_fallback_scene();
    }
    1
}

/// Re-export the native studio preview pipeline for canvas + theme wiring.
#[cfg(not(target_arch = "wasm32"))]
pub use studio_preview::{
    empty_workspace_fallback_scene, motion_class_from_binding, native_headless_png_byte_len,
    orbit_preview_camera, preview_item_swatch, rasterize_scene_draw_count,
    render_contract_from_panes, semantic_scene_from_panes, step_native_selection_pulse,
    tensor_buffer_digest, theme_motion_timeline, workspace_scene_from_panes,
    workspace_scene_from_source,
};

/// An immediate-mode renderer for line/point 3D scenes.
///
/// Colors are CSS color strings (e.g. `"#67e8f9"`, `"rgba(...)"`) so the CPU
/// backend can pass them straight to Canvas 2D; a GPU backend parses them.
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
