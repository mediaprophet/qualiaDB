//! Theatrical cue sequencing, show control, and power estimation (Local rows only).

use web_sys::Element;

pub(super) fn run(container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "productions:cue-record" | "productions:add-cue" => Some(record_cue(container)),
        "productions:edit-cue" => Some(tag_attr(container, "data-cue-edited", "selection_updated")),
        "productions:cue-sequence" => Some(tag_attr(container, "data-cue-sequence", "order_saved")),
        "productions:cue-trigger" => Some(tag_attr(container, "data-cue-trigger", "manual_smpte_event")),
        "productions:cue-fade-time" | "productions:cue-fade" => Some(step_fade_time(container)),
        "productions:cue-playback" => Some(playback_next_cue(container)),
        "productions:smpte-sync" => Some(tag_attr(container, "data-smpte-sync", "locked_to_timecode")),
        "productions:midi-timecode" => Some(tag_attr(container, "data-midi-timecode", "mtc_locked")),
        "productions:osc-trigger" | "productions:osc-config" => Some(set_osc_trigger(container)),
        "productions:timeline-trigger" => Some(tag_attr(container, "data-timeline-trigger", "position_mapped")),
        "productions:power-consumption-estimate" | "productions:power-calculator" => {
            Some(estimate_power(container))
        }
        "productions:metadata-view" => Some(tag_attr(container, "data-production-metadata", "show=v1;cues=active")),
        _ => None,
    }
}

pub(crate) fn next_fade_time(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("0.0s (Cut)") => "2.0s (Fast Dissolve)",
        Some("2.0s (Fast Dissolve)") => "5.0s (Standard)",
        Some("5.0s (Standard)") => "10.0s (Slow Wash)",
        _ => "0.0s (Cut)",
    }
}

fn record_cue(container: &Element) -> Result<(), String> {
    let current_cue: u32 = container
        .get_attribute("data-cue-count")
        .and_then(|c| c.parse().ok())
        .unwrap_or(0);
    let next_cue = current_cue.saturating_add(1);
    container
        .set_attribute("data-cue-count", &next_cue.to_string())
        .map_err(|_| "Failed to record theatrical cue.".to_string())?;
    let _ = container.set_attribute("data-active-cue", &format!("Cue_{next_cue:03}"));
    Ok(())
}

fn playback_next_cue(container: &Element) -> Result<(), String> {
    let current_cue: u32 = container
        .get_attribute("data-current-playing-cue")
        .and_then(|c| c.parse().ok())
        .unwrap_or(1);
    let total_cues: u32 = container
        .get_attribute("data-cue-count")
        .and_then(|c| c.parse().ok())
        .unwrap_or(1)
        .max(1);
    let next_cue = (current_cue % total_cues) + 1;
    container
        .set_attribute("data-current-playing-cue", &next_cue.to_string())
        .map_err(|_| "Failed to step cue playback.".to_string())
}

fn step_fade_time(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-cue-fade-time");
    let next = next_fade_time(current.as_deref());
    container
        .set_attribute("data-cue-fade-time", next)
        .map_err(|_| "Failed to update cue fade time.".to_string())
}

fn set_osc_trigger(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-osc-address", "/cue/fire/1")
        .map_err(|_| "Failed to bind OSC trigger address.".to_string())
}

fn estimate_power(container: &Element) -> Result<(), String> {
    let fixtures = container
        .get_attribute("data-dmx-fixtures")
        .unwrap_or_default()
        .split(',')
        .filter(|f| !f.is_empty())
        .count()
        .max(1);
    let power_watts = fixtures * 180;
    container
        .set_attribute("data-power-estimate", &format!("{power_watts} Watts"))
        .map_err(|_| "Failed to calculate power estimate.".to_string())
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
    fn fade_times_cycle_properly() {
        assert_eq!(next_fade_time(None), "0.0s (Cut)");
        assert_eq!(next_fade_time(Some("10.0s (Slow Wash)")), "0.0s (Cut)");
    }
}
