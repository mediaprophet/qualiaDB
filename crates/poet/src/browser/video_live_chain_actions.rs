//! Dual-path Tool Chest actions for curated Host-bound `Video.*` ids (wave 23).
//!
//! Distinct from spec_tools hyphenated `video:*` DOM edits. Live ids are
//! `video:live_*` and bind exact Host scopes already in ALL_BOUND.
//! Local sketches mirror Host `hypermedia` Video.* unwrap_or defaults.

use serde_json::json;
use web_sys::{Document, Element};

const DEFAULT_PROJECT_ID: &str = "proj-1";
const DEFAULT_PROJECT_NAME: &str = "Untitled";
const DEFAULT_WIDTH: u64 = 1920;
const DEFAULT_HEIGHT: u64 = 1080;
const DEFAULT_FPS: f64 = 30.0;
const DEFAULT_TRACK_NAME: &str = "Track";
const DEFAULT_SOURCE: &str = "clip-1";
const DEFAULT_DURATION: f64 = 10.0;
const DEFAULT_TRANSITION: &str = "cross_dissolve";
const DEFAULT_TRANSITION_DURATION: f64 = 1.0;
const DEFAULT_CLIP_ID: &str = "clip-1";
const DEFAULT_FORMAT: &str = "h264";
const DEFAULT_BITRATE: u64 = 8_000_000;
const DEFAULT_SPEED: f64 = 1.0;

fn selected_container(document: &Document) -> Option<Element> {
    document
        .query_selector(".canvas-container-node.selected")
        .ok()
        .flatten()
}

fn numeric_attr(el: Option<&Element>, name: &str) -> Option<f64> {
    el.and_then(|e| e.get_attribute(name))
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|v| v.is_finite())
}

fn u64_attr(el: Option<&Element>, name: &str) -> Option<u64> {
    numeric_attr(el, name).map(|v| v.max(0.0) as u64)
}

fn string_attr(el: Option<&Element>, name: &str) -> Option<String> {
    el.and_then(|e| e.get_attribute(name))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn invoke_dual(
    document: &Document,
    label: &str,
    cap_id: &'static str,
    local_message: String,
    args: serde_json::Value,
) {
    let label = label.to_string();
    if !super::native_daemon::is_daemon_connected() {
        let report = super::tool_dual_path::local_sketch(cap_id, &local_message);
        super::interactions::show_tool_status(
            document,
            &label,
            &report.message,
            report.status_kind,
        );
        return;
    }
    super::interactions::show_tool_status(
        document,
        &label,
        &format!("Running {cap_id}…"),
        "running",
    );
    wasm_bindgen_futures::spawn_local(async move {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        match super::native_daemon::daemon_invoke(cap_id, args).await {
            Ok(response) if response.ok => {
                let report = super::tool_dual_path::live_ok(cap_id, &response.value);
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Ok(response) => {
                let report = super::tool_dual_path::live_denied(
                    cap_id,
                    response
                        .diagnostic
                        .as_deref()
                        .unwrap_or("capability invoke failed."),
                );
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Err(error) => {
                let report = super::tool_dual_path::live_denied(cap_id, &error);
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
        }
    });
}

fn local_id(from_attr: Option<&str>, default: &str) -> String {
    from_attr
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(default)
        .to_string()
}

fn local_project_dims(width: Option<u64>, height: Option<u64>, fps: Option<f64>) -> (u64, u64, f64) {
    (
        width.unwrap_or(DEFAULT_WIDTH),
        height.unwrap_or(DEFAULT_HEIGHT),
        fps.filter(|v| *v > 0.0).unwrap_or(DEFAULT_FPS),
    )
}

fn local_trim(in_point: Option<f64>, out_point: Option<f64>) -> (f64, f64) {
    (in_point.unwrap_or(0.0), out_point.unwrap_or(0.0))
}

fn local_grade(brightness: Option<f64>, contrast: Option<f64>, saturation: Option<f64>) -> (f64, f64, f64) {
    (
        brightness.unwrap_or(0.0),
        contrast.unwrap_or(0.0),
        saturation.unwrap_or(0.0),
    )
}

fn local_transition(kind: Option<&str>, duration: Option<f64>) -> (String, f64) {
    (
        local_id(kind, DEFAULT_TRANSITION),
        duration.filter(|v| *v >= 0.0).unwrap_or(DEFAULT_TRANSITION_DURATION),
    )
}

fn resolve_project_id(container: Option<&Element>) -> String {
    local_id(
        string_attr(container, "data-video-id")
            .or_else(|| string_attr(container, "data-project-id"))
            .as_deref(),
        DEFAULT_PROJECT_ID,
    )
}

/// `Video.new_project` — `{ id, name?, width?, height?, fps? }`.
pub(super) fn run_new_project(document: &Document, label: &str) {
    let container = selected_container(document);
    let id = resolve_project_id(container.as_ref());
    let name = local_id(
        string_attr(container.as_ref(), "data-name").as_deref(),
        DEFAULT_PROJECT_NAME,
    );
    let (width, height, fps) = local_project_dims(
        u64_attr(container.as_ref(), "data-width"),
        u64_attr(container.as_ref(), "data-height"),
        numeric_attr(container.as_ref(), "data-fps"),
    );
    invoke_dual(
        document,
        label,
        "Video.new_project",
        format!("video new_project sketch id={id} name={name} {width}x{height}@{fps}"),
        json!({ "id": id, "name": name, "width": width, "height": height, "fps": fps }),
    );
}

/// `Video.add_track` — `{ id, name? }`.
pub(super) fn run_add_track(document: &Document, label: &str) {
    let container = selected_container(document);
    let id = resolve_project_id(container.as_ref());
    let name = local_id(
        string_attr(container.as_ref(), "data-track-name").as_deref(),
        DEFAULT_TRACK_NAME,
    );
    invoke_dual(
        document,
        label,
        "Video.add_track",
        format!("video add_track sketch id={id} name={name}"),
        json!({ "id": id, "name": name }),
    );
}

/// `Video.add_clip` — `{ id, source, duration? }`.
pub(super) fn run_add_clip(document: &Document, label: &str) {
    let container = selected_container(document);
    let id = resolve_project_id(container.as_ref());
    let source = local_id(
        string_attr(container.as_ref(), "data-source").as_deref(),
        DEFAULT_SOURCE,
    );
    let duration = numeric_attr(container.as_ref(), "data-duration").unwrap_or(DEFAULT_DURATION);
    invoke_dual(
        document,
        label,
        "Video.add_clip",
        format!("video add_clip sketch id={id} source={source} duration={duration}"),
        json!({ "id": id, "source": source, "duration": duration }),
    );
}

/// `Video.trim_clip` — `{ id, in_point?, out_point? }`.
pub(super) fn run_trim_clip(document: &Document, label: &str) {
    let container = selected_container(document);
    let id = resolve_project_id(container.as_ref());
    let (in_point, out_point) = local_trim(
        numeric_attr(container.as_ref(), "data-in-point"),
        numeric_attr(container.as_ref(), "data-out-point"),
    );
    invoke_dual(
        document,
        label,
        "Video.trim_clip",
        format!("video trim_clip sketch id={id} in={in_point} out={out_point}"),
        json!({ "id": id, "in_point": in_point, "out_point": out_point }),
    );
}

/// `Video.set_speed` — `{ id, speed }`.
pub(super) fn run_set_speed(document: &Document, label: &str) {
    let container = selected_container(document);
    let id = resolve_project_id(container.as_ref());
    let speed = numeric_attr(container.as_ref(), "data-speed").unwrap_or(DEFAULT_SPEED);
    invoke_dual(
        document,
        label,
        "Video.set_speed",
        format!("video set_speed sketch id={id} speed={speed}"),
        json!({ "id": id, "speed": speed }),
    );
}

/// `Video.colour_grade` — `{ id, brightness?, contrast?, saturation? }`.
pub(super) fn run_colour_grade(document: &Document, label: &str) {
    let container = selected_container(document);
    let id = resolve_project_id(container.as_ref());
    let (brightness, contrast, saturation) = local_grade(
        numeric_attr(container.as_ref(), "data-brightness"),
        numeric_attr(container.as_ref(), "data-contrast"),
        numeric_attr(container.as_ref(), "data-saturation"),
    );
    invoke_dual(
        document,
        label,
        "Video.colour_grade",
        format!("video colour_grade sketch id={id} b={brightness} c={contrast} s={saturation}"),
        json!({
            "id": id,
            "brightness": brightness,
            "contrast": contrast,
            "saturation": saturation
        }),
    );
}

/// `Video.add_transition` — `{ id, transition_type?, duration? }`.
pub(super) fn run_add_transition(document: &Document, label: &str) {
    let container = selected_container(document);
    let id = resolve_project_id(container.as_ref());
    let (transition_type, duration) = local_transition(
        string_attr(container.as_ref(), "data-transition-type").as_deref(),
        numeric_attr(container.as_ref(), "data-duration"),
    );
    invoke_dual(
        document,
        label,
        "Video.add_transition",
        format!("video add_transition sketch id={id} type={transition_type} duration={duration}"),
        json!({ "id": id, "transition_type": transition_type, "duration": duration }),
    );
}

/// `Video.set_render_format` — `{ id, format }`.
pub(super) fn run_set_render_format(document: &Document, label: &str) {
    let container = selected_container(document);
    let id = resolve_project_id(container.as_ref());
    let format = local_id(
        string_attr(container.as_ref(), "data-format").as_deref(),
        DEFAULT_FORMAT,
    );
    invoke_dual(
        document,
        label,
        "Video.set_render_format",
        format!("video set_render_format sketch id={id} format={format}"),
        json!({ "id": id, "format": format }),
    );
}

/// `Video.set_render_bitrate` — `{ id, bitrate }`.
pub(super) fn run_set_render_bitrate(document: &Document, label: &str) {
    let container = selected_container(document);
    let id = resolve_project_id(container.as_ref());
    let bitrate = u64_attr(container.as_ref(), "data-bitrate").unwrap_or(DEFAULT_BITRATE);
    invoke_dual(
        document,
        label,
        "Video.set_render_bitrate",
        format!("video set_render_bitrate sketch id={id} bitrate={bitrate}"),
        json!({ "id": id, "bitrate": bitrate }),
    );
}

/// `Video.remove_clip` — `{ id, clip_id }`.
pub(super) fn run_remove_clip(document: &Document, label: &str) {
    let container = selected_container(document);
    let id = resolve_project_id(container.as_ref());
    let clip_id = local_id(
        string_attr(container.as_ref(), "data-clip-id").as_deref(),
        DEFAULT_CLIP_ID,
    );
    invoke_dual(
        document,
        label,
        "Video.remove_clip",
        format!("video remove_clip sketch id={id} clip_id={clip_id}"),
        json!({ "id": id, "clip_id": clip_id }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_video_wave23_defaults() {
        assert_eq!(local_id(None, "proj-1"), "proj-1");
        assert_eq!(local_id(Some("  cut  "), "proj-1"), "cut");
        assert_eq!(local_project_dims(None, None, None), (1920, 1080, 30.0));
        assert_eq!(local_project_dims(Some(640), Some(360), Some(24.0)), (640, 360, 24.0));
        assert_eq!(local_trim(None, None), (0.0, 0.0));
        assert_eq!(local_grade(None, None, None), (0.0, 0.0, 0.0));
        let (kind, dur) = local_transition(None, None);
        assert_eq!(kind, "cross_dissolve");
        assert!((dur - 1.0).abs() < 1e-12);
    }
}
