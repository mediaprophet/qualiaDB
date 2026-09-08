//! Dual-path Tool Chest actions for curated `NLP.*` ALL_BOUND ids (wave 21).
//!
//! Exposes tokenization, sentence splitting, coreference, frame extraction,
//! FST lookup, gazetteer compilation, graph-augmented retrieval, relation
//! extraction, and substrate synthesis.
//! No Host widen — scopes must already exist in `poet_host/invoke/ids.rs`.

use serde_json::{json, Value};
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

fn source_or_default(document: &Document, default: &'static str) -> String {
    selected_source(document).unwrap_or_else(|| default.to_string())
}

fn invoke_dual(
    document: &Document,
    label: &str,
    cap_id: &'static str,
    local_message: String,
    args: Value,
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

// ── Pure local fallback sketches ──────────────────────────────────────────

pub(crate) fn local_word_count(text: &str) -> usize {
    text.split_whitespace().count()
}

pub(crate) fn local_sentence_count(text: &str) -> usize {
    let count = text
        .split(|c| c == '.' || c == '!' || c == '?')
        .filter(|s| !s.trim().is_empty())
        .count();
    count.max(1)
}

// ── Live action runners ───────────────────────────────────────────────────

/// `NLP.tokenize` — tokenize selected text into words and punctuation.
pub fn run_tokenize(document: &Document, label: &str) {
    let text = source_or_default(document, "The reference system processed the input stream.");
    let words = local_word_count(&text);
    invoke_dual(
        document,
        label,
        "NLP.tokenize",
        format!("tokenized sketch ~{words} tokens from {} chars", text.len()),
        json!({ "text": text }),
    );
}

/// `NLP.split_sentences` — segment text into sentences.
pub fn run_split_sentences(document: &Document, label: &str) {
    let text = source_or_default(
        document,
        "The sensor recorded telemetry. Signals were verified by the gate.",
    );
    let sents = local_sentence_count(&text);
    invoke_dual(
        document,
        label,
        "NLP.split_sentences",
        format!("split sketch into ~{sents} sentences"),
        json!({ "text": text }),
    );
}

/// `NLP.coref_resolve` — resolve coreference chains.
pub fn run_coref_resolve(document: &Document, label: &str) {
    let text = source_or_default(
        document,
        "Alice reviewed the dataset. She signed the attestation record.",
    );
    invoke_dual(
        document,
        label,
        "NLP.coref_resolve",
        format!("coref sketch over {} chars (antecedent detection)", text.len()),
        json!({ "text": text }),
    );
}

/// `NLP.frame_extract` — extract semantic frame instances.
pub fn run_frame_extract(document: &Document, label: &str) {
    let text = source_or_default(
        document,
        "The agent transferred 50 credits to the provider.",
    );
    invoke_dual(
        document,
        label,
        "NLP.frame_extract",
        format!("frame extraction sketch over {} chars", text.len()),
        Value::String(text),
    );
}

/// `NLP.fst_lookup` — morphological lookup via finite state transducer.
pub fn run_fst_lookup(document: &Document, label: &str) {
    let word = source_or_default(document, "verifying");
    let clean_word = word.split_whitespace().next().unwrap_or("verifying");
    invoke_dual(
        document,
        label,
        "NLP.fst_lookup",
        format!("FST lemma sketch for word '{clean_word}'"),
        json!({ "word": clean_word }),
    );
}

/// `NLP.gazetteer_build` — compile a gazetteer matcher index.
pub fn run_gazetteer_build(document: &Document, label: &str) {
    let text = source_or_default(document, "catchment sensor station");
    let patterns: Vec<String> = text
        .split_whitespace()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    invoke_dual(
        document,
        label,
        "NLP.gazetteer_build",
        format!("gazetteer built sketch with {} patterns", patterns.len()),
        json!({ "patterns": patterns }),
    );
}

/// `NLP.graphrag_query` — graph-augmented contextual retrieval.
pub fn run_graphrag_query(document: &Document, label: &str) {
    let query = source_or_default(document, "catchment safety bounds");
    invoke_dual(
        document,
        label,
        "NLP.graphrag_query",
        format!("graph-rag query sketch: '{query}'"),
        json!({
            "query": query,
            "k": 5,
            "triples": [
                ["CatchmentA", "hasThreshold", "Safe"],
                ["CatchmentA", "monitoredBy", "Sensor42"]
            ]
        }),
    );
}

/// `NLP.relation_extract` — extract RDF-Star relation statements.
pub fn run_relation_extract(document: &Document, label: &str) {
    let text = source_or_default(
        document,
        "The thermal governor regulates the execution quota.",
    );
    invoke_dual(
        document,
        label,
        "NLP.relation_extract",
        format!("relation extraction sketch over {} chars", text.len()),
        Value::String(text),
    );
}

/// `NLP.substrate_extract` — end-to-end symbolic extraction pipeline.
pub fn run_substrate_extract(document: &Document, label: &str) {
    let text = source_or_default(
        document,
        "Alice inspected Station-7. She confirmed normal flow levels.",
    );
    invoke_dual(
        document,
        label,
        "NLP.substrate_extract",
        format!("full substrate extraction pipeline over {} chars", text.len()),
        Value::String(text),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_nlp_wave21_sketches() {
        assert_eq!(local_word_count("hello world from poet"), 4);
        assert_eq!(local_sentence_count("First sentence. Second sentence! Third?"), 3);
        assert_eq!(local_sentence_count("Single sentence without trailing dot"), 1);
    }
}
