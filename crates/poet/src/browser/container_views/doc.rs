//! CML HyperDoc container: view switcher, editor panes, and Aura Tray.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

use crate::browser::cml_document::CmlDocument;

pub fn build_doc_view(document: &Document) -> Element {
    let cml_doc = CmlDocument::default();
    let report = cml_doc.validate_shacl();

    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el
        .style()
        .set_css_text("display: flex; flex-direction: column; flex: 1; gap: 6px;");

    // View switcher tabs & Header
    let switcher_row = document.create_element("div").unwrap();
    let sr_el: HtmlElement = switcher_row.clone().dyn_into().unwrap();
    sr_el.style().set_css_text(
        "display: flex; align-items: center; justify-content: space-between; gap: 8px;",
    );

    let switcher = document.create_element("div").unwrap();
    switcher.set_class_name("doc-view-switcher");
    let switcher_el: HtmlElement = switcher.clone().dyn_into().unwrap();
    switcher_el.style().set_css_text(
        "display: flex; gap: 2px; padding: 2px; background: var(--surface-panel); \
         border-radius: var(--radius-xs); font-size: 10px; font-family: var(--font-mono);",
    );

    let views = [
        ("Visual CML", "visual"),
        ("Markdown+CML", "markdown"),
        ("RDF-Star (N-Quins)", "rdf"),
    ];
    for (idx, (label, view_id)) in views.iter().enumerate() {
        let tab = document.create_element("button").unwrap();
        tab.set_class_name("doc-view-tab");
        tab.set_attribute("data-doc-view", view_id).unwrap();
        if idx == 0 {
            tab.class_list().add_1("active").unwrap();
        }
        let tab_el: HtmlElement = tab.clone().dyn_into().unwrap();
        tab_el.style().set_css_text(
            "padding: 4px 10px; border: none; border-radius: var(--radius-xs); \
             cursor: pointer; font-size: 10px; font-family: var(--font-mono); \
             transition: var(--trans-fast);",
        );
        if idx == 0 {
            tab_el
                .style()
                .set_property("background", "var(--surface-panel-elevated)")
                .unwrap();
            tab_el
                .style()
                .set_property("color", "var(--text-primary)")
                .unwrap();
        } else {
            tab_el
                .style()
                .set_property("background", "transparent")
                .unwrap();
            tab_el
                .style()
                .set_property("color", "var(--text-muted)")
                .unwrap();
        }
        tab.set_text_content(Some(label));
        switcher.append_child(&tab).unwrap();
    }
    switcher_row.append_child(&switcher).unwrap();

    // Document Meta Chip
    let meta_chip = document.create_element("div").unwrap();
    let mc_el: HtmlElement = meta_chip.clone().dyn_into().unwrap();
    mc_el.style().set_css_text(
        "font-size: 9px; font-family: var(--font-mono); color: var(--text-muted); \
         display: flex; align-items: center; gap: 6px; padding: 2px 8px; \
         background: var(--surface-panel); border-radius: var(--radius-xs);",
    );
    meta_chip.set_text_content(Some(&format!(
        "\u{1F4C4} {} \u{00B7} Level {}",
        cml_doc.title, cml_doc.sensitivity_class
    )));
    switcher_row.append_child(&meta_chip).unwrap();

    wrapper.append_child(&switcher_row).unwrap();

    super::doc_toolbar::append_doc_toolbar(document, &wrapper);

    // Visual editor (<q-doc> contenteditable)
    let editor = document.create_element("div").unwrap();
    editor.set_class_name("doc-editor doc-view-panel q-doc-container");
    editor
        .set_attribute("data-doc-view-panel", "visual")
        .unwrap();
    let editor_el: HtmlElement = editor.clone().dyn_into().unwrap();
    editor_el.style().set_css_text(
        "flex: 1; background: var(--canvas-bg); border: 1px solid var(--border-subtle); \
         border-radius: var(--radius-xs); padding: 12px; color: var(--text-primary); \
         font-size: 13px; line-height: 1.6; overflow-y: auto; min-height: 140px;",
    );
    editor.set_attribute("contenteditable", "true").unwrap();
    editor.set_inner_html(&cml_doc.to_cml_html());
    wrapper.append_child(&editor).unwrap();

    // Embedded Reactive Calculation Cell (<q-cell>)
    let embedded_cell = crate::browser::vibe_cell::build_q_cell_element(
        document,
        crate::browser::vibe_cell::VibeCell::default(),
    );
    embedded_cell.set_class_name("doc-view-panel q-cell-widget");
    embedded_cell
        .set_attribute("data-doc-view-panel", "visual")
        .unwrap();
    wrapper.append_child(&embedded_cell).unwrap();

    // Markdown view (hidden by default)
    let md_view = document.create_element("div").unwrap();
    md_view.set_class_name("doc-markdown-view doc-view-panel");
    md_view
        .set_attribute("data-doc-view-panel", "markdown")
        .unwrap();
    let md_el: HtmlElement = md_view.clone().dyn_into().unwrap();
    md_el
        .style()
        .set_css_text("flex: 1; display: none; flex-direction: column; gap: 4px;");

    let md_textarea = document.create_element("textarea").unwrap();
    let md_ta_el: web_sys::HtmlTextAreaElement = md_textarea.clone().dyn_into().unwrap();
    md_ta_el.set_value(&cml_doc.to_markdown());
    md_textarea
        .set_attribute(
            "style",
            "flex: 1; background: var(--canvas-bg); border: 1px solid var(--border-subtle); \
         border-radius: var(--radius-xs); padding: 12px; color: var(--text-primary); \
         font-family: var(--font-mono); font-size: 12px; line-height: 1.6; \
         resize: none; outline: none; min-height: 140px;",
        )
        .unwrap();
    md_view.append_child(&md_textarea).unwrap();
    wrapper.append_child(&md_view).unwrap();

    // RDF Triples view (hidden by default)
    let rdf_view = document.create_element("div").unwrap();
    rdf_view.set_class_name("doc-rdf-view doc-view-panel");
    rdf_view
        .set_attribute("data-doc-view-panel", "rdf")
        .unwrap();
    let rdf_el: HtmlElement = rdf_view.clone().dyn_into().unwrap();
    rdf_el.style().set_css_text(
        "flex: 1; display: none; flex-direction: column; gap: 4px; overflow-y: auto;",
    );

    // RDF header banner
    let rdf_info = document.create_element("div").unwrap();
    rdf_info.set_class_name("doc-rdf-info");
    let info_el: HtmlElement = rdf_info.clone().dyn_into().unwrap();
    info_el.style().set_css_text(
        "padding: 6px 10px; background: var(--surface-panel); border-radius: var(--radius-xs); \
         font-size: 10px; color: var(--text-secondary); font-family: var(--font-mono); \
         display: flex; justify-content: space-between; align-items: center;",
    );
    rdf_info.set_text_content(Some(&format!(
        "\u{1F4CB} Extracted RDF-Star Triples: {} terms \u{00B7} 48-byte Super-Quins: {}",
        cml_doc.to_rdf_star_triples().len(),
        report.quin_count
    )));
    rdf_view.append_child(&rdf_info).unwrap();

    let rdf_table = document.create_element("div").unwrap();
    rdf_table.set_class_name("rdf-triple-table");
    let table_el: HtmlElement = rdf_table.clone().dyn_into().unwrap();
    table_el.style().set_css_text(
        "flex: 1; overflow-y: auto; font-family: var(--font-mono); font-size: 10px; \
         border: 1px solid var(--border-subtle); border-radius: var(--radius-xs);",
    );

    // Header row
    let header_row = document.create_element("div").unwrap();
    let hr_el: HtmlElement = header_row.clone().dyn_into().unwrap();
    hr_el.style().set_css_text(
        "display: flex; border-bottom: 1px solid var(--border-subtle); \
         background: var(--surface-panel); font-weight: 700;",
    );
    for col in &["Subject", "Predicate", "Object", "Confidence", "Provenance"] {
        let cell = document.create_element("div").unwrap();
        let cell_el: HtmlElement = cell.clone().dyn_into().unwrap();
        cell_el.style().set_css_text(
            "flex: 1; padding: 4px 8px; color: var(--accent-cyan); \
             border-right: 1px solid var(--border-subtle);",
        );
        cell.set_text_content(Some(col));
        header_row.append_child(&cell).unwrap();
    }
    rdf_table.append_child(&header_row).unwrap();

    // Extracted triple rows
    for triple in cml_doc.to_rdf_star_triples() {
        let row = document.create_element("div").unwrap();
        let row_el: HtmlElement = row.clone().dyn_into().unwrap();
        row_el
            .style()
            .set_css_text("display: flex; border-bottom: 1px solid var(--border-subtle);");

        let conf_pct = format!("{:.0}%", triple.confidence * 100.0);
        let vals = [
            triple.subject.as_str(),
            triple.predicate.as_str(),
            triple.object.as_str(),
            conf_pct.as_str(),
            triple.provenance.as_str(),
        ];
        for (i, val) in vals.iter().enumerate() {
            let cell = document.create_element("div").unwrap();
            let cell_el: HtmlElement = cell.clone().dyn_into().unwrap();
            cell_el.style().set_css_text(
                "flex: 1; padding: 4px 8px; color: var(--text-secondary); \
                 border-right: 1px solid var(--border-subtle); overflow: hidden; \
                 text-overflow: ellipsis; white-space: nowrap;",
            );
            if i == 3 {
                cell_el
                    .style()
                    .set_property("color", "var(--accent-emerald)")
                    .unwrap();
            }
            cell.set_text_content(Some(val));
            row.append_child(&cell).unwrap();
        }
        rdf_table.append_child(&row).unwrap();
    }
    rdf_view.append_child(&rdf_table).unwrap();
    wrapper.append_child(&rdf_view).unwrap();

    // =========================================================================
    // Aura Tray (<q-aura-tray>) Footer
    // =========================================================================
    let aura_tray = document.create_element("q-aura-tray").unwrap();
    aura_tray.set_class_name("q-aura-tray");
    let at_el: HtmlElement = aura_tray.clone().dyn_into().unwrap();
    at_el.style().set_css_text(
        "display: flex; align-items: center; justify-content: space-between; gap: 8px; \
         padding: 4px 10px; background: var(--surface-panel); border: 1px solid var(--border-subtle); \
         border-radius: var(--radius-xs); font-family: var(--font-mono); font-size: 10px;",
    );

    // Left: SHACL Conformance Badge
    let shacl_badge = document.create_element("div").unwrap();
    let sb_el: HtmlElement = shacl_badge.clone().dyn_into().unwrap();
    let badge_color = if report.conforms {
        "var(--accent-emerald)"
    } else {
        "var(--accent-rose)"
    };
    sb_el.style().set_css_text(&format!(
        "display: flex; align-items: center; gap: 5px; color: {}; font-weight: 600;",
        badge_color
    ));
    shacl_badge.set_text_content(Some(&format!("\u{2705} {}", report.status_label)));
    aura_tray.append_child(&shacl_badge).unwrap();

    // Middle: Modality & Quins Metric
    let metric_div = document.create_element("div").unwrap();
    let md_el: HtmlElement = metric_div.clone().dyn_into().unwrap();
    md_el
        .style()
        .set_css_text("color: var(--text-muted); display: flex; gap: 8px;");
    metric_div.set_text_content(Some(&format!(
        "\u{1F9E0} Certainty: 96% \u{00B7} {} Quins (Sentinel Bounded)",
        report.quin_count
    )));
    aura_tray.append_child(&metric_div).unwrap();

    // Right: Action button
    let export_btn = document.create_element("button").unwrap();
    export_btn.set_class_name("vibe-run-btn");
    let eb_el: HtmlElement = export_btn.clone().dyn_into().unwrap();
    eb_el
        .style()
        .set_css_text("padding: 2px 8px; font-size: 9px;");
    export_btn.set_text_content(Some("\u{1F4E6} Export .hcf"));
    aura_tray.append_child(&export_btn).unwrap();

    wrapper.append_child(&aura_tray).unwrap();

    // Wire the view switcher
    super::doc_switcher::wire_doc_view_switcher(document);

    wrapper
}
