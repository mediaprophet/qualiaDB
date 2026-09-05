//! Audio track state mutations (mute, solo, arm, pan, track management) on Poet media containers.

use wasm_bindgen::JsCast;
use web_sys::{Element, HtmlMediaElement};

pub(super) fn run(container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "audio:mute-track" => Some(toggle_mute(container)),
        "audio:solo-track" => Some(toggle_solo(container)),
        "audio:arm-recording" => Some(toggle_arm(container)),
        "audio:pan" => Some(step_pan(container)),
        "audio:add-track" => Some(add_track(container)),
        "audio:delete-track" => Some(delete_track(container)),
        _ => None,
    }
}

pub(crate) fn next_pan(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("C (Center)") => "L50",
        Some("L50") => "L100",
        Some("L100") => "R50",
        Some("R50") => "R100",
        _ => "C (Center)",
    }
}

fn toggle_mute(container: &Element) -> Result<(), String> {
    let current = container
        .get_attribute("data-track-muted")
        .is_some_and(|v| v == "true");
    let next = !current;
    if let Ok(Some(el)) = container.query_selector("audio, video") {
        if let Ok(media) = el.dyn_into::<HtmlMediaElement>() {
            media.set_muted(next);
        }
    }
    container
        .set_attribute("data-track-muted", if next { "true" } else { "false" })
        .map_err(|_| "Failed to toggle track mute.".to_string())
}

fn toggle_solo(container: &Element) -> Result<(), String> {
    let current = container
        .get_attribute("data-track-solo")
        .is_some_and(|v| v == "true");
    let next = if current { "false" } else { "true" };
    container
        .set_attribute("data-track-solo", next)
        .map_err(|_| "Failed to toggle track solo.".to_string())
}

fn toggle_arm(container: &Element) -> Result<(), String> {
    let current = container
        .get_attribute("data-track-armed")
        .is_some_and(|v| v == "true");
    let next = if current { "false" } else { "true" };
    container
        .set_attribute("data-track-armed", next)
        .map_err(|_| "Failed to toggle track arm.".to_string())
}

fn step_pan(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-track-pan");
    let next = next_pan(current.as_deref());
    container
        .set_attribute("data-track-pan", next)
        .map_err(|_| "Failed to update stereo pan.".to_string())
}

fn add_track(container: &Element) -> Result<(), String> {
    let tracks: u32 = container
        .get_attribute("data-track-count")
        .and_then(|c| c.parse().ok())
        .unwrap_or(1);
    let next = tracks.saturating_add(1);
    container
        .set_attribute("data-track-count", &next.to_string())
        .map_err(|_| "Failed to increment track count.".to_string())
}

fn delete_track(container: &Element) -> Result<(), String> {
    let tracks: u32 = container
        .get_attribute("data-track-count")
        .and_then(|c| c.parse().ok())
        .unwrap_or(1);
    let next = tracks.saturating_sub(1).max(1);
    container
        .set_attribute("data-track-count", &next.to_string())
        .map_err(|_| "Failed to update track count.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stereo_pan_cycles_through_positions() {
        assert_eq!(next_pan(None), "C (Center)");
        assert_eq!(next_pan(Some("C (Center)")), "L50");
        assert_eq!(next_pan(Some("L50")), "L100");
        assert_eq!(next_pan(Some("L100")), "R50");
        assert_eq!(next_pan(Some("R50")), "R100");
        assert_eq!(next_pan(Some("R100")), "C (Center)");
    }
}
