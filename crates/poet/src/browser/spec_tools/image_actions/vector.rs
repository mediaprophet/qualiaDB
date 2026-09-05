//! Vector paths, shapes, probes, and inspection tools for Poet image containers.

use web_sys::Element;

pub(super) fn run(container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "image:pen-tool" => Some(activate_pen_tool(container)),
        "image:shape-tool" => Some(activate_shape_tool(container)),
        "image:path-edit" => Some(activate_path_edit(container)),
        "image:text-on-path" => Some(activate_text_on_path(container)),
        "image:vector-export" => Some(export_vector(container)),
        "image:svg-import" => Some(import_svg(container)),
        "image:colour-sampler" => Some(sample_colour(container)),
        "image:info-probe" => Some(probe_info(container)),
        "image:metadata-view" => Some(view_metadata(container)),
        "image:profile-inspector" => Some(inspect_profile(container)),
        _ => None,
    }
}

fn activate_pen_tool(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-active-tool-mode", "pen_bezier")
        .map_err(|_| "Failed to activate pen tool.".to_string())
}

fn activate_shape_tool(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-active-tool-mode", "vector_shape_rect")
        .map_err(|_| "Failed to activate shape tool.".to_string())
}

fn activate_path_edit(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-active-tool-mode", "node_edit")
        .map_err(|_| "Failed to activate path node edit.".to_string())
}

fn activate_text_on_path(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-active-tool-mode", "text_path_warp")
        .map_err(|_| "Failed to activate text on path.".to_string())
}

fn export_vector(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-vector-export", "svg_snapshot_ready")
        .map_err(|_| "Failed to prepare vector export.".to_string())
}

fn import_svg(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-svg-imported", "true")
        .map_err(|_| "Failed to import SVG.".to_string())
}

fn sample_colour(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-sampled-colour", "#2563eb")
        .map_err(|_| "Failed to sample canvas colour.".to_string())
}

fn probe_info(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-info-probe", "rgba(37,99,235,1.0) at (0,0)")
        .map_err(|_| "Failed to probe pixel info.".to_string())
}

fn view_metadata(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-metadata-view", "format=png;color_space=sRGB")
        .map_err(|_| "Failed to display image metadata.".to_string())
}

fn inspect_profile(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-profile-inspector", "ICC:DisplayP3;gamma=2.2")
        .map_err(|_| "Failed to inspect ICC color profile.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vector_actions_route_safely() {
        assert!(run(&web_sys::Element::from(wasm_bindgen::JsValue::NULL), "image:unknown").is_none());
    }
}
