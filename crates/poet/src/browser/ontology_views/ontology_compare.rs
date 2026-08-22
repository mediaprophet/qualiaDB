//! Ontology Compare — side-by-side diff of two ontologies (P0).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const LEFT_ONTOLOGY: &str = "social.n3 (soc:)";
const RIGHT_ONTOLOGY: &str = "social-connections.n3 (sc:)";

const DIFF_CLASSES: &[(&str, &str, &str)] = &[
    ("soc:Person", "sc:Person", "equivalent"),
    ("soc:Connection", "sc:Connection", "equivalent"),
    ("soc:Peer", "sc:Peer", "equivalent"),
    ("soc:Group", "\u{2014}", "left only"),
    ("\u{2014}", "sc:Circle", "right only"),
    ("soc:Relationship", "sc:Relationship", "name match"),
    ("soc:Trust", "sc:Trust", "equivalent"),
    ("soc:Reputation", "\u{2014}", "left only"),
    ("\u{2014}", "sc:Endorsement", "right only"),
    ("soc:Profile", "sc:Profile", "equivalent"),
];

const DIFF_PROPS: &[(&str, &str, &str)] = &[
    ("soc:hasPeer", "sc:hasPeer", "equivalent"),
    ("soc:hasConnection", "sc:hasConnection", "equivalent"),
    ("soc:trustLevel", "sc:trustScore", "similar"),
    ("soc:hasReputation", "\u{2014}", "left only"),
    ("\u{2014}", "sc:hasEndorsement", "right only"),
    (
        "soc:requestsConnection",
        "sc:requestsConnection",
        "equivalent",
    ),
    ("soc:epistemicModality", "\u{2014}", "left only"),
    ("\u{2014}", "sc:inCircle", "right only"),
];

pub fn build_ontology_compare_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 4px; overflow: hidden;",
    );

    let toolbar = document.create_element("div").unwrap();
    let tb_el: HtmlElement = toolbar.clone().dyn_into().unwrap();
    tb_el.style().set_css_text(
        "display: flex; gap: 4px; padding: 4px 8px; border-bottom: 1px solid var(--border-subtle); \
         align-items: center;",
    );

    let left_sel = document.create_element("select").unwrap();
    left_sel.set_text_content(Some(LEFT_ONTOLOGY));
    let ls_el: HtmlElement = left_sel.clone().dyn_into().unwrap();
    ls_el.style().set_css_text(
        "padding: 2px 6px; border: 1px solid var(--border-medium); background: var(--surface-bg); \
         color: var(--text-primary); border-radius: 3px; font-size: 8px; \
         font-family: var(--font-mono);",
    );
    toolbar.append_child(&left_sel).unwrap();

    let vs = document.create_element("span").unwrap();
    vs.set_text_content(Some("vs"));
    let vs_el: HtmlElement = vs.clone().dyn_into().unwrap();
    vs_el
        .style()
        .set_css_text("font-size: 8px; color: var(--text-muted); font-family: var(--font-mono);");
    toolbar.append_child(&vs).unwrap();

    let right_sel = document.create_element("select").unwrap();
    right_sel.set_text_content(Some(RIGHT_ONTOLOGY));
    let rs_el: HtmlElement = right_sel.clone().dyn_into().unwrap();
    rs_el.style().set_css_text(
        "padding: 2px 6px; border: 1px solid var(--border-medium); background: var(--surface-bg); \
         color: var(--text-primary); border-radius: 3px; font-size: 8px; \
         font-family: var(--font-mono);",
    );
    toolbar.append_child(&right_sel).unwrap();

    let spacer = document.create_element("div").unwrap();
    let sp_el: HtmlElement = spacer.clone().dyn_into().unwrap();
    sp_el.style().set_css_text("flex: 1;");
    toolbar.append_child(&spacer).unwrap();

    for label in &["Merge", "Export Diff", "Suggest Mappings"] {
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

    // Summary bar
    let summary = document.create_element("div").unwrap();
    summary.set_text_content(Some(
        "10 classes compared | 5 equivalent | 1 similar | 2 left only | 2 right only  ||  8 properties | 3 equivalent | 1 similar | 2 left only | 2 right only",
    ));
    let s_el: HtmlElement = summary.clone().dyn_into().unwrap();
    s_el.style().set_css_text(
        "padding: 3px 8px; font-size: 8px; color: var(--text-muted); \
         font-family: var(--font-mono); border-bottom: 1px solid var(--border-subtle);",
    );
    wrapper.append_child(&summary).unwrap();

    let content = document.create_element("div").unwrap();
    let c_el: HtmlElement = content.clone().dyn_into().unwrap();
    c_el.style()
        .set_css_text("flex: 1; overflow-y: auto; padding: 8px;");

    // Classes diff
    let classes_header = document.create_element("div").unwrap();
    classes_header.set_text_content(Some("Classes"));
    let ch_el: HtmlElement = classes_header.clone().dyn_into().unwrap();
    ch_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); margin-bottom: 4px;",
    );
    content.append_child(&classes_header).unwrap();

    let classes_table = make_table(document, &[LEFT_ONTOLOGY, RIGHT_ONTOLOGY, "Status"]);
    let classes_tbody = document.create_element("tbody").unwrap();
    for (left, right, status) in DIFF_CLASSES {
        let tr = document.create_element("tr").unwrap();
        let vals: Vec<String> = vec![left.to_string(), right.to_string(), status.to_string()];
        for (i, val) in vals.iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 2 {
                let color = match &**status {
                    "equivalent" => "rgba(100, 200, 100, 0.8)",
                    "similar" | "name match" => "rgba(255, 165, 0, 0.8)",
                    "left only" => "rgba(0, 200, 255, 0.6)",
                    "right only" => "rgba(200, 150, 255, 0.6)",
                    _ => "var(--text-muted)",
                };
                td_el.style().set_css_text(&format!(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: {}; font-size: 8px; font-weight: 600; font-family: var(--font-mono);",
                    color,
                ));
            } else {
                let color = if val == "\u{2014}" {
                    "var(--text-muted)"
                } else {
                    "var(--text-primary)"
                };
                td_el.style().set_css_text(&format!(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: {}; font-size: 9px; font-family: var(--font-mono);",
                    color,
                ));
            }
            tr.append_child(&td).unwrap();
        }
        classes_tbody.append_child(&tr).unwrap();
    }
    classes_table.append_child(&classes_tbody).unwrap();
    content.append_child(&classes_table).unwrap();

    // Properties diff
    let props_header = document.create_element("div").unwrap();
    props_header.set_text_content(Some("Properties"));
    let ph_el: HtmlElement = props_header.clone().dyn_into().unwrap();
    ph_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); margin-top: 10px; margin-bottom: 4px;",
    );
    content.append_child(&props_header).unwrap();

    let props_table = make_table(document, &[LEFT_ONTOLOGY, RIGHT_ONTOLOGY, "Status"]);
    let props_tbody = document.create_element("tbody").unwrap();
    for (left, right, status) in DIFF_PROPS {
        let tr = document.create_element("tr").unwrap();
        let vals: Vec<String> = vec![left.to_string(), right.to_string(), status.to_string()];
        for (i, val) in vals.iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 2 {
                let color = match &**status {
                    "equivalent" => "rgba(100, 200, 100, 0.8)",
                    "similar" => "rgba(255, 165, 0, 0.8)",
                    "left only" => "rgba(0, 200, 255, 0.6)",
                    "right only" => "rgba(200, 150, 255, 0.6)",
                    _ => "var(--text-muted)",
                };
                td_el.style().set_css_text(&format!(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: {}; font-size: 8px; font-weight: 600; font-family: var(--font-mono);",
                    color,
                ));
            } else {
                let color = if val == "\u{2014}" {
                    "var(--text-muted)"
                } else {
                    "var(--text-primary)"
                };
                td_el.style().set_css_text(&format!(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: {}; font-size: 9px; font-family: var(--font-mono);",
                    color,
                ));
            }
            tr.append_child(&td).unwrap();
        }
        props_tbody.append_child(&tr).unwrap();
    }
    props_table.append_child(&props_tbody).unwrap();
    content.append_child(&props_table).unwrap();

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} ontology compare requires qualia_core_db graph diff engine.",
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
