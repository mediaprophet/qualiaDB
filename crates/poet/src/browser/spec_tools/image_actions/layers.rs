//! Image layer mutations, stack ordering, and blend modes on Poet canvas containers.

use web_sys::Element;

pub(super) fn run(container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "image:add-layer" => Some(add_layer(container)),
        "image:delete-layer" => Some(delete_layer(container)),
        "image:reorder-layer" => Some(reorder_layer(container)),
        "image:merge-layers" => Some(merge_layers(container)),
        "image:duplicate-layer" => Some(duplicate_layer(container)),
        "image:group-layers" => Some(group_layers(container)),
        "image:layer-blend-mode" => Some(cycle_blend_mode(container)),
        _ => None,
    }
}

pub(crate) fn next_blend_mode(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("normal") => "multiply",
        Some("multiply") => "screen",
        Some("screen") => "overlay",
        Some("overlay") => "darken",
        Some("darken") => "lighten",
        Some("lighten") => "color-dodge",
        _ => "normal",
    }
}

fn add_layer(container: &Element) -> Result<(), String> {
    let current_layers: u32 = container
        .get_attribute("data-layer-count")
        .and_then(|c| c.parse().ok())
        .unwrap_or(1);
    let next_layers = current_layers.saturating_add(1);
    container
        .set_attribute("data-layer-count", &next_layers.to_string())
        .map_err(|_| "Failed to increment layer count.".to_string())?;
    let _ = container.set_attribute("data-active-layer", &format!("layer_{next_layers}"));
    Ok(())
}

fn delete_layer(container: &Element) -> Result<(), String> {
    let current_layers: u32 = container
        .get_attribute("data-layer-count")
        .and_then(|c| c.parse().ok())
        .unwrap_or(1);
    let remaining = current_layers.saturating_sub(1).max(1);
    container
        .set_attribute("data-layer-count", &remaining.to_string())
        .map_err(|_| "Failed to update layer count.".to_string())?;
    let _ = container.set_attribute("data-active-layer", &format!("layer_{remaining}"));
    Ok(())
}

fn reorder_layer(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-layers-reordered", "true")
        .map_err(|_| "Failed to reorder layers.".to_string())
}

fn merge_layers(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-layer-count", "1")
        .map_err(|_| "Failed to merge layers.".to_string())?;
    let _ = container.set_attribute("data-active-layer", "layer_flattened");
    Ok(())
}

fn duplicate_layer(container: &Element) -> Result<(), String> {
    add_layer(container)
}

fn group_layers(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-layer-group", "group_active")
        .map_err(|_| "Failed to group layers.".to_string())
}

fn cycle_blend_mode(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-layer-blend-mode");
    let next = next_blend_mode(current.as_deref());
    container
        .set_attribute("data-layer-blend-mode", next)
        .map_err(|_| "Failed to update layer blend mode.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blend_mode_cycles() {
        assert_eq!(next_blend_mode(None), "normal");
        assert_eq!(next_blend_mode(Some("normal")), "multiply");
        assert_eq!(next_blend_mode(Some("multiply")), "screen");
        assert_eq!(next_blend_mode(Some("screen")), "overlay");
        assert_eq!(next_blend_mode(Some("overlay")), "darken");
        assert_eq!(next_blend_mode(Some("darken")), "lighten");
        assert_eq!(next_blend_mode(Some("lighten")), "color-dodge");
        assert_eq!(next_blend_mode(Some("color-dodge")), "normal");
    }
}
