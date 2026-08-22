//! Additional container body views: doc, sheet, graph, ontology, pulse, rights, wallet.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

use super::cml_document::CmlDocument;

/// Rich text CML HyperDoc container — contenteditable with Context Markup Language
/// (<q-entity>, <q-relation>), Tri-View switcher (Visual / Markdown / RDF-Star),
/// and Aura Tray with live SHACL validation & Super-Quin metrics.
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
    sr_el.style().set_css_text("display: flex; align-items: center; justify-content: space-between; gap: 8px;");

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
    meta_chip.set_text_content(Some(&format!("\u{1F4C4} {} \u{00B7} Level {}", cml_doc.title, cml_doc.sensitivity_class)));
    switcher_row.append_child(&meta_chip).unwrap();

    wrapper.append_child(&switcher_row).unwrap();

    // Toolbar (Visual mode)
    let toolbar = document.create_element("div").unwrap();
    toolbar.set_class_name("doc-toolbar doc-view-panel");
    toolbar
        .set_attribute("data-doc-view-panel", "visual")
        .unwrap();
    for label in &[
        "B", "I", "U", "\u{1F517} Link", "\u{1F4CD} Tag Entity", 
        "\u{2696}\u{FE0F} Deontic Rule", "\u{1F50D} CML Gazetteer", "\u{26A1} Reactive Cell"
    ] {
        let btn = document.create_element("button").unwrap();
        btn.set_class_name("vibe-run-btn");
        btn.set_text_content(Some(label));
        toolbar.append_child(&btn).unwrap();
    }
    wrapper.append_child(&toolbar).unwrap();

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
    let embedded_cell = super::vibe_cell::build_q_cell_element(document, super::vibe_cell::VibeCell::default());
    embedded_cell.set_class_name("doc-view-panel q-cell-widget");
    embedded_cell.set_attribute("data-doc-view-panel", "visual").unwrap();
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
        cml_doc.to_rdf_star_triples().len(), report.quin_count
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
                cell_el.style().set_property("color", "var(--accent-emerald)").unwrap();
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
    let badge_color = if report.conforms { "var(--accent-emerald)" } else { "var(--accent-rose)" };
    sb_el.style().set_css_text(&format!(
        "display: flex; align-items: center; gap: 5px; color: {}; font-weight: 600;",
        badge_color
    ));
    shacl_badge.set_text_content(Some(&format!("\u{2705} {}", report.status_label)));
    aura_tray.append_child(&shacl_badge).unwrap();

    // Middle: Modality & Quins Metric
    let metric_div = document.create_element("div").unwrap();
    let md_el: HtmlElement = metric_div.clone().dyn_into().unwrap();
    md_el.style().set_css_text("color: var(--text-muted); display: flex; gap: 8px;");
    metric_div.set_text_content(Some(&format!(
        "\u{1F9E0} Certainty: 96% \u{00B7} {} Quins (Sentinel Bounded)",
        report.quin_count
    )));
    aura_tray.append_child(&metric_div).unwrap();

    // Right: Action button
    let export_btn = document.create_element("button").unwrap();
    export_btn.set_class_name("vibe-run-btn");
    let eb_el: HtmlElement = export_btn.clone().dyn_into().unwrap();
    eb_el.style().set_css_text("padding: 2px 8px; font-size: 9px;");
    export_btn.set_text_content(Some("\u{1F4E6} Export .hcf"));
    aura_tray.append_child(&export_btn).unwrap();

    wrapper.append_child(&aura_tray).unwrap();

    // Wire the view switcher
    wire_doc_view_switcher(document);

    wrapper
}

/// Wire doc view switcher tabs to toggle between Visual / Markdown / RDF views.
fn wire_doc_view_switcher(document: &Document) {
    let tabs = document.query_selector_all(".doc-view-tab").unwrap();
    for i in 0..tabs.length() {
        let tab = tabs.get(i).unwrap();
        let tab_el: Element = tab.dyn_into().unwrap();
        let tab_el_for_listener = tab_el.clone();

        let closure = Closure::wrap(Box::new(move |_e: web_sys::Event| {
            let doc = web_sys::window().unwrap().document().unwrap();
            let view_id = tab_el.get_attribute("data-doc-view").unwrap_or_default();

            // Update tab styles
            let all_tabs = doc.query_selector_all(".doc-view-tab").unwrap();
            for j in 0..all_tabs.length() {
                let t = all_tabs.get(j).unwrap();
                let te: Element = t.dyn_into().unwrap();
                let te_html: HtmlElement = te.clone().dyn_into().unwrap();
                if te == tab_el {
                    te.class_list().add_1("active").unwrap();
                    te_html
                        .style()
                        .set_property("background", "var(--surface-panel-elevated)")
                        .unwrap();
                    te_html
                        .style()
                        .set_property("color", "var(--text-primary)")
                        .unwrap();
                } else {
                    te.class_list().remove_1("active").unwrap();
                    te_html
                        .style()
                        .set_property("background", "transparent")
                        .unwrap();
                    te_html
                        .style()
                        .set_property("color", "var(--text-muted)")
                        .unwrap();
                }
            }

            // Show/hide panels
            let panels = doc.query_selector_all(".doc-view-panel").unwrap();
            for j in 0..panels.length() {
                let p = panels.get(j).unwrap();
                let pe: Element = p.dyn_into().unwrap();
                let pe_html: HtmlElement = pe.clone().dyn_into().unwrap();
                let panel_view = pe.get_attribute("data-doc-view-panel").unwrap_or_default();
                if panel_view == view_id {
                    pe_html.style().set_property("display", "flex").unwrap();
                } else {
                    pe_html.style().set_property("display", "none").unwrap();
                }
            }
        }) as Box<dyn FnMut(web_sys::Event)>);

        tab_el_for_listener
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

/// Spreadsheet container — polymorphic <q-view-switcher> + formula bar + reactive grid.
pub fn build_sheet_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el
        .style()
        .set_css_text("display: flex; flex-direction: column; flex: 1; gap: 4px;");

    // Polymorphic Projection Switcher (<q-view-switcher>)
    let view_switcher = super::projections::build_view_switcher(document, "mode.spreadsheet");
    wrapper.append_child(&view_switcher).unwrap();

    // Formula bar
    let formula = document.create_element("div").unwrap();
    formula.set_class_name("vibe-toolbar");
    let fx_label = document.create_element("span").unwrap();
    fx_label.set_text_content(Some("fx"));
    let fx_label_el: HtmlElement = fx_label.clone().dyn_into().unwrap();
    fx_label_el.style().set_css_text("color: var(--accent-cyan); font-family: var(--font-mono); font-size: 11px; font-weight: 700;");
    formula.append_child(&fx_label).unwrap();

    let input = document.create_element("input").unwrap();
    let input_el: web_sys::HtmlInputElement = input.clone().dyn_into().unwrap();
    input_el.set_placeholder("=SUM(A1:A10)");
    input.set_attribute("style", "flex: 1; background: var(--canvas-bg); border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); padding: 4px 8px; color: var(--accent-emerald); font-family: var(--font-mono); font-size: 11px;").unwrap();
    formula.append_child(&input).unwrap();
    wrapper.append_child(&formula).unwrap();

    // Grid
    let grid = document.create_element("div").unwrap();
    let grid_el: HtmlElement = grid.clone().dyn_into().unwrap();
    grid_el.style().set_css_text(
        "flex: 1; overflow: auto; background: var(--canvas-bg); border: 1px solid var(--border-subtle); \
         border-radius: var(--radius-xs); font-family: var(--font-mono); font-size: 10px;"
    );

    // Header row
    let header_row = document.create_element("div").unwrap();
    let header_el: HtmlElement = header_row.clone().dyn_into().unwrap();
    header_el
        .style()
        .set_css_text("display: flex; border-bottom: 1px solid var(--border-subtle);");
    for col in &["", "A", "B", "C", "D", "E"] {
        let cell = document.create_element("div").unwrap();
        let cell_el: HtmlElement = cell.clone().dyn_into().unwrap();
        cell_el.style().set_css_text("min-width: 60px; padding: 3px 6px; text-align: center; color: var(--text-muted); border-right: 1px solid var(--border-subtle);");
        cell.set_text_content(Some(col));
        header_row.append_child(&cell).unwrap();
    }
    grid.append_child(&header_row).unwrap();

    // Data rows — cells are editable with formula support
    for row_idx in 1..=6 {
        let row = document.create_element("div").unwrap();
        let row_el: HtmlElement = row.clone().dyn_into().unwrap();
        row_el
            .style()
            .set_css_text("display: flex; border-bottom: 1px solid var(--border-subtle);");
        let row_label = document.create_element("div").unwrap();
        let rl_el: HtmlElement = row_label.clone().dyn_into().unwrap();
        rl_el.style().set_css_text("min-width: 60px; padding: 3px 6px; text-align: center; color: var(--text-muted); border-right: 1px solid var(--border-subtle);");
        row_label.set_text_content(Some(&row_idx.to_string()));
        row.append_child(&row_label).unwrap();
        for col_idx in 0..5 {
            let cell = document.create_element("div").unwrap();
            cell.set_class_name("sheet-cell");
            cell.set_attribute("data-row", &row_idx.to_string())
                .unwrap();
            let col_letter = (b'A' + col_idx as u8) as char;
            cell.set_attribute("data-col", &col_letter.to_string())
                .unwrap();
            cell.set_attribute("data-cell-ref", &format!("{}{}", col_letter, row_idx))
                .unwrap();
            let cell_el: HtmlElement = cell.clone().dyn_into().unwrap();
            cell_el.style().set_css_text(
                "min-width: 60px; padding: 3px 6px; color: var(--text-secondary); \
                 border-right: 1px solid var(--border-subtle); cursor: text; \
                 transition: var(--trans-fast);",
            );
            // Seed some initial data
            if row_idx == 1 && col_idx == 0 {
                cell.set_text_content(Some("42"));
            } else if row_idx == 2 && col_idx == 0 {
                cell.set_text_content(Some("18"));
            } else if row_idx == 3 && col_idx == 0 {
                cell.set_text_content(Some("60"));
                cell.set_attribute("data-formula", "=A1+A2").unwrap();
            }
            row.append_child(&cell).unwrap();
        }
        grid.append_child(&row).unwrap();
    }
    wrapper.append_child(&grid).unwrap();

    // Wire cell editing and formula evaluation
    wire_sheet_cells(document);

    wrapper
}

/// Wire sheet cell click-to-edit and formula evaluation.
fn wire_sheet_cells(document: &Document) {
    let cells = document.query_selector_all(".sheet-cell").unwrap();
    for i in 0..cells.length() {
        let cell = cells.get(i).unwrap();
        let cell_el: Element = cell.dyn_into().unwrap();
        let cell_el_for_listener = cell_el.clone();

        let closure = Closure::wrap(Box::new(move |_e: web_sys::Event| {
            let doc = web_sys::window().unwrap().document().unwrap();
            let cell_ref = cell_el.get_attribute("data-cell-ref").unwrap_or_default();
            let cell_ref_for_blur = cell_ref.clone();
            let formula = cell_el.get_attribute("data-formula").unwrap_or_default();

            // Don't re-edit if already editing
            if cell_el.class_list().contains("editing") {
                return;
            }
            cell_el.class_list().add_1("editing").unwrap();

            // Replace cell content with an input
            let current_text = cell_el.text_content().unwrap_or_default();
            let input = doc.create_element("input").unwrap();
            let input_el: web_sys::HtmlInputElement = input.clone().dyn_into().unwrap();
            // Show formula if present, otherwise show value
            input_el.set_value(if !formula.is_empty() {
                &formula
            } else {
                &current_text
            });
            input.set_attribute("style",
                "width: 100%; box-sizing: border-box; background: var(--surface-panel-elevated); \
                 border: 1px solid var(--accent-cyan); border-radius: 2px; padding: 2px 4px; \
                 color: var(--accent-emerald); font-family: var(--font-mono); font-size: 10px; \
                 outline: none;"
            ).unwrap();

            cell_el.set_text_content(Some(""));
            cell_el.append_child(&input).unwrap();
            input_el.focus().unwrap();
            input_el.select();

            // On Enter or blur, commit the value
            let cell_el_for_commit = cell_el.clone();
            let input_for_commit = input.clone();
            let doc_for_commit = doc.clone();
            let commit_closure = Closure::wrap(Box::new(move |ev: web_sys::Event| {
                let ke: Option<web_sys::KeyboardEvent> = ev.dyn_into().ok();
                if let Some(ke) = &ke {
                    if ke.key() != "Enter" && ke.key() != "Tab" {
                        return;
                    }
                }

                let input_el: web_sys::HtmlInputElement =
                    input_for_commit.clone().dyn_into().unwrap();
                let new_val = input_el.value();

                // Remove the input
                input_for_commit.remove();
                cell_el_for_commit.class_list().remove_1("editing").unwrap();

                // Check if it's a formula
                if let Some(expr) = new_val.strip_prefix('=') {
                    // Store formula
                    cell_el_for_commit
                        .set_attribute("data-formula", &new_val)
                        .unwrap();
                    // Evaluate
                    let result = evaluate_formula(&doc_for_commit, expr);
                    cell_el_for_commit.set_text_content(Some(&result));
                    // Style as formula result
                    let cell_html: HtmlElement = cell_el_for_commit.clone().dyn_into().unwrap();
                    cell_html
                        .style()
                        .set_property("color", "var(--accent-emerald)")
                        .unwrap();
                } else {
                    // Plain value
                    cell_el_for_commit.remove_attribute("data-formula").unwrap();
                    cell_el_for_commit.set_text_content(Some(&new_val));
                    let cell_html: HtmlElement = cell_el_for_commit.clone().dyn_into().unwrap();
                    cell_html
                        .style()
                        .set_property("color", "var(--text-secondary)")
                        .unwrap();
                }

                // Re-evaluate any cells that reference this cell
                reevaluate_dependents(&doc_for_commit, &cell_ref);
            }) as Box<dyn FnMut(web_sys::Event)>);

            input_el
                .add_event_listener_with_callback(
                    "keydown",
                    commit_closure.as_ref().unchecked_ref(),
                )
                .unwrap();
            commit_closure.forget();

            let input_for_blur = input.clone();
            let cell_el_for_blur = cell_el.clone();
            let doc_for_blur = doc.clone();
            let blur_closure = Closure::wrap(Box::new(move |_ev: web_sys::Event| {
                let input_el: web_sys::HtmlInputElement =
                    input_for_blur.clone().dyn_into().unwrap();
                let new_val = input_el.value();
                input_for_blur.remove();
                cell_el_for_blur.class_list().remove_1("editing").unwrap();

                if let Some(expr) = new_val.strip_prefix('=') {
                    cell_el_for_blur
                        .set_attribute("data-formula", &new_val)
                        .unwrap();
                    let result = evaluate_formula(&doc_for_blur, expr);
                    cell_el_for_blur.set_text_content(Some(&result));
                    let cell_html: HtmlElement = cell_el_for_blur.clone().dyn_into().unwrap();
                    cell_html
                        .style()
                        .set_property("color", "var(--accent-emerald)")
                        .unwrap();
                } else {
                    cell_el_for_blur.remove_attribute("data-formula").unwrap();
                    cell_el_for_blur.set_text_content(Some(&new_val));
                    let cell_html: HtmlElement = cell_el_for_blur.clone().dyn_into().unwrap();
                    cell_html
                        .style()
                        .set_property("color", "var(--text-secondary)")
                        .unwrap();
                }
                reevaluate_dependents(&doc_for_blur, &cell_ref_for_blur);
            }) as Box<dyn FnMut(web_sys::Event)>);

            input_el
                .add_event_listener_with_callback("blur", blur_closure.as_ref().unchecked_ref())
                .unwrap();
            blur_closure.forget();
        }) as Box<dyn FnMut(web_sys::Event)>);

        cell_el_for_listener
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

/// Evaluate a simple spreadsheet formula. Supports:
/// - Cell references: A1, B2, etc.
/// - Addition: =A1+A2
/// - Subtraction: =A1-A2
/// - Multiplication: =A1*A2
/// - Division: =A1/A2
/// - SUM range: =SUM(A1:A3)
/// - Numbers: =42+8
fn evaluate_formula(document: &Document, expr: &str) -> String {
    let expr = expr.trim();

    // SUM(range) — e.g. SUM(A1:A3)
    if let Some(rest) = expr.strip_prefix("SUM(") {
        if let Some(range) = rest.strip_suffix(')') {
            if let Some((start, end)) = range.split_once(':') {
                let sum = sum_range(document, start, end);
                return format!("{}", sum);
            }
        }
        return "#NAME?".to_string();
    }

    // Simple arithmetic: split on + - * /
    // Try + and - first (lower precedence)
    if let Some((left, right)) = split_top_level(expr, '+') {
        let lv = resolve_operand(document, left.trim());
        let rv = resolve_operand(document, right.trim());
        if let (Some(a), Some(b)) = (lv, rv) {
            return format!("{}", a + b);
        }
    }
    if let Some((left, right)) = split_top_level(expr, '-') {
        let lv = resolve_operand(document, left.trim());
        let rv = resolve_operand(document, right.trim());
        if let (Some(a), Some(b)) = (lv, rv) {
            return format!("{}", a - b);
        }
    }
    // Then * and /
    if let Some((left, right)) = split_top_level(expr, '*') {
        let lv = resolve_operand(document, left.trim());
        let rv = resolve_operand(document, right.trim());
        if let (Some(a), Some(b)) = (lv, rv) {
            return format!("{}", a * b);
        }
    }
    if let Some((left, right)) = split_top_level(expr, '/') {
        let lv = resolve_operand(document, left.trim());
        let rv = resolve_operand(document, right.trim());
        if let (Some(a), Some(b)) = (lv, rv) {
            if b == 0.0 {
                return "#DIV/0!".to_string();
            }
            return format!("{}", a / b);
        }
    }

    // Single operand (cell ref or number)
    if let Some(v) = resolve_operand(document, expr) {
        return format!("{}", v);
    }

    "#VALUE!".to_string()
}

/// Split an expression at the top-level operator (not inside parentheses).
fn split_top_level(expr: &str, op: char) -> Option<(String, String)> {
    let mut depth = 0;
    for (i, c) in expr.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            _ if depth == 0 && c == op => {
                return Some((expr[..i].to_string(), expr[i + c.len_utf8()..].to_string()));
            }
            _ => {}
        }
    }
    None
}

/// Resolve an operand: either a number or a cell reference (e.g. "A1").
fn resolve_operand(document: &Document, token: &str) -> Option<f64> {
    let token = token.trim();
    // Try parsing as a number
    if let Ok(n) = token.parse::<f64>() {
        return Some(n);
    }
    // Try as a cell reference
    get_cell_value(document, token)
}

/// Get the numeric value of a cell by reference (e.g. "A1").
fn get_cell_value(document: &Document, cell_ref: &str) -> Option<f64> {
    let selector = format!(".sheet-cell[data-cell-ref=\"{}\"]", cell_ref);
    if let Some(cell) = document.query_selector(&selector).unwrap() {
        let text = cell.text_content().unwrap_or_default();
        return text.trim().parse::<f64>().ok();
    }
    None
}

/// Sum a range of cells (e.g. from "A1" to "A3").
fn sum_range(document: &Document, start: &str, end: &str) -> f64 {
    // Parse cell refs: column letter + row number
    let (start_col, start_row) = parse_cell_ref(start);
    let (end_col, end_row) = parse_cell_ref(end);

    let mut sum = 0.0;
    for row in start_row..=end_row {
        for col in start_col..=end_col {
            let col_letter = (b'A' + col) as char;
            let ref_str = format!("{}{}", col_letter, row);
            if let Some(v) = get_cell_value(document, &ref_str) {
                sum += v;
            }
        }
    }
    sum
}

/// Parse a cell reference like "A1" into (col_index, row_number).
fn parse_cell_ref(ref_str: &str) -> (u8, u32) {
    let mut col = 0u8;
    let mut row_str = String::new();
    for c in ref_str.chars() {
        if c.is_ascii_alphabetic() {
            col = c.to_ascii_uppercase() as u8 - b'A';
        } else if c.is_ascii_digit() {
            row_str.push(c);
        }
    }
    let row = row_str.parse::<u32>().unwrap_or(1);
    (col, row)
}

/// Re-evaluate cells that have formulas and might depend on the changed cell.
fn reevaluate_dependents(document: &Document, _changed_cell: &str) {
    let cells = document
        .query_selector_all(".sheet-cell[data-formula]")
        .unwrap();
    for i in 0..cells.length() {
        let cell = cells.get(i).unwrap();
        let cell_el: Element = cell.dyn_into().unwrap();
        let formula = cell_el.get_attribute("data-formula").unwrap_or_default();
        if let Some(expr) = formula.strip_prefix('=') {
            let result = evaluate_formula(document, expr);
            cell_el.set_text_content(Some(&result));
        }
    }
}

/// Graph/SPARQL explorer container.
pub fn build_graph_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el
        .style()
        .set_css_text("display: flex; flex-direction: column; flex: 1; gap: 6px;");

    // SPARQL query input
    let query_bar = document.create_element("div").unwrap();
    query_bar.set_class_name("vibe-toolbar");
    let run_btn = document.create_element("button").unwrap();
    run_btn.set_class_name("vibe-run-btn");
    run_btn.set_text_content(Some("\u{25B6} Run SPARQL"));
    query_bar.append_child(&run_btn).unwrap();
    wrapper.append_child(&query_bar).unwrap();

    let editor = document.create_element("div").unwrap();
    editor.set_class_name("vibe-editor");
    editor.set_text_content(Some(
        "PREFIX soc: <https://qualiadb.org/ontology/social#>\n\
         SELECT ?peer ?modality WHERE {\n\
         \x20\x20?s soc:hasPeer ?peer .\n\
         \x20\x20?s soc:epistemicModality ?modality .\n\
         } LIMIT 10",
    ));
    wrapper.append_child(&editor).unwrap();

    // Results
    let results = document.create_element("div").unwrap();
    results.set_class_name("vibe-output");
    let line1 = document.create_element("div").unwrap();
    line1.set_class_name("vibe-out-line");
    line1.set_text_content(Some(
        "peer=\u{2014}did:qualia:alice, modality=\u{2014}objective",
    ));
    let line2 = document.create_element("div").unwrap();
    line2.set_class_name("vibe-out-line");
    line2.set_text_content(Some(
        "peer=\u{2014}did:qualia:bob, modality=\u{2014}subjective",
    ));
    let line3 = document.create_element("div").unwrap();
    line3.set_class_name("vibe-out-line");
    line3.set_text_content(Some(
        "\u{2139}\u{FE0F} 2 results \u{00B7} 12ms \u{00B7} CBOR-LD: 48 bytes",
    ));
    results.append_child(&line1).unwrap();
    results.append_child(&line2).unwrap();
    results.append_child(&line3).unwrap();
    wrapper.append_child(&results).unwrap();

    wrapper
}

/// Ontology browser container — tree view of classes and properties.
pub fn build_ontology_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    wrapper.set_class_name("ontology-tree");

    let tree = document.create_element("div").unwrap();
    let tree_el: HtmlElement = tree.clone().dyn_into().unwrap();
    tree_el
        .style()
        .set_css_text("overflow-y: auto; flex: 1; padding: 8px;");

    let entries = &[
        ("soc:", "Social", "class", 0),
        ("  soc:hasPeer", "hasPeer", "prop", 1),
        ("  soc:epistemicModality", "epistemicModality", "prop", 1),
        ("  soc:requestsConnection", "requestsConnection", "prop", 1),
        ("health:", "Health", "class", 0),
        ("  health:hasCondition", "hasCondition", "prop", 1),
        ("  health:hasMedication", "hasMedication", "prop", 1),
        ("geo:", "Geospatial", "class", 0),
        ("  geo:hasGeometry", "hasGeometry", "prop", 1),
        ("  geo:withinHull", "withinHull", "prop", 1),
        ("vibe:", "VibeScript", "class", 0),
        ("  vibe:Intent", "Intent", "class", 1),
        ("  vibe:Receipt", "Receipt", "class", 1),
    ];

    for (prefix, label, kind, depth) in entries {
        let node = document.create_element("div").unwrap();
        node.set_class_name("ontology-tree-node");
        let node_el: HtmlElement = node.clone().dyn_into().unwrap();
        node_el
            .style()
            .set_css_text(&format!("padding-left: {}px;", depth * 16));

        let pfx = document.create_element("span").unwrap();
        pfx.set_class_name("ot-prefix");
        pfx.set_text_content(Some(prefix));
        node.append_child(&pfx).unwrap();

        let cls = document.create_element("span").unwrap();
        cls.set_class_name(if *kind == "class" {
            "ot-class"
        } else {
            "ot-prop"
        });
        cls.set_text_content(Some(label));
        node.append_child(&cls).unwrap();

        tree.append_child(&node).unwrap();
    }

    wrapper.append_child(&tree).unwrap();
    wrapper
}

/// Pulse stream container — event log with publish allowlist.
pub fn build_pulse_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el
        .style()
        .set_css_text("display: flex; flex-direction: column; flex: 1; gap: 4px;");

    // Publish bar
    let bar = document.create_element("div").unwrap();
    bar.set_class_name("vibe-toolbar");
    let label = document.create_element("span").unwrap();
    label.set_text_content(Some("topic:"));
    let label_el: HtmlElement = label.clone().dyn_into().unwrap();
    label_el
        .style()
        .set_css_text("color: var(--text-muted); font-size: 10px; font-family: var(--font-mono);");
    bar.append_child(&label).unwrap();
    let input = document.create_element("input").unwrap();
    let input_el: web_sys::HtmlInputElement = input.clone().dyn_into().unwrap();
    input_el.set_placeholder("pulse:topic:event");
    input.set_attribute("style", "flex: 1; background: var(--canvas-bg); border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); padding: 3px 6px; color: var(--accent-cyan); font-family: var(--font-mono); font-size: 10px;").unwrap();
    bar.append_child(&input).unwrap();
    let btn = document.create_element("button").unwrap();
    btn.set_class_name("vibe-run-btn");
    btn.set_text_content(Some("Publish"));
    bar.append_child(&btn).unwrap();
    wrapper.append_child(&bar).unwrap();

    // Event log
    let log = document.create_element("div").unwrap();
    log.set_class_name("vibe-output");
    let events = &[
        "12:04:01 \u{00B7} pulse:social:connect \u{00B7} did:qualia:alice \u{2192} did:qualia:bob",
        "12:03:45 \u{00B7} pulse:graph:mutate \u{00B7} quin.statement #a42",
        "12:03:12 \u{00B7} pulse:aura:validate \u{00B7} SHACL shape:ok",
        "12:02:58 \u{00B7} pulse:telemetry:tick \u{00B7} wavefunction collapse",
    ];
    for ev in events {
        let line = document.create_element("div").unwrap();
        line.set_class_name("vibe-out-line");
        line.set_text_content(Some(ev));
        log.append_child(&line).unwrap();
    }
    wrapper.append_child(&log).unwrap();

    wrapper
}
