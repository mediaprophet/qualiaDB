//! Dual-path Tool Chest actions for curated Host-bound `SymbolicODE.*` ids (wave 25).

use serde_json::json;
use web_sys::Document;

const DEFAULT_VAR: &str = "x";

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

/// `SymbolicODE.solve_linear_first_order` — `{ a, b, var? }`.
pub(super) fn run_lin1(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "SymbolicODE.solve_linear_first_order",
        "ode lin1 sketch y' + 0·y = 1 var=x".into(),
        json!({ "a": 0.0, "b": 1.0, "var": DEFAULT_VAR }),
    );
}

/// `SymbolicODE.solve_linear_second_order` — `{ a, b, c, var? }`.
pub(super) fn run_lin2(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "SymbolicODE.solve_linear_second_order",
        "ode lin2 sketch y'' + y = 0".into(),
        json!({ "a": 1.0, "b": 0.0, "c": 1.0, "var": DEFAULT_VAR }),
    );
}

/// `SymbolicODE.classify_second_order_pde` — `{ a_xx, b_xy, c_yy }`.
pub(super) fn run_classify_pde(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "SymbolicODE.classify_second_order_pde",
        "ode classify_pde sketch a_xx=1 b_xy=0 c_yy=1 (elliptic)".into(),
        json!({ "a_xx": 1.0, "b_xy": 0.0, "c_yy": 1.0 }),
    );
}

/// `SymbolicODE.solve_separable` — `{ g, h, x?, y? }`.
pub(super) fn run_separable(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "SymbolicODE.solve_separable",
        "ode separable sketch g=1 h=1".into(),
        json!({ "g": "1", "h": "1", "x": "x", "y": "y" }),
    );
}

/// `SymbolicODE.solve_first_order_linear_pde` — `{ a, b, x?, y? }`.
pub(super) fn run_pde1(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "SymbolicODE.solve_first_order_linear_pde",
        "ode pde1 sketch a=1 b=1".into(),
        json!({ "a": 1.0, "b": 1.0, "x": "x", "y": "y" }),
    );
}

#[cfg(test)]
mod tests {
    #[test]
    fn local_ode_wave25_defaults() {
        assert_eq!(super::DEFAULT_VAR, "x");
    }
}
