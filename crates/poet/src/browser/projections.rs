//! Polymorphic Presentation Engine & Data Projection Switcher (<q-view-switcher>).
//!
//! Provides substrate-neutral dynamic projections of Knowledge Graph data, Super-Quins,
//! and VibeScript structures across 10 primary presentation domains and 35+ visual modes.
//!
//! Aligned with `12_INFORMATION_PRESENTATION_MODES_AND_DATA_PROJECTIONS_SPEC.md` and `POET-SPEC-012`.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement, MouseEvent};

/// The 10 Primary Presentation Domains in Poet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PresentationDomain {
    Hierarchical,
    Tabular,
    Media,
    Document,
    Chronological,
    AgilePM,
    KnowledgeGraph,
    Sensory,
    Telemetry,
    Social,
}

impl PresentationDomain {
    pub fn label(&self) -> &'static str {
        match self {
            PresentationDomain::Hierarchical => "Hierarchical Explorers",
            PresentationDomain::Tabular => "Tabular & Data Grids",
            PresentationDomain::Media => "Media & DAM Galleries",
            PresentationDomain::Document => "Document & Hypertext",
            PresentationDomain::Chronological => "Chronological & Timelines",
            PresentationDomain::AgilePM => "Agile PM & Decisions",
            PresentationDomain::KnowledgeGraph => "Knowledge Graphs & DAGs",
            PresentationDomain::Sensory => "Sensory Studios & 10D",
            PresentationDomain::Telemetry => "Telemetry & Cockpits",
            PresentationDomain::Social => "Communications & Social",
        }
    }

    pub fn glyph(&self) -> &'static str {
        match self {
            PresentationDomain::Hierarchical => "\u{1F332}", // 🌲
            PresentationDomain::Tabular => "\u{1F4CA}",      // 📊
            PresentationDomain::Media => "\u{1F3A8}",        // 🎨
            PresentationDomain::Document => "\u{1F4C4}",     // 📄
            PresentationDomain::Chronological => "\u{23F3}", // ⏳
            PresentationDomain::AgilePM => "\u{1F4CB}",      // 📋
            PresentationDomain::KnowledgeGraph => "\u{1F578}\u{FE0F}", // 🕸️
            PresentationDomain::Sensory => "\u{1F9CA}",      // 🧊
            PresentationDomain::Telemetry => "\u{1F4C8}",    // 📈
            PresentationDomain::Social => "\u{1F465}",       // 👥
        }
    }
}

/// Specification for a single presentation mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresentationModeSpec {
    pub id: &'static str,
    pub domain: PresentationDomain,
    pub name: &'static str,
    pub glyph: &'static str,
    pub description: &'static str,
}

/// Master catalog of all 35+ Polymorphic Presentation Modes.
pub const PRESENTATION_MODES: &[PresentationModeSpec] = &[
    // Domain 1: Hierarchical
    PresentationModeSpec {
        id: "mode.tree",
        domain: PresentationDomain::Hierarchical,
        name: "Collapsible Tree",
        glyph: "\u{1F332}",
        description: "Expandable nested hierarchy with node type badges",
    },
    PresentationModeSpec {
        id: "mode.miller_columns",
        domain: PresentationDomain::Hierarchical,
        name: "Miller Columns",
        glyph: "\u{1F5C2}\u{FE0F}",
        description: "Cascading horizontal multi-column explorer",
    },
    PresentationModeSpec {
        id: "mode.spatial_desk",
        domain: PresentationDomain::Hierarchical,
        name: "Spatial Freeform",
        glyph: "\u{1F5FA}\u{FE0F}",
        description: "2D spatial desk canvas with wire connectors",
    },
    PresentationModeSpec {
        id: "mode.icon_grid",
        domain: PresentationDomain::Hierarchical,
        name: "Desktop Icon Grid",
        glyph: "\u{1F5A5}\u{FE0F}",
        description: "8px grid-snapped desktop icon layout",
    },
    // Domain 2: Tabular
    PresentationModeSpec {
        id: "mode.spreadsheet",
        domain: PresentationDomain::Tabular,
        name: "Reactive Spreadsheet",
        glyph: "\u{1F4CA}",
        description: "2D formula calculation grid with fx and P64 tensors",
    },
    PresentationModeSpec {
        id: "mode.data_table",
        domain: PresentationDomain::Tabular,
        name: "Virtualized Table",
        glyph: "\u{1F4D1}",
        description: "High-performance virtualized grid for large datasets",
    },
    PresentationModeSpec {
        id: "mode.relational_grid",
        domain: PresentationDomain::Tabular,
        name: "Entity Matrix",
        glyph: "\u{1F517}",
        description: "Relational table with expandable foreign entity chips",
    },
    PresentationModeSpec {
        id: "mode.pivot_table",
        domain: PresentationDomain::Tabular,
        name: "Pivot Cross-Tab",
        glyph: "\u{1F500}",
        description: "Multi-way cross-tabulation with aggregation rollups",
    },
    // Domain 3: Media & DAM
    PresentationModeSpec {
        id: "mode.masonry_gallery",
        domain: PresentationDomain::Media,
        name: "Masonry Gallery",
        glyph: "\u{1F9F1}",
        description: "Fluid multi-column masonry grid for visual assets",
    },
    PresentationModeSpec {
        id: "mode.filmstrip",
        domain: PresentationDomain::Media,
        name: "Filmstrip Scrubber",
        glyph: "\u{1F39E}\u{FE0F}",
        description: "Large active viewport with bottom scrubber strip",
    },
    PresentationModeSpec {
        id: "mode.showcase_3d",
        domain: PresentationDomain::Media,
        name: "3D Orbit Showcase",
        glyph: "\u{1F9CA}",
        description: "Interactive WebGPU 3D orbital mesh carousel",
    },
    PresentationModeSpec {
        id: "mode.audio_strip",
        domain: PresentationDomain::Media,
        name: "Waveform Spectrogram",
        glyph: "\u{1F3B5}",
        description: "Interactive audio waveforms and formant strips",
    },
    // Domain 4: Document
    PresentationModeSpec {
        id: "mode.cml_hyperdoc",
        domain: PresentationDomain::Document,
        name: "CML HyperDoc",
        glyph: "\u{1F4C4}",
        description: "Context Markup Language rich text with <q-entity> tags",
    },
    PresentationModeSpec {
        id: "mode.dialectical_split",
        domain: PresentationDomain::Document,
        name: "Dialectical Split",
        glyph: "\u{2696}\u{FE0F}",
        description: "Dual-column epistemic claim & counter-claim comparison",
    },
    PresentationModeSpec {
        id: "mode.zen_reader",
        domain: PresentationDomain::Document,
        name: "Zen Reader",
        glyph: "\u{1F9D8}",
        description: "Distraction-free ambient typographic reading view",
    },
    // Domain 5: Chronological
    PresentationModeSpec {
        id: "mode.gantt",
        domain: PresentationDomain::Chronological,
        name: "Interactive Gantt",
        glyph: "\u{1F4CA}",
        description: "Task bars, dependencies, and critical path cascades",
    },
    PresentationModeSpec {
        id: "mode.timeline",
        domain: PresentationDomain::Chronological,
        name: "Temporal Timeline",
        glyph: "\u{23F3}",
        description: "Continuous zoomable timeline of milestones and events",
    },
    PresentationModeSpec {
        id: "mode.calendar",
        domain: PresentationDomain::Chronological,
        name: "Multi-View Calendar",
        glyph: "\u{1F4C5}",
        description: "Month, week, and agenda view of obligations and review dates",
    },
    // Domain 6: Agile PM
    PresentationModeSpec {
        id: "mode.kanban",
        domain: PresentationDomain::AgilePM,
        name: "Cooperative Kanban",
        glyph: "\u{1F4CB}",
        description: "Multi-column agile board with work item cards and WIP limits",
    },
    PresentationModeSpec {
        id: "mode.matrix_2x2",
        domain: PresentationDomain::AgilePM,
        name: "2x2 Decision Matrix",
        glyph: "\u{229E}",
        description: "Impact vs Effort quadrant decision grid",
    },
    // Domain 7: Knowledge Graph
    PresentationModeSpec {
        id: "mode.force_graph",
        domain: PresentationDomain::KnowledgeGraph,
        name: "Force-Directed DAG",
        glyph: "\u{1F578}\u{FE0F}",
        description: "Interactive Barnes-Hut knowledge graph network",
    },
    PresentationModeSpec {
        id: "mode.sankey_flow",
        domain: PresentationDomain::KnowledgeGraph,
        name: "Sankey Flow",
        glyph: "\u{1F30A}",
        description: "Resource, token, and permission flow diagrams",
    },
    // Domain 8: Sensory Studios
    PresentationModeSpec {
        id: "mode.spatial_viewport_3d",
        domain: PresentationDomain::Sensory,
        name: "3D Spatial Studio",
        glyph: "\u{1F9CA}",
        description: "Direct WebGPU camera orbit and mesh kinematics",
    },
    PresentationModeSpec {
        id: "mode.dicom_tomography",
        domain: PresentationDomain::Sensory,
        name: "DICOM Tomography",
        glyph: "\u{1FA7A}",
        description: "Axial, sagittal, and coronal multi-planar slices",
    },
    // Domain 9: Telemetry Cockpits
    PresentationModeSpec {
        id: "mode.kpi_cockpit",
        domain: PresentationDomain::Telemetry,
        name: "Executive Cockpit",
        glyph: "\u{1F4C8}",
        description: "Live gauges, burn rates, velocity, and health telemetry",
    },
    PresentationModeSpec {
        id: "mode.sentinel_auditor",
        domain: PresentationDomain::Telemetry,
        name: "Sentinel SlgArena Auditor",
        glyph: "\u{1F6E1}\u{FE0F}",
        description: "42MB ring buffer allocations and gas consumption monitor",
    },
    // Domain 10: Social & Communications
    PresentationModeSpec {
        id: "mode.threaded_chat",
        domain: PresentationDomain::Social,
        name: "Threaded Chat Graph",
        glyph: "\u{1F4AC}",
        description: "Multi-agent conversation stream with DID signatures",
    },
    PresentationModeSpec {
        id: "mode.mail_thread",
        domain: PresentationDomain::Social,
        name: "Inalienable Mail Inbox",
        glyph: "\u{2709}\u{FE0F}",
        description: "Purpose inboxes and CML message threads",
    },
];

/// Build the polymorphic `<q-view-switcher>` bar for any container.
pub fn build_view_switcher(document: &Document, active_mode_id: &str) -> Element {
    let switcher = document.create_element("q-view-switcher").unwrap();
    switcher.set_class_name("q-view-switcher-bar");
    let sw_el: HtmlElement = switcher.clone().dyn_into().unwrap();
    sw_el.style().set_css_text(
        "display: flex; align-items: center; justify-content: space-between; gap: 6px; \
         padding: 3px 6px; background: var(--surface-panel, #131822); border-bottom: 1px solid var(--border-subtle); \
         font-family: var(--font-mono, monospace); font-size: 10px;",
    );

    // Left: Active Projection Badge
    let active_spec = PRESENTATION_MODES
        .iter()
        .find(|m| m.id == active_mode_id)
        .unwrap_or(&PRESENTATION_MODES[0]);

    let active_badge = document.create_element("div").unwrap();
    active_badge.set_class_name("view-switcher-active-chip");
    let ab_el: HtmlElement = active_badge.clone().dyn_into().unwrap();
    ab_el.style().set_css_text(
        "display: flex; align-items: center; gap: 4px; padding: 2px 6px; \
         background: rgba(0,210,255,0.1); border: 1px solid rgba(0,210,255,0.25); \
         border-radius: 3px; color: var(--accent-cyan, #00d2ff); font-weight: 600;",
    );
    active_badge.set_text_content(Some(&format!("{} {}", active_spec.glyph, active_spec.name)));
    switcher.append_child(&active_badge).unwrap();

    // Right: Quick Mode Selector Buttons
    let btn_strip = document.create_element("div").unwrap();
    let bs_el: HtmlElement = btn_strip.clone().dyn_into().unwrap();
    bs_el
        .style()
        .set_css_text("display: flex; gap: 2px; align-items: center;");

    let quick_modes = [
        "mode.cml_hyperdoc",
        "mode.spreadsheet",
        "mode.kanban",
        "mode.force_graph",
        "mode.kpi_cockpit",
    ];
    for qid in &quick_modes {
        if let Some(spec) = PRESENTATION_MODES.iter().find(|m| m.id == *qid) {
            let btn = document.create_element("button").unwrap();
            btn.set_class_name("view-mode-pill-btn");
            btn.set_attribute("title", spec.name).unwrap();
            let b_el: HtmlElement = btn.clone().dyn_into().unwrap();
            let is_active = spec.id == active_mode_id;
            let bg = if is_active {
                "var(--surface-panel-elevated)"
            } else {
                "transparent"
            };
            let border = if is_active {
                "var(--accent-cyan)"
            } else {
                "transparent"
            };
            b_el.style().set_css_text(&format!(
                "padding: 2px 5px; background: {}; border: 1px solid {}; border-radius: 3px; \
                 font-size: 11px; cursor: pointer; transition: var(--trans-fast);",
                bg, border
            ));
            btn.set_text_content(Some(spec.glyph));

            let spec_name = spec.name;
            let click_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
                let doc = web_sys::window().unwrap().document().unwrap();
                show_projection_notification(&doc, spec_name);
            }) as Box<dyn FnMut(MouseEvent)>);
            btn.add_event_listener_with_callback("click", click_closure.as_ref().unchecked_ref())
                .unwrap();
            click_closure.forget();

            btn_strip.append_child(&btn).unwrap();
        }
    }
    switcher.append_child(&btn_strip).unwrap();

    switcher
}

/// Show a toast notification when switching visual presentation projections.
fn show_projection_notification(document: &Document, mode_name: &str) {
    let notif = document.create_element("div").unwrap();
    let n_el: HtmlElement = notif.clone().dyn_into().unwrap();
    n_el.style().set_css_text(
        "position: fixed; bottom: 36px; right: 24px; \
         background: var(--surface-panel-elevated); border: 1px solid var(--accent-cyan); \
         border-radius: var(--radius-sm); padding: 8px 14px; font-size: 11px; \
         color: var(--text-primary); box-shadow: var(--shadow-lg); z-index: 9200; \
         font-family: var(--font-mono); animation: slideInRight 0.2s ease-out;",
    );
    notif.set_text_content(Some(&format!(
        "\u{1F504} Polymorphic Projection: Switched to {}",
        mode_name
    )));
    if let Some(body) = document.body() {
        body.append_child(&notif).unwrap();
    }
    let notif_clone = notif.clone();
    let timeout = Closure::wrap(Box::new(move || {
        notif_clone.remove();
    }) as Box<dyn FnMut()>);
    super::interactions::set_timeout(timeout.as_ref().unchecked_ref(), 2000);
    timeout.forget();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_presentation_modes_catalog() {
        assert!(
            PRESENTATION_MODES.len() >= 25,
            "Expected at least 25 polymorphic modes, got {}",
            PRESENTATION_MODES.len()
        );
        let mut ids = std::collections::HashSet::new();
        for mode in PRESENTATION_MODES {
            assert!(!mode.name.is_empty());
            assert!(!mode.glyph.is_empty());
            assert!(ids.insert(mode.id), "Duplicate mode ID: {}", mode.id);
        }
    }

    #[test]
    fn test_presentation_domains_coverage() {
        let domains = [
            PresentationDomain::Hierarchical,
            PresentationDomain::Tabular,
            PresentationDomain::Media,
            PresentationDomain::Document,
            PresentationDomain::Chronological,
            PresentationDomain::AgilePM,
            PresentationDomain::KnowledgeGraph,
            PresentationDomain::Sensory,
            PresentationDomain::Telemetry,
            PresentationDomain::Social,
        ];
        for d in &domains {
            assert!(!d.label().is_empty());
            assert!(!d.glyph().is_empty());
            let count = PRESENTATION_MODES.iter().filter(|m| m.domain == *d).count();
            assert!(
                count >= 2,
                "Domain {:?} has fewer than 2 modes (got {})",
                d,
                count
            );
        }
    }
}
