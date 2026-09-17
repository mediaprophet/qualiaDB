//! Virtual world building, object placement, and world state for Poet portals.

use web_sys::Element;

pub(super) fn run(container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "portals:world-create" | "portals:create-world" => Some(create_world(container)),
        "portals:world-load" => Some(load_world(container)),
        "portals:terrain-edit" => Some(tag_attr(container, "data-terrain-edit", "sculpt_brush_active")),
        "portals:spawn-point" => Some(set_spawn_point(container)),
        "portals:skybox-set" => Some(cycle_skybox(container)),
        "portals:boundary-set" => Some(tag_attr(container, "data-world-boundary", "radius=512m")),
        "portals:world-save" => Some(tag_attr(container, "data-world-save", "snapshot_written")),
        "portals:world-publish" => Some(tag_attr(container, "data-world-publish", "host_manifest_staged")),
        "portals:place-prop" => Some(place_item(container, "prop")),
        "portals:place-furniture" => Some(place_item(container, "furniture")),
        "portals:interactive-object" => Some(place_item(container, "interactive")),
        "portals:portal-anchor" => Some(tag_attr(container, "data-portal-anchor", "marker_placed")),
        "portals:object-transform" => Some(tag_attr(container, "data-object-transform", "translate_rotate_scale")),
        "portals:object-duplicate" => Some(tag_attr(container, "data-object-duplicate", "copy_spawned")),
        "portals:object-delete" => Some(tag_attr(container, "data-object-delete", "selection_removed")),
        "portals:telemetry-view" => Some(view_telemetry(container)),
        "portals:visitor-count" | "portals:player-list" => Some(query_visitors(container)),
        "portals:gravity-set" => Some(cycle_gravity(container)),
        _ => None,
    }
}

pub(crate) fn next_skybox(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("sky:clear_noon") => "sky:golden_sunset",
        Some("sky:golden_sunset") => "sky:midnight_aurora",
        Some("sky:midnight_aurora") => "sky:deep_cosmos",
        _ => "sky:clear_noon",
    }
}

fn create_world(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-portal-world-id", "world:genesis_commons")
        .map_err(|_| "Failed to create portal world.".to_string())?;
    let _ = container.set_attribute("data-portal-skybox", "sky:clear_noon");
    Ok(())
}

fn load_world(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-portal-world-status", "loaded:active_manifold")
        .map_err(|_| "Failed to load portal world.".to_string())
}

fn set_spawn_point(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-portal-spawn", "x=0,y=0,z=0;rot=0")
        .map_err(|_| "Failed to set spawn point.".to_string())
}

fn cycle_skybox(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-portal-skybox");
    let next = next_skybox(current.as_deref());
    container
        .set_attribute("data-portal-skybox", next)
        .map_err(|_| "Failed to update skybox environment.".to_string())
}

fn place_item(container: &Element, kind: &str) -> Result<(), String> {
    let current = container
        .get_attribute("data-world-objects")
        .unwrap_or_default();
    let updated = if current.is_empty() {
        kind.to_string()
    } else {
        format!("{current},{kind}")
    };
    container
        .set_attribute("data-world-objects", &updated)
        .map_err(|_| format!("Failed to place {kind}."))
}

fn view_telemetry(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-world-telemetry", "fps=60;draw_calls=48;vram=240MB")
        .map_err(|_| "Failed to display portal telemetry.".to_string())
}

fn query_visitors(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-visitor-count", "1 active presence")
        .map_err(|_| "Failed to query world visitors.".to_string())
}

fn cycle_gravity(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-world-gravity");
    let next = if current.as_deref() == Some("gravity:zero_g") {
        "gravity:earth_9.8ms2"
    } else {
        "gravity:zero_g"
    };
    container
        .set_attribute("data-world-gravity", next)
        .map_err(|_| "Failed to update world gravity.".to_string())
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
    fn skybox_cycles() {
        assert_eq!(next_skybox(None), "sky:clear_noon");
        assert_eq!(next_skybox(Some("sky:deep_cosmos")), "sky:clear_noon");
    }
}
