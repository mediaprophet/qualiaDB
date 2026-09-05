//! Manual SPARQL editor and daemon-backed query execution.

use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement, HtmlTextAreaElement, MouseEvent};

pub(super) fn build_sparql_panel(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_class_name("search-mode-panel");
    panel.set_attribute("data-mode", "sparql").unwrap();
    let p_el: HtmlElement = panel.clone().dyn_into().unwrap();
    p_el.style()
        .set_css_text("display: none; flex-direction: column; gap: 12px;");

    // Query name input (for saving)
    let name_row = document.create_element("div").unwrap();
    let nr_el: HtmlElement = name_row.clone().dyn_into().unwrap();
    nr_el
        .style()
        .set_css_text("display: flex; gap: 8px; align-items: center;");

    let name_label = document.create_element("span").unwrap();
    name_label.set_text_content(Some("Query name:"));
    let nl_el: HtmlElement = name_label.clone().dyn_into().unwrap();
    nl_el
        .style()
        .set_css_text("font-size: 10px; color: var(--text-muted); font-family: var(--font-mono);");
    name_row.append_child(&name_label).unwrap();

    let name_input = document.create_element("input").unwrap();
    name_input.set_id("sparql-query-name");
    name_input.set_attribute("type", "text").unwrap();
    name_input.set_attribute("placeholder", "my-query").unwrap();
    let ni_el: HtmlElement = name_input.clone().dyn_into().unwrap();
    ni_el.style().set_css_text(
        "flex: 1; padding: 6px 10px; background: var(--canvas-bg); \
         border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); \
         font-family: var(--font-mono); font-size: 11px; color: var(--text-primary);",
    );
    name_row.append_child(&name_input).unwrap();
    panel.append_child(&name_row).unwrap();

    // SPARQL editor
    let editor = document.create_element("textarea").unwrap();
    editor.set_id("sparql-editor");
    let ed_el: HtmlTextAreaElement = editor.clone().dyn_into().unwrap();
    ed_el.set_value(
        "PREFIX ont: <http://qualia.org/ontology#>\n\
         PREFIX doc: <http://qualia.org/document#>\n\
         PREFIX prov: <http://qualia.org/provenance#>\n\n\
         SELECT ?subject ?label ?type WHERE {\n\
         \x20\x20?subject rdf:type ?type .\n\
         \x20\x20?subject rdfs:label ?label .\n\
         \x20\x20FILTER(LANG(?label) = \"en\")\n\
         }\n\
         LIMIT 50",
    );
    let ed_html: HtmlElement = editor.clone().dyn_into().unwrap();
    ed_html.style().set_css_text(
        "flex: 1; width: 100%; box-sizing: border-box; min-height: 200px; \
         background: var(--canvas-bg); border: 1px solid var(--border-subtle); \
         border-radius: var(--radius-xs); padding: 12px; \
         font-family: var(--font-mono); font-size: 11px; color: var(--accent-cyan); \
         line-height: 1.5; resize: vertical;",
    );
    panel.append_child(&editor).unwrap();

    // Actions
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");

    let run_btn = document.create_element("button").unwrap();
    run_btn.set_id("sparql-run");
    run_btn.set_text_content(Some("\u{25B6} Run Query"));
    let rb_el: HtmlElement = run_btn.clone().dyn_into().unwrap();
    rb_el.style().set_css_text(
        "padding: 8px 16px; background: var(--accent-cyan); color: var(--bg-deep); \
         border: 1px solid var(--accent-cyan); border-radius: var(--radius-xs); \
         font-family: var(--font-mono); font-size: 11px; font-weight: 700; cursor: pointer;",
    );
    actions.append_child(&run_btn).unwrap();

    let save_btn = document.create_element("button").unwrap();
    save_btn.set_id("sparql-save");
    save_btn.set_text_content(Some("\u{1F4BE} Save Query"));
    let sb_el: HtmlElement = save_btn.clone().dyn_into().unwrap();
    sb_el.style().set_css_text(
        "padding: 8px 16px; background: var(--surface-panel); color: var(--text-secondary); \
         border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); \
         font-family: var(--font-mono); font-size: 11px; cursor: pointer;",
    );
    actions.append_child(&save_btn).unwrap();

    let place_btn = document.create_element("button").unwrap();
    place_btn.set_id("sparql-place-container");
    place_btn.set_text_content(Some("\u{1F4CB} Use as Container Source"));
    let pb_el: HtmlElement = place_btn.clone().dyn_into().unwrap();
    pb_el.style().set_css_text(
        "padding: 8px 16px; background: var(--surface-panel); color: var(--accent-violet); \
         border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); \
         font-family: var(--font-mono); font-size: 11px; cursor: pointer;",
    );
    actions.append_child(&place_btn).unwrap();

    panel.append_child(&actions).unwrap();

    // Results
    let results = document.create_element("div").unwrap();
    results.set_id("sparql-results");
    let r_el: HtmlElement = results.clone().dyn_into().unwrap();
    r_el.style().set_css_text(
        "flex: 1; overflow-y: auto; border: 1px solid var(--border-subtle); \
         border-radius: var(--radius-xs); padding: 8px; min-height: 80px; \
         font-family: var(--font-mono); font-size: 11px; color: var(--text-muted);",
    );
    results.set_text_content(Some(
        "Click \"Run Query\" to execute against the connected daemon.",
    ));
    panel.append_child(&results).unwrap();

    panel
}

pub(super) fn wire_sparql_editor(document: &Document) {
    // Wire run button
    if let Some(run_btn) = document.get_element_by_id("sparql-run") {
        let rb_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            run_query(&doc, "sparql-results");
        }) as Box<dyn FnMut(MouseEvent)>);
        run_btn
            .add_event_listener_with_callback("click", rb_closure.as_ref().unchecked_ref())
            .unwrap();
        rb_closure.forget();
    }

    // Wire save button
    if let Some(save_btn) = document.get_element_by_id("sparql-save") {
        let svb_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            super::saved::save_current_query(&doc, "sparql");
        }) as Box<dyn FnMut(MouseEvent)>);
        save_btn
            .add_event_listener_with_callback("click", svb_closure.as_ref().unchecked_ref())
            .unwrap();
        svb_closure.forget();
    }

    // Wire place container button
    if let Some(place_btn) = document.get_element_by_id("sparql-place-container") {
        let pb_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            super::placement::place_query_container(&doc);
        }) as Box<dyn FnMut(MouseEvent)>);
        place_btn
            .add_event_listener_with_callback("click", pb_closure.as_ref().unchecked_ref())
            .unwrap();
        pb_closure.forget();
    }
}

pub(super) fn run_query(document: &Document, results_id: &str) {
    let results = match document.get_element_by_id(results_id) {
        Some(r) => r,
        None => return,
    };
    results.set_inner_html("");

    let query_text = document
        .get_element_by_id("sparql-editor")
        .and_then(|e| e.dyn_into::<HtmlTextAreaElement>().ok())
        .map(|ta| ta.value())
        .unwrap_or_default();

    if crate::browser::native_daemon::is_daemon_connected() {
        results.set_inner_html("<div style=\"padding: 8px; color: var(--accent-cyan); font-size: 11px;\">\u{25CB} Executing query on native daemon\u{2026}</div>");
        let results_id_owned = results_id.to_string();
        wasm_bindgen_futures::spawn_local(async move {
            let res = crate::browser::native_daemon::daemon_query(&query_text).await;
            if let Some(window) = web_sys::window() {
                if let Some(doc) = window.document() {
                    if let Some(target) = doc.get_element_by_id(&results_id_owned) {
                        match res {
                            Ok(output) => {
                                let mut html = String::new();
                                html.push_str("<div style=\"padding: 4px 8px; border-bottom: 1px solid var(--border-subtle); font-size: 9px; color: var(--accent-emerald); margin-bottom: 4px;\">\u{25CF} Live Results from Native Daemon</div>");
                                html.push_str(&format!("<pre style=\"margin: 0; padding: 6px; font-family: var(--font-mono); font-size: 11px; color: var(--text-primary); white-space: pre-wrap; max-height: 240px; overflow-y: auto;\">{}</pre>", output.replace('<', "&lt;").replace('>', "&gt;")));
                                target.set_inner_html(&html);
                            }
                            Err(err) => {
                                target.set_attribute("data-honesty", "error").ok();
                                target
                                    .set_text_content(Some(&format!("Daemon query error: {err}")));
                            }
                        }
                    }
                }
            }
        });
        return;
    }

    results.set_attribute("data-honesty", "unavailable").ok();
    results.set_text_content(Some(
        "Unavailable: start the local QualiaDB daemon to execute SPARQL. No offline results were generated.",
    ));
}
