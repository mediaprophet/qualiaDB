//! Video timeline markers, playback speed, and clip editing on Poet media containers.

use wasm_bindgen::JsCast;
use web_sys::{Element, HtmlMediaElement};

pub(super) fn run(container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "video:mark-in" | "video:set-in-point" => Some(set_in_point(container)),
        "video:mark-out" | "video:set-out-point" => Some(set_out_point(container)),
        "video:set-speed" => Some(cycle_speed(container)),
        "video:shuttle" => Some(cycle_shuttle(container)),
        "video:razor" | "video:split-clip" => Some(split_clip(container)),
        "video:splice" => Some(tag_edit(container, "data-clip-spliced", "joined")),
        "video:overwrite" => Some(tag_edit(container, "data-edit-mode", "overwrite")),
        "video:insert" => Some(tag_edit(container, "data-edit-mode", "insert")),
        "video:ripple-delete" => Some(ripple_delete(container)),
        "video:lift" => Some(tag_edit(container, "data-clip-lifted", "gap_left")),
        "video:append" => Some(tag_edit(container, "data-clip-appended", "track_end")),
        "video:match-frame" => Some(tag_edit(container, "data-match-frame", "source_bin_synced")),
        "video:replace" => Some(tag_edit(container, "data-clip-replaced", "swap_active")),
        "video:time-stretch" => Some(time_stretch(container)),
        "video:slip-edit" => Some(tag_edit(container, "data-edit-mode", "slip")),
        "video:slide-edit" => Some(tag_edit(container, "data-edit-mode", "slide")),
        _ => None,
    }
}

pub(crate) fn next_video_speed(current: Option<&str>) -> (f64, &'static str) {
    match current.map(str::trim) {
        Some("1.0x") => (1.25, "1.25x"),
        Some("1.25x") => (1.5, "1.5x"),
        Some("1.5x") => (2.0, "2.0x"),
        Some("2.0x") => (0.5, "0.5x"),
        _ => (1.0, "1.0x"),
    }
}

pub(crate) fn next_shuttle(current: Option<&str>) -> (&'static str, f64) {
    match current.map(str::trim) {
        Some("2x forward") => ("4x forward", 4.0),
        Some("4x forward") => ("2x reverse", -2.0),
        Some("2x reverse") => ("1x forward", 1.0),
        _ => ("2x forward", 2.0),
    }
}

fn find_media(container: &Element) -> Option<HtmlMediaElement> {
    container
        .query_selector("video, audio")
        .ok()
        .flatten()
        .and_then(|el| el.dyn_into::<HtmlMediaElement>().ok())
}

fn set_in_point(container: &Element) -> Result<(), String> {
    let current_time = find_media(container).map(|m| m.current_time()).unwrap_or(0.0);
    container
        .set_attribute("data-video-in-point", &format!("{current_time:.2}s"))
        .map_err(|_| "Failed to set video in-point.".to_string())
}

fn set_out_point(container: &Element) -> Result<(), String> {
    let current_time = find_media(container).map(|m| m.current_time()).unwrap_or(0.0);
    container
        .set_attribute("data-video-out-point", &format!("{current_time:.2}s"))
        .map_err(|_| "Failed to set video out-point.".to_string())
}

fn cycle_speed(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-playback-speed");
    let (rate, label) = next_video_speed(current.as_deref());
    if let Some(media) = find_media(container) {
        media.set_playback_rate(rate);
    }
    container
        .set_attribute("data-playback-speed", label)
        .map_err(|_| "Failed to update playback speed.".to_string())
}

fn cycle_shuttle(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-shuttle-speed");
    let (label, rate) = next_shuttle(current.as_deref());
    if let Some(media) = find_media(container) {
        media.set_playback_rate(rate);
    }
    container
        .set_attribute("data-shuttle-speed", label)
        .map_err(|_| "Failed to update shuttle speed.".to_string())
}

fn split_clip(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-clip-split", "split_at_playhead")
        .map_err(|_| "Failed to split clip.".to_string())
}

fn ripple_delete(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-clip-ripple-deleted", "true")
        .map_err(|_| "Failed to perform ripple delete.".to_string())
}

fn time_stretch(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-clip-time-stretched", "120%")
        .map_err(|_| "Failed to time stretch clip.".to_string())
}

fn tag_edit(container: &Element, key: &str, value: &str) -> Result<(), String> {
    container
        .set_attribute(key, value)
        .map_err(|_| format!("Failed to apply edit ({key})."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speed_cycling_progresses_through_all_rates() {
        assert_eq!(next_video_speed(None), (1.0, "1.0x"));
        assert_eq!(next_video_speed(Some("1.0x")), (1.25, "1.25x"));
        assert_eq!(next_video_speed(Some("2.0x")), (0.5, "0.5x"));
    }

    #[test]
    fn shuttle_cycles_forward_and_reverse() {
        assert_eq!(next_shuttle(None), ("2x forward", 2.0));
        assert_eq!(next_shuttle(Some("2x forward")), ("4x forward", 4.0));
        assert_eq!(next_shuttle(Some("4x forward")), ("2x reverse", -2.0));
    }
}
