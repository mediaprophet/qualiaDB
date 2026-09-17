//! Audio transport helpers that are honest DOM markers only (not a DAW).

use web_sys::Element;

pub(super) fn run(container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "audio:metronome" => Some(toggle_metronome(container)),
        "audio:record" => Some(Err(
            "Recording needs microphone access and is not available in this browser preview."
                .to_string(),
        )),
        _ => None,
    }
}

fn toggle_metronome(container: &Element) -> Result<(), String> {
    let current = container
        .get_attribute("data-metronome")
        .is_some_and(|v| v == "on");
    let next = if current { "off" } else { "on" };
    container
        .set_attribute("data-metronome", next)
        .map_err(|_| "Failed to toggle metronome click.".to_string())
}
