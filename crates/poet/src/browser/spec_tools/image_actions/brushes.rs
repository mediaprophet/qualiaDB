//! Image brush parameters, drawing modes, and retouching tools.

use web_sys::Element;

pub(super) fn run(container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "image:brush-select" => Some(select_brush(container)),
        "image:brush-size" => Some(step_size(container)),
        "image:brush-opacity" => Some(step_opacity(container)),
        "image:brush-hardness" => Some(step_hardness(container)),
        "image:brush-flow" => Some(step_flow(container)),
        "image:eraser" => Some(activate_eraser(container)),
        "image:clone-stamp" => Some(activate_clone_stamp(container)),
        "image:healing-brush" => Some(activate_healing_brush(container)),
        _ => None,
    }
}

pub(crate) fn next_brush_size(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("5px") => "10px",
        Some("10px") => "20px",
        Some("20px") => "40px",
        Some("40px") => "80px",
        _ => "5px",
    }
}

pub(crate) fn next_brush_opacity(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("100%") => "75%",
        Some("75%") => "50%",
        Some("50%") => "25%",
        _ => "100%",
    }
}

fn select_brush(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-active-brush", "round_standard")
        .map_err(|_| "Failed to select active brush.".to_string())
}

fn step_size(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-brush-size");
    let next = next_brush_size(current.as_deref());
    container
        .set_attribute("data-brush-size", next)
        .map_err(|_| "Failed to update brush size.".to_string())
}

fn step_opacity(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-brush-opacity");
    let next = next_brush_opacity(current.as_deref());
    container
        .set_attribute("data-brush-opacity", next)
        .map_err(|_| "Failed to update brush opacity.".to_string())
}

fn step_hardness(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-brush-hardness");
    let next = if current.as_deref() == Some("100%") { "50%" } else { "100%" };
    container
        .set_attribute("data-brush-hardness", next)
        .map_err(|_| "Failed to update brush hardness.".to_string())
}

fn step_flow(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-brush-flow", "80%")
        .map_err(|_| "Failed to update brush flow.".to_string())
}

fn activate_eraser(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-active-tool-mode", "eraser")
        .map_err(|_| "Failed to activate eraser mode.".to_string())
}

fn activate_clone_stamp(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-active-tool-mode", "clone_stamp")
        .map_err(|_| "Failed to activate clone stamp.".to_string())
}

fn activate_healing_brush(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-active-tool-mode", "healing_brush")
        .map_err(|_| "Failed to activate healing brush.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brush_size_and_opacity_cycle() {
        assert_eq!(next_brush_size(None), "5px");
        assert_eq!(next_brush_size(Some("5px")), "10px");
        assert_eq!(next_brush_size(Some("10px")), "20px");
        assert_eq!(next_brush_size(Some("20px")), "40px");
        assert_eq!(next_brush_size(Some("40px")), "80px");
        assert_eq!(next_brush_size(Some("80px")), "5px");

        assert_eq!(next_brush_opacity(None), "100%");
        assert_eq!(next_brush_opacity(Some("100%")), "75%");
        assert_eq!(next_brush_opacity(Some("75%")), "50%");
        assert_eq!(next_brush_opacity(Some("50%")), "25%");
        assert_eq!(next_brush_opacity(Some("25%")), "100%");
    }
}
