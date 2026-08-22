//! Scene Graph — hierarchical list of artefacts + lights + cameras (§2.2, P1).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const NODES: &[(&str, &str, &str, &str, bool)] = &[
    ("Scene Root", "root", "", "", true),
    ("  Cameras", "group", "Scene Root", "", true),
    ("    Main Camera", "camera", "Cameras", "(0,5,20)", true),
    ("    Top Camera", "camera", "Cameras", "(0,30,0)", false),
    ("  Lights", "group", "Scene Root", "", true),
    (
        "    Sun (directional)",
        "light",
        "Lights",
        "dir: (1,-1,0)",
        true,
    ),
    ("    Ambient Sky", "light", "Lights", "intensity: 0.3", true),
    ("    Point Light A", "light", "Lights", "(5,3,5)", false),
    ("  Artefacts", "group", "Scene Root", "", true),
    ("    House Model", "mesh", "Artefacts", "GLB, LOD 2", true),
    ("    Terrain DEM", "mesh", "Artefacts", "GIS, LOD 1", true),
    (
        "    Tensor Field A",
        "tensor",
        "Artefacts",
        "10D, LOD 0",
        true,
    ),
    (
        "    Particle System",
        "particles",
        "Artefacts",
        "1000 nodes",
        false,
    ),
];

pub fn build_scene_graph_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 4px; overflow: hidden;",
    );

    let toolbar = document.create_element("div").unwrap();
    let tb_el: HtmlElement = toolbar.clone().dyn_into().unwrap();
    tb_el.style().set_css_text(
        "display: flex; gap: 4px; padding: 4px 8px; border-bottom: 1px solid var(--border-subtle);",
    );
    for label in &[
        "+ Artefact",
        "+ Light",
        "+ Camera",
        "Expand All",
        "Collapse All",
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

    let content = document.create_element("div").unwrap();
    let c_el: HtmlElement = content.clone().dyn_into().unwrap();
    c_el.style()
        .set_css_text("flex: 1; overflow-y: auto; padding: 4px 8px;");

    let table = make_table(document, &["Node", "Type", "Parent", "Params", "Visible"]);
    let tbody = document.create_element("tbody").unwrap();

    for (name, ntype, parent, params, visible) in NODES {
        let tr = document.create_element("tr").unwrap();

        // Node name (with indentation)
        let td = document.create_element("td").unwrap();
        td.set_text_content(Some(name));
        let td_el: HtmlElement = td.clone().dyn_into().unwrap();
        td_el.style().set_css_text(
            "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
             color: var(--text-primary); font-size: 9px; font-family: var(--font-mono); \
             white-space: pre;",
        );
        tr.append_child(&td).unwrap();

        // Type with color
        let td = document.create_element("td").unwrap();
        td.set_text_content(Some(ntype));
        let td_el: HtmlElement = td.clone().dyn_into().unwrap();
        let type_color = match *ntype {
            "root" => "rgba(200, 150, 255, 0.8)",
            "group" => "var(--text-muted)",
            "camera" => "rgba(0, 200, 255, 0.8)",
            "light" => "rgba(255, 220, 100, 0.8)",
            "mesh" => "rgba(100, 200, 100, 0.8)",
            "tensor" => "rgba(255, 165, 0, 0.8)",
            "particles" => "rgba(255, 100, 200, 0.8)",
            _ => "var(--text-primary)",
        };
        td_el.style().set_css_text(&format!(
            "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
             color: {}; font-size: 8px; font-family: var(--font-mono); font-weight: 600;",
            type_color,
        ));
        tr.append_child(&td).unwrap();

        // Parent
        let td = document.create_element("td").unwrap();
        td.set_text_content(Some(parent));
        let td_el: HtmlElement = td.clone().dyn_into().unwrap();
        td_el.style().set_css_text(
            "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
             color: var(--text-muted); font-size: 8px; font-family: var(--font-mono);",
        );
        tr.append_child(&td).unwrap();

        // Params
        let td = document.create_element("td").unwrap();
        td.set_text_content(Some(params));
        let td_el: HtmlElement = td.clone().dyn_into().unwrap();
        td_el.style().set_css_text(
            "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
             color: var(--text-secondary); font-size: 8px; font-family: var(--font-mono);",
        );
        tr.append_child(&td).unwrap();

        // Visible
        let td = document.create_element("td").unwrap();
        td.set_text_content(Some(if *visible { "\u{1F441}" } else { "\u{2014}" }));
        let td_el: HtmlElement = td.clone().dyn_into().unwrap();
        let vis_color = if *visible {
            "rgba(100, 200, 100, 0.8)"
        } else {
            "var(--text-muted)"
        };
        td_el.style().set_css_text(&format!(
            "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
             color: {}; font-size: 10px; text-align: center;",
            vis_color,
        ));
        tr.append_child(&td).unwrap();

        tbody.append_child(&tr).unwrap();
    }
    table.append_child(&tbody).unwrap();
    content.append_child(&table).unwrap();
    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} scene graph requires R3D-6 scene graph engine.",
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
            "text-align: left; padding: 4px 6px; border-bottom: 1px solid var(--border-medium); \
             color: var(--text-muted); font-family: var(--font-mono);",
        );
        tr.append_child(&th).unwrap();
    }
    thead.append_child(&tr).unwrap();
    table.append_child(&thead).unwrap();
    table
}
