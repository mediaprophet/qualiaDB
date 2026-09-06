//! Portal physics bodies, colliders, and local inspection.

use web_sys::Element;

pub(super) fn run(container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "portals:add-collider" => Some(tag_attr(container, "data-collider", "box_shape_added")),
        "portals:add-rigidbody" => Some(tag_attr(container, "data-rigidbody", "dynamic_body_added")),
        "portals:add-trigger" => Some(tag_attr(container, "data-physics-trigger", "volume_sensor_added")),
        "portals:physics-material" => Some(tag_attr(container, "data-physics-material", "friction=0.5;bounce=0.2")),
        "portals:physics-bake" => Some(tag_attr(container, "data-physics-bake", "scene_baked")),
        _ => None,
    }
}

fn tag_attr(container: &Element, key: &str, value: &str) -> Result<(), String> {
    container
        .set_attribute(key, value)
        .map_err(|_| format!("Failed to set {key}."))
}
