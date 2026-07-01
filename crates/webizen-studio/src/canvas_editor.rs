//! Canvas editor utilities — undo history, grid snapping, QPrime elevation.

use crate::canvas_model::{LayoutStrategy, Page, PresentationMode, WebizenWorkspace};
use crate::theme_engine::{builtin_theme_catalog, ThemeBinding};

pub const MAX_HISTORY: usize = 32;

/// Edit vs preview: preview hides handles and blocks layout mutations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanvasEditorMode {
    Edit,
    Preview,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaneInteraction {
    Drag {
        idx: usize,
        orig_x: u16,
        orig_y: u16,
    },
    Resize {
        idx: usize,
        orig_w: u16,
        orig_h: u16,
    },
}

/// Bounded undo/redo stack for workspace manifest edits.
#[derive(Clone, Debug)]
pub struct WorkspaceHistory {
    entries: Vec<WebizenWorkspace>,
    index: usize,
}

impl WorkspaceHistory {
    pub fn new(initial: WebizenWorkspace) -> Self {
        Self {
            entries: vec![initial],
            index: 0,
        }
    }

    pub fn can_undo(&self) -> bool {
        self.index > 0
    }

    pub fn can_redo(&self) -> bool {
        self.index + 1 < self.entries.len()
    }

    pub fn current(&self) -> &WebizenWorkspace {
        &self.entries[self.index]
    }

    pub fn push(&mut self, workspace: WebizenWorkspace) {
        if self.entries.get(self.index) == Some(&workspace) {
            return;
        }
        self.entries.truncate(self.index + 1);
        self.entries.push(workspace);
        if self.entries.len() > MAX_HISTORY {
            let overflow = self.entries.len() - MAX_HISTORY;
            self.entries.drain(0..overflow);
            self.index = self.index.saturating_sub(overflow);
        }
        self.index = self.entries.len().saturating_sub(1);
    }

    pub fn undo(&mut self) -> Option<WebizenWorkspace> {
        if !self.can_undo() {
            return None;
        }
        self.index -= 1;
        Some(self.entries[self.index].clone())
    }

    pub fn redo(&mut self) -> Option<WebizenWorkspace> {
        if !self.can_redo() {
            return None;
        }
        self.index += 1;
        Some(self.entries[self.index].clone())
    }
}

pub fn default_fiduciary_binding() -> ThemeBinding {
    ThemeBinding {
        theme_id: Some("fiduciary-dark".to_string()),
        ..ThemeBinding::default()
    }
}

pub fn new_workspace_shell(name: String, panes: Vec<crate::canvas_model::PanePlacement>) -> WebizenWorkspace {
    let mut ws = WebizenWorkspace::default();
    ws.themes = builtin_theme_catalog();
    ws.environment_theme = default_fiduciary_binding();
    ws.app_theme = default_fiduciary_binding();
    ws.pages.push(crate::canvas_model::Page {
        url_path: "/".to_string(),
        name,
        panes,
        layout_strategy: LayoutStrategy::default(),
        presentation_mode: PresentationMode::GridBound,
        coordinate_space: crate::canvas_model::CoordinateSpace::default(),
        pan_and_zoom: true,
        theme: ThemeBinding::default(),
    });
    ws
}

pub fn grid_metrics(page: &Page) -> (u16, u16, u16) {
    match &page.layout_strategy {
        LayoutStrategy::PointGrid {
            width_points,
            height_points,
            snap_step,
            ..
        } => (*width_points, *height_points, *snap_step.max(&1)),
        _ => (96, 64, 2),
    }
}

pub fn snap_u16(value: u16, step: u16) -> u16 {
    let step = step.max(1);
    ((value + step / 2) / step) * step
}

/// Convert pixel delta on the canvas element to grid-point delta.
pub fn pixel_delta_to_grid(
    dx_px: f64,
    dy_px: f64,
    canvas_width_px: f64,
    canvas_height_px: f64,
    grid_w: u16,
    grid_h: u16,
) -> (i32, i32) {
    let cw = canvas_width_px.max(1.0);
    let ch = canvas_height_px.max(1.0);
    let gx = (dx_px / cw * grid_w as f64).round() as i32;
    let gy = (dy_px / ch * grid_h as f64).round() as i32;
    (gx, gy)
}

pub fn clamp_pane_origin(x: i32, y: i32, w: u16, h: u16, grid_w: u16, grid_h: u16) -> (u16, u16) {
    let max_x = grid_w.saturating_sub(w.max(1));
    let max_y = grid_h.saturating_sub(h.max(1));
    (x.clamp(0, max_x as i32) as u16, y.clamp(0, max_y as i32) as u16)
}

pub fn clamp_pane_size(w: i32, h: i32, x: u16, y: u16, grid_w: u16, grid_h: u16) -> (u16, u16) {
    let max_w = grid_w.saturating_sub(x).max(4);
    let max_h = grid_h.saturating_sub(y).max(4);
    (w.clamp(4, max_w as i32) as u16, h.clamp(4, max_h as i32) as u16)
}

/// QPrime semantic elevation + reduced-motion guard.
pub fn qprime_elevation_css() -> &'static str {
    r#"
.webizen-studio-shell {
  --qualia-elevation-1: 0 12px 26px rgba(0, 0, 0, 0.18);
  --qualia-elevation-2: 0 22px 50px rgba(0, 0, 0, 0.28);
  --qualia-elevation-3: 0 28px 80px rgba(0, 0, 0, 0.38);
}
.webizen-module-pane {
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
  transition: box-shadow 0.22s ease, transform 0.22s ease, border-color 0.18s ease;
}
.webizen-module-pane[data-selected="true"] {
  transform: translateY(-2px);
  box-shadow: var(--qualia-elevation-2);
  z-index: 12;
}
.webizen-canvas-toolbar button {
  transition: background 0.15s ease, border-color 0.15s ease, opacity 0.15s ease;
}
@media (prefers-reduced-motion: reduce) {
  .webizen-module-pane,
  .webizen-canvas-toolbar button {
    transition: none !important;
    transform: none !important;
  }
}
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_undo_redo_roundtrip() {
        let mut h = WorkspaceHistory::new(new_workspace_shell("Test".into(), vec![]));
        let mut ws = h.current().clone();
        ws.pages[0].name = "A".into();
        h.push(ws);
        let mut ws2 = h.current().clone();
        ws2.pages[0].name = "B".into();
        h.push(ws2);
        assert!(h.can_undo());
        let prev = h.undo().unwrap();
        assert_eq!(prev.pages[0].name, "A");
        assert!(h.can_redo());
        let next = h.redo().unwrap();
        assert_eq!(next.pages[0].name, "B");
    }

    #[test]
    fn snap_and_clamp_pane() {
        assert_eq!(snap_u16(7, 4), 8);
        assert_eq!(clamp_pane_origin(200, -3, 10, 8, 96, 64), (86, 0));
    }
}