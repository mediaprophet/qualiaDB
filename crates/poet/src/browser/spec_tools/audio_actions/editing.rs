//! Clip editing markers on Poet media containers (not real waveform surgery).

use web_sys::Element;

pub(super) fn run(container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "audio:split-clip" => Some(set_marker(container, "data-clip-split", "split_at_playhead")),
        "audio:join-clips" => Some(set_marker(container, "data-clip-joined", "true")),
        "audio:trim-clip" => Some(cycle_trim(container)),
        "audio:fade-in" => Some(cycle_fade(container, "data-fade-in")),
        "audio:fade-out" => Some(cycle_fade(container, "data-fade-out")),
        "audio:crossfade" => Some(set_marker(container, "data-crossfade", "250ms")),
        "audio:time-stretch" => Some(cycle_time_stretch(container)),
        "audio:pitch-shift" => Some(cycle_pitch_shift(container)),
        _ => None,
    }
}

fn set_marker(container: &Element, attr: &str, value: &str) -> Result<(), String> {
    container
        .set_attribute(attr, value)
        .map_err(|_| format!("Failed to set {attr}."))
}

fn cycle_trim(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-clip-trimmed");
    let next = match current.as_deref() {
        Some("start") => "end",
        Some("end") => "both",
        _ => "start",
    };
    container
        .set_attribute("data-clip-trimmed", next)
        .map_err(|_| "Failed to trim clip.".to_string())
}

fn cycle_fade(container: &Element, attr: &str) -> Result<(), String> {
    let current = container.get_attribute(attr);
    let next = match current.as_deref() {
        Some("250ms") => "500ms",
        Some("500ms") => "1000ms",
        _ => "250ms",
    };
    container
        .set_attribute(attr, next)
        .map_err(|_| format!("Failed to update {attr}."))
}

pub(crate) fn next_time_stretch(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("90%") => "100%",
        Some("100%") => "110%",
        Some("110%") => "125%",
        _ => "90%",
    }
}

fn cycle_time_stretch(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-time-stretch");
    let next = next_time_stretch(current.as_deref());
    container
        .set_attribute("data-time-stretch", next)
        .map_err(|_| "Failed to time-stretch clip.".to_string())
}

pub(crate) fn next_pitch_shift(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("-12st") => "-7st",
        Some("-7st") => "0st",
        Some("0st") => "+7st",
        Some("+7st") => "+12st",
        _ => "0st",
    }
}

fn cycle_pitch_shift(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-pitch-shift");
    let next = next_pitch_shift(current.as_deref());
    container
        .set_attribute("data-pitch-shift", next)
        .map_err(|_| "Failed to shift pitch.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_stretch_and_pitch_shift_cycle() {
        assert_eq!(next_time_stretch(None), "90%");
        assert_eq!(next_time_stretch(Some("125%")), "90%");
        assert_eq!(next_pitch_shift(None), "0st");
        assert_eq!(next_pitch_shift(Some("+12st")), "0st");
    }
}
