//! Dual-path Tool Chest actions for curated Host-bound `Pulse.*` ids (wave 26).
//!
//! Distinct from spec Live contracts (`Pulse.publish` / `Pulse.publish_presence`)
//! and from `comm:pulse_presence`. Host args use `{ channel, payload_type? }`.

use serde_json::json;
use web_sys::{Document, Element};

const DEFAULT_CHANNEL: &str = "poet/social";
const DEFAULT_TRANSPORT: &str = "sse";

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

fn resolve_channel(document: &Document) -> String {
    string_attr(selected_container(document).as_ref(), "data-pulse-channel")
        .unwrap_or_else(|| DEFAULT_CHANNEL.to_string())
}

fn publish(document: &Document, label: &str, cap_id: &'static str, payload_type: &str) {
    let channel = resolve_channel(document);
    invoke_dual(
        document,
        label,
        cap_id,
        format!("pulse {payload_type} sketch channel={channel}"),
        json!({ "channel": channel, "payload_type": payload_type }),
    );
}

/// `Pulse.publish` — `{ channel, payload_type? }`.
pub(super) fn run_publish(document: &Document, label: &str) {
    publish(document, label, "Pulse.publish", "generic");
}

/// `Pulse.publish_graph_mutation`.
pub(super) fn run_publish_graph_mutation(document: &Document, label: &str) {
    publish(document, label, "Pulse.publish_graph_mutation", "graph-mutation");
}

/// `Pulse.publish_notification`.
pub(super) fn run_publish_notification(document: &Document, label: &str) {
    publish(document, label, "Pulse.publish_notification", "notification");
}

/// `Pulse.publish_telemetry`.
pub(super) fn run_publish_telemetry(document: &Document, label: &str) {
    publish(document, label, "Pulse.publish_telemetry", "telemetry");
}

/// `Pulse.publish_agent_message`.
pub(super) fn run_publish_agent_message(document: &Document, label: &str) {
    publish(document, label, "Pulse.publish_agent_message", "agent-message");
}

/// `Pulse.publish_sync`.
pub(super) fn run_publish_sync(document: &Document, label: &str) {
    publish(document, label, "Pulse.publish_sync", "sync");
}

/// `Pulse.open_channel` — `{ channel, channel_type? }`.
pub(super) fn run_open_channel(document: &Document, label: &str) {
    let channel = resolve_channel(document);
    invoke_dual(
        document,
        label,
        "Pulse.open_channel",
        format!("pulse open_channel sketch channel={channel} type=topic"),
        json!({ "channel": channel, "channel_type": "topic" }),
    );
}

/// `Pulse.close_channel` — `{ channel }`.
pub(super) fn run_close_channel(document: &Document, label: &str) {
    let channel = resolve_channel(document);
    invoke_dual(
        document,
        label,
        "Pulse.close_channel",
        format!("pulse close_channel sketch channel={channel}"),
        json!({ "channel": channel }),
    );
}

/// `Pulse.set_transport` — `{ channel, transport }`.
pub(super) fn run_set_transport(document: &Document, label: &str) {
    let channel = resolve_channel(document);
    invoke_dual(
        document,
        label,
        "Pulse.set_transport",
        format!("pulse set_transport sketch channel={channel} transport={DEFAULT_TRANSPORT}"),
        json!({ "channel": channel, "transport": DEFAULT_TRANSPORT }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_pulse_wave26_defaults() {
        assert_eq!(DEFAULT_CHANNEL, "poet/social");
        assert_eq!(DEFAULT_TRANSPORT, "sse");
    }
}
