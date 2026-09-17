//! Projection surface mapping and calibration (Local rows only).

use web_sys::Element;

pub(super) fn run(container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "productions:add-surface" => Some(add_surface(container)),
        "productions:surface-map" => Some(tag_attr(container, "data-surface-map", "content_warped")),
        "productions:edge-blend" => Some(tag_attr(container, "data-edge-blend", "overlap_softened")),
        "productions:geometry-correct" => Some(tag_attr(container, "data-geometry-correct", "keystone_applied")),
        "productions:calibration-point" => Some(pin_calibration(container)),
        "productions:projection-content" => Some(tag_attr(container, "data-projection-content", "clip_assigned")),
        _ => None,
    }
}

fn add_surface(container: &Element) -> Result<(), String> {
    let current = container
        .get_attribute("data-projection-surfaces")
        .unwrap_or_default();
    let entry = "surface_flat_01";
    let updated = if current.is_empty() {
        entry.to_string()
    } else {
        format!("{current},{entry}")
    };
    container
        .set_attribute("data-projection-surfaces", &updated)
        .map_err(|_| "Failed to add projection surface.".to_string())
}

fn pin_calibration(container: &Element) -> Result<(), String> {
    let current: u32 = container
        .get_attribute("data-calibration-points")
        .and_then(|c| c.parse().ok())
        .unwrap_or(0);
    let next = current.saturating_add(1);
    container
        .set_attribute("data-calibration-points", &next.to_string())
        .map_err(|_| "Failed to pin calibration point.".to_string())
}

fn tag_attr(container: &Element, key: &str, value: &str) -> Result<(), String> {
    container
        .set_attribute(key, value)
        .map_err(|_| format!("Failed to set {key}."))
}
