//! Dual-path Tool Chest actions for curated Host-bound `VectorCalculus.*` ids (wave 24).
//! Local sketches use Host unwrap_or defaults; no Host widen.

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

fn local_expr(from_attr: Option<&str>) -> String {
    from_attr
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("x^2 + y^2")
        .to_string()
}

fn local_vars2() -> Vec<&'static str> {
    vec!["x", "y"]
}

fn local_vars3() -> Vec<&'static str> {
    vec!["x", "y", "z"]
}

fn local_t_span() -> [f64; 2] {
    [0.0, 1.0]
}

/// `VectorCalculus.gradient` — `{ expr, vars }`.
pub(super) fn run_gradient(document: &Document, label: &str) {
    let expr = local_expr(string_attr(selected_container(document).as_ref(), "data-expr").as_deref());
    let vars = local_vars2();
    invoke_dual(
        document,
        label,
        "VectorCalculus.gradient",
        format!("vc gradient sketch expr={expr} vars={vars:?}"),
        json!({ "expr": expr, "vars": vars }),
    );
}

/// `VectorCalculus.divergence` — `{ field, vars }`.
pub(super) fn run_divergence(document: &Document, label: &str) {
    let field = vec!["x", "y"];
    let vars = local_vars2();
    invoke_dual(
        document,
        label,
        "VectorCalculus.divergence",
        format!("vc divergence sketch field={field:?} vars={vars:?}"),
        json!({ "field": field, "vars": vars }),
    );
}

/// `VectorCalculus.curl` — `{ field, vars }` (3 components).
pub(super) fn run_curl(document: &Document, label: &str) {
    let field = vec!["-y", "x", "0"];
    let vars = local_vars3();
    invoke_dual(
        document,
        label,
        "VectorCalculus.curl",
        format!("vc curl sketch field={field:?} vars={vars:?}"),
        json!({ "field": field, "vars": vars }),
    );
}

/// `VectorCalculus.laplacian` — `{ expr, vars }`.
pub(super) fn run_laplacian(document: &Document, label: &str) {
    let expr = local_expr(string_attr(selected_container(document).as_ref(), "data-expr").as_deref());
    let vars = local_vars2();
    invoke_dual(
        document,
        label,
        "VectorCalculus.laplacian",
        format!("vc laplacian sketch expr={expr} vars={vars:?}"),
        json!({ "expr": expr, "vars": vars }),
    );
}

/// `VectorCalculus.line_integral_scalar` — `{ expr, curve, t_span, steps? }`.
pub(super) fn run_line_integral_scalar(document: &Document, label: &str) {
    let expr = "1";
    let curve = vec!["t", "0"];
    let t_span = local_t_span();
    invoke_dual(
        document,
        label,
        "VectorCalculus.line_integral_scalar",
        format!("vc line_integral_scalar sketch expr={expr} curve={curve:?} t_span={t_span:?} steps=100"),
        json!({ "expr": expr, "curve": curve, "t_span": t_span, "steps": 100 }),
    );
}

/// `VectorCalculus.line_integral_work` — `{ field, curve, t_span, steps? }`.
pub(super) fn run_line_integral_work(document: &Document, label: &str) {
    let field = vec!["0", "0", "0"];
    let curve = vec!["t", "0", "0"];
    let t_span = local_t_span();
    invoke_dual(
        document,
        label,
        "VectorCalculus.line_integral_work",
        format!("vc line_integral_work sketch field={field:?} curve={curve:?} t_span={t_span:?} steps=100"),
        json!({ "field": field, "curve": curve, "t_span": t_span, "steps": 100 }),
    );
}

/// `VectorCalculus.surface_flux` — `{ field, surface, u_span, v_span, steps? }`.
pub(super) fn run_surface_flux(document: &Document, label: &str) {
    let field = vec!["0", "0", "1"];
    let surface = vec!["u", "v", "0"];
    invoke_dual(
        document,
        label,
        "VectorCalculus.surface_flux",
        "vc surface_flux sketch field=[0,0,1] surface=[u,v,0] u_span=[0,1] v_span=[0,1] steps=50".into(),
        json!({
            "field": field,
            "surface": surface,
            "u_span": [0.0, 1.0],
            "v_span": [0.0, 1.0],
            "steps": 50
        }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_vc_wave24_defaults() {
        assert_eq!(local_expr(None), "x^2 + y^2");
        assert_eq!(local_expr(Some("x")), "x");
        assert_eq!(local_vars2(), vec!["x", "y"]);
        assert_eq!(local_vars3().len(), 3);
        assert_eq!(local_t_span(), [0.0, 1.0]);
    }
}
