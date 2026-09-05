//! Virtual world generation, avatar poses, skybox environments, and telemetry for Poet portals.

use web_sys::Element;

pub(super) fn run(container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "portals:world-create" => Some(create_world(container)),
        "portals:world-load" => Some(load_world(container)),
        "portals:spawn-point" => Some(set_spawn_point(container)),
        "portals:skybox-set" => Some(cycle_skybox(container)),
        "portals:avatar-select" => Some(select_avatar(container)),
        "portals:avatar-pose" => Some(cycle_pose(container)),
        "portals:avatar-emote" => Some(trigger_emote(container)),
        "portals:voice-zone" => Some(toggle_voice_zone(container)),
        "portals:portal-link" => Some(link_portal(container)),
        "portals:telemetry-view" => Some(view_telemetry(container)),
        "portals:visitor-count" => Some(query_visitors(container)),
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

pub(crate) fn next_pose(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("pose:idle_standing") => "pose:conversational",
        Some("pose:conversational") => "pose:presentation",
        Some("pose:presentation") => "pose:seated_council",
        _ => "pose:idle_standing",
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

fn link_portal(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-portal-link", "target=world:adjacent_domain")
        .map_err(|_| "Failed to configure portal link.".to_string())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skybox_and_avatar_poses_cycle() {
        assert_eq!(next_skybox(None), "sky:clear_noon");
        assert_eq!(next_skybox(Some("sky:clear_noon")), "sky:golden_sunset");
        assert_eq!(next_skybox(Some("sky:golden_sunset")), "sky:midnight_aurora");
        assert_eq!(next_skybox(Some("sky:midnight_aurora")), "sky:deep_cosmos");
        assert_eq!(next_skybox(Some("sky:deep_cosmos")), "sky:clear_noon");

        assert_eq!(next_pose(None), "pose:idle_standing");
        assert_eq!(next_pose(Some("pose:idle_standing")), "pose:conversational");
        assert_eq!(next_pose(Some("pose:conversational")), "pose:presentation");
        assert_eq!(next_pose(Some("pose:presentation")), "pose:seated_council");
        assert_eq!(next_pose(Some("pose:seated_council")), "pose:idle_standing");
    }
}
