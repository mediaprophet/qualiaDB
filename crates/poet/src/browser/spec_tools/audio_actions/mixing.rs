//! Audio mixing, tempo, quantization, and metering for Poet media containers.

use web_sys::Element;

pub(super) fn run(container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "audio:bpm-tempo" | "audio:tempo-set" => Some(step_bpm(container)),
        "audio:quantize" | "audio:quantise" => Some(step_quantize(container)),
        "audio:normalize" | "audio:normalise" => Some(normalize_audio(container)),
        "audio:reverse" => Some(toggle_reverse(container)),
        "audio:spectrum-analyzer" | "audio:spectrum-analyser" => Some(toggle_spectrum(container)),
        "audio:master-limiter" => Some(toggle_limiter(container)),
        "audio:fader-level" => Some(set_fader(container)),
        "audio:send-bus" | "audio:set-send" => Some(set_send_bus(container)),
        "audio:set-bus" => Some(set_output_bus(container)),
        "audio:automation-edit" => Some(set_automation_edit(container)),
        _ => None,
    }
}

pub(crate) fn next_bpm(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("120 BPM") => "128 BPM",
        Some("128 BPM") => "140 BPM",
        Some("140 BPM") => "90 BPM",
        Some("90 BPM") => "110 BPM",
        _ => "120 BPM",
    }
}

pub(crate) fn next_quantize(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("1/4") => "1/8",
        Some("1/8") => "1/16",
        Some("1/16") => "1/32",
        _ => "1/4",
    }
}

pub(crate) fn next_fader(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("0.0dB (Unity)") => "-6.0dB",
        Some("-6.0dB") => "-12.0dB",
        Some("-12.0dB") => "+3.0dB",
        _ => "0.0dB (Unity)",
    }
}

pub(crate) fn next_output_bus(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("master") => "bus_a",
        Some("bus_a") => "bus_b",
        _ => "master",
    }
}

fn step_bpm(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-audio-bpm");
    let next = next_bpm(current.as_deref());
    container
        .set_attribute("data-audio-bpm", next)
        .map_err(|_| "Failed to update BPM tempo.".to_string())
}

fn step_quantize(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-audio-quantize");
    let next = next_quantize(current.as_deref());
    container
        .set_attribute("data-audio-quantize", next)
        .map_err(|_| "Failed to update quantization grid.".to_string())
}

fn normalize_audio(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-audio-normalized", "peak_to_-0.1dB")
        .map_err(|_| "Failed to normalize audio.".to_string())
}

fn toggle_reverse(container: &Element) -> Result<(), String> {
    let current = container
        .get_attribute("data-audio-reversed")
        .is_some_and(|v| v == "true");
    let next = if current { "false" } else { "true" };
    container
        .set_attribute("data-audio-reversed", next)
        .map_err(|_| "Failed to toggle reverse audio.".to_string())
}

fn toggle_spectrum(container: &Element) -> Result<(), String> {
    let current = container
        .get_attribute("data-spectrum-analyzer")
        .is_some_and(|v| v == "active");
    let next = if current { "inactive" } else { "active" };
    container
        .set_attribute("data-spectrum-analyzer", next)
        .map_err(|_| "Failed to toggle spectrum analyzer.".to_string())
}

fn toggle_limiter(container: &Element) -> Result<(), String> {
    let current = container
        .get_attribute("data-master-limiter")
        .is_some_and(|v| v == "engaged");
    let next = if current { "bypassed" } else { "engaged" };
    container
        .set_attribute("data-master-limiter", next)
        .map_err(|_| "Failed to toggle master limiter.".to_string())
}

fn set_fader(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-fader-level");
    let next = next_fader(current.as_deref());
    container
        .set_attribute("data-fader-level", next)
        .map_err(|_| "Failed to update fader level.".to_string())
}

fn set_send_bus(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-send-bus", "bus_aux_reverb")
        .map_err(|_| "Failed to route send bus.".to_string())
}

fn set_output_bus(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-output-bus");
    let next = next_output_bus(current.as_deref());
    container
        .set_attribute("data-output-bus", next)
        .map_err(|_| "Failed to route bus output.".to_string())
}

fn set_automation_edit(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-automation-edit", "point_moved_at_playhead")
        .map_err(|_| "Failed to edit automation point.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bpm_and_quantize_cycle() {
        assert_eq!(next_bpm(None), "120 BPM");
        assert_eq!(next_bpm(Some("110 BPM")), "120 BPM");
        assert_eq!(next_quantize(Some("1/32")), "1/4");
    }

    #[test]
    fn fader_and_bus_cycle() {
        assert_eq!(next_fader(None), "0.0dB (Unity)");
        assert_eq!(next_fader(Some("+3.0dB")), "0.0dB (Unity)");
        assert_eq!(next_output_bus(Some("bus_b")), "master");
    }
}
