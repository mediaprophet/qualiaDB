//! Dual-path Tool Chest actions for curated `Econ.*` ALL_BOUND ids.
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

fn parse_numbers(source: &str) -> Vec<f64> {
    source
        .split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ';' | '|' | '\n' | '\r'))
        .filter_map(|token| token.trim().parse::<f64>().ok())
        .filter(|n| n.is_finite())
        .take(4096)
        .collect()
}

fn numeric_attr(el: Option<&Element>, name: &str) -> Option<f64> {
    el.and_then(|e| e.get_attribute(name))
        .and_then(|v| v.parse::<f64>().ok())
}

fn local_gini(incomes: &[f64]) -> Option<f64> {
    if incomes.len() < 2 {
        return None;
    }
    let mut sorted = incomes.to_vec();
    sorted.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
    let n = sorted.len() as f64;
    let sum: f64 = sorted.iter().sum();
    if sum <= 0.0 {
        return Some(0.0);
    }
    let mut weighted = 0.0;
    for (i, x) in sorted.iter().enumerate() {
        weighted += (2.0 * (i as f64 + 1.0) - n - 1.0) * x;
    }
    Some(weighted / (n * sum))
}

/// Offline Atkinson sketch (ε>0, strictly positive incomes) — not a Host invoke.
fn local_atkinson(incomes: &[f64], epsilon: f64) -> Option<f64> {
    if incomes.len() < 2 || !(epsilon > 0.0) || !epsilon.is_finite() {
        return None;
    }
    if incomes.iter().any(|x| !x.is_finite() || *x <= 0.0) {
        return None;
    }
    let n = incomes.len() as f64;
    let mean = incomes.iter().sum::<f64>() / n;
    if mean <= 0.0 {
        return None;
    }
    let a = if (epsilon - 1.0).abs() < f64::EPSILON {
        let log_sum: f64 = incomes.iter().map(|x| x.ln()).sum();
        let geo = (log_sum / n).exp();
        1.0 - (geo / mean)
    } else {
        let one_minus = 1.0 - epsilon;
        let mean_pow = incomes.iter().map(|x| x.powf(one_minus)).sum::<f64>() / n;
        let power_mean = mean_pow.powf(1.0 / one_minus);
        1.0 - (power_mean / mean)
    };
    a.is_finite().then_some(a.clamp(0.0, 1.0))
}

/// Offline historical VaR sketch — left-tail loss at `confidence` (default 0.95).
fn local_historical_var(returns: &[f64], confidence: f64) -> Option<f64> {
    if returns.len() < 2 || !(0.0..1.0).contains(&confidence) {
        return None;
    }
    if returns.iter().any(|r| !r.is_finite()) {
        return None;
    }
    let mut sorted = returns.to_vec();
    sorted.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
    let n = sorted.len();
    let idx = (((1.0 - confidence) * n as f64).ceil() as usize).saturating_sub(1);
    Some((-sorted[idx]).max(0.0))
}

fn local_gordon(next_dividend: f64, required_return: f64, growth_rate: f64) -> Option<f64> {
    if !(next_dividend.is_finite()
        && required_return.is_finite()
        && growth_rate.is_finite()
        && next_dividend >= 0.0
        && required_return > 0.0
        && growth_rate >= 0.0
        && growth_rate < required_return)
    {
        return None;
    }
    Some(next_dividend / (required_return - growth_rate))
}

fn local_cournot(a: f64, b: f64, c1: f64, c2: f64) -> Option<(f64, f64, f64)> {
    if !(a.is_finite() && b.is_finite() && c1.is_finite() && c2.is_finite()) || b <= 0.0 {
        return None;
    }
    let q1 = (a - 2.0 * c1 + c2) / (3.0 * b);
    let q2 = (a - 2.0 * c2 + c1) / (3.0 * b);
    let price = (a + c1 + c2) / 3.0;
    (q1 >= 0.0 && q2 >= 0.0).then_some((q1, q2, price))
}

fn local_bertrand(c1: f64, c2: f64) -> Option<f64> {
    if !(c1.is_finite() && c2.is_finite()) || c1 < 0.0 || c2 < 0.0 {
        return None;
    }
    Some(if c1 == c2 { c1 } else { c1.max(c2) })
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

/// `Econ.gini` — incomes from selected surface numbers.
pub(super) fn run_gini(document: &Document, label: &str) {
    let source = selected_source(document).unwrap_or_default();
    let incomes = parse_numbers(&source);
    let Some(g) = local_gini(&incomes) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Select a sheet or document with at least two income numbers.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Econ.gini",
        format!("Gini sketch over {} incomes: {g:.4}", incomes.len()),
        json!({ "incomes": incomes }),
    );
}

/// `Econ.capm_expected_return` — attrs `data-rf`, `data-beta`, `data-market-premium`.
pub(super) fn run_capm(document: &Document, label: &str) {
    let container = selected_container(document);
    let rf = numeric_attr(container.as_ref(), "data-rf").unwrap_or(0.02);
    let beta = numeric_attr(container.as_ref(), "data-beta").unwrap_or(1.0);
    let market_premium =
        numeric_attr(container.as_ref(), "data-market-premium").unwrap_or(0.05);
    let expected = rf + beta * market_premium;
    invoke_dual(
        document,
        label,
        "Econ.capm_expected_return",
        format!("CAPM sketch: rf={rf} beta={beta} premium={market_premium} → {expected:.4}"),
        json!({
            "rf": rf,
            "beta": beta,
            "market_premium": market_premium,
        }),
    );
}

/// `Econ.black_scholes` — option params from data-* attrs (defaults are illustrative only).
pub(super) fn run_black_scholes(document: &Document, label: &str) {
    let container = selected_container(document);
    let spot = numeric_attr(container.as_ref(), "data-spot").unwrap_or(100.0);
    let strike = numeric_attr(container.as_ref(), "data-strike").unwrap_or(100.0);
    let time_to_expiry =
        numeric_attr(container.as_ref(), "data-time-to-expiry").unwrap_or(1.0);
    let risk_free_rate =
        numeric_attr(container.as_ref(), "data-risk-free-rate").unwrap_or(0.03);
    let volatility = numeric_attr(container.as_ref(), "data-volatility").unwrap_or(0.2);
    let dividend_yield =
        numeric_attr(container.as_ref(), "data-dividend-yield").unwrap_or(0.0);
    let is_call = container
        .as_ref()
        .and_then(|c| c.get_attribute("data-is-call"))
        .map(|v| v != "0" && !v.eq_ignore_ascii_case("false"))
        .unwrap_or(true);
    invoke_dual(
        document,
        label,
        "Econ.black_scholes",
        format!(
            "Black–Scholes sketch: S={spot} K={strike} T={time_to_expiry} r={risk_free_rate} σ={volatility} call={is_call}"
        ),
        json!({
            "spot": spot,
            "strike": strike,
            "time_to_expiry": time_to_expiry,
            "risk_free_rate": risk_free_rate,
            "volatility": volatility,
            "dividend_yield": dividend_yield,
            "is_call": is_call,
        }),
    );
}

/// `Econ.mixed_nash_2x2` — eight numbers from surface, or Prisoner's Dilemma defaults.
pub(super) fn run_mixed_nash(document: &Document, label: &str) {
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let (a, b) = if nums.len() >= 8 {
        (nums[0..4].to_vec(), nums[4..8].to_vec())
    } else {
        // Classic PD payoffs (row / col) — local sketch only until live.
        (vec![3.0, 0.0, 5.0, 1.0], vec![3.0, 5.0, 0.0, 1.0])
    };
    invoke_dual(
        document,
        label,
        "Econ.mixed_nash_2x2",
        format!("Mixed-Nash sketch over matrices A={a:?} B={b:?}"),
        json!({
            "payoff_matrix_a": a,
            "payoff_matrix_b": b,
        }),
    );
}

/// `Econ.solow_steady_state` — macro params from data-* attrs.
pub(super) fn run_solow(document: &Document, label: &str) {
    let container = selected_container(document);
    let savings_rate =
        numeric_attr(container.as_ref(), "data-savings-rate").unwrap_or(0.3);
    let alpha = numeric_attr(container.as_ref(), "data-alpha").unwrap_or(0.33);
    let depreciation =
        numeric_attr(container.as_ref(), "data-depreciation").unwrap_or(0.05);
    let population_growth =
        numeric_attr(container.as_ref(), "data-population-growth").unwrap_or(0.01);
    let tech_growth = numeric_attr(container.as_ref(), "data-tech-growth").unwrap_or(0.02);
    invoke_dual(
        document,
        label,
        "Econ.solow_steady_state",
        format!(
            "Solow sketch: s={savings_rate} α={alpha} δ={depreciation} n={population_growth} g={tech_growth}"
        ),
        json!({
            "savings_rate": savings_rate,
            "alpha": alpha,
            "depreciation": depreciation,
            "population_growth": population_growth,
            "tech_growth": tech_growth,
        }),
    );
}

/// `Econ.cournot_duopoly` — demand intercept/slope and two costs from data-* attrs.
pub(super) fn run_cournot(document: &Document, label: &str) {
    let container = selected_container(document);
    let demand_intercept =
        numeric_attr(container.as_ref(), "data-demand-intercept").unwrap_or(100.0);
    let demand_slope = numeric_attr(container.as_ref(), "data-demand-slope").unwrap_or(1.0);
    let cost_1 = numeric_attr(container.as_ref(), "data-cost-1").unwrap_or(10.0);
    let cost_2 = numeric_attr(container.as_ref(), "data-cost-2").unwrap_or(10.0);
    let sketch = match local_cournot(demand_intercept, demand_slope, cost_1, cost_2) {
        Some((q1, q2, price)) => {
            format!("Cournot sketch: q1={q1:.3} q2={q2:.3} P={price:.3}")
        }
        None => format!(
            "Cournot sketch inputs a={demand_intercept} b={demand_slope} c1={cost_1} c2={cost_2} (no interior equilibrium offline)"
        ),
    };
    invoke_dual(
        document,
        label,
        "Econ.cournot_duopoly",
        sketch,
        json!({
            "demand_intercept": demand_intercept,
            "demand_slope": demand_slope,
            "cost_1": cost_1,
            "cost_2": cost_2,
        }),
    );
}

/// `Econ.bertrand_duopoly` — two marginal costs from data-* attrs.
pub(super) fn run_bertrand(document: &Document, label: &str) {
    let container = selected_container(document);
    let cost_1 = numeric_attr(container.as_ref(), "data-cost-1").unwrap_or(10.0);
    let cost_2 = numeric_attr(container.as_ref(), "data-cost-2").unwrap_or(12.0);
    let sketch = match local_bertrand(cost_1, cost_2) {
        Some(price) => format!("Bertrand sketch: equilibrium price ≈ {price:.3} (c1={cost_1} c2={cost_2})"),
        None => format!("Bertrand sketch inputs c1={cost_1} c2={cost_2} (invalid offline)"),
    };
    invoke_dual(
        document,
        label,
        "Econ.bertrand_duopoly",
        sketch,
        json!({
            "cost_1": cost_1,
            "cost_2": cost_2,
        }),
    );
}

/// `Econ.historical_var` — returns from selected surface numbers; confidence from attr.
pub(super) fn run_historical_var(document: &Document, label: &str) {
    let source = selected_source(document).unwrap_or_default();
    let returns = parse_numbers(&source);
    let confidence = numeric_attr(selected_container(document).as_ref(), "data-confidence")
        .unwrap_or(0.95);
    let Some(var) = local_historical_var(&returns, confidence) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Select a sheet or document with at least two return numbers (optional data-confidence).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Econ.historical_var",
        format!(
            "Historical VaR sketch over {} returns @ {confidence:.2}: {var:.4}",
            returns.len()
        ),
        json!({
            "returns": returns,
            "confidence": confidence,
        }),
    );
}

/// `Econ.atkinson` — incomes from surface; `data-epsilon` (default 1.0).
pub(super) fn run_atkinson(document: &Document, label: &str) {
    let source = selected_source(document).unwrap_or_default();
    let incomes = parse_numbers(&source);
    let epsilon =
        numeric_attr(selected_container(document).as_ref(), "data-epsilon").unwrap_or(1.0);
    let Some(a) = local_atkinson(&incomes, epsilon) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Select a sheet or document with at least two strictly positive incomes (optional data-epsilon).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Econ.atkinson",
        format!(
            "Atkinson sketch over {} incomes (ε={epsilon}): {a:.4}",
            incomes.len()
        ),
        json!({
            "incomes": incomes,
            "epsilon": epsilon,
        }),
    );
}

/// `Econ.gordon_growth` — dividend discount from data-* attrs.
pub(super) fn run_gordon_growth(document: &Document, label: &str) {
    let container = selected_container(document);
    let next_dividend =
        numeric_attr(container.as_ref(), "data-next-dividend").unwrap_or(2.0);
    let required_return =
        numeric_attr(container.as_ref(), "data-required-return").unwrap_or(0.08);
    let growth_rate = numeric_attr(container.as_ref(), "data-growth-rate").unwrap_or(0.03);
    let sketch = match local_gordon(next_dividend, required_return, growth_rate) {
        Some(price) => {
            format!("Gordon sketch: D1={next_dividend} r={required_return} g={growth_rate} → P={price:.3}")
        }
        None => format!(
            "Gordon sketch inputs D1={next_dividend} r={required_return} g={growth_rate} (need r>g≥0)"
        ),
    };
    invoke_dual(
        document,
        label,
        "Econ.gordon_growth",
        sketch,
        json!({
            "next_dividend": next_dividend,
            "required_return": required_return,
            "growth_rate": growth_rate,
        }),
    );
}

#[cfg(test)]
mod tests {
    use super::{
        local_atkinson, local_bertrand, local_cournot, local_gini, local_gordon,
        local_historical_var,
    };

    #[test]
    fn local_gini_rejects_singleton() {
        assert!(local_gini(&[10.0]).is_none());
    }

    #[test]
    fn local_gini_equal_incomes_is_zero() {
        let g = local_gini(&[10.0, 10.0, 10.0]).unwrap();
        assert!(g.abs() < 1e-9);
    }

    #[test]
    fn local_gini_unequal_is_positive() {
        let g = local_gini(&[10.0, 20.0, 30.0, 40.0]).unwrap();
        assert!(g > 0.0);
        assert!(g < 1.0);
    }

    #[test]
    fn local_atkinson_equal_is_zero() {
        let a = local_atkinson(&[10.0, 10.0, 10.0], 1.0).unwrap();
        assert!(a.abs() < 1e-9);
    }

    #[test]
    fn local_atkinson_rejects_nonpositive() {
        assert!(local_atkinson(&[10.0, 0.0], 1.0).is_none());
    }

    #[test]
    fn local_historical_var_positive_loss() {
        let returns = [-0.05, -0.02, 0.0, 0.01, 0.03];
        let v = local_historical_var(&returns, 0.8).unwrap();
        assert!(v >= 0.0);
    }

    #[test]
    fn local_gordon_basic() {
        let p = local_gordon(2.0, 0.08, 0.03).unwrap();
        assert!((p - 40.0).abs() < 1e-9);
    }

    #[test]
    fn local_cournot_symmetric() {
        let (q1, q2, price) = local_cournot(100.0, 1.0, 10.0, 10.0).unwrap();
        assert!((q1 - q2).abs() < 1e-9);
        assert!(price > 0.0);
    }

    #[test]
    fn local_bertrand_equal_costs() {
        assert!((local_bertrand(5.0, 5.0).unwrap() - 5.0).abs() < 1e-9);
    }
}
