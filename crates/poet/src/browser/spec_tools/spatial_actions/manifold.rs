//! Ten-axis manifold camera navigation markers on Poet 3D containers.

use super::{active_node_id, append_semicolon_attr, find_node, ok_true};
use web_sys::Element;

pub(super) fn run(_document: &web_sys::Document, container: &Element, tool_id: &str) -> Option<Result<bool, String>> {
    Some(match tool_id {
        "spatial:navigate-spatial" => ok_true(navigate_axis(container, "spatial", "dx=1,dy=0,dz=0")),
        "spatial:navigate-temporal" => ok_true(navigate_axis(container, "temporal", "dt=1")),
        "spatial:navigate-quantum" => ok_true(navigate_axis(container, "quantum", "branch=what_if")),
        "spatial:navigate-topology" => ok_true(cycle_topology(container)),
        "spatial:navigate-manifold" => ok_true(cycle_manifold_domain(container)),
        "spatial:navigate-spectral" => ok_true(navigate_axis(container, "spectral", "brightness=+0.1")),
        "spatial:focus-node" => ok_true(focus_node(container)),
        "spatial:set-manifold-camera" => ok_true(set_manifold_camera(container)),
        "spatial:set-spectral-perception" => ok_true(set_spectral_perception(container)),
        _ => return None,
    })
}

pub(crate) fn next_topology(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("flat") => "loop",
        Some("loop") => "tree",
        Some("tree") => "hyperbolic",
        _ => "flat",
    }
}

pub(crate) fn next_manifold_domain(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("commons") => "bilateral",
        Some("bilateral") => "spatial",
        Some("spatial") => "temporal",
        Some("temporal") => "spectral",
        _ => "commons",
    }
}

pub(crate) fn next_spectral_perception(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("human_visible") => "extended_spectrum",
        Some("extended_spectrum") => "modulation_aware",
        Some("modulation_aware") => "signature_trace",
        _ => "human_visible",
    }
}

fn navigate_axis(container: &Element, axis: &str, delta: &str) -> Result<(), String> {
    append_semicolon_attr(container, "data-manifold-nav-log", &format!("{axis}:{delta}"))?;
    container
        .set_attribute("data-manifold-nav-active", axis)
        .map_err(|_| "Failed to record manifold navigation.".to_string())
}

fn cycle_topology(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-manifold-topology");
    let next = next_topology(current.as_deref());
    container
        .set_attribute("data-manifold-topology", next)
        .map_err(|_| "Failed to update manifold topology.".to_string())
}

fn cycle_manifold_domain(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-manifold-domain");
    let next = next_manifold_domain(current.as_deref());
    container
        .set_attribute("data-manifold-domain", next)
        .map_err(|_| "Failed to change manifold domain.".to_string())
}

fn focus_node(container: &Element) -> Result<(), String> {
    let node_id = active_node_id(container).ok_or_else(|| {
        "Select a scene node before focusing the camera.".to_string()
    })?;
    let Some(node) = find_node(container, &node_id)? else {
        return Err("Active spatial node was not found in this scene.".to_string());
    };
    let transform = node
        .get_attribute("data-spatial-transform")
        .unwrap_or_else(|| "tx=0,ty=0,tz=0".to_string());
    container
        .set_attribute("data-manifold-camera-focus", &format!("target={node_id};{transform}"))
        .map_err(|_| "Failed to focus manifold camera on node.".to_string())
}

fn set_manifold_camera(container: &Element) -> Result<(), String> {
    container
        .set_attribute(
            "data-manifold-camera-axes",
            "spatial,temporal,topology,spectral,quantum",
        )
        .map_err(|_| "Failed to set manifold camera axes.".to_string())
}

fn set_spectral_perception(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-manifold-spectral-perception");
    let next = next_spectral_perception(current.as_deref());
    container
        .set_attribute("data-manifold-spectral-perception", next)
        .map_err(|_| "Failed to update spectral perception mode.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifold_cycles_are_deterministic() {
        assert_eq!(next_topology(None), "flat");
        assert_eq!(next_topology(Some("tree")), "hyperbolic");
        assert_eq!(next_manifold_domain(None), "commons");
        assert_eq!(next_manifold_domain(Some("temporal")), "spectral");
        assert_eq!(next_spectral_perception(None), "human_visible");
        assert_eq!(
            next_spectral_perception(Some("modulation_aware")),
            "signature_trace"
        );
    }
}
