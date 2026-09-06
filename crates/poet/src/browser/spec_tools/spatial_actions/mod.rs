//! Spatial tool actions router for Poet spec tools (`spatial:*` Local rows).

mod manifold;
mod scene;

use web_sys::{Document, Element};

/// Dispatches a spatial tool action to its specific handler.
///
/// `None` — another provider may handle this id (e.g. Live / Place rows).
/// `Some(Ok(true))` — DOM markers were updated on the selected container.
/// `Some(Ok(false))` — recognised but intentionally no-op (unused here).
/// `Some(Err(_))` — honest failure; no partial mutation committed.
pub fn run(document: &Document, container: &Element, tool_id: &str) -> Option<Result<bool, String>> {
    if !tool_id.starts_with("spatial:") {
        return None;
    }
    scene::run(document, container, tool_id)
        .or_else(|| manifold::run(document, container, tool_id))
}

pub(super) fn js_error(_: wasm_bindgen::JsValue) -> String {
    "DOM operation failed.".to_string()
}

pub(super) fn append_semicolon_attr(container: &Element, attr: &str, item: &str) -> Result<(), String> {
    let current = container.get_attribute(attr).unwrap_or_default();
    let updated = if current.is_empty() {
        item.to_string()
    } else {
        format!("{current};{item}")
    };
    container
        .set_attribute(attr, &updated)
        .map_err(|_| format!("Failed to update {attr}."))
}

pub(super) fn ensure_scene_root(document: &Document, container: &Element) -> Result<Element, String> {
    if let Ok(Some(existing)) = container.query_selector("[data-spatial-scene]") {
        return Ok(existing);
    }
    let scene = document.create_element("div").map_err(js_error)?;
    scene
        .set_attribute("data-spatial-scene", "root")
        .map_err(js_error)?;
    container
        .append_child(&scene)
        .map_err(js_error)?;
    let _ = container.set_attribute("data-spatial-scene-ready", "true");
    Ok(scene)
}

pub(super) fn active_node_id(container: &Element) -> Option<String> {
    container.get_attribute("data-active-spatial-node")
}

pub(super) fn select_node(container: &Element, node_id: &str) -> Result<(), String> {
    container
        .set_attribute("data-active-spatial-node", node_id)
        .map_err(|_| "Failed to select spatial node.".to_string())
}

pub(super) fn find_node(container: &Element, node_id: &str) -> Result<Option<Element>, String> {
    let selector = format!("[data-spatial-node=\"{node_id}\"]");
    container
        .query_selector(&selector)
        .map_err(|_| "Failed to query spatial node.".to_string())
}

pub(super) fn node_count(container: &Element) -> Result<u32, String> {
    Ok(container
        .query_selector_all("[data-spatial-node]")
        .map_err(|_| "Failed to count spatial nodes.".to_string())?
        .length())
}

pub(super) fn ok_true(result: Result<(), String>) -> Result<bool, String> {
    result.map(|()| true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spatial_prefix_gate() {
        let container = web_sys::Element::from(wasm_bindgen::JsValue::NULL);
        let document = web_sys::Document::from(wasm_bindgen::JsValue::NULL);
        assert!(run(&document, &container, "image:add-layer").is_none());
        assert!(run(&document, &container, "spatial:unknown-tool").is_none());
    }
}
