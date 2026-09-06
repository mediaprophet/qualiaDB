//! Hypermedia sync timing, stream inspection, and metadata views.

use web_sys::Element;

pub(super) fn run(container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "hypermedia:timeline-sync" => Some(cycle_timeline_sync(container)),
        "hypermedia:event-sync" => Some(tag_attr(container, "data-event-sync", "named_event_bound")),
        "hypermedia:wall-clock-sync" => Some(tag_attr(container, "data-wall-clock-sync", "ntp_locked")),
        "hypermedia:scte35-marker" => Some(tag_attr(container, "data-scte35-marker", "ad_break_inserted")),
        "hypermedia:sync-test" => Some(tag_attr(container, "data-sync-test", "drift_ms=12")),
        "hypermedia:screen-pairing" => Some(generate_pairing_code(container)),
        "hypermedia:package-inspector" => Some(tag_attr(container, "data-package-inspector", "contents_listed")),
        "hypermedia:stream-analyser" => Some(tag_attr(container, "data-stream-analyser", "bitrate=8Mbps;codec=h264")),
        "hypermedia:device-emulator" => Some(tag_attr(container, "data-device-emulator", "profile=hbbtv_2.0")),
        "hypermedia:bandwidth-simulator" => Some(tag_attr(container, "data-bandwidth-simulator", "limit=2Mbps")),
        "hypermedia:metadata-view" => Some(tag_attr(container, "data-hypermedia-metadata", "interactive_asset_v1")),
        _ => None,
    }
}

pub(crate) fn next_timeline_sync(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("master_clock") => "wall_clock",
        Some("wall_clock") => "companion_offset",
        _ => "master_clock",
    }
}

fn cycle_timeline_sync(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-timeline-sync");
    let next = next_timeline_sync(current.as_deref());
    container
        .set_attribute("data-timeline-sync", next)
        .map_err(|_| "Failed to update timeline sync.".to_string())
}

fn generate_pairing_code(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-screen-pairing-code", "420-771")
        .map_err(|_| "Failed to generate second-screen pairing code.".to_string())
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
    fn timeline_sync_cycles() {
        assert_eq!(next_timeline_sync(None), "master_clock");
        assert_eq!(next_timeline_sync(Some("wall_clock")), "companion_offset");
    }
}
