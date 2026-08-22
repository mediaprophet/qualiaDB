//! Remote Control — compact control surface for phone/remote devices (P0).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const MANIFOLDS: &[(&str, &str, bool)] = &[
    ("research", "Research", false),
    ("media", "Media", false),
    ("social", "Social", false),
    ("communications", "Comms", false),
    ("knowledge", "Knowledge", false),
    ("ontology", "Ontology", true),
    ("projects", "Projects", false),
    ("rights", "Rights", false),
    ("sanctuary", "Sanctuary", false),
    ("health", "Health", false),
    ("studio", "Studio", false),
    ("datasets", "Datasets", false),
    ("settings", "Settings", false),
    ("devices", "Devices", false),
    ("vibe", "Vibe", false),
];

const QUICK_ACTIONS: &[(&str, &str)] = &[
    ("\u{25B6}", "Play"),
    ("\u{23F8}", "Pause"),
    ("\u{23ED}", "Next"),
    ("\u{23EE}", "Prev"),
    ("\u{1F4DD}", "Note"),
    ("\u{1F50D}", "Search"),
    ("\u{2705}", "Approve"),
    ("\u{274C}", "Reject"),
    ("\u{1F4E7}", "Send"),
    ("\u{1F4DE}", "Call"),
    ("\u{1F4F7}", "Capture"),
    ("\u{1F3A4}", "Record"),
];

pub fn build_remote_control_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: row; flex: 1; gap: 8px; overflow: hidden; \
         padding: 4px 8px; align-items: center;",
    );

    // Manifold switcher (left)
    let manifolds_section = document.create_element("div").unwrap();
    let m_el: HtmlElement = manifolds_section.clone().dyn_into().unwrap();
    m_el.style().set_css_text(
        "display: flex; flex-direction: column; gap: 2px; flex-shrink: 0; \
         border-right: 1px solid var(--border-subtle); padding-right: 8px;",
    );

    let manifolds_label = document.create_element("div").unwrap();
    manifolds_label.set_text_content(Some("Manifolds"));
    let ml_el: HtmlElement = manifolds_label.clone().dyn_into().unwrap();
    ml_el.style().set_css_text(
        "font-size: 7px; font-weight: 700; color: var(--text-muted); \
         font-family: var(--font-mono); text-transform: uppercase; margin-bottom: 2px;",
    );
    manifolds_section.append_child(&manifolds_label).unwrap();

    let manifolds_grid = document.create_element("div").unwrap();
    let mg_el: HtmlElement = manifolds_grid.clone().dyn_into().unwrap();
    mg_el
        .style()
        .set_css_text("display: flex; flex-wrap: wrap; gap: 2px; max-width: 200px;");

    for (id, label, is_active) in MANIFOLDS {
        let btn = document.create_element("button").unwrap();
        btn.set_text_content(Some(label));
        let b_el: HtmlElement = btn.clone().dyn_into().unwrap();
        let bg = if *is_active {
            "rgba(0, 200, 255, 0.15)"
        } else {
            "transparent"
        };
        let color = if *is_active {
            "var(--accent-cyan)"
        } else {
            "var(--text-secondary)"
        };
        let border = if *is_active {
            "var(--accent-cyan)"
        } else {
            "var(--border-medium)"
        };
        b_el.style().set_css_text(&format!(
            "padding: 3px 6px; border: 1px solid {}; background: {}; color: {}; \
             border-radius: 3px; cursor: pointer; font-size: 7px; font-family: var(--font-mono); \
             font-weight: {};",
            border,
            bg,
            color,
            if *is_active { "600" } else { "400" },
        ));
        b_el.set_attribute("data-manifold", id).unwrap();
        manifolds_grid.append_child(&btn).unwrap();
    }
    manifolds_section.append_child(&manifolds_grid).unwrap();
    wrapper.append_child(&manifolds_section).unwrap();

    // Quick actions (center)
    let actions_section = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions_section.clone().dyn_into().unwrap();
    a_el.style().set_css_text(
        "display: flex; flex-direction: column; gap: 2px; flex-shrink: 0; \
         border-right: 1px solid var(--border-subtle); padding-right: 8px;",
    );

    let actions_label = document.create_element("div").unwrap();
    actions_label.set_text_content(Some("Quick Actions"));
    let al_el: HtmlElement = actions_label.clone().dyn_into().unwrap();
    al_el.style().set_css_text(
        "font-size: 7px; font-weight: 700; color: var(--text-muted); \
         font-family: var(--font-mono); text-transform: uppercase; margin-bottom: 2px;",
    );
    actions_section.append_child(&actions_label).unwrap();

    let actions_grid = document.create_element("div").unwrap();
    let ag_el: HtmlElement = actions_grid.clone().dyn_into().unwrap();
    ag_el
        .style()
        .set_css_text("display: grid; grid-template-columns: repeat(6, 1fr); gap: 2px;");

    for (icon, label) in QUICK_ACTIONS {
        let btn = document.create_element("button").unwrap();
        btn.set_text_content(Some(icon));
        let b_el: HtmlElement = btn.clone().dyn_into().unwrap();
        b_el.style().set_css_text(
            "width: 28px; height: 28px; border: 1px solid var(--border-medium); \
             background: var(--surface-panel); color: var(--text-primary); border-radius: 4px; \
             cursor: pointer; font-size: 14px; display: flex; align-items: center; \
             justify-content: center;",
        );
        b_el.set_attribute("title", label).unwrap();
        actions_grid.append_child(&btn).unwrap();
    }
    actions_section.append_child(&actions_grid).unwrap();
    wrapper.append_child(&actions_section).unwrap();

    // Status + device info (right)
    let status_section = document.create_element("div").unwrap();
    let s_el: HtmlElement = status_section.clone().dyn_into().unwrap();
    s_el.style()
        .set_css_text("flex: 1; display: flex; flex-direction: column; gap: 2px;");

    let status_label = document.create_element("div").unwrap();
    status_label.set_text_content(Some("Status"));
    let sl_el: HtmlElement = status_label.clone().dyn_into().unwrap();
    sl_el.style().set_css_text(
        "font-size: 7px; font-weight: 700; color: var(--text-muted); \
         font-family: var(--font-mono); text-transform: uppercase; margin-bottom: 2px;",
    );
    status_section.append_child(&status_label).unwrap();

    let status_items = [
        ("Active Manifold:", "Ontology"),
        ("Active Container:", "Semantic Graph Canvas"),
        ("Workspace:", "v47 \u{2713} synced"),
        ("Peers Online:", "4 devices"),
        ("This Device:", "Pixel 9 Pro (Remote)"),
        ("Latency:", "45ms to desktop-01"),
        ("Battery:", "78%"),
        ("Signal:", "\u{1F4F6}\u{1F4F6}\u{1F4F6}\u{1F4F6}"),
    ];

    for (key, val) in &status_items {
        let row = document.create_element("div").unwrap();
        let r_el: HtmlElement = row.clone().dyn_into().unwrap();
        r_el.style()
            .set_css_text("display: flex; justify-content: space-between;");

        let key_span = document.create_element("span").unwrap();
        key_span.set_text_content(Some(key));
        let k_el: HtmlElement = key_span.clone().dyn_into().unwrap();
        k_el.style().set_css_text(
            "font-size: 7px; color: var(--text-muted); font-family: var(--font-mono);",
        );
        row.append_child(&key_span).unwrap();

        let val_span = document.create_element("span").unwrap();
        val_span.set_text_content(Some(val));
        let v_el: HtmlElement = val_span.clone().dyn_into().unwrap();
        v_el.style().set_css_text(
            "font-size: 7px; color: var(--text-primary); font-family: var(--font-mono); \
             font-weight: 600;",
        );
        row.append_child(&val_span).unwrap();
        status_section.append_child(&row).unwrap();
    }
    wrapper.append_child(&status_section).unwrap();

    wrapper
}
