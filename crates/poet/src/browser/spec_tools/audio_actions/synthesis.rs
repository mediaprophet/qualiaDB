//! Synthesiser configuration attrs on Poet media containers.

use web_sys::Element;

pub(super) fn run(container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "audio:osc-config" => Some(set_config(
            container,
            "data-osc-config",
            "wave=saw;pitch=A4;detune=+4ct",
        )),
        "audio:filter-config" => Some(cycle_filter(container)),
        "audio:lfo-config" => Some(set_config(
            container,
            "data-lfo-config",
            "rate=0.5Hz;depth=0.3;target=filter_cutoff",
        )),
        "audio:mod-routing" => Some(set_config(
            container,
            "data-mod-routing",
            "source=lfo1;target=osc_pitch;amount=0.15",
        )),
        "audio:preset-save" => Some(save_preset(container)),
        "audio:preset-load" | "audio:wavetable-import" => Some(Err(
            "Loading external sound or wavetable files is not available in this browser preview."
                .to_string(),
        )),
        _ => None,
    }
}

fn set_config(container: &Element, attr: &str, value: &str) -> Result<(), String> {
    container
        .set_attribute(attr, value)
        .map_err(|_| format!("Failed to update {attr}."))
}

pub(crate) fn next_filter_type(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("lowpass") => "highpass",
        Some("highpass") => "bandpass",
        Some("bandpass") => "notch",
        _ => "lowpass",
    }
}

fn cycle_filter(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-filter-config");
    let kind = current
        .as_deref()
        .and_then(|v| v.split(';').next())
        .and_then(|part| part.strip_prefix("type="))
        .map(str::trim);
    let next_kind = next_filter_type(kind);
    let value = format!("type={next_kind};cutoff=1200Hz;resonance=0.4");
    container
        .set_attribute("data-filter-config", &value)
        .map_err(|_| "Failed to update filter.".to_string())
}

fn save_preset(container: &Element) -> Result<(), String> {
    let count: u32 = container
        .get_attribute("data-preset-count")
        .and_then(|c| c.parse().ok())
        .unwrap_or(0u32)
        .saturating_add(1);
    container
        .set_attribute("data-preset-count", &count.to_string())
        .map_err(|_| "Failed to save preset.".to_string())?;
    container
        .set_attribute("data-preset-last-saved", &format!("preset_{count:03}"))
        .map_err(|_| "Failed to record saved preset.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_types_cycle() {
        assert_eq!(next_filter_type(None), "lowpass");
        assert_eq!(next_filter_type(Some("notch")), "lowpass");
    }
}
