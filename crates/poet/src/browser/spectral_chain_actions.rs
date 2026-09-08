//! Dual-path Tool Chest actions for curated Host-bound `Spectral.*` ids (wave 24).

use serde_json::json;
use web_sys::{Document, Element};

const SPD_SAMPLES: usize = 41;

fn selected_container(document: &Document) -> Option<Element> {
    document
        .query_selector(".canvas-container-node.selected")
        .ok()
        .flatten()
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

fn local_emf(container: Option<&Element>) -> (f64, f64, f64) {
    (
        numeric_attr(container, "data-alpha").unwrap_or(1.0),
        numeric_attr(container, "data-mu").unwrap_or(0.0),
        numeric_attr(container, "data-sigma").filter(|v| *v > 0.0).unwrap_or(0.5),
    )
}

fn local_xyz(container: Option<&Element>) -> (f64, f64, f64) {
    (
        numeric_attr(container, "data-x").unwrap_or(0.0),
        numeric_attr(container, "data-y").unwrap_or(1.0),
        numeric_attr(container, "data-z").unwrap_or(1.0),
    )
}

fn local_blend(container: Option<&Element>) -> (f64, f64, f64, f64, f64, f64, f64) {
    (
        numeric_attr(container, "data-alpha-a").unwrap_or(1.0),
        numeric_attr(container, "data-mu-a").unwrap_or(0.0),
        numeric_attr(container, "data-sigma-a").unwrap_or(0.2),
        numeric_attr(container, "data-alpha-b").unwrap_or(1.0),
        numeric_attr(container, "data-mu-b").unwrap_or(0.0),
        numeric_attr(container, "data-sigma-b").unwrap_or(0.8),
        numeric_attr(container, "data-t").unwrap_or(0.5),
    )
}

/// `Spectral.emf_to_spd` — `{ alpha, mu?, sigma? }`.
pub(super) fn run_emf_to_spd(document: &Document, label: &str) {
    let (alpha, mu, sigma) = local_emf(selected_container(document).as_ref());
    invoke_dual(
        document,
        label,
        "Spectral.emf_to_spd",
        format!("spectral emf_to_spd sketch alpha={alpha} mu={mu} sigma={sigma} n={SPD_SAMPLES}"),
        json!({ "alpha": alpha, "mu": mu, "sigma": sigma }),
    );
}

/// `Spectral.spd_to_xyz` — `{ spd: [f64; 41] }`.
pub(super) fn run_spd_to_xyz(document: &Document, label: &str) {
    let spd = vec![0.0; SPD_SAMPLES];
    invoke_dual(
        document,
        label,
        "Spectral.spd_to_xyz",
        format!("spectral spd_to_xyz sketch n={SPD_SAMPLES} xyz≈[0,0,0]"),
        json!({ "spd": spd }),
    );
}

/// `Spectral.emf_to_rgb` — `{ alpha, mu?, sigma? }`.
pub(super) fn run_emf_to_rgb(document: &Document, label: &str) {
    let (alpha, mu, sigma) = local_emf(selected_container(document).as_ref());
    invoke_dual(
        document,
        label,
        "Spectral.emf_to_rgb",
        format!("spectral emf_to_rgb sketch alpha={alpha} mu={mu} sigma={sigma}"),
        json!({ "alpha": alpha, "mu": mu, "sigma": sigma }),
    );
}

/// `Spectral.blend` — two EMF payloads + t.
pub(super) fn run_blend(document: &Document, label: &str) {
    let (alpha_a, mu_a, sigma_a, alpha_b, mu_b, sigma_b, t) =
        local_blend(selected_container(document).as_ref());
    invoke_dual(
        document,
        label,
        "Spectral.blend",
        format!("spectral blend sketch t={t}"),
        json!({
            "alpha_a": alpha_a,
            "mu_a": mu_a,
            "sigma_a": sigma_a,
            "alpha_b": alpha_b,
            "mu_b": mu_b,
            "sigma_b": sigma_b,
            "t": t
        }),
    );
}

/// `Spectral.gamut_map` — `{ x, y?, z? }`.
pub(super) fn run_gamut_map(document: &Document, label: &str) {
    let (x, y, z) = local_xyz(selected_container(document).as_ref());
    invoke_dual(
        document,
        label,
        "Spectral.gamut_map",
        format!("spectral gamut_map sketch x={x} y={y} z={z}"),
        json!({ "x": x, "y": y, "z": z }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_spectral_wave24_defaults() {
        let (a, mu, s) = local_emf(None);
        assert!((a - 1.0).abs() < 1e-12);
        assert!((mu - 0.0).abs() < 1e-12);
        assert!((s - 0.5).abs() < 1e-12);
        let (x, y, z) = local_xyz(None);
        assert_eq!((x, y, z), (0.0, 1.0, 1.0));
        let t = local_blend(None).6;
        assert!((t - 0.5).abs() < 1e-12);
        assert_eq!(SPD_SAMPLES, 41);
    }
}
