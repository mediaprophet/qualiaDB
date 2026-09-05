//! Place a SPARQL query as a graph container on the manifold canvas.

use wasm_bindgen::JsCast;
use web_sys::{Document, HtmlElement, HtmlInputElement, HtmlTextAreaElement};

pub(super) fn place_query_container(document: &Document) {
    // Get the current SPARQL query
    let query = document
        .get_element_by_id("sparql-editor")
        .and_then(|e| e.dyn_into::<HtmlTextAreaElement>().ok())
        .map(|ta| ta.value())
        .unwrap_or_default();
    let name = document
        .get_element_by_id("sparql-query-name")
        .and_then(|n| n.dyn_into::<HtmlInputElement>().ok())
        .map(|i| i.value())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "Query Results".to_string());

    place_query_container_on_canvas(document, &name, &query);
}

pub(super) fn place_named_query_container(document: &Document, query: &super::persist::SavedQuery) {
    place_query_container_on_canvas(document, &query.name, &query.query);
    // Close the workbench
    if let Some(wb) = document.get_element_by_id("search-workbench") {
        let wb_el: HtmlElement = wb.dyn_into().unwrap();
        wb_el.style().set_property("display", "none").unwrap();
    }
}

pub(super) fn place_query_container_on_canvas(document: &Document, name: &str, query: &str) {
    use crate::tool_chest::core::registry::SeedContainer;

    let existing = document
        .query_selector_all(".canvas-container-node")
        .unwrap();
    let count = existing.length() as f32;
    let x = 80.0 + (count % 5.0) * 40.0;
    let y = 60.0 + (count % 5.0) * 40.0;

    let mut container = SeedContainer {
        id: super::super::canvas_state::next_container_id("graph"),
        container_type: "graph".into(),
        title: format!("\u{1F50D} {}", name),
        x,
        y,
        width: 480.0,
        height: 360.0,
        z: 100.0 + count,
        honesty: "present".into(),
        ..Default::default()
    };
    container
        .view_state
        .insert("sparql-source".into(), format!("text:{query}"));

    if let Some(canvas) = document.get_element_by_id("manifold-canvas") {
        let el = super::super::containers::build_container(document, &container);

        // Retain query identity for canvas inspection in addition to view-state persistence.
        el.set_attribute("data-query", query).unwrap();
        el.set_attribute("data-query-name", name).unwrap();

        if let Some(content) = canvas.query_selector(".canvas-content-layer").unwrap() {
            content.append_child(&el).unwrap();
        } else {
            canvas.append_child(&el).unwrap();
        }

        // Re-wire interactions
        super::super::interactions::wire_container_selection(document);
        super::super::interactions::wire_container_dragging(document);
        super::super::interactions::wire_container_resize(document);
        super::super::interactions::wire_container_deletion(document);
        super::super::interactions::wire_port_dragging(document);

        super::super::history::push_current_frame("place query container");
    }

    super::shell::show_search_notification(
        document,
        &format!(
            "Placed \u{201C}{}\u{201D} as graph container on canvas",
            name
        ),
    );
}
