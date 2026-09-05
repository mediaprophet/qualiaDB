//! Wire Visual / Markdown / RDF-Star tabs on a CML HyperDoc.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

/// Wire doc view switcher tabs to toggle between Visual / Markdown / RDF views.
pub(super) fn wire_doc_view_switcher(document: &Document) {
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
