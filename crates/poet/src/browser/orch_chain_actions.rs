//! Dual-path Tool Chest actions for curated `Orchestration.*` ALL_BOUND ids (wave 20).
//!
//! No Host widen — scopes must already exist in `poet_host/invoke/ids.rs`.
//! Session lifecycle is Host-authoritative; offline path is a local sketch only.

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
        .filter(|s| !s.trim().is_empty())
}

fn tokens(source: &str) -> Vec<String> {
    source
        .split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ';' | '|' | '\n' | '\r'))
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(|t| t.chars().take(128).collect())
        .take(16)
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

fn resolve_session_id(container: Option<&Element>, toks: &[String]) -> String {
    string_attr(container, "data-session-id")
        .or_else(|| string_attr(container, "data-id"))
        .or_else(|| toks.first().cloned())
        .unwrap_or_else(|| "orch-demo".into())
}

fn resolve_task(container: Option<&Element>, toks: &[String], source: &str) -> String {
    string_attr(container, "data-task")
        .or_else(|| {
            if toks.len() >= 2 {
                Some(toks[1..].join(" "))
            } else {
                let trimmed = source.trim();
                (!trimmed.is_empty()).then(|| trimmed.chars().take(256).collect())
            }
        })
        .unwrap_or_else(|| "demo task".into())
}

/// `Orchestration.session_create` — `{ id, task }`.
pub(super) fn run_session_create(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let toks = tokens(&source);
    let id = resolve_session_id(container.as_ref(), &toks);
    let task = resolve_task(container.as_ref(), &toks, &source);
    invoke_dual(
        document,
        label,
        "Orchestration.session_create",
        format!("Orchestration session sketch created id={id}"),
        json!({ "id": id, "task": task }),
    );
}

/// `Orchestration.session_plan` — `{ id }`.
pub(super) fn run_session_plan(document: &Document, label: &str) {
    let container = selected_container(document);
    let toks = tokens(&selected_source(document).unwrap_or_default());
    let id = resolve_session_id(container.as_ref(), &toks);
    invoke_dual(
        document,
        label,
        "Orchestration.session_plan",
        format!("Orchestration plan sketch for id={id}"),
        json!({ "id": id }),
    );
}

/// `Orchestration.session_execute` — `{ id }`.
pub(super) fn run_session_execute(document: &Document, label: &str) {
    let container = selected_container(document);
    let toks = tokens(&selected_source(document).unwrap_or_default());
    let id = resolve_session_id(container.as_ref(), &toks);
    invoke_dual(
        document,
        label,
        "Orchestration.session_execute",
        format!("Orchestration execute sketch for id={id}"),
        json!({ "id": id }),
    );
}

/// `Orchestration.session_status` — `{ id }`.
pub(super) fn run_session_status(document: &Document, label: &str) {
    let container = selected_container(document);
    let toks = tokens(&selected_source(document).unwrap_or_default());
    let id = resolve_session_id(container.as_ref(), &toks);
    invoke_dual(
        document,
        label,
        "Orchestration.session_status",
        format!("Orchestration status sketch for id={id} → created"),
        json!({ "id": id }),
    );
}

/// `Orchestration.roster_register` — `{ session_id, agent_id, did, name, role? }`.
pub(super) fn run_roster_register(document: &Document, label: &str) {
    let container = selected_container(document);
    let toks = tokens(&selected_source(document).unwrap_or_default());
    let session_id = resolve_session_id(container.as_ref(), &toks);
    let agent_id = string_attr(container.as_ref(), "data-agent-id")
        .or_else(|| toks.get(1).cloned())
        .unwrap_or_else(|| "agent-1".into());
    let did = string_attr(container.as_ref(), "data-did")
        .or_else(|| toks.get(2).cloned())
        .unwrap_or_else(|| "did:q42:demo".into());
    let name = string_attr(container.as_ref(), "data-name")
        .or_else(|| toks.get(3).cloned())
        .unwrap_or_else(|| "Demo Agent".into());
    let role = string_attr(container.as_ref(), "data-role")
        .or_else(|| toks.get(4).cloned())
        .unwrap_or_else(|| "researcher".into());
    invoke_dual(
        document,
        label,
        "Orchestration.roster_register",
        format!("Roster register sketch session={session_id} agent={agent_id}"),
        json!({
            "session_id": session_id,
            "agent_id": agent_id,
            "did": did,
            "name": name,
            "role": role,
        }),
    );
}

/// `Orchestration.roster_list` — `{ session_id }`.
pub(super) fn run_roster_list(document: &Document, label: &str) {
    let container = selected_container(document);
    let toks = tokens(&selected_source(document).unwrap_or_default());
    let session_id = resolve_session_id(container.as_ref(), &toks);
    invoke_dual(
        document,
        label,
        "Orchestration.roster_list",
        format!("Roster list sketch session={session_id} → count=0"),
        json!({ "session_id": session_id }),
    );
}

/// `Orchestration.roster_capabilities` — `{ session_id }`.
pub(super) fn run_roster_capabilities(document: &Document, label: &str) {
    let container = selected_container(document);
    let toks = tokens(&selected_source(document).unwrap_or_default());
    let session_id = resolve_session_id(container.as_ref(), &toks);
    invoke_dual(
        document,
        label,
        "Orchestration.roster_capabilities",
        format!("Roster capabilities sketch session={session_id}"),
        json!({ "session_id": session_id }),
    );
}

/// `Orchestration.assign_agents` — `{ session_id }`.
pub(super) fn run_assign_agents(document: &Document, label: &str) {
    let container = selected_container(document);
    let toks = tokens(&selected_source(document).unwrap_or_default());
    let session_id = resolve_session_id(container.as_ref(), &toks);
    invoke_dual(
        document,
        label,
        "Orchestration.assign_agents",
        format!("Assign agents sketch session={session_id} → count=0"),
        json!({ "session_id": session_id }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_orch_wave20_resolve_defaults() {
        let empty: Vec<String> = Vec::new();
        assert_eq!(resolve_session_id(None, &empty), "orch-demo");
        assert_eq!(resolve_task(None, &empty, ""), "demo task");
        let toks = tokens("s1 Research climate impacts");
        assert_eq!(toks.first().map(String::as_str), Some("s1"));
        assert_eq!(
            resolve_task(None, &toks, "s1 Research climate impacts"),
            "Research climate impacts"
        );
    }
}
