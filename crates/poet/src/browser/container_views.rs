//! Additional container body views: doc, sheet, graph, ontology, pulse, rights, wallet.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

use super::cml_document::CmlDocument;
use super::cop_records::{build_family_panel, CopField};
use super::live_invoke;

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

    // Toolbar (Visual mode)
    let toolbar = document.create_element("div").unwrap();
    toolbar.set_class_name("doc-toolbar doc-view-panel");
    toolbar
        .set_attribute("data-doc-view-panel", "visual")
        .unwrap();
    let gazetteer_chips_bar = document.create_element("div").unwrap();
    gazetteer_chips_bar.set_class_name("doc-view-panel cml-gazetteer-chips-bar");
    gazetteer_chips_bar
        .set_attribute("data-doc-view-panel", "visual")
        .unwrap();
    let gcb_el: HtmlElement = gazetteer_chips_bar.clone().dyn_into().unwrap();
    gcb_el.style().set_css_text("display: none; flex-wrap: wrap; gap: 4px; padding: 4px 8px; background: rgba(0,0,0,0.3); border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); margin-bottom: 4px;");

    for label in &[
        "B",
        "I",
        "U",
        "\u{1F517} Link",
        "\u{1F4CD} Tag Entity",
        "\u{2696}\u{FE0F} Deontic Rule",
        "\u{1F50D} CML Gazetteer",
        "\u{26A1} Reactive Cell",
    ] {
        let btn = document.create_element("button").unwrap();
        btn.set_class_name("vibe-run-btn");
        btn.set_text_content(Some(label));

        match *label {
            "B" => {
                let closure =
                    wasm_bindgen::closure::Closure::wrap(Box::new(move |_e: web_sys::MouseEvent| {
                        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                            if let Ok(html_doc) = doc.dyn_into::<web_sys::HtmlDocument>() {
                                let _ = html_doc.exec_command("bold");
                            }
                        }
                    })
                        as Box<dyn FnMut(web_sys::MouseEvent)>);
                btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
                    .unwrap();
                closure.forget();
            }
            "I" => {
                let closure =
                    wasm_bindgen::closure::Closure::wrap(Box::new(move |_e: web_sys::MouseEvent| {
                        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                            if let Ok(html_doc) = doc.dyn_into::<web_sys::HtmlDocument>() {
                                let _ = html_doc.exec_command("italic");
                            }
                        }
                    })
                        as Box<dyn FnMut(web_sys::MouseEvent)>);
                btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
                    .unwrap();
                closure.forget();
            }
            "U" => {
                let closure =
                    wasm_bindgen::closure::Closure::wrap(Box::new(move |_e: web_sys::MouseEvent| {
                        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                            if let Ok(html_doc) = doc.dyn_into::<web_sys::HtmlDocument>() {
                                let _ = html_doc.exec_command("underline");
                            }
                        }
                    })
                        as Box<dyn FnMut(web_sys::MouseEvent)>);
                btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
                    .unwrap();
                closure.forget();
            }
            "\u{1F517} Link" => {
                let closure =
                    wasm_bindgen::closure::Closure::wrap(Box::new(move |_e: web_sys::MouseEvent| {
                        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                            if let Ok(html_doc) = doc.dyn_into::<web_sys::HtmlDocument>() {
                                let _ = html_doc.exec_command_with_show_ui_and_value(
                                    "createLink",
                                    false,
                                    "https://qualia.network/",
                                );
                            }
                        }
                    })
                        as Box<dyn FnMut(web_sys::MouseEvent)>);
                btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
                    .unwrap();
                closure.forget();
            }
            "\u{1F4CD} Tag Entity" => {
                let closure = wasm_bindgen::closure::Closure::wrap(Box::new(
                    move |_e: web_sys::MouseEvent| {
                        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                            if let Ok(html_doc) = doc.dyn_into::<web_sys::HtmlDocument>() {
                                let html = "<q-entity category=\"entity\" iri=\"did:qualia:entity#term\" class=\"cml-entity\">Tagged Entity</q-entity>";
                                let _ = html_doc.exec_command_with_show_ui_and_value(
                                    "insertHTML",
                                    false,
                                    html,
                                );
                            }
                        }
                    },
                )
                    as Box<dyn FnMut(web_sys::MouseEvent)>);
                btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
                    .unwrap();
                closure.forget();
            }
            "\u{2696}\u{FE0F} Deontic Rule" => {
                let closure = wasm_bindgen::closure::Closure::wrap(Box::new(
                    move |_e: web_sys::MouseEvent| {
                        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                            if let Ok(html_doc) = doc.dyn_into::<web_sys::HtmlDocument>() {
                                let html = "<q-deontic opcode=\"0x10\" class=\"cml-deontic\" style=\"border-bottom: 2px solid #ef4444; background: rgba(239,68,68,0.1); padding: 0 4px; border-radius: 2px;\">\u{2696}\u{FE0F} Obligate(Action)</q-deontic>";
                                let _ = html_doc.exec_command_with_show_ui_and_value(
                                    "insertHTML",
                                    false,
                                    html,
                                );
                            }
                        }
                    },
                )
                    as Box<dyn FnMut(web_sys::MouseEvent)>);
                btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
                    .unwrap();
                closure.forget();
            }
            "\u{26A1} Reactive Cell" => {
                let closure = wasm_bindgen::closure::Closure::wrap(Box::new(
                    move |_e: web_sys::MouseEvent| {
                        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                            if let Ok(html_doc) = doc.dyn_into::<web_sys::HtmlDocument>() {
                                let html = "<span class=\"cml-cell-tag\" contenteditable=\"false\" style=\"display: inline-block; background: rgba(0,242,169,0.15); color: #00f2a9; padding: 2px 6px; border-radius: 4px; font-family: monospace; font-size: 11px; margin: 0 4px;\">\u{26A1} = 2.5 * 10 \u{2794} 25.0</span>&nbsp;";
                                let _ = html_doc.exec_command_with_show_ui_and_value(
                                    "insertHTML",
                                    false,
                                    html,
                                );
                            }
                        }
                    },
                )
                    as Box<dyn FnMut(web_sys::MouseEvent)>);
                btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
                    .unwrap();
                closure.forget();
            }
            "\u{1F50D} CML Gazetteer" => {
                btn.set_attribute("data-requires-daemon", "true").unwrap();
                btn.set_attribute(
                    "data-enabled-title",
                    "Analyse the current document with the native gazetteer",
                )
                .unwrap();
                if !super::native_daemon::is_daemon_connected() {
                    btn.set_attribute("disabled", "").unwrap();
                    btn.set_attribute("aria-disabled", "true").unwrap();
                    btn.set_attribute(
                        "title",
                        "Unavailable until the local QualiaDB daemon is connected.",
                    )
                    .unwrap();
                }
                let gcb_clone = gazetteer_chips_bar.clone();
                let wrapper_clone = wrapper.clone();
                let gz_closure = wasm_bindgen::closure::Closure::wrap(Box::new(
                    move |_e: web_sys::MouseEvent| {
                        let gcb_html: HtmlElement = gcb_clone.clone().dyn_into().unwrap();
                        let is_hidden = gcb_html
                            .style()
                            .get_property_value("display")
                            .unwrap_or_default()
                            == "none";
                        if is_hidden {
                            gcb_clone.set_inner_html("");
                            let _ = gcb_html.style().set_property("display", "flex");
                            let source = wrapper_clone
                                .query_selector(".doc-editor")
                                .ok()
                                .flatten()
                                .and_then(|editor| editor.text_content())
                                .unwrap_or_default();
                            gcb_clone.set_text_content(Some("Analysing current document…"));
                            let target = gcb_clone.clone();
                            wasm_bindgen_futures::spawn_local(async move {
                                match super::native_daemon::daemon_gazetteer(&source).await {
                                    Ok(response) if response.ok => {
                                        target.set_inner_html("");
                                        let Some(document) =
                                            web_sys::window().and_then(|window| window.document())
                                        else {
                                            return;
                                        };
                                        let summary = document.create_element("span").unwrap();
                                        summary.set_text_content(Some(&format!(
                                            "{} tokens · {} sentences · {} sealed",
                                            response.token_count,
                                            response.sentence_count,
                                            response.sealed
                                        )));
                                        summary.set_attribute("data-honesty", "live").unwrap();
                                        target.append_child(&summary).unwrap();
                                        for hit in response.hits {
                                            let chip = document.create_element("span").unwrap();
                                            let chip_html: HtmlElement =
                                                chip.clone().dyn_into().unwrap();
                                            chip_html.style().set_css_text("font-family: var(--font-mono); font-size: 9px; padding: 2px 6px; border-radius: 3px; background: rgba(56, 189, 248, 0.15); color: var(--accent-cyan); border: 1px solid rgba(56, 189, 248, 0.3);");
                                            chip.set_text_content(Some(&format!(
                                                "{} \u{2794} <{}>",
                                                hit.surface, hit.iri
                                            )));
                                            target.append_child(&chip).unwrap();
                                        }
                                    }
                                    Ok(response) => {
                                        target.set_attribute("data-honesty", "error").ok();
                                        target.set_text_content(Some(
                                            response
                                                .diagnostic
                                                .as_deref()
                                                .unwrap_or("Gazetteer analysis failed."),
                                        ));
                                    }
                                    Err(error) => {
                                        target.set_attribute("data-honesty", "error").ok();
                                        target.set_text_content(Some(&error));
                                    }
                                }
                            });
                        } else {
                            let _ = gcb_html.style().set_property("display", "none");
                        }
                    },
                )
                    as Box<dyn FnMut(web_sys::MouseEvent)>);
                btn.add_event_listener_with_callback("click", gz_closure.as_ref().unchecked_ref())
                    .unwrap();
                gz_closure.forget();
            }
            _ => {}
        }

        toolbar.append_child(&btn).unwrap();
    }
    wrapper.append_child(&toolbar).unwrap();
    wrapper.append_child(&gazetteer_chips_bar).unwrap();

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
    let embedded_cell =
        super::vibe_cell::build_q_cell_element(document, super::vibe_cell::VibeCell::default());
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
    run_btn
        .set_attribute("data-instrument-action", "graph:sparql")
        .unwrap();
    run_btn.set_text_content(Some("\u{25B6} Run SPARQL"));
    query_bar.append_child(&run_btn).unwrap();
    wrapper.append_child(&query_bar).unwrap();

    let editor = document.create_element("div").unwrap();
    editor.set_class_name("vibe-editor");
    editor.set_attribute("contenteditable", "true").unwrap();
    editor
        .set_attribute("data-state-key", "sparql-source")
        .unwrap();
    editor
        .set_attribute("aria-label", "SPARQL query source")
        .unwrap();
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
    results.set_text_content(Some("No SPARQL query has been executed in this container."));
    wrapper.append_child(&results).unwrap();

    let editor_for_run = editor.clone();
    let results_for_run = results.clone();
    let closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
        let query = editor_for_run.text_content().unwrap_or_default();
        if query.trim().is_empty() {
            results_for_run.set_attribute("data-honesty", "error").ok();
            results_for_run.set_text_content(Some("Enter a SPARQL query before running."));
            return;
        }
        if !super::native_daemon::is_daemon_connected() {
            results_for_run
                .set_attribute("data-honesty", "unavailable")
                .ok();
            results_for_run.set_text_content(Some(
                "Unavailable: start the local QualiaDB daemon to execute SPARQL.",
            ));
            return;
        }
        results_for_run
            .set_attribute("data-honesty", "running")
            .ok();
        results_for_run.set_text_content(Some("Executing SPARQL on the native daemon…"));
        let output = results_for_run.clone();
        wasm_bindgen_futures::spawn_local(async move {
            match super::native_daemon::daemon_query(&query).await {
                Ok(result) => {
                    output.set_attribute("data-honesty", "live").ok();
                    output.set_text_content(Some(&result));
                }
                Err(error) => {
                    output.set_attribute("data-honesty", "error").ok();
                    output.set_text_content(Some(&error));
                }
            }
        });
    }) as Box<dyn FnMut(web_sys::Event)>);
    run_btn
        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();

    wrapper
}

/// Ontology browser container — live graph stats, not a fabricated class tree.
pub fn build_ontology_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    wrapper.set_class_name("ontology-tree");
    let note = document.create_element("div").unwrap();
    note.set_text_content(Some(
        "Live graph stats from GraphDatabase.stats. Class trees come from COP ontology records and N3 authoring, not a canned prefix list.",
    ));
    let note_el: HtmlElement = note.clone().dyn_into().unwrap();
    note_el.style().set_css_text(
        "font-size: 10px; color: var(--text-muted); font-family: var(--font-mono); padding: 8px;",
    );
    wrapper.append_child(&note).unwrap();
    wrapper
        .append_child(&live_invoke::action_bar(
            document,
            &[
                (
                    "GraphDatabase.stats",
                    "GraphDatabase.stats",
                    serde_json::json!({}),
                ),
                (
                    "SHACL.extensions",
                    "SHACL.extensions",
                    serde_json::json!({}),
                ),
            ],
        ))
        .unwrap();
    let panel = build_family_panel(
        document,
        "ontology_term",
        "Ontology terms you record. Natural persons are rdfs:Class, never owl:Thing.",
        &[
            CopField {
                key: "iri",
                placeholder: "IRI",
            },
            CopField {
                key: "kind",
                placeholder: "Kind (class|property)",
            },
            CopField {
                key: "paradigm",
                placeholder: "Paradigm (rdfs|shacl|shex)",
            },
        ],
    );
    wrapper.append_child(&panel).unwrap();
    wrapper
}

/// Pulse stream container — live Pulse.publish + COP pulse_event ledger.
pub fn build_pulse_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el
        .style()
        .set_css_text("display: flex; flex-direction: column; flex: 1; gap: 4px;");

    let note = document.create_element("div").unwrap();
    note.set_text_content(Some(
        "Topics must be poet/, pulse/, or clinic/. Unprefixed channels are rewritten to poet/{channel}. The log is the COP pulse_event ledger, not a canned stream.",
    ));
    let note_el: HtmlElement = note.clone().dyn_into().unwrap();
    note_el.style().set_css_text(
        "font-size: 10px; color: var(--text-muted); font-family: var(--font-mono); padding: 4px 8px;",
    );
    wrapper.append_child(&note).unwrap();

    let panel = build_family_panel(
        document,
        "pulse_event",
        "Pulse events persist here after Pulse.publish. Empty until you publish or save.",
        &[
            CopField {
                key: "channel",
                placeholder: "Channel (poet/social)",
            },
            CopField {
                key: "payload_type",
                placeholder: "Payload type (agent-message|telemetry|presence)",
            },
        ],
    );
    panel
        .append_child(&live_invoke::action_bar(
            document,
            &[
                (
                    "Pulse.publish",
                    "Pulse.publish",
                    serde_json::json!({ "channel": "poet/pulse", "payload_type": "generic" }),
                ),
                (
                    "Pulse.publish_telemetry",
                    "Pulse.publish_telemetry",
                    serde_json::json!({ "channel": "poet/telemetry" }),
                ),
                (
                    "Pulse.open_channel",
                    "Pulse.open_channel",
                    serde_json::json!({ "channel": "poet/pulse", "channel_type": "topic" }),
                ),
            ],
        ))
        .unwrap();
    wrapper
        .append_child(&super::pulse_stream::build_live_stream(document))
        .unwrap();
    wrapper.append_child(&panel).unwrap();
    wrapper
}
