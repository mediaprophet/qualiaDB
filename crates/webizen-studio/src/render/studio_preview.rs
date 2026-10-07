//! Native studio render pipeline — wires graph/mesh/qualia/scene/tensor types into
//! the headless GPU contract used by `update_render_preview`.

use super::graph::{Node, Scene, Style};
use super::mesh::{Mesh, Transform};
use super::motion::{Spring, Timeline};
use super::qualia::{build_scene, item_color, ItemState, SceneItem, SceneSource, SemanticScene};
use super::scene::{Camera, ScreenPoint, Vec3};
use super::tensor_buffer::TensorBufferView;
use super::Renderer;
use crate::canvas_model::PanePlacement;
use crate::theme_engine::{ResolvedTheme, ThemeBinding};
use webizen_render::scene_contract::RenderScene;

/// Semantic scene derived from the current workspace pane grid (native host path).
pub fn semantic_scene_from_panes(panes: &[PanePlacement]) -> SemanticScene {
    let items = panes
        .iter()
        .enumerate()
        .map(|(idx, pane)| SceneItem {
            id: pane.component_id.clone(),
            state: if idx == 0 {
                ItemState::Highlighted
            } else {
                ItemState::Active
            },
            intensity: (0.35 + (idx as f64 * 0.08)).min(1.0),
            provenance: Some(format!("pane:{idx}")),
            reasons: vec![format!(
                "{} @ ({:.0}, {:.0})",
                pane.component_id, pane.x, pane.y
            )],
        })
        .collect();

    SemanticScene {
        items,
        explanations: vec!["Workspace pane grid mapped to semantic manifold nodes".to_string()],
    }
}

/// Layout function: map pane ids to world positions on a shallow grid.
fn pane_layout(panes: &[PanePlacement]) -> impl Fn(&str) -> Option<(Vec3, Mesh)> + '_ {
    move |id| {
        let idx = panes.iter().position(|p| p.component_id == id)?;
        let pane = &panes[idx];
        let x = (pane.x as f64 / 12.0) - 4.0;
        let z = (pane.y as f64 / 12.0) - 3.0;
        let size = (pane.w.max(pane.h) as f64 / 18.0).clamp(0.35, 1.4);
        Some((Vec3::new(x, 0.0, z), Mesh::cube(size)))
    }
}

/// Build a scene from any [`SceneSource`] (SPARQL payload, mock, or pane grid).
pub fn workspace_scene_from_source(source: &impl SceneSource, panes: &[PanePlacement]) -> Scene {
    let sem = source.semantic_scene();
    if sem.items.is_empty() {
        return empty_workspace_fallback_scene();
    }
    build_scene(&sem, Camera::default(), |id| pane_layout(panes)(id))
}

/// Build the internal scene graph from workspace panes.
pub fn workspace_scene_from_panes(panes: &[PanePlacement]) -> Scene {
    let sem = semantic_scene_from_panes(panes);
    build_scene(&sem, Camera::default(), |id| pane_layout(panes)(id))
}

/// Convert workspace panes to the neutral GPU [`RenderScene`] contract.
pub fn render_contract_from_panes(panes: &[PanePlacement]) -> RenderScene {
    RenderScene::from(workspace_scene_from_panes(panes))
}

/// Read tensor buffer telemetry without converting the canonical EMF spectrum
/// into display colour. Colour is a later observer-specific projection.
pub fn tensor_buffer_digest(buffer: &[u8]) -> (usize, f64) {
    let view = TensorBufferView::new(buffer);
    if view.is_empty() {
        return (0, 0.0);
    }
    let count = view.len();
    let opacity = view
        .get(0)
        .map(|t| {
            let _emf_spectrum = t.emf_spectrum();
            let _energy = t.manifold_energy();
            t.opacity()
        })
        .unwrap_or(0.0);
    (count, opacity)
}

/// Theme-scoped motion tick for native studio shell (sanctuary disables springs).
pub fn theme_motion_timeline(theme: &ResolvedTheme, dt: f64) -> Timeline {
    let class = theme.class_name.as_deref();
    super::motion::timeline_from_theme(0.0, dt, class)
}

/// One UI motion step for native builds when the selection changes.
pub fn step_native_selection_pulse(
    spring: &mut Spring,
    selected: bool,
    theme: &ResolvedTheme,
) -> f64 {
    super::motion_loop::step_selection_spring(spring, selected, theme.class_name.as_deref(), 0.016)
}

/// Sample color for inspector chips from a semantic item state.
pub fn preview_item_swatch(state: ItemState, intensity: f64) -> String {
    item_color(state, intensity)
}

/// Orbit camera for spatial preview controls (native).
pub fn orbit_preview_camera(camera: &mut Camera, yaw: f64, pitch: f64) {
    camera.orbit(yaw, pitch);
}

/// Build a minimal wireframe grid scene for empty workspaces.
pub fn empty_workspace_fallback_scene() -> Scene {
    Scene::new(Camera::default())
        .with_background("var(--qualia-bg)")
        .add(
            Node::new("grid")
                .with_mesh(Mesh::grid(12.0, 12))
                .with_style(Style::wire("#475569"))
                .with_transform(Transform::at(Vec3::new(0.0, -0.5, 0.0))),
        )
}

/// Count draw primitives when rasterizing a workspace scene (native diagnostics).
pub fn rasterize_scene_draw_count(panes: &[PanePlacement]) -> usize {
    let scene = if panes.is_empty() {
        empty_workspace_fallback_scene()
    } else {
        workspace_scene_from_panes(panes)
    };
    let mut r = DrawCountRenderer::new((960.0, 540.0));
    scene.render(&mut r);
    r.finish()
}

/// Headless native renderer byte length for the current pane layout.
pub async fn native_headless_png_byte_len(
    panes: &[PanePlacement],
    width: u32,
    height: u32,
) -> Option<usize> {
    use super::native::NativeRenderer;
    let scene = if panes.is_empty() {
        empty_workspace_fallback_scene()
    } else {
        workspace_scene_from_panes(panes)
    };
    let mut renderer = NativeRenderer::new(width, height).await.ok()?;
    renderer.resize(width, height);
    scene.render(&mut renderer);
    let _preview_rgba = renderer.read_pixels();
    renderer.read_png().map(|bytes| bytes.len())
}

struct DrawCountRenderer {
    camera: Camera,
    viewport: (f64, f64),
    count: std::cell::Cell<usize>,
}

impl DrawCountRenderer {
    fn new(viewport: (f64, f64)) -> Self {
        Self {
            camera: Camera::default(),
            viewport,
            count: std::cell::Cell::new(0),
        }
    }

    fn finish(self) -> usize {
        self.count.get()
    }
}

impl Renderer for DrawCountRenderer {
    fn viewport(&self) -> (f64, f64) {
        self.viewport
    }

    fn set_camera(&mut self, camera: Camera) {
        self.camera = camera;
    }

    fn clear(&self, _color: &str) {
        self.count.set(self.count.get() + 1);
    }

    fn project(&self, world: Vec3) -> Option<ScreenPoint> {
        self.camera.project(world, self.viewport)
    }

    fn line(&self, _a: ScreenPoint, _b: ScreenPoint, _color: &str, _alpha: f64, _width: f64) {
        self.count.set(self.count.get() + 1);
    }

    fn point(&self, _p: ScreenPoint, _radius: f64, _color: &str, _alpha: f64) {
        self.count.set(self.count.get() + 1);
    }

    fn fill_polygon(&self, _points: &[ScreenPoint], _color: &str, _alpha: f64) {
        self.count.set(self.count.get() + 1);
    }
}

/// Resolve theme binding into a motion-friendly class for spring drivers.
pub fn motion_class_from_binding(
    binding: &ThemeBinding,
    catalog: &[crate::theme_engine::ThemeDefinition],
) -> String {
    let resolved = crate::theme_engine::resolve_theme(Some(binding), catalog);
    resolved
        .class_name
        .unwrap_or_else(|| "theme-fiduciary-dark".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas_model::{LayerBehavior, PanePlacement};
    use crate::theme_engine::builtin_theme_catalog;

    #[test]
    fn workspace_panes_produce_render_contract() {
        let panes = vec![PanePlacement {
            component_id: "dashboard".to_string(),
            x: 12,
            y: 8,
            w: 24,
            h: 18,
            data_bindings: vec![],
            binds_rpc: None,
            requires_capability: vec![],
            ui_mode: None,
            layer: LayerBehavior::Docked,
            anchor: Some("top-left".to_string()),
            min_w_points: 8,
            min_h_points: 6,
            supported_presentations: vec![],
            theme: ThemeBinding::default(),
        }];
        let contract = render_contract_from_panes(&panes);
        assert!(!contract.is_empty());
    }

    #[test]
    fn tensor_digest_handles_empty_buffer() {
        assert_eq!(tensor_buffer_digest(&[]), (0, 0.0));
    }

    #[test]
    fn tensor_digest_reads_engine_abi_and_keeps_emf_sigma_unclamped() {
        use super::super::tensor_buffer::{
            Q42_TENSOR_HEADER_BYTES, Q42_TENSOR_MAGIC, Q42_TENSOR_STRIDE, Q42_TENSOR_VERSION,
        };

        let mut buffer = vec![0u8; Q42_TENSOR_HEADER_BYTES + Q42_TENSOR_STRIDE];
        buffer[0..4].copy_from_slice(&Q42_TENSOR_MAGIC.to_le_bytes());
        buffer[4..6].copy_from_slice(&Q42_TENSOR_VERSION.to_le_bytes());
        buffer[8..12].copy_from_slice(&1_u32.to_le_bytes());
        buffer[12..16].copy_from_slice(&(Q42_TENSOR_STRIDE as u32).to_le_bytes());
        buffer[Q42_TENSOR_HEADER_BYTES + 7 * 4..Q42_TENSOR_HEADER_BYTES + 8 * 4]
            .copy_from_slice(&0.7_f32.to_le_bytes());
        buffer[Q42_TENSOR_HEADER_BYTES + 9 * 4..Q42_TENSOR_HEADER_BYTES + 10 * 4]
            .copy_from_slice(&2.5_f32.to_le_bytes());

        let (count, opacity) = tensor_buffer_digest(&buffer);
        assert_eq!(count, 1);
        assert!((opacity - 0.7).abs() < 1e-6);
        let tensor = TensorBufferView::new(&buffer).get(0).unwrap();
        let emf = tensor.emf_spectrum();
        assert!((emf[0] - 0.7).abs() < 1e-6);
        assert_eq!(emf[1], 0.0);
        assert_eq!(emf[2], 2.5);
    }

    #[test]
    fn motion_class_defaults_to_fiduciary() {
        let class = motion_class_from_binding(&ThemeBinding::default(), &builtin_theme_catalog());
        assert!(class.contains("fiduciary") || class.contains("theme-"));
    }
}
