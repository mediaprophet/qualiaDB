//! Recursive Sub-Manifold Zoom & Breadcrumb Navigation (POET-SPEC-001 / POET-SPEC-002).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//!
//! Implements multi-level nested sub-canvas traversal, affine camera zoom
//! transitions, Level-of-Detail (LOD) scaling, and breadcrumb stack navigation.

use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

/// A single step in the sub-manifold navigation breadcrumb stack.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifoldBreadcrumb {
    pub id: String,
    pub title: String,
    pub depth: usize,
}

/// Camera transform state for spatial affine zooming.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CameraTransform {
    pub pan_x: f64,
    pub pan_y: f64,
    pub zoom: f64,
}

impl Default for CameraTransform {
    fn default() -> Self {
        Self {
            pan_x: 0.0,
            pan_y: 0.0,
            zoom: 1.0,
        }
    }
}

/// State container for recursive sub-manifold navigation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SubmanifoldNavigator {
    pub breadcrumb_stack: Vec<ManifoldBreadcrumb>,
    pub camera: CameraTransform,
    pub max_lod_depth: usize,
}

impl SubmanifoldNavigator {
    pub fn new(root_id: &str, root_title: &str) -> Self {
        Self {
            breadcrumb_stack: vec![ManifoldBreadcrumb {
                id: root_id.to_string(),
                title: root_title.to_string(),
                depth: 0,
            }],
            camera: CameraTransform::default(),
            max_lod_depth: 6,
        }
    }

    /// Current active manifold ID.
    pub fn current_manifold_id(&self) -> &str {
        &self.breadcrumb_stack.last().unwrap().id
    }

    /// Current navigation depth.
    pub fn current_depth(&self) -> usize {
        self.breadcrumb_stack.len().saturating_sub(1)
    }

    /// Dive into a nested `.subcanvas` container node.
    pub fn dive_into_subcanvas(&mut self, subcanvas_id: &str, title: &str) -> bool {
        if self.current_depth() >= self.max_lod_depth {
            return false;
        }
        let next_depth = self.breadcrumb_stack.len();
        self.breadcrumb_stack.push(ManifoldBreadcrumb {
            id: subcanvas_id.to_string(),
            title: title.to_string(),
            depth: next_depth,
        });

        // Reset local camera zoom & pan for the nested sub-manifold
        self.camera.pan_x = 0.0;
        self.camera.pan_y = 0.0;
        self.camera.zoom = 1.0;
        true
    }

    /// Pop back up to a specific breadcrumb level.
    pub fn pop_to_depth(&mut self, depth: usize) -> bool {
        if depth < self.breadcrumb_stack.len() {
            self.breadcrumb_stack.truncate(depth + 1);
            self.camera.pan_x = 0.0;
            self.camera.pan_y = 0.0;
            self.camera.zoom = 1.0;
            true
        } else {
            false
        }
    }

    /// Pop back up one level to the parent manifold.
    pub fn pop_one_level(&mut self) -> bool {
        if self.breadcrumb_stack.len() > 1 {
            self.breadcrumb_stack.pop();
            self.camera.pan_x = 0.0;
            self.camera.pan_y = 0.0;
            self.camera.zoom = 1.0;
            true
        } else {
            false
        }
    }
}

// ---------------------------------------------------------------------------
// DOM UI Component Builders
// ---------------------------------------------------------------------------

/// Build the Sub-Manifold Navigation Breadcrumb Bar & Viewport.
pub fn build_submanifold_nav_view(document: &Document, nav: &SubmanifoldNavigator) -> Element {
    let root = document.create_element("div").unwrap();
    let root_el: HtmlElement = root.clone().dyn_into().unwrap();
    root_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; padding: 12px; gap: 10px; \
         background: #020617; color: #f8fafc; overflow-y: auto; font-family: sans-serif;",
    );

    // Breadcrumb Strip
    let strip = document.create_element("div").unwrap();
    let strip_el: HtmlElement = strip.clone().dyn_into().unwrap();
    strip_el.style().set_css_text(
        "display: flex; align-items: center; gap: 8px; background: rgba(30, 41, 59, 0.7); \
         border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 8px; padding: 8px 12px; font-size: 12px;"
    );

    let home_icon = document.create_element("span").unwrap();
    home_icon.set_text_content(Some("\u{1F5FA}\u{FE0F}"));
    strip.append_child(&home_icon).unwrap();

    for (idx, crumb) in nav.breadcrumb_stack.iter().enumerate() {
        if idx > 0 {
            let sep = document.create_element("span").unwrap();
            sep.set_text_content(Some("\u{25B8}"));
            let sep_el: HtmlElement = sep.clone().dyn_into().unwrap();
            sep_el.style().set_css_text("color: #64748b;");
            strip.append_child(&sep).unwrap();
        }

        let item = document.create_element("span").unwrap();
        item.set_text_content(Some(&crumb.title));
        let item_el: HtmlElement = item.clone().dyn_into().unwrap();
        let is_current = idx == nav.breadcrumb_stack.len() - 1;
        item_el.style().set_css_text(&format!(
            "font-weight: {}; color: {}; cursor: pointer; padding: 2px 6px; border-radius: 4px; \
             background: {};",
            if is_current { "700" } else { "500" },
            if is_current { "#38bdf8" } else { "#94a3b8" },
            if is_current {
                "rgba(56, 189, 248, 0.15)"
            } else {
                "transparent"
            }
        ));
        strip.append_child(&item).unwrap();
    }

    let depth_badge = document.create_element("span").unwrap();
    depth_badge.set_text_content(Some(&format!(
        "LOD Depth: {}/{}",
        nav.current_depth(),
        nav.max_lod_depth
    )));
    let depth_badge_el: HtmlElement = depth_badge.clone().dyn_into().unwrap();
    depth_badge_el.style().set_css_text("margin-left: auto; font-size: 10px; font-family: var(--font-mono); color: #34d399; background: rgba(0,0,0,0.3); padding: 2px 8px; border-radius: 10px;");
    strip.append_child(&depth_badge).unwrap();

    root.append_child(&strip).unwrap();

    // Canvas Stage Info
    let stage_card = document.create_element("div").unwrap();
    let stage_card_el: HtmlElement = stage_card.clone().dyn_into().unwrap();
    stage_card_el.style().set_css_text(
        "background: rgba(15, 23, 42, 0.7); border: 1px solid rgba(255, 255, 255, 0.08); \
         border-radius: 8px; padding: 14px; display: flex; flex-direction: column; gap: 8px;",
    );

    let card_title = document.create_element("h4").unwrap();
    card_title.set_text_content(Some(&format!(
        "Active Sub-Manifold: {}",
        nav.current_manifold_id()
    )));
    let card_title_el: HtmlElement = card_title.clone().dyn_into().unwrap();
    card_title_el
        .style()
        .set_css_text("margin: 0; font-size: 13px; color: #38bdf8;");
    stage_card.append_child(&card_title).unwrap();

    let desc = document.create_element("p").unwrap();
    desc.set_text_content(Some(
        "Double-clicking any .subcanvas container smoothly performs an affine camera zoom dive into its internal graph, preserving the parent manifold in the breadcrumb stack without state loss."
    ));
    let desc_el: HtmlElement = desc.clone().dyn_into().unwrap();
    desc_el
        .style()
        .set_css_text("margin: 0; font-size: 11px; color: #cbd5e1; line-height: 1.45;");
    stage_card.append_child(&desc).unwrap();

    root.append_child(&stage_card).unwrap();
    root
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_submanifold_nav_default_state() {
        let nav = SubmanifoldNavigator::new("root-manifold", "Research Manifold");
        assert_eq!(nav.current_manifold_id(), "root-manifold");
        assert_eq!(nav.current_depth(), 0);
        assert_eq!(nav.breadcrumb_stack.len(), 1);
    }

    #[test]
    fn test_dive_and_pop_navigation() {
        let mut nav = SubmanifoldNavigator::new("root-manifold", "Research Manifold");

        // Dive into subcanvas 1
        assert!(nav.dive_into_subcanvas("sub-01", "Catchment Basin Deep Dive"));
        assert_eq!(nav.current_manifold_id(), "sub-01");
        assert_eq!(nav.current_depth(), 1);

        // Dive into nested subcanvas 2
        assert!(nav.dive_into_subcanvas("sub-02", "Nitrate Sensor Mesh"));
        assert_eq!(nav.current_manifold_id(), "sub-02");
        assert_eq!(nav.current_depth(), 2);

        // Pop one level back to sub-01
        assert!(nav.pop_one_level());
        assert_eq!(nav.current_manifold_id(), "sub-01");
        assert_eq!(nav.current_depth(), 1);

        // Pop back to root by depth
        assert!(nav.pop_to_depth(0));
        assert_eq!(nav.current_manifold_id(), "root-manifold");
        assert_eq!(nav.current_depth(), 0);
    }

    #[test]
    fn test_max_lod_depth_boundary() {
        let mut nav = SubmanifoldNavigator::new("root", "Root");
        nav.max_lod_depth = 2;

        assert!(nav.dive_into_subcanvas("level-1", "Level 1"));
        assert!(nav.dive_into_subcanvas("level-2", "Level 2"));
        // At max depth, cannot dive further
        assert!(!nav.dive_into_subcanvas("level-3", "Level 3"));
        assert_eq!(nav.current_depth(), 2);
    }
}
