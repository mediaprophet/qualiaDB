//! 3D parametric mesh primitive creation and geometry mutations on Poet 3D containers.

use web_sys::Element;

pub(super) fn run(container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "spatial3d:cube" => Some(add_primitive(container, "cube")),
        "spatial3d:sphere" => Some(add_primitive(container, "sphere")),
        "spatial3d:cylinder" => Some(add_primitive(container, "cylinder")),
        "spatial3d:plane" => Some(add_primitive(container, "plane")),
        "spatial3d:torus" => Some(add_primitive(container, "torus")),
        "spatial3d:extrude" => Some(mutate_geometry(container, "extrude_faces")),
        "spatial3d:bevel" => Some(mutate_geometry(container, "bevel_edges")),
        "spatial3d:subdivide" => Some(subdivide_mesh(container)),
        "spatial3d:boolean-union" => Some(mutate_geometry(container, "csg_union")),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mesh_primitives_route_safely() {
        assert!(run(&web_sys::Element::from(wasm_bindgen::JsValue::NULL), "spatial3d:unknown").is_none());
    }
}
