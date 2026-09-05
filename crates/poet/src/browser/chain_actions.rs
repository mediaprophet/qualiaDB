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
    let source = selected_source(document).unwrap_or_default();
    let values = parse_numbers(&source);
    let Some(mean) = local_mean(&values) else {
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
        super::interactions::show_tool_status(
            document,
            &label,
            &format!("Local mean of {} values: {mean}", values.len()),
            "success",
        );
        return;
    }
    super::interactions::show_tool_status(document, &label, "Running Statistics.mean…", "running");
    wasm_bindgen_futures::spawn_local(async move {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        let args = serde_json::json!({ "values": values });
        match super::native_daemon::daemon_invoke("Statistics.mean", args).await {
            Ok(response) if response.ok => {
                super::interactions::show_tool_status(&document, &label, &response.value, "success")
            }
            Ok(response) => super::interactions::show_tool_status(
                &document,
                &label,
                &format!(
                    "Local mean {mean} after daemon rejection ({})",
                    response
                        .diagnostic
                        .as_deref()
                        .unwrap_or("Statistics.mean failed.")
                ),
                "success",
            ),
            Err(error) => super::interactions::show_tool_status(&document, &label, &error, "error"),
        }
    });
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
        super::interactions::show_tool_status(
            document,
            &label,
            &format!(
                "Local grounding sketch: {} DID/URN citations in {} characters. Live Inference.grounding needs the daemon.",
                cites,
                text.chars().count()
            ),
            "success",
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
                super::interactions::show_tool_status(&document, &label, &response.value, "success")
            }
            Ok(response) => super::interactions::show_tool_status(
                &document,
                &label,
                response
                    .diagnostic
                    .as_deref()
                    .unwrap_or("Inference.grounding failed."),
                "error",
            ),
            Err(error) => super::interactions::show_tool_status(&document, &label, &error, "error"),
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
        super::interactions::show_tool_status(
            document,
            &label,
            "Local presence mark set. Pulse.publish_presence needs the QualiaDB daemon.",
            "success",
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
                super::interactions::show_tool_status(&document, &label, &response.value, "success")
            }
            Ok(response) => super::interactions::show_tool_status(
                &document,
                &label,
                response
                    .diagnostic
                    .as_deref()
                    .unwrap_or("Pulse.publish_presence failed."),
                "error",
            ),
            Err(error) => super::interactions::show_tool_status(&document, &label, &error, "error"),
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
        super::interactions::show_tool_status(
            document,
            &label,
            "Local obligation tag set. DeonticLogic.evaluate against live quins needs the daemon.",
            "success",
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
                super::interactions::show_tool_status(&document, &label, &response.value, "success")
            }
            Ok(response) => super::interactions::show_tool_status(
                &document,
                &label,
                response
                    .diagnostic
                    .as_deref()
                    .unwrap_or("DeonticLogic.evaluate failed."),
                "error",
            ),
            Err(error) => super::interactions::show_tool_status(&document, &label, &error, "error"),
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
    fn parse_numbers_accepts_utf8_separators() {
        let values = parse_numbers("1 2,3;4");
        assert_eq!(values, vec![1.0, 2.0, 3.0, 4.0]);
    }
}
