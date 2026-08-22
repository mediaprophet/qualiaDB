//! Lineage Graph — derived dataset lineage (parent -> transform -> derived) (§4.1, P1).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const NODES: &[(&str, &str, &str)] = &[
    ("DS-001", "Experimental Results CSV", "source"),
    ("DS-002", "Citation Graph (RDF-Star)", "source"),
    ("DS-003", "Simulation Tensor Slice", "source"),
    ("DS-009", "Filtered Experimental Data", "derived"),
    ("DS-010", "Aggregated by Column A", "derived"),
    ("DS-011", "Joined CSV + RDF", "derived"),
    ("DS-012", "Tensor Mesh Extract", "derived"),
];

const EDGES: &[(&str, &str, &str)] = &[
    ("DS-001", "DS-009", "filter: col_C > 0.5"),
    ("DS-001", "DS-010", "aggregate: mean(col_A)"),
    ("DS-001", "DS-011", "join: on citation_id"),
    ("DS-002", "DS-011", "join: on citation_id"),
    ("DS-003", "DS-012", "mesh: isosurface(0.5)"),
];

pub fn build_lineage_graph_view(document: &Document) -> Element {
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
        "Layout: Hierarchical",
        "Layout: Force",
        "Export DOT",
        "Export SVG",
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
        .set_css_text("flex: 1; overflow-y: auto; padding: 8px;");

    // Graph visualization placeholder
    let graph_area = document.create_element("div").unwrap();
    let g_el: HtmlElement = graph_area.clone().dyn_into().unwrap();
    g_el.style().set_css_text(
        "height: 140px; background: var(--surface-panel); border-radius: 6px; \
         margin-bottom: 8px; display: flex; align-items: center; justify-content: center; \
         border: 1px solid var(--border-subtle);",
    );
    let graph_ph = document.create_element("div").unwrap();
    graph_ph.set_text_content(Some(
        "Lineage Graph Visualization\n\
         DS-001 -> DS-009 (filter)\n\
         DS-001 -> DS-010 (aggregate)\n\
         DS-001 + DS-002 -> DS-011 (join)\n\
         DS-003 -> DS-012 (mesh)",
    ));
    let gp_el: HtmlElement = graph_ph.clone().dyn_into().unwrap();
    gp_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); font-family: var(--font-mono); \
         white-space: pre-wrap; text-align: center; line-height: 1.6;",
    );
    graph_area.append_child(&graph_ph).unwrap();
    content.append_child(&graph_area).unwrap();

    // Nodes table
    let nodes_header = document.create_element("div").unwrap();
    nodes_header.set_text_content(Some("Datasets (Nodes)"));
    let nh_el: HtmlElement = nodes_header.clone().dyn_into().unwrap();
    nh_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); margin-bottom: 4px;",
    );
    content.append_child(&nodes_header).unwrap();

    let nodes_table = make_table(document, &["ID", "Name", "Type"]);
    let nodes_tbody = document.create_element("tbody").unwrap();
    for (id, name, ntype) in NODES {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [id, name, ntype].iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 2 {
                let color = if *ntype == "source" {
                    "rgba(100, 200, 100, 0.8)"
                } else {
                    "rgba(0, 200, 255, 0.8)"
                };
                td_el.style().set_css_text(&format!(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: {}; font-size: 8px; font-weight: 600; text-transform: uppercase;",
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
        nodes_tbody.append_child(&tr).unwrap();
    }
    nodes_table.append_child(&nodes_tbody).unwrap();
    content.append_child(&nodes_table).unwrap();

    // Edges table
    let edges_header = document.create_element("div").unwrap();
    edges_header.set_text_content(Some("Transformations (Edges)"));
    let eh_el: HtmlElement = edges_header.clone().dyn_into().unwrap();
    eh_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); margin-top: 8px; margin-bottom: 4px;",
    );
    content.append_child(&edges_header).unwrap();

    let edges_table = make_table(document, &["From", "To", "Transform"]);
    let edges_tbody = document.create_element("tbody").unwrap();
    for (from, to, transform) in EDGES {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [from, to, transform].iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 2 {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--accent-cyan); font-size: 8px; font-family: var(--font-mono);",
                );
            } else {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-primary); font-size: 9px; font-family: var(--font-mono);",
                );
            }
            tr.append_child(&td).unwrap();
        }
        edges_tbody.append_child(&tr).unwrap();
    }
    edges_table.append_child(&edges_tbody).unwrap();
    content.append_child(&edges_table).unwrap();

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} lineage graph requires DAT-27 engine.",
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
