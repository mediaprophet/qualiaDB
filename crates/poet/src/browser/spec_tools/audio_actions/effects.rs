//! Effect chain and processor configuration attrs on Poet media containers.

use web_sys::Element;

pub(super) fn run(container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "audio:add-effect" => Some(append_effect(container)),
        "audio:reorder-effect" => Some(set_marker(
            container,
            "data-effect-order",
            "reordered",
        )),
        "audio:remove-effect" => Some(set_marker(
            container,
            "data-effect-removed",
            "last_in_chain",
        )),
        "audio:reverb-config" => Some(set_config(
            container,
            "data-reverb-config",
            "room=medium;damp=0.4;mix=0.25",
        )),
        "audio:delay-config" => Some(set_config(
            container,
            "data-delay-config",
            "time=250ms;feedback=0.35;mix=0.2",
        )),
        "audio:compressor-config" => Some(set_config(
            container,
            "data-compressor-config",
            "threshold=-18dB;ratio=4:1;attack=10ms;release=120ms",
        )),
        "audio:eq-config" => Some(cycle_eq_config(container)),
        "audio:saturation-config" => Some(set_config(
            container,
            "data-saturation-config",
            "drive=0.3;mix=0.15;character=tape",
        )),
        "audio:bounce-track" | "audio:bounce-mix" => Some(Err(
            "Bouncing to a sound file needs a save location and is not available here."
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

fn set_config(container: &Element, attr: &str, value: &str) -> Result<(), String> {
    container
        .set_attribute(attr, value)
        .map_err(|_| format!("Failed to update {attr}."))
}

fn append_effect(container: &Element) -> Result<(), String> {
    let chain = container
        .get_attribute("data-effect-chain")
        .unwrap_or_default();
    let next = if chain.is_empty() {
        "reverb".to_string()
    } else {
        format!("{chain},delay")
    };
    container
        .set_attribute("data-effect-chain", &next)
        .map_err(|_| "Failed to add effect.".to_string())
}

pub(crate) fn next_eq_preset(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("flat") => "low_shelf_boost",
        Some("low_shelf_boost") => "presence",
        Some("presence") => "air",
        _ => "flat",
    }
}

fn cycle_eq_config(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-eq-config");
    let preset = next_eq_preset(current.as_deref());
    let value = match preset {
        "flat" => "bands=3;low=0dB;mid=0dB;high=0dB",
        "low_shelf_boost" => "bands=3;low=+3dB;mid=0dB;high=0dB",
        "presence" => "bands=3;low=0dB;mid=+2dB;high=+1dB",
        _ => "bands=3;low=0dB;mid=0dB;high=+4dB",
    };
    container
        .set_attribute("data-eq-config", value)
        .map_err(|_| "Failed to update equaliser.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eq_presets_cycle() {
        assert_eq!(next_eq_preset(None), "flat");
        assert_eq!(next_eq_preset(Some("air")), "flat");
    }
}
