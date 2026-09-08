//! Dual-path Tool Chest actions for curated `Constructibility.*` ALL_BOUND ids (wave 15).
//!
//! No Host widen — scopes must already exist in `poet_host/invoke/ids.rs`.

use serde_json::json;
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

fn surface_n(container: &Element) -> u64 {
    numeric_attr(Some(container), "data-n")
        .or_else(|| numeric_attr(Some(container), "data-sides"))
        .filter(|v| v.is_finite() && *v >= 0.0 && *v == v.floor())
        .map(|v| v as u64)
        .unwrap_or(17)
}

fn surface_degree(container: &Element) -> u64 {
    numeric_attr(Some(container), "data-degree")
        .filter(|v| v.is_finite() && *v >= 0.0 && *v == v.floor())
        .map(|v| v as u64)
        .unwrap_or(4)
}

fn surface_formula(container: &Element) -> String {
    container
        .get_attribute("data-formula")
        .or_else(|| container.text_content())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "sqrt(2)".into())
        .trim()
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

fn local_is_power_of_two(n: u64) -> bool {
    n != 0 && (n & (n - 1)) == 0
}

fn local_is_prime(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    if n % 2 == 0 {
        return n == 2;
    }
    let mut d = 3u64;
    while d.saturating_mul(d) <= n {
        if n % d == 0 {
            return false;
        }
        d += 2;
    }
    true
}

fn local_is_fermat_prime(n: u64) -> bool {
    if !local_is_prime(n) || n < 3 {
        return false;
    }
    let m = n - 1;
    if !local_is_power_of_two(m) {
        return false;
    }
    let exp = m.trailing_zeros() as u64;
    local_is_power_of_two(exp)
}

fn local_is_regular_polygon_constructible(n: u64) -> bool {
    if n < 3 {
        return false;
    }
    let mut odd = n;
    while odd % 2 == 0 {
        odd /= 2;
    }
    if odd == 1 {
        return true;
    }
    let mut p = 3u64;
    let mut rem = odd;
    while p.saturating_mul(p) <= rem {
        if rem % p == 0 {
            if !local_is_fermat_prime(p) {
                return false;
            }
            rem /= p;
            if rem % p == 0 {
                return false;
            }
        }
        p += 2;
    }
    rem == 1 || local_is_fermat_prime(rem)
}

/// `Constructibility.is_regular_polygon_constructible` — `{ n }`.
pub(super) fn run_constr_regular_polygon(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with data-n before testing a regular polygon.",
    ) else {
        return;
    };
    let n = surface_n(&container);
    let value = local_is_regular_polygon_constructible(n);
    invoke_dual(
        document,
        label,
        "Constructibility.is_regular_polygon_constructible",
        format!(
            "Local regular-polygon sketch: n={n} → {value}. Connect QualiaDB for a live Gauss–Wantzel check."
        ),
        json!({ "n": n }),
    );
}

/// `Constructibility.is_fermat_prime` — `{ n }`.
pub(super) fn run_constr_fermat_prime(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with data-n before testing a Fermat prime.",
    ) else {
        return;
    };
    let n = surface_n(&container);
    let value = local_is_fermat_prime(n);
    invoke_dual(
        document,
        label,
        "Constructibility.is_fermat_prime",
        format!(
            "Local Fermat-prime sketch: n={n} → {value}. Connect QualiaDB for a live is_fermat_prime."
        ),
        json!({ "n": n }),
    );
}

/// `Constructibility.constructible_from_min_poly_degree` — `{ degree }`.
pub(super) fn run_constr_min_poly_degree(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with data-degree before a Wantzel degree gate.",
    ) else {
        return;
    };
    let degree = surface_degree(&container);
    let value = local_is_power_of_two(degree);
    invoke_dual(
        document,
        label,
        "Constructibility.constructible_from_min_poly_degree",
        format!(
            "Local min-poly-degree sketch: degree={degree} → {value}. Connect QualiaDB for a live Wantzel gate."
        ),
        json!({ "degree": degree }),
    );
}

/// `Constructibility.is_power_of_two` — `{ n }`.
pub(super) fn run_constr_power_of_two(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with data-n before testing a power of two.",
    ) else {
        return;
    };
    let n = surface_n(&container);
    let value = local_is_power_of_two(n);
    invoke_dual(
        document,
        label,
        "Constructibility.is_power_of_two",
        format!(
            "Local power-of-two sketch: n={n} → {value}. Connect QualiaDB for a live is_power_of_two."
        ),
        json!({ "n": n }),
    );
}

/// `Constructibility.is_central_angle_constructible` — `{ n }`.
pub(super) fn run_constr_central_angle(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with data-n before testing a central angle.",
    ) else {
        return;
    };
    let n = surface_n(&container);
    let value = local_is_regular_polygon_constructible(n);
    invoke_dual(
        document,
        label,
        "Constructibility.is_central_angle_constructible",
        format!(
            "Local central-angle sketch: 2π/{n} → {value}. Connect QualiaDB for a live constructibility check."
        ),
        json!({ "n": n }),
    );
}

/// `Constructibility.doubling_the_cube_constructible` — `{}`.
pub(super) fn run_constr_doubling_cube(document: &Document, label: &str) {
    if need_container(
        document,
        label,
        "Select a surface before asking about doubling the cube.",
    )
    .is_none()
    {
        return;
    }
    let value = local_is_power_of_two(3);
    invoke_dual(
        document,
        label,
        "Constructibility.doubling_the_cube_constructible",
        format!(
            "Local doubling-the-cube sketch → {value} (degree 3). Connect QualiaDB for a live classical verdict."
        ),
        json!({}),
    );
}

/// `Constructibility.trisecting_general_angle_constructible` — `{}`.
pub(super) fn run_constr_trisect_angle(document: &Document, label: &str) {
    if need_container(
        document,
        label,
        "Select a surface before asking about trisecting a general angle.",
    )
    .is_none()
    {
        return;
    }
    let value = local_is_power_of_two(3);
    invoke_dual(
        document,
        label,
        "Constructibility.trisecting_general_angle_constructible",
        format!(
            "Local angle-trisection sketch → {value} (degree 3). Connect QualiaDB for a live classical verdict."
        ),
        json!({}),
    );
}

/// `Constructibility.squaring_the_circle_constructible` — `{}`.
pub(super) fn run_constr_square_circle(document: &Document, label: &str) {
    if need_container(
        document,
        label,
        "Select a surface before asking about squaring the circle.",
    )
    .is_none()
    {
        return;
    }
    let value = false;
    invoke_dual(
        document,
        label,
        "Constructibility.squaring_the_circle_constructible",
        format!(
            "Local squaring-the-circle sketch → {value} (π transcendental). Connect QualiaDB for a live classical verdict."
        ),
        json!({}),
    );
}

/// `Constructibility.is_constructible_number` — `{ expr }`.
pub(super) fn run_constr_number(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with a formula before testing constructibility.",
    ) else {
        return;
    };
    let expr = surface_formula(&container);
    invoke_dual(
        document,
        label,
        "Constructibility.is_constructible_number",
        format!(
            "Local constructible-number sketch of ({expr}). Connect QualiaDB for a live CAS verdict."
        ),
        json!({ "expr": expr }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_gauss_wantzel_heptadecagon() {
        assert!(local_is_regular_polygon_constructible(17));
        assert!(!local_is_regular_polygon_constructible(7));
        assert!(local_is_fermat_prime(17));
        assert!(!local_is_fermat_prime(9));
        assert!(local_is_power_of_two(8));
        assert!(!local_is_power_of_two(12));
    }
}
