//! Dual-path Tool Chest actions for curated Host-bound `Research.*` ids (wave 27).
//!
//! Spec hyphenated `research:*` tools already occupy that id family. New Live
//! tools use `research:live_*` on chain `research:live`. Local sketches record
//! enquiry/corpus/dark-link status; Host is the status-record seam.

use serde_json::json;
use web_sys::{Document, Element};

const DEFAULT_ID: &str = "enquiry-1";

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

fn first_line(source: &str) -> String {
    source
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("")
        .to_string()
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

fn enquiry_id(document: &Document) -> String {
    string_attr(selected_container(document).as_ref(), "data-research-id")
        .unwrap_or_else(|| DEFAULT_ID.to_string())
}

fn surface_text(document: &Document) -> String {
    selected_source(document).unwrap_or_default()
}

fn run_id(
    document: &Document,
    label: &str,
    cap_id: &'static str,
    sketch: &str,
    extra: serde_json::Value,
) {
    let id = enquiry_id(document);
    let mut args = extra;
    if let Some(obj) = args.as_object_mut() {
        obj.entry("id".to_string())
            .or_insert_with(|| json!(id.clone()));
    }
    invoke_dual(
        document,
        label,
        cap_id,
        format!("{sketch} id={id}"),
        args,
    );
}

/// `Research.new` — `{ id, purpose?, scope? }`.
pub(super) fn run_new(document: &Document, label: &str) {
    let purpose = string_attr(selected_container(document).as_ref(), "data-purpose")
        .or_else(|| {
            let t = first_line(&surface_text(document));
            (!t.is_empty()).then_some(t)
        })
        .unwrap_or_else(|| "curated enquiry".into());
    run_id(
        document,
        label,
        "Research.new",
        "research new sketch",
        json!({ "purpose": purpose, "scope": ["poet"] }),
    );
}

/// `Research.set_purpose` — `{ id, purpose }`.
pub(super) fn run_set_purpose(document: &Document, label: &str) {
    let purpose = string_attr(selected_container(document).as_ref(), "data-purpose")
        .or_else(|| {
            let t = first_line(&surface_text(document));
            (!t.is_empty()).then_some(t)
        })
        .unwrap_or_else(|| "purpose".into());
    run_id(
        document,
        label,
        "Research.set_purpose",
        "set_purpose sketch",
        json!({ "purpose": purpose }),
    );
}

/// `Research.define_scope` — `{ id, scope: [str] }`.
pub(super) fn run_define_scope(document: &Document, label: &str) {
    let source = surface_text(document);
    let mut scope: Vec<String> = source
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .take(16)
        .collect();
    if scope.is_empty() {
        scope = vec!["poet".into(), "epistemics".into()];
    }
    run_id(
        document,
        label,
        "Research.define_scope",
        "define_scope sketch",
        json!({ "scope": scope }),
    );
}

/// `Research.add_constraint` — `{ id, constraint_type?, value?, description? }`.
pub(super) fn run_add_constraint(document: &Document, label: &str) {
    let description = first_line(&surface_text(document));
    run_id(
        document,
        label,
        "Research.add_constraint",
        "add_constraint sketch",
        json!({
            "constraint_type": "general",
            "value": "",
            "description": description,
        }),
    );
}

/// `Research.add_question` — `{ id, question_id, text }`.
pub(super) fn run_add_question(document: &Document, label: &str) {
    let qid = string_attr(selected_container(document).as_ref(), "data-question-id")
        .unwrap_or_else(|| "q1".into());
    let text = first_line(&surface_text(document));
    let text = if text.is_empty() {
        "what follows?".into()
    } else {
        text
    };
    run_id(
        document,
        label,
        "Research.add_question",
        "add_question sketch",
        json!({ "question_id": qid, "text": text }),
    );
}

/// `Research.link_questions` — `{ id, q1, q2 }`.
pub(super) fn run_link_questions(document: &Document, label: &str) {
    run_id(
        document,
        label,
        "Research.link_questions",
        "link_questions sketch",
        json!({ "q1": "q1", "q2": "q2" }),
    );
}

/// `Research.add_corpus_item` — `{ id, source_type?, title?, content? }`.
pub(super) fn run_add_corpus_item(document: &Document, label: &str) {
    let title = first_line(&surface_text(document));
    run_id(
        document,
        label,
        "Research.add_corpus_item",
        "add_corpus_item sketch",
        json!({
            "source_type": "literature",
            "title": title,
            "content": surface_text(document),
        }),
    );
}

/// `Research.import_literature` — `{ id, title?, content? }`.
pub(super) fn run_import_literature(document: &Document, label: &str) {
    let title = first_line(&surface_text(document));
    run_id(
        document,
        label,
        "Research.import_literature",
        "import_literature sketch",
        json!({ "title": title, "content": surface_text(document) }),
    );
}

/// `Research.import_dataset` — `{ id, title?, content? }`.
pub(super) fn run_import_dataset(document: &Document, label: &str) {
    let title = first_line(&surface_text(document));
    run_id(
        document,
        label,
        "Research.import_dataset",
        "import_dataset sketch",
        json!({ "title": title, "content": surface_text(document) }),
    );
}

/// `Research.set_corpus_confidence` — `{ id, confidence }`.
pub(super) fn run_set_corpus_confidence(document: &Document, label: &str) {
    let confidence = string_attr(selected_container(document).as_ref(), "data-confidence")
        .unwrap_or_else(|| "medium".into());
    run_id(
        document,
        label,
        "Research.set_corpus_confidence",
        "set_corpus_confidence sketch",
        json!({ "confidence": confidence }),
    );
}

/// `Research.extract_from_corpus` — `{ keyword }`.
pub(super) fn run_extract_from_corpus(document: &Document, label: &str) {
    let keyword = first_line(&surface_text(document));
    let keyword = if keyword.is_empty() {
        "keyword".into()
    } else {
        keyword
    };
    invoke_dual(
        document,
        label,
        "Research.extract_from_corpus",
        format!("extract_from_corpus sketch keyword={keyword}"),
        json!({ "keyword": keyword }),
    );
}

/// `Research.infer_dark_link` — `{ id, source, target, link_type? }`.
pub(super) fn run_infer_dark_link(document: &Document, label: &str) {
    run_id(
        document,
        label,
        "Research.infer_dark_link",
        "infer_dark_link sketch",
        json!({ "source": "a", "target": "b", "link_type": "causal" }),
    );
}

fn corpus_items(document: &Document) -> Vec<serde_json::Value> {
    let source = surface_text(document);
    let mut items: Vec<serde_json::Value> = source
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .take(16)
        .enumerate()
        .map(|(i, line)| json!({ "id": format!("item-{i}"), "content": line, "source": "surface" }))
        .collect();
    if items.is_empty() {
        items.push(json!({ "id": "item-0", "content": "note", "source": "surface" }));
    }
    items
}

/// `Research.detect_provenance_gaps` — `{ items }`.
pub(super) fn run_detect_provenance_gaps(document: &Document, label: &str) {
    let items = corpus_items(document);
    invoke_dual(
        document,
        label,
        "Research.detect_provenance_gaps",
        format!("detect_provenance_gaps sketch items={}", items.len()),
        json!({ "items": items }),
    );
}

/// `Research.detect_concealment` — `{ items }`.
pub(super) fn run_detect_concealment(document: &Document, label: &str) {
    let items = corpus_items(document);
    invoke_dual(
        document,
        label,
        "Research.detect_concealment",
        format!("detect_concealment sketch items={}", items.len()),
        json!({ "items": items }),
    );
}

/// `Research.confirm_dark_link` — `{ id }`.
pub(super) fn run_confirm_dark_link(document: &Document, label: &str) {
    run_id(
        document,
        label,
        "Research.confirm_dark_link",
        "confirm_dark_link sketch",
        json!({}),
    );
}

/// `Research.refute_dark_link` — `{ id }`.
pub(super) fn run_refute_dark_link(document: &Document, label: &str) {
    run_id(
        document,
        label,
        "Research.refute_dark_link",
        "refute_dark_link sketch",
        json!({}),
    );
}

/// `Research.make_inference` — `{ id, premise, conclusion, confidence? }`.
pub(super) fn run_make_inference(document: &Document, label: &str) {
    let premise = first_line(&surface_text(document));
    run_id(
        document,
        label,
        "Research.make_inference",
        "make_inference sketch",
        json!({
            "premise": if premise.is_empty() { "premise".into() } else { premise },
            "conclusion": "follows",
            "confidence": 0.5,
        }),
    );
}

/// `Research.chain_inference` — `{ id, conclusion, confidence? }`.
pub(super) fn run_chain_inference(document: &Document, label: &str) {
    let conclusion = first_line(&surface_text(document));
    run_id(
        document,
        label,
        "Research.chain_inference",
        "chain_inference sketch",
        json!({
            "conclusion": if conclusion.is_empty() { "follows".into() } else { conclusion },
            "confidence": 0.5,
        }),
    );
}

/// `Research.set_inference_confidence` — `{ id, confidence }`.
pub(super) fn run_set_inference_confidence(document: &Document, label: &str) {
    run_id(
        document,
        label,
        "Research.set_inference_confidence",
        "set_inference_confidence sketch",
        json!({ "confidence": 0.75 }),
    );
}

/// `Research.validate_inference` — `{ id }`.
pub(super) fn run_validate_inference(document: &Document, label: &str) {
    run_id(
        document,
        label,
        "Research.validate_inference",
        "validate_inference sketch",
        json!({}),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wave27_research_defaults() {
        assert_eq!(DEFAULT_ID, "enquiry-1");
        assert_eq!(first_line("  hello\nworld"), "hello");
    }
}
