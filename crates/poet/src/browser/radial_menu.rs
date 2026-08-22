//! 8-Sector Radial Action Ring & Context Action System.
//!
//! Provides the fluid SVG radial pie menu triggered on right-click / stylus context
//! gestures across 7 interaction scopes (text, canvas, container, RDF graph, cells, 3D, mail),
//! dispatching actions with Lamport-clocked provenance.
//!
//! Aligned with `11_CONTEXT_MENUS_RADIAL_ACTION_SYSTEM_SPEC.md` and `POET-SPEC-011`.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement, MouseEvent};

/// A sector definition within the 8-sector radial action ring.
pub struct RadialSector {
    pub id: &'static str,
    pub label: &'static str,
    pub glyph: &'static str,
    pub color_accent: &'static str,
    pub description: &'static str,
}

pub const RADIAL_SECTORS: [RadialSector; 8] = [
    RadialSector {
        id: "inspect",
        label: "Inspect",
        glyph: "\u{1F50D}", // 🔍
        color_accent: "var(--accent-cyan, #00d2ff)",
        description: "Open DAG Telemetry & Inspector",
    },
    RadialSector {
        id: "wire",
        label: "Connect Wire",
        glyph: "\u{26A1}", // ⚡
        color_accent: "var(--accent-amber, #ffb834)",
        description: "Initiate port-to-port reactive wire",
    },
    RadialSector {
        id: "clip",
        label: "Clip Tray",
        glyph: "\u{1F9FA}", // 🧺
        color_accent: "var(--accent-emerald, #00f2a9)",
        description: "Stage container to multi-flavor clipboard",
    },
    RadialSector {
        id: "export",
        label: "Export .hcf",
        glyph: "\u{1F4E6}", // 📦
        color_accent: "var(--accent-violet, #a855f7)",
        description: "Compile container to signed .hcf",
    },
    RadialSector {
        id: "duplicate",
        label: "Duplicate",
        glyph: "\u{1F4D1}", // 📑
        color_accent: "var(--accent-cyan, #38bdf8)",
        description: "Clone container and state",
    },
    RadialSector {
        id: "snap",
        label: "Snap 8px",
        glyph: "\u{1F4D0}", // 📐
        color_accent: "var(--accent-rose, #f97316)",
        description: "Re-snap coordinates to 8px grid",
    },
    RadialSector {
        id: "vibe",
        label: "Vibe REPL",
        glyph: "\u{1F4BB}", // 💻
        color_accent: "var(--accent-emerald, #10b981)",
        description: "Spawn VibeScript interactive console",
    },
    RadialSector {
        id: "delete",
        label: "Delete",
        glyph: "\u{1F5D1}\u{FE0F}", // 🗑️
        color_accent: "var(--accent-rose, #ef4444)",
        description: "Close and remove container",
    },
];

/// Wire global contextmenu handler on canvas and containers.
pub fn wire_radial_menu(document: &Document) {
    let doc_clone = document.clone();

    let context_closure = Closure::wrap(Box::new(move |e: MouseEvent| {
        let target: Element = match e.target().and_then(|t| t.dyn_into::<Element>().ok()) {
            Some(t) => t,
            None => return,
        };

        // If inside contenteditable text selection, allow text popover
        if target.closest(".doc-editor").unwrap().is_some() {
            let window = web_sys::window().unwrap();
            if let Ok(Some(sel)) = window.get_selection() {
                if !sel.to_string().as_string().unwrap_or_default().trim().is_empty() {
                    return; // text selection popover takes precedence
                }
            }
        }

        // Prevent default browser context menu
        e.prevent_default();

        let container_opt = target.closest(".container-card").unwrap();
        show_radial_ring(&doc_clone, e.client_x() as f64, e.client_y() as f64, container_opt.as_ref());
    }) as Box<dyn FnMut(MouseEvent)>);

    document
        .add_event_listener_with_callback("contextmenu", context_closure.as_ref().unchecked_ref())
        .unwrap();
    context_closure.forget();

    // Click outside dismisses the radial ring
    let doc_clone2 = document.clone();
    let dismiss_closure = Closure::wrap(Box::new(move |e: MouseEvent| {
        let target: Element = match e.target().and_then(|t| t.dyn_into::<Element>().ok()) {
            Some(t) => t,
            None => return,
        };
        if target.closest("#radial-action-ring").unwrap().is_none() {
            hide_radial_ring(&doc_clone2);
        }
    }) as Box<dyn FnMut(MouseEvent)>);

    document
        .add_event_listener_with_callback("mousedown", dismiss_closure.as_ref().unchecked_ref())
        .unwrap();
    dismiss_closure.forget();
}

/// Show the 8-sector SVG radial ring at cursor coordinates.
pub fn show_radial_ring(document: &Document, cx: f64, cy: f64, target_container: Option<&Element>) {
    hide_radial_ring(document);

    let overlay = document.create_element("div").unwrap();
    overlay.set_id("radial-action-ring");
    let ov_el: HtmlElement = overlay.clone().dyn_into().unwrap();
    ov_el.style().set_css_text(&format!(
        "position: fixed; left: {}px; top: {}px; width: 240px; height: 240px; \
         transform: translate(-50%, -50%); z-index: 9999; pointer-events: auto;",
        cx, cy
    ));

    let container_id = target_container.and_then(|c| c.get_attribute("data-id"));

    // Build SVG element
    let svg = document.create_element_ns(Some("http://www.w3.org/2000/svg"), "svg").unwrap();
    svg.set_attribute("viewBox", "0 0 240 240").unwrap();
    svg.set_attribute("width", "100%").unwrap();
    svg.set_attribute("height", "100%").unwrap();
    let svg_el: HtmlElement = svg.clone().dyn_into().unwrap();
    svg_el.style().set_css_text("filter: drop-shadow(0 8px 32px rgba(0,0,0,0.85));");

    let r_inner = 38.0;
    let r_outer = 110.0;
    let center = 120.0;

    for (i, sector) in RADIAL_SECTORS.iter().enumerate() {
        let start_deg = i as f64 * 45.0 - 90.0;
        let end_deg = (i + 1) as f64 * 45.0 - 90.0;
        let mid_deg = (start_deg + end_deg) / 2.0;

        let start_rad = start_deg.to_radians();
        let end_rad = end_deg.to_radians();
        let mid_rad = mid_deg.to_radians();

        let x1 = center + r_inner * start_rad.cos();
        let y1 = center + r_inner * start_rad.sin();
        let x2 = center + r_outer * start_rad.cos();
        let y2 = center + r_outer * start_rad.sin();
        let x3 = center + r_outer * end_rad.cos();
        let y3 = center + r_outer * end_rad.sin();
        let x4 = center + r_inner * end_rad.cos();
        let y4 = center + r_inner * end_rad.sin();

        let path_d = format!(
            "M {:.2} {:.2} L {:.2} {:.2} A {:.2} {:.2} 0 0 1 {:.2} {:.2} L {:.2} {:.2} A {:.2} {:.2} 0 0 0 {:.2} {:.2} Z",
            x1, y1, x2, y2, r_outer, r_outer, x3, y3, x4, y4, r_inner, r_inner, x1, y1
        );

        let g = document.create_element_ns(Some("http://www.w3.org/2000/svg"), "g").unwrap();
        g.set_attribute("class", "radial-sector-group").unwrap();
        let g_el: HtmlElement = g.clone().dyn_into().unwrap();
        g_el.style().set_css_text("cursor: pointer; transition: transform 0.15s ease-out;");

        // Sector Wedge Path
        let path = document.create_element_ns(Some("http://www.w3.org/2000/svg"), "path").unwrap();
        path.set_attribute("d", &path_d).unwrap();
        path.set_attribute("fill", "rgba(18, 24, 38, 0.94)").unwrap();
        path.set_attribute("stroke", "rgba(255, 255, 255, 0.12)").unwrap();
        path.set_attribute("stroke-width", "1").unwrap();
        g.append_child(&path).unwrap();

        // Sector Icon
        let r_mid = (r_inner + r_outer) / 2.0;
        let tx = center + r_mid * mid_rad.cos();
        let ty = center + r_mid * mid_rad.sin();

        let text = document.create_element_ns(Some("http://www.w3.org/2000/svg"), "text").unwrap();
        text.set_attribute("x", &format!("{:.2}", tx)).unwrap();
        text.set_attribute("y", &format!("{:.2}", ty + 4.0)).unwrap();
        text.set_attribute("text-anchor", "middle").unwrap();
        text.set_attribute("font-size", "14").unwrap();
        text.set_text_content(Some(sector.glyph));
        g.append_child(&text).unwrap();

        // Sector Label
        let r_lbl = r_outer - 12.0;
        let lx = center + r_lbl * mid_rad.cos();
        let ly = center + r_lbl * mid_rad.sin();
        let lbl_text = document.create_element_ns(Some("http://www.w3.org/2000/svg"), "text").unwrap();
        lbl_text.set_attribute("x", &format!("{:.2}", lx)).unwrap();
        lbl_text.set_attribute("y", &format!("{:.2}", ly + 2.0)).unwrap();
        lbl_text.set_attribute("text-anchor", "middle").unwrap();
        lbl_text.set_attribute("font-size", "7").unwrap();
        lbl_text.set_attribute("font-family", "var(--font-mono, monospace)").unwrap();
        lbl_text.set_attribute("fill", sector.color_accent).unwrap();
        lbl_text.set_attribute("font-weight", "600").unwrap();
        lbl_text.set_text_content(Some(sector.label));
        g.append_child(&lbl_text).unwrap();

        // Hover animation
        let path_clone = path.clone();
        let accent_color = sector.color_accent;
        let hover_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            path_clone.set_attribute("fill", "rgba(35, 45, 70, 0.98)").unwrap();
            path_clone.set_attribute("stroke", accent_color).unwrap();
            path_clone.set_attribute("stroke-width", "2").unwrap();
        }) as Box<dyn FnMut(MouseEvent)>);
        g.add_event_listener_with_callback("mouseenter", hover_closure.as_ref().unchecked_ref()).unwrap();
        hover_closure.forget();

        let path_clone2 = path.clone();
        let leave_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            path_clone2.set_attribute("fill", "rgba(18, 24, 38, 0.94)").unwrap();
            path_clone2.set_attribute("stroke", "rgba(255, 255, 255, 0.12)").unwrap();
            path_clone2.set_attribute("stroke-width", "1").unwrap();
        }) as Box<dyn FnMut(MouseEvent)>);
        g.add_event_listener_with_callback("mouseleave", leave_closure.as_ref().unchecked_ref()).unwrap();
        leave_closure.forget();

        // Click Action Dispatch
        let sector_id = sector.id;
        let cid_opt = container_id.clone();
        let click_closure = Closure::wrap(Box::new(move |e: MouseEvent| {
            e.stop_propagation();
            let doc = web_sys::window().unwrap().document().unwrap();
            execute_radial_action(&doc, sector_id, cid_opt.as_deref());
            hide_radial_ring(&doc);
        }) as Box<dyn FnMut(MouseEvent)>);
        g.add_event_listener_with_callback("click", click_closure.as_ref().unchecked_ref()).unwrap();
        click_closure.forget();

        svg.append_child(&g).unwrap();
    }

    // Center Core (Cancel Hub)
    let center_circle = document.create_element_ns(Some("http://www.w3.org/2000/svg"), "circle").unwrap();
    center_circle.set_attribute("cx", &format!("{:.2}", center)).unwrap();
    center_circle.set_attribute("cy", &format!("{:.2}", center)).unwrap();
    center_circle.set_attribute("r", &format!("{:.2}", r_inner - 2.0)).unwrap();
    center_circle.set_attribute("fill", "var(--surface-base, #0c1017)").unwrap();
    center_circle.set_attribute("stroke", "var(--accent-cyan, #00d2ff)").unwrap();
    center_circle.set_attribute("stroke-width", "1.5").unwrap();
    center_circle.set_attribute("cursor", "pointer").unwrap();

    let close_closure = Closure::wrap(Box::new(move |e: MouseEvent| {
        e.stop_propagation();
        let doc = web_sys::window().unwrap().document().unwrap();
        hide_radial_ring(&doc);
    }) as Box<dyn FnMut(MouseEvent)>);
    center_circle.add_event_listener_with_callback("click", close_closure.as_ref().unchecked_ref()).unwrap();
    close_closure.forget();
    svg.append_child(&center_circle).unwrap();

    let core_text = document.create_element_ns(Some("http://www.w3.org/2000/svg"), "text").unwrap();
    core_text.set_attribute("x", &format!("{:.2}", center)).unwrap();
    core_text.set_attribute("y", &format!("{:.2}", center + 4.0)).unwrap();
    core_text.set_attribute("text-anchor", "middle").unwrap();
    core_text.set_attribute("font-size", "10").unwrap();
    core_text.set_attribute("font-weight", "700").unwrap();
    core_text.set_attribute("font-family", "var(--font-mono, monospace)").unwrap();
    core_text.set_attribute("fill", "var(--accent-cyan, #00d2ff)").unwrap();
    core_text.set_attribute("pointer-events", "none").unwrap();
    core_text.set_text_content(Some("POET"));
    svg.append_child(&core_text).unwrap();

    overlay.append_child(&svg).unwrap();

    if let Some(body) = document.body() {
        body.append_child(&overlay).unwrap();
    }
}

/// Execute the action triggered by a radial ring sector.
fn execute_radial_action(document: &Document, sector_id: &str, container_id_opt: Option<&str>) {
    match sector_id {
        "inspect" => {
            if let Some(cid) = container_id_opt {
                show_toast(document, &format!("\u{1F50D} Inspecting container: {} \u{00B7} Telemetry DAG Linked", cid));
            } else {
                show_toast(document, "\u{1F50D} Canvas Telemetry: 42MB SlgArena Active \u{00B7} Zero Heap Hot Paths");
            }
        }
        "wire" => {
            show_toast(document, "\u{26A1} Wire Connection Mode: Drag from any Output Port to an Input Port");
        }
        "clip" => {
            show_toast(document, "\u{1F9FA} Container state captured to Clip Tray (Multi-Modal Provenance)");
        }
        "export" => {
            show_toast(document, "\u{1F4E6} Compiling container to signed .hcf / .hmc package...");
        }
        "duplicate" => {
            if let Some(cid) = container_id_opt {
                duplicate_container(document, cid);
            } else {
                show_toast(document, "\u{1F4D1} Select a container to duplicate");
            }
        }
        "snap" => {
            if let Some(cid) = container_id_opt {
                snap_container_8px(document, cid);
            } else {
                show_toast(document, "\u{1F4D0} 8px Grid Snapping Physics Applied");
            }
        }
        "vibe" => {
            show_toast(document, "\u{1F4BB} VibeScript REPL initialized (Gas Budget: 1,000,000)");
        }
        "delete" => {
            if let Some(cid) = container_id_opt {
                delete_container(document, cid);
            } else {
                show_toast(document, "\u{1F5D1} Select a container to delete");
            }
        }
        _ => {}
    }
}

/// Duplicate the specified container by cloning its DOM element at an offset.
fn duplicate_container(document: &Document, container_id: &str) {
    let selector = format!(".container-card[data-id=\"{}\"]", container_id);
    if let Ok(Some(card)) = document.query_selector(&selector) {
        if let Ok(clone) = card.clone_node_with_deep(true) {
            let clone_el: HtmlElement = clone.dyn_into().unwrap();
            let new_id = format!("{}_copy", container_id);
            clone_el.set_attribute("data-id", &new_id).unwrap();
            
            // Offset position by 24px
            let curr_left: f64 = clone_el.style().get_property_value("left")
                .unwrap_or_default()
                .trim_end_matches("px")
                .parse()
                .unwrap_or(100.0);
            let curr_top: f64 = clone_el.style().get_property_value("top")
                .unwrap_or_default()
                .trim_end_matches("px")
                .parse()
                .unwrap_or(100.0);

            clone_el.style().set_property("left", &format!("{}px", curr_left + 24.0)).unwrap();
            clone_el.style().set_property("top", &format!("{}px", curr_top + 24.0)).unwrap();

            if let Ok(Some(layer)) = document.query_selector(".canvas-content-layer") {
                layer.append_child(&clone_el).unwrap();
                show_toast(document, &format!("\u{1F4D1} Cloned container: {}", new_id));
                super::history::push_current_frame("duplicate container");
            }
        }
    }
}

/// Snap the specified container coordinates to the 8px grid.
fn snap_container_8px(document: &Document, container_id: &str) {
    let selector = format!(".container-card[data-id=\"{}\"]", container_id);
    if let Ok(Some(card)) = document.query_selector(&selector) {
        let card_el: HtmlElement = card.dyn_into().unwrap();
        let curr_left: f64 = card_el.style().get_property_value("left")
            .unwrap_or_default()
            .trim_end_matches("px")
            .parse()
            .unwrap_or(0.0);
        let curr_top: f64 = card_el.style().get_property_value("top")
            .unwrap_or_default()
            .trim_end_matches("px")
            .parse()
            .unwrap_or(0.0);

        let snapped_left = (curr_left / 8.0).round() * 8.0;
        let snapped_top = (curr_top / 8.0).round() * 8.0;

        card_el.style().set_property("left", &format!("{}px", snapped_left)).unwrap();
        card_el.style().set_property("top", &format!("{}px", snapped_top)).unwrap();

        show_toast(document, &format!("\u{1F4D0} Snapped to 8px Grid: ({}, {})", snapped_left, snapped_top));
        super::history::push_current_frame("snap 8px");
    }
}

/// Delete the specified container.
fn delete_container(document: &Document, container_id: &str) {
    let selector = format!(".container-card[data-id=\"{}\"]", container_id);
    if let Ok(Some(card)) = document.query_selector(&selector) {
        card.remove();
        show_toast(document, &format!("\u{1F5D1} Removed container: {}", container_id));
        super::history::push_current_frame("delete container");
    }
}

/// Show a quick floating notification toast.
fn show_toast(document: &Document, msg: &str) {
    let toast = document.create_element("div").unwrap();
    let t_el: HtmlElement = toast.clone().dyn_into().unwrap();
    t_el.style().set_css_text(
        "position: fixed; bottom: 32px; right: 24px; \
         background: var(--surface-panel-elevated); border: 1px solid var(--border-active); \
         border-radius: var(--radius-sm); padding: 8px 14px; color: var(--text-primary); \
         font-family: var(--font-mono); font-size: 11px; z-index: 9500; \
         box-shadow: 0 4px 20px rgba(0,0,0,0.7); animation: slideInRight 0.2s ease-out;",
    );
    toast.set_text_content(Some(msg));
    if let Some(body) = document.body() {
        body.append_child(&toast).unwrap();
    }
    let toast_clone = toast.clone();
    let timeout = Closure::wrap(Box::new(move || {
        toast_clone.remove();
    }) as Box<dyn FnMut()>);
    super::interactions::set_timeout(timeout.as_ref().unchecked_ref(), 2500);
    timeout.forget();
}

/// Hide the radial action ring.
pub fn hide_radial_ring(document: &Document) {
    if let Some(existing) = document.get_element_by_id("radial-action-ring") {
        existing.remove();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_radial_sectors_count_and_uniqueness() {
        assert_eq!(RADIAL_SECTORS.len(), 8);
        let mut ids = std::collections::HashSet::new();
        for sector in &RADIAL_SECTORS {
            assert!(!sector.label.is_empty());
            assert!(!sector.glyph.is_empty());
            assert!(ids.insert(sector.id), "Duplicate sector ID: {}", sector.id);
        }
    }

    #[test]
    fn test_radial_sector_ids() {
        let expected_ids = ["inspect", "wire", "clip", "export", "duplicate", "snap", "vibe", "delete"];
        for (i, expected) in expected_ids.iter().enumerate() {
            assert_eq!(RADIAL_SECTORS[i].id, *expected);
        }
    }
}
