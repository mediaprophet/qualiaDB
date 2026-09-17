//! Portal links, destinations, and avatar configuration.

use web_sys::Element;

pub(super) fn run(container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "portals:create-portal" => Some(tag_attr(container, "data-portal-created", "link_spawned")),
        "portals:portal-link" => Some(link_portal(container)),
        "portals:portal-destination" => Some(tag_attr(container, "data-portal-destination", "world:adjacent_domain")),
        "portals:portal-list" => Some(list_portals(container)),
        "portals:portal-test" => Some(tag_attr(container, "data-portal-test", "traversal_ok")),
        "portals:avatar-select" => Some(select_avatar(container)),
        "portals:avatar-customise" | "portals:avatar-pose" => Some(cycle_pose(container)),
        "portals:avatar-emote" => Some(trigger_emote(container)),
        "portals:controller-map" => Some(tag_attr(container, "data-controller-map", "hands_sticks_bound")),
        "portals:avatar-spawn" => Some(tag_attr(container, "data-avatar-spawn", "placed_at_spawn")),
        "portals:voice-zone" => Some(toggle_voice_zone(container)),
        _ => None,
    }
}

pub(crate) fn next_pose(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("pose:idle_standing") => "pose:conversational",
        Some("pose:conversational") => "pose:presentation",
        Some("pose:presentation") => "pose:seated_council",
        _ => "pose:idle_standing",
    }
}

fn link_portal(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-portal-link", "target=world:adjacent_domain")
        .map_err(|_| "Failed to configure portal link.".to_string())
}

fn list_portals(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-portal-list", "portals=1;active=genesis_gate")
        .map_err(|_| "Failed to list portals.".to_string())
}

fn select_avatar(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-avatar-model", "avatar:humanoid_standard")
        .map_err(|_| "Failed to select avatar model.".to_string())
}

fn cycle_pose(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-avatar-pose");
    let next = next_pose(current.as_deref());
    container
        .set_attribute("data-avatar-pose", next)
        .map_err(|_| "Failed to update avatar pose.".to_string())
}

fn trigger_emote(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-avatar-emote", "emote:wave_greeting")
        .map_err(|_| "Failed to trigger avatar emote.".to_string())
}

fn toggle_voice_zone(container: &Element) -> Result<(), String> {
    let current = container
        .get_attribute("data-voice-zone")
        .is_some_and(|v| v == "spatial_active");
    let next = if current { "disabled" } else { "spatial_active" };
    container
        .set_attribute("data-voice-zone", next)
        .map_err(|_| "Failed to toggle spatial voice zone.".to_string())
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
    fn avatar_poses_cycle() {
        assert_eq!(next_pose(None), "pose:idle_standing");
        assert_eq!(next_pose(Some("pose:seated_council")), "pose:idle_standing");
    }
}
