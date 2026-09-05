//! Video transitions, aspect ratios, timecode, and metadata inspection.

use web_sys::Element;

pub(super) fn run(container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "video:cross-dissolve" => Some(set_transition(container, "cross-dissolve")),
        "video:fade-to-black" => Some(set_transition(container, "fade-to-black")),
        "video:wipe" => Some(set_transition(container, "wipe")),
        "video:dip-to-color" => Some(set_transition(container, "dip-to-color")),
        "video:timecode-display" => Some(toggle_timecode(container)),
        "video:aspect-ratio" => Some(cycle_aspect_ratio(container)),
        "video:clip-metadata" => Some(show_metadata(container)),
        "video:codec-info" => Some(show_codec_info(container)),
        "video:render-preview" => Some(render_preview(container)),
        "video:render-export" => Some(render_export(container)),
        _ => None,
    }
}

pub(crate) fn next_aspect_ratio(current: Option<&str>) -> &'static str {
    match current.map(str::trim) {
        Some("16:9") => "9:16",
        Some("9:16") => "4:3",
        Some("4:3") => "1:1",
        Some("1:1") => "21:9",
        _ => "16:9",
    }
}

fn set_transition(container: &Element, kind: &str) -> Result<(), String> {
    container
        .set_attribute("data-video-transition", kind)
        .map_err(|_| format!("Failed to apply {kind} transition."))
}

fn toggle_timecode(container: &Element) -> Result<(), String> {
    let current = container
        .get_attribute("data-timecode-display")
        .is_some_and(|v| v == "true");
    let next = if current { "false" } else { "true" };
    container
        .set_attribute("data-timecode-display", next)
        .map_err(|_| "Failed to toggle timecode display.".to_string())
}

fn cycle_aspect_ratio(container: &Element) -> Result<(), String> {
    let current = container.get_attribute("data-video-aspect-ratio");
    let next = next_aspect_ratio(current.as_deref());
    container
        .set_attribute("data-video-aspect-ratio", next)
        .map_err(|_| "Failed to update aspect ratio.".to_string())
}

fn show_metadata(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-clip-metadata", "res=1920x1080;fps=24;dur=00:01:30")
        .map_err(|_| "Failed to inspect clip metadata.".to_string())
}

fn show_codec_info(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-codec-info", "codec=avc1.640028;container=mp4")
        .map_err(|_| "Failed to inspect codec info.".to_string())
}

fn render_preview(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-render-preview", "rendered:cached_frame_buffer")
        .map_err(|_| "Failed to render preview frame.".to_string())
}

fn render_export(container: &Element) -> Result<(), String> {
    container
        .set_attribute("data-render-export", "export_job_enqueued")
        .map_err(|_| "Failed to enqueue render export.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aspect_ratios_cycle_cleanly() {
        assert_eq!(next_aspect_ratio(None), "16:9");
        assert_eq!(next_aspect_ratio(Some("16:9")), "9:16");
        assert_eq!(next_aspect_ratio(Some("9:16")), "4:3");
        assert_eq!(next_aspect_ratio(Some("4:3")), "1:1");
        assert_eq!(next_aspect_ratio(Some("1:1")), "21:9");
        assert_eq!(next_aspect_ratio(Some("21:9")), "16:9");
    }
}
