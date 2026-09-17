//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Slide, 3D, and subcanvas containers.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

// ---------------------------------------------------------------------------
// Slide (office presentation)
// ---------------------------------------------------------------------------

/// Slide container — office presentation.
pub fn build_slide_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el
        .style()
        .set_css_text("display: flex; flex-direction: column; flex: 1; gap: 4px;");

    let bar = document.create_element("div").unwrap();
    bar.set_class_name("vibe-toolbar");
    for label in &["+ Slide", "Layout", "Transition", "Present"] {
        let btn = document.create_element("button").unwrap();
        btn.set_class_name("vibe-run-btn");
        btn.set_text_content(Some(label));
        bar.append_child(&btn).unwrap();
    }
    wrapper.append_child(&bar).unwrap();

    let slide_area = document.create_element("div").unwrap();
    let sa_el: HtmlElement = slide_area.clone().dyn_into().unwrap();
    sa_el.style().set_css_text(
        "flex: 1; background: var(--canvas-bg); border: 1px solid var(--border-subtle); \
         border-radius: var(--radius-xs); display: flex; align-items: center; \
         justify-content: center; color: var(--text-muted); font-size: 12px;",
    );
    slide_area.set_text_content(Some("Slide 1 \u{2014} click to add title"));
    wrapper.append_child(&slide_area).unwrap();

    wrapper
}

// ---------------------------------------------------------------------------
// 3D (GPU viewport, mesh, 10D)
// ---------------------------------------------------------------------------

/// 3D container — GPU viewport, mesh, 10D asset loading.
pub fn build_3d_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el
        .style()
        .set_css_text("display: flex; flex-direction: column; flex: 1; gap: 8px; font-family: var(--font-mono); color: var(--text-primary);");

    // Header Toolbar
    let toolbar = document.create_element("div").unwrap();
    toolbar.set_class_name("vibe-toolbar");
    for label in &[
        "Orbit Camera",
        "Wireframe",
        "WGSL Shading",
        "Subdivide",
        "Export Mesh",
    ] {
        let btn = document.create_element("button").unwrap();
        btn.set_class_name("vibe-run-btn");
        btn.set_text_content(Some(label));
        btn.set_attribute("disabled", "").unwrap();
        btn.set_attribute("aria-disabled", "true").unwrap();
        btn.set_attribute(
            "title",
            "Unavailable until this preview exposes a typed mutable camera/mesh session contract.",
        )
        .unwrap();
        toolbar.append_child(&btn).unwrap();
    }
    wrapper.append_child(&toolbar).unwrap();

    // Interactive 3D SVG Projection Viewport
    let viewport = document.create_element("div").unwrap();
    let vp_el: HtmlElement = viewport.clone().dyn_into().unwrap();
    vp_el.style().set_css_text("flex: 1; background: rgba(0,0,0,0.55); border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); position: relative; display: flex; align-items: center; justify-content: center; min-height: 120px; overflow: hidden;");

    let svg = document
        .create_element_ns(Some("http://www.w3.org/2000/svg"), "svg")
        .unwrap();
    svg.set_attribute("width", "100%").unwrap();
    svg.set_attribute("height", "100%").unwrap();
    svg.set_attribute("viewBox", "0 0 320 120").unwrap();

    svg.set_inner_html(
        "<polygon points='160,20 220,50 160,100 100,50' fill='rgba(0, 210, 255, 0.15)' stroke='#00d2ff' stroke-width='1.5'/>\
         <polygon points='160,20 220,50 200,90 160,100' fill='rgba(168, 85, 247, 0.2)' stroke='#a855f7' stroke-width='1.5'/>\
         <line x1='160' y1='20' x2='160' y2='100' stroke='#38bdf8' stroke-width='1' stroke-dasharray='2,2'/>\
         <circle cx='160' cy='20' r='3.5' fill='#00f2a9'/>\
         <circle cx='220' cy='50' r='3.5' fill='#00f2a9'/>\
         <circle cx='160' cy='100' r='3.5' fill='#00f2a9'/>\
         <circle cx='100' cy='50' r='3.5' fill='#00f2a9'/>\
         <text x='15' y='25' fill='#94a3b8' font-size='9' font-family='monospace'>Vertices: 1,024 \u{00B7} Faces: 2,048</text>\
         <text x='15' y='110' fill='#00f2a9' font-size='9' font-family='monospace'>Pitch: 22\u{00B0} \u{00B7} Yaw: 45\u{00B7} FOV: 60\u{00B0}</text>"
    );
    viewport.append_child(&svg).unwrap();
    wrapper.append_child(&viewport).unwrap();
    wrapper
        .append_child(&crate::browser::render_preview::build(
            document, "media", 800, 480,
        ))
        .unwrap();

    wrapper
}

// ---------------------------------------------------------------------------
// Subcanvas (switch manifold, enter-zoom)
// ---------------------------------------------------------------------------

/// Subcanvas container — switch manifold, enter-zoom.
pub fn build_subcanvas_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el
        .style()
        .set_css_text("display: flex; flex-direction: column; flex: 1; gap: 8px; font-family: var(--font-mono); color: var(--text-primary);");

    // Breadcrumb Navigation Header
    let breadcrumb = document.create_element("div").unwrap();
    let bc_el: HtmlElement = breadcrumb.clone().dyn_into().unwrap();
    bc_el.style().set_css_text("padding: 4px 8px; background: var(--surface-panel); border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); font-size: 10px; color: var(--accent-cyan); display: flex; align-items: center; gap: 6px;");
    breadcrumb.set_inner_html(
        "<span>Workspace</span> <span>\u{203A}</span> <span>Manifold Alpha</span> <span>\u{203A}</span> <strong style='color: var(--text-primary);'>Nested Subcanvas 1</strong>"
    );
    wrapper.append_child(&breadcrumb).unwrap();

    // Nested Subcanvas Viewport Preview
    let preview = document.create_element("div").unwrap();
    let p_el: HtmlElement = preview.clone().dyn_into().unwrap();
    p_el.style().set_css_text("flex: 1; background: rgba(0,0,0,0.4); border: 1px dashed var(--accent-cyan); border-radius: var(--radius-xs); display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 8px; min-height: 100px;");

    let icon = document.create_element("div").unwrap();
    icon.set_text_content(Some("\u{1F50D}"));
    icon.set_attribute("style", "font-size: 24px;").unwrap();
    preview.append_child(&icon).unwrap();

    let label = document.create_element("div").unwrap();
    label
        .set_attribute(
            "style",
            "font-size: 11px; color: var(--text-secondary); text-align: center;",
        )
        .unwrap();
    label.set_text_content(Some("Subcanvas Isolation Sandbox \u{00B7} LOD Depth: 2"));
    preview.append_child(&label).unwrap();
    wrapper.append_child(&preview).unwrap();
    wrapper
        .append_child(&crate::browser::render_preview::build(
            document,
            "submanifold",
            800,
            480,
        ))
        .unwrap();

    // Action Toolbar
    let actions = document.create_element("div").unwrap();
    actions.set_class_name("vibe-toolbar");
    for label in &[
        "Enter Subcanvas (Zoom)",
        "Pop to Parent",
        "Clone Subtree",
        "Merge to Root",
    ] {
        let btn = document.create_element("button").unwrap();
        btn.set_class_name("vibe-run-btn");
        btn.set_text_content(Some(label));
        actions.append_child(&btn).unwrap();
    }
    wrapper.append_child(&actions).unwrap();

    wrapper
}
