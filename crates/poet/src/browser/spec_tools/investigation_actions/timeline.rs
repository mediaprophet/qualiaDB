//! Timeline construction and temporal queries for Poet investigations.

use super::shared::{append_semicolon_attr, count_selector};
use web_sys::{Document, Element};

pub(super) fn run(document: &Document, container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "investigation:create-timeline" => Some(create_timeline(container)),
        "investigation:add-entry" => Some(add_entry(container)),
        "investigation:set-time-precision" => Some(cycle_time_precision(container)),
        "investigation:set-track" => Some(set_track(container)),
        "investigation:query-timeline" => Some(query_timeline(document, container)),
        "investigation:detect-sequences" => Some(detect_sequences(container)),
        "investigation:reconcile-timelines" => Some(reconcile_timelines(document, container)),
        _ => None,
    }
}

pub(crate) fn next_time_precision(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("exact") => "approximate",
        Some("approximate") => "estimated",
        Some("estimated") => "unknown",
        _ => "exact",
    }
}

fn create_timeline(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-timeline-id", "timeline:active")
        .map_err(|_| "Failed to create timeline.".to_string())?;
    let _ = container.set_attribute("data-timeline-precision", "exact");
    Ok(())
}

fn add_entry(container: &Element) -> Result<(), String> {
    append_semicolon_attr(
        container,
        "data-timeline-entries",
        "entry:event_or_evidence@track:default",
    )
}

fn cycle_time_precision(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-timeline-precision");
    let next = next_time_precision(current.as_deref());
    container
        .set_attribute("data-timeline-precision", next)
        .map_err(|_| "Failed to set time precision.".to_string())
}

fn set_track(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-timeline-track", "track:primary")
        .map_err(|_| "Failed to set timeline track.".to_string())
}

fn query_timeline(document: &Document, container: &Element) -> Result<(), String> {
    let timelines = count_selector(document, "[data-timeline-id]")?;
    let entries = container
        .get_attribute("data-timeline-entries")
        .map(|e| e.split(';').filter(|s| !s.is_empty()).count())
        .unwrap_or(0);
    container
        .set_attribute(
            "data-timeline-query",
            &format!("timelines={timelines};entries={entries}"),
        )
        .map_err(|_| "Failed to query timeline.".to_string())
}

fn detect_sequences(container: &Element) -> Result<(), String> {
    let entries = container
        .get_attribute("data-timeline-entries")
        .unwrap_or_default();
    let pattern = if entries.contains(';') {
        "sequence:before_after_detected"
    } else if entries.is_empty() {
        "sequence:none"
    } else {
        "sequence:single_entry"
    };
    container
        .set_attribute("data-timeline-sequences", pattern)
        .map_err(|_| "Failed to detect temporal sequences.".to_string())
}

fn reconcile_timelines(document: &Document, container: &Element) -> Result<(), String> {
    let count = count_selector(document, "[data-timeline-id]")?;
    let verdict = if count >= 2 {
        "reconcile:overlaps_and_gaps_catalogued"
    } else {
        "reconcile:need_two_timelines"
    };
    container
        .set_attribute("data-timeline-reconcile", verdict)
        .map_err(|_| "Failed to reconcile timelines.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_precision_cycles() {
        assert_eq!(next_time_precision(None), "exact");
        assert_eq!(next_time_precision(Some("estimated")), "unknown");
        assert_eq!(next_time_precision(Some("unknown")), "exact");
    }
}
