//! Annotation Panel — annotations on selected dataset (§4.1, P1).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const ANNOTATIONS: &[(&str, &str, &str, &str, &str, &str)] = &[
    (
        "ANN-001",
        "DS-001",
        "Outlier detected in column C",
        "did:qualia:timothy_charles_holborn",
        "2026-08-15",
        "Public",
    ),
    (
        "ANN-002",
        "DS-002",
        "Citation graph shows clustering around 3 hubs",
        "did:qualia:timothy_charles_holborn",
        "2026-08-16",
        "Public",
    ),
    (
        "ANN-003",
        "DS-003",
        "Tensor slice at z=128 shows anomalous density",
        "did:qualia:timothy_charles_holborn",
        "2026-08-17",
        "Restricted",
    ),
    (
        "ANN-004",
        "DS-005",
        "DICOM slice 256: region of interest marked",
        "did:qualia:timothy_charles_holborn",
        "2026-08-18",
        "Restricted",
    ),
    (
        "ANN-005",
        "DS-006",
        "Contribution spike correlates with release v2.0",
        "did:qualia:timothy_charles_holborn",
        "2026-08-18",
        "Public",
    ),
    (
        "ANN-006",
        "DS-001",
        "Data quality flag: 3 rows have missing values in column B",
        "did:qualia:timothy_charles_holborn",
        "2026-08-10",
        "Public",
    ),
];

pub fn build_annotation_panel_view(document: &Document) -> Element {
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
        "+ Annotation",
        "Filter by Dataset",
        "Filter by Sensitivity",
        "Export",
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

    for (id, dataset, body, author, date, sens) in ANNOTATIONS {
        let card = document.create_element("div").unwrap();
        let cd_el: HtmlElement = card.clone().dyn_into().unwrap();
        cd_el.style().set_css_text(
            "border: 1px solid var(--border-subtle); border-radius: 6px; \
             margin-bottom: 6px; background: var(--surface-panel); padding: 6px 8px;",
        );

        // Header row
        let hdr = document.create_element("div").unwrap();
        let h_el: HtmlElement = hdr.clone().dyn_into().unwrap();
        h_el.style()
            .set_css_text("display: flex; align-items: center; gap: 6px; margin-bottom: 4px;");

        let id_span = document.create_element("span").unwrap();
        id_span.set_text_content(Some(id));
        let is_el: HtmlElement = id_span.clone().dyn_into().unwrap();
        is_el.style().set_css_text(
            "font-size: 8px; color: var(--accent-cyan); font-family: var(--font-mono); font-weight: 600;",
        );
        hdr.append_child(&id_span).unwrap();

        let ds_span = document.create_element("span").unwrap();
        ds_span.set_text_content(Some(dataset));
        let ds_el: HtmlElement = ds_span.clone().dyn_into().unwrap();
        ds_el.style().set_css_text(
            "font-size: 8px; color: var(--text-muted); font-family: var(--font-mono);",
        );
        hdr.append_child(&ds_span).unwrap();

        let sens_badge = document.create_element("span").unwrap();
        sens_badge.set_text_content(Some(sens));
        let sb_el: HtmlElement = sens_badge.clone().dyn_into().unwrap();
        let sens_color = match *sens {
            "Public" => "rgba(100, 200, 100, 0.8)",
            "Restricted" => "rgba(255, 165, 0, 0.8)",
            "Selfhood" => "rgba(200, 150, 255, 0.8)",
            _ => "var(--text-muted)",
        };
        sb_el.style().set_css_text(&format!(
            "margin-left: auto; font-size: 7px; color: {}; font-family: var(--font-mono); \
             font-weight: 600; text-transform: uppercase;",
            sens_color,
        ));
        hdr.append_child(&sens_badge).unwrap();
        card.append_child(&hdr).unwrap();

        // Body
        let body_div = document.create_element("div").unwrap();
        body_div.set_text_content(Some(body));
        let bd_el: HtmlElement = body_div.clone().dyn_into().unwrap();
        bd_el.style().set_css_text(
            "font-size: 9px; color: var(--text-primary); font-family: var(--font-mono); \
             margin-bottom: 4px; line-height: 1.4;",
        );
        card.append_child(&body_div).unwrap();

        // Provenance
        let prov = document.create_element("div").unwrap();
        prov.set_text_content(Some(&format!("Author: {}  |  Date: {}", author, date)));
        let p_el: HtmlElement = prov.clone().dyn_into().unwrap();
        p_el.style().set_css_text(
            "font-size: 7px; color: var(--text-muted); font-family: var(--font-mono);",
        );
        card.append_child(&prov).unwrap();

        content.append_child(&card).unwrap();
    }

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} annotation panel requires DAT-26 engine.",
    ));
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 2px 8px; \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&footer).unwrap();

    wrapper
}
