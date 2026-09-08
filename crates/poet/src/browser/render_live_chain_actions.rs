//! Dual-path Tool Chest actions for curated Host-bound `Render.*` ids (wave 30).
//!
//! CPU CSS/SVG/animation/scene binds. `animation_compute_pass` sketches GPU
//! WGSL generation honestly. Spec tools already occupy `Render.scene` Live
//! contracts — Tool Chest ids use `render:live_*`.

use serde_json::json;
use web_sys::{Document, Element};

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
    let bounded: String = text.chars().take(8_192).collect();
    (!bounded.trim().is_empty()).then_some(bounded)
}

fn string_attr(el: Option<&Element>, name: &str) -> Option<String> {
    el.and_then(|e| e.get_attribute(name))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn numeric_attr(el: Option<&Element>, name: &str) -> Option<f64> {
    el.and_then(|e| e.get_attribute(name))
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|v| v.is_finite())
}

fn parse_f64s(source: &str) -> Vec<f64> {
    source
        .split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ';' | '|'))
        .filter_map(|t| t.parse::<f64>().ok())
        .filter(|n| n.is_finite())
        .take(64)
        .collect()
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
        super::interactions::show_tool_status(document, &label, &report.message, report.status_kind);
        return;
    }
    super::interactions::show_tool_status(document, &label, &format!("Running {cap_id}…"), "running");
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

fn nums(document: &Document) -> Vec<f64> {
    parse_f64s(&selected_source(document).unwrap_or_default())
}

pub(super) fn run_scene(document: &Document, label: &str) {
    let kind = string_attr(selected_container(document).as_ref(), "data-scene-kind")
        .unwrap_or_else(|| "research".into());
    invoke_dual(
        document,
        label,
        "Render.scene",
        format!("scene sketch kind={kind}"),
        json!({ "kind": kind }),
    );
}

pub(super) fn run_css_animation(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Render.css_animation",
        "css_animation sketch opacity 0→1".into(),
        json!({
            "name": "poet-fade",
            "property": "opacity",
            "keyframes": [{ "time": 0.0, "value": 0.0 }, { "time": 1.0, "value": 1.0 }],
        }),
    );
}

pub(super) fn run_css_color(document: &Document, label: &str) {
    let n = nums(document);
    let alpha = n.first().copied().unwrap_or(1.0);
    let mu = n.get(1).copied().unwrap_or(0.5);
    let sigma = n.get(2).copied().unwrap_or(0.2);
    invoke_dual(
        document,
        label,
        "Render.css_color",
        format!("css_color sketch α={alpha} μ={mu} σ={sigma}"),
        json!({ "alpha": alpha, "mu": mu, "sigma": sigma }),
    );
}

pub(super) fn run_css_transform(document: &Document, label: &str) {
    let n = nums(document);
    let tx = n.first().copied().unwrap_or(8.0);
    let ty = n.get(1).copied().unwrap_or(0.0);
    let rotate = n.get(2).copied().unwrap_or(0.0);
    invoke_dual(
        document,
        label,
        "Render.css_transform",
        format!("css_transform sketch translate=({tx},{ty}) rotate={rotate}"),
        json!({ "translate": [tx, ty], "rotate": rotate, "scale": [1.0, 1.0] }),
    );
}

pub(super) fn run_animation_eval_curve(document: &Document, label: &str) {
    let t = numeric_attr(selected_container(document).as_ref(), "data-t").unwrap_or(0.5);
    invoke_dual(
        document,
        label,
        "Render.animation_eval_curve",
        format!("animation_eval_curve sketch t={t}"),
        json!({ "curve": "cubic-in-out", "t": t }),
    );
}

pub(super) fn run_animation_spring_step(document: &Document, label: &str) {
    let n = nums(document);
    let current = n.first().copied().unwrap_or(0.0);
    let target = n.get(1).copied().unwrap_or(1.0);
    invoke_dual(
        document,
        label,
        "Render.animation_spring_step",
        format!("animation_spring_step sketch {current}→{target}"),
        json!({ "current": current, "target": target, "velocity": 0.0 }),
    );
}

pub(super) fn run_animation_sclerp(document: &Document, label: &str) {
    let t = numeric_attr(selected_container(document).as_ref(), "data-t").unwrap_or(0.5);
    invoke_dual(
        document,
        label,
        "Render.animation_sclerp",
        format!("animation_sclerp sketch t={t} (identity motors)"),
        json!({ "t": t }),
    );
}

pub(super) fn run_animation_eval_preset(document: &Document, label: &str) {
    let t = numeric_attr(selected_container(document).as_ref(), "data-t").unwrap_or(0.0);
    invoke_dual(
        document,
        label,
        "Render.animation_eval_preset",
        format!("animation_eval_preset sketch orbit_spin t={t}"),
        json!({ "family": "spatial_kinematics", "preset": "orbit_spin", "t": t }),
    );
}

pub(super) fn run_animation_squad_step(document: &Document, label: &str) {
    let t = numeric_attr(selected_container(document).as_ref(), "data-t").unwrap_or(0.5);
    invoke_dual(
        document,
        label,
        "Render.animation_squad_step",
        format!("animation_squad_step sketch t={t} (identity quats)"),
        json!({ "t": t }),
    );
}

pub(super) fn run_animation_list_presets(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Render.animation_list_presets",
        "animation_list_presets sketch".into(),
        json!({}),
    );
}

pub(super) fn run_animation_compute_pass(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Render.animation_compute_pass",
        "animation_compute_pass sketch WGSL batch (needs GPU)".into(),
        json!({ "entity_count": 64, "preset": "orbit", "dt": 0.016 }),
    );
}

pub(super) fn run_svg_path(document: &Document, label: &str) {
    let mut points = nums(document);
    if points.len() < 4 {
        points = vec![0.0, 0.0, 8.0, 8.0];
    }
    invoke_dual(
        document,
        label,
        "Render.svg_path",
        format!("svg_path sketch n={}", points.len() / 2),
        json!({ "points": points }),
    );
}

pub(super) fn run_svg_circle(document: &Document, label: &str) {
    let n = nums(document);
    invoke_dual(
        document,
        label,
        "Render.svg_circle",
        "svg_circle sketch".into(),
        json!({
            "cx": n.first().copied().unwrap_or(16.0),
            "cy": n.get(1).copied().unwrap_or(16.0),
            "r": n.get(2).copied().unwrap_or(8.0),
        }),
    );
}

pub(super) fn run_svg_rect(document: &Document, label: &str) {
    let n = nums(document);
    invoke_dual(
        document,
        label,
        "Render.svg_rect",
        "svg_rect sketch".into(),
        json!({
            "x": n.first().copied().unwrap_or(0.0),
            "y": n.get(1).copied().unwrap_or(0.0),
            "width": n.get(2).copied().unwrap_or(32.0),
            "height": n.get(3).copied().unwrap_or(16.0),
        }),
    );
}

pub(super) fn run_svg_line(document: &Document, label: &str) {
    let n = nums(document);
    invoke_dual(
        document,
        label,
        "Render.svg_line",
        "svg_line sketch".into(),
        json!({
            "x1": n.first().copied().unwrap_or(0.0),
            "y1": n.get(1).copied().unwrap_or(0.0),
            "x2": n.get(2).copied().unwrap_or(32.0),
            "y2": n.get(3).copied().unwrap_or(16.0),
        }),
    );
}

pub(super) fn run_svg_bezier(document: &Document, label: &str) {
    let mut cp = nums(document);
    if cp.len() < 6 {
        cp = vec![0.0, 0.0, 0.0, 8.0, 8.0, 0.0];
    }
    while cp.len() % 3 != 0 {
        cp.push(0.0);
    }
    invoke_dual(
        document,
        label,
        "Render.svg_bezier",
        format!("svg_bezier sketch control={}", cp.len() / 3),
        json!({ "control_points": cp, "segments": 16 }),
    );
}

pub(super) fn run_svg_field(document: &Document, label: &str) {
    let mut amplitudes = nums(document);
    if amplitudes.is_empty() {
        amplitudes = vec![0.0, 0.5, 1.0, 0.25];
    }
    invoke_dual(
        document,
        label,
        "Render.svg_field",
        format!("svg_field sketch n={}", amplitudes.len()),
        json!({ "amplitudes": amplitudes, "nx": 2, "ny": 2 }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wave30_render_parse_f64s() {
        assert_eq!(parse_f64s("1 2,3"), vec![1.0, 2.0, 3.0]);
    }
}
