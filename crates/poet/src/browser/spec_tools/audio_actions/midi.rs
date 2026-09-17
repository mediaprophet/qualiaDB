//! MIDI note and control-lane markers on Poet media containers.

use web_sys::Element;

pub(super) fn run(container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "audio:note-draw" => Some(set_marker(container, "data-midi-note-added", "C4@0.0")),
        "audio:note-edit" => Some(set_marker(container, "data-midi-note-edited", "pitch_or_length")),
        "audio:velocity-edit" => Some(cycle_velocity(container)),
        "audio:cc-edit" => Some(cycle_cc_lane(container)),
        "audio:transpose" => Some(cycle_transpose(container)),
        "audio:midi-import" => Some(Err(
            "Bringing in MIDI needs a file picker and is not available in this browser preview."
                .to_string(),
        )),
        _ => None,
    }
}

fn set_marker(container: &Element, attr: &str, value: &str) -> Result<(), String> {
    container
        .set_attribute(attr, value)
        .map_err(|_| format!("Failed to set {attr}."))
}

pub(crate) fn next_velocity(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("64") => "96",
        Some("96") => "127",
        Some("127") => "32",
        _ => "64",
    }
}

fn cycle_velocity(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-midi-velocity");
    let next = next_velocity(current.as_deref());
    container
        .set_attribute("data-midi-velocity", next)
        .map_err(|_| "Failed to update note velocity.".to_string())
}

pub(crate) fn next_cc_lane(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("modulation") => "expression",
        Some("expression") => "sustain",
        _ => "modulation",
    }
}

fn cycle_cc_lane(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-cc-lane");
    let next = next_cc_lane(current.as_deref());
    container
        .set_attribute("data-cc-lane", next)
        .map_err(|_| "Failed to update control lane.".to_string())
}

pub(crate) fn next_transpose(current: Option<&str>) -> i8 {
    match current.map(str::trim) {
        Some("-12") => -7,
        Some("-7") => 0,
        Some("0") => 7,
        Some("7") => 12,
        _ => 0,
    }
}

fn cycle_transpose(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-midi-transpose");
    let next = next_transpose(current.as_deref());
    container
        .set_attribute("data-midi-transpose", &next.to_string())
        .map_err(|_| "Failed to transpose notes.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn midi_cycles_progress() {
        assert_eq!(next_velocity(None), "64");
        assert_eq!(next_velocity(Some("127")), "32");
        assert_eq!(next_cc_lane(None), "modulation");
        assert_eq!(next_transpose(Some("12")), 0);
    }
}
