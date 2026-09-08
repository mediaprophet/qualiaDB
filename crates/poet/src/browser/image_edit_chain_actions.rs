//! Dual-path Tool Chest actions for curated Host-bound `Image.*` ids (wave 22).
//!
//! Live image-document edit — new, layers, pixels, fill, brush, filter, opacity,
//! blend, visibility, mask, composite, selections. No Host widen: scopes must
//! already exist in `poet_host/invoke/ids.rs`. Local sketches mirror Host
//! `hypermedia` Image.* unwrap_or defaults. Distinct from ComputerVision
//! `image_chain_actions.rs`.

use serde_json::json;
use web_sys::{Document, Element};

const DEFAULT_DOC_ID: &str = "doc-1";
const DEFAULT_WIDTH: u64 = 1920;
const DEFAULT_HEIGHT: u64 = 1080;
const DEFAULT_LAYER_NAME: &str = "Layer";
const DEFAULT_FILTER: &str = "blur";
const DEFAULT_BLEND_MODE: &str = "normal";
const DEFAULT_SELECTION_ID: &str = "sel-1";
const DEFAULT_BRUSH_SIZE: f64 = 10.0;
const DEFAULT_INTENSITY: f64 = 1.0;
const DEFAULT_OPACITY: f64 = 1.0;
const DEFAULT_PIXEL_A: f64 = 255.0;
const COMPOSITE_FORMAT: &str = "rgba8";

fn selected_container(document: &Document) -> Option<Element> {
    document
        .query_selector(".canvas-container-node.selected")
        .ok()
        .flatten()
}

fn selected_source(document: &Document) -> Option<String> {
    let container = selected_container(document)?;
    let text = container
        .query_selector(".vibe-editor, .vibe-editor-textarea, .doc-editor, .sheet-grid")
        .ok()
        .flatten()
        .and_then(|editor| editor.text_content())
        .or_else(|| container.text_content())?;
    let bounded: String = text.chars().take(16_384).collect();
    (!bounded.trim().is_empty()).then_some(bounded)
}

fn parse_numbers(source: &str) -> Vec<f64> {
    source
        .split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ';' | '|' | '\n' | '\r'))
        .filter_map(|token| token.trim().parse::<f64>().ok())
        .filter(|n| n.is_finite())
        .take(64)
        .collect()
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

fn bool_attr(el: Option<&Element>, name: &str) -> Option<bool> {
    string_attr(el, name).and_then(|s| match s.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    })
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

fn resolve_image_id(container: Option<&Element>) -> String {
    local_image_id(string_attr(container, "data-image-id").as_deref())
}

fn local_image_id(from_attr: Option<&str>) -> String {
    from_attr
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(DEFAULT_DOC_ID)
        .to_string()
}

fn local_new_dims(width: Option<u64>, height: Option<u64>) -> (u64, u64) {
    (width.unwrap_or(DEFAULT_WIDTH), height.unwrap_or(DEFAULT_HEIGHT))
}

fn local_layer_name(name: Option<&str>) -> String {
    name.map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(DEFAULT_LAYER_NAME)
        .to_string()
}

fn local_remove_index(index: Option<u64>) -> u64 {
    index.unwrap_or(0)
}

fn local_pixel(
    x: Option<u64>,
    y: Option<u64>,
    r: Option<f64>,
    g: Option<f64>,
    b: Option<f64>,
    a: Option<f64>,
) -> (u64, u64, f64, f64, f64, f64) {
    (
        x.unwrap_or(0),
        y.unwrap_or(0),
        r.unwrap_or(0.0),
        g.unwrap_or(0.0),
        b.unwrap_or(0.0),
        a.unwrap_or(DEFAULT_PIXEL_A),
    )
}

fn local_fill(r: Option<f64>, g: Option<f64>, b: Option<f64>) -> (f64, f64, f64) {
    (r.unwrap_or(0.0), g.unwrap_or(0.0), b.unwrap_or(0.0))
}

fn local_brush(size: Option<f64>, points: &[f64]) -> (f64, u64) {
    (size.unwrap_or(DEFAULT_BRUSH_SIZE), (points.len() / 2) as u64)
}

fn local_filter(filter: Option<&str>, intensity: Option<f64>) -> (String, f64) {
    (
        filter
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(DEFAULT_FILTER)
            .to_string(),
        intensity.unwrap_or(DEFAULT_INTENSITY),
    )
}

fn local_opacity(opacity: Option<f64>) -> f64 {
    opacity.unwrap_or(DEFAULT_OPACITY)
}

fn local_blend_mode(mode: Option<&str>) -> String {
    mode.map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(DEFAULT_BLEND_MODE)
        .to_string()
}

fn local_visible(visible: Option<bool>) -> bool {
    visible.unwrap_or(true)
}

fn local_mask(
    x: Option<u64>,
    y: Option<u64>,
    width: Option<u64>,
    height: Option<u64>,
) -> (u64, u64, u64, u64) {
    (
        x.unwrap_or(0),
        y.unwrap_or(0),
        width.unwrap_or(0),
        height.unwrap_or(0),
    )
}

fn local_selection_id(from_attr: Option<&str>) -> String {
    from_attr
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(DEFAULT_SELECTION_ID)
        .to_string()
}

/// `Image.new` — `{ id, width?, height? }` → created document (default 1920×1080).
pub(super) fn run_new(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let id = resolve_image_id(container.as_ref());
    let width = u64_attr(container.as_ref(), "data-width")
        .or_else(|| nums.first().map(|v| v.max(0.0) as u64));
    let height = u64_attr(container.as_ref(), "data-height")
        .or_else(|| nums.get(1).map(|v| v.max(0.0) as u64));
    let (width, height) = local_new_dims(width, height);
    invoke_dual(
        document,
        label,
        "Image.new",
        format!("image new sketch id={id} {width}x{height} layer_count=0"),
        json!({ "id": id, "width": width, "height": height }),
    );
}

/// `Image.add_layer` — `{ id, name? }`.
pub(super) fn run_add_layer(document: &Document, label: &str) {
    let container = selected_container(document);
    let id = resolve_image_id(container.as_ref());
    let name = local_layer_name(
        string_attr(container.as_ref(), "data-name")
            .or_else(|| string_attr(container.as_ref(), "data-layer-name"))
            .as_deref(),
    );
    invoke_dual(
        document,
        label,
        "Image.add_layer",
        format!("image add_layer sketch id={id} name={name}"),
        json!({ "id": id, "name": name }),
    );
}

/// `Image.remove_layer` — `{ id, index? }`.
pub(super) fn run_remove_layer(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let id = resolve_image_id(container.as_ref());
    let index = local_remove_index(
        u64_attr(container.as_ref(), "data-index")
            .or_else(|| u64_attr(container.as_ref(), "data-layer-index"))
            .or_else(|| nums.first().map(|v| v.max(0.0) as u64)),
    );
    invoke_dual(
        document,
        label,
        "Image.remove_layer",
        format!("image remove_layer sketch id={id} index={index}"),
        json!({ "id": id, "index": index }),
    );
}

/// `Image.set_pixel` — `{ id, x, y, r?, g?, b?, a? }`.
pub(super) fn run_set_pixel(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let id = resolve_image_id(container.as_ref());
    let (x, y, r, g, b, a) = local_pixel(
        u64_attr(container.as_ref(), "data-x").or_else(|| nums.first().map(|v| v.max(0.0) as u64)),
        u64_attr(container.as_ref(), "data-y").or_else(|| nums.get(1).map(|v| v.max(0.0) as u64)),
        numeric_attr(container.as_ref(), "data-r").or_else(|| nums.get(2).copied()),
        numeric_attr(container.as_ref(), "data-g").or_else(|| nums.get(3).copied()),
        numeric_attr(container.as_ref(), "data-b").or_else(|| nums.get(4).copied()),
        numeric_attr(container.as_ref(), "data-a").or_else(|| nums.get(5).copied()),
    );
    invoke_dual(
        document,
        label,
        "Image.set_pixel",
        format!("image set_pixel sketch id={id} ({x},{y}) rgba=({r:.1},{g:.1},{b:.1},{a:.1})"),
        json!({ "id": id, "x": x, "y": y, "r": r, "g": g, "b": b, "a": a }),
    );
}

/// `Image.fill` — `{ id, r?, g?, b? }`.
pub(super) fn run_fill(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let id = resolve_image_id(container.as_ref());
    let (r, g, b) = local_fill(
        numeric_attr(container.as_ref(), "data-r").or_else(|| nums.first().copied()),
        numeric_attr(container.as_ref(), "data-g").or_else(|| nums.get(1).copied()),
        numeric_attr(container.as_ref(), "data-b").or_else(|| nums.get(2).copied()),
    );
    invoke_dual(
        document,
        label,
        "Image.fill",
        format!("image fill sketch id={id} rgb=({r:.1},{g:.1},{b:.1})"),
        json!({ "id": id, "r": r, "g": g, "b": b }),
    );
}

/// `Image.brush` — `{ id, size?, points? }`.
pub(super) fn run_brush(document: &Document, label: &str) {
    let container = selected_container(document);
    let points = parse_numbers(&selected_source(document).unwrap_or_default());
    let id = resolve_image_id(container.as_ref());
    let (size, point_count) =
        local_brush(numeric_attr(container.as_ref(), "data-size"), &points);
    invoke_dual(
        document,
        label,
        "Image.brush",
        format!("image brush sketch id={id} size={size:.1} point_count={point_count}"),
        json!({ "id": id, "size": size, "points": points }),
    );
}

/// `Image.apply_filter` — `{ id, filter, intensity? }` (default filter `blur`).
pub(super) fn run_apply_filter(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let id = resolve_image_id(container.as_ref());
    let (filter, intensity) = local_filter(
        string_attr(container.as_ref(), "data-filter").as_deref(),
        numeric_attr(container.as_ref(), "data-intensity").or_else(|| nums.first().copied()),
    );
    invoke_dual(
        document,
        label,
        "Image.apply_filter",
        format!("image apply_filter sketch id={id} filter={filter} intensity={intensity:.3}"),
        json!({ "id": id, "filter": filter, "intensity": intensity }),
    );
}

/// `Image.set_opacity` — `{ id, opacity }`.
pub(super) fn run_set_opacity(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let id = resolve_image_id(container.as_ref());
    let opacity = local_opacity(
        numeric_attr(container.as_ref(), "data-opacity").or_else(|| nums.first().copied()),
    );
    invoke_dual(
        document,
        label,
        "Image.set_opacity",
        format!("image set_opacity sketch id={id} opacity={opacity:.3}"),
        json!({ "id": id, "opacity": opacity }),
    );
}

/// `Image.set_blend_mode` — `{ id, blend_mode }` (default `normal`).
pub(super) fn run_set_blend_mode(document: &Document, label: &str) {
    let container = selected_container(document);
    let id = resolve_image_id(container.as_ref());
    let blend_mode = local_blend_mode(string_attr(container.as_ref(), "data-blend-mode").as_deref());
    invoke_dual(
        document,
        label,
        "Image.set_blend_mode",
        format!("image set_blend_mode sketch id={id} blend_mode={blend_mode}"),
        json!({ "id": id, "blend_mode": blend_mode }),
    );
}

/// `Image.set_visible` — `{ id, visible? }`.
pub(super) fn run_set_visible(document: &Document, label: &str) {
    let container = selected_container(document);
    let id = resolve_image_id(container.as_ref());
    let visible = local_visible(bool_attr(container.as_ref(), "data-visible"));
    invoke_dual(
        document,
        label,
        "Image.set_visible",
        format!("image set_visible sketch id={id} visible={visible}"),
        json!({ "id": id, "visible": visible }),
    );
}

/// `Image.set_mask` — `{ id, x?, y?, width?, height? }`.
pub(super) fn run_set_mask(document: &Document, label: &str) {
    let container = selected_container(document);
    let nums = parse_numbers(&selected_source(document).unwrap_or_default());
    let id = resolve_image_id(container.as_ref());
    let (x, y, width, height) = local_mask(
        u64_attr(container.as_ref(), "data-x").or_else(|| nums.first().map(|v| v.max(0.0) as u64)),
        u64_attr(container.as_ref(), "data-y").or_else(|| nums.get(1).map(|v| v.max(0.0) as u64)),
        u64_attr(container.as_ref(), "data-width")
            .or_else(|| nums.get(2).map(|v| v.max(0.0) as u64)),
        u64_attr(container.as_ref(), "data-height")
            .or_else(|| nums.get(3).map(|v| v.max(0.0) as u64)),
    );
    invoke_dual(
        document,
        label,
        "Image.set_mask",
        format!("image set_mask sketch id={id} rect=({x},{y},{width},{height})"),
        json!({ "id": id, "x": x, "y": y, "width": width, "height": height }),
    );
}

/// `Image.clear_mask` — `{ id }`.
pub(super) fn run_clear_mask(document: &Document, label: &str) {
    let id = resolve_image_id(selected_container(document).as_ref());
    invoke_dual(
        document,
        label,
        "Image.clear_mask",
        format!("image clear_mask sketch id={id} status=mask_cleared"),
        json!({ "id": id }),
    );
}

/// `Image.composite` — `{ id }`.
pub(super) fn run_composite(document: &Document, label: &str) {
    let id = resolve_image_id(selected_container(document).as_ref());
    invoke_dual(
        document,
        label,
        "Image.composite",
        format!("image composite sketch id={id} format={COMPOSITE_FORMAT}"),
        json!({ "id": id }),
    );
}

/// `Image.add_selection` — `{ id, selection_id }`.
pub(super) fn run_add_selection(document: &Document, label: &str) {
    let container = selected_container(document);
    let id = resolve_image_id(container.as_ref());
    let selection_id =
        local_selection_id(string_attr(container.as_ref(), "data-selection-id").as_deref());
    invoke_dual(
        document,
        label,
        "Image.add_selection",
        format!("image add_selection sketch id={id} selection_id={selection_id}"),
        json!({ "id": id, "selection_id": selection_id }),
    );
}

/// `Image.clear_selections` — `{ id }`.
pub(super) fn run_clear_selections(document: &Document, label: &str) {
    let id = resolve_image_id(selected_container(document).as_ref());
    invoke_dual(
        document,
        label,
        "Image.clear_selections",
        format!("image clear_selections sketch id={id} status=selections_cleared"),
        json!({ "id": id }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_image_wave22_edit_defaults() {
        assert_eq!(local_image_id(None), "doc-1");
        assert_eq!(local_image_id(Some("")), "doc-1");
        assert_eq!(local_image_id(Some("  ")), "doc-1");
        assert_eq!(local_image_id(Some("sheet-a")), "sheet-a");

        assert_eq!(local_new_dims(None, None), (1920, 1080));
        assert_eq!(local_new_dims(Some(64), Some(48)), (64, 48));

        assert_eq!(local_layer_name(None), "Layer");
        assert_eq!(local_layer_name(Some("Overlay")), "Overlay");
        assert_eq!(local_remove_index(None), 0);
        assert_eq!(local_remove_index(Some(2)), 2);

        assert_eq!(local_pixel(None, None, None, None, None, None), (0, 0, 0.0, 0.0, 0.0, 255.0));
        assert_eq!(local_fill(None, None, None), (0.0, 0.0, 0.0));

        let (size, count) = local_brush(None, &[]);
        assert!((size - 10.0).abs() < 1e-12);
        assert_eq!(count, 0);
        let (size, count) = local_brush(Some(4.0), &[0.0, 0.0, 8.0, 8.0]);
        assert!((size - 4.0).abs() < 1e-12);
        assert_eq!(count, 2);

        let (filter, intensity) = local_filter(None, None);
        assert_eq!(filter, "blur");
        assert!((intensity - 1.0).abs() < 1e-12);

        assert!((local_opacity(None) - 1.0).abs() < 1e-12);
        assert_eq!(local_blend_mode(None), "normal");
        assert!(local_visible(None));
        assert!(!local_visible(Some(false)));
        assert_eq!(local_mask(None, None, None, None), (0, 0, 0, 0));
        assert_eq!(local_selection_id(None), "sel-1");
        assert_eq!(COMPOSITE_FORMAT, "rgba8");
        assert_eq!(DEFAULT_DOC_ID, "doc-1");
    }
}
