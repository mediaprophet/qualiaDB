//! Orphan-only DMX desk handlers. These IDs are not aliased to Gated row IDs
//! (`productions:add-universe`, `productions:patch-fixture`, etc.) so the desk
//! tools remain honestly gated in the tool chest.

use web_sys::Element;

pub(super) fn run(container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "productions:dmx-universe" => Some(step_universe(container)),
        "productions:fixture-add" => Some(add_fixture(container)),
        "productions:fixture-patch" => Some(patch_fixture(container)),
        "productions:emergency-blackout" => Some(toggle_blackout(container)),
        "productions:channel-fader" => Some(set_fader(container)),
        "productions:rgb-color-picker" => Some(set_rgb_colour(container)),
        "productions:moving-head-pan-tilt" => Some(set_pan_tilt(container)),
        _ => None,
    }
}

pub(crate) fn next_dmx_universe(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("Universe 1 (Main Stage)") => "Universe 2 (Balcony & Truss)",
        Some("Universe 2 (Balcony & Truss)") => "Universe 3 (Auditorium)",
        Some("Universe 3 (Auditorium)") => "Universe 4 (Effects & Lasers)",
        _ => "Universe 1 (Main Stage)",
    }
}

fn step_universe(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-dmx-universe");
    let next = next_dmx_universe(current.as_deref());
    container
        .set_attribute("data-dmx-universe", next)
        .map_err(|_| "Failed to select DMX universe.".to_string())
}

fn add_fixture(container: &Element) -> Result<(), String> {
    let current = container
        .get_attribute("data-dmx-fixtures")
        .unwrap_or_default();
    let entry = "LED_Par_RGBWA";
    let updated = if current.is_empty() {
        entry.to_string()
    } else {
        format!("{current},{entry}")
    };
    container
        .set_attribute("data-dmx-fixtures", &updated)
        .map_err(|_| "Failed to add DMX fixture.".to_string())
}

fn patch_fixture(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-dmx-address", "CH_001_DMX512")
        .map_err(|_| "Failed to patch DMX address.".to_string())
}

fn toggle_blackout(container: &Element) -> Result<(), String> {
    let current = container
        .get_attribute("data-dmx-blackout")
        .is_some_and(|v| v == "active");
    let next = if current { "standby" } else { "active" };
    container
        .set_attribute("data-dmx-blackout", next)
        .map_err(|_| "Failed to toggle emergency blackout.".to_string())
}

fn set_fader(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-dmx-master-fader", "255 (100%)")
        .map_err(|_| "Failed to set DMX fader level.".to_string())
}

fn set_rgb_colour(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-dmx-rgb", "R:255 G:180 B:60")
        .map_err(|_| "Failed to set fixture RGB color.".to_string())
}

fn set_pan_tilt(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-dmx-pan-tilt", "Pan:180° Tilt:45°")
        .map_err(|_| "Failed to set moving head coordinates.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dmx_universes_cycle_through_stages() {
        assert_eq!(next_dmx_universe(None), "Universe 1 (Main Stage)");
        assert_eq!(
            next_dmx_universe(Some("Universe 4 (Effects & Lasers)")),
            "Universe 1 (Main Stage)"
        );
    }
}
