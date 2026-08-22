//! SHACL Shapes Editor — node shapes, property shapes, constraints (P0).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const SHAPES: &[(&str, &str, &str, u32, &str)] = &[
    ("PersonShape", "sh:NodeShape", "coop:Person", 4, "Valid"),
    ("DocumentShape", "sh:NodeShape", "coop:Document", 3, "Valid"),
    (
        "OrganizationShape",
        "sh:NodeShape",
        "coop:Organization",
        2,
        "Valid",
    ),
    (
        "ObligationShape",
        "sh:NodeShape",
        "obl:Obligation",
        5,
        "Warning",
    ),
    ("ProjectShape", "sh:NodeShape", "coop:Project", 6, "Valid"),
    ("ArtefactShape", "sh:NodeShape", "coop:Artefact", 3, "Valid"),
];

const CONSTRAINTS: &[(&str, &str, &str, &str)] = &[
    ("PersonShape", "sh:property", "hasName", "name"),
    ("PersonShape", "sh:minCount", "1", "name"),
    ("PersonShape", "sh:datatype", "xsd:string", "name"),
    ("PersonShape", "sh:maxCount", "1", "name"),
    ("PersonShape", "sh:property", "hasMember", "hasMember"),
    ("DocumentShape", "sh:property", "authoredBy", "authoredBy"),
    ("DocumentShape", "sh:minCount", "1", "authoredBy"),
    ("DocumentShape", "sh:class", "coop:Person", "authoredBy"),
    ("ObligationShape", "sh:minCount", "1", "obligatedTo"),
    ("ObligationShape", "sh:maxCount", "1", "obligatedTo"),
    ("ObligationShape", "sh:nodeKind", "sh:IRI", "obligatedTo"),
    (
        "ObligationShape",
        "sh:class",
        "coop:Organization",
        "obligatedTo",
    ),
    ("ObligationShape", "sh:minInclusive", "0", "severity"),
];

pub fn build_shacl_shapes_view(document: &Document) -> Element {
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
    for label in &["+ Shape", "+ Property Shape", "Validate All", "Export"] {
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

    // Validation summary
    let summary = document.create_element("div").unwrap();
    summary.set_text_content(Some(
        "6 shapes | 13 constraints | 5 valid | 1 warning | 0 violations",
    ));
    let s_el: HtmlElement = summary.clone().dyn_into().unwrap();
    s_el.style().set_css_text(
        "padding: 4px 8px; font-size: 8px; color: var(--text-muted); \
         font-family: var(--font-mono); border-bottom: 1px solid var(--border-subtle);",
    );
    wrapper.append_child(&summary).unwrap();

    let content = document.create_element("div").unwrap();
    let c_el: HtmlElement = content.clone().dyn_into().unwrap();
    c_el.style()
        .set_css_text("flex: 1; overflow-y: auto; padding: 8px;");

    // Shapes table
    let shapes_header = document.create_element("div").unwrap();
    shapes_header.set_text_content(Some("Node Shapes (6)"));
    let sh_el: HtmlElement = shapes_header.clone().dyn_into().unwrap();
    sh_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); margin-bottom: 4px;",
    );
    content.append_child(&shapes_header).unwrap();

    let shapes_table = make_table(
        document,
        &["Shape", "Type", "Target Class", "Properties", "Status"],
    );
    let shapes_tbody = document.create_element("tbody").unwrap();
    for (name, stype, target, count, status) in SHAPES {
        let tr = document.create_element("tr").unwrap();
        let vals: Vec<String> = vec![
            name.to_string(),
            stype.to_string(),
            target.to_string(),
            count.to_string(),
            status.to_string(),
        ];
        for (i, val) in vals.iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 4 {
                let color = match &**status {
                    "Valid" => "rgba(100, 200, 100, 0.8)",
                    "Warning" => "rgba(255, 165, 0, 0.8)",
                    "Violation" => "rgba(255, 0, 0, 0.8)",
                    _ => "var(--text-muted)",
                };
                td_el.style().set_css_text(&format!(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: {}; font-size: 8px; font-weight: 700; font-family: var(--font-mono);",
                    color,
                ));
            } else if i == 0 {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--accent-cyan); font-size: 9px; font-weight: 600; \
                     font-family: var(--font-mono);",
                );
            } else {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-primary); font-size: 8px; font-family: var(--font-mono);",
                );
            }
            tr.append_child(&td).unwrap();
        }
        shapes_tbody.append_child(&tr).unwrap();
    }
    shapes_table.append_child(&shapes_tbody).unwrap();
    content.append_child(&shapes_table).unwrap();

    // Constraints table
    let const_header = document.create_element("div").unwrap();
    const_header.set_text_content(Some("Property Constraints (13)"));
    let ch_el: HtmlElement = const_header.clone().dyn_into().unwrap();
    ch_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); margin-top: 10px; margin-bottom: 4px;",
    );
    content.append_child(&const_header).unwrap();

    let const_table = make_table(document, &["Shape", "Constraint", "Value", "Property"]);
    let const_tbody = document.create_element("tbody").unwrap();
    for (shape, constraint, value, prop) in CONSTRAINTS {
        let tr = document.create_element("tr").unwrap();
        let vals: Vec<String> = vec![
            shape.to_string(),
            constraint.to_string(),
            value.to_string(),
            prop.to_string(),
        ];
        for (i, val) in vals.iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 1 {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--accent-cyan); font-size: 8px; font-weight: 600; \
                     font-family: var(--font-mono);",
                );
            } else if i == 2 {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: rgba(255, 165, 0, 0.8); font-size: 8px; font-family: var(--font-mono);",
                );
            } else {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-primary); font-size: 8px; font-family: var(--font-mono);",
                );
            }
            tr.append_child(&td).unwrap();
        }
        const_tbody.append_child(&tr).unwrap();
    }
    const_table.append_child(&const_tbody).unwrap();
    content.append_child(&const_table).unwrap();

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} SHACL shapes require qualia_core_db SHACL validator.",
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
