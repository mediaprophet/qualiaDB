//! Dual-path ribbon actions for logic ALL_BOUND ids already in `live_args`.
//!
//! Kept out of `chain_actions.rs` so that file does not grow further.

use web_sys::{Document, Element};

fn selected_container(document: &Document) -> Option<Element> {
    document
        .query_selector(".canvas-container-node.selected")
        .ok()
        .flatten()
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

fn numeric_attr(el: Option<&Element>, name: &str) -> Option<f64> {
    el.and_then(|e| e.get_attribute(name))
        .and_then(|v| v.parse::<f64>().ok())
}

/// `ParaconsistentLogic.route` — args mirror `live_args::epistemic_args`.
pub(super) fn run_paraconsistent_route(document: &Document, label: &str) {
    let container = selected_container(document);
    let agent = container
        .as_ref()
        .and_then(|c| c.get_attribute("data-agent-did"))
        .unwrap_or_else(|| "did:q42:agent:default".into());
    let world = container
        .as_ref()
        .and_then(|c| c.get_attribute("data-epistemic-world"))
        .unwrap_or_else(|| "did:q42:world:actual".into());
    let certainty = numeric_attr(container.as_ref(), "data-certainty").unwrap_or(1.0);
    let label = label.to_string();
    if !super::native_daemon::is_daemon_connected() {
        let report = super::tool_dual_path::local_sketch(
            "ParaconsistentLogic.route",
            &format!(
                "Local contradiction-routing sketch for agent {agent} in {world} (certainty {certainty}). Connect QualiaDB for a live route."
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
        "Running ParaconsistentLogic.route…",
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
        match super::native_daemon::daemon_invoke("ParaconsistentLogic.route", args).await {
            Ok(response) if response.ok => {
                let report =
                    super::tool_dual_path::live_ok("ParaconsistentLogic.route", &response.value);
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Ok(response) => {
                let report = super::tool_dual_path::live_denied(
                    "ParaconsistentLogic.route",
                    response
                        .diagnostic
                        .as_deref()
                        .unwrap_or("ParaconsistentLogic.route failed."),
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
                    super::tool_dual_path::live_denied("ParaconsistentLogic.route", &error);
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

/// `TemporalAndDescriptionLogic.ltl.evaluate` — args mirror `live_args` LTL row.
pub(super) fn run_ltl_evaluate(document: &Document, label: &str) {
    let container = selected_container(document);
    let property = numeric_attr(container.as_ref(), "data-property-hash").unwrap_or(0.0) as u64;
    let label = label.to_string();
    if !super::native_daemon::is_daemon_connected() {
        let report = super::tool_dual_path::local_sketch(
            "TemporalAndDescriptionLogic.ltl.evaluate",
            &format!(
                "Local LTL Globally sketch for property hash {property}. Connect QualiaDB for a live trace check."
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
        "Running TemporalAndDescriptionLogic.ltl.evaluate…",
        "running",
    );
    wasm_bindgen_futures::spawn_local(async move {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        let args = serde_json::json!({
            "formula": "Globally",
            "property": property,
        });
        match super::native_daemon::daemon_invoke(
            "TemporalAndDescriptionLogic.ltl.evaluate",
            args,
        )
        .await
        {
            Ok(response) if response.ok => {
                let report = super::tool_dual_path::live_ok(
                    "TemporalAndDescriptionLogic.ltl.evaluate",
                    &response.value,
                );
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Ok(response) => {
                let report = super::tool_dual_path::live_denied(
                    "TemporalAndDescriptionLogic.ltl.evaluate",
                    response
                        .diagnostic
                        .as_deref()
                        .unwrap_or("TemporalAndDescriptionLogic.ltl.evaluate failed."),
                );
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Err(error) => {
                let report = super::tool_dual_path::live_denied(
                    "TemporalAndDescriptionLogic.ltl.evaluate",
                    &error,
                );
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

/// `SymbolicAlgebra.eval` — args mirror `live_args::symbolic_args` (`data-formula`).
pub(super) fn run_symbolic_eval(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with a formula before evaluating.",
    ) else {
        return;
    };
    let expr = container
        .get_attribute("data-formula")
        .or_else(|| container.text_content())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "x^2 + 2*x + 1".into());
    let expr = expr.trim().to_string();
    let label = label.to_string();
    if !super::native_daemon::is_daemon_connected() {
        let report = super::tool_dual_path::local_sketch(
            "SymbolicAlgebra.eval",
            &format!(
                "Local formula sketch ({expr}). Connect QualiaDB for a live SymbolicAlgebra.eval."
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
        "Running SymbolicAlgebra.eval…",
        "running",
    );
    wasm_bindgen_futures::spawn_local(async move {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        let args = serde_json::json!({ "expr": expr });
        match super::native_daemon::daemon_invoke("SymbolicAlgebra.eval", args).await {
            Ok(response) if response.ok => {
                let report =
                    super::tool_dual_path::live_ok("SymbolicAlgebra.eval", &response.value);
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Ok(response) => {
                let report = super::tool_dual_path::live_denied(
                    "SymbolicAlgebra.eval",
                    response
                        .diagnostic
                        .as_deref()
                        .unwrap_or("SymbolicAlgebra.eval failed."),
                );
                super::interactions::show_tool_status(
                    &document,
                    &label,
                    &report.message,
                    report.status_kind,
                );
            }
            Err(error) => {
                let report = super::tool_dual_path::live_denied("SymbolicAlgebra.eval", &error);
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
