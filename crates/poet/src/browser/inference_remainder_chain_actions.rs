//! Dual-path Tool Chest remainder for Host-bound `Inference.*` ids (wave 27).
//!
//! Completes the `ai:inf` chain: load/unload, transformer, reranker, and
//! constrained decode. Local sketches stay honest about native-only seams.

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
    let bounded: String = text.chars().take(16_384).collect();
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

fn parse_u64s(source: &str) -> Vec<u64> {
    source
        .split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ';' | '|' | '\n' | '\r'))
        .filter_map(|token| token.trim().parse::<u64>().ok())
        .take(256)
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

fn default_vocab() -> Vec<(u64, String)> {
    vec![
        (0, "<pad>".into()),
        (1, "hello".into()),
        (2, "world".into()),
        (3, "the".into()),
    ]
}

fn parse_vocab_lines(source: &str) -> Vec<(u64, String)> {
    let mut vocab = Vec::new();
    for (i, line) in source.lines().filter(|l| !l.trim().is_empty()).take(64).enumerate() {
        let mut parts = line.split_whitespace();
        if let Some(first) = parts.next() {
            if let Ok(id) = first.parse::<u64>() {
                let token = parts.next().unwrap_or(first).to_string();
                vocab.push((id, token));
                continue;
            }
        }
        vocab.push((i as u64, line.trim().to_string()));
    }
    if vocab.is_empty() {
        default_vocab()
    } else {
        vocab
    }
}

/// `Inference.load_model` — `{ path, model_id?, mlock? }`. Native Host; WASM E300.
pub(super) fn run_load_model(document: &Document, label: &str) {
    let container = selected_container(document);
    let path = string_attr(container.as_ref(), "data-model-path")
        .or_else(|| selected_source(document).map(|s| s.trim().lines().next().unwrap_or("").to_string()))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "/models/default.gguf".into());
    let model_id = numeric_attr(container.as_ref(), "data-model-id")
        .map(|v| v.max(0.0) as u64)
        .unwrap_or(0);
    let mlock = string_attr(container.as_ref(), "data-mlock")
        .map(|s| matches!(s.as_str(), "1" | "true" | "yes"))
        .unwrap_or(false);
    invoke_dual(
        document,
        label,
        "Inference.load_model",
        format!("load_model sketch path={path} model_id={model_id} (native mmap)"),
        json!({ "path": path, "model_id": model_id, "mlock": mlock }),
    );
}

/// `Inference.unload_model` — no args.
pub(super) fn run_unload_model(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Inference.unload_model",
        "unload_model sketch: drop resident mmap".into(),
        json!({}),
    );
}

/// `Inference.run_transformer` — `{ tokens, max_layers? }`.
pub(super) fn run_run_transformer(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_else(|| "1 2 3 4".into());
    let tokens = parse_u64s(&source);
    let tokens = if tokens.is_empty() {
        vec![1u64, 2, 3]
    } else {
        tokens
    };
    let max_layers = numeric_attr(container.as_ref(), "data-max-layers")
        .map(|v| v.max(0.0) as u64)
        .unwrap_or(0);
    invoke_dual(
        document,
        label,
        "Inference.run_transformer",
        format!(
            "run_transformer sketch tokens={} max_layers={max_layers} (needs resident model)",
            tokens.len()
        ),
        json!({ "tokens": tokens, "max_layers": max_layers }),
    );
}

/// `Inference.run_reranker` — `{ query, candidates }`.
pub(super) fn run_run_reranker(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_else(|| "hello world\nmachine learning".into());
    let mut lines: Vec<String> = source
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .take(32)
        .collect();
    if lines.is_empty() {
        lines = vec!["hello world".into(), "machine learning".into()];
    }
    let query = string_attr(container.as_ref(), "data-query")
        .or_else(|| lines.first().cloned())
        .unwrap_or_else(|| "hello".into());
    invoke_dual(
        document,
        label,
        "Inference.run_reranker",
        format!("run_reranker sketch query={query} candidates={}", lines.len()),
        json!({ "query": query, "candidates": lines }),
    );
}

/// `Inference.constrained_decode` — `{ vocab: [[id, str], ...], logits? }`.
pub(super) fn run_constrained_decode(document: &Document, label: &str) {
    let source = selected_source(document).unwrap_or_default();
    let vocab = parse_vocab_lines(&source);
    let vocab_json: Vec<serde_json::Value> = vocab
        .iter()
        .map(|(id, token)| json!([*id, token]))
        .collect();
    invoke_dual(
        document,
        label,
        "Inference.constrained_decode",
        format!("constrained_decode sketch vocab_n={}", vocab.len()),
        json!({ "vocab": vocab_json }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wave27_parse_u64s_and_vocab() {
        assert_eq!(parse_u64s("1, 2 3"), vec![1, 2, 3]);
        let vocab = parse_vocab_lines("0 pad\nhello");
        assert_eq!(vocab[0], (0, "pad".into()));
        assert_eq!(vocab[1], (1, "hello".into()));
        assert!(!default_vocab().is_empty());
    }
}
