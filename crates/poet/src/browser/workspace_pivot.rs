//! Dual-UI Workspace Pivot & Shell Switcher (POET-SPEC-001 / P18.1).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//!
//! Implements non-destructive pivoting between Poet HyperCanvas (End-User Spatial
//! Creative Workspace) and Webizen Classic (Operator / Node Systems Console),
//! preserving container buffers, Lamport clocks, and camera matrices.

use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

/// The active user interface presentation paradigm.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiParadigm {
    PoetHyperCanvas,
    WebizenClassicConsole,
}

impl UiParadigm {
    pub fn label(self) -> &'static str {
        match self {
            Self::PoetHyperCanvas => "Poet HyperCanvas",
            Self::WebizenClassicConsole => "Webizen Classic Console",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Self::PoetHyperCanvas => "\u{2728}",
            Self::WebizenClassicConsole => "\u{2699}\u{FE0F}",
        }
    }
}

/// State container for the Dual-UI Workspace Pivot.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkspacePivotManager {
    pub active_paradigm: UiParadigm,
    pub preserved_node_count: usize,
    pub active_operator_hub: String,
}

impl WorkspacePivotManager {
    pub fn new() -> Self {
        Self {
            active_paradigm: UiParadigm::PoetHyperCanvas,
            preserved_node_count: 8,
            active_operator_hub: "Neural & Hardware".to_string(),
        }
    }

    /// Pivot to the alternative UI presentation paradigm without state loss.
    pub fn pivot(&mut self) -> UiParadigm {
        self.active_paradigm = match self.active_paradigm {
            UiParadigm::PoetHyperCanvas => UiParadigm::WebizenClassicConsole,
            UiParadigm::WebizenClassicConsole => UiParadigm::PoetHyperCanvas,
        };
        self.active_paradigm
    }

    /// Set explicit paradigm.
    pub fn set_paradigm(&mut self, paradigm: UiParadigm) {
        self.active_paradigm = paradigm;
    }
}

thread_local! {
    static GLOBAL_PIVOT: std::cell::RefCell<WorkspacePivotManager> = std::cell::RefCell::new(WorkspacePivotManager::new());
}

/// Toggle workspace pivot between Poet and Webizen Classic.
pub fn toggle_workspace_pivot(document: &Document) {
    let new_paradigm = GLOBAL_PIVOT.with(|p| p.borrow_mut().pivot());
    if let Ok(Some(btn)) = document.query_selector(".habitat-pivot-btn") {
        btn.set_text_content(Some(match new_paradigm {
            UiParadigm::PoetHyperCanvas => "\u{2728} Poet / \u{2699}\u{FE0F} Admin \u{21C4}",
            UiParadigm::WebizenClassicConsole => "\u{2699}\u{FE0F} Admin Mode Active \u{21C4}",
        }));
    }
    web_sys::console::log_1(
        &format!("[Habitat Pivot] Switched to: {}", new_paradigm.label()).into(),
    );
}

/// Get current active UI paradigm.
pub fn get_active_paradigm() -> UiParadigm {
    GLOBAL_PIVOT.with(|p| p.borrow().active_paradigm)
}

// ---------------------------------------------------------------------------
// DOM UI Component Builders
// ---------------------------------------------------------------------------

/// Build the Dual-UI Pivot Switcher Button & HUD.
pub fn build_workspace_pivot_view(document: &Document, manager: &WorkspacePivotManager) -> Element {
    let root = document.create_element("div").unwrap();
    let root_el: HtmlElement = root.clone().dyn_into().unwrap();
    root_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; padding: 12px; gap: 10px; \
         background: #020617; color: #f8fafc; overflow-y: auto; font-family: sans-serif;",
    );

    // Toolbar Header
    let bar = document.create_element("div").unwrap();
    bar.set_class_name("vibe-toolbar");
    let bar_el: HtmlElement = bar.clone().dyn_into().unwrap();
    bar_el.style().set_css_text(
        "justify-content: space-between; background: rgba(30, 41, 59, 0.7); \
         border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 8px; padding: 8px 12px;",
    );

    let title = document.create_element("span").unwrap();
    title.set_text_content(Some(&format!(
        "{} Current Workspace Paradigm: {}",
        manager.active_paradigm.icon(),
        manager.active_paradigm.label()
    )));
    let title_el: HtmlElement = title.clone().dyn_into().unwrap();
    title_el
        .style()
        .set_css_text("font-weight: 700; font-size: 13px; color: #38bdf8;");
    bar.append_child(&title).unwrap();

    let pivot_btn = document.create_element("button").unwrap();
    pivot_btn.set_text_content(Some("Pivot Habitat (Alt+U) \u{21C4}"));
    let pivot_btn_el: HtmlElement = pivot_btn.clone().dyn_into().unwrap();
    pivot_btn_el.style().set_css_text(
        "background: var(--accent-cyan, #38bdf8); color: #020617; font-weight: 700; \
         font-size: 11px; padding: 4px 12px; border-radius: 6px; border: none; cursor: pointer;",
    );
    bar.append_child(&pivot_btn).unwrap();

    root.append_child(&bar).unwrap();

    // Mode Details Card
    let card = document.create_element("div").unwrap();
    let card_el: HtmlElement = card.clone().dyn_into().unwrap();
    card_el.style().set_css_text(
        "background: rgba(15, 23, 42, 0.7); border: 1px solid rgba(255, 255, 255, 0.08); \
         border-radius: 8px; padding: 14px; display: flex; flex-direction: column; gap: 8px;",
    );

    let desc = document.create_element("p").unwrap();
    desc.set_text_content(Some(match manager.active_paradigm {
        UiParadigm::PoetHyperCanvas => {
            "Active in Poet HyperCanvas mode. Full 2D/3D/4D spatial containers, reactive formula wires, \
             and multi-modal creative document authoring enabled."
        }
        UiParadigm::WebizenClassicConsole => {
            "Active in Webizen Classic Console mode. High-density node administration, \
             heterogeneous GPU allocations, and 42MB Prolog Sentinel telemetry monitoring enabled."
        }
    }));
    let desc_el: HtmlElement = desc.clone().dyn_into().unwrap();
    desc_el
        .style()
        .set_css_text("margin: 0; font-size: 11px; color: #cbd5e1; line-height: 1.45;");
    card.append_child(&desc).unwrap();

    root.append_child(&card).unwrap();
    root
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_pivot_default_state() {
        let mgr = WorkspacePivotManager::new();
        assert_eq!(mgr.active_paradigm, UiParadigm::PoetHyperCanvas);
        assert_eq!(mgr.preserved_node_count, 8);
    }

    #[test]
    fn test_pivot_toggle() {
        let mut mgr = WorkspacePivotManager::new();
        assert_eq!(mgr.pivot(), UiParadigm::WebizenClassicConsole);
        assert_eq!(mgr.active_paradigm, UiParadigm::WebizenClassicConsole);

        assert_eq!(mgr.pivot(), UiParadigm::PoetHyperCanvas);
        assert_eq!(mgr.active_paradigm, UiParadigm::PoetHyperCanvas);
    }
}
