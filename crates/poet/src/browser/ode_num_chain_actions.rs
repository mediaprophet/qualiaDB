//! Dual-path Tool Chest actions for Host-bound numeric `Ode.*` ids.

use serde_json::json;
use web_sys::{Document, Element};

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

fn numeric_attr(el: Option<&Element>, name: &str) -> Option<f64> {
    el.and_then(|e| e.get_attribute(name))
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|v| v.is_finite())
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

fn el(document: &Document) -> Option<Element> {
    selected_container(document)
}

fn scalar_ivp(document: &Document) -> serde_json::Value {
    let dt = numeric_attr(el(document).as_ref(), "data-dt").unwrap_or(0.1);
    json!({
        "system": ["-y"],
        "vars": ["y"],
        "t_span": [0.0, 1.0],
        "y0": [1.0],
        "dt": dt,
    })
}

pub(super) fn run_rk4(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Ode.rk4_integrate",
        "rk4_integrate sketch y'=-y, y(0)=1".to_string(),
        scalar_ivp(document),
    );
}

pub(super) fn run_dopri5(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Ode.dopri5",
        "dopri5 sketch y'=-y, y(0)=1".to_string(),
        scalar_ivp(document),
    );
}

pub(super) fn run_bdf(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Ode.bdf",
        "bdf sketch y'=-y, y(0)=1".to_string(),
        scalar_ivp(document),
    );
}

pub(super) fn run_symplectic_step(document: &Document, label: &str) {
    let dt = numeric_attr(el(document).as_ref(), "data-dt").unwrap_or(0.016);
    let method = string_attr(el(document).as_ref(), "data-method").unwrap_or_else(|| "verlet".into());
    invoke_dual(
        document,
        label,
        "Ode.symplectic_step",
        format!("symplectic_step sketch method={method} dt={dt}"),
        json!({
            "force": "-1.0 * q",
            "q": 1.0,
            "p": 0.0,
            "dt": dt,
            "method": method,
        }),
    );
}
