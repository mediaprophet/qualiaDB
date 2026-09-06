//! 3D parametric mesh primitive creation and geometry mutations on Poet 3D containers.

use web_sys::Element;

pub(super) fn run(container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "spatial3d:add-mesh" | "spatial3d:cube" => Some(add_primitive(container, "cube")),
        "spatial3d:sphere" => Some(add_primitive(container, "sphere")),
        "spatial3d:cylinder" => Some(add_primitive(container, "cylinder")),
        "spatial3d:plane" => Some(add_primitive(container, "plane")),
        "spatial3d:torus" => Some(add_primitive(container, "torus")),
        "spatial3d:import-mesh" => Some(tag_attr(container, "data-mesh-import", "gltf_pending")),
        "spatial3d:delete-object" => Some(tag_attr(container, "data-object-deleted", "selection_removed")),
        "spatial3d:duplicate-object" => Some(tag_attr(container, "data-object-duplicated", "copy_spawned")),
        "spatial3d:parent-object" => Some(tag_attr(container, "data-object-parented", "hierarchy_linked")),
        "spatial3d:group-objects" => Some(tag_attr(container, "data-object-group", "group_active")),
        "spatial3d:snap-to-grid" => Some(toggle_snap(container)),
        "spatial3d:transform-object" => Some(tag_attr(container, "data-transform-mode", "translate_rotate_scale")),
        "spatial3d:extrude" => Some(mutate_geometry(container, "extrude_faces")),
        "spatial3d:inset" => Some(mutate_geometry(container, "inset_faces")),
        "spatial3d:bevel" => Some(mutate_geometry(container, "bevel_edges")),
        "spatial3d:loop-cut" => Some(mutate_geometry(container, "loop_cut")),
        "spatial3d:subdivide" => Some(subdivide_mesh(container)),
        "spatial3d:merge-vertices" => Some(mutate_geometry(container, "merge_vertices")),
        "spatial3d:knife-tool" => Some(tag_attr(container, "data-edit-tool", "knife")),
        "spatial3d:boolean-op" | "spatial3d:boolean-union" => Some(mutate_geometry(container, "csg_union")),
        "spatial3d:boolean-difference" => Some(mutate_geometry(container, "csg_difference")),
        _ => None,
    }
}

fn add_primitive(container: &Element, primitive: &str) -> Result<(), String> {
    let current = container
        .get_attribute("data-3d-primitives")
        .unwrap_or_default();
    let updated = if current.is_empty() {
        primitive.to_string()
    } else {
        format!("{current},{primitive}")
    };
    container
        .set_attribute("data-3d-primitives", &updated)
        .map_err(|_| format!("Failed to add 3D {primitive}."))?;
    let _ = container.set_attribute("data-active-primitive", primitive);
    Ok(())
}

fn mutate_geometry(container: &Element, op: &str) -> Result<(), String> {
    container
        .set_attribute("data-geometry-operation", op)
        .map_err(|_| format!("Failed to apply geometry operation {op}."))
}

fn subdivide_mesh(container: &Element) -> Result<(), String> {
    let current_level: u32 = container
        .get_attribute("data-subdivision-level")
        .and_then(|l| l.parse().ok())
        .unwrap_or(0);
    let next_level = current_level.saturating_add(1).min(4);
    container
        .set_attribute("data-subdivision-level", &next_level.to_string())
        .map_err(|_| "Failed to update subdivision level.".to_string())
}

fn toggle_snap(container: &Element) -> Result<(), String> {
    let current = container
        .get_attribute("data-snap-to-grid")
        .is_some_and(|v| v == "active");
    let next = if current { "off" } else { "active" };
    container
        .set_attribute("data-snap-to-grid", next)
        .map_err(|_| "Failed to toggle snap to grid.".to_string())
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
    fn mesh_primitives_route_safely() {
        assert!(run(&web_sys::Element::from(wasm_bindgen::JsValue::NULL), "spatial3d:unknown").is_none());
    }
}
