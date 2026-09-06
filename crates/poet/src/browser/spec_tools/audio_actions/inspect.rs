//! Inspect/query markers for audio meters and metadata on Poet media containers.

use web_sys::Element;

pub(super) fn run(container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "audio:waveform-view" => Some(set_marker(
            container,
            "data-waveform-view",
            "visible:selected_clip",
        )),
        "audio:phase-meter" => Some(toggle_phase_meter(container)),
        "audio:loudness-meter" => Some(set_marker(
            container,
            "data-loudness-meter",
            "integrated_LUFS=-14.0",
        )),
        "audio:metadata-view" => Some(set_marker(
            container,
            "data-audio-metadata",
            "rate=48000Hz;depth=24bit;channels=2;length=00:03:12",
        )),
        _ => None,
    }
}

fn set_marker(container: &Element, attr: &str, value: &str) -> Result<(), String> {
    container
        .set_attribute(attr, value)
        .map_err(|_| format!("Failed to set {attr}."))
}

fn toggle_phase_meter(container: &Element) -> Result<(), String> {
    let current = container
        .get_attribute("data-phase-meter")
        .is_some_and(|v| v == "visible");
    let next = if current { "hidden" } else { "visible" };
    container
        .set_attribute("data-phase-meter", next)
        .map_err(|_| "Failed to toggle phase meter.".to_string())
}
