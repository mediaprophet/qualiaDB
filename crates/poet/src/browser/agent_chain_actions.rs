//! Dual-path Tool Chest actions for curated Host-bound `Agent.*` ids (wave 25).

use serde_json::json;
use web_sys::{Document, Element};

const DEFAULT_TASK: &str = "inspect graph";
const DEFAULT_INSTRUMENT: &str = "poet";

fn selected_container(document: &Document) -> Option<Element> {
    document
        .query_selector(".canvas-container-node.selected")
        .ok()
        .flatten()
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

fn local_id(from_attr: Option<&str>, default: &str) -> String {
    from_attr
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(default)
        .to_string()
}

/// `Agent.trace` — `{ instrument_id }`.
pub(super) fn run_trace(document: &Document, label: &str) {
    let instrument_id = local_id(
        string_attr(selected_container(document).as_ref(), "data-instrument-id").as_deref(),
        DEFAULT_INSTRUMENT,
    );
    invoke_dual(
        document,
        label,
        "Agent.trace",
        format!("agent trace sketch instrument_id={instrument_id} status=ledger_ready"),
        json!({ "instrument_id": instrument_id }),
    );
}

/// `Agent.verify` — `{ windowed_faults?, usury_event? }`.
pub(super) fn run_verify(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Agent.verify",
        "agent verify sketch windowed_faults=0 usury_event=false".into(),
        json!({ "windowed_faults": 0, "usury_event": false }),
    );
}

/// `Agent.plan` — `{ task, capabilities? }`.
pub(super) fn run_plan(document: &Document, label: &str) {
    let task = local_id(
        string_attr(selected_container(document).as_ref(), "data-task").as_deref(),
        DEFAULT_TASK,
    );
    invoke_dual(
        document,
        label,
        "Agent.plan",
        format!("agent plan sketch task={task} capabilities=[]"),
        json!({ "task": task, "capabilities": [] }),
    );
}

/// `Agent.execute` — `{ task, capabilities? }`.
pub(super) fn run_execute(document: &Document, label: &str) {
    let task = local_id(
        string_attr(selected_container(document).as_ref(), "data-task").as_deref(),
        DEFAULT_TASK,
    );
    invoke_dual(
        document,
        label,
        "Agent.execute",
        format!("agent execute sketch task={task} status=planned"),
        json!({ "task": task, "capabilities": [] }),
    );
}

/// `Agent.evaluate` — `{ expected, outputs, method? }`.
pub(super) fn run_evaluate(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Agent.evaluate",
        "agent evaluate sketch expected=[ok] outputs=[ok] method=exact".into(),
        json!({ "expected": ["ok"], "outputs": ["ok"], "method": "exact" }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_agent_wave25_defaults() {
        assert_eq!(local_id(None, DEFAULT_TASK), DEFAULT_TASK);
        assert_eq!(DEFAULT_INSTRUMENT, "poet");
    }
}
