//! 3D viewport overlays, materials, cameras, lighting, and scene inspection.

use web_sys::Element;

pub(super) fn run(container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "spatial3d:wireframe-toggle" => Some(toggle_wireframe(container)),
        "spatial3d:bounding-box" => Some(toggle_bounding_box(container)),
        "spatial3d:poly-count" | "spatial3d:mesh-statistics" => Some(audit_poly_count(container)),
        "spatial3d:camera-focal-length" | "spatial3d:camera-frame" => Some(step_focal_length(container)),
        "spatial3d:camera-add" | "spatial3d:add-camera" => Some(add_camera(container)),
        "spatial3d:light-point" | "spatial3d:add-light" => Some(add_point_light(container)),
        "spatial3d:light-type" => Some(cycle_light_type(container)),
        "spatial3d:render-viewport" => Some(render_viewport(container)),
        "spatial3d:export-gltf" | "spatial3d:narrative-export" => Some(export_gltf(container)),
        "spatial3d:create-material" => Some(tag_attr(container, "data-material-created", "pbr_default")),
        "spatial3d:assign-material" => Some(tag_attr(container, "data-material-assigned", "selection_bound")),
        "spatial3d:texture-paint" => Some(tag_attr(container, "data-texture-paint", "brush_active")),
        "spatial3d:uv-edit" => Some(tag_attr(container, "data-uv-edit", "unwrap_mode")),
        "spatial3d:material-node" => Some(tag_attr(container, "data-material-node", "shader_graph_open")),
        "spatial3d:pbr-preset" => Some(cycle_pbr_preset(container)),
        "spatial3d:hdri-setup" => Some(tag_attr(container, "data-hdri", "studio_neutral.hdr")),
        "spatial3d:shadow-setup" => Some(tag_attr(container, "data-shadow-setup", "soft_shadows=on")),
        "spatial3d:material-inspector" => Some(tag_attr(container, "data-material-inspector", "slots_listed")),
        "spatial3d:scene-graph" => Some(tag_attr(container, "data-scene-graph", "hierarchy_expanded")),
        "spatial3d:metadata-view" => Some(tag_attr(container, "data-scene-metadata", "objects=1;materials=1")),
        _ => None,
    }
}

pub(crate) fn next_focal_length(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("50mm (Standard)") => "85mm (Portrait)",
        Some("85mm (Portrait)") => "24mm (Wide)",
        Some("24mm (Wide)") => "35mm (Cinematic)",
        _ => "50mm (Standard)",
    }
}

pub(crate) fn next_pbr_preset(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("matte_plastic") => "brushed_metal",
        Some("brushed_metal") => "clear_glass",
        Some("clear_glass") => "rough_concrete",
        _ => "matte_plastic",
    }
}

fn toggle_wireframe(container: &Element) -> Result<(), String> {
    let current = container
        .get_attribute("data-wireframe-mode")
        .is_some_and(|v| v == "active");
    let next = if current { "shaded" } else { "active" };
    container
        .set_attribute("data-wireframe-mode", next)
        .map_err(|_| "Failed to toggle wireframe mode.".to_string())
}

fn toggle_bounding_box(container: &Element) -> Result<(), String> {
    let current = container
        .get_attribute("data-bounding-box")
        .is_some_and(|v| v == "visible");
    let next = if current { "hidden" } else { "visible" };
    container
        .set_attribute("data-bounding-box", next)
        .map_err(|_| "Failed to toggle bounding box.".to_string())
}

fn audit_poly_count(container: &Element) -> Result<(), String> {
    let primitives_count = container
        .get_attribute("data-3d-primitives")
        .unwrap_or_default()
        .split(',')
        .filter(|p| !p.is_empty())
        .count()
        .max(1);
    let estimated_polys = primitives_count * 1024;
    container
        .set_attribute("data-poly-count", &format!("{estimated_polys} triangles"))
        .map_err(|_| "Failed to audit polygon count.".to_string())
}

fn step_focal_length(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-camera-focal-length");
    let next = next_focal_length(current.as_deref());
    container
        .set_attribute("data-camera-focal-length", next)
        .map_err(|_| "Failed to update camera focal length.".to_string())
}

fn add_camera(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-active-camera", "camera:perspective_rig")
        .map_err(|_| "Failed to add perspective camera.".to_string())
}

fn add_point_light(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-active-light", "light:point_radiance")
        .map_err(|_| "Failed to add point light.".to_string())
}

fn cycle_light_type(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-light-type");
    let next = match current.as_deref() {
        Some("point") => "spot",
        Some("spot") => "directional",
        Some("directional") => "area",
        _ => "point",
    };
    container
        .set_attribute("data-light-type", next)
        .map_err(|_| "Failed to cycle light type.".to_string())
}

fn cycle_pbr_preset(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-pbr-preset");
    let next = next_pbr_preset(current.as_deref());
    container
        .set_attribute("data-pbr-preset", next)
        .map_err(|_| "Failed to apply PBR preset.".to_string())
}

fn render_viewport(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-viewport-rendered", "true")
        .map_err(|_| "Failed to trigger viewport render.".to_string())
}

fn export_gltf(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-export-gltf", "binary_gltf_prepared")
        .map_err(|_| "Failed to prepare glTF export.".to_string())
}

fn tag_attr(container: &Element, key: &str, value: &str) -> Result<(), String> {
    container
        .set_attribute(key, value)
        .map_err(|_| format!("Failed to set {key}."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focal_lengths_cycle_through_common_prime_lenses() {
        assert_eq!(next_focal_length(None), "50mm (Standard)");
        assert_eq!(next_focal_length(Some("35mm (Cinematic)")), "50mm (Standard)");
    }

    #[test]
    fn pbr_presets_cycle() {
        assert_eq!(next_pbr_preset(None), "matte_plastic");
        assert_eq!(next_pbr_preset(Some("clear_glass")), "rough_concrete");
    }
}
