//! Scene View — 3D wgpu viewport placeholder with toolbar and view-mode selector (§2.1, P0).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const VIEW_MODES: &[&str] = &[
    "3D",
    "2D",
    "Wireframe",
    "Solid",
    "Point Cloud",
    "Volumetric",
];

const ARTEFACTS: &[(&str, &str, &str, &str)] = &[
    ("House Model", "GLB", "LOD 2", "Public"),
    ("Terrain DEM", "GIS", "LOD 1", "Public"),
    ("Tensor Field A", "10D", "LOD 0", "Restricted"),
];

pub fn build_scene_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 4px; overflow: hidden;",
    );

    // Header: artefact name, LOD badge, sensitivity badge, capability badge
    let header = document.create_element("div").unwrap();
    let h_el: HtmlElement = header.clone().dyn_into().unwrap();
    h_el.style().set_css_text(
        "display: flex; align-items: center; gap: 8px; padding: 4px 8px; \
         border-bottom: 1px solid var(--border-subtle);",
    );

    let title = document.create_element("span").unwrap();
    title.set_text_content(Some("Scene: Untitled Studio Scene"));
    let t_el: HtmlElement = title.clone().dyn_into().unwrap();
    t_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono);",
    );
    header.append_child(&title).unwrap();

    for (label, color) in &[
        ("LOD 2", "rgba(0, 200, 255, 0.6)"),
        ("Public", "rgba(100, 200, 100, 0.6)"),
        ("WASM", "rgba(200, 150, 255, 0.6)"),
    ] {
        let badge = document.create_element("span").unwrap();
        badge.set_text_content(Some(label));
        let b_el: HtmlElement = badge.clone().dyn_into().unwrap();
        b_el.style().set_css_text(&format!(
            "font-size: 8px; padding: 1px 4px; border-radius: 2px; \
             color: {}; border: 1px solid {}; font-family: var(--font-mono);",
            color, color,
        ));
        header.append_child(&badge).unwrap();
    }

    // View-mode selector
    let mode_sel = document.create_element("select").unwrap();
    let ms_el: HtmlElement = mode_sel.clone().dyn_into().unwrap();
    ms_el.style().set_css_text(
        "font-size: 9px; font-family: var(--font-mono); background: transparent; \
         color: var(--text-secondary); border: 1px solid var(--border-medium); \
         border-radius: 3px; padding: 1px 4px; margin-left: auto;",
    );
    for mode in VIEW_MODES {
        let opt = document.create_element("option").unwrap();
        opt.set_attribute("value", mode).unwrap();
        opt.set_text_content(Some(mode));
        mode_sel.append_child(&opt).unwrap();
    }
    header.append_child(&mode_sel).unwrap();
    wrapper.append_child(&header).unwrap();

    // Toolbar
    let toolbar = document.create_element("div").unwrap();
    let tb_el: HtmlElement = toolbar.clone().dyn_into().unwrap();
    tb_el.style().set_css_text(
        "display: flex; gap: 4px; padding: 4px 8px; border-bottom: 1px solid var(--border-subtle); \
         flex-wrap: wrap;",
    );
    for label in &[
        "Import Mesh",
        "Upload Tensor",
        "Add Light",
        "Add Artefact",
        "Snapshot .10d",
        "Export GLB",
        "Toggle Picking",
        "Toggle Bloom",
        "Toggle Shadows",
    ] {
        let btn = document.create_element("button").unwrap();
        btn.set_text_content(Some(label));
        let b_el: HtmlElement = btn.clone().dyn_into().unwrap();
        b_el.style().set_css_text(
            "padding: 2px 6px; border: 1px solid var(--border-medium); \
             background: transparent; color: var(--text-secondary); border-radius: 3px; \
             cursor: pointer; font-size: 8px; font-family: var(--font-mono);",
        );
        toolbar.append_child(&btn).unwrap();
    }
    wrapper.append_child(&toolbar).unwrap();

    // Viewport placeholder (wgpu surface would mount here)
    let viewport = document.create_element("div").unwrap();
    let vp_el: HtmlElement = viewport.clone().dyn_into().unwrap();
    vp_el.style().set_css_text(
        "flex: 1; display: flex; align-items: center; justify-content: center; \
         background: var(--surface-panel); border-radius: 6px; margin: 4px 8px; \
         position: relative; min-height: 200px;",
    );

    let placeholder = document.create_element("div").unwrap();
    placeholder.set_text_content(Some(
        "wgpu Viewport\n\
         \u{2014} PortalGpu tensor upload: not wired\n\
         \u{2014} Mesh render: not wired\n\
         \u{2014} Orbit/pan/zoom camera: not wired\n\
         \u{2014} Click-to-pick: not wired\n\n\
         Honesty: present (UI structure exists, backend not wired)",
    ));
    let p_el: HtmlElement = placeholder.clone().dyn_into().unwrap();
    p_el.style().set_css_text(
        "text-align: center; font-size: 10px; color: var(--text-muted); \
         font-family: var(--font-mono); line-height: 1.6; white-space: pre-wrap;",
    );
    viewport.append_child(&placeholder).unwrap();

    // Thermal state indicator
    let thermal = document.create_element("div").unwrap();
    thermal.set_text_content(Some("\u{1F321} Cool"));
    let th_el: HtmlElement = thermal.clone().dyn_into().unwrap();
    th_el.style().set_css_text(
        "position: absolute; top: 4px; right: 6px; font-size: 8px; \
         color: rgba(100, 200, 100, 0.6); font-family: var(--font-mono);",
    );
    viewport.append_child(&thermal).unwrap();

    wrapper.append_child(&viewport).unwrap();

    // Artefact list
    let list_header = document.create_element("div").unwrap();
    list_header.set_text_content(Some("Scene Artefacts"));
    let lh_el: HtmlElement = list_header.clone().dyn_into().unwrap();
    lh_el.style().set_css_text(
        "font-size: 9px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); padding: 4px 8px 2px;",
    );
    wrapper.append_child(&list_header).unwrap();

    let list = document.create_element("div").unwrap();
    let l_el: HtmlElement = list.clone().dyn_into().unwrap();
    l_el.style()
        .set_css_text("padding: 0 8px 4px; max-height: 120px; overflow-y: auto;");

    let table = make_table(document, &["Artefact", "Format", "LOD", "Sensitivity"]);
    let tbody = document.create_element("tbody").unwrap();
    for (name, fmt, lod, sens) in ARTEFACTS {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [name, fmt, lod, sens].iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 3 {
                let color = match **val {
                    "Public" => "rgba(100, 200, 100, 0.8)",
                    "Restricted" => "rgba(255, 165, 0, 0.8)",
                    "Classified" => "rgba(255, 0, 0, 0.8)",
                    _ => "var(--text-primary)",
                };
                td_el.style().set_css_text(&format!(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: {}; font-size: 9px; font-weight: 600;",
                    color,
                ));
            } else {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-primary); font-size: 9px; font-family: var(--font-mono);",
                );
            }
            tr.append_child(&td).unwrap();
        }
        tbody.append_child(&tr).unwrap();
    }
    table.append_child(&tbody).unwrap();
    list.append_child(&table).unwrap();
    wrapper.append_child(&list).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} 3D viewport requires PortalGpu wgpu surface + R3D-1..R3D-3 engine.",
    ));
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 2px 8px; \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&footer).unwrap();

    wrapper
}

fn make_table(document: &Document, headers: &[&str]) -> Element {
    let table = document.create_element("table").unwrap();
    let t_el: HtmlElement = table.clone().dyn_into().unwrap();
    t_el.style()
        .set_css_text("width: 100%; border-collapse: collapse; font-size: 9px;");
    let thead = document.create_element("thead").unwrap();
    let tr = document.create_element("tr").unwrap();
    for h in headers {
        let th = document.create_element("th").unwrap();
        th.set_text_content(Some(h));
        let th_el: HtmlElement = th.clone().dyn_into().unwrap();
        th_el.style().set_css_text(
            "text-align: left; padding: 3px 6px; border-bottom: 1px solid var(--border-medium); \
             color: var(--text-muted); font-family: var(--font-mono);",
        );
        tr.append_child(&th).unwrap();
    }
    thead.append_child(&tr).unwrap();
    table.append_child(&thead).unwrap();
    table
}
