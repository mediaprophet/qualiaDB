//! Scene graph, viewport, layout, and mesh-adjacent markers on Poet 3D containers.

use super::{
    active_node_id, append_semicolon_attr, ensure_scene_root, find_node, js_error, node_count,
    ok_true, select_node,
};
use web_sys::{Document, Element};

const HEAVY_MESH_MSG: &str =
    "Mesh editing and file import/export need the native geometry pipeline; not on this surface.";

pub(super) fn run(document: &Document, container: &Element, tool_id: &str) -> Option<Result<bool, String>> {
    Some(match tool_id {
        "spatial:add-node" => ok_true(add_node(document, container)),
        "spatial:remove-node" => ok_true(remove_node(container)),
        "spatial:set-transform" => ok_true(set_transform(container)),
        "spatial:set-mesh" => ok_true(set_mesh(container)),
        "spatial:set-material" => ok_true(set_material(container)),
        "spatial:add-light" => ok_true(add_light(document, container)),
        "spatial:add-camera" => ok_true(add_camera(document, container)),
        "spatial:link-semantic" => ok_true(link_semantic(container)),
        "spatial:duplicate-node" => ok_true(duplicate_node(document, container)),
        "spatial:import-mesh" | "spatial:export-mesh" | "spatial:edit-vertex" | "spatial:subdivide-mesh"
        | "spatial:decimate-mesh" | "spatial:compute-normals" | "spatial:add-rig"
        | "spatial:add-blend-space" => Err(HEAVY_MESH_MSG.to_string()),
        "spatial:set-viewport" => ok_true(set_viewport(container)),
        "spatial:set-clear-colour" => ok_true(set_clear_colour(container)),
        "spatial:add-post-process" => ok_true(add_post_process(container)),
        "spatial:capture-frame" => ok_true(capture_frame(container)),
        "spatial:set-render-budget" => ok_true(set_render_budget(container)),
        "spatial:place-ui-element" => ok_true(place_ui_element(document, container)),
        "spatial:snap-to-grid" => ok_true(snap_to_grid(container)),
        "spatial:snap-to-surface" => ok_true(snap_to_surface(container)),
        "spatial:measure-distance" => ok_true(measure_distance(container)),
        "spatial:align-nodes" => ok_true(align_nodes(container)),
        "spatial:group-nodes" => ok_true(group_nodes(document, container)),
        "spatial:set-spatial-layout" => ok_true(set_spatial_layout(container)),
        _ => return None,
    })
}

pub(crate) fn next_clear_colour(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("#0f172a") => "#1e293b",
        Some("#1e293b") => "#334155",
        Some("#334155") => "#f8fafc",
        Some("#f8fafc") => "#fef3c7",
        _ => "#0f172a",
    }
}

pub(crate) fn next_post_process(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("none") => "bloom",
        Some("bloom") => "tone_map",
        Some("tone_map") => "fxaa",
        Some("fxaa") => "ssao",
        _ => "none",
    }
}

pub(crate) fn next_layout_mode(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("grid") => "ring",
        Some("ring") => "tree",
        Some("tree") => "line",
        _ => "grid",
    }
}

fn add_node(document: &Document, container: &Element) -> Result<(), String> {
    let scene = ensure_scene_root(document, container)?;
    let next = node_count(container)?.saturating_add(1);
    let node_id = format!("node_{next}");
    let node = document.create_element("div").map_err(js_error)?;
    node.set_attribute("data-spatial-node", &node_id).map_err(js_error)?;
    node.set_attribute(
        "data-spatial-transform",
        "tx=0,ty=0,tz=0;rx=0,ry=0,rz=0;sx=1,sy=1,sz=1;time=0",
    )
    .map_err(js_error)?;
    scene.append_child(&node).map_err(js_error)?;
    select_node(container, &node_id)?;
    append_semicolon_attr(container, "data-spatial-node-list", &node_id)
}

fn remove_node(container: &Element) -> Result<(), String> {
    let node_id = active_node_id(container).ok_or_else(|| {
        "Select a scene node first (add a node or set the active node).".to_string()
    })?;
    let Some(node) = find_node(container, &node_id)? else {
        return Err("Active spatial node was not found in this scene.".to_string());
    };
    node.remove();
    let _ = container.remove_attribute("data-active-spatial-node");
    Ok(())
}

fn set_transform(container: &Element) -> Result<(), String> {
    let node_id = active_node_id(container).ok_or_else(|| {
        "Select a scene node before setting its pose.".to_string()
    })?;
    let Some(node) = find_node(container, &node_id)? else {
        return Err("Active spatial node was not found in this scene.".to_string());
    };
    node.set_attribute(
        "data-spatial-transform",
        "tx=1,ty=0.5,tz=0;rx=0,ry=45,rz=0;sx=1,sy=1,sz=1;time=0",
    )
    .map_err(|_| "Failed to set spatial transform.".to_string())
}

fn set_mesh(container: &Element) -> Result<(), String> {
    let node_id = active_node_id(container).ok_or_else(|| {
        "Select a scene node before assigning a mesh.".to_string()
    })?;
    let Some(node) = find_node(container, &node_id)? else {
        return Err("Active spatial node was not found in this scene.".to_string());
    };
    node.set_attribute("data-spatial-mesh", "mesh:unit_cube")
        .map_err(|_| "Failed to assign mesh marker.".to_string())
}

fn set_material(container: &Element) -> Result<(), String> {
    let node_id = active_node_id(container).ok_or_else(|| {
        "Select a scene node before assigning a material.".to_string()
    })?;
    let Some(node) = find_node(container, &node_id)? else {
        return Err("Active spatial node was not found in this scene.".to_string());
    };
    node.set_attribute("data-spatial-material", "mat:pbr_neutral;roughness=0.4;metalness=0.1")
        .map_err(|_| "Failed to assign material marker.".to_string())
}

fn add_light(document: &Document, container: &Element) -> Result<(), String> {
    let scene = ensure_scene_root(document, container)?;
    let light = document.create_element("div").map_err(js_error)?;
    light
        .set_attribute("data-spatial-light", "light:point_radiance")
        .map_err(js_error)?;
    light
        .set_attribute("data-spatial-transform", "tx=2,ty=3,tz=1")
        .map_err(js_error)?;
    scene.append_child(&light).map_err(js_error)?;
    append_semicolon_attr(container, "data-spatial-lights", "light:point_radiance")
}

fn add_camera(document: &Document, container: &Element) -> Result<(), String> {
    let scene = ensure_scene_root(document, container)?;
    let camera = document.create_element("div").map_err(js_error)?;
    camera
        .set_attribute("data-spatial-camera", "camera:perspective_rig")
        .map_err(js_error)?;
    camera
        .set_attribute("data-spatial-transform", "tx=0,ty=1.6,tz=4;look_at=origin")
        .map_err(js_error)?;
    scene.append_child(&camera).map_err(js_error)?;
    container
        .set_attribute("data-active-spatial-camera", "camera:perspective_rig")
        .map_err(|_| "Failed to register spatial camera.".to_string())
}

fn link_semantic(container: &Element) -> Result<(), String> {
    let node_id = active_node_id(container).ok_or_else(|| {
        "Select a scene node before linking meaning.".to_string()
    })?;
    let Some(node) = find_node(container, &node_id)? else {
        return Err("Active spatial node was not found in this scene.".to_string());
    };
    node.set_attribute("data-spatial-semantic", "term:human_supplied_record")
        .map_err(|_| "Failed to link semantic term.".to_string())
}

fn duplicate_node(document: &Document, container: &Element) -> Result<(), String> {
    let node_id = active_node_id(container).ok_or_else(|| {
        "Select a scene node to duplicate.".to_string()
    })?;
    let Some(source) = find_node(container, &node_id)? else {
        return Err("Active spatial node was not found in this scene.".to_string());
    };
    let scene = ensure_scene_root(document, container)?;
    let next = node_count(container)?.saturating_add(1);
    let clone_id = format!("node_{next}");
    let clone = document.create_element("div").map_err(js_error)?;
    clone
        .set_attribute("data-spatial-node", &clone_id)
        .map_err(js_error)?;
    if let Some(transform) = source.get_attribute("data-spatial-transform") {
        let _ = clone.set_attribute("data-spatial-transform", &transform);
    }
    if let Some(mesh) = source.get_attribute("data-spatial-mesh") {
        let _ = clone.set_attribute("data-spatial-mesh", &mesh);
    }
    if let Some(material) = source.get_attribute("data-spatial-material") {
        let _ = clone.set_attribute("data-spatial-material", &material);
    }
    clone
        .set_attribute("data-spatial-duplicated-from", &node_id)
        .map_err(js_error)?;
    scene.append_child(&clone).map_err(js_error)?;
    select_node(container, &clone_id)
}

fn set_viewport(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-spatial-viewport", "1280x720;format=rgba8")
        .map_err(|_| "Failed to set spatial viewport.".to_string())
}

fn set_clear_colour(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-spatial-clear-colour");
    let next = next_clear_colour(current.as_deref());
    container
        .set_attribute("data-spatial-clear-colour", next)
        .map_err(|_| "Failed to set viewport background colour.".to_string())
}

fn add_post_process(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-spatial-post-process");
    let next = next_post_process(current.as_deref());
    append_semicolon_attr(container, "data-spatial-post-process", next)
}

fn capture_frame(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-spatial-frame-capture", "snapshot:viewport_rgba8_pending")
        .map_err(|_| "Failed to mark frame capture.".to_string())
}

fn set_render_budget(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-spatial-render-budget", "16ms_per_frame")
        .map_err(|_| "Failed to set render time budget.".to_string())
}

fn place_ui_element(document: &Document, container: &Element) -> Result<(), String> {
    let scene = ensure_scene_root(document, container)?;
    let panel = document.create_element("div").map_err(js_error)?;
    panel
        .set_attribute("data-spatial-ui", "panel:control_handle")
        .map_err(js_error)?;
    panel
        .set_attribute("data-spatial-transform", "tx=0,ty=0,tz=0.01")
        .map_err(js_error)?;
    scene.append_child(&panel).map_err(js_error)?;
    append_semicolon_attr(container, "data-spatial-ui-elements", "panel:control_handle")
}

fn snap_to_grid(container: &Element) -> Result<(), String> {
    let node_id = active_node_id(container).ok_or_else(|| {
        "Select a scene node before snapping to grid.".to_string()
    })?;
    let Some(node) = find_node(container, &node_id)? else {
        return Err("Active spatial node was not found in this scene.".to_string());
    };
    node.set_attribute("data-spatial-snap", "grid:1.0")
        .map_err(|_| "Failed to snap node to grid.".to_string())
}

fn snap_to_surface(container: &Element) -> Result<(), String> {
    let node_id = active_node_id(container).ok_or_else(|| {
        "Select a scene node before snapping to a surface.".to_string()
    })?;
    let Some(node) = find_node(container, &node_id)? else {
        return Err("Active spatial node was not found in this scene.".to_string());
    };
    node.set_attribute("data-spatial-snap", "surface:nearest_mesh")
        .map_err(|_| "Failed to snap node to surface.".to_string())
}

fn measure_distance(container: &Element) -> Result<(), String> {
    let count = node_count(container)?;
    if count < 2 {
        return Err("Add at least two scene nodes before measuring distance.".to_string());
    }
    container
        .set_attribute("data-spatial-measurement", "distance=1.414;units=scene")
        .map_err(|_| "Failed to record distance measurement.".to_string())
}

fn align_nodes(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-spatial-align", "axis=x;nodes=active_set")
        .map_err(|_| "Failed to align spatial nodes.".to_string())
}

fn group_nodes(document: &Document, container: &Element) -> Result<(), String> {
    let scene = ensure_scene_root(document, container)?;
    let group = document.create_element("div").map_err(js_error)?;
    group
        .set_attribute("data-spatial-group", "group:active")
        .map_err(js_error)?;
    scene.append_child(&group).map_err(js_error)?;
    container
        .set_attribute("data-spatial-active-group", "group:active")
        .map_err(|_| "Failed to group spatial nodes.".to_string())
}

fn set_spatial_layout(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-spatial-layout");
    let next = next_layout_mode(current.as_deref());
    container
        .set_attribute("data-spatial-layout", next)
        .map_err(|_| "Failed to apply spatial layout.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clear_colour_and_post_process_cycle() {
        assert_eq!(next_clear_colour(None), "#0f172a");
        assert_eq!(next_clear_colour(Some("#0f172a")), "#1e293b");
        assert_eq!(next_post_process(None), "none");
        assert_eq!(next_post_process(Some("bloom")), "tone_map");
        assert_eq!(next_layout_mode(None), "grid");
        assert_eq!(next_layout_mode(Some("ring")), "tree");
    }
}
