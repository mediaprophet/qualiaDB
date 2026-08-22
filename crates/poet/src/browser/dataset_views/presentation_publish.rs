//! Presentation Publish — publish presentation as shareable artefact (§4.3, P2).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const PUBLISH_TARGETS: &[(&str, &str, bool)] = &[
    (
        "QualiaDB",
        "Store as named presentation in personal graph",
        true,
    ),
    (
        "HCF Bundle",
        "Export as Hypermedia Content Format (.hcf)",
        true,
    ),
    ("CBOR-LD", "Compact binary serialisation for WASM", false),
    ("JSON-LD", "Human-readable linked data serialisation", true),
    (
        "Static HTML",
        "Self-contained HTML with embedded viewer",
        false,
    ),
    ("PDF Report", "Print-quality PDF with all views", false),
];

const PUBLISH_INFO: &[(&str, &str)] = &[
    ("Presentation ID", "PRES-003"),
    ("Title", "Experimental Results \u{2014} Q3 2026"),
    ("Views", "4 (Table, Graph 2D, Tensor Heatmap, Timeline)"),
    ("Datasets", "DS-001, DS-002, DS-003"),
    ("Sensitivity", "Public"),
    ("Author", "did:qualia:timothy_charles_holborn"),
    ("Created", "2026-08-18"),
    ("Version", "1.2.0"),
    ("SHACL Validated", "Passed (0 violations)"),
    ("Provenance Chain", "3 transformations tracked"),
];

pub fn build_presentation_publish_view(document: &Document) -> Element {
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
    for label in &["Publish", "Validate SHACL", "Preview", "Copy Link"] {
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

    // Presentation info
    let info_header = document.create_element("div").unwrap();
    info_header.set_text_content(Some("Presentation Metadata"));
    let ih_el: HtmlElement = info_header.clone().dyn_into().unwrap();
    ih_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); margin-bottom: 4px;",
    );
    content.append_child(&info_header).unwrap();

    let info_table = make_table(document, &["Field", "Value"]);
    let info_tbody = document.create_element("tbody").unwrap();
    for (field, value) in PUBLISH_INFO {
        let tr = document.create_element("tr").unwrap();
        let vals: Vec<String> = vec![field.to_string(), value.to_string()];
        for (i, val) in vals.iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 1 {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--accent-cyan); font-size: 9px; font-family: var(--font-mono);",
                );
            } else {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-muted); font-size: 8px; font-family: var(--font-mono);",
                );
            }
            tr.append_child(&td).unwrap();
        }
        info_tbody.append_child(&tr).unwrap();
    }
    info_table.append_child(&info_tbody).unwrap();
    content.append_child(&info_table).unwrap();

    // Publish targets
    let targets_header = document.create_element("div").unwrap();
    targets_header.set_text_content(Some("Publish Targets"));
    let th_el: HtmlElement = targets_header.clone().dyn_into().unwrap();
    th_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); margin-top: 10px; margin-bottom: 4px;",
    );
    content.append_child(&targets_header).unwrap();

    let targets_grid = document.create_element("div").unwrap();
    let tg_el: HtmlElement = targets_grid.clone().dyn_into().unwrap();
    tg_el
        .style()
        .set_css_text("display: grid; grid-template-columns: repeat(2, 1fr); gap: 6px;");

    for (name, desc, selected) in PUBLISH_TARGETS {
        let card = document.create_element("div").unwrap();
        let cd_el: HtmlElement = card.clone().dyn_into().unwrap();
        let border = if *selected {
            "var(--accent-cyan)"
        } else {
            "var(--border-subtle)"
        };
        cd_el.style().set_css_text(&format!(
            "padding: 6px 8px; background: var(--surface-panel); border-radius: 6px; \
             border: 2px solid {}; cursor: pointer;",
            border,
        ));

        let row = document.create_element("div").unwrap();
        let r_el: HtmlElement = row.clone().dyn_into().unwrap();
        r_el.style()
            .set_css_text("display: flex; align-items: center; gap: 4px;");

        let checkbox = document.create_element("span").unwrap();
        checkbox.set_text_content(Some(if *selected { "\u{2705}" } else { "\u{2610}" }));
        let cb_el: HtmlElement = checkbox.clone().dyn_into().unwrap();
        cb_el.style().set_css_text("font-size: 10px;");
        row.append_child(&checkbox).unwrap();

        let name_div = document.create_element("div").unwrap();
        name_div.set_text_content(Some(name));
        let n_el: HtmlElement = name_div.clone().dyn_into().unwrap();
        let name_color = if *selected {
            "var(--accent-cyan)"
        } else {
            "var(--text-primary)"
        };
        n_el.style().set_css_text(&format!(
            "font-size: 10px; font-weight: 600; color: {}; font-family: var(--font-mono);",
            name_color,
        ));
        row.append_child(&name_div).unwrap();
        card.append_child(&row).unwrap();

        let desc_div = document.create_element("div").unwrap();
        desc_div.set_text_content(Some(desc));
        let d_el: HtmlElement = desc_div.clone().dyn_into().unwrap();
        d_el.style().set_css_text(
            "font-size: 7px; color: var(--text-muted); font-family: var(--font-mono); \
             margin-top: 2px; margin-left: 18px;",
        );
        card.append_child(&desc_div).unwrap();

        tg_el.append_child(&card).unwrap();
    }
    content.append_child(&targets_grid).unwrap();

    // Access control
    let access_header = document.create_element("div").unwrap();
    access_header.set_text_content(Some("Access Control"));
    let ah_el: HtmlElement = access_header.clone().dyn_into().unwrap();
    ah_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); margin-top: 10px; margin-bottom: 4px;",
    );
    content.append_child(&access_header).unwrap();

    let access_opts = [
        "Visibility: Public (anyone with link)",
        "Consent: All datasets have publish consent",
        "Sensitivity check: Passed (no Selfhood data)",
        "License: CC BY-NC-ND 4.0",
        "Attribution: did:qualia:timothy_charles_holborn",
        "Expiry: Never",
    ];
    for opt in &access_opts {
        let row = document.create_element("div").unwrap();
        row.set_text_content(Some(opt));
        let r_el: HtmlElement = row.clone().dyn_into().unwrap();
        r_el.style().set_css_text(
            "padding: 2px 8px; font-size: 8px; color: var(--text-secondary); \
             font-family: var(--font-mono);",
        );
        content.append_child(&row).unwrap();
    }

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} presentation publish requires DAT-29 engine.",
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
