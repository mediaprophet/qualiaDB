//! Audio track state mutations (mute, solo, arm, pan, track management) on Poet media containers.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlMediaElement};

pub(super) fn run(
    document: &Document,
    container: &Element,
    tool_id: &str,
) -> Option<Result<(), String>> {
    match tool_id {
        "audio:mute-track" => Some(toggle_mute(container)),
        "audio:solo-track" => Some(toggle_solo(container)),
        "audio:arm-recording" => Some(toggle_arm(container)),
        "audio:pan" | "audio:track-pan" => Some(step_pan(container)),
        "audio:add-track" | "audio:add-audio-track" => Some(add_audio_track(container)),
        "audio:add-midi-track" => Some(add_midi_track(container)),
        "audio:add-bus" => Some(add_bus(container)),
        "audio:delete-track" => Some(delete_track(container)),
        "audio:rename-track" => Some(rename_track(document, container)),
        "audio:track-routing" => Some(cycle_routing(container)),
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

pub(crate) fn next_track_name(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("Track 1") => "Track 2",
        Some("Track 2") => "Track 3",
        Some("Track 3") => "Track 4",
        _ => "Track 1",
    }
}

pub(crate) fn next_routing(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("master") => "bus_a",
        Some("bus_a") => "bus_b",
        Some("bus_b") => "aux_reverb",
        _ => "master",
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

fn add_audio_track(container: &Element) -> Result<(), String> {
    increment_count(container, "data-track-count", "Failed to increment track count.")
}

fn add_midi_track(container: &Element) -> Result<(), String> {
    increment_count(
        container,
        "data-midi-track-count",
        "Failed to increment MIDI track count.",
    )
}

fn add_bus(container: &Element) -> Result<(), String> {
    increment_count(container, "data-bus-count", "Failed to increment bus count.")
}

fn increment_count(container: &Element, attr: &str, err: &str) -> Result<(), String> {
    let tracks: u32 = container
        .get_attribute(attr)
        .and_then(|c| c.parse().ok())
        .unwrap_or(0u32)
        .saturating_add(1);
    container
        .set_attribute(attr, &tracks.to_string())
        .map_err(|_| err.to_string())
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

fn rename_track(_document: &Document, container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-track-name");
    if let Some(window) = web_sys::window() {
        if let Ok(Some(value)) = window.prompt_with_message_and_default(
            "Track name",
            current.as_deref().unwrap_or("Track 1"),
        ) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return container
                    .set_attribute("data-track-name", trimmed)
                    .map_err(|_| "Failed to rename track.".to_string());
            }
        }
    }
    let next = next_track_name(current.as_deref());
    container
        .set_attribute("data-track-name", next)
        .map_err(|_| "Failed to rename track.".to_string())
}

fn cycle_routing(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-track-routing");
    let next = next_routing(current.as_deref());
    container
        .set_attribute("data-track-routing", next)
        .map_err(|_| "Failed to update track routing.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stereo_pan_cycles_through_positions() {
        assert_eq!(next_pan(None), "C (Center)");
        assert_eq!(next_pan(Some("R50")), "R100");
        assert_eq!(next_pan(Some("R100")), "C (Center)");
    }

    #[test]
    fn track_names_and_routing_cycle() {
        assert_eq!(next_track_name(None), "Track 1");
        assert_eq!(next_track_name(Some("Track 4")), "Track 1");
        assert_eq!(next_routing(Some("aux_reverb")), "master");
    }
}
