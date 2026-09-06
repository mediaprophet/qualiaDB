//! Local and live-invoke actions for previously empty Tool Chest chains.
//!
//! Named beats stay on the chrome shell. No Host widen.

use std::collections::BTreeMap;

use vibe::{Span, Value};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, Event, HtmlElement, HtmlInputElement};

fn selected_container(document: &Document) -> Option<Element> {
    document
        .query_selector(".canvas-container-node.selected")
        .ok()
        .flatten()
}

fn selected_source(document: &Document) -> Option<String> {
    let container = selected_container(document)?;
    let text = container
        .query_selector(".vibe-editor, .vibe-editor-textarea, .doc-editor")
        .ok()
        .flatten()
        .and_then(|editor| editor.text_content())
        .or_else(|| container.text_content())?;
    let bounded: String = text.chars().take(16_384).collect();
    (!bounded.trim().is_empty()).then_some(bounded)
}

fn need_container(document: &Document, label: &str, message: &str) -> Option<Element> {
    match selected_container(document) {
        Some(container) => Some(container),
        None => {
            super::interactions::show_tool_status(document, label, message, "error");
            None
        }
    }
}

fn apply_style(container: &Element, property: &str, value: &str) {
    if let Ok(el) = container.clone().dyn_into::<HtmlElement>() {
        let _ = el.style().set_property(property, value);
    }
}

pub(super) fn parse_numbers(source: &str) -> Vec<f64> {
    source
        .split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ';' | '|'))
        .filter_map(|token| token.trim().parse::<f64>().ok())
        .filter(|n| n.is_finite())
        .take(4096)
        .collect()
}

pub(super) fn local_mean(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    Some(values.iter().sum::<f64>() / values.len() as f64)
}

pub(super) fn local_median(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
    let n = sorted.len();
    if n % 2 == 0 {
        Some((sorted[n / 2 - 1] + sorted[n / 2]) / 2.0)
    } else {
        Some(sorted[n / 2])
    }
}

pub(super) fn local_variance(values: &[f64], sample: bool) -> Option<f64> {
    let mean = local_mean(values)?;
    let ss = values
        .iter()
        .map(|value| {
            let delta = value - mean;
            delta * delta
        })
        .sum::<f64>();
    let denom = if sample {
        (values.len() - 1) as f64
    } else {
        values.len() as f64
    };
    Some(ss / denom)
}

pub(super) fn local_std_dev(values: &[f64], sample: bool) -> Option<f64> {
    local_variance(values, sample).map(f64::sqrt)
}

pub(super) fn local_min(values: &[f64]) -> Option<f64> {
    values
        .iter()
        .copied()
        .reduce(|a, b| if b < a { b } else { a })
}

pub(super) fn local_max(values: &[f64]) -> Option<f64> {
    values
        .iter()
        .copied()
        .reduce(|a, b| if b > a { b } else { a })
}

fn run_sheet_stat<F>(
    document: &Document,
    label: &str,
    cap_id: &'static str,
    stat_noun: &'static str,
    local_fn: fn(&[f64]) -> Option<f64>,
    build_args: F,
) where
    F: FnOnce(&[f64]) -> serde_json::Value,
{
    let source = selected_source(document).unwrap_or_default();
    let values = parse_numbers(&source);
    let Some(value) = local_fn(&values) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Select a sheet or document that contains numbers.",
            "error",
        );
        return;
    };
    let label = label.to_string();
    if !super::native_daemon::is_daemon_connected() {
        let report = super::tool_dual_path::local_sketch(
            cap_id,
            &format!("{stat_noun} of {} values: {value}", values.len()),
        );
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
    let args = build_args(&values);
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

pub(super) fn run_brush_stroke(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a container before applying a stroke.",
    ) else {
        return;
    };
    apply_style(
        &container,
        "outline",
        "2px solid color-mix(in srgb, var(--media-2d) 70%, transparent)",
    );
    let _ = container.set_attribute("data-brush-stroke", "1");
    super::history::push_current_frame("brush stroke");
    super::interactions::show_tool_status(
        document,
        label,
        "Stroke applied to the selected surface.",
        "success",
    );
}

pub(super) fn run_brush_clear(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a container before clearing a stroke.",
    ) else {
        return;
    };
    apply_style(&container, "outline", "none");
    let _ = container.remove_attribute("data-brush-stroke");
    super::history::push_current_frame("clear stroke");
    super::interactions::show_tool_status(document, label, "Stroke cleared.", "success");
}

pub(super) fn run_fill(document: &Document, label: &str, token: &str) {
    let Some(container) =
        need_container(document, label, "Select a container before applying fill.")
    else {
        return;
    };
    let color = match token {
        "warm" => "color-mix(in srgb, var(--media-film) 18%, var(--surface-panel))",
        _ => "color-mix(in srgb, var(--media-3d) 18%, var(--surface-panel))",
    };
    apply_style(&container, "background", color);
    let _ = container.set_attribute("data-fill", token);
    super::history::push_current_frame("palette fill");
    super::interactions::show_tool_status(
        document,
        label,
        &format!("Applied {token} fill to the selected surface."),
        "success",
    );
}

pub(super) fn run_heatmap(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a container with numeric values before generating a heatmap.",
    ) else {
        return;
    };
    let source = container.text_content().unwrap_or_default();
    let values = parse_numbers(&source);
    if values.is_empty() {
        super::interactions::show_tool_status(
            document,
            label,
            "No finite numbers found on the selected surface.",
            "error",
        );
        return;
    }
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let t = if (max - min).abs() < f64::EPSILON {
        0.5
    } else {
        ((local_mean(&values).unwrap_or(min) - min) / (max - min)).clamp(0.0, 1.0)
    };
    let mix = (t * 70.0).round() as u32;
    apply_style(
        &container,
        "background",
        &format!("color-mix(in srgb, var(--media-film) {mix}%, var(--surface-panel))"),
    );
    let _ = container.set_attribute("data-heatmap", "1");
    super::history::push_current_frame("heatmap");
    super::interactions::show_tool_status(
        document,
        label,
        &format!(
            "Heatmap from {} values (min {min:.4}, max {max:.4}).",
            values.len()
        ),
        "success",
    );
}

pub(super) fn run_camera_reset(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a map or 3D container before resetting the camera.",
    ) else {
        return;
    };
    let _ = container.set_attribute("data-camera-yaw", "0");
    let _ = container.set_attribute("data-camera-pitch", "0");
    let _ = container.set_attribute("data-camera-zoom", "1");
    super::history::push_current_frame("camera reset");
    super::interactions::show_tool_status(
        document,
        label,
        "Camera orbit reset to yaw 0, pitch 0, zoom 1 on the selected surface.",
        "success",
    );
}

pub(super) fn run_orbit_preview(document: &Document, label: &str) {
    let mut args = BTreeMap::new();
    args.insert("family".into(), Value::String("spatial_kinematics".into()));
    args.insert("preset".into(), Value::String("orbit_spin".into()));
    args.insert("t".into(), Value::F64(0.5));
    match crate::vibe_host::capability_invoke(
        "Animation.evaluate_preset",
        &Value::Record(args),
        Span::point(0),
    ) {
        Ok(value) => {
            if let Some(container) = selected_container(document) {
                let _ = container.set_attribute("data-orbit-preview", "1");
            }
            super::interactions::show_tool_status(
                document,
                label,
                &format!("Animation.evaluate_preset orbit_spin t=0.5 → {value:?}"),
                "success",
            );
        }
        Err(error) => {
            super::interactions::show_tool_status(document, label, &error.message, "error")
        }
    }
}

pub(super) fn run_sheet_mean(document: &Document, label: &str) {
    run_sheet_stat(
        document,
        label,
        "Statistics.mean",
        "mean",
        local_mean,
        |values| serde_json::json!({ "values": values }),
    );
}

pub(super) fn run_sheet_median(document: &Document, label: &str) {
    run_sheet_stat(
        document,
        label,
        "Statistics.median",
        "median",
        local_median,
        |values| serde_json::json!({ "values": values }),
    );
}

pub(super) fn run_sheet_variance(document: &Document, label: &str) {
    run_sheet_stat(
        document,
        label,
        "Statistics.variance",
        "variance",
        |values| local_variance(values, true),
        |values| serde_json::json!({ "values": values, "sample": true }),
    );
}

pub(super) fn run_sheet_std_dev(document: &Document, label: &str) {
    run_sheet_stat(
        document,
        label,
        "Statistics.std_dev",
        "std dev",
        |values| local_std_dev(values, true),
        |values| serde_json::json!({ "values": values, "sample": true }),
    );
}

pub(super) fn run_sheet_min(document: &Document, label: &str) {
    run_sheet_stat(
        document,
        label,
        "Statistics.min",
        "min",
        local_min,
        |values| serde_json::json!({ "values": values }),
    );
}

pub(super) fn run_sheet_max(document: &Document, label: &str) {
    run_sheet_stat(
        document,
        label,
        "Statistics.max",
        "max",
        local_max,
        |values| serde_json::json!({ "values": values }),
    );
}

pub(super) fn run_sheet_import(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a spreadsheet container before importing.",
    ) else {
        return;
    };
    let input = document.create_element("input").unwrap();
    input.set_attribute("type", "file").ok();
    input.set_attribute("accept", ".csv,.tsv,.txt,.hcf").ok();
    let input_el: HtmlInputElement = input.clone().dyn_into().unwrap();
    let _ = input_el.style().set_property("display", "none");
    let label = label.to_string();
    let container = container.clone();
    let picker = input_el.clone();
    let closure = Closure::wrap(Box::new(move |_event: Event| {
        let Some(file) = picker.files().and_then(|list| list.get(0)) else {
            return;
        };
        let Ok(reader) = web_sys::FileReader::new() else {
            return;
        };
        let label = label.clone();
        let container = container.clone();
        let reader_for_load = reader.clone();
        let load = Closure::wrap(Box::new(move |_event: Event| {
            let Some(document) = web_sys::window().and_then(|window| window.document()) else {
                return;
            };
            let text = reader_for_load
                .result()
                .ok()
                .and_then(|value| value.as_string())
                .unwrap_or_default();
            match super::sheet::import_delimited_into(&container, &text) {
                Ok(written) => super::interactions::show_tool_status(
                    &document,
                    &label,
                    &format!("Imported {written} cells from A1."),
                    "success",
                ),
                Err(error) => {
                    super::interactions::show_tool_status(&document, &label, error, "error")
                }
            }
        }) as Box<dyn FnMut(Event)>);
        let _ = reader.add_event_listener_with_callback("load", load.as_ref().unchecked_ref());
        load.forget();
        let _ = reader.read_as_text(&file);
    }) as Box<dyn FnMut(Event)>);
    let _ = input.add_event_listener_with_callback("change", closure.as_ref().unchecked_ref());
    closure.forget();
    if let Some(body) = document.body() {
        let _ = body.append_child(&input);
        input_el.click();
        input.remove();
    }
}

pub(super) fn run_vibe_diagnose(document: &Document, label: &str) {
    let Some(source) = selected_source(document) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Select a VibeScript cell or document with source first.",
            "error",
        );
        return;
    };
    let report = crate::vibe_host::diagnose(&source);
    let json = report.to_json();
    let kind = if report.valid { "success" } else { "error" };
    super::interactions::show_tool_status(document, label, &json, kind);
}

pub(super) fn run_quin_statement(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a container before constructing a quin.statement.",
    ) else {
        return;
    };
    let source = selected_source(document).unwrap_or_default();
    let tokens: Vec<&str> = source.split_whitespace().take(3).collect();
    if tokens.len() < 3 {
        super::interactions::show_tool_status(
            document,
            label,
            "Need three UTF-8 tokens (subject predicate object) in the selected surface.",
            "error",
        );
        return;
    }
    let _ = container.set_attribute("data-quin-subject", tokens[0]);
    let _ = container.set_attribute("data-quin-predicate", tokens[1]);
    let _ = container.set_attribute("data-quin-object", tokens[2]);
    super::history::push_current_frame("quin.statement");
    super::interactions::show_tool_status(
        document,
        label,
        &format!(
            "quin.statement {{ {} {} {} }} on the selected container.",
            tokens[0], tokens[1], tokens[2]
        ),
        "success",
    );
}

pub(super) fn run_grounding(document: &Document, label: &str) {
    let Some(text) = selected_source(document) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Select a document or cell with generation text first.",
            "error",
        );
        return;
    };
    let prompt = "ground selected generation";
    let label = label.to_string();
    if !super::native_daemon::is_daemon_connected() {
        let cites = text.matches("did:").count() + text.matches("urn:").count();
        let report = super::tool_dual_path::local_sketch(
            "Inference.grounding",
            &format!(
                "{} DID/URN citations in {} characters.",
                cites,
                text.chars().count()
            ),
        );
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
        "Running Inference.grounding…",
        "running",
    );
    wasm_bindgen_futures::spawn_local(async move {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        let args = serde_json::json!({ "prompt": prompt, "text": text });
        match super::native_daemon::daemon_invoke("Inference.grounding", args).await {
            Ok(response) if response.ok => {
                let report = super::tool_dual_path::live_ok("Inference.grounding", &response.value);
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Ok(response) => {
                let report = super::tool_dual_path::live_denied(
                    "Inference.grounding",
                    response
                        .diagnostic
                        .as_deref()
                        .unwrap_or("Inference.grounding failed."),
                );
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Err(error) => {
                let report = super::tool_dual_path::live_denied("Inference.grounding", &error);
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

pub(super) fn run_epistemic_evaluate(document: &Document, label: &str) {
    let container = selected_container(document);
    let agent = container
        .as_ref()
        .and_then(|c| c.get_attribute("data-agent-did"))
        .unwrap_or_else(|| "did:q42:agent:default".into());
    let world = container
        .as_ref()
        .and_then(|c| c.get_attribute("data-epistemic-world"))
        .unwrap_or_else(|| "did:q42:world:actual".into());
    let certainty = container
        .as_ref()
        .and_then(|c| c.get_attribute("data-certainty"))
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(1.0);
    let label = label.to_string();
    if !super::native_daemon::is_daemon_connected() {
        let report = super::tool_dual_path::local_sketch(
            "EpistemicLogic.evaluate",
            &format!(
                "Local epistemic frame sketch for agent {agent} in {world} (certainty {certainty}). Connect QualiaDB for a live frame scan."
            ),
        );
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
        "Running EpistemicLogic.evaluate…",
        "running",
    );
    wasm_bindgen_futures::spawn_local(async move {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        let args = serde_json::json!({
            "agent": agent,
            "world": world,
            "certainty": certainty,
        });
        match super::native_daemon::daemon_invoke("EpistemicLogic.evaluate", args).await {
            Ok(response) if response.ok => {
                let report =
                    super::tool_dual_path::live_ok("EpistemicLogic.evaluate", &response.value);
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Ok(response) => {
                let report = super::tool_dual_path::live_denied(
                    "EpistemicLogic.evaluate",
                    response
                        .diagnostic
                        .as_deref()
                        .unwrap_or("EpistemicLogic.evaluate failed."),
                );
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Err(error) => {
                let report = super::tool_dual_path::live_denied("EpistemicLogic.evaluate", &error);
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

fn run_inference_draft_check(
    document: &Document,
    label: &str,
    cap_id: &'static str,
    draft_key: &'static str,
) {
    let Some(text) = selected_source(document) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Select a document or cell with generation text first.",
            "error",
        );
        return;
    };
    let prompt = "check selected generation";
    let label = label.to_string();
    if !super::native_daemon::is_daemon_connected() {
        let cites = text.matches("did:").count() + text.matches("urn:").count();
        let report = super::tool_dual_path::local_sketch(
            cap_id,
            &format!(
                "Local sketch: {} citation markers in {} characters. Connect QualiaDB for {cap_id}.",
                cites,
                text.chars().count()
            ),
        );
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
        let mut args = serde_json::Map::new();
        args.insert("prompt".into(), serde_json::Value::String(prompt.into()));
        args.insert(draft_key.into(), serde_json::Value::String(text));
        match super::native_daemon::daemon_invoke(cap_id, serde_json::Value::Object(args)).await {
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

pub(super) fn run_detect_ungrounded(document: &Document, label: &str) {
    run_inference_draft_check(document, label, "Inference.detect_ungrounded", "draft");
}

pub(super) fn run_verify_turn(document: &Document, label: &str) {
    run_inference_draft_check(document, label, "Inference.verify_turn", "draft");
}

pub(super) fn run_image_histogram(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a picture surface before running a histogram.",
    ) else {
        return;
    };
    let has_pixels = container.get_attribute("data-grayscale-u8").is_some()
        && container.get_attribute("data-pixel-width").is_some()
        && container.get_attribute("data-pixel-height").is_some();
    let label = label.to_string();
    if !super::native_daemon::is_daemon_connected() {
        let sketch = if has_pixels {
            "Local histogram sketch: greyscale pixels are present on this surface. Connect QualiaDB for ComputerVision.histogram."
        } else {
            "Local histogram sketch: decode greyscale pixels onto this surface (data-grayscale-u8) before a live histogram."
        };
        let report = super::tool_dual_path::local_sketch("ComputerVision.histogram", sketch);
        super::interactions::show_tool_status(
            document,
            &label,
            &report.message,
            report.status_kind,
        );
        return;
    }
    if !has_pixels {
        super::interactions::show_tool_status(
            document,
            &label,
            "The selected picture has no decoded greyscale pixels yet.",
            "unavailable",
        );
        return;
    }
    let width = container
        .get_attribute("data-pixel-width")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);
    let height = container
        .get_attribute("data-pixel-height")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);
    let raw = container
        .get_attribute("data-grayscale-u8")
        .unwrap_or_default();
    let data: Vec<u8> = raw
        .split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ';'))
        .filter_map(|token| token.trim().parse::<u8>().ok())
        .take(512 * 512)
        .collect();
    if data.len() as u64 != width.saturating_mul(height) || width == 0 || height == 0 {
        super::interactions::show_tool_status(
            document,
            &label,
            "Greyscale pixel length does not match the declared picture size.",
            "error",
        );
        return;
    }
    super::interactions::show_tool_status(
        document,
        &label,
        "Running ComputerVision.histogram…",
        "running",
    );
    wasm_bindgen_futures::spawn_local(async move {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        let args = serde_json::json!({
            "data": data,
            "width": width,
            "height": height,
            "stride": width,
        });
        match super::native_daemon::daemon_invoke("ComputerVision.histogram", args).await {
            Ok(response) if response.ok => {
                let report =
                    super::tool_dual_path::live_ok("ComputerVision.histogram", &response.value);
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Ok(response) => {
                let report = super::tool_dual_path::live_denied(
                    "ComputerVision.histogram",
                    response
                        .diagnostic
                        .as_deref()
                        .unwrap_or("ComputerVision.histogram failed."),
                );
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Err(error) => {
                let report =
                    super::tool_dual_path::live_denied("ComputerVision.histogram", &error);
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

pub(super) fn run_pulse_presence(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a presence container before publishing pulse presence.",
    ) else {
        return;
    };
    let _ = container.set_attribute("data-pulse-presence", "1");
    let label = label.to_string();
    if !super::native_daemon::is_daemon_connected() {
        let report = super::tool_dual_path::local_sketch(
            "Pulse.publish_presence",
            "presence mark set on the selected container.",
        );
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
        "Running Pulse.publish_presence…",
        "running",
    );
    wasm_bindgen_futures::spawn_local(async move {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        let args = serde_json::json!({ "topic": "presence", "payload": "poet-surface" });
        match super::native_daemon::daemon_invoke("Pulse.publish_presence", args).await {
            Ok(response) if response.ok => {
                let report =
                    super::tool_dual_path::live_ok("Pulse.publish_presence", &response.value);
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Ok(response) => {
                let report = super::tool_dual_path::live_denied(
                    "Pulse.publish_presence",
                    response
                        .diagnostic
                        .as_deref()
                        .unwrap_or("Pulse.publish_presence failed."),
                );
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Err(error) => {
                let report = super::tool_dual_path::live_denied("Pulse.publish_presence", &error);
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

pub(super) fn run_deontic_obligate(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a container before tagging an obligation.",
    ) else {
        return;
    };
    let _ = container.set_attribute("data-deontic", "obligate");
    super::history::push_current_frame("deontic obligate");
    let label = label.to_string();
    if !super::native_daemon::is_daemon_connected() {
        let report = super::tool_dual_path::local_sketch(
            "DeonticLogic.evaluate",
            "obligation tag set on the selected container.",
        );
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
        "Running DeonticLogic.evaluate…",
        "running",
    );
    wasm_bindgen_futures::spawn_local(async move {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        let args = serde_json::json!({ "operation": "compile", "modality": "obligate" });
        match super::native_daemon::daemon_invoke("DeonticLogic.evaluate", args).await {
            Ok(response) if response.ok => {
                let report =
                    super::tool_dual_path::live_ok("DeonticLogic.evaluate", &response.value);
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Ok(response) => {
                let report = super::tool_dual_path::live_denied(
                    "DeonticLogic.evaluate",
                    response
                        .diagnostic
                        .as_deref()
                        .unwrap_or("DeonticLogic.evaluate failed."),
                );
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Err(error) => {
                let report = super::tool_dual_path::live_denied("DeonticLogic.evaluate", &error);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_mean_of_three() {
        assert_eq!(local_mean(&[1.0, 2.0, 3.0]), Some(2.0));
        assert_eq!(local_mean(&[]), None);
    }

    #[test]
    fn local_median_of_four() {
        assert_eq!(local_median(&[1.0, 2.0, 3.0, 4.0]), Some(2.5));
        assert_eq!(local_median(&[1.0, 3.0, 2.0]), Some(2.0));
    }

    #[test]
    fn local_variance_sample_of_three() {
        assert_eq!(local_variance(&[1.0, 2.0, 3.0], true), Some(1.0));
    }

    #[test]
    fn local_std_dev_sample_of_three() {
        assert_eq!(local_std_dev(&[1.0, 2.0, 3.0], true), Some(1.0));
    }

    #[test]
    fn local_min_max() {
        assert_eq!(local_min(&[3.0, 1.0, 4.0]), Some(1.0));
        assert_eq!(local_max(&[3.0, 1.0, 4.0]), Some(4.0));
    }

    #[test]
    fn parse_numbers_accepts_utf8_separators() {
        let values = parse_numbers("1 2,3;4");
        assert_eq!(values, vec![1.0, 2.0, 3.0, 4.0]);
    }
}
