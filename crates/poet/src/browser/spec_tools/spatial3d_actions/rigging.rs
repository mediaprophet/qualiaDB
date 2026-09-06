//! 3D rigging, animation, and narrative scene tools.

use web_sys::Element;

pub(super) fn run(container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "spatial3d:add-bone" => Some(tag_attr(container, "data-rig-bone", "bone_added")),
        "spatial3d:weight-paint" => Some(tag_attr(container, "data-weight-paint", "brush_active")),
        "spatial3d:inverse-kinematics" => Some(tag_attr(container, "data-ik-chain", "solver_active")),
        "spatial3d:bone-constraint" => Some(tag_attr(container, "data-bone-constraint", "limit_rotation")),
        "spatial3d:pose-mirror" => Some(tag_attr(container, "data-pose-mirror", "symmetry_applied")),
        "spatial3d:rig-test" => Some(tag_attr(container, "data-rig-test", "deformation_ok")),
        "spatial3d:keyframe-insert" => Some(insert_keyframe(container)),
        "spatial3d:keyframe-edit" => Some(tag_attr(container, "data-keyframe-edit", "curve_selected")),
        "spatial3d:dope-sheet" => Some(tag_attr(container, "data-dope-sheet", "timeline_open")),
        "spatial3d:graph-editor" => Some(tag_attr(container, "data-graph-editor", "fcurve_open")),
        "spatial3d:action-clip" => Some(tag_attr(container, "data-action-clip", "clip_bound")),
        "spatial3d:animation-mixer" => Some(tag_attr(container, "data-animation-mixer", "layers_blended")),
        "spatial3d:motion-path" => Some(tag_attr(container, "data-motion-path", "path_drawn")),
        "spatial3d:add-scene" => Some(tag_attr(container, "data-scene-added", "scene_slot_open")),
        "spatial3d:scene-sequence" => Some(tag_attr(container, "data-scene-sequence", "ordered")),
        "spatial3d:camera-path" => Some(tag_attr(container, "data-camera-path", "path_keyframed")),
        "spatial3d:storyboard-frame" => Some(tag_attr(container, "data-storyboard-frame", "frame_pinned")),
        "spatial3d:trigger-zone" => Some(tag_attr(container, "data-trigger-zone", "volume_placed")),
        "spatial3d:rig-inspector" => Some(tag_attr(container, "data-rig-inspector", "bones_listed")),
        _ => None,
    }
}

fn insert_keyframe(container: &Element) -> Result<(), String> {
    let current: u32 = container
        .get_attribute("data-keyframe-count")
        .and_then(|c| c.parse().ok())
        .unwrap_or(0);
    let next = current.saturating_add(1);
    container
        .set_attribute("data-keyframe-count", &next.to_string())
        .map_err(|_| "Failed to insert keyframe.".to_string())
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
    fn rigging_actions_route_safely() {
        assert!(run(&web_sys::Element::from(wasm_bindgen::JsValue::NULL), "spatial3d:unknown").is_none());
    }
}
