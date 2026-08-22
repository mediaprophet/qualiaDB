//! 10D Semantic Manifold Scrubber & Projection Navigator (Subsystem 4.4).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//!
//! Allows continuous traversal through the 10-dimensional topological manifold
//! used by the QualiaDB Tensor10D engine and WebizenVM.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const DIM_LABELS: &[(&str, &str, &str)] = &[
    ("D0: Epistemic (Cert)", "0.94", "#3b82f6"),
    ("D1: Deontic (Norm)", "0.82", "#10b981"),
    ("D2: Spatial X", "12.4", "#f59e0b"),
    ("D3: Spatial Y", "-4.8", "#ef4444"),
    ("D4: Spatial Z", "3.2", "#8b5cf6"),
    ("D5: Temporal Lamport", "142.0", "#ec4899"),
    ("D6: Paraconsistency", "0.05", "#06b6d4"),
    ("D7: Sensitivity Class", "0.0", "#84cc16"),
    ("D8: Modality Weight", "0.76", "#eab308"),
    ("D9: Permissive Lane", "1.0", "#6366f1"),
];

/// Build the 10D Hyper-Dimensional Manifold Scrubber container view.
pub fn build_spatial_10d_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 8px; \
         padding: 10px; background: #0f172a; color: #f8fafc; overflow-y: auto;"
    );

    // Header HUD
    let hud = document.create_element("div").unwrap();
    hud.set_class_name("vibe-toolbar");
    let hud_el: HtmlElement = hud.clone().dyn_into().unwrap();
    hud_el.style().set_css_text(
        "justify-content: space-between; background: rgba(30, 41, 59, 0.7); \
         border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 6px; padding: 6px 10px;"
    );

    let title = document.create_element("span").unwrap();
    title.set_text_content(Some("\u{25CE} 10D Manifold Scrubber"));
    let title_el: HtmlElement = title.clone().dyn_into().unwrap();
    title_el.style().set_css_text("font-weight: 600; font-size: 12px; color: #38bdf8;");
    hud.append_child(&title).unwrap();

    let coords = document.create_element("span").unwrap();
    coords.set_text_content(Some("Norm: ||v|| = 143.19 \u{00B7} Curvature: \u{03BA} = 0.0042"));
    let coords_el: HtmlElement = coords.clone().dyn_into().unwrap();
    coords_el.style().set_css_text("font-family: var(--font-mono); font-size: 10px; color: #94a3b8;");
    hud.append_child(&coords).unwrap();

    wrapper.append_child(&hud).unwrap();

    // 10D Dimension Sliders Grid
    let grid = document.create_element("div").unwrap();
    let grid_el: HtmlElement = grid.clone().dyn_into().unwrap();
    grid_el.style().set_css_text(
        "display: grid; grid-template-columns: repeat(auto-fit, minmax(220px, 1fr)); \
         gap: 6px; flex: 1;"
    );

    for (label, val, color) in DIM_LABELS {
        let card = document.create_element("div").unwrap();
        let card_el: HtmlElement = card.clone().dyn_into().unwrap();
        card_el.style().set_css_text(
            "background: rgba(30, 41, 59, 0.5); border: 1px solid rgba(255, 255, 255, 0.05); \
             border-radius: 4px; padding: 6px 8px; display: flex; flex-direction: column; gap: 4px;"
        );

        let row = document.create_element("div").unwrap();
        let row_el: HtmlElement = row.clone().dyn_into().unwrap();
        row_el.style().set_css_text("display: flex; justify-content: space-between; font-size: 10px;");

        let name = document.create_element("span").unwrap();
        name.set_text_content(Some(label));
        let name_el: HtmlElement = name.clone().dyn_into().unwrap();
        name_el.style().set_css_text(&format!("color: {}; font-weight: 500;", color));
        row.append_child(&name).unwrap();

        let num = document.create_element("span").unwrap();
        num.set_text_content(Some(val));
        let num_el: HtmlElement = num.clone().dyn_into().unwrap();
        num_el.style().set_css_text("font-family: var(--font-mono); opacity: 0.8;");
        row.append_child(&num).unwrap();

        card.append_child(&row).unwrap();

        // Track bar
        let track = document.create_element("div").unwrap();
        let track_el: HtmlElement = track.clone().dyn_into().unwrap();
        track_el.style().set_css_text(
            "height: 6px; background: rgba(0,0,0,0.3); border-radius: 3px; position: relative; overflow: hidden;"
        );

        let fill = document.create_element("div").unwrap();
        let fill_el: HtmlElement = fill.clone().dyn_into().unwrap();
        fill_el.style().set_css_text(&format!(
            "height: 100%; width: 65%; background: {}; border-radius: 3px;",
            color
        ));
        track.append_child(&fill).unwrap();
        card.append_child(&track).unwrap();

        grid.append_child(&card).unwrap();
    }

    wrapper.append_child(&grid).unwrap();

    // Controls bar
    let ctrl = document.create_element("div").unwrap();
    ctrl.set_class_name("vibe-toolbar");
    for btn_label in &["Project 3D", "Orthonormalize", "Reset Origin", "Capture Trajectory"] {
        let btn = document.create_element("button").unwrap();
        btn.set_class_name("vibe-run-btn");
        btn.set_text_content(Some(btn_label));
        ctrl.append_child(&btn).unwrap();
    }
    wrapper.append_child(&ctrl).unwrap();

    wrapper
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dim_labels_count() {
        assert_eq!(DIM_LABELS.len(), 10);
    }
}
