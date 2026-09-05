//! SPARQL explorer container.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

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
        if !crate::browser::native_daemon::is_daemon_connected() {
            let Some(document) = web_sys::window().and_then(|window| window.document()) else {
                results_for_run.set_text_content(Some("Browser document is unavailable."));
                return;
            };
            let result = crate::browser::tool_actions::local_graph_query(&document, &query);
            results_for_run.set_attribute("data-honesty", "local").ok();
            results_for_run.set_text_content(Some(&result));
            return;
        }
        results_for_run
            .set_attribute("data-honesty", "running")
            .ok();
        results_for_run.set_text_content(Some("Executing SPARQL on the native daemon…"));
        let output = results_for_run.clone();
        wasm_bindgen_futures::spawn_local(async move {
            match crate::browser::native_daemon::daemon_query(&query).await {
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
