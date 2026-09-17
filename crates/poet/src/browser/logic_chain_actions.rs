//! Dual-path ribbon actions for logic / CAS ALL_BOUND ids.
//!
//! Kept out of `chain_actions.rs` so that file does not grow further.
//! No Host widen — scopes must already exist in `poet_host/invoke/ids.rs`.

use serde_json::{json, Value};
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

fn surface_formula(container: &Element) -> String {
    container
        .get_attribute("data-formula")
        .or_else(|| container.text_content())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "x^2 + 2*x + 1".into())
        .trim()
        .to_string()
}

fn surface_var(container: &Element) -> String {
    container
        .get_attribute("data-var")
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "x".into())
}

fn invoke_dual(
    document: &Document,
    label: &str,
    cap_id: &'static str,
    local_message: String,
    args: Value,
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
                let fallback = format!("{cap_id} failed.");
                let report = super::tool_dual_path::live_denied(
                    cap_id,
                    response.diagnostic.as_deref().unwrap_or(&fallback),
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
    invoke_dual(
        document,
        label,
        "ParaconsistentLogic.route",
        format!(
            "Local contradiction-routing sketch for agent {agent} in {world} (certainty {certainty}). Connect QualiaDB for a live route."
        ),
        json!({
            "agent": agent,
            "world": world,
            "certainty": certainty,
        }),
    );
}

/// `TemporalAndDescriptionLogic.ltl.evaluate` — args mirror `live_args` LTL row.
pub(super) fn run_ltl_evaluate(document: &Document, label: &str) {
    let container = selected_container(document);
    let property = numeric_attr(container.as_ref(), "data-property-hash").unwrap_or(0.0) as u64;
    invoke_dual(
        document,
        label,
        "TemporalAndDescriptionLogic.ltl.evaluate",
        format!(
            "Local LTL Globally sketch for property hash {property}. Connect QualiaDB for a live trace check."
        ),
        json!({
            "formula": "Globally",
            "property": property,
        }),
    );
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
    let expr = surface_formula(&container);
    invoke_dual(
        document,
        label,
        "SymbolicAlgebra.eval",
        format!(
            "Local formula sketch ({expr}). Connect QualiaDB for a live SymbolicAlgebra.eval."
        ),
        json!({ "expr": expr }),
    );
}

/// `SymbolicAlgebra.differentiate` — `{ expr, var }`.
pub(super) fn run_symbolic_differentiate(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with a formula before differentiating.",
    ) else {
        return;
    };
    let expr = surface_formula(&container);
    let var = surface_var(&container);
    invoke_dual(
        document,
        label,
        "SymbolicAlgebra.differentiate",
        format!("Local d/d{var} sketch of ({expr}). Connect QualiaDB for a live derivative."),
        json!({ "expr": expr, "var": var }),
    );
}

/// `SymbolicAlgebra.simplify` — `{ expr }`.
pub(super) fn run_symbolic_simplify(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with a formula before simplifying.",
    ) else {
        return;
    };
    let expr = surface_formula(&container);
    invoke_dual(
        document,
        label,
        "SymbolicAlgebra.simplify",
        format!("Local simplify sketch of ({expr}). Connect QualiaDB for a live simplify."),
        json!({ "expr": expr }),
    );
}

/// `SymbolicAlgebra.expand` — `{ expr }`.
pub(super) fn run_symbolic_expand(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with a formula before expanding.",
    ) else {
        return;
    };
    let expr = surface_formula(&container);
    invoke_dual(
        document,
        label,
        "SymbolicAlgebra.expand",
        format!("Local expand sketch of ({expr}). Connect QualiaDB for a live expand."),
        json!({ "expr": expr }),
    );
}

/// `SymbolicAlgebra.factor` — `{ a, b, c, var? }` for a real quadratic.
pub(super) fn run_symbolic_factor(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface before factoring a quadratic.",
    ) else {
        return;
    };
    let a = numeric_attr(Some(&container), "data-a").unwrap_or(1.0);
    let b = numeric_attr(Some(&container), "data-b").unwrap_or(-5.0);
    let c = numeric_attr(Some(&container), "data-c").unwrap_or(6.0);
    let var = surface_var(&container);
    invoke_dual(
        document,
        label,
        "SymbolicAlgebra.factor",
        format!("Local factor sketch of {a}{var}^2 + {b}{var} + {c}. Connect QualiaDB for a live factor."),
        json!({ "a": a, "b": b, "c": c, "var": var }),
    );
}

/// `SymbolicAlgebra.integrate` — `{ expr, var? }`.
pub(super) fn run_symbolic_integrate(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with a formula before integrating.",
    ) else {
        return;
    };
    let expr = surface_formula(&container);
    let var = surface_var(&container);
    invoke_dual(
        document,
        label,
        "SymbolicAlgebra.integrate",
        format!("Local ∫ sketch of ({expr}) d{var}. Connect QualiaDB for a live integrate."),
        json!({ "expr": expr, "var": var }),
    );
}

/// `SymbolicAlgebra.simplify_trig` — `{ expr }`.
pub(super) fn run_symbolic_simplify_trig(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with a formula before trig simplify.",
    ) else {
        return;
    };
    let expr = surface_formula(&container);
    invoke_dual(
        document,
        label,
        "SymbolicAlgebra.simplify_trig",
        format!(
            "Local trig-simplify sketch of ({expr}). Connect QualiaDB for a live simplify_trig."
        ),
        json!({ "expr": expr }),
    );
}

/// `SymbolicAlgebra.partial` — `{ expr, var }`.
pub(super) fn run_symbolic_partial(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with a formula before partial derivative.",
    ) else {
        return;
    };
    let expr = surface_formula(&container);
    let var = surface_var(&container);
    invoke_dual(
        document,
        label,
        "SymbolicAlgebra.partial",
        format!("Local ∂/∂{var} sketch of ({expr}). Connect QualiaDB for a live partial."),
        json!({ "expr": expr, "var": var }),
    );
}

/// `SymbolicAlgebra.limit` — `{ expr, a, var? }`.
pub(super) fn run_symbolic_limit(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with a formula before taking a limit.",
    ) else {
        return;
    };
    let expr = surface_formula(&container);
    let var = surface_var(&container);
    let a = numeric_attr(Some(&container), "data-limit-at")
        .or_else(|| numeric_attr(Some(&container), "data-at"))
        .unwrap_or(0.0);
    invoke_dual(
        document,
        label,
        "SymbolicAlgebra.limit",
        format!(
            "Local lim_{var}→{a} sketch of ({expr}). Connect QualiaDB for a live limit."
        ),
        json!({ "expr": expr, "a": a, "var": var }),
    );
}

fn surface_expr_attr(container: &Element, name: &str, fallback: &str) -> String {
    container
        .get_attribute(name)
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| fallback.to_string())
        .trim()
        .to_string()
}

fn surface_binary_operands(container: &Element) -> (String, String) {
    let fallback_a = surface_formula(container);
    let a = surface_expr_attr(container, "data-a", &fallback_a);
    let b = surface_expr_attr(container, "data-b", "1");
    (a, b)
}

/// `SymbolicAlgebra.add` — `{ a, b }` expression strings.
pub(super) fn run_symbolic_add(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface before building an addition node.",
    ) else {
        return;
    };
    let (a, b) = surface_binary_operands(&container);
    invoke_dual(
        document,
        label,
        "SymbolicAlgebra.add",
        format!("Local add sketch of ({a}) + ({b}). Connect QualiaDB for a live add."),
        json!({ "a": a, "b": b }),
    );
}

/// `SymbolicAlgebra.sub` — `{ a, b }` expression strings.
pub(super) fn run_symbolic_sub(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface before building a subtraction node.",
    ) else {
        return;
    };
    let (a, b) = surface_binary_operands(&container);
    invoke_dual(
        document,
        label,
        "SymbolicAlgebra.sub",
        format!("Local sub sketch of ({a}) − ({b}). Connect QualiaDB for a live sub."),
        json!({ "a": a, "b": b }),
    );
}

/// `SymbolicAlgebra.mul` — `{ a, b }` expression strings.
pub(super) fn run_symbolic_mul(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface before building a multiplication node.",
    ) else {
        return;
    };
    let (a, b) = surface_binary_operands(&container);
    invoke_dual(
        document,
        label,
        "SymbolicAlgebra.mul",
        format!("Local mul sketch of ({a}) × ({b}). Connect QualiaDB for a live mul."),
        json!({ "a": a, "b": b }),
    );
}

/// `SymbolicAlgebra.div` — `{ a, b }` expression strings.
pub(super) fn run_symbolic_div(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface before building a division node.",
    ) else {
        return;
    };
    let (a, b) = surface_binary_operands(&container);
    invoke_dual(
        document,
        label,
        "SymbolicAlgebra.div",
        format!("Local div sketch of ({a}) / ({b}). Connect QualiaDB for a live div."),
        json!({ "a": a, "b": b }),
    );
}

/// `SymbolicAlgebra.pow` — `{ expr, exp }` with integer exponent.
pub(super) fn run_symbolic_pow(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with a formula before building a power node.",
    ) else {
        return;
    };
    let expr = surface_formula(&container);
    let exp = numeric_attr(Some(&container), "data-exp")
        .or_else(|| numeric_attr(Some(&container), "data-power"))
        .unwrap_or(2.0) as i64;
    invoke_dual(
        document,
        label,
        "SymbolicAlgebra.pow",
        format!("Local pow sketch of ({expr})^{exp}. Connect QualiaDB for a live pow."),
        json!({ "expr": expr, "exp": exp }),
    );
}

/// `SymbolicAlgebra.neg` — `{ expr }`.
pub(super) fn run_symbolic_neg(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with a formula before negating.",
    ) else {
        return;
    };
    let expr = surface_formula(&container);
    invoke_dual(
        document,
        label,
        "SymbolicAlgebra.neg",
        format!("Local neg sketch of −({expr}). Connect QualiaDB for a live neg."),
        json!({ "expr": expr }),
    );
}

/// `SymbolicAlgebra.sqrt` — `{ expr }`.
pub(super) fn run_symbolic_sqrt(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with a formula before taking a square root.",
    ) else {
        return;
    };
    let expr = surface_formula(&container);
    invoke_dual(
        document,
        label,
        "SymbolicAlgebra.sqrt",
        format!("Local sqrt sketch of √({expr}). Connect QualiaDB for a live sqrt."),
        json!({ "expr": expr }),
    );
}

/// `SymbolicAlgebra.exp` — `{ expr }`.
pub(super) fn run_symbolic_exp(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with a formula before building an exp node.",
    ) else {
        return;
    };
    let expr = surface_formula(&container);
    invoke_dual(
        document,
        label,
        "SymbolicAlgebra.exp",
        format!("Local exp sketch of e^({expr}). Connect QualiaDB for a live exp."),
        json!({ "expr": expr }),
    );
}

/// `SymbolicAlgebra.ln` — `{ expr }`.
pub(super) fn run_symbolic_ln(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with a formula before building a ln node.",
    ) else {
        return;
    };
    let expr = surface_formula(&container);
    invoke_dual(
        document,
        label,
        "SymbolicAlgebra.ln",
        format!("Local ln sketch of ln({expr}). Connect QualiaDB for a live ln."),
        json!({ "expr": expr }),
    );
}

/// `SymbolicAlgebra.sin` — `{ expr }`.
pub(super) fn run_symbolic_sin(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with a formula before building a sin node.",
    ) else {
        return;
    };
    let expr = surface_formula(&container);
    invoke_dual(
        document,
        label,
        "SymbolicAlgebra.sin",
        format!("Local sin sketch of sin({expr}). Connect QualiaDB for a live sin."),
        json!({ "expr": expr }),
    );
}

/// `SymbolicAlgebra.cos` — `{ expr }`.
pub(super) fn run_symbolic_cos(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with a formula before building a cos node.",
    ) else {
        return;
    };
    let expr = surface_formula(&container);
    invoke_dual(
        document,
        label,
        "SymbolicAlgebra.cos",
        format!("Local cos sketch of cos({expr}). Connect QualiaDB for a live cos."),
        json!({ "expr": expr }),
    );
}

/// `SymbolicAlgebra.tan` — `{ expr }`.
pub(super) fn run_symbolic_tan(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with a formula before building a tan node.",
    ) else {
        return;
    };
    let expr = surface_formula(&container);
    invoke_dual(
        document,
        label,
        "SymbolicAlgebra.tan",
        format!("Local tan sketch of tan({expr}). Connect QualiaDB for a live tan."),
        json!({ "expr": expr }),
    );
}

/// `SymbolicAlgebra.parse` — `{ expr }`.
pub(super) fn run_symbolic_parse(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with a formula before parsing.",
    ) else {
        return;
    };
    let expr = surface_formula(&container);
    invoke_dual(
        document,
        label,
        "SymbolicAlgebra.parse",
        format!("Local parse sketch of ({expr}). Connect QualiaDB for a live parse."),
        json!({ "expr": expr }),
    );
}

/// `SymbolicAlgebra.c` — `{ value }`.
pub(super) fn run_symbolic_c(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface before building a constant leaf.",
    ) else {
        return;
    };
    let value = numeric_attr(Some(&container), "data-value")
        .or_else(|| numeric_attr(Some(&container), "data-c"))
        .unwrap_or(1.0);
    invoke_dual(
        document,
        label,
        "SymbolicAlgebra.c",
        format!("Local constant sketch of {value}. Connect QualiaDB for a live c."),
        json!({ "value": value }),
    );
}

/// `SymbolicAlgebra.var` — `{ name }`.
pub(super) fn run_symbolic_var(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface before building a variable leaf.",
    ) else {
        return;
    };
    let name = container
        .get_attribute("data-name")
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| surface_var(&container));
    invoke_dual(
        document,
        label,
        "SymbolicAlgebra.var",
        format!("Local var sketch of {name}. Connect QualiaDB for a live var."),
        json!({ "name": name }),
    );
}

fn surface_vars(container: &Element) -> Vec<String> {
    if let Some(raw) = container.get_attribute("data-vars") {
        let vars: Vec<String> = raw
            .split(|c: char| c == ',' || c == '|' || c.is_whitespace())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect();
        if !vars.is_empty() {
            return vars;
        }
    }
    vec![surface_var(container)]
}

/// `SymbolicAlgebra.hessian` — `{ expr, vars }`.
pub(super) fn run_symbolic_hessian(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with a formula before computing a Hessian.",
    ) else {
        return;
    };
    let expr = surface_formula(&container);
    let vars = surface_vars(&container);
    invoke_dual(
        document,
        label,
        "SymbolicAlgebra.hessian",
        format!(
            "Local Hessian sketch of ({expr}) in {:?}. Connect QualiaDB for a live hessian.",
            vars
        ),
        json!({ "expr": expr, "vars": vars }),
    );
}

fn parse_number_list(source: &str) -> Vec<f64> {
    source
        .split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ';' | '|' | '\n' | '\r'))
        .filter_map(|token| token.trim().parse::<f64>().ok())
        .filter(|n| n.is_finite())
        .take(256)
        .collect()
}

fn surface_coeffs(container: &Element) -> Vec<f64> {
    if let Some(raw) = container.get_attribute("data-coeffs") {
        let coeffs = parse_number_list(&raw);
        if !coeffs.is_empty() {
            return coeffs;
        }
    }
    let text = container
        .text_content()
        .unwrap_or_default();
    let coeffs = parse_number_list(&text);
    if coeffs.len() >= 2 {
        return coeffs;
    }
    vec![1.0, -3.0, 2.0] // x² − 3x + 2
}

fn surface_exprs(container: &Element) -> Vec<String> {
    if let Some(raw) = container.get_attribute("data-exprs") {
        let exprs: Vec<String> = raw
            .split(|c: char| c == '|' || c == ';')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect();
        if !exprs.is_empty() {
            return exprs;
        }
    }
    vec![surface_formula(container)]
}

fn surface_point(container: &Element, vars: &[String]) -> Value {
    let mut map = serde_json::Map::new();
    if let Some(raw) = container.get_attribute("data-point") {
        for part in raw.split(|c: char| c == ',' || c == ';' || c == '|') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            let (k, v) = if let Some((k, v)) = part.split_once('=') {
                (k, v)
            } else if let Some((k, v)) = part.split_once(':') {
                (k, v)
            } else {
                continue;
            };
            if let Ok(n) = v.trim().parse::<f64>() {
                if n.is_finite() {
                    map.insert(k.trim().to_string(), json!(n));
                }
            }
        }
    }
    for var in vars {
        if map.contains_key(var) {
            continue;
        }
        let attr = format!("data-{var}");
        let n = numeric_attr(Some(container), &attr).unwrap_or(0.0);
        map.insert(var.clone(), json!(n));
    }
    if map.is_empty() {
        map.insert("x".into(), json!(0.0));
    }
    Value::Object(map)
}

/// `SymbolicAlgebra.integrate_definite` — `{ expr, a, b, var?, steps? }`.
pub(super) fn run_symbolic_integrate_definite(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with a formula before definite integration.",
    ) else {
        return;
    };
    let expr = surface_formula(&container);
    let var = surface_var(&container);
    let a = numeric_attr(Some(&container), "data-from")
        .or_else(|| numeric_attr(Some(&container), "data-lo"))
        .or_else(|| numeric_attr(Some(&container), "data-a"))
        .unwrap_or(0.0);
    let b = numeric_attr(Some(&container), "data-to")
        .or_else(|| numeric_attr(Some(&container), "data-hi"))
        .or_else(|| numeric_attr(Some(&container), "data-b"))
        .unwrap_or(1.0);
    let steps = numeric_attr(Some(&container), "data-steps").unwrap_or(64.0) as u64;
    invoke_dual(
        document,
        label,
        "SymbolicAlgebra.integrate_definite",
        format!(
            "Local ∫_{a}^{b} sketch of ({expr}) d{var}. Connect QualiaDB for a live integrate_definite."
        ),
        json!({ "expr": expr, "a": a, "b": b, "var": var, "steps": steps }),
    );
}

/// `SymbolicAlgebra.limit_at_infinity` — `{ expr, var? }`.
pub(super) fn run_symbolic_limit_at_infinity(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with a formula before a limit at infinity.",
    ) else {
        return;
    };
    let expr = surface_formula(&container);
    let var = surface_var(&container);
    invoke_dual(
        document,
        label,
        "SymbolicAlgebra.limit_at_infinity",
        format!(
            "Local lim_{var}→∞ sketch of ({expr}). Connect QualiaDB for a live limit_at_infinity."
        ),
        json!({ "expr": expr, "var": var }),
    );
}

/// `SymbolicAlgebra.real_roots` — `{ coeffs, tol? }`.
pub(super) fn run_symbolic_real_roots(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with polynomial coeffs before real roots.",
    ) else {
        return;
    };
    let coeffs = surface_coeffs(&container);
    let tol = numeric_attr(Some(&container), "data-tol").unwrap_or(1e-8);
    invoke_dual(
        document,
        label,
        "SymbolicAlgebra.real_roots",
        format!(
            "Local real-roots sketch of {:?}. Connect QualiaDB for a live real_roots.",
            coeffs
        ),
        json!({ "coeffs": coeffs, "tol": tol }),
    );
}

/// `SymbolicAlgebra.roots` — `{ coeffs }` complex roots.
pub(super) fn run_symbolic_roots(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with polynomial coeffs before complex roots.",
    ) else {
        return;
    };
    let coeffs = surface_coeffs(&container);
    invoke_dual(
        document,
        label,
        "SymbolicAlgebra.roots",
        format!(
            "Local complex-roots sketch of {:?}. Connect QualiaDB for a live roots.",
            coeffs
        ),
        json!({ "coeffs": coeffs }),
    );
}

/// `SymbolicAlgebra.taylor_coefficients` — `{ expr, a, order, var? }`.
pub(super) fn run_symbolic_taylor_coefficients(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with a formula before Taylor coefficients.",
    ) else {
        return;
    };
    let expr = surface_formula(&container);
    let var = surface_var(&container);
    let a = numeric_attr(Some(&container), "data-center")
        .or_else(|| numeric_attr(Some(&container), "data-a"))
        .or_else(|| numeric_attr(Some(&container), "data-at"))
        .unwrap_or(0.0);
    let order = numeric_attr(Some(&container), "data-order").unwrap_or(4.0) as u64;
    invoke_dual(
        document,
        label,
        "SymbolicAlgebra.taylor_coefficients",
        format!(
            "Local Taylor coeffs sketch of ({expr}) about {var}={a} order {order}. Connect QualiaDB for a live taylor_coefficients."
        ),
        json!({ "expr": expr, "a": a, "order": order, "var": var }),
    );
}

/// `SymbolicAlgebra.taylor_eval` — `{ coeffs, a, x }`.
pub(super) fn run_symbolic_taylor_eval(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with Taylor coeffs before evaluating.",
    ) else {
        return;
    };
    let coeffs = surface_coeffs(&container);
    let a = numeric_attr(Some(&container), "data-center")
        .or_else(|| numeric_attr(Some(&container), "data-a"))
        .unwrap_or(0.0);
    let x = numeric_attr(Some(&container), "data-x")
        .or_else(|| numeric_attr(Some(&container), "data-at"))
        .unwrap_or(0.1);
    invoke_dual(
        document,
        label,
        "SymbolicAlgebra.taylor_eval",
        format!(
            "Local Taylor eval sketch of {:?} about {a} at {x}. Connect QualiaDB for a live taylor_eval.",
            coeffs
        ),
        json!({ "coeffs": coeffs, "a": a, "x": x }),
    );
}

/// `SymbolicAlgebra.jacobian` — `{ exprs, vars }`.
pub(super) fn run_symbolic_jacobian(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with formulas before a Jacobian.",
    ) else {
        return;
    };
    let exprs = surface_exprs(&container);
    let vars = surface_vars(&container);
    invoke_dual(
        document,
        label,
        "SymbolicAlgebra.jacobian",
        format!(
            "Local Jacobian sketch of {:?} in {:?}. Connect QualiaDB for a live jacobian.",
            exprs, vars
        ),
        json!({ "exprs": exprs, "vars": vars }),
    );
}

/// `SymbolicAlgebra.gradient_at` — `{ expr, vars, point }`.
pub(super) fn run_symbolic_gradient_at(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with a formula before a numeric gradient.",
    ) else {
        return;
    };
    let expr = surface_formula(&container);
    let vars = surface_vars(&container);
    let point = surface_point(&container, &vars);
    invoke_dual(
        document,
        label,
        "SymbolicAlgebra.gradient_at",
        format!(
            "Local gradient_at sketch of ({expr}) at {point}. Connect QualiaDB for a live gradient_at."
        ),
        json!({ "expr": expr, "vars": vars, "point": point }),
    );
}

fn surface_assumptions(container: &Element) -> Value {
    if let Some(raw) = container.get_attribute("data-assumptions") {
        let mut items = Vec::new();
        for part in raw.split(|c: char| c == ',' || c == ';' || c == '|') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            let (var, sign) = if let Some((v, s)) = part.split_once(':') {
                (v.trim(), s.trim())
            } else if let Some((v, s)) = part.split_once('=') {
                (v.trim(), s.trim())
            } else {
                continue;
            };
            if !var.is_empty() && !sign.is_empty() {
                items.push(json!({ "var": var, "sign": sign }));
            }
        }
        if !items.is_empty() {
            return Value::Array(items);
        }
    }
    let var = surface_var(container);
    let sign = container
        .get_attribute("data-sign")
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "nonnegative".into());
    Value::Array(vec![json!({ "var": var, "sign": sign })])
}

fn surface_quins(container: &Element) -> Value {
    if let Some(raw) = container.get_attribute("data-quins") {
        if let Ok(Value::Array(items)) = serde_json::from_str::<Value>(&raw) {
            if !items.is_empty() {
                return Value::Array(items);
            }
        }
    }
    // Offline-safe demo leaf: Host `from_quins` rejects empty; live path needs records.
    Value::Array(vec![json!({
        "s": 0u64,
        "p": 0u64,
        "o": 0u64,
        "c": 0u64,
        "m": 0u64
    })])
}

/// `SymbolicAlgebra.hessian_at` — `{ expr, vars, point }`.
pub(super) fn run_symbolic_hessian_at(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with a formula before a numeric Hessian.",
    ) else {
        return;
    };
    let expr = surface_formula(&container);
    let vars = surface_vars(&container);
    let point = surface_point(&container, &vars);
    invoke_dual(
        document,
        label,
        "SymbolicAlgebra.hessian_at",
        format!(
            "Local hessian_at sketch of ({expr}) at {point}. Connect QualiaDB for a live hessian_at."
        ),
        json!({ "expr": expr, "vars": vars, "point": point }),
    );
}

/// `SymbolicAlgebra.solve_quadratic` — `{ a, b, c }`.
pub(super) fn run_symbolic_solve_quadratic(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface before solving a quadratic.",
    ) else {
        return;
    };
    let a = numeric_attr(Some(&container), "data-a").unwrap_or(1.0);
    let b = numeric_attr(Some(&container), "data-b").unwrap_or(-5.0);
    let c = numeric_attr(Some(&container), "data-c").unwrap_or(6.0);
    invoke_dual(
        document,
        label,
        "SymbolicAlgebra.solve_quadratic",
        format!(
            "Local solve_quadratic sketch of {a}x^2 + {b}x + {c}. Connect QualiaDB for a live solve_quadratic."
        ),
        json!({ "a": a, "b": b, "c": c }),
    );
}

/// `SymbolicAlgebra.solve_quadratic_symbolic` — `{ a, b, c }`.
pub(super) fn run_symbolic_solve_quadratic_symbolic(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface before a symbolic quadratic solve.",
    ) else {
        return;
    };
    let a = numeric_attr(Some(&container), "data-a").unwrap_or(1.0);
    let b = numeric_attr(Some(&container), "data-b").unwrap_or(0.0);
    let c = numeric_attr(Some(&container), "data-c").unwrap_or(-1.0);
    invoke_dual(
        document,
        label,
        "SymbolicAlgebra.solve_quadratic_symbolic",
        format!(
            "Local solve_quadratic_symbolic sketch of {a}x^2 + {b}x + {c}. Connect QualiaDB for a live solve_quadratic_symbolic."
        ),
        json!({ "a": a, "b": b, "c": c }),
    );
}

/// `SymbolicAlgebra.factor_quadratic` — `{ a, b, c, var? }`.
pub(super) fn run_symbolic_factor_quadratic(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface before factoring a quadratic.",
    ) else {
        return;
    };
    let a = numeric_attr(Some(&container), "data-a").unwrap_or(1.0);
    let b = numeric_attr(Some(&container), "data-b").unwrap_or(0.0);
    let c = numeric_attr(Some(&container), "data-c").unwrap_or(-1.0);
    let var = surface_var(&container);
    invoke_dual(
        document,
        label,
        "SymbolicAlgebra.factor_quadratic",
        format!(
            "Local factor_quadratic sketch of {a}{var}^2 + {b}{var} + {c}. Connect QualiaDB for a live factor_quadratic."
        ),
        json!({ "a": a, "b": b, "c": c, "var": var }),
    );
}

/// `SymbolicAlgebra.solve_polynomial_expr` — `{ expr, degree, var?, tol? }`.
pub(super) fn run_symbolic_solve_polynomial_expr(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with a polynomial formula before solving.",
    ) else {
        return;
    };
    let expr = surface_formula(&container);
    let var = surface_var(&container);
    let degree = numeric_attr(Some(&container), "data-degree").unwrap_or(2.0) as u64;
    let tol = numeric_attr(Some(&container), "data-tol").unwrap_or(1e-8);
    invoke_dual(
        document,
        label,
        "SymbolicAlgebra.solve_polynomial_expr",
        format!(
            "Local solve_polynomial_expr sketch of ({expr}) degree {degree}. Connect QualiaDB for a live solve_polynomial_expr."
        ),
        json!({ "expr": expr, "degree": degree, "var": var, "tol": tol }),
    );
}

/// `SymbolicAlgebra.simplify_with_assumptions` — `{ expr, assumptions? }`.
pub(super) fn run_symbolic_simplify_with_assumptions(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with a formula before simplifying with assumptions.",
    ) else {
        return;
    };
    let expr = surface_formula(&container);
    let assumptions = surface_assumptions(&container);
    invoke_dual(
        document,
        label,
        "SymbolicAlgebra.simplify_with_assumptions",
        format!(
            "Local simplify_with_assumptions sketch of ({expr}) under {assumptions}. Connect QualiaDB for a live simplify_with_assumptions."
        ),
        json!({ "expr": expr, "assumptions": assumptions }),
    );
}

/// `SymbolicAlgebra.expr_citation_hash` — `{ expr }`.
pub(super) fn run_symbolic_expr_citation_hash(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with a formula before hashing a citation.",
    ) else {
        return;
    };
    let expr = surface_formula(&container);
    invoke_dual(
        document,
        label,
        "SymbolicAlgebra.expr_citation_hash",
        format!(
            "Local expr_citation_hash sketch of ({expr}). Connect QualiaDB for a live expr_citation_hash."
        ),
        json!({ "expr": expr }),
    );
}

/// `SymbolicAlgebra.to_quins` — `{ expr }`.
pub(super) fn run_symbolic_to_quins(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with a formula before encoding quins.",
    ) else {
        return;
    };
    let expr = surface_formula(&container);
    invoke_dual(
        document,
        label,
        "SymbolicAlgebra.to_quins",
        format!(
            "Local to_quins sketch of ({expr}). Connect QualiaDB for a live to_quins."
        ),
        json!({ "expr": expr }),
    );
}

/// `SymbolicAlgebra.from_quins` — `{ quins }`.
pub(super) fn run_symbolic_from_quins(document: &Document, label: &str) {
    let Some(container) = need_container(
        document,
        label,
        "Select a surface with data-quins before reconstructing an expression.",
    ) else {
        return;
    };
    let quins = surface_quins(&container);
    invoke_dual(
        document,
        label,
        "SymbolicAlgebra.from_quins",
        format!(
            "Local from_quins sketch of {quins}. Connect QualiaDB for a live from_quins."
        ),
        json!({ "quins": quins }),
    );
}
