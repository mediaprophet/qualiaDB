//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Triad and portal containers.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

// ---------------------------------------------------------------------------
// Triad (q42+p64+d10, qualia-audio articulatory)
// ---------------------------------------------------------------------------

/// Triad container — q42+p64+d10 articulatory inspector.
pub fn build_triad_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el
        .style()
        .set_css_text("display: flex; flex-direction: column; flex: 1; gap: 8px; font-family: var(--font-mono); color: var(--text-primary);");

    // Triad Cores Grid
    let grid = document.create_element("div").unwrap();
    let g_el: HtmlElement = grid.clone().dyn_into().unwrap();
    g_el.style()
        .set_css_text("display: grid; grid-template-columns: repeat(3, 1fr); gap: 6px;");

    let cores = [
        (
            "Core 0: Reasoning",
            "42MB Sentinel",
            "12% Load",
            "4.2 MB / 42 MB",
            "var(--accent-cyan)",
        ),
        (
            "Core 1: QTensor",
            "DirectML / GGUF",
            "48% Load",
            "1.2 GB VRAM",
            "var(--accent-amber)",
        ),
        (
            "Core 2: Volumetric",
            "WebGPU 60 FPS",
            "24% Load",
            "16.6ms Render",
            "var(--accent-emerald)",
        ),
    ];

    for (name, role, load, mem, col) in cores {
        let card = document.create_element("div").unwrap();
        card.set_class_name("cr-card");
        let c_el: HtmlElement = card.clone().dyn_into().unwrap();
        c_el.style().set_css_text("padding: 8px; background: var(--surface-panel); border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); display: flex; flex-direction: column; gap: 3px; font-size: 9px;");

        let h = document.create_element("div").unwrap();
        h.set_attribute("style", &format!("font-weight: 700; color: {};", col))
            .unwrap();
        h.set_text_content(Some(name));
        card.append_child(&h).unwrap();

        let r = document.create_element("div").unwrap();
        r.set_attribute("style", "color: var(--text-secondary); font-size: 9px;")
            .unwrap();
        r.set_text_content(Some(role));
        card.append_child(&r).unwrap();

        let m = document.create_element("div").unwrap();
        m.set_attribute("style", "display: flex; justify-content: space-between; color: var(--text-muted); font-size: 8px; margin-top: 4px;").unwrap();
        m.set_inner_html(&format!("<span>{}</span><span>{}</span>", load, mem));
        card.append_child(&m).unwrap();

        grid.append_child(&card).unwrap();
    }
    wrapper.append_child(&grid).unwrap();

    // Thermal & Governor Status
    let gov_card = document.create_element("div").unwrap();
    gov_card.set_class_name("cr-card");
    let gc_el: HtmlElement = gov_card.clone().dyn_into().unwrap();
    gc_el.style().set_css_text("padding: 6px 10px; background: rgba(0,0,0,0.3); border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); font-size: 9px; display: flex; justify-content: space-between; align-items: center;");
    gov_card.set_inner_html(
        "<div><span style='color: var(--accent-emerald); font-weight: 700;'>Governor:</span> Cool (46\u{00B0}C) \u{00B7} Zero Throttling</div>\
         <div style='color: var(--text-muted);'>Power Budget: 25W (Balanced)</div>"
    );
    wrapper.append_child(&gov_card).unwrap();

    // Action Toolbar
    let actions = document.create_element("div").unwrap();
    actions.set_class_name("vibe-toolbar");
    for label in &[
        "Benchmark Triad",
        "Reset Arena",
        "Governor Mode",
        "Export Telemetry",
    ] {
        let btn = document.create_element("button").unwrap();
        btn.set_class_name("vibe-run-btn");
        btn.set_text_content(Some(label));
        actions.append_child(&btn).unwrap();
    }
    wrapper.append_child(&actions).unwrap();

    wrapper
}

// ---------------------------------------------------------------------------
// Portal (QApp dispatch, wormhole IRI)
// ---------------------------------------------------------------------------

/// Portal container — QApp dispatch, wormhole IRI.
pub fn build_portal_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el
        .style()
        .set_css_text("display: flex; flex-direction: column; flex: 1; gap: 8px; font-family: var(--font-mono); color: var(--text-primary);");

    // Target Manifold Resolver Bar
    let bar = document.create_element("div").unwrap();
    bar.set_class_name("vibe-toolbar");
    let input = document.create_element("input").unwrap();
    let input_el: web_sys::HtmlInputElement = input.clone().dyn_into().unwrap();
    input_el.set_value("did:qualia:manifold:spatial-neuro-catchment#v1");
    input.set_attribute("style", "flex: 1; background: var(--canvas-bg); border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); padding: 4px 8px; color: var(--text-primary); font-size: 11px; font-family: var(--font-mono); outline: none;").unwrap();
    bar.append_child(&input).unwrap();

    let teleport_btn = document.create_element("button").unwrap();
    teleport_btn.set_class_name("vibe-run-btn");
    teleport_btn.set_text_content(Some("\u{2728} Teleport"));
    bar.append_child(&teleport_btn).unwrap();
    wrapper.append_child(&bar).unwrap();

    // Portal Destination Matrix
    let port_list = document.create_element("div").unwrap();
    let pl_el: HtmlElement = port_list.clone().dyn_into().unwrap();
    pl_el
        .style()
        .set_css_text("display: flex; flex-direction: column; gap: 4px;");

    let portals = [
        (
            "Catchment Studio",
            "Local Workspace",
            "SHA-256 Verified",
            "var(--accent-emerald)",
        ),
        (
            "Neuro-Anatomy 10D",
            "P2P Swarm Relay",
            "Signed DID Token",
            "var(--accent-cyan)",
        ),
        (
            "Quantum Chemistry Lab",
            "Federated Cluster",
            "Integrity Checked",
            "var(--accent-violet)",
        ),
    ];

    for (name, domain, auth, col) in portals {
        let row = document.create_element("div").unwrap();
        row.set_class_name("vibe-output");
        let r_el: HtmlElement = row.clone().dyn_into().unwrap();
        r_el.style().set_css_text("padding: 6px 8px; display: flex; justify-content: space-between; align-items: center; font-size: 9px; background: var(--surface-panel); border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); cursor: pointer;");

        let left = document.create_element("div").unwrap();
        left.set_inner_html(&format!("<strong style='color: {};'>{}</strong> \u{00B7} <span style='color: var(--text-muted);'>{}</span>", col, name, domain));
        row.append_child(&left).unwrap();

        let right = document.create_element("div").unwrap();
        right
            .set_attribute("style", "color: var(--accent-emerald); font-size: 8px;")
            .unwrap();
        right.set_text_content(Some(auth));
        row.append_child(&right).unwrap();

        port_list.append_child(&row).unwrap();
    }
    wrapper.append_child(&port_list).unwrap();

    // Actions
    let actions = document.create_element("div").unwrap();
    actions.set_class_name("vibe-toolbar");
    for label in &["Verify Wormhole", "Sandbox Config", "Export Link"] {
        let btn = document.create_element("button").unwrap();
        btn.set_class_name("vibe-run-btn");
        btn.set_text_content(Some(label));
        actions.append_child(&btn).unwrap();
    }
    wrapper.append_child(&actions).unwrap();

    wrapper
}
