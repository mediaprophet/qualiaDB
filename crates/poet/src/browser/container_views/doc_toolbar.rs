//! Visual-mode CML toolbar and gazetteer chip bar.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

pub(super) fn append_doc_toolbar(document: &Document, wrapper: &Element) {
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
                btn.set_attribute(
                    "data-enabled-title",
                    "Analyse the current document with Poet's bounded extractor",
                )
                .unwrap();
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
                            if !crate::browser::native_daemon::is_daemon_connected() {
                                let (token_count, sentence_count, entities) =
                                    crate::browser::tool_actions::local_extract_summary(&source);
                                gcb_clone.set_inner_html("");
                                let Some(document) =
                                    web_sys::window().and_then(|window| window.document())
                                else {
                                    return;
                                };
                                let summary = document.create_element("span").unwrap();
                                summary.set_text_content(Some(&format!(
                                    "{} tokens · {} sentences · {} local entities",
                                    token_count,
                                    sentence_count,
                                    entities.len()
                                )));
                                summary.set_attribute("data-honesty", "local").unwrap();
                                gcb_clone.append_child(&summary).unwrap();
                                for entity in entities {
                                    let chip = document.create_element("span").unwrap();
                                    chip.set_text_content(Some(&entity));
                                    chip.set_attribute("data-honesty", "local").unwrap();
                                    gcb_clone.append_child(&chip).unwrap();
                                }
                                return;
                            }
                            gcb_clone.set_text_content(Some("Analysing current document…"));
                            let target = gcb_clone.clone();
                            wasm_bindgen_futures::spawn_local(async move {
                                match crate::browser::native_daemon::daemon_gazetteer(&source).await
                                {
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
}
