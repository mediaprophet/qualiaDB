//! Video timeline markers, playback speed, and clip editing on Poet media containers.

use wasm_bindgen::JsCast;
use web_sys::{Element, HtmlMediaElement};

pub(super) fn run(container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "video:set-in-point" => Some(set_in_point(container)),
        "video:set-out-point" => Some(set_out_point(container)),
        "video:set-speed" => Some(cycle_speed(container)),
        "video:split-clip" => Some(split_clip(container)),
        "video:ripple-delete" => Some(ripple_delete(container)),
        "video:time-stretch" => Some(time_stretch(container)),
        "video:slip-edit" => Some(slip_edit(container)),
        "video:slide-edit" => Some(slide_edit(container)),
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

fn slip_edit(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-edit-mode", "slip")
        .map_err(|_| "Failed to perform slip edit.".to_string())
}

fn slide_edit(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-edit-mode", "slide")
        .map_err(|_| "Failed to perform slide edit.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speed_cycling_progresses_through_all_rates() {
        assert_eq!(next_video_speed(None), (1.0, "1.0x"));
        assert_eq!(next_video_speed(Some("1.0x")), (1.25, "1.25x"));
        assert_eq!(next_video_speed(Some("1.25x")), (1.5, "1.5x"));
        assert_eq!(next_video_speed(Some("1.5x")), (2.0, "2.0x"));
        assert_eq!(next_video_speed(Some("2.0x")), (0.5, "0.5x"));
        assert_eq!(next_video_speed(Some("0.5x")), (1.0, "1.0x"));
    }
}
