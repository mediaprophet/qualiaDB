//! Presentation Editor — compose datasets + view specs into a presentation (§4.1, P0).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const VIEW_KINDS: &[(&str, &str)] = &[
    ("Table", "HTML/SVG table with sort/filter/pagination"),
    ("Graph 2D", "SVG/canvas force-directed layout"),
    ("Graph 3D", "PortalGpu 3D node-link + picking"),
    ("3D Mesh", "PortalGpu 3D container"),
    ("3D Point Cloud", "PortalGpu ambient particles"),
    ("3D Volume", "Volumetric renderer (CT/MRI)"),
    ("3D Isosurface", "Marching cubes \u{2192} mesh"),
    ("Chart", "Bar / line / scatter / area"),
    ("Map GIS", "2D/3D GIS + GeoSPARQL"),
    ("Tensor Heatmap", "Colour-mapped grid"),
    ("Tree / Hierarchy", "Tree layout (SVG/canvas)"),
    ("Timeline", "Time-scrub + provenance bounds"),
    ("Spectrogram", "STFT colour-mapped time-frequency"),
    ("Image", "Computer vision decode + display"),
    ("Video", "Frame sequence"),
    ("Audio Waveform", "PCM waveform view"),
];

const VIEWS: &[(&str, &str, &str, &str, &str)] = &[
    (
        "VS-001",
        "Table",
        "Experimental Results CSV",
        "Public",
        "DAT-9",
    ),
    (
        "VS-002",
        "Graph 2D",
        "Citation Graph (RDF-Star)",
        "Public",
        "DAT-10",
    ),
    (
        "VS-003",
        "Tensor Heatmap",
        "Simulation Tensor Slice",
        "Restricted",
        "DAT-18",
    ),
    (
        "VS-004",
        "Timeline",
        "Contribution Graph",
        "Public",
        "DAT-20",
    ),
];

pub fn build_presentation_editor_view(document: &Document) -> Element {
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
    for label in &["+ Add View", "Save Presentation", "Publish", "Preview"] {
        let btn = document.create_element("button").unwrap();
        btn.set_text_content(Some(label));
        let b_el: HtmlElement = btn.clone().dyn_into().unwrap();
        b_el.style().set_css_text(
            "padding: 2px 8px; border: 1px solid var(--border-medium); \
             background: transparent; color: var(--text-secondary); border-radius: 3px; \
             cursor: pointer; font-size: 9px; font-family: var(--font-mono);",
        );
        toolbar.append_child(&btn).unwrap();
    }
    wrapper.append_child(&toolbar).unwrap();

    let content = document.create_element("div").unwrap();
    let c_el: HtmlElement = content.clone().dyn_into().unwrap();
    c_el.style()
        .set_css_text("flex: 1; overflow-y: auto; padding: 8px;");

    // Presentation metadata
    let meta = document.create_element("div").unwrap();
    let m_el: HtmlElement = meta.clone().dyn_into().unwrap();
    m_el.style().set_css_text(
        "padding: 6px 8px; background: var(--surface-panel); border-radius: 4px; \
         margin-bottom: 8px; font-size: 9px; color: var(--text-primary); \
         font-family: var(--font-mono);",
    );
    meta.set_text_content(Some(
        "Presentation: Research Project Overview\n\
         Author: did:qualia:timothy_charles_holborn  |  Created: 2026-08-18  |  4 views  |  Public",
    ));
    content.append_child(&meta).unwrap();

    // Current views
    let views_header = document.create_element("div").unwrap();
    views_header.set_text_content(Some("Current Views"));
    let vh_el: HtmlElement = views_header.clone().dyn_into().unwrap();
    vh_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); margin-bottom: 4px;",
    );
    content.append_child(&views_header).unwrap();

    let table = make_table(
        document,
        &["ID", "View Kind", "Dataset", "Sensitivity", "Engine"],
    );
    let tbody = document.create_element("tbody").unwrap();
    for (id, kind, dataset, sens, engine) in VIEWS {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [id, kind, dataset, sens, engine].iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 3 {
                let color = match **val {
                    "Public" => "rgba(100, 200, 100, 0.8)",
                    "Restricted" => "rgba(255, 165, 0, 0.8)",
                    _ => "var(--text-primary)",
                };
                td_el.style().set_css_text(&format!(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: {}; font-size: 9px; font-weight: 600;",
                    color,
                ));
            } else if i == 1 {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--accent-cyan); font-size: 9px; font-weight: 600; \
                     font-family: var(--font-mono);",
                );
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
    content.append_child(&table).unwrap();

    // Available view kinds
    let kinds_header = document.create_element("div").unwrap();
    kinds_header.set_text_content(Some("Available View Kinds"));
    let kh_el: HtmlElement = kinds_header.clone().dyn_into().unwrap();
    kh_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); margin-top: 10px; margin-bottom: 4px;",
    );
    content.append_child(&kinds_header).unwrap();

    let kinds_grid = document.create_element("div").unwrap();
    let kg_el: HtmlElement = kinds_grid.clone().dyn_into().unwrap();
    kg_el
        .style()
        .set_css_text("display: grid; grid-template-columns: repeat(4, 1fr); gap: 4px;");

    for (kind, desc) in VIEW_KINDS {
        let card = document.create_element("div").unwrap();
        let c_el: HtmlElement = card.clone().dyn_into().unwrap();
        c_el.style().set_css_text(
            "padding: 4px; background: var(--surface-panel); border-radius: 3px; \
             border: 1px solid var(--border-subtle); cursor: pointer;",
        );

        let name = document.create_element("div").unwrap();
        name.set_text_content(Some(kind));
        let n_el: HtmlElement = name.clone().dyn_into().unwrap();
        n_el.style().set_css_text(
            "font-size: 9px; font-weight: 600; color: var(--accent-cyan); \
             font-family: var(--font-mono);",
        );
        card.append_child(&name).unwrap();

        let description = document.create_element("div").unwrap();
        description.set_text_content(Some(desc));
        let d_el: HtmlElement = description.clone().dyn_into().unwrap();
        d_el.style().set_css_text(
            "font-size: 7px; color: var(--text-muted); font-family: var(--font-mono); \
             margin-top: 1px;",
        );
        card.append_child(&description).unwrap();
        kg_el.append_child(&card).unwrap();
    }
    content.append_child(&kinds_grid).unwrap();

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} presentation editor requires DAT-5..DAT-6 engine.",
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
