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

/// Offline β-δ hyperbolic discount — matches Host `Econ.hyperbolic_discount`.
fn local_hyperbolic_discount(t: u32, beta: f64, delta: f64) -> Option<f64> {
    if !(0.0..=1.0).contains(&beta) || !(0.0..=1.0).contains(&delta) {
        return None;
    }
    if t == 0 {
        return Some(1.0);
    }
    let d = beta * delta.powi(t as i32);
    d.is_finite().then_some(d)
}

/// Offline fiscal multiplier — matches Host `Econ.fiscal_multiplier`.
fn local_fiscal_multiplier(initial_spending: f64, mpc: f64, leakage_rate: f64) -> Option<f64> {
    if !(initial_spending.is_finite() && mpc.is_finite() && leakage_rate.is_finite()) {
        return None;
    }
    if !(0.0..=1.0).contains(&mpc) || leakage_rate < 0.0 {
        return None;
    }
    let denom = 1.0 - mpc + leakage_rate;
    if denom <= 0.0 {
        return None;
    }
    let m = initial_spending / denom;
    m.is_finite().then_some(m)
}

/// Offline headcount poverty — matches Host `Econ.headcount_poverty`.
fn local_headcount_poverty(incomes: &[f64], poverty_line: f64) -> Option<(usize, f64)> {
    if incomes.is_empty() || !(poverty_line > 0.0 && poverty_line.is_finite()) {
        return None;
    }
    if incomes.iter().any(|x| !x.is_finite() || *x < 0.0) {
        return None;
    }
    let count = incomes.iter().filter(|&&x| x < poverty_line).count();
    let rate = count as f64 / incomes.len() as f64;
    Some((count, rate))
}

/// Offline max-drawdown magnitude from a wealth path (Host returns full series).
fn local_max_drawdown(wealth: &[f64]) -> Option<f64> {
    if wealth.is_empty() || wealth.iter().any(|w| !w.is_finite()) {
        return None;
    }
    let mut peak = wealth[0];
    let mut worst = 0.0f64;
    for &w in wealth {
        if w > peak {
            peak = w;
        }
        let dd = if peak == 0.0 { 0.0 } else { w / peak - 1.0 };
        if dd < worst {
            worst = dd;
        }
    }
    Some((-worst).max(0.0))
}

/// Offline one-step CRR European option sketch — not a multi-step Host binomial.
fn local_binomial_one_step(
    spot: f64,
    strike: f64,
    time_to_expiry: f64,
    risk_free_rate: f64,
    volatility: f64,
    dividend_yield: f64,
    is_call: bool,
) -> Option<f64> {
    if !(spot > 0.0
        && strike > 0.0
        && time_to_expiry > 0.0
        && volatility > 0.0
        && spot.is_finite()
        && strike.is_finite()
        && time_to_expiry.is_finite()
        && risk_free_rate.is_finite()
        && volatility.is_finite()
        && dividend_yield.is_finite())
    {
        return None;
    }
    let u = (volatility * time_to_expiry.sqrt()).exp();
    let d = 1.0 / u;
    let a = ((risk_free_rate - dividend_yield) * time_to_expiry).exp();
    let p = ((a - d) / (u - d)).clamp(0.0, 1.0);
    let su = spot * u;
    let sd = spot * d;
    let pay_u = if is_call {
        (su - strike).max(0.0)
    } else {
        (strike - su).max(0.0)
    };
    let pay_d = if is_call {
        (sd - strike).max(0.0)
    } else {
        (strike - sd).max(0.0)
    };
    let disc = (-risk_free_rate * time_to_expiry).exp();
    let price = disc * (p * pay_u + (1.0 - p) * pay_d);
    price.is_finite().then_some(price)
}

/// Offline GBM drift endpoint — deterministic sketch, not Host RNG path.
fn local_gbm_drift_endpoint(s0: f64, mu: f64, sigma: f64, dt: f64, n_steps: usize) -> Option<f64> {
    if !(s0 > 0.0
        && dt > 0.0
        && n_steps >= 1
        && s0.is_finite()
        && mu.is_finite()
        && sigma.is_finite()
        && dt.is_finite()
        && sigma >= 0.0)
    {
        return None;
    }
    let t = dt * n_steps as f64;
    let end = s0 * ((mu - 0.5 * sigma * sigma) * t).exp();
    end.is_finite().then_some(end)
}

/// Offline two-point annualized forward from flat zeros (illustrative only).
fn local_forward_rate_two_point(r1: f64, t1: f64, r2: f64, t2: f64) -> Option<f64> {
    if !(r1.is_finite() && r2.is_finite() && t1.is_finite() && t2.is_finite()) || t2 <= t1 || t1 < 0.0
    {
        return None;
    }
    // Continuous-compound style: exp(r2*t2)/exp(r1*t1) ^ (1/(t2-t1)) - 1
    let df1 = (-r1 * t1).exp();
    let df2 = (-r2 * t2).exp();
    if df1 <= 0.0 || df2 <= 0.0 {
        return None;
    }
    let f = (df1 / df2).powf(1.0 / (t2 - t1)) - 1.0;
    f.is_finite().then_some(f)
}

/// Offline CAPM beta — cov(asset, market) / var(market), population form.
fn local_capm_beta(asset: &[f64], market: &[f64]) -> Option<f64> {
    if asset.len() != market.len() || asset.len() < 2 {
        return None;
    }
    if asset.iter().chain(market.iter()).any(|x| !x.is_finite()) {
        return None;
    }
    let n = asset.len() as f64;
    let mean_a = asset.iter().sum::<f64>() / n;
    let mean_m = market.iter().sum::<f64>() / n;
    let mut cov = 0.0;
    let mut var_m = 0.0;
    for i in 0..asset.len() {
        let da = asset[i] - mean_a;
        let dm = market[i] - mean_m;
        cov += da * dm;
        var_m += dm * dm;
    }
    cov /= n;
    var_m /= n;
    if var_m <= 0.0 {
        return None;
    }
    let beta = cov / var_m;
    beta.is_finite().then_some(beta)
}

/// Offline lag-k autocorrelation (Box–Jenkins, full-series denominator).
fn local_autocorrelation(values: &[f64], lag: usize) -> Option<f64> {
    if values.len() < 2 || lag == 0 || lag >= values.len() {
        return None;
    }
    if values.iter().any(|x| !x.is_finite()) {
        return None;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let mut denom = 0.0;
    for &v in values {
        let d = v - mean;
        denom += d * d;
    }
    if denom == 0.0 {
        return Some(0.0);
    }
    let mut num = 0.0;
    for t in 0..values.len() - lag {
        num += (values[t] - mean) * (values[t + lag] - mean);
    }
    let ac = num / denom;
    ac.is_finite().then_some(ac)
}

/// Offline Pearson r for equal-length overlap (lag-0 cross-correlation sketch).
fn local_pearson(x: &[f64], y: &[f64]) -> Option<f64> {
    if x.len() != y.len() || x.len() < 2 {
        return None;
    }
    if x.iter().chain(y.iter()).any(|v| !v.is_finite()) {
        return None;
    }
    let n = x.len() as f64;
    let mx = x.iter().sum::<f64>() / n;
    let my = y.iter().sum::<f64>() / n;
    let mut cov = 0.0;
    let mut vx = 0.0;
    let mut vy = 0.0;
    for i in 0..x.len() {
        let dx = x[i] - mx;
        let dy = y[i] - my;
        cov += dx * dy;
        vx += dx * dx;
        vy += dy * dy;
    }
    if vx <= 0.0 || vy <= 0.0 {
        return None;
    }
    let r = cov / (vx.sqrt() * vy.sqrt());
    r.is_finite().then_some(r)
}

/// Offline Bertrand-with-demand: price = max(c1,c2) (or c if equal); q = max(0,(a-p)/b).
fn local_bertrand_with_demand(a: f64, b: f64, c1: f64, c2: f64) -> Option<(f64, f64)> {
    if !(a.is_finite() && b.is_finite() && c1.is_finite() && c2.is_finite()) || b <= 0.0 {
        return None;
    }
    if c1 < 0.0 || c2 < 0.0 {
        return None;
    }
    let price = if c1 == c2 { c1 } else { c1.max(c2) };
    let quantity = ((a - price) / b).max(0.0);
    Some((price, quantity))
}

/// Offline budget-balance: sum(payments) ≥ 0 (within 1e-10).
fn local_check_budget_balance(payments: &[f64]) -> Option<(bool, f64)> {
    if payments.is_empty() || payments.iter().any(|p| !p.is_finite()) {
        return None;
    }
    let surplus: f64 = payments.iter().sum();
    Some((surplus >= -1e-10, surplus))
}

/// Offline CCAPM equity premium — Breeden–Lucas form.
fn local_ccapm_equity_premium(gamma: f64, sigma_c: f64, sigma_m: f64) -> Option<f64> {
    if !(gamma > 0.0 && sigma_c >= 0.0 && sigma_m >= 0.0)
        || !(gamma.is_finite() && sigma_c.is_finite() && sigma_m.is_finite())
    {
        return None;
    }
    let premium = gamma * sigma_c * sigma_m + 0.5 * gamma * gamma * sigma_c * sigma_c;
    premium.is_finite().then_some(premium)
}

/// Offline arithmetic mean return.
fn local_mean_return(returns: &[f64]) -> Option<f64> {
    if returns.is_empty() || returns.iter().any(|r| !r.is_finite()) {
        return None;
    }
    Some(returns.iter().sum::<f64>() / returns.len() as f64)
}

/// Offline poverty-gap ratio — average shortfall / poverty line.
fn local_poverty_gap(incomes: &[f64], poverty_line: f64) -> Option<f64> {
    if incomes.is_empty() || !(poverty_line > 0.0 && poverty_line.is_finite()) {
        return None;
    }
    if incomes.iter().any(|x| !x.is_finite() || *x < 0.0) {
        return None;
    }
    let mut gap = 0.0;
    for &x in incomes {
        if x < poverty_line {
            gap += poverty_line - x;
        }
    }
    let ratio = gap / (incomes.len() as f64 * poverty_line);
    ratio.is_finite().then_some(ratio)
}

/// Offline sample variance (Bessel-corrected) — matches Host `Econ.sample_variance`.
fn local_sample_variance(returns: &[f64]) -> Option<f64> {
    if returns.len() < 2 || returns.iter().any(|r| !r.is_finite()) {
        return None;
    }
    let mean = returns.iter().sum::<f64>() / returns.len() as f64;
    let mut sum_sq = 0.0;
    for &r in returns {
        let d = r - mean;
        sum_sq += d * d;
    }
    let v = sum_sq / (returns.len() - 1) as f64;
    v.is_finite().then_some(v)
}

/// Offline utilitarian welfare — sum of utilities.
fn local_utilitarian_welfare(utilities: &[f64]) -> Option<f64> {
    if utilities.is_empty() || utilities.iter().any(|u| !u.is_finite()) {
        return None;
    }
    Some(utilities.iter().sum())
}

/// Offline Rawlsian welfare — min of utilities.
fn local_rawlsian_welfare(utilities: &[f64]) -> Option<f64> {
    if utilities.is_empty() || utilities.iter().any(|u| !u.is_finite()) {
        return None;
    }
    utilities.iter().copied().reduce(f64::min)
}

/// Offline Nash welfare — product of strictly positive utilities.
fn local_nash_welfare(utilities: &[f64]) -> Option<f64> {
    if utilities.is_empty() {
        return None;
    }
    if utilities.iter().any(|u| !u.is_finite() || *u <= 0.0) {
        return None;
    }
    let product = utilities.iter().product::<f64>();
    product.is_finite().then_some(product)
}

/// Offline Stackelberg duopoly — leader/follower quantities and price.
fn local_stackelberg(a: f64, b: f64, c1: f64, c2: f64) -> Option<(f64, f64, f64)> {
    if !(a.is_finite() && b.is_finite() && c1.is_finite() && c2.is_finite()) || b <= 0.0 {
        return None;
    }
    let q1 = (a + c2 - 2.0 * c1) / (2.0 * b);
    let q2 = (a - 3.0 * c2 + 2.0 * c1) / (4.0 * b);
    if q1 < 0.0 || q2 < 0.0 {
        return None;
    }
    let price = a - b * (q1 + q2);
    price.is_finite().then_some((q1, q2, price))
}

/// Offline put-call parity residual.
fn local_put_call_parity(
    call_price: f64,
    put_price: f64,
    spot: f64,
    strike: f64,
    risk_free_rate: f64,
    dividend_yield: f64,
    time_to_expiry: f64,
) -> Option<f64> {
    if !(call_price.is_finite()
        && put_price.is_finite()
        && spot > 0.0
        && strike > 0.0
        && risk_free_rate.is_finite()
        && dividend_yield.is_finite()
        && time_to_expiry >= 0.0
        && spot.is_finite()
        && strike.is_finite()
        && time_to_expiry.is_finite())
    {
        return None;
    }
    let residual = call_price
        - put_price
        - (spot * (-dividend_yield * time_to_expiry).exp()
            - strike * (-risk_free_rate * time_to_expiry).exp());
    residual.is_finite().then_some(residual)
}

/// Compact Acklam inverse-normal for parametric VaR left-tail probability.
fn local_normal_quantile(probability: f64) -> Option<f64> {
    if !(0.0..1.0).contains(&probability) {
        return None;
    }
    const A: [f64; 6] = [
        -3.969_683_028_665_376e1,
        2.209_460_984_245_205e2,
        -2.759_285_104_469_687e2,
        1.383_577_518_672_69e2,
        -3.066_479_806_614_716e1,
        2.506_628_277_459_239,
    ];
    const B: [f64; 5] = [
        -5.447_609_879_822_406e1,
        1.615_858_368_580_409e2,
        -1.556_989_798_598_866e2,
        6.680_131_188_771_972e1,
        -1.328_068_155_288_572e1,
    ];
    const C: [f64; 6] = [
        -7.784_894_002_430_293e-3,
        -3.223_964_580_411_365e-1,
        -2.400_758_277_161_838,
        -2.549_732_539_343_734,
        4.374_664_141_464_968,
        2.938_163_982_698_783,
    ];
    const D: [f64; 4] = [
        7.784_695_709_041_462e-3,
        3.224_671_290_700_398e-1,
        2.445_134_137_142_996,
        3.754_408_661_907_416,
    ];
    let plow = 0.02425;
    let phigh = 1.0 - plow;
    let z = if probability < plow {
        let q = (-2.0 * probability.ln()).sqrt();
        (((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    } else if probability > phigh {
        let q = (-2.0 * (1.0 - probability).ln()).sqrt();
        -(((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    } else {
        let q = probability - 0.5;
        let r = q * q;
        (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q
            / (((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r + 1.0)
    };
    z.is_finite().then_some(z)
}

/// Offline parametric (Gaussian) VaR — positive loss fraction.
fn local_parametric_var(mean: f64, std_dev: f64, confidence: f64) -> Option<f64> {
    if !mean.is_finite()
        || !std_dev.is_finite()
        || std_dev < 0.0
        || !(0.0..1.0).contains(&confidence)
    {
        return None;
    }
    let z = local_normal_quantile(1.0 - confidence)?;
    Some((-(mean + z * std_dev)).max(0.0))
}

/// Offline Laffer revenue — `t * B * (1-t)^ε`.
fn local_laffer_curve(tax_rate: f64, tax_base: f64, elasticity: f64) -> Option<f64> {
    if !(tax_rate.is_finite() && tax_base.is_finite() && elasticity.is_finite()) {
        return None;
    }
    if !(0.0..=1.0).contains(&tax_rate) || tax_base < 0.0 || elasticity < 0.0 {
        return None;
    }
    let rev = tax_rate * tax_base * (1.0 - tax_rate).powf(elasticity);
    rev.is_finite().then_some(rev)
}

/// Offline historical CVaR — average positive left-tail loss.
fn local_historical_cvar(returns: &[f64], confidence: f64) -> Option<f64> {
    if returns.len() < 2 || !(0.0..1.0).contains(&confidence) {
        return None;
    }
    if returns.iter().any(|r| !r.is_finite()) {
        return None;
    }
    let mut sorted = returns.to_vec();
    sorted.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
    let n = sorted.len();
    let tail_count = (((1.0 - confidence) * n as f64).ceil() as usize).max(1);
    let mut loss = 0.0;
    for value in sorted.iter().take(tail_count) {
        loss += (-*value).max(0.0);
    }
    Some(loss / tail_count as f64)
}

/// Offline endowment WTA = WTP × λ.
fn local_endowment_effect(wtp: f64, lambda: f64) -> Option<f64> {
    if !(wtp.is_finite() && lambda.is_finite()) || wtp < 0.0 || lambda <= 0.0 {
        return None;
    }
    let wta = wtp * lambda;
    wta.is_finite().then_some(wta)
}

/// Offline prospect-theory value — matches Host `Econ.prospect_value`.
fn local_prospect_value(x: f64, alpha: f64, beta: f64, lambda: f64) -> Option<f64> {
    if !(x.is_finite() && alpha.is_finite() && beta.is_finite() && lambda.is_finite()) {
        return None;
    }
    if !(0.0..=1.0).contains(&alpha) || !(0.0..=1.0).contains(&beta) || lambda <= 0.0 {
        return None;
    }
    let v = if x >= 0.0 {
        x.powf(alpha)
    } else {
        -lambda * (-x).powf(beta)
    };
    v.is_finite().then_some(v)
}

/// Offline Prelec probability weight — matches Host `Econ.probability_weight`.
fn local_probability_weight(p: f64, gamma: f64) -> Option<f64> {
    if !(p.is_finite() && gamma.is_finite()) {
        return None;
    }
    if !(0.0..=1.0).contains(&p) || p == 0.0 || gamma <= 0.0 {
        return None;
    }
    let w = (-(-p.ln()).powf(gamma)).exp();
    w.is_finite().then_some(w)
}

/// Offline CCAPM SDF — matches Host `Econ.ccapm_sdf`.
fn local_ccapm_sdf(consumption_growth: f64, gamma: f64, beta: f64) -> Option<f64> {
    if !(consumption_growth.is_finite() && gamma.is_finite() && beta.is_finite()) {
        return None;
    }
    if consumption_growth <= 0.0 || gamma <= 0.0 || !(0.0..=1.0).contains(&beta) {
        return None;
    }
    let m = beta * consumption_growth.powf(-gamma);
    m.is_finite().then_some(m)
}

/// Offline gravity flow — matches Host `Econ.gravity_flow`.
fn local_gravity_flow(
    mass_1: f64,
    mass_2: f64,
    distance: f64,
    alpha: f64,
    beta: f64,
    gamma: f64,
) -> Option<f64> {
    if !(mass_1.is_finite()
        && mass_2.is_finite()
        && distance.is_finite()
        && alpha.is_finite()
        && beta.is_finite()
        && gamma.is_finite())
    {
        return None;
    }
    if mass_1 < 0.0 || mass_2 < 0.0 || distance <= 0.0 {
        return None;
    }
    let flow = mass_1.powf(alpha) * mass_2.powf(beta) / distance.powf(gamma);
    flow.is_finite().then_some(flow)
}

/// Offline means-tested transfer — matches Host `Econ.transfer_payment`.
fn local_transfer_payment(
    base: f64,
    income: f64,
    threshold: f64,
    phaseout_rate: f64,
) -> Option<f64> {
    if !(base.is_finite() && income.is_finite() && threshold.is_finite() && phaseout_rate.is_finite())
    {
        return None;
    }
    if base < 0.0 || income < 0.0 || threshold < 0.0 || phaseout_rate < 0.0 {
        return None;
    }
    let excess = (income - threshold).max(0.0);
    let payment = (base - phaseout_rate * excess).max(0.0);
    payment.is_finite().then_some(payment)
}

/// Offline efficiency units — matches Host `Econ.efficiency_units`.
fn local_efficiency_units(raw_labor: f64, human_capital: f64) -> Option<f64> {
    if !(raw_labor.is_finite() && human_capital.is_finite()) {
        return None;
    }
    if raw_labor < 0.0 || human_capital < 0.0 {
        return None;
    }
    let e = raw_labor * human_capital;
    e.is_finite().then_some(e)
}

/// Offline social cost of carbon — matches Host `Econ.social_cost_of_carbon`.
fn local_social_cost_of_carbon(emissions: f64, damage_per_ton: f64) -> Option<f64> {
    if !(emissions.is_finite() && damage_per_ton.is_finite()) {
        return None;
    }
    if emissions < 0.0 || damage_per_ton < 0.0 {
        return None;
    }
    let scc = emissions * damage_per_ton;
    scc.is_finite().then_some(scc)
}

/// Offline quadratic pollution damage — matches Host `Econ.pollution_damage`.
fn local_pollution_damage(emissions: f64, damage_coeff: f64) -> Option<f64> {
    if !(emissions.is_finite() && damage_coeff.is_finite()) {
        return None;
    }
    if emissions < 0.0 || damage_coeff < 0.0 {
        return None;
    }
    let d = 0.5 * damage_coeff * emissions * emissions;
    d.is_finite().then_some(d)
}

/// Offline marginal damage — matches Host `Econ.marginal_damage`.
fn local_marginal_damage(emissions: f64, damage_coeff: f64) -> Option<f64> {
    if !(emissions.is_finite() && damage_coeff.is_finite()) {
        return None;
    }
    if emissions < 0.0 || damage_coeff < 0.0 {
        return None;
    }
    let md = damage_coeff * emissions;
    md.is_finite().then_some(md)
}

/// Offline Ramsey steady-state capital — matches Host `Econ.ramsey_steady_state`.
fn local_ramsey_steady_state(alpha: f64, beta: f64, depreciation: f64) -> Option<f64> {
    if !(alpha.is_finite() && beta.is_finite() && depreciation.is_finite()) {
        return None;
    }
    if !(0.0..=1.0).contains(&alpha) || !(0.0..=1.0).contains(&beta) || depreciation < 0.0 {
        return None;
    }
    if alpha == 0.0 || beta == 0.0 {
        return None;
    }
    let inner = (1.0 / beta - 1.0 + depreciation) / alpha;
    if inner <= 0.0 {
        return None;
    }
    let k = inner.powf(1.0 / (alpha - 1.0));
    k.is_finite().then_some(k)
}

/// Offline simple returns — last period and count (Host returns full series).
fn local_simple_returns_summary(prices: &[f64]) -> Option<(usize, f64)> {
    if prices.len() < 2 || prices.iter().any(|p| !p.is_finite() || *p <= 0.0) {
        return None;
    }
    let n = prices.len() - 1;
    let last = prices[n] / prices[n - 1] - 1.0;
    last.is_finite().then_some((n, last))
}

/// Offline log returns — last period and count (Host returns full series).
fn local_log_returns_summary(prices: &[f64]) -> Option<(usize, f64)> {
    if prices.len() < 2 || prices.iter().any(|p| !p.is_finite() || *p <= 0.0) {
        return None;
    }
    let n = prices.len() - 1;
    let last = (prices[n] / prices[n - 1]).ln();
    last.is_finite().then_some((n, last))
}

/// Offline rolling-mean last window — matches Host window semantics.
fn local_rolling_mean_last(values: &[f64], window: usize) -> Option<(usize, f64)> {
    if window == 0 || values.len() < window || values.iter().any(|v| !v.is_finite()) {
        return None;
    }
    let count = values.len() - window + 1;
    let sum: f64 = values[values.len() - window..].iter().sum();
    let mean = sum / window as f64;
    mean.is_finite().then_some((count, mean))
}

/// Offline rolling population variance for the last window.
fn local_rolling_variance_last(values: &[f64], window: usize) -> Option<(usize, f64)> {
    if window == 0 || values.len() < window || values.iter().any(|v| !v.is_finite()) {
        return None;
    }
    let count = values.len() - window + 1;
    let slice = &values[values.len() - window..];
    let mean = slice.iter().sum::<f64>() / window as f64;
    let mut var = 0.0;
    for &v in slice {
        let d = v - mean;
        var += d * d;
    }
    var /= window as f64;
    var.is_finite().then_some((count, var))
}

/// Offline Cobb–Douglas labor supply — matches Host `Econ.labor_supply`.
fn local_labor_supply(
    wage: f64,
    time_endowment: f64,
    non_labor_income: f64,
    alpha: f64,
) -> Option<(f64, f64)> {
    if !(wage.is_finite()
        && time_endowment.is_finite()
        && non_labor_income.is_finite()
        && alpha.is_finite())
    {
        return None;
    }
    if wage <= 0.0
        || time_endowment <= 0.0
        || non_labor_income < 0.0
        || !(0.0..=1.0).contains(&alpha)
    {
        return None;
    }
    let h_raw = alpha * time_endowment - (1.0 - alpha) * non_labor_income / wage;
    let h = h_raw.clamp(0.0, time_endowment);
    let c = wage * h + non_labor_income;
    (h.is_finite() && c.is_finite()).then_some((h, c))
}

/// Offline optimal pollution E* — matches Host `Econ.optimal_pollution`.
fn local_optimal_pollution(
    baseline_emissions: f64,
    abatement_coeff: f64,
    damage_coeff: f64,
) -> Option<f64> {
    if !(baseline_emissions.is_finite() && abatement_coeff.is_finite() && damage_coeff.is_finite()) {
        return None;
    }
    if baseline_emissions < 0.0 || abatement_coeff <= 0.0 || damage_coeff <= 0.0 {
        return None;
    }
    let e = abatement_coeff * baseline_emissions / (abatement_coeff + damage_coeff);
    e.is_finite().then_some(e)
}

/// Offline optimal abatement A* = E0 − E*.
fn local_optimal_abatement(
    baseline_emissions: f64,
    abatement_coeff: f64,
    damage_coeff: f64,
) -> Option<f64> {
    let e_star = local_optimal_pollution(baseline_emissions, abatement_coeff, damage_coeff)?;
    let a = baseline_emissions - e_star;
    a.is_finite().then_some(a)
}

/// Offline OLG steady-state capital/output — matches Host `Econ.olg_steady_state`.
fn local_olg_steady_state(alpha: f64, beta: f64, population_growth: f64) -> Option<(f64, f64)> {
    if !(alpha.is_finite() && beta.is_finite() && population_growth.is_finite()) {
        return None;
    }
    if !(0.0..=1.0).contains(&alpha) || !(0.0..=1.0).contains(&beta) || population_growth < 0.0 {
        return None;
    }
    if alpha >= 1.0 || beta == 0.0 {
        return None;
    }
    let rho = (1.0 - beta) / beta;
    let factor = (1.0 - alpha) / ((2.0 + rho) * (1.0 + population_growth));
    if factor <= 0.0 {
        return None;
    }
    let k = factor.powf(1.0 / (1.0 - alpha));
    let y = k.powf(alpha);
    (k.is_finite() && y.is_finite()).then_some((k, y))
}

/// Offline Ramsey Euler residual (steady-state c_t = c_{t+1} sketch).
fn local_ramsey_euler_residual(
    capital: f64,
    capital_next: f64,
    beta: f64,
    alpha: f64,
    delta: f64,
    sigma: f64,
) -> Option<f64> {
    if !(capital.is_finite()
        && capital_next.is_finite()
        && beta.is_finite()
        && alpha.is_finite()
        && delta.is_finite()
        && sigma.is_finite())
        || capital <= 0.0
    {
        return None;
    }
    let c_t = capital.powf(alpha) - capital_next + (1.0 - delta) * capital;
    if c_t <= 0.0 {
        return None;
    }
    let r = alpha * capital.powf(alpha - 1.0) + 1.0 - delta;
    let residual = 1.0 - beta * r * (c_t / c_t).powf(sigma);
    residual.is_finite().then_some(residual)
}

/// Offline present-biased utility — β-δ discounted sum.
fn local_present_biased_utility(utilities: &[f64], beta: f64, delta: f64) -> Option<f64> {
    if utilities.is_empty() || utilities.iter().any(|u| !u.is_finite()) {
        return None;
    }
    let mut total = 0.0;
    for (t, &u) in utilities.iter().enumerate() {
        let d = local_hyperbolic_discount(t as u32, beta, delta)?;
        total += d * u;
    }
    total.is_finite().then_some(total)
}

/// Offline reference-dependent utility — prospect of (x − reference).
fn local_reference_dependent_utility(
    x: f64,
    reference: f64,
    alpha: f64,
    beta: f64,
    lambda: f64,
) -> Option<f64> {
    local_prospect_value(x - reference, alpha, beta, lambda)
}

/// Offline NPV — matches Host `Econ.npv` (Σ (B−C)/(1+r)^t from t=0).
fn local_npv(benefits: &[f64], costs: &[f64], discount_rate: f64, n_periods: usize) -> Option<f64> {
    if n_periods == 0
        || benefits.len() < n_periods
        || costs.len() < n_periods
        || !discount_rate.is_finite()
        || discount_rate <= -1.0
    {
        return None;
    }
    let one_plus_r = 1.0 + discount_rate;
    let mut npv = 0.0;
    let mut discount = 1.0;
    for t in 0..n_periods {
        let b = benefits[t];
        let c = costs[t];
        if !b.is_finite() || !c.is_finite() {
            return None;
        }
        npv += (b - c) * discount;
        discount /= one_plus_r;
    }
    npv.is_finite().then_some(npv)
}

/// Offline multi-period DDM with Gordon terminal — matches Host `Econ.multi_period_ddm`.
fn local_multi_period_ddm(
    dividends: &[f64],
    discount_rate: f64,
    terminal_growth: f64,
) -> Option<f64> {
    if dividends.is_empty()
        || !discount_rate.is_finite()
        || !terminal_growth.is_finite()
        || discount_rate <= 0.0
        || terminal_growth < 0.0
        || terminal_growth >= discount_rate
        || dividends.iter().any(|d| !d.is_finite() || *d < 0.0)
    {
        return None;
    }
    let n = dividends.len();
    let r = 1.0 + discount_rate;
    let mut price = 0.0;
    for (t, &d) in dividends.iter().enumerate() {
        price += d / r.powf((t + 1) as f64);
    }
    let d_next = dividends[n - 1] * (1.0 + terminal_growth);
    let tv = d_next / (discount_rate - terminal_growth);
    price += tv / r.powf(n as f64);
    price.is_finite().then_some(price)
}

/// Offline portfolio max drawdown from returns — matches Host (signed trough).
fn local_portfolio_max_drawdown(returns: &[f64]) -> Option<f64> {
    if returns.is_empty() || returns.iter().any(|r| !r.is_finite() || *r <= -1.0) {
        return None;
    }
    let mut wealth = 1.0;
    let mut peak = 1.0;
    let mut worst = 0.0;
    for &r in returns {
        wealth *= 1.0 + r;
        if wealth > peak {
            peak = wealth;
        }
        let drawdown = wealth / peak - 1.0;
        if drawdown < worst {
            worst = drawdown;
        }
    }
    worst.is_finite().then_some(worst)
}

/// Offline linear zero-rate interpolate — matches Host `Econ.interpolate_zero_rate`.
fn local_interpolate_zero_rate(
    maturities: &[f64],
    rates: &[f64],
    target: f64,
) -> Option<f64> {
    if maturities.is_empty()
        || maturities.len() != rates.len()
        || !target.is_finite()
        || target < 0.0
        || maturities.iter().any(|t| !t.is_finite() || *t < 0.0)
        || rates.iter().any(|r| !r.is_finite())
    {
        return None;
    }
    for w in maturities.windows(2) {
        if w[1] <= w[0] {
            return None;
        }
    }
    if target <= maturities[0] {
        return Some(rates[0]);
    }
    for i in 0..maturities.len() - 1 {
        let t0 = maturities[i];
        let t1 = maturities[i + 1];
        if target <= t1 {
            let weight = (target - t0) / (t1 - t0);
            let r = rates[i] + weight * (rates[i + 1] - rates[i]);
            return r.is_finite().then_some(r);
        }
    }
    Some(rates[rates.len() - 1])
}

/// Offline DF from interpolated zero — matches Host `Econ.discount_factor`.
fn local_discount_factor(
    maturities: &[f64],
    rates: &[f64],
    target: f64,
    compounding_per_year: u32,
) -> Option<f64> {
    if compounding_per_year == 0 {
        return None;
    }
    let zero = local_interpolate_zero_rate(maturities, rates, target)?;
    let f = compounding_per_year as f64;
    let base = 1.0 + zero / f;
    if base <= 0.0 {
        return None;
    }
    let df = base.powf(-f * target);
    df.is_finite().then_some(df)
}

/// Offline par yield from zero curve — matches Host `Econ.par_yield`.
fn local_par_yield(
    maturities: &[f64],
    rates: &[f64],
    maturity_years: f64,
    coupon_frequency: u32,
) -> Option<f64> {
    if coupon_frequency == 0 || !maturity_years.is_finite() || maturity_years <= 0.0 {
        return None;
    }
    let periods_f = maturity_years * coupon_frequency as f64;
    let periods = periods_f.round() as u32;
    if periods == 0 || (periods as f64 - periods_f).abs() > 1e-9 {
        return None;
    }
    let mut annuity = 0.0;
    for period in 1..=periods {
        let t = period as f64 / coupon_frequency as f64;
        annuity += local_discount_factor(maturities, rates, t, coupon_frequency)?;
    }
    let maturity_df =
        local_discount_factor(maturities, rates, maturity_years, coupon_frequency)?;
    if annuity <= 0.0 {
        return None;
    }
    let y = coupon_frequency as f64 * (1.0 - maturity_df) / annuity;
    y.is_finite().then_some(y)
}

/// Offline progressive tax total — matches Host `Econ.progressive_tax`.
fn local_progressive_tax(
    income: f64,
    thresholds: &[f64],
    rates: &[f64],
) -> Option<(f64, f64)> {
    if thresholds.is_empty()
        || thresholds.len() != rates.len()
        || !income.is_finite()
        || income < 0.0
    {
        return None;
    }
    let mut prev = -1.0;
    for (i, &th) in thresholds.iter().enumerate() {
        let rate = rates[i];
        if !th.is_finite() || !rate.is_finite() || th < 0.0 || !(0.0..=1.0).contains(&rate) {
            return None;
        }
        if th <= prev && prev >= 0.0 {
            return None;
        }
        prev = th;
    }
    let n = thresholds.len();
    let mut total_tax = 0.0;
    for i in 0..n {
        let lower = thresholds[i];
        let upper = if i + 1 < n {
            thresholds[i + 1]
        } else {
            f64::INFINITY
        };
        let taxable = if income > lower {
            (income.min(upper) - lower).max(0.0)
        } else {
            0.0
        };
        total_tax += taxable * rates[i];
    }
    let avg = if income > 0.0 {
        total_tax / income
    } else {
        0.0
    };
    (total_tax.is_finite() && avg.is_finite()).then_some((total_tax, avg))
}

/// Offline abatement net benefit — matches Host `Econ.abatement_net_benefit`.
fn local_abatement_net_benefit(
    baseline: f64,
    actual: f64,
    abatement_coeff: f64,
    damage_coeff: f64,
) -> Option<f64> {
    if !(baseline.is_finite()
        && actual.is_finite()
        && abatement_coeff.is_finite()
        && damage_coeff.is_finite())
        || baseline < 0.0
        || actual < 0.0
        || actual > baseline
    {
        return None;
    }
    let avoided = 0.5 * damage_coeff * (baseline.powi(2) - actual.powi(2));
    let cost = 0.5 * abatement_coeff * (baseline - actual).powi(2);
    let nb = avoided - cost;
    nb.is_finite().then_some(nb)
}

/// Offline CES household production — matches Host `Econ.household_production_ces`.
fn local_household_production_ces(time: f64, goods: f64, alpha: f64, rho: f64) -> Option<f64> {
    if !(time.is_finite() && goods.is_finite() && alpha.is_finite() && rho.is_finite())
        || time < 0.0
        || goods < 0.0
        || !(0.0..=1.0).contains(&alpha)
        || rho == 0.0
    {
        return None;
    }
    let inner = alpha * time.powf(rho) + (1.0 - alpha) * goods.powf(rho);
    if inner <= 0.0 {
        return None;
    }
    let out = inner.powf(1.0 / rho);
    out.is_finite().then_some(out)
}

/// Offline malfeasance delta — matches Host `Econ.malfeasance_delta`.
fn local_malfeasance_delta(capital_allocated: f64, delivered: f64) -> Option<f64> {
    if !(capital_allocated.is_finite() && delivered.is_finite()) || capital_allocated < 0.0 {
        return None;
    }
    let delta = capital_allocated - delivered;
    delta.is_finite().then_some(delta)
}

/// Offline portfolio variance w'Σw — matches Host `Econ.portfolio_variance`.
fn local_portfolio_variance(
    weights: &[f64],
    covariance: &[f64],
    n_assets: usize,
) -> Option<f64> {
    if n_assets == 0
        || weights.len() < n_assets
        || covariance.len() < n_assets * n_assets
        || weights[..n_assets].iter().any(|w| !w.is_finite())
        || covariance[..n_assets * n_assets]
            .iter()
            .any(|c| !c.is_finite())
    {
        return None;
    }
    let mut variance = 0.0;
    for row in 0..n_assets {
        for col in 0..n_assets {
            variance += weights[row] * covariance[row * n_assets + col] * weights[col];
        }
    }
    if variance < 0.0 && variance > -1e-15 {
        return Some(0.0);
    }
    (variance >= 0.0 && variance.is_finite()).then_some(variance)
}

/// Offline portfolio returns (row-major periods × assets) — matches Host.
fn local_portfolio_returns_summary(
    asset_returns: &[f64],
    weights: &[f64],
    n_periods: usize,
    n_assets: usize,
) -> Option<(usize, f64)> {
    if n_periods == 0
        || n_assets == 0
        || weights.len() < n_assets
        || asset_returns.len() < n_periods * n_assets
        || weights[..n_assets].iter().any(|w| !w.is_finite())
        || asset_returns[..n_periods * n_assets]
            .iter()
            .any(|r| !r.is_finite())
    {
        return None;
    }
    let mut last = 0.0;
    for period in 0..n_periods {
        let offset = period * n_assets;
        let mut value = 0.0;
        for asset in 0..n_assets {
            value += asset_returns[offset + asset] * weights[asset];
        }
        last = value;
    }
    last.is_finite().then_some((n_periods, last))
}

/// Offline distributional NPV — matches Host `Econ.distributional_npv`.
fn local_distributional_npv(
    benefits: &[f64],
    costs: &[f64],
    weights: &[f64],
    discount_rate: f64,
    n_periods: usize,
) -> Option<(f64, f64)> {
    if n_periods == 0
        || benefits.len() < n_periods
        || costs.len() < n_periods
        || weights.len() < n_periods
        || !discount_rate.is_finite()
        || discount_rate <= -1.0
    {
        return None;
    }
    let one_plus_r = 1.0 + discount_rate;
    let mut weighted = 0.0;
    let mut unweighted = 0.0;
    let mut discount = 1.0;
    for t in 0..n_periods {
        let b = benefits[t];
        let c = costs[t];
        let w = weights[t];
        if !b.is_finite() || !c.is_finite() || !w.is_finite() || w < 0.0 {
            return None;
        }
        let flow = (b - c) * discount;
        weighted += w * flow;
        unweighted += flow;
        discount /= one_plus_r;
    }
    (weighted.is_finite() && unweighted.is_finite()).then_some((weighted, unweighted))
}

/// Offline stress scenario (scale series by shock) — matches Host.
fn local_stress_scenario(returns: &[f64], shock: f64) -> Option<(usize, f64)> {
    if returns.is_empty() || !shock.is_finite() || returns.iter().any(|r| !r.is_finite()) {
        return None;
    }
    let last = returns[returns.len() - 1] * shock;
    last.is_finite().then_some((returns.len(), last))
}

/// Offline repeated-game geometric payoff — matches Host.
fn local_repeated_game_payoff(
    stage_payoffs: &[f64],
    discount: f64,
    n_rounds: usize,
) -> Option<f64> {
    if n_rounds == 0
        || stage_payoffs.len() < n_rounds
        || !discount.is_finite()
        || discount < 0.0
        || stage_payoffs[..n_rounds].iter().any(|p| !p.is_finite())
    {
        return None;
    }
    let mut total = 0.0;
    let mut weight = 1.0;
    for t in 0..n_rounds {
        total += stage_payoffs[t] * weight;
        weight *= discount;
    }
    total.is_finite().then_some(total)
}

/// Offline total transport cost Σ flow·distance — matches Host.
fn local_total_transport_cost(flows: &[f64], distances: &[f64], n: usize) -> Option<f64> {
    let need = n * n;
    if n == 0 || flows.len() < need || distances.len() < need {
        return None;
    }
    let mut total = 0.0;
    for i in 0..need {
        let f = flows[i];
        let d = distances[i];
        if !f.is_finite() || !d.is_finite() {
            return None;
        }
        total += f * d;
    }
    total.is_finite().then_some(total)
}

/// Offline Markov P[from,to] — matches Host `Econ.transition_probability`.
fn local_transition_probability(
    matrix: &[f64],
    n: usize,
    from: usize,
    to: usize,
) -> Option<f64> {
    if n == 0 || from >= n || to >= n || matrix.len() < n * n {
        return None;
    }
    let p = matrix[from * n + to];
    p.is_finite().then_some(p)
}

/// Offline expected holding time 1/(1−P_ii) — matches Host.
fn local_expected_holding_time(matrix: &[f64], n: usize, state: usize) -> Option<f64> {
    if n == 0 || state >= n || matrix.len() < n * n {
        return None;
    }
    let self_loop = matrix[state * n + state];
    if !self_loop.is_finite() {
        return None;
    }
    let denom = 1.0 - self_loop;
    if denom <= 0.0 {
        return None;
    }
    let t = 1.0 / denom;
    t.is_finite().then_some(t)
}

/// Offline individual rationality (payments ≤ valuations) — matches Host.
fn local_check_ir(valuations: &[f64], payments: &[f64]) -> Option<bool> {
    if valuations.is_empty() || valuations.len() != payments.len() {
        return None;
    }
    for i in 0..valuations.len() {
        let v = valuations[i];
        let p = payments[i];
        if !v.is_finite() || !p.is_finite() {
            return None;
        }
        if p > v {
            return Some(false);
        }
    }
    Some(true)
}

/// Offline Vickrey (second-price) payment — matches Host `Econ.vcg_payment`.
fn local_vcg_payment(valuations: &[f64]) -> Option<(f64, f64)> {
    if valuations.is_empty() || valuations.iter().any(|v| !v.is_finite() || *v < 0.0) {
        return None;
    }
    let mut winner = 0;
    let mut highest = valuations[0];
    for (i, &v) in valuations.iter().enumerate().skip(1) {
        if v > highest {
            highest = v;
            winner = i;
        }
    }
    let mut second = 0.0;
    for (i, &v) in valuations.iter().enumerate() {
        if i == winner {
            continue;
        }
        if v > second {
            second = v;
        }
    }
    Some((second, second))
}

/// Offline row-stochastic check — matches Host `Econ.validate_transition_matrix`.
fn local_validate_transition_matrix(matrix: &[f64], n: usize) -> Option<bool> {
    if n == 0 || n > 32 || matrix.len() < n * n {
        return None;
    }
    for row in 0..n {
        let mut sum = 0.0;
        for col in 0..n {
            let entry = matrix[row * n + col];
            if !entry.is_finite() || entry < 0.0 {
                return Some(false);
            }
            sum += entry;
        }
        if (sum - 1.0).abs() > 1e-9 {
            return Some(false);
        }
    }
    Some(true)
}

/// Offline stationary π via power iteration — matches Host sketch.
fn local_stationary_distribution(matrix: &[f64], n: usize) -> Option<Vec<f64>> {
    if local_validate_transition_matrix(matrix, n) != Some(true) || n > 8 {
        return None;
    }
    let mut pi = vec![1.0 / n as f64; n];
    let mut next = vec![0.0; n];
    for _ in 0..1_000 {
        for j in 0..n {
            let mut acc = 0.0;
            for i in 0..n {
                acc += pi[i] * matrix[i * n + j];
            }
            next[j] = acc;
        }
        let mut diff = 0.0_f64;
        for i in 0..n {
            diff = diff.max((next[i] - pi[i]).abs());
            pi[i] = next[i];
        }
        if diff < 1e-9 {
            return Some(pi);
        }
    }
    Some(pi)
}

/// Offline mean first-passage times to `target` — matches Host sketch.
fn local_mean_first_passage(matrix: &[f64], n: usize, target: usize) -> Option<Vec<f64>> {
    if local_validate_transition_matrix(matrix, n) != Some(true) || target >= n || n > 8 {
        return None;
    }
    let mut m = vec![0.0; n];
    let mut next = vec![0.0; n];
    for _ in 0..1_000 {
        let mut diff = 0.0_f64;
        for i in 0..n {
            if i == target {
                next[i] = 0.0;
                continue;
            }
            let mut s = 1.0;
            for j in 0..n {
                if j == target {
                    continue;
                }
                s += matrix[i * n + j] * m[j];
            }
            if !s.is_finite() {
                return None;
            }
            next[i] = s;
            diff = diff.max((s - m[i]).abs());
        }
        m.copy_from_slice(&next);
        if diff < 1e-9 {
            return Some(m);
        }
    }
    Some(m)
}

/// Offline out-degree centrality — matches Host `Econ.degree_centrality`.
fn local_degree_centrality(adjacency: &[f64], n: usize) -> Option<Vec<f64>> {
    if n == 0 || n > 32 || adjacency.len() < n * n {
        return None;
    }
    if adjacency[..n * n].iter().any(|x| !x.is_finite()) {
        return None;
    }
    let mut out = vec![0.0; n];
    for i in 0..n {
        let mut acc = 0.0;
        for j in 0..n {
            acc += adjacency[i * n + j];
        }
        out[i] = acc;
    }
    Some(out)
}

/// Offline eigenvector centrality (A+I power iter) — matches Host sketch.
fn local_eigenvector_centrality(adjacency: &[f64], n: usize) -> Option<Vec<f64>> {
    if n == 0 || n > 8 || adjacency.len() < n * n {
        return None;
    }
    if adjacency[..n * n].iter().any(|x| !x.is_finite()) {
        return None;
    }
    let init = (1.0 / n as f64).sqrt();
    let mut out = vec![init; n];
    let mut next = vec![0.0; n];
    for _ in 0..100 {
        for i in 0..n {
            let mut acc = out[i];
            for j in 0..n {
                acc += adjacency[i * n + j] * out[j];
            }
            next[i] = acc;
        }
        let mut norm = 0.0;
        for v in &next {
            norm += v * v;
        }
        norm = norm.sqrt();
        if !(norm > 0.0) {
            return None;
        }
        let mut diff = 0.0_f64;
        for i in 0..n {
            let v = next[i] / norm;
            diff = diff.max((v - out[i]).abs());
            out[i] = v;
        }
        if diff < 1e-6 {
            return Some(out);
        }
    }
    Some(out)
}

/// Offline New Keynesian 1-step solve — matches Host `Econ.new_keynesian_solve`.
fn local_new_keynesian_solve(
    r_prev: f64,
    beta: f64,
    kappa: f64,
    sigma: f64,
    phi_pi: f64,
    phi_y: f64,
    rho_r: f64,
    r_nat: f64,
) -> Option<(f64, f64, f64)> {
    if !(0.0..=1.0).contains(&beta)
        || kappa < 0.0
        || sigma <= 0.0
        || phi_pi < 0.0
        || phi_y < 0.0
        || !(0.0..=1.0).contains(&rho_r)
    {
        return None;
    }
    let a = (1.0 - rho_r) * (phi_pi * kappa + phi_y);
    let denom = sigma + a;
    if denom.abs() < 1e-14 {
        return None;
    }
    let y_gap = -(rho_r * r_prev - r_nat) / denom;
    let pi = kappa * y_gap;
    let r = rho_r * r_prev + (1.0 - rho_r) * (phi_pi * pi + phi_y * y_gap);
    if y_gap.is_finite() && pi.is_finite() && r.is_finite() {
        Some((y_gap, pi, r))
    } else {
        None
    }
}

/// Offline nearest-facility assignment — matches Host `Econ.nearest_facility`.
fn local_nearest_facility(
    demands: &[f64],
    facilities: &[f64],
    n_demands: usize,
    n_facilities: usize,
) -> Option<Vec<usize>> {
    if n_demands == 0
        || n_facilities == 0
        || n_demands > 64
        || n_facilities > 64
        || demands.len() < n_demands * 2
        || facilities.len() < n_facilities * 2
    {
        return None;
    }
    let mut out = vec![0usize; n_demands];
    for d in 0..n_demands {
        let dx = demands[d * 2];
        let dy = demands[d * 2 + 1];
        if !dx.is_finite() || !dy.is_finite() {
            return None;
        }
        let mut best = 0usize;
        let mut best_dist = f64::INFINITY;
        for f in 0..n_facilities {
            let fx = facilities[f * 2];
            let fy = facilities[f * 2 + 1];
            if !fx.is_finite() || !fy.is_finite() {
                return None;
            }
            let dist = (dx - fx).powi(2) + (dy - fy).powi(2);
            if dist < best_dist {
                best_dist = dist;
                best = f;
            }
        }
        out[d] = best;
    }
    Some(out)
}

/// Offline pure Nash equilibria — matches Host `Econ.pure_nash_equilibria`.
fn local_pure_nash_equilibria(
    payoff_row: &[f64],
    payoff_col: &[f64],
    n_row: usize,
    n_col: usize,
) -> Option<Vec<(usize, usize)>> {
    if n_row == 0
        || n_col == 0
        || n_row > 8
        || n_col > 8
        || payoff_row.len() < n_row * n_col
        || payoff_col.len() < n_row * n_col
    {
        return None;
    }
    if payoff_row[..n_row * n_col]
        .iter()
        .chain(payoff_col[..n_row * n_col].iter())
        .any(|x| !x.is_finite())
    {
        return None;
    }
    let mut row_best = vec![false; n_row * n_col];
    let mut col_best = vec![false; n_row * n_col];
    for c in 0..n_col {
        let mut best_val = f64::NEG_INFINITY;
        for r in 0..n_row {
            best_val = best_val.max(payoff_row[r * n_col + c]);
        }
        for r in 0..n_row {
            if payoff_row[r * n_col + c] >= best_val {
                row_best[r * n_col + c] = true;
            }
        }
    }
    for r in 0..n_row {
        let mut best_val = f64::NEG_INFINITY;
        for c in 0..n_col {
            best_val = best_val.max(payoff_col[r * n_col + c]);
        }
        for c in 0..n_col {
            if payoff_col[r * n_col + c] >= best_val {
                col_best[r * n_col + c] = true;
            }
        }
    }
    let mut eqs = Vec::new();
    for r in 0..n_row {
        for c in 0..n_col {
            let idx = r * n_col + c;
            if row_best[idx] && col_best[idx] {
                eqs.push((r, c));
            }
        }
    }
    Some(eqs)
}

/// Offline Moran's I — matches Host `Econ.morans_i`.
fn local_morans_i(values: &[f64], weights: &[f64], n: usize) -> Option<f64> {
    if n < 2 || n > 32 || values.len() < n || weights.len() < n * n {
        return None;
    }
    if values[..n].iter().any(|v| !v.is_finite()) {
        return None;
    }
    let mean: f64 = values[..n].iter().sum::<f64>() / n as f64;
    let mut s0 = 0.0;
    let mut numerator = 0.0;
    for i in 0..n {
        for j in 0..n {
            let w = weights[i * n + j];
            if !w.is_finite() {
                return None;
            }
            s0 += w;
            numerator += w * (values[i] - mean) * (values[j] - mean);
        }
    }
    let mut denominator = 0.0;
    for i in 0..n {
        denominator += (values[i] - mean).powi(2);
    }
    if s0 == 0.0 || denominator == 0.0 {
        return None;
    }
    let i = (n as f64 / s0) * (numerator / denominator);
    i.is_finite().then_some(i)
}

/// Offline 2×2 strategy-proofness check — matches Host.
fn local_strategy_proofness(
    valuations: &[f64],
    allocation: &[bool],
    payments: &[f64],
) -> Option<bool> {
    if valuations.len() < 4 || allocation.len() < 4 || payments.len() < 4 {
        return None;
    }
    if valuations[..4].iter().any(|v| !v.is_finite()) || payments[..4].iter().any(|p| !p.is_finite())
    {
        return None;
    }
    for true_type in 0..2 {
        for opponent_type in 0..2 {
            let truthful_alloc = allocation[true_type * 2 + opponent_type];
            let truthful_payment = payments[true_type * 2 + opponent_type];
            let truthful_utility = if truthful_alloc {
                valuations[true_type] - truthful_payment
            } else {
                -truthful_payment
            };
            let misreport_type = 1 - true_type;
            let misreport_alloc = allocation[misreport_type * 2 + opponent_type];
            let misreport_payment = payments[misreport_type * 2 + opponent_type];
            let misreport_utility = if misreport_alloc {
                valuations[true_type] - misreport_payment
            } else {
                -misreport_payment
            };
            if misreport_utility > truthful_utility + 1e-10 {
                return Some(false);
            }
        }
    }
    Some(true)
}

/// Offline Lorenz last population/income share — matches Host `Econ.lorenz_curve`.
fn local_lorenz_curve(incomes: &[f64]) -> Option<(f64, f64)> {
    if incomes.len() < 2 || incomes.iter().any(|x| !x.is_finite() || *x < 0.0) {
        return None;
    }
    let mut sorted = incomes.to_vec();
    sorted.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
    let total: f64 = sorted.iter().sum();
    if total <= 0.0 {
        return None;
    }
    let n = sorted.len() as f64;
    let mut cum = 0.0;
    for (i, x) in sorted.iter().enumerate() {
        cum += x;
        if i + 1 == sorted.len() {
            return Some(((i + 1) as f64 / n, cum / total));
        }
    }
    None
}

/// Offline simple OLS (n_reg=1 through origin) — matches Host `Econ.ols` sketch.
fn local_ols(x: &[f64], y: &[f64]) -> Option<(f64, f64)> {
    if x.len() < 2 || y.len() < 2 || x.len() != y.len() {
        return None;
    }
    if x.iter().any(|v| !v.is_finite()) || y.iter().any(|v| !v.is_finite()) {
        return None;
    }
    let n = x.len();
    let mut xtx = 0.0;
    let mut xty = 0.0;
    for i in 0..n {
        xtx += x[i] * x[i];
        xty += x[i] * y[i];
    }
    if xtx.abs() < 1e-15 {
        return None;
    }
    let beta = xty / xtx;
    let y_mean = y.iter().sum::<f64>() / n as f64;
    let mut tss = 0.0;
    let mut rss = 0.0;
    for i in 0..n {
        let resid = y[i] - beta * x[i];
        rss += resid * resid;
        let d = y[i] - y_mean;
        tss += d * d;
    }
    let r2 = if tss > 0.0 { 1.0 - rss / tss } else { 0.0 };
    r2.is_finite().then_some((beta, r2))
}

/// Offline WLS (n_reg=1) — matches Host `Econ.wls` sketch.
fn local_wls(x: &[f64], y: &[f64], weights: &[f64]) -> Option<(f64, f64)> {
    if x.len() < 2 || x.len() != y.len() || y.len() != weights.len() {
        return None;
    }
    if x.iter().any(|v| !v.is_finite())
        || y.iter().any(|v| !v.is_finite())
        || weights.iter().any(|w| !w.is_finite() || *w < 0.0)
    {
        return None;
    }
    let n = x.len();
    let mut xtwx = 0.0;
    let mut xtwy = 0.0;
    let mut sw = 0.0;
    let mut swy = 0.0;
    for i in 0..n {
        xtwx += x[i] * weights[i] * x[i];
        xtwy += x[i] * weights[i] * y[i];
        sw += weights[i];
        swy += weights[i] * y[i];
    }
    if xtwx.abs() < 1e-15 || sw <= 0.0 {
        return None;
    }
    let beta = xtwy / xtwx;
    let y_wmean = swy / sw;
    let mut rss = 0.0;
    let mut wss = 0.0;
    for i in 0..n {
        let resid = y[i] - beta * x[i];
        rss += weights[i] * resid * resid;
        let d = y[i] - y_wmean;
        wss += weights[i] * d * d;
    }
    let r2 = if wss > 0.0 { 1.0 - rss / wss } else { 0.0 };
    r2.is_finite().then_some((beta, r2))
}

/// Offline Lucas tree price — matches Host `Econ.lucas_asset_price`.
fn local_lucas_asset_price(
    dividends: &[f64],
    consumption: &[f64],
    n_paths: usize,
    n_periods: usize,
    beta: f64,
    gamma: f64,
) -> Option<f64> {
    if n_paths == 0
        || n_periods == 0
        || !(0.0..=1.0).contains(&beta)
        || !(gamma > 0.0)
        || !beta.is_finite()
        || !gamma.is_finite()
    {
        return None;
    }
    if dividends.len() < n_paths * n_periods || consumption.len() < n_paths * n_periods {
        return None;
    }
    let mut total = 0.0;
    for p in 0..n_paths {
        let c0 = consumption[p * n_periods];
        if !(c0 > 0.0) {
            return None;
        }
        let mut price = 0.0;
        for t in 0..n_periods {
            let d = dividends[p * n_periods + t];
            let c = consumption[p * n_periods + t];
            if !d.is_finite() || !c.is_finite() || !(c > 0.0) {
                return None;
            }
            let m = (c / c0).powf(-gamma);
            price += beta.powi(t as i32) * m * d;
        }
        total += price;
    }
    let avg = total / n_paths as f64;
    avg.is_finite().then_some(avg)
}

/// Offline Bellman one-state update — matches Host `Econ.bellman_update`.
fn local_bellman_update(
    rewards: &[f64],
    transitions: &[f64],
    discount: f64,
    values: &[f64],
    n_states: usize,
    n_actions: usize,
    state: usize,
) -> Option<f64> {
    if n_states == 0
        || n_actions == 0
        || state >= n_states
        || !(0.0..1.0).contains(&discount)
        || rewards.len() < n_states * n_actions
        || transitions.len() < n_states * n_actions * n_states
        || values.len() < n_states
    {
        return None;
    }
    if rewards[..n_states * n_actions].iter().any(|r| !r.is_finite())
        || values[..n_states].iter().any(|v| !v.is_finite())
        || transitions[..n_states * n_actions * n_states]
            .iter()
            .any(|p| !p.is_finite() || *p < 0.0)
    {
        return None;
    }
    let mut best = f64::NEG_INFINITY;
    for a in 0..n_actions {
        let mut cont = 0.0;
        for sp in 0..n_states {
            let p = transitions[state * n_actions * n_states + a * n_states + sp];
            cont += p * values[sp];
        }
        let q = rewards[state * n_actions + a] + discount * cont;
        if !q.is_finite() {
            return None;
        }
        if q > best {
            best = q;
        }
    }
    best.is_finite().then_some(best)
}

/// Offline block-bootstrap mean sketch — series mean as Host `Econ.block_bootstrap` proxy.
fn local_block_bootstrap_mean_sketch(
    returns: &[f64],
    block_size: usize,
    n_resamples: usize,
) -> Option<(f64, usize)> {
    if returns.len() < 2
        || block_size == 0
        || block_size > returns.len()
        || n_resamples == 0
        || returns.iter().any(|r| !r.is_finite())
    {
        return None;
    }
    let mean = returns.iter().sum::<f64>() / returns.len() as f64;
    mean.is_finite().then_some((mean, n_resamples))
}

/// Offline greedy Markov path — matches Host `Econ.simulate_chain` sketch (argmax).
fn local_simulate_chain_greedy(
    matrix: &[f64],
    n: usize,
    start: usize,
    n_steps: usize,
) -> Option<Vec<usize>> {
    if local_validate_transition_matrix(matrix, n) != Some(true) || start >= n || n_steps > 64 {
        return None;
    }
    let mut path = Vec::with_capacity(n_steps + 1);
    let mut state = start;
    path.push(state);
    for _ in 0..n_steps {
        let base = state * n;
        let mut best_col = 0usize;
        let mut best_p = f64::NEG_INFINITY;
        for col in 0..n {
            let p = matrix[base + col];
            if p > best_p {
                best_p = p;
                best_col = col;
            }
        }
        state = best_col;
        path.push(state);
    }
    Some(path)
}

/// Offline value iteration (small MDP) — matches Host `Econ.value_iteration`.
fn local_value_iteration(
    rewards: &[f64],
    transitions: &[f64],
    discount: f64,
    n_states: usize,
    n_actions: usize,
    max_iter: u32,
    tolerance: f64,
) -> Option<(Vec<f64>, Vec<u32>)> {
    if n_states == 0
        || n_actions == 0
        || n_states > 8
        || !(0.0..1.0).contains(&discount)
        || !(tolerance > 0.0)
        || max_iter == 0
        || rewards.len() < n_states * n_actions
        || transitions.len() < n_states * n_actions * n_states
    {
        return None;
    }
    let mut values = vec![0.0; n_states];
    let mut next = vec![0.0; n_states];
    let mut policy = vec![0u32; n_states];
    for _ in 0..max_iter {
        let mut residual = 0.0_f64;
        for s in 0..n_states {
            let mut best = f64::NEG_INFINITY;
            let mut best_a = 0u32;
            for a in 0..n_actions {
                let mut cont = 0.0;
                for sp in 0..n_states {
                    cont += transitions[s * n_actions * n_states + a * n_states + sp] * values[sp];
                }
                let q = rewards[s * n_actions + a] + discount * cont;
                if q > best {
                    best = q;
                    best_a = a as u32;
                }
            }
            next[s] = best;
            policy[s] = best_a;
            residual = residual.max((best - values[s]).abs());
        }
        values.copy_from_slice(&next);
        if residual < tolerance {
            break;
        }
    }
    Some((values, policy))
}

/// Offline IV/2SLS (n_reg=n_instr=1) — matches Host `Econ.iv_2sls` sketch.
fn local_iv_2sls(x: &[f64], z: &[f64], y: &[f64]) -> Option<(f64, f64)> {
    if x.len() < 2 || x.len() != z.len() || z.len() != y.len() {
        return None;
    }
    if x.iter().chain(z.iter()).chain(y.iter()).any(|v| !v.is_finite()) {
        return None;
    }
    let n = x.len();
    // First stage: x = gamma * z
    let mut ztz = 0.0;
    let mut ztx = 0.0;
    for i in 0..n {
        ztz += z[i] * z[i];
        ztx += z[i] * x[i];
    }
    if ztz.abs() < 1e-15 {
        return None;
    }
    let gamma = ztx / ztz;
    let mut x_hat = vec![0.0; n];
    for i in 0..n {
        x_hat[i] = gamma * z[i];
    }
    local_ols(&x_hat, y)
}

/// Offline logistic MLE (n_reg=1, Newton) — matches Host `Econ.logistic_mle` sketch.
fn local_logistic_mle(x: &[f64], y: &[f64]) -> Option<f64> {
    if x.len() < 2 || x.len() != y.len() {
        return None;
    }
    if x.iter().any(|v| !v.is_finite())
        || y.iter()
            .any(|v| !v.is_finite() || (*v != 0.0 && *v != 1.0))
    {
        return None;
    }
    let n = x.len();
    let mut beta = 0.0;
    for _ in 0..50 {
        let mut score = 0.0;
        let mut info = 0.0;
        for i in 0..n {
            let eta = (beta * x[i]).clamp(-20.0, 20.0);
            let p = 1.0 / (1.0 + (-eta).exp());
            score += x[i] * (y[i] - p);
            info += x[i] * x[i] * p * (1.0 - p);
        }
        if info.abs() < 1e-15 {
            return None;
        }
        let step = score / info;
        beta += step;
        if step.abs() < 1e-8 {
            break;
        }
    }
    beta.is_finite().then_some(beta)
}

/// Offline Eisenberg–Noe interbank clearing — matches Host `Econ.interbank_clearing`.
fn local_interbank_clearing(
    exposures: &[f64],
    capital: &[f64],
    n: usize,
) -> Option<Vec<f64>> {
    if n == 0 || n > 8 || exposures.len() < n * n || capital.len() < n {
        return None;
    }
    if exposures[..n * n].iter().any(|x| !x.is_finite())
        || capital[..n].iter().any(|x| !x.is_finite())
    {
        return None;
    }
    let mut total_liab = vec![0.0; n];
    for i in 0..n {
        let mut acc = 0.0;
        for j in 0..n {
            acc += exposures[i * n + j];
        }
        total_liab[i] = acc;
    }
    let mut payments = total_liab.clone();
    let mut received = vec![0.0; n];
    let mut new_payments = vec![0.0; n];
    for _ in 0..200 {
        for i in 0..n {
            let mut acc = capital[i];
            for j in 0..n {
                if total_liab[j] > 0.0 {
                    acc += payments[j] * exposures[j * n + i] / total_liab[j];
                }
            }
            received[i] = acc;
        }
        let mut delta = 0.0;
        for i in 0..n {
            new_payments[i] = total_liab[i].min(received[i]).max(0.0);
            delta += (new_payments[i] - payments[i]).abs();
        }
        payments.copy_from_slice(&new_payments);
        if delta < 1e-9 {
            return Some(payments);
        }
    }
    Some(payments)
}

/// Offline Leontief (I−A)⁻¹ via Neumann series — matches Host `Econ.leontief_inverse`.
fn local_leontief_inverse(a_matrix: &[f64], n: usize) -> Option<Vec<f64>> {
    if n == 0 || n > 8 || a_matrix.len() < n * n {
        return None;
    }
    if a_matrix[..n * n].iter().any(|x| !x.is_finite()) {
        return None;
    }
    let need = n * n;
    let mut out = vec![0.0; need];
    for i in 0..n {
        out[i * n + i] = 1.0;
    }
    let mut power = a_matrix[..need].to_vec();
    for i in 0..need {
        out[i] += power[i];
    }
    let mut next_power = vec![0.0; need];
    for _ in 0..200 {
        for i in 0..n {
            for j in 0..n {
                let mut acc = 0.0;
                for k in 0..n {
                    acc += power[i * n + k] * a_matrix[k * n + j];
                }
                next_power[i * n + j] = acc;
            }
        }
        let mut l1 = 0.0;
        for i in 0..need {
            l1 += next_power[i].abs();
            out[i] += next_power[i];
            power[i] = next_power[i];
        }
        if l1 < 1e-12 {
            return out.iter().all(|x| x.is_finite()).then_some(out);
        }
        if !l1.is_finite() || l1 > 1e18 {
            return None;
        }
    }
    out.iter().all(|x| x.is_finite()).then_some(out)
}

/// Offline output multipliers (column sums of Leontief inverse).
fn local_output_multipliers(inverse: &[f64], n: usize) -> Option<Vec<f64>> {
    if n == 0 || n > 8 || inverse.len() < n * n {
        return None;
    }
    if inverse[..n * n].iter().any(|x| !x.is_finite()) {
        return None;
    }
    let mut out = vec![0.0; n];
    for j in 0..n {
        let mut acc = 0.0;
        for i in 0..n {
            acc += inverse[i * n + j];
        }
        out[j] = acc;
    }
    out.iter().all(|x| x.is_finite()).then_some(out)
}

/// Offline scalar econ constraint check — matches Host `Econ.validate_scalar_constraint`.
fn local_validate_scalar_constraint(value: f64, min: f64, max: f64) -> Option<bool> {
    if !(value.is_finite() && min.is_finite() && max.is_finite()) || min > max {
        return None;
    }
    Some(value >= min && value <= max)
}

/// Offline sample covariance for small asset count (row-major periods × assets).
fn local_covariance_matrix(
    returns: &[f64],
    n_periods: usize,
    n_assets: usize,
) -> Option<Vec<f64>> {
    if n_periods < 2 || n_assets < 1 || n_assets > 8 {
        return None;
    }
    if returns.len() < n_periods * n_assets {
        return None;
    }
    if returns[..n_periods * n_assets]
        .iter()
        .any(|x| !x.is_finite())
    {
        return None;
    }
    let mut means = vec![0.0; n_assets];
    for period in 0..n_periods {
        let offset = period * n_assets;
        for asset in 0..n_assets {
            means[asset] += returns[offset + asset];
        }
    }
    for m in means.iter_mut() {
        *m /= n_periods as f64;
    }
    let mut out = vec![0.0; n_assets * n_assets];
    let denom = (n_periods - 1) as f64;
    for row in 0..n_assets {
        for col in row..n_assets {
            let mut sum = 0.0;
            for period in 0..n_periods {
                let offset = period * n_assets;
                let a = returns[offset + row] - means[row];
                let b = returns[offset + col] - means[col];
                sum += a * b;
            }
            let cov = sum / denom;
            out[row * n_assets + col] = cov;
            out[col * n_assets + row] = cov;
        }
    }
    Some(out)
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

/// `Econ.binomial_option` — CRR tree params from data-* attrs (defaults illustrative).
pub(super) fn run_binomial_option(document: &Document, label: &str) {
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
    let steps = numeric_attr(container.as_ref(), "data-steps")
        .map(|s| s.round().max(1.0) as u64)
        .unwrap_or(100);
    let is_call = container
        .as_ref()
        .and_then(|c| c.get_attribute("data-is-call"))
        .map(|v| v != "0" && !v.eq_ignore_ascii_case("false"))
        .unwrap_or(true);
    let sketch = match local_binomial_one_step(
        spot,
        strike,
        time_to_expiry,
        risk_free_rate,
        volatility,
        dividend_yield,
        is_call,
    ) {
        Some(price) => format!(
            "Binomial one-step sketch: S={spot} K={strike} T={time_to_expiry} σ={volatility} call={is_call} → ≈{price:.3} (live uses {steps} steps)"
        ),
        None => format!(
            "Binomial sketch inputs S={spot} K={strike} T={time_to_expiry} r={risk_free_rate} σ={volatility} steps={steps}"
        ),
    };
    invoke_dual(
        document,
        label,
        "Econ.binomial_option",
        sketch,
        json!({
            "spot": spot,
            "strike": strike,
            "time_to_expiry": time_to_expiry,
            "risk_free_rate": risk_free_rate,
            "volatility": volatility,
            "dividend_yield": dividend_yield,
            "steps": steps,
            "is_call": is_call,
        }),
    );
}

/// `Econ.forward_rate` — maturities/rates from surface pairs, or illustrative curve.
pub(super) fn run_forward_rate(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let (maturities, rates) = if nums.len() >= 4 && nums.len() % 2 == 0 {
        let half = nums.len() / 2;
        (nums[..half].to_vec(), nums[half..].to_vec())
    } else {
        (
            vec![0.5, 1.0, 2.0, 5.0],
            vec![0.03, 0.035, 0.04, 0.045],
        )
    };
    let t1 = numeric_attr(container.as_ref(), "data-t1").unwrap_or(1.0);
    let t2 = numeric_attr(container.as_ref(), "data-t2").unwrap_or(2.0);
    let compounding = numeric_attr(container.as_ref(), "data-compounding-per-year")
        .map(|c| c.round().max(1.0) as u64)
        .unwrap_or(1);
    let sketch = {
        // Prefer nearest curve points around t1/t2 for a continuous-style sketch.
        let r1 = rates
            .iter()
            .zip(maturities.iter())
            .min_by(|a, b| {
                (a.1 - t1)
                    .abs()
                    .partial_cmp(&(b.1 - t1).abs())
                    .unwrap_or(core::cmp::Ordering::Equal)
            })
            .map(|(r, _)| *r)
            .unwrap_or(0.03);
        let r2 = rates
            .iter()
            .zip(maturities.iter())
            .min_by(|a, b| {
                (a.1 - t2)
                    .abs()
                    .partial_cmp(&(b.1 - t2).abs())
                    .unwrap_or(core::cmp::Ordering::Equal)
            })
            .map(|(r, _)| *r)
            .unwrap_or(0.04);
        match local_forward_rate_two_point(r1, t1, r2, t2) {
            Some(f) => format!(
                "Forward-rate sketch t1={t1} t2={t2} from zeros≈({r1:.4},{r2:.4}) → {f:.4} (offline continuous; live uses curve)"
            ),
            None => format!(
                "Forward-rate sketch inputs maturities={maturities:?} rates={rates:?} t1={t1} t2={t2}"
            ),
        }
    };
    invoke_dual(
        document,
        label,
        "Econ.forward_rate",
        sketch,
        json!({
            "maturities": maturities,
            "rates": rates,
            "t1": t1,
            "t2": t2,
            "compounding_per_year": compounding,
        }),
    );
}

/// `Econ.gbm_simulate` — path params from data-* attrs.
pub(super) fn run_gbm_simulate(document: &Document, label: &str) {
    let container = selected_container(document);
    let s0 = numeric_attr(container.as_ref(), "data-s0").unwrap_or(100.0);
    let mu = numeric_attr(container.as_ref(), "data-mu").unwrap_or(0.05);
    let sigma = numeric_attr(container.as_ref(), "data-sigma").unwrap_or(0.2);
    let dt = numeric_attr(container.as_ref(), "data-dt").unwrap_or(1.0 / 252.0);
    let n_steps = numeric_attr(container.as_ref(), "data-n-steps")
        .map(|n| n.round().max(1.0) as u64)
        .unwrap_or(64);
    let seed = numeric_attr(container.as_ref(), "data-seed")
        .map(|s| s.round() as u64)
        .unwrap_or(42);
    let sketch = match local_gbm_drift_endpoint(s0, mu, sigma, dt, n_steps as usize) {
        Some(end) => format!(
            "GBM drift-endpoint sketch: S0={s0} μ={mu} σ={sigma} dt={dt} n={n_steps} → E[S_T]≈{end:.3} (live is seeded RNG path)"
        ),
        None => format!(
            "GBM sketch inputs S0={s0} μ={mu} σ={sigma} dt={dt} n_steps={n_steps}"
        ),
    };
    invoke_dual(
        document,
        label,
        "Econ.gbm_simulate",
        sketch,
        json!({
            "s0": s0,
            "mu": mu,
            "sigma": sigma,
            "dt": dt,
            "n_steps": n_steps,
            "seed": seed,
        }),
    );
}

/// `Econ.headcount_poverty` — incomes from surface; `data-poverty-line`.
pub(super) fn run_headcount_poverty(document: &Document, label: &str) {
    let source = selected_source(document).unwrap_or_default();
    let incomes = parse_numbers(&source);
    let poverty_line =
        numeric_attr(selected_container(document).as_ref(), "data-poverty-line").unwrap_or(2.15);
    let Some((count, rate)) = local_headcount_poverty(&incomes, poverty_line) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Select a sheet or document with at least one non-negative income (optional data-poverty-line).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Econ.headcount_poverty",
        format!(
            "Headcount poverty sketch: {count}/{} below {poverty_line} → rate={rate:.4}",
            incomes.len()
        ),
        json!({
            "incomes": incomes,
            "poverty_line": poverty_line,
        }),
    );
}

/// `Econ.hyperbolic_discount` — β-δ present bias from data-* attrs.
pub(super) fn run_hyperbolic_discount(document: &Document, label: &str) {
    let container = selected_container(document);
    let t = numeric_attr(container.as_ref(), "data-t")
        .map(|v| v.round().max(0.0) as u64)
        .unwrap_or(1);
    let beta = numeric_attr(container.as_ref(), "data-beta").unwrap_or(0.7);
    let delta = numeric_attr(container.as_ref(), "data-delta").unwrap_or(0.99);
    let sketch = match local_hyperbolic_discount(t as u32, beta, delta) {
        Some(d) => format!("Hyperbolic discount sketch: t={t} β={beta} δ={delta} → {d:.6}"),
        None => format!(
            "Hyperbolic discount inputs t={t} β={beta} δ={delta} (need β,δ in [0,1])"
        ),
    };
    invoke_dual(
        document,
        label,
        "Econ.hyperbolic_discount",
        sketch,
        json!({
            "t": t,
            "beta": beta,
            "delta": delta,
        }),
    );
}

/// `Econ.fiscal_multiplier` — spending / MPC / leakage from data-* attrs.
pub(super) fn run_fiscal_multiplier(document: &Document, label: &str) {
    let container = selected_container(document);
    let initial_spending =
        numeric_attr(container.as_ref(), "data-initial-spending").unwrap_or(100.0);
    let mpc = numeric_attr(container.as_ref(), "data-mpc").unwrap_or(0.6);
    let leakage_rate =
        numeric_attr(container.as_ref(), "data-leakage-rate").unwrap_or(0.0);
    let sketch = match local_fiscal_multiplier(initial_spending, mpc, leakage_rate) {
        Some(m) => format!(
            "Fiscal multiplier sketch: G={initial_spending} mpc={mpc} leak={leakage_rate} → GDPΔ≈{m:.3}"
        ),
        None => format!(
            "Fiscal multiplier inputs G={initial_spending} mpc={mpc} leak={leakage_rate}"
        ),
    };
    invoke_dual(
        document,
        label,
        "Econ.fiscal_multiplier",
        sketch,
        json!({
            "initial_spending": initial_spending,
            "mpc": mpc,
            "leakage_rate": leakage_rate,
        }),
    );
}

/// `Econ.drawdown` — wealth path from selected surface numbers.
pub(super) fn run_drawdown(document: &Document, label: &str) {
    let source = selected_source(document).unwrap_or_default();
    let wealth = parse_numbers(&source);
    let Some(max_dd) = local_max_drawdown(&wealth) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Select a sheet or document with a wealth index series (at least one number).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Econ.drawdown",
        format!(
            "Drawdown sketch over {} wealth points: max drawdown ≈ {max_dd:.4} (live returns full series)",
            wealth.len()
        ),
        json!({ "wealth": wealth }),
    );
}

/// `Econ.covariance_matrix` — flat period×asset returns; `data-n-assets` / `data-n-periods`.
pub(super) fn run_covariance_matrix(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let returns = parse_numbers(&source);
    let n_assets = numeric_attr(container.as_ref(), "data-n-assets")
        .map(|n| n.round().max(1.0) as u64)
        .unwrap_or(2);
    let n_periods = numeric_attr(container.as_ref(), "data-n-periods")
        .map(|n| n.round().max(2.0) as u64)
        .unwrap_or_else(|| {
            if n_assets == 0 {
                0
            } else {
                (returns.len() as u64 / n_assets).max(2)
            }
        });
    let needed = (n_periods * n_assets) as usize;
    if returns.len() < needed {
        super::interactions::show_tool_status(
            document,
            label,
            &format!(
                "Need at least {needed} return numbers for {n_periods} periods × {n_assets} assets (optional data-n-assets / data-n-periods)."
            ),
            "error",
        );
        return;
    }
    let sketch = match local_covariance_matrix(&returns, n_periods as usize, n_assets as usize) {
        Some(cov) => format!(
            "Covariance sketch: {n_periods}×{n_assets} → diag≈[{:.4}, …] ({} entries)",
            cov.first().copied().unwrap_or(0.0),
            cov.len()
        ),
        None => format!(
            "Covariance sketch inputs: {} returns, n_periods={n_periods}, n_assets={n_assets}",
            returns.len()
        ),
    };
    invoke_dual(
        document,
        label,
        "Econ.covariance_matrix",
        sketch,
        json!({
            "returns": returns[..needed].to_vec(),
            "n_periods": n_periods,
            "n_assets": n_assets,
        }),
    );
}

/// `Econ.capm_beta` — paired asset/market returns (split surface or defaults).
pub(super) fn run_capm_beta(document: &Document, label: &str) {
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let (asset, market) = if nums.len() >= 4 && nums.len() % 2 == 0 {
        let half = nums.len() / 2;
        (nums[..half].to_vec(), nums[half..].to_vec())
    } else {
        (
            vec![0.01, 0.02, -0.01, 0.03],
            vec![0.01, 0.015, -0.005, 0.02],
        )
    };
    let sketch = match local_capm_beta(&asset, &market) {
        Some(beta) => format!(
            "CAPM β sketch over {} paired returns → β≈{beta:.4}",
            asset.len()
        ),
        None => format!(
            "CAPM β sketch inputs: {} asset / {} market returns (need ≥2 equal-length pairs)",
            asset.len(),
            market.len()
        ),
    };
    invoke_dual(
        document,
        label,
        "Econ.capm_beta",
        sketch,
        json!({
            "asset_returns": asset,
            "market_returns": market,
        }),
    );
}

/// `Econ.autocorrelation` — series from surface; `data-lag` (default 1).
pub(super) fn run_autocorrelation(document: &Document, label: &str) {
    let source = selected_source(document).unwrap_or_default();
    let values = parse_numbers(&source);
    let lag = numeric_attr(selected_container(document).as_ref(), "data-lag")
        .map(|l| l.round().max(1.0) as u64)
        .unwrap_or(1);
    let Some(ac) = local_autocorrelation(&values, lag as usize) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Select a sheet or document with at least three numbers (optional data-lag ≥ 1).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Econ.autocorrelation",
        format!(
            "Autocorrelation sketch over {} values at lag={lag}: {ac:.4}",
            values.len()
        ),
        json!({
            "values": values,
            "lag": lag,
        }),
    );
}

/// `Econ.cross_correlation` — two equal series (split surface); optional `data-lag`.
pub(super) fn run_cross_correlation(document: &Document, label: &str) {
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let lag = numeric_attr(selected_container(document).as_ref(), "data-lag")
        .map(|l| l.round() as i64)
        .unwrap_or(0);
    let (series_a, series_b) = if nums.len() >= 4 && nums.len() % 2 == 0 {
        let half = nums.len() / 2;
        (nums[..half].to_vec(), nums[half..].to_vec())
    } else {
        (vec![1.0, 2.0, 3.0, 4.0], vec![1.1, 2.1, 2.9, 4.2])
    };
    let sketch = match local_pearson(&series_a, &series_b) {
        Some(r) if lag == 0 => format!(
            "Cross-correlation sketch (lag=0 Pearson) over {} pairs → r≈{r:.4}",
            series_a.len()
        ),
        Some(_) => format!(
            "Cross-correlation sketch: {} pairs, lag={lag} (offline shows lag-0 Pearson only)",
            series_a.len()
        ),
        None => format!(
            "Cross-correlation sketch inputs: {} / {} (need equal length ≥2)",
            series_a.len(),
            series_b.len()
        ),
    };
    invoke_dual(
        document,
        label,
        "Econ.cross_correlation",
        sketch,
        json!({
            "series_a": series_a,
            "series_b": series_b,
            "lag": lag,
        }),
    );
}

/// `Econ.bertrand_with_demand` — demand + costs from data-* attrs.
pub(super) fn run_bertrand_with_demand(document: &Document, label: &str) {
    let container = selected_container(document);
    let demand_intercept =
        numeric_attr(container.as_ref(), "data-demand-intercept").unwrap_or(100.0);
    let demand_slope = numeric_attr(container.as_ref(), "data-demand-slope").unwrap_or(1.0);
    let cost_1 = numeric_attr(container.as_ref(), "data-cost-1").unwrap_or(10.0);
    let cost_2 = numeric_attr(container.as_ref(), "data-cost-2").unwrap_or(12.0);
    let sketch = match local_bertrand_with_demand(demand_intercept, demand_slope, cost_1, cost_2) {
        Some((price, quantity)) => format!(
            "Bertrand+demand sketch: P≈{price:.3} Q≈{quantity:.3} (a={demand_intercept} b={demand_slope})"
        ),
        None => format!(
            "Bertrand+demand inputs a={demand_intercept} b={demand_slope} c1={cost_1} c2={cost_2}"
        ),
    };
    invoke_dual(
        document,
        label,
        "Econ.bertrand_with_demand",
        sketch,
        json!({
            "demand_intercept": demand_intercept,
            "demand_slope": demand_slope,
            "cost_1": cost_1,
            "cost_2": cost_2,
        }),
    );
}

/// `Econ.check_budget_balance` — payment numbers from selected surface.
pub(super) fn run_check_budget_balance(document: &Document, label: &str) {
    let source = selected_source(document).unwrap_or_default();
    let payments = parse_numbers(&source);
    let Some((balanced, surplus)) = local_check_budget_balance(&payments) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Select a sheet or document with at least one payment number.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Econ.check_budget_balance",
        format!(
            "Budget-balance sketch over {} payments: balanced={balanced} surplus≈{surplus:.4}",
            payments.len()
        ),
        json!({ "payments": payments }),
    );
}

/// `Econ.ccapm_equity_premium` — gamma / σ_c / σ_m from data-* attrs.
pub(super) fn run_ccapm_equity_premium(document: &Document, label: &str) {
    let container = selected_container(document);
    let gamma = numeric_attr(container.as_ref(), "data-gamma").unwrap_or(2.0);
    let consumption_growth_std =
        numeric_attr(container.as_ref(), "data-consumption-growth-std").unwrap_or(0.02);
    let market_return_std =
        numeric_attr(container.as_ref(), "data-market-return-std").unwrap_or(0.15);
    let sketch = match local_ccapm_equity_premium(gamma, consumption_growth_std, market_return_std)
    {
        Some(premium) => format!(
            "CCAPM premium sketch: γ={gamma} σc={consumption_growth_std} σm={market_return_std} → {premium:.4}"
        ),
        None => format!(
            "CCAPM premium inputs γ={gamma} σc={consumption_growth_std} σm={market_return_std}"
        ),
    };
    invoke_dual(
        document,
        label,
        "Econ.ccapm_equity_premium",
        sketch,
        json!({
            "gamma": gamma,
            "consumption_growth_std": consumption_growth_std,
            "market_return_std": market_return_std,
        }),
    );
}

/// `Econ.mean_return` — arithmetic mean of return numbers on the surface.
pub(super) fn run_mean_return(document: &Document, label: &str) {
    let source = selected_source(document).unwrap_or_default();
    let returns = parse_numbers(&source);
    let Some(mean) = local_mean_return(&returns) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Select a sheet or document with at least one return number.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Econ.mean_return",
        format!("Mean-return sketch over {} returns: {mean:.6}", returns.len()),
        json!({ "returns": returns }),
    );
}

/// `Econ.poverty_gap` — incomes from surface; `data-poverty-line`.
pub(super) fn run_poverty_gap(document: &Document, label: &str) {
    let source = selected_source(document).unwrap_or_default();
    let incomes = parse_numbers(&source);
    let poverty_line =
        numeric_attr(selected_container(document).as_ref(), "data-poverty-line").unwrap_or(2.15);
    let Some(gap) = local_poverty_gap(&incomes, poverty_line) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Select a sheet or document with at least one non-negative income (optional data-poverty-line).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Econ.poverty_gap",
        format!(
            "Poverty-gap sketch over {} incomes @ line={poverty_line}: ratio≈{gap:.4}",
            incomes.len()
        ),
        json!({
            "incomes": incomes,
            "poverty_line": poverty_line,
        }),
    );
}


/// `Econ.sample_variance` — returns from selected surface numbers.
pub(super) fn run_sample_variance(document: &Document, label: &str) {
    let source = selected_source(document).unwrap_or_default();
    let returns = parse_numbers(&source);
    let Some(var) = local_sample_variance(&returns) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Select a sheet or document with at least two return numbers.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Econ.sample_variance",
        format!("Sample-variance sketch over {} returns: {var:.6}", returns.len()),
        json!({ "returns": returns }),
    );
}

/// `Econ.utilitarian_welfare` — utilities from selected surface numbers.
pub(super) fn run_utilitarian_welfare(document: &Document, label: &str) {
    let source = selected_source(document).unwrap_or_default();
    let utilities = parse_numbers(&source);
    let Some(w) = local_utilitarian_welfare(&utilities) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Select a sheet or document with at least one utility number.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Econ.utilitarian_welfare",
        format!("Utilitarian welfare sketch over {} utilities: {w:.4}", utilities.len()),
        json!({ "utilities": utilities }),
    );
}

/// `Econ.rawlsian_welfare` — utilities from selected surface numbers.
pub(super) fn run_rawlsian_welfare(document: &Document, label: &str) {
    let source = selected_source(document).unwrap_or_default();
    let utilities = parse_numbers(&source);
    let Some(w) = local_rawlsian_welfare(&utilities) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Select a sheet or document with at least one utility number.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Econ.rawlsian_welfare",
        format!("Rawlsian welfare sketch (min of {}): {w:.4}", utilities.len()),
        json!({ "utilities": utilities }),
    );
}

/// `Econ.nash_welfare` — strictly positive utilities from the surface.
pub(super) fn run_nash_welfare(document: &Document, label: &str) {
    let source = selected_source(document).unwrap_or_default();
    let utilities = parse_numbers(&source);
    let Some(w) = local_nash_welfare(&utilities) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Select a sheet or document with at least one strictly positive utility.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Econ.nash_welfare",
        format!("Nash welfare sketch (product of {}): {w:.4}", utilities.len()),
        json!({ "utilities": utilities }),
    );
}

/// `Econ.stackelberg_duopoly` — demand intercept/slope and leader/follower costs.
pub(super) fn run_stackelberg(document: &Document, label: &str) {
    let container = selected_container(document);
    let a = numeric_attr(container.as_ref(), "data-demand-intercept").unwrap_or(100.0);
    let b = numeric_attr(container.as_ref(), "data-demand-slope").unwrap_or(1.0);
    let c1 = numeric_attr(container.as_ref(), "data-cost-leader").unwrap_or(10.0);
    let c2 = numeric_attr(container.as_ref(), "data-cost-follower").unwrap_or(10.0);
    let sketch = match local_stackelberg(a, b, c1, c2) {
        Some((q1, q2, price)) => format!(
            "Stackelberg sketch: a={a} b={b} cL={c1} cF={c2} -> qL={q1:.4} qF={q2:.4} P={price:.4}"
        ),
        None => format!("Stackelberg inputs a={a} b={b} cL={c1} cF={c2}"),
    };
    invoke_dual(
        document,
        label,
        "Econ.stackelberg_duopoly",
        sketch,
        json!({
            "demand_intercept": a,
            "demand_slope": b,
            "cost_leader": c1,
            "cost_follower": c2,
        }),
    );
}

/// `Econ.put_call_parity` — option parity residual from data-* attrs.
pub(super) fn run_put_call_parity(document: &Document, label: &str) {
    let container = selected_container(document);
    let call_price = numeric_attr(container.as_ref(), "data-call-price").unwrap_or(10.0);
    let put_price = numeric_attr(container.as_ref(), "data-put-price").unwrap_or(5.0);
    let spot = numeric_attr(container.as_ref(), "data-spot").unwrap_or(100.0);
    let strike = numeric_attr(container.as_ref(), "data-strike").unwrap_or(100.0);
    let time_to_expiry = numeric_attr(container.as_ref(), "data-time").unwrap_or(1.0);
    let risk_free_rate = numeric_attr(container.as_ref(), "data-rate").unwrap_or(0.03);
    let dividend_yield = numeric_attr(container.as_ref(), "data-dividend").unwrap_or(0.0);
    let sketch = match local_put_call_parity(
        call_price,
        put_price,
        spot,
        strike,
        risk_free_rate,
        dividend_yield,
        time_to_expiry,
    ) {
        Some(err) => format!("Put-call parity residual sketch: {err:.6}"),
        None => format!(
            "Put-call parity inputs C={call_price} P={put_price} S={spot} K={strike} T={time_to_expiry}"
        ),
    };
    invoke_dual(
        document,
        label,
        "Econ.put_call_parity",
        sketch,
        json!({
            "call_price": call_price,
            "put_price": put_price,
            "spot": spot,
            "strike": strike,
            "time_to_expiry": time_to_expiry,
            "risk_free_rate": risk_free_rate,
            "dividend_yield": dividend_yield,
        }),
    );
}

/// `Econ.parametric_var` — Gaussian VaR from mean / std / confidence attrs.
pub(super) fn run_parametric_var(document: &Document, label: &str) {
    let container = selected_container(document);
    let mean = numeric_attr(container.as_ref(), "data-mean").unwrap_or(0.0);
    let std_dev = numeric_attr(container.as_ref(), "data-std-dev").unwrap_or(0.02);
    let confidence = numeric_attr(container.as_ref(), "data-confidence").unwrap_or(0.95);
    let sketch = match local_parametric_var(mean, std_dev, confidence) {
        Some(v) => format!(
            "Parametric VaR sketch: mean={mean} std={std_dev} conf={confidence} -> {v:.6}"
        ),
        None => format!("Parametric VaR inputs mean={mean} std={std_dev} conf={confidence}"),
    };
    invoke_dual(
        document,
        label,
        "Econ.parametric_var",
        sketch,
        json!({
            "mean": mean,
            "std_dev": std_dev,
            "confidence": confidence,
        }),
    );
}

/// `Econ.laffer_curve` — tax revenue from rate / base / elasticity attrs.
pub(super) fn run_laffer_curve(document: &Document, label: &str) {
    let container = selected_container(document);
    let tax_rate = numeric_attr(container.as_ref(), "data-tax-rate").unwrap_or(0.3);
    let tax_base = numeric_attr(container.as_ref(), "data-tax-base").unwrap_or(100.0);
    let elasticity = numeric_attr(container.as_ref(), "data-elasticity").unwrap_or(1.0);
    let sketch = match local_laffer_curve(tax_rate, tax_base, elasticity) {
        Some(rev) => format!(
            "Laffer revenue sketch: t={tax_rate} B={tax_base} e={elasticity} -> {rev:.4}"
        ),
        None => format!("Laffer inputs t={tax_rate} B={tax_base} e={elasticity}"),
    };
    invoke_dual(
        document,
        label,
        "Econ.laffer_curve",
        sketch,
        json!({
            "tax_rate": tax_rate,
            "tax_base": tax_base,
            "elasticity": elasticity,
        }),
    );
}

/// `Econ.historical_cvar` — expected shortfall from surface returns.
pub(super) fn run_historical_cvar(document: &Document, label: &str) {
    let source = selected_source(document).unwrap_or_default();
    let returns = parse_numbers(&source);
    let confidence =
        numeric_attr(selected_container(document).as_ref(), "data-confidence").unwrap_or(0.95);
    let Some(cvar) = local_historical_cvar(&returns, confidence) else {
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
        "Econ.historical_cvar",
        format!(
            "Historical CVaR sketch over {} returns at {confidence}: {cvar:.6}",
            returns.len()
        ),
        json!({
            "returns": returns,
            "confidence": confidence,
        }),
    );
}

/// `Econ.endowment_effect` — WTA from WTP and loss-aversion lambda attrs.
pub(super) fn run_endowment_effect(document: &Document, label: &str) {
    let container = selected_container(document);
    let wtp = numeric_attr(container.as_ref(), "data-wtp").unwrap_or(10.0);
    let lambda = numeric_attr(container.as_ref(), "data-lambda").unwrap_or(2.25);
    let sketch = match local_endowment_effect(wtp, lambda) {
        Some(wta) => format!("Endowment WTA sketch: WTP={wtp} lambda={lambda} -> WTA={wta:.4}"),
        None => format!("Endowment inputs WTP={wtp} lambda={lambda}"),
    };
    invoke_dual(
        document,
        label,
        "Econ.endowment_effect",
        sketch,
        json!({
            "wtp": wtp,
            "lambda": lambda,
        }),
    );
}

/// `Econ.prospect_value` — Kahneman–Tversky value from data-* attrs.
pub(super) fn run_prospect_value(document: &Document, label: &str) {
    let container = selected_container(document);
    let x = numeric_attr(container.as_ref(), "data-x").unwrap_or(100.0);
    let alpha = numeric_attr(container.as_ref(), "data-alpha").unwrap_or(0.88);
    let beta = numeric_attr(container.as_ref(), "data-beta").unwrap_or(0.88);
    let lambda = numeric_attr(container.as_ref(), "data-lambda").unwrap_or(2.25);
    let sketch = match local_prospect_value(x, alpha, beta, lambda) {
        Some(v) => format!("Prospect value sketch: x={x} -> {v:.6}"),
        None => format!("Prospect inputs x={x} alpha={alpha} beta={beta} lambda={lambda}"),
    };
    invoke_dual(
        document,
        label,
        "Econ.prospect_value",
        sketch,
        json!({
            "x": x,
            "alpha": alpha,
            "beta": beta,
            "lambda": lambda,
        }),
    );
}

/// `Econ.probability_weight` — Prelec weight from p / gamma attrs.
pub(super) fn run_probability_weight(document: &Document, label: &str) {
    let container = selected_container(document);
    let p = numeric_attr(container.as_ref(), "data-p").unwrap_or(0.1);
    let gamma = numeric_attr(container.as_ref(), "data-gamma").unwrap_or(0.65);
    let sketch = match local_probability_weight(p, gamma) {
        Some(w) => format!("Probability weight sketch: p={p} gamma={gamma} -> {w:.6}"),
        None => format!("Probability weight inputs p={p} gamma={gamma}"),
    };
    invoke_dual(
        document,
        label,
        "Econ.probability_weight",
        sketch,
        json!({
            "p": p,
            "gamma": gamma,
        }),
    );
}

/// `Econ.ccapm_sdf` — stochastic discount factor from growth / gamma / beta.
pub(super) fn run_ccapm_sdf(document: &Document, label: &str) {
    let container = selected_container(document);
    let consumption_growth =
        numeric_attr(container.as_ref(), "data-consumption-growth").unwrap_or(1.02);
    let gamma = numeric_attr(container.as_ref(), "data-gamma").unwrap_or(2.0);
    let beta = numeric_attr(container.as_ref(), "data-beta").unwrap_or(0.99);
    let sketch = match local_ccapm_sdf(consumption_growth, gamma, beta) {
        Some(m) => format!(
            "CCAPM SDF sketch: g={consumption_growth} gamma={gamma} beta={beta} -> {m:.6}"
        ),
        None => format!("CCAPM SDF inputs g={consumption_growth} gamma={gamma} beta={beta}"),
    };
    invoke_dual(
        document,
        label,
        "Econ.ccapm_sdf",
        sketch,
        json!({
            "consumption_growth": consumption_growth,
            "gamma": gamma,
            "beta": beta,
        }),
    );
}

/// `Econ.gravity_flow` — spatial gravity flow from masses / distance attrs.
pub(super) fn run_gravity_flow(document: &Document, label: &str) {
    let container = selected_container(document);
    let mass_1 = numeric_attr(container.as_ref(), "data-mass-1").unwrap_or(100.0);
    let mass_2 = numeric_attr(container.as_ref(), "data-mass-2").unwrap_or(80.0);
    let distance = numeric_attr(container.as_ref(), "data-distance").unwrap_or(10.0);
    let alpha = numeric_attr(container.as_ref(), "data-alpha").unwrap_or(1.0);
    let beta = numeric_attr(container.as_ref(), "data-beta").unwrap_or(1.0);
    let gamma = numeric_attr(container.as_ref(), "data-gamma").unwrap_or(2.0);
    let sketch = match local_gravity_flow(mass_1, mass_2, distance, alpha, beta, gamma) {
        Some(flow) => format!(
            "Gravity flow sketch: m1={mass_1} m2={mass_2} d={distance} -> {flow:.4}"
        ),
        None => format!("Gravity inputs m1={mass_1} m2={mass_2} d={distance}"),
    };
    invoke_dual(
        document,
        label,
        "Econ.gravity_flow",
        sketch,
        json!({
            "mass_1": mass_1,
            "mass_2": mass_2,
            "distance": distance,
            "alpha": alpha,
            "beta": beta,
            "gamma": gamma,
        }),
    );
}

/// `Econ.transfer_payment` — means-tested transfer from data-* attrs.
pub(super) fn run_transfer_payment(document: &Document, label: &str) {
    let container = selected_container(document);
    let base = numeric_attr(container.as_ref(), "data-base").unwrap_or(500.0);
    let income = numeric_attr(container.as_ref(), "data-income").unwrap_or(600.0);
    let threshold = numeric_attr(container.as_ref(), "data-threshold").unwrap_or(400.0);
    let phaseout_rate = numeric_attr(container.as_ref(), "data-phaseout-rate").unwrap_or(0.5);
    let sketch = match local_transfer_payment(base, income, threshold, phaseout_rate) {
        Some(t) => format!(
            "Transfer payment sketch: base={base} income={income} thr={threshold} -> {t:.2}"
        ),
        None => format!("Transfer inputs base={base} income={income} thr={threshold}"),
    };
    invoke_dual(
        document,
        label,
        "Econ.transfer_payment",
        sketch,
        json!({
            "base": base,
            "income": income,
            "threshold": threshold,
            "phaseout_rate": phaseout_rate,
        }),
    );
}

/// `Econ.efficiency_units` — effective labor = raw × human capital.
pub(super) fn run_efficiency_units(document: &Document, label: &str) {
    let container = selected_container(document);
    let raw_labor = numeric_attr(container.as_ref(), "data-raw-labor").unwrap_or(40.0);
    let human_capital = numeric_attr(container.as_ref(), "data-human-capital").unwrap_or(1.5);
    let sketch = match local_efficiency_units(raw_labor, human_capital) {
        Some(e) => format!(
            "Efficiency units sketch: L={raw_labor} h={human_capital} -> {e:.4}"
        ),
        None => format!("Efficiency inputs L={raw_labor} h={human_capital}"),
    };
    invoke_dual(
        document,
        label,
        "Econ.efficiency_units",
        sketch,
        json!({
            "raw_labor": raw_labor,
            "human_capital": human_capital,
        }),
    );
}

/// `Econ.social_cost_of_carbon` — emissions × damage per ton.
pub(super) fn run_social_cost_of_carbon(document: &Document, label: &str) {
    let container = selected_container(document);
    let emissions = numeric_attr(container.as_ref(), "data-emissions").unwrap_or(100.0);
    let damage_per_ton = numeric_attr(container.as_ref(), "data-damage-per-ton").unwrap_or(50.0);
    let sketch = match local_social_cost_of_carbon(emissions, damage_per_ton) {
        Some(scc) => format!(
            "Social cost of carbon sketch: E={emissions} d={damage_per_ton} -> {scc:.2}"
        ),
        None => format!("SCC inputs E={emissions} d={damage_per_ton}"),
    };
    invoke_dual(
        document,
        label,
        "Econ.social_cost_of_carbon",
        sketch,
        json!({
            "emissions": emissions,
            "damage_per_ton": damage_per_ton,
        }),
    );
}

/// `Econ.pollution_damage` — quadratic damage from emissions / coeff.
pub(super) fn run_pollution_damage(document: &Document, label: &str) {
    let container = selected_container(document);
    let emissions = numeric_attr(container.as_ref(), "data-emissions").unwrap_or(10.0);
    let damage_coeff = numeric_attr(container.as_ref(), "data-damage-coeff").unwrap_or(0.5);
    let sketch = match local_pollution_damage(emissions, damage_coeff) {
        Some(d) => format!(
            "Pollution damage sketch: E={emissions} c={damage_coeff} -> {d:.4}"
        ),
        None => format!("Pollution damage inputs E={emissions} c={damage_coeff}"),
    };
    invoke_dual(
        document,
        label,
        "Econ.pollution_damage",
        sketch,
        json!({
            "emissions": emissions,
            "damage_coeff": damage_coeff,
        }),
    );
}

/// `Econ.marginal_damage` — linear MD from emissions / coeff.
pub(super) fn run_marginal_damage(document: &Document, label: &str) {
    let container = selected_container(document);
    let emissions = numeric_attr(container.as_ref(), "data-emissions").unwrap_or(10.0);
    let damage_coeff = numeric_attr(container.as_ref(), "data-damage-coeff").unwrap_or(0.5);
    let sketch = match local_marginal_damage(emissions, damage_coeff) {
        Some(md) => format!(
            "Marginal damage sketch: E={emissions} c={damage_coeff} -> {md:.4}"
        ),
        None => format!("Marginal damage inputs E={emissions} c={damage_coeff}"),
    };
    invoke_dual(
        document,
        label,
        "Econ.marginal_damage",
        sketch,
        json!({
            "emissions": emissions,
            "damage_coeff": damage_coeff,
        }),
    );
}

/// `Econ.ramsey_steady_state` — steady-state capital from alpha / beta / delta.
pub(super) fn run_ramsey_steady_state(document: &Document, label: &str) {
    let container = selected_container(document);
    let alpha = numeric_attr(container.as_ref(), "data-alpha").unwrap_or(0.33);
    let beta = numeric_attr(container.as_ref(), "data-beta").unwrap_or(0.96);
    let depreciation = numeric_attr(container.as_ref(), "data-depreciation").unwrap_or(0.08);
    let sketch = match local_ramsey_steady_state(alpha, beta, depreciation) {
        Some(k) => format!(
            "Ramsey steady-state sketch: α={alpha} β={beta} δ={depreciation} -> k*={k:.4}"
        ),
        None => format!("Ramsey inputs α={alpha} β={beta} δ={depreciation}"),
    };
    invoke_dual(
        document,
        label,
        "Econ.ramsey_steady_state",
        sketch,
        json!({
            "alpha": alpha,
            "beta": beta,
            "depreciation": depreciation,
        }),
    );
}

/// `Econ.simple_returns` — prices from selected surface numbers.
pub(super) fn run_simple_returns(document: &Document, label: &str) {
    let source = selected_source(document).unwrap_or_default();
    let prices = parse_numbers(&source);
    let Some((count, last)) = local_simple_returns_summary(&prices) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Select a sheet or document with at least two strictly positive prices.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Econ.simple_returns",
        format!("Simple-returns sketch: {count} periods, last≈{last:.6} (live returns full series)"),
        json!({ "prices": prices }),
    );
}

/// `Econ.log_returns` — prices from selected surface numbers.
pub(super) fn run_log_returns(document: &Document, label: &str) {
    let source = selected_source(document).unwrap_or_default();
    let prices = parse_numbers(&source);
    let Some((count, last)) = local_log_returns_summary(&prices) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Select a sheet or document with at least two strictly positive prices.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Econ.log_returns",
        format!("Log-returns sketch: {count} periods, last≈{last:.6} (live returns full series)"),
        json!({ "prices": prices }),
    );
}

/// `Econ.rolling_mean` — series from surface; `data-window` (default 3).
pub(super) fn run_rolling_mean(document: &Document, label: &str) {
    let source = selected_source(document).unwrap_or_default();
    let values = parse_numbers(&source);
    let window = numeric_attr(selected_container(document).as_ref(), "data-window")
        .map(|w| w.round().max(1.0) as u64)
        .unwrap_or(3);
    let Some((count, last)) = local_rolling_mean_last(&values, window as usize) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Select a sheet or document with at least `data-window` numbers (default window=3).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Econ.rolling_mean",
        format!("Rolling-mean sketch: window={window}, {count} means, last≈{last:.6}"),
        json!({
            "values": values,
            "window": window,
        }),
    );
}

/// `Econ.rolling_variance` — series from surface; `data-window` (default 3).
pub(super) fn run_rolling_variance(document: &Document, label: &str) {
    let source = selected_source(document).unwrap_or_default();
    let values = parse_numbers(&source);
    let window = numeric_attr(selected_container(document).as_ref(), "data-window")
        .map(|w| w.round().max(1.0) as u64)
        .unwrap_or(3);
    let Some((count, last)) = local_rolling_variance_last(&values, window as usize) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Select a sheet or document with at least `data-window` numbers (default window=3).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Econ.rolling_variance",
        format!("Rolling-variance sketch: window={window}, {count} vars, last≈{last:.6}"),
        json!({
            "values": values,
            "window": window,
        }),
    );
}

/// `Econ.labor_supply` — wage / endowment / income / alpha from data-* attrs.
pub(super) fn run_labor_supply(document: &Document, label: &str) {
    let container = selected_container(document);
    let wage = numeric_attr(container.as_ref(), "data-wage").unwrap_or(20.0);
    let time_endowment = numeric_attr(container.as_ref(), "data-time-endowment").unwrap_or(24.0);
    let non_labor_income =
        numeric_attr(container.as_ref(), "data-non-labor-income").unwrap_or(0.0);
    let alpha = numeric_attr(container.as_ref(), "data-alpha").unwrap_or(0.5);
    let sketch = match local_labor_supply(wage, time_endowment, non_labor_income, alpha) {
        Some((h, c)) => format!(
            "Labor-supply sketch: w={wage} T={time_endowment} α={alpha} -> h={h:.4} c={c:.4}"
        ),
        None => format!("Labor-supply inputs w={wage} T={time_endowment} y={non_labor_income}"),
    };
    invoke_dual(
        document,
        label,
        "Econ.labor_supply",
        sketch,
        json!({
            "wage": wage,
            "time_endowment": time_endowment,
            "non_labor_income": non_labor_income,
            "alpha": alpha,
        }),
    );
}

/// `Econ.optimal_abatement` — baseline / coeffs from data-* attrs.
pub(super) fn run_optimal_abatement(document: &Document, label: &str) {
    let container = selected_container(document);
    let baseline_emissions =
        numeric_attr(container.as_ref(), "data-baseline-emissions").unwrap_or(100.0);
    let abatement_coeff = numeric_attr(container.as_ref(), "data-abatement-coeff").unwrap_or(1.0);
    let damage_coeff = numeric_attr(container.as_ref(), "data-damage-coeff").unwrap_or(1.0);
    let sketch = match local_optimal_abatement(baseline_emissions, abatement_coeff, damage_coeff) {
        Some(a) => format!(
            "Optimal-abatement sketch: E0={baseline_emissions} -> A*={a:.4}"
        ),
        None => format!(
            "Optimal-abatement inputs E0={baseline_emissions} a={abatement_coeff} d={damage_coeff}"
        ),
    };
    invoke_dual(
        document,
        label,
        "Econ.optimal_abatement",
        sketch,
        json!({
            "baseline_emissions": baseline_emissions,
            "abatement_coeff": abatement_coeff,
            "damage_coeff": damage_coeff,
        }),
    );
}

/// `Econ.optimal_pollution` — baseline / coeffs from data-* attrs.
pub(super) fn run_optimal_pollution(document: &Document, label: &str) {
    let container = selected_container(document);
    let baseline_emissions =
        numeric_attr(container.as_ref(), "data-baseline-emissions").unwrap_or(100.0);
    let abatement_coeff = numeric_attr(container.as_ref(), "data-abatement-coeff").unwrap_or(1.0);
    let damage_coeff = numeric_attr(container.as_ref(), "data-damage-coeff").unwrap_or(1.0);
    let sketch = match local_optimal_pollution(baseline_emissions, abatement_coeff, damage_coeff) {
        Some(e) => format!(
            "Optimal-pollution sketch: E0={baseline_emissions} -> E*={e:.4}"
        ),
        None => format!(
            "Optimal-pollution inputs E0={baseline_emissions} a={abatement_coeff} d={damage_coeff}"
        ),
    };
    invoke_dual(
        document,
        label,
        "Econ.optimal_pollution",
        sketch,
        json!({
            "baseline_emissions": baseline_emissions,
            "abatement_coeff": abatement_coeff,
            "damage_coeff": damage_coeff,
        }),
    );
}

/// `Econ.olg_steady_state` — alpha / beta / population growth from data-* attrs.
pub(super) fn run_olg_steady_state(document: &Document, label: &str) {
    let container = selected_container(document);
    let alpha = numeric_attr(container.as_ref(), "data-alpha").unwrap_or(0.33);
    let beta = numeric_attr(container.as_ref(), "data-beta").unwrap_or(0.96);
    let population_growth =
        numeric_attr(container.as_ref(), "data-population-growth").unwrap_or(0.0);
    let sketch = match local_olg_steady_state(alpha, beta, population_growth) {
        Some((k, y)) => format!(
            "OLG steady-state sketch: α={alpha} β={beta} n={population_growth} -> k*={k:.4} y*={y:.4}"
        ),
        None => format!("OLG inputs α={alpha} β={beta} n={population_growth}"),
    };
    invoke_dual(
        document,
        label,
        "Econ.olg_steady_state",
        sketch,
        json!({
            "alpha": alpha,
            "beta": beta,
            "population_growth": population_growth,
        }),
    );
}

/// `Econ.ramsey_euler_residual` — capital path params from data-* attrs.
pub(super) fn run_ramsey_euler_residual(document: &Document, label: &str) {
    let container = selected_container(document);
    let capital = numeric_attr(container.as_ref(), "data-capital").unwrap_or(1.0);
    let capital_next = numeric_attr(container.as_ref(), "data-capital-next").unwrap_or(1.0);
    let beta = numeric_attr(container.as_ref(), "data-beta").unwrap_or(0.96);
    let alpha = numeric_attr(container.as_ref(), "data-alpha").unwrap_or(0.33);
    let delta = numeric_attr(container.as_ref(), "data-delta").unwrap_or(0.08);
    let sigma = numeric_attr(container.as_ref(), "data-sigma").unwrap_or(1.0);
    let sketch = match local_ramsey_euler_residual(capital, capital_next, beta, alpha, delta, sigma)
    {
        Some(r) => format!("Ramsey Euler residual sketch: {r:.6}"),
        None => format!(
            "Ramsey Euler inputs k={capital} k'={capital_next} β={beta} α={alpha} δ={delta}"
        ),
    };
    invoke_dual(
        document,
        label,
        "Econ.ramsey_euler_residual",
        sketch,
        json!({
            "capital": capital,
            "capital_next": capital_next,
            "beta": beta,
            "alpha": alpha,
            "delta": delta,
            "sigma": sigma,
        }),
    );
}

/// `Econ.present_biased_utility` — utilities from surface; β/δ from attrs.
pub(super) fn run_present_biased_utility(document: &Document, label: &str) {
    let source = selected_source(document).unwrap_or_default();
    let utilities = parse_numbers(&source);
    let container = selected_container(document);
    let beta = numeric_attr(container.as_ref(), "data-beta").unwrap_or(0.7);
    let delta = numeric_attr(container.as_ref(), "data-delta").unwrap_or(0.99);
    let Some(u) = local_present_biased_utility(&utilities, beta, delta) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Select a sheet or document with at least one utility (optional data-beta / data-delta).",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Econ.present_biased_utility",
        format!(
            "Present-biased utility sketch over {} periods (β={beta} δ={delta}): {u:.6}",
            utilities.len()
        ),
        json!({
            "utilities": utilities,
            "beta": beta,
            "delta": delta,
        }),
    );
}

/// `Econ.reference_dependent_utility` — outcome vs reference from data-* attrs.
pub(super) fn run_reference_dependent_utility(document: &Document, label: &str) {
    let container = selected_container(document);
    let x = numeric_attr(container.as_ref(), "data-x").unwrap_or(100.0);
    let reference = numeric_attr(container.as_ref(), "data-reference").unwrap_or(0.0);
    let alpha = numeric_attr(container.as_ref(), "data-alpha").unwrap_or(0.88);
    let beta = numeric_attr(container.as_ref(), "data-beta").unwrap_or(0.88);
    let lambda = numeric_attr(container.as_ref(), "data-lambda").unwrap_or(2.25);
    let sketch = match local_reference_dependent_utility(x, reference, alpha, beta, lambda) {
        Some(u) => format!("Reference-dependent utility sketch: x={x} ref={reference} -> {u:.6}"),
        None => format!("Reference-dependent inputs x={x} ref={reference}"),
    };
    invoke_dual(
        document,
        label,
        "Econ.reference_dependent_utility",
        sketch,
        json!({
            "x": x,
            "reference": reference,
            "alpha": alpha,
            "beta": beta,
            "lambda": lambda,
        }),
    );
}

fn default_zero_curve() -> (Vec<f64>, Vec<f64>) {
    (
        vec![0.5, 1.0, 2.0, 5.0],
        vec![0.03, 0.035, 0.04, 0.045],
    )
}

fn curve_from_surface(nums: &[f64]) -> (Vec<f64>, Vec<f64>) {
    if nums.len() >= 4 && nums.len() % 2 == 0 {
        let half = nums.len() / 2;
        (nums[..half].to_vec(), nums[half..].to_vec())
    } else {
        default_zero_curve()
    }
}

/// `Econ.npv` — benefits/costs from surface (split halves) or benefits-only with zero costs.
pub(super) fn run_npv(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let discount_rate =
        numeric_attr(container.as_ref(), "data-discount-rate").unwrap_or(0.05);
    let (benefits, costs) = if nums.len() >= 2 && nums.len() % 2 == 0 {
        let half = nums.len() / 2;
        (nums[..half].to_vec(), nums[half..].to_vec())
    } else if !nums.is_empty() {
        (nums.clone(), vec![0.0; nums.len()])
    } else {
        (vec![100.0, 100.0, 100.0], vec![80.0, 20.0, 10.0])
    };
    let n_periods = numeric_attr(container.as_ref(), "data-n-periods")
        .map(|n| n.round().max(1.0) as u64)
        .unwrap_or(benefits.len().min(costs.len()) as u64);
    let sketch = match local_npv(&benefits, &costs, discount_rate, n_periods as usize) {
        Some(npv) => format!(
            "NPV sketch over {n_periods} periods at r={discount_rate}: {npv:.4}"
        ),
        None => format!("NPV inputs benefits={benefits:?} costs={costs:?} r={discount_rate}"),
    };
    invoke_dual(
        document,
        label,
        "Econ.npv",
        sketch,
        json!({
            "benefits": benefits,
            "costs": costs,
            "discount_rate": discount_rate,
            "n_periods": n_periods,
        }),
    );
}

/// `Econ.multi_period_ddm` — dividends from surface; r/g from attrs.
pub(super) fn run_multi_period_ddm(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let mut dividends = parse_numbers(&source);
    if dividends.is_empty() {
        dividends = vec![2.0, 2.2, 2.4];
    }
    let discount_rate =
        numeric_attr(container.as_ref(), "data-discount-rate").unwrap_or(0.08);
    let terminal_growth =
        numeric_attr(container.as_ref(), "data-terminal-growth").unwrap_or(0.03);
    let sketch = match local_multi_period_ddm(&dividends, discount_rate, terminal_growth) {
        Some(p) => format!(
            "Multi-period DDM sketch over {} dividends (r={discount_rate} g={terminal_growth}): {p:.4}",
            dividends.len()
        ),
        None => format!(
            "Multi-period DDM inputs dividends={dividends:?} r={discount_rate} g={terminal_growth}"
        ),
    };
    invoke_dual(
        document,
        label,
        "Econ.multi_period_ddm",
        sketch,
        json!({
            "dividends": dividends,
            "discount_rate": discount_rate,
            "terminal_growth": terminal_growth,
        }),
    );
}

/// `Econ.portfolio_max_drawdown` — returns from selected surface numbers.
pub(super) fn run_portfolio_max_drawdown(document: &Document, label: &str) {
    let source = selected_source(document).unwrap_or_default();
    let returns = parse_numbers(&source);
    let Some(md) = local_portfolio_max_drawdown(&returns) else {
        super::interactions::show_tool_status(
            document,
            label,
            "Select a sheet or document with at least one return greater than −1.",
            "error",
        );
        return;
    };
    invoke_dual(
        document,
        label,
        "Econ.portfolio_max_drawdown",
        format!(
            "Portfolio max-drawdown sketch over {} returns: {md:.4}",
            returns.len()
        ),
        json!({ "returns": returns }),
    );
}

/// `Econ.interpolate_zero_rate` — curve from surface pairs; target from attr.
pub(super) fn run_interpolate_zero_rate(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let (maturities, rates) = curve_from_surface(&nums);
    let target =
        numeric_attr(container.as_ref(), "data-target-maturity").unwrap_or(1.5);
    let sketch = match local_interpolate_zero_rate(&maturities, &rates, target) {
        Some(r) => format!("Zero-rate interpolate sketch at T={target}: {r:.6}"),
        None => format!("Zero-rate interpolate inputs T={target}"),
    };
    invoke_dual(
        document,
        label,
        "Econ.interpolate_zero_rate",
        sketch,
        json!({
            "maturities": maturities,
            "rates": rates,
            "target_maturity": target,
        }),
    );
}

/// `Econ.discount_factor` — DF from zero curve at target maturity.
pub(super) fn run_discount_factor(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let (maturities, rates) = curve_from_surface(&nums);
    let target =
        numeric_attr(container.as_ref(), "data-target-maturity").unwrap_or(1.0);
    let compounding = numeric_attr(container.as_ref(), "data-compounding-per-year")
        .map(|c| c.round().max(1.0) as u64)
        .unwrap_or(1);
    let sketch = match local_discount_factor(&maturities, &rates, target, compounding as u32) {
        Some(df) => format!("Discount-factor sketch at T={target} (m={compounding}): {df:.6}"),
        None => format!("Discount-factor inputs T={target} m={compounding}"),
    };
    invoke_dual(
        document,
        label,
        "Econ.discount_factor",
        sketch,
        json!({
            "maturities": maturities,
            "rates": rates,
            "target_maturity": target,
            "compounding_per_year": compounding,
        }),
    );
}

/// `Econ.par_yield` — par coupon from zero curve at target maturity.
pub(super) fn run_par_yield(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let (maturities, rates) = curve_from_surface(&nums);
    let target =
        numeric_attr(container.as_ref(), "data-target-maturity").unwrap_or(2.0);
    let compounding = numeric_attr(container.as_ref(), "data-compounding-per-year")
        .map(|c| c.round().max(1.0) as u64)
        .unwrap_or(1);
    let sketch = match local_par_yield(&maturities, &rates, target, compounding as u32) {
        Some(y) => format!("Par-yield sketch at T={target} (m={compounding}): {y:.6}"),
        None => format!("Par-yield inputs T={target} m={compounding}"),
    };
    invoke_dual(
        document,
        label,
        "Econ.par_yield",
        sketch,
        json!({
            "maturities": maturities,
            "rates": rates,
            "target_maturity": target,
            "compounding_per_year": compounding,
        }),
    );
}

/// `Econ.progressive_tax` — brackets from surface pairs; income from attr.
pub(super) fn run_progressive_tax(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let (thresholds, rates) = if nums.len() >= 4 && nums.len() % 2 == 0 {
        let half = nums.len() / 2;
        (nums[..half].to_vec(), nums[half..].to_vec())
    } else {
        (vec![0.0, 50_000.0, 100_000.0], vec![0.1, 0.2, 0.3])
    };
    let income = numeric_attr(container.as_ref(), "data-income").unwrap_or(120_000.0);
    let sketch = match local_progressive_tax(income, &thresholds, &rates) {
        Some((total, effective)) => format!(
            "Progressive-tax sketch on income={income}: total={total:.2} effective={effective:.4}"
        ),
        None => format!("Progressive-tax inputs income={income}"),
    };
    invoke_dual(
        document,
        label,
        "Econ.progressive_tax",
        sketch,
        json!({
            "thresholds": thresholds,
            "rates": rates,
            "income": income,
        }),
    );
}

/// `Econ.abatement_net_benefit` — baseline/actual emissions and MAC/MD coeffs.
pub(super) fn run_abatement_net_benefit(document: &Document, label: &str) {
    let container = selected_container(document);
    let baseline =
        numeric_attr(container.as_ref(), "data-baseline-emissions").unwrap_or(100.0);
    let actual =
        numeric_attr(container.as_ref(), "data-actual-emissions").unwrap_or(60.0);
    let abatement_coeff =
        numeric_attr(container.as_ref(), "data-abatement-coeff").unwrap_or(1.0);
    let damage_coeff =
        numeric_attr(container.as_ref(), "data-damage-coeff").unwrap_or(1.0);
    let sketch = match local_abatement_net_benefit(
        baseline,
        actual,
        abatement_coeff,
        damage_coeff,
    ) {
        Some(nb) => format!(
            "Abatement net-benefit sketch E0={baseline} E={actual}: {nb:.4}"
        ),
        None => format!("Abatement net-benefit inputs E0={baseline} E={actual}"),
    };
    invoke_dual(
        document,
        label,
        "Econ.abatement_net_benefit",
        sketch,
        json!({
            "baseline_emissions": baseline,
            "actual_emissions": actual,
            "abatement_coeff": abatement_coeff,
            "damage_coeff": damage_coeff,
        }),
    );
}

/// `Econ.household_production_ces` — time, goods, α, ρ from data-* attrs.
pub(super) fn run_household_production_ces(document: &Document, label: &str) {
    let container = selected_container(document);
    let time = numeric_attr(container.as_ref(), "data-time").unwrap_or(8.0);
    let goods = numeric_attr(container.as_ref(), "data-goods").unwrap_or(10.0);
    let alpha = numeric_attr(container.as_ref(), "data-alpha").unwrap_or(0.5);
    let rho = numeric_attr(container.as_ref(), "data-rho").unwrap_or(0.5);
    let sketch = match local_household_production_ces(time, goods, alpha, rho) {
        Some(out) => format!(
            "Household CES production sketch time={time} goods={goods}: {out:.4}"
        ),
        None => format!("Household CES inputs time={time} goods={goods} α={alpha} ρ={rho}"),
    };
    invoke_dual(
        document,
        label,
        "Econ.household_production_ces",
        sketch,
        json!({
            "time": time,
            "goods": goods,
            "alpha": alpha,
            "rho": rho,
        }),
    );
}

/// `Econ.malfeasance_delta` — capital vs delivered from data-* attrs.
pub(super) fn run_malfeasance_delta(document: &Document, label: &str) {
    let container = selected_container(document);
    let capital_allocated =
        numeric_attr(container.as_ref(), "data-capital-allocated").unwrap_or(100.0);
    let delivered = numeric_attr(container.as_ref(), "data-delivered").unwrap_or(80.0);
    let inverted = container
        .as_ref()
        .and_then(|c| c.get_attribute("data-inverted"))
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let sketch = match local_malfeasance_delta(capital_allocated, delivered) {
        Some(delta) => format!(
            "Malfeasance delta sketch capital={capital_allocated} delivered={delivered}: {delta:.4}"
        ),
        None => format!(
            "Malfeasance inputs capital={capital_allocated} delivered={delivered}"
        ),
    };
    invoke_dual(
        document,
        label,
        "Econ.malfeasance_delta",
        sketch,
        json!({
            "capital_allocated": capital_allocated,
            "delivered": delivered,
            "inverted": inverted,
        }),
    );
}

/// `Econ.portfolio_variance` — weights + row-major covariance (split surface or defaults).
pub(super) fn run_portfolio_variance(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let n_assets = numeric_attr(container.as_ref(), "data-n-assets")
        .map(|n| n.round().max(1.0) as usize)
        .unwrap_or(2);
    let (weights, covariance) = if nums.len() >= n_assets + n_assets * n_assets {
        (
            nums[..n_assets].to_vec(),
            nums[n_assets..n_assets + n_assets * n_assets].to_vec(),
        )
    } else {
        (
            vec![0.5, 0.5],
            vec![0.04, 0.01, 0.01, 0.09],
        )
    };
    let n = n_assets
        .min(weights.len())
        .min(((covariance.len() as f64).sqrt().floor() as usize).max(1));
    let sketch = match local_portfolio_variance(&weights, &covariance, n) {
        Some(v) => format!("Portfolio variance sketch over {n} assets: {v:.6}"),
        None => format!("Portfolio variance inputs n_assets={n}"),
    };
    invoke_dual(
        document,
        label,
        "Econ.portfolio_variance",
        sketch,
        json!({
            "weights": weights,
            "covariance": covariance,
            "n_assets": n as u64,
        }),
    );
}

/// `Econ.portfolio_returns` — row-major asset returns + weights.
pub(super) fn run_portfolio_returns(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let n_assets = numeric_attr(container.as_ref(), "data-n-assets")
        .map(|n| n.round().max(1.0) as usize)
        .unwrap_or(2);
    let (asset_returns, weights, n_periods) = if nums.len() > n_assets {
        let weights = nums[nums.len() - n_assets..].to_vec();
        let flat = nums[..nums.len() - n_assets].to_vec();
        let n_periods = flat.len() / n_assets;
        (flat, weights, n_periods.max(1))
    } else {
        (
            vec![0.01, 0.02, -0.01, 0.03, 0.015, -0.005],
            vec![0.5, 0.5],
            3,
        )
    };
    let sketch = match local_portfolio_returns_summary(
        &asset_returns,
        &weights,
        n_periods,
        n_assets,
    ) {
        Some((count, last)) => format!(
            "Portfolio returns sketch over {count} periods (last≈{last:.4})"
        ),
        None => format!(
            "Portfolio returns inputs periods={n_periods} assets={n_assets}"
        ),
    };
    invoke_dual(
        document,
        label,
        "Econ.portfolio_returns",
        sketch,
        json!({
            "asset_returns": asset_returns,
            "weights": weights,
            "n_periods": n_periods as u64,
            "n_assets": n_assets as u64,
        }),
    );
}

/// `Econ.distributional_npv` — benefits/costs/weights from surface thirds or defaults.
pub(super) fn run_distributional_npv(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let discount_rate =
        numeric_attr(container.as_ref(), "data-discount-rate").unwrap_or(0.05);
    let (benefits, costs, weights) = if nums.len() >= 3 && nums.len() % 3 == 0 {
        let third = nums.len() / 3;
        (
            nums[..third].to_vec(),
            nums[third..2 * third].to_vec(),
            nums[2 * third..].to_vec(),
        )
    } else if nums.len() >= 2 && nums.len() % 2 == 0 {
        let half = nums.len() / 2;
        (
            nums[..half].to_vec(),
            nums[half..].to_vec(),
            vec![1.0; half],
        )
    } else {
        (
            vec![100.0, 100.0, 100.0],
            vec![80.0, 20.0, 10.0],
            vec![1.2, 1.0, 0.8],
        )
    };
    let n_periods = numeric_attr(container.as_ref(), "data-n-periods")
        .map(|n| n.round().max(1.0) as usize)
        .unwrap_or(benefits.len().min(costs.len()).min(weights.len()));
    let sketch = match local_distributional_npv(
        &benefits,
        &costs,
        &weights,
        discount_rate,
        n_periods,
    ) {
        Some((w, u)) => format!(
            "Distributional NPV sketch over {n_periods} periods: weighted={w:.4} unweighted={u:.4}"
        ),
        None => format!("Distributional NPV inputs r={discount_rate} n={n_periods}"),
    };
    invoke_dual(
        document,
        label,
        "Econ.distributional_npv",
        sketch,
        json!({
            "benefits": benefits,
            "costs": costs,
            "weights": weights,
            "discount_rate": discount_rate,
            "n_periods": n_periods as u64,
        }),
    );
}

/// `Econ.stress_scenario` — scale selected returns by shock attr.
pub(super) fn run_stress_scenario(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let mut returns = parse_numbers(&source);
    if returns.is_empty() {
        returns = vec![0.01, -0.02, 0.015, -0.01];
    }
    let shock = numeric_attr(container.as_ref(), "data-shock").unwrap_or(0.8);
    let sketch = match local_stress_scenario(&returns, shock) {
        Some((n, last)) => format!(
            "Stress-scenario sketch over {n} returns × shock={shock}: last≈{last:.4}"
        ),
        None => format!("Stress-scenario inputs shock={shock}"),
    };
    invoke_dual(
        document,
        label,
        "Econ.stress_scenario",
        sketch,
        json!({
            "returns": returns,
            "shock": shock,
        }),
    );
}

/// `Econ.repeated_game_payoff` — stage payoffs from surface; δ from attr.
pub(super) fn run_repeated_game_payoff(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let mut stage_payoffs = parse_numbers(&source);
    if stage_payoffs.is_empty() {
        stage_payoffs = vec![1.0, 1.0, 1.0];
    }
    let discount = numeric_attr(container.as_ref(), "data-discount").unwrap_or(0.9);
    let n_rounds = numeric_attr(container.as_ref(), "data-n-rounds")
        .map(|n| n.round().max(1.0) as usize)
        .unwrap_or(stage_payoffs.len());
    let sketch = match local_repeated_game_payoff(&stage_payoffs, discount, n_rounds) {
        Some(total) => format!(
            "Repeated-game payoff sketch over {n_rounds} rounds (δ={discount}): {total:.4}"
        ),
        None => format!("Repeated-game payoff inputs δ={discount} n={n_rounds}"),
    };
    invoke_dual(
        document,
        label,
        "Econ.repeated_game_payoff",
        sketch,
        json!({
            "stage_payoffs": stage_payoffs,
            "discount": discount,
            "n_rounds": n_rounds as u64,
        }),
    );
}

/// `Econ.total_transport_cost` — flows then distances (split halves) × n.
pub(super) fn run_total_transport_cost(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let n = numeric_attr(container.as_ref(), "data-n")
        .map(|v| v.round().max(1.0) as usize)
        .unwrap_or(2);
    let need = n * n;
    let (flows, distances) = if nums.len() >= 2 * need {
        (nums[..need].to_vec(), nums[need..2 * need].to_vec())
    } else {
        (
            vec![0.0, 10.0, 5.0, 0.0],
            vec![0.0, 2.0, 3.0, 0.0],
        )
    };
    let sketch = match local_total_transport_cost(&flows, &distances, n) {
        Some(total) => format!("Total transport-cost sketch n={n}: {total:.4}"),
        None => format!("Total transport-cost inputs n={n}"),
    };
    invoke_dual(
        document,
        label,
        "Econ.total_transport_cost",
        sketch,
        json!({
            "flows": flows,
            "distances": distances,
            "n": n as u64,
        }),
    );
}

/// `Econ.transition_probability` — row-major P; from/to from attrs.
pub(super) fn run_transition_probability(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let mut matrix = parse_numbers(&source);
    let n = numeric_attr(container.as_ref(), "data-n")
        .map(|v| v.round().max(1.0) as usize)
        .unwrap_or_else(|| {
            let len = matrix.len();
            let root = (len as f64).sqrt().round() as usize;
            root.max(1)
        });
    if matrix.len() < n * n {
        matrix = vec![0.7, 0.3, 0.4, 0.6];
    }
    let from = numeric_attr(container.as_ref(), "data-from")
        .map(|v| v.round().max(0.0) as u64)
        .unwrap_or(0);
    let to = numeric_attr(container.as_ref(), "data-to")
        .map(|v| v.round().max(0.0) as u64)
        .unwrap_or(1);
    let sketch = match local_transition_probability(&matrix, n, from as usize, to as usize) {
        Some(p) => format!("Transition P[{from}→{to}] sketch (n={n}): {p:.4}"),
        None => format!("Transition probability inputs n={n} from={from} to={to}"),
    };
    invoke_dual(
        document,
        label,
        "Econ.transition_probability",
        sketch,
        json!({
            "matrix": matrix,
            "n": n as u64,
            "from": from,
            "to": to,
        }),
    );
}

/// `Econ.expected_holding_time` — row-major P; state from attr.
pub(super) fn run_expected_holding_time(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let mut matrix = parse_numbers(&source);
    let n = numeric_attr(container.as_ref(), "data-n")
        .map(|v| v.round().max(1.0) as usize)
        .unwrap_or_else(|| {
            let len = matrix.len();
            let root = (len as f64).sqrt().round() as usize;
            root.max(1)
        });
    if matrix.len() < n * n {
        matrix = vec![0.7, 0.3, 0.4, 0.6];
    }
    let state = numeric_attr(container.as_ref(), "data-state")
        .map(|v| v.round().max(0.0) as u64)
        .unwrap_or(0);
    let sketch = match local_expected_holding_time(&matrix, n, state as usize) {
        Some(t) => format!("Expected holding-time sketch state={state} (n={n}): {t:.4}"),
        None => format!("Expected holding-time inputs n={n} state={state}"),
    };
    invoke_dual(
        document,
        label,
        "Econ.expected_holding_time",
        sketch,
        json!({
            "matrix": matrix,
            "n": n as u64,
            "state": state,
        }),
    );
}

/// `Econ.check_ir` — valuations then payments (split halves).
pub(super) fn run_check_ir(document: &Document, label: &str) {
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let (valuations, payments) = if nums.len() >= 2 && nums.len() % 2 == 0 {
        let half = nums.len() / 2;
        (nums[..half].to_vec(), nums[half..].to_vec())
    } else {
        (vec![10.0, 8.0, 6.0], vec![5.0, 4.0, 3.0])
    };
    let sketch = match local_check_ir(&valuations, &payments) {
        Some(ok) => format!(
            "Individual-rationality sketch over {} agents: {ok}",
            valuations.len()
        ),
        None => format!(
            "Individual-rationality inputs {} valuations / {} payments",
            valuations.len(),
            payments.len()
        ),
    };
    invoke_dual(
        document,
        label,
        "Econ.check_ir",
        sketch,
        json!({
            "valuations": valuations,
            "payments": payments,
        }),
    );
}

/// `Econ.vcg_payment` — valuations from selected surface numbers.
pub(super) fn run_vcg_payment(document: &Document, label: &str) {
    let source = selected_source(document).unwrap_or_default();
    let mut valuations = parse_numbers(&source);
    if valuations.is_empty() {
        valuations = vec![10.0, 8.0, 6.0];
    }
    let sketch = match local_vcg_payment(&valuations) {
        Some((payment, revenue)) => format!(
            "VCG/Vickrey payment sketch over {} bids: winner pays {payment:.4} (revenue={revenue:.4})",
            valuations.len()
        ),
        None => format!("VCG payment inputs {} valuations", valuations.len()),
    };
    invoke_dual(
        document,
        label,
        "Econ.vcg_payment",
        sketch,
        json!({ "valuations": valuations }),
    );
}

fn square_matrix_from_surface(document: &Document, default: Vec<f64>) -> (Vec<f64>, usize) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let mut matrix = parse_numbers(&source);
    let n = numeric_attr(container.as_ref(), "data-n")
        .map(|v| v.round().max(1.0) as usize)
        .unwrap_or_else(|| {
            let len = matrix.len();
            let root = (len as f64).sqrt().round() as usize;
            root.max(1)
        });
    if matrix.len() < n * n {
        matrix = default;
    }
    (matrix, n)
}

/// `Econ.validate_transition_matrix` — row-major P from surface.
pub(super) fn run_validate_transition_matrix(document: &Document, label: &str) {
    let (matrix, n) = square_matrix_from_surface(document, vec![0.7, 0.3, 0.4, 0.6]);
    let sketch = match local_validate_transition_matrix(&matrix, n) {
        Some(true) => format!("Transition-matrix validity sketch n={n}: valid"),
        Some(false) => format!("Transition-matrix validity sketch n={n}: invalid"),
        None => format!("Transition-matrix validity inputs n={n}"),
    };
    invoke_dual(
        document,
        label,
        "Econ.validate_transition_matrix",
        sketch,
        json!({
            "matrix": matrix,
            "n": n as u64,
        }),
    );
}

/// `Econ.stationary_distribution` — row-major P from surface.
pub(super) fn run_stationary_distribution(document: &Document, label: &str) {
    let (matrix, n) = square_matrix_from_surface(document, vec![0.7, 0.3, 0.4, 0.6]);
    let sketch = match local_stationary_distribution(&matrix, n) {
        Some(pi) => format!(
            "Stationary-distribution sketch n={n}: π≈[{:.4}, …] (len {})",
            pi.first().copied().unwrap_or(0.0),
            pi.len()
        ),
        None => format!("Stationary-distribution inputs n={n}"),
    };
    invoke_dual(
        document,
        label,
        "Econ.stationary_distribution",
        sketch,
        json!({
            "matrix": matrix,
            "n": n as u64,
        }),
    );
}

/// `Econ.mean_first_passage` — row-major P; target from attr.
pub(super) fn run_mean_first_passage(document: &Document, label: &str) {
    let container = selected_container(document);
    let (matrix, n) = square_matrix_from_surface(document, vec![0.7, 0.3, 0.4, 0.6]);
    let target = numeric_attr(container.as_ref(), "data-target")
        .map(|v| v.round().max(0.0) as u64)
        .unwrap_or(0);
    let sketch = match local_mean_first_passage(&matrix, n, target as usize) {
        Some(m) => format!(
            "Mean first-passage sketch to state {target} (n={n}): m0≈{:.4}",
            m.first().copied().unwrap_or(0.0)
        ),
        None => format!("Mean first-passage inputs n={n} target={target}"),
    };
    invoke_dual(
        document,
        label,
        "Econ.mean_first_passage",
        sketch,
        json!({
            "matrix": matrix,
            "n": n as u64,
            "target": target,
        }),
    );
}

/// `Econ.degree_centrality` — row-major adjacency from surface.
pub(super) fn run_degree_centrality(document: &Document, label: &str) {
    let (adjacency, n) = square_matrix_from_surface(document, vec![0.0, 1.0, 1.0, 0.0]);
    let sketch = match local_degree_centrality(&adjacency, n) {
        Some(c) => format!(
            "Degree-centrality sketch n={n}: degrees≈{c:?}"
        ),
        None => format!("Degree-centrality inputs n={n}"),
    };
    invoke_dual(
        document,
        label,
        "Econ.degree_centrality",
        sketch,
        json!({
            "adjacency": adjacency,
            "n": n as u64,
        }),
    );
}

/// `Econ.eigenvector_centrality` — row-major adjacency from surface.
pub(super) fn run_eigenvector_centrality(document: &Document, label: &str) {
    let (adjacency, n) = square_matrix_from_surface(document, vec![0.0, 1.0, 1.0, 0.0]);
    let sketch = match local_eigenvector_centrality(&adjacency, n) {
        Some(c) => format!(
            "Eigenvector-centrality sketch n={n}: c0≈{:.4}",
            c.first().copied().unwrap_or(0.0)
        ),
        None => format!("Eigenvector-centrality inputs n={n}"),
    };
    invoke_dual(
        document,
        label,
        "Econ.eigenvector_centrality",
        sketch,
        json!({
            "adjacency": adjacency,
            "n": n as u64,
        }),
    );
}

/// `Econ.new_keynesian_solve` — NK params from data-* attrs.
pub(super) fn run_new_keynesian_solve(document: &Document, label: &str) {
    let container = selected_container(document);
    let r_prev = numeric_attr(container.as_ref(), "data-r-prev").unwrap_or(0.02);
    let beta = numeric_attr(container.as_ref(), "data-beta").unwrap_or(0.99);
    let kappa = numeric_attr(container.as_ref(), "data-kappa").unwrap_or(0.1);
    let sigma = numeric_attr(container.as_ref(), "data-sigma").unwrap_or(1.0);
    let phi_pi = numeric_attr(container.as_ref(), "data-phi-pi").unwrap_or(1.5);
    let phi_y = numeric_attr(container.as_ref(), "data-phi-y").unwrap_or(0.5);
    let rho_r = numeric_attr(container.as_ref(), "data-rho-r").unwrap_or(0.8);
    let r_nat = numeric_attr(container.as_ref(), "data-r-nat").unwrap_or(0.02);
    let sketch = match local_new_keynesian_solve(
        r_prev, beta, kappa, sigma, phi_pi, phi_y, rho_r, r_nat,
    ) {
        Some((y, pi, r)) => {
            format!("New-Keynesian sketch: y_gap={y:.4} π={pi:.4} r={r:.4}")
        }
        None => "New-Keynesian inputs incomplete".to_string(),
    };
    invoke_dual(
        document,
        label,
        "Econ.new_keynesian_solve",
        sketch,
        json!({
            "r_prev": r_prev,
            "beta": beta,
            "kappa": kappa,
            "sigma": sigma,
            "phi_pi": phi_pi,
            "phi_y": phi_y,
            "rho_r": rho_r,
            "r_nat": r_nat,
        }),
    );
}

/// `Econ.nearest_facility` — demand then facility xy pairs (split halves).
pub(super) fn run_nearest_facility(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let n_demands = numeric_attr(container.as_ref(), "data-n-demands")
        .map(|v| v.round().max(1.0) as usize)
        .unwrap_or(2);
    let n_facilities = numeric_attr(container.as_ref(), "data-n-facilities")
        .map(|v| v.round().max(1.0) as usize)
        .unwrap_or(2);
    let need_d = n_demands * 2;
    let need_f = n_facilities * 2;
    let (demands, facilities) = if nums.len() >= need_d + need_f {
        (
            nums[..need_d].to_vec(),
            nums[need_d..need_d + need_f].to_vec(),
        )
    } else {
        (vec![0.0, 0.0, 1.0, 1.0], vec![0.0, 1.0, 1.0, 0.0])
    };
    let sketch = match local_nearest_facility(&demands, &facilities, n_demands, n_facilities) {
        Some(asg) => format!(
            "Nearest-facility sketch: {} demands → assignments {asg:?}",
            n_demands
        ),
        None => format!("Nearest-facility inputs demands={n_demands} facilities={n_facilities}"),
    };
    invoke_dual(
        document,
        label,
        "Econ.nearest_facility",
        sketch,
        json!({
            "demands": demands,
            "facilities": facilities,
            "n_demands": n_demands as u64,
            "n_facilities": n_facilities as u64,
        }),
    );
}

/// `Econ.pure_nash_equilibria` — eight numbers or Prisoner's Dilemma defaults.
pub(super) fn run_pure_nash_equilibria(document: &Document, label: &str) {
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let (payoff_row, payoff_col) = if nums.len() >= 8 {
        (nums[0..4].to_vec(), nums[4..8].to_vec())
    } else {
        (vec![3.0, 0.0, 5.0, 1.0], vec![3.0, 5.0, 0.0, 1.0])
    };
    let n_row = 2usize;
    let n_col = 2usize;
    let sketch = match local_pure_nash_equilibria(&payoff_row, &payoff_col, n_row, n_col) {
        Some(eqs) => format!(
            "Pure-Nash sketch: {} equilibria {eqs:?}",
            eqs.len()
        ),
        None => "Pure-Nash inputs incomplete".to_string(),
    };
    invoke_dual(
        document,
        label,
        "Econ.pure_nash_equilibria",
        sketch,
        json!({
            "payoff_row": payoff_row,
            "payoff_col": payoff_col,
            "n_row": n_row as u64,
            "n_col": n_col as u64,
        }),
    );
}

/// `Econ.morans_i` — values then weights (split); or illustrative 2×2.
pub(super) fn run_morans_i(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let n = numeric_attr(container.as_ref(), "data-n")
        .map(|v| v.round().max(2.0) as usize)
        .unwrap_or(2);
    let (values, weights) = if nums.len() >= n + n * n {
        (nums[..n].to_vec(), nums[n..n + n * n].to_vec())
    } else {
        (
            vec![1.0, 2.0],
            vec![0.0, 1.0, 1.0, 0.0],
        )
    };
    let sketch = match local_morans_i(&values, &weights, n) {
        Some(mi) => format!("Moran's I sketch n={n}: I={mi:.4}"),
        None => format!("Moran's I inputs n={n}"),
    };
    invoke_dual(
        document,
        label,
        "Econ.morans_i",
        sketch,
        json!({
            "values": values,
            "weights": weights,
            "n": n as u64,
        }),
    );
}

/// `Econ.strategy_proofness` — valuations / payments from surface; allocation defaults.
pub(super) fn run_strategy_proofness(document: &Document, label: &str) {
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let (valuations, payments) = if nums.len() >= 8 {
        (nums[0..4].to_vec(), nums[4..8].to_vec())
    } else {
        // Flat zero-payment / always-allocate rule is strategy-proof for agent 0.
        (vec![10.0, 5.0, 8.0, 4.0], vec![0.0, 0.0, 0.0, 0.0])
    };
    let allocation = vec![true, true, true, true];
    let sketch = match local_strategy_proofness(&valuations, &allocation, &payments) {
        Some(ok) => format!("Strategy-proofness sketch (2×2): {ok}"),
        None => "Strategy-proofness inputs incomplete".to_string(),
    };
    invoke_dual(
        document,
        label,
        "Econ.strategy_proofness",
        sketch,
        json!({
            "valuations": valuations,
            "allocation": allocation,
            "payments": payments,
        }),
    );
}

/// `Econ.lorenz_curve` — incomes from selected surface numbers.
pub(super) fn run_lorenz_curve(document: &Document, label: &str) {
    let source = selected_source(document).unwrap_or_default();
    let incomes = parse_numbers(&source);
    let incomes = if incomes.len() >= 2 {
        incomes
    } else {
        vec![10.0, 20.0, 30.0, 40.0]
    };
    let sketch = match local_lorenz_curve(&incomes) {
        Some((pop, share)) => format!("Lorenz endpoint pop={pop:.3} income={share:.3}"),
        None => "Lorenz inputs incomplete".to_string(),
    };
    invoke_dual(
        document,
        label,
        "Econ.lorenz_curve",
        sketch,
        json!({ "incomes": incomes }),
    );
}

/// `Econ.ols` — x then y (split halves) or illustrative y=2x.
pub(super) fn run_ols(document: &Document, label: &str) {
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let (x, y) = if nums.len() >= 4 && nums.len() % 2 == 0 {
        let mid = nums.len() / 2;
        (nums[..mid].to_vec(), nums[mid..].to_vec())
    } else {
        (vec![1.0, 2.0, 3.0, 4.0], vec![2.0, 4.0, 6.0, 8.0])
    };
    let n_obs = x.len().min(y.len());
    let sketch = match local_ols(&x[..n_obs], &y[..n_obs]) {
        Some((beta, r2)) => format!("OLS β={beta:.4} R²={r2:.4}"),
        None => "OLS inputs incomplete".to_string(),
    };
    invoke_dual(
        document,
        label,
        "Econ.ols",
        sketch,
        json!({
            "x": x[..n_obs].to_vec(),
            "y": y[..n_obs].to_vec(),
            "n_obs": n_obs as u64,
            "n_reg": 1u64,
        }),
    );
}

/// `Econ.wls` — x / y / weights thirds, or equal-weight y=2x.
pub(super) fn run_wls(document: &Document, label: &str) {
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let (x, y, weights) = if nums.len() >= 6 && nums.len() % 3 == 0 {
        let third = nums.len() / 3;
        (
            nums[..third].to_vec(),
            nums[third..2 * third].to_vec(),
            nums[2 * third..].to_vec(),
        )
    } else {
        (
            vec![1.0, 2.0, 3.0, 4.0],
            vec![2.0, 4.0, 6.0, 8.0],
            vec![1.0, 1.0, 1.0, 1.0],
        )
    };
    let n_obs = x.len().min(y.len()).min(weights.len());
    let sketch = match local_wls(&x[..n_obs], &y[..n_obs], &weights[..n_obs]) {
        Some((beta, r2)) => format!("WLS β={beta:.4} R²={r2:.4}"),
        None => "WLS inputs incomplete".to_string(),
    };
    invoke_dual(
        document,
        label,
        "Econ.wls",
        sketch,
        json!({
            "x": x[..n_obs].to_vec(),
            "y": y[..n_obs].to_vec(),
            "weights": weights[..n_obs].to_vec(),
            "n_obs": n_obs as u64,
            "n_reg": 1u64,
        }),
    );
}

/// `Econ.lucas_asset_price` — dividends then consumption (split); β/γ from attrs.
pub(super) fn run_lucas_asset_price(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let beta = numeric_attr(container.as_ref(), "data-beta").unwrap_or(0.99);
    let gamma = numeric_attr(container.as_ref(), "data-gamma").unwrap_or(2.0);
    let (dividends, consumption, n_paths, n_periods) = if nums.len() >= 4 && nums.len() % 2 == 0 {
        let mid = nums.len() / 2;
        let n_periods = mid;
        (
            nums[..mid].to_vec(),
            nums[mid..].to_vec(),
            1usize,
            n_periods,
        )
    } else {
        (vec![1.0, 1.1], vec![1.0, 1.05], 1usize, 2usize)
    };
    let sketch = match local_lucas_asset_price(
        &dividends,
        &consumption,
        n_paths,
        n_periods,
        beta,
        gamma,
    ) {
        Some(p) => format!("Lucas price ≈ {p:.4}"),
        None => "Lucas pricing inputs incomplete".to_string(),
    };
    invoke_dual(
        document,
        label,
        "Econ.lucas_asset_price",
        sketch,
        json!({
            "dividend_paths": dividends,
            "consumption_paths": consumption,
            "n_paths": n_paths as u64,
            "n_periods": n_periods as u64,
            "beta": beta,
            "gamma": gamma,
        }),
    );
}

fn default_two_state_mdp() -> (Vec<f64>, Vec<f64>, usize, usize) {
    // Same layout as Host DP tests: bad/good × stay/switch.
    let rewards = vec![0.0, -0.5, 2.0, 1.5];
    let transitions = vec![1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0];
    (rewards, transitions, 2, 2)
}

/// `Econ.bellman_update` — MDP from surface or illustrative 2-state; state/discount attrs.
pub(super) fn run_bellman_update(document: &Document, label: &str) {
    let container = selected_container(document);
    let discount = numeric_attr(container.as_ref(), "data-discount").unwrap_or(0.9);
    let state = numeric_attr(container.as_ref(), "data-state")
        .map(|s| s as u64)
        .unwrap_or(1);
    let (rewards, transitions, n_states, n_actions) = default_two_state_mdp();
    let values = vec![17.5, 20.0];
    let sketch = match local_bellman_update(
        &rewards,
        &transitions,
        discount,
        &values,
        n_states,
        n_actions,
        state as usize,
    ) {
        Some(v) => format!("Bellman V'({state}) ≈ {v:.4}"),
        None => "Bellman update inputs incomplete".to_string(),
    };
    invoke_dual(
        document,
        label,
        "Econ.bellman_update",
        sketch,
        json!({
            "rewards": rewards,
            "transitions": transitions,
            "discount": discount,
            "values": values,
            "n_states": n_states as u64,
            "n_actions": n_actions as u64,
            "state": state,
        }),
    );
}

/// `Econ.block_bootstrap` — returns from surface; block/n_resamples/seed from attrs.
pub(super) fn run_block_bootstrap(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let returns = parse_numbers(&source);
    let returns = if returns.len() >= 4 {
        returns
    } else {
        vec![0.01, -0.02, 0.015, 0.0, 0.02, -0.01]
    };
    let block_size = numeric_attr(container.as_ref(), "data-block-size")
        .map(|b| b as u64)
        .unwrap_or(2);
    let n_resamples = numeric_attr(container.as_ref(), "data-n-resamples")
        .map(|n| n as u64)
        .unwrap_or(8);
    let seed = numeric_attr(container.as_ref(), "data-seed")
        .map(|s| s as u64)
        .unwrap_or(42);
    let sketch = match local_block_bootstrap_mean_sketch(
        &returns,
        block_size as usize,
        n_resamples as usize,
    ) {
        Some((mean, n)) => format!("Bootstrap sketch mean={mean:.4} ({n} resamples)"),
        None => "Block bootstrap inputs incomplete".to_string(),
    };
    invoke_dual(
        document,
        label,
        "Econ.block_bootstrap",
        sketch,
        json!({
            "returns": returns,
            "block_size": block_size,
            "n_resamples": n_resamples,
            "seed": seed,
        }),
    );
}

/// `Econ.simulate_chain` — row-major P from surface; start/steps/seed from attrs.
pub(super) fn run_simulate_chain(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let (matrix, n) = if nums.len() >= 4 {
        let n = (nums.len() as f64).sqrt().round() as usize;
        if n * n == nums.len() && n >= 2 {
            (nums, n)
        } else {
            (vec![0.7, 0.3, 0.4, 0.6], 2)
        }
    } else {
        (vec![0.7, 0.3, 0.4, 0.6], 2)
    };
    let start = numeric_attr(container.as_ref(), "data-start")
        .map(|s| s as u64)
        .unwrap_or(0);
    let n_steps = numeric_attr(container.as_ref(), "data-n-steps")
        .map(|s| s as u64)
        .unwrap_or(5);
    let seed = numeric_attr(container.as_ref(), "data-seed")
        .map(|s| s as u64)
        .unwrap_or(42);
    let sketch = match local_simulate_chain_greedy(&matrix, n, start as usize, n_steps as usize) {
        Some(path) => format!("Markov greedy path {:?}", path),
        None => "Simulate-chain inputs incomplete".to_string(),
    };
    invoke_dual(
        document,
        label,
        "Econ.simulate_chain",
        sketch,
        json!({
            "matrix": matrix,
            "n": n as u64,
            "start": start,
            "n_steps": n_steps,
            "seed": seed,
        }),
    );
}

/// `Econ.value_iteration` — illustrative 2-state MDP; discount from attr.
pub(super) fn run_value_iteration(document: &Document, label: &str) {
    let container = selected_container(document);
    let discount = numeric_attr(container.as_ref(), "data-discount").unwrap_or(0.9);
    let max_iter = numeric_attr(container.as_ref(), "data-max-iter")
        .map(|m| m as u64)
        .unwrap_or(1000);
    let tolerance = numeric_attr(container.as_ref(), "data-tolerance").unwrap_or(1e-6);
    let (rewards, transitions, n_states, n_actions) = default_two_state_mdp();
    let sketch = match local_value_iteration(
        &rewards,
        &transitions,
        discount,
        n_states,
        n_actions,
        max_iter as u32,
        tolerance,
    ) {
        Some((values, policy)) => {
            format!("VFI V≈[{:.2}, {:.2}] π={:?}", values[0], values[1], policy)
        }
        None => "Value iteration inputs incomplete".to_string(),
    };
    invoke_dual(
        document,
        label,
        "Econ.value_iteration",
        sketch,
        json!({
            "rewards": rewards,
            "transitions": transitions,
            "discount": discount,
            "n_states": n_states as u64,
            "n_actions": n_actions as u64,
            "max_iter": max_iter,
            "tolerance": tolerance,
        }),
    );
}

/// `Econ.iv_2sls` — x / z / y thirds or illustrative just-identified case.
pub(super) fn run_iv_2sls(document: &Document, label: &str) {
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let (x, z, y) = if nums.len() >= 6 && nums.len() % 3 == 0 {
        let third = nums.len() / 3;
        (
            nums[..third].to_vec(),
            nums[third..2 * third].to_vec(),
            nums[2 * third..].to_vec(),
        )
    } else {
        (
            vec![1.0, 2.0, 3.0, 4.0],
            vec![1.1, 1.9, 3.1, 3.9],
            vec![2.0, 4.0, 6.0, 8.0],
        )
    };
    let n_obs = x.len().min(z.len()).min(y.len());
    let sketch = match local_iv_2sls(&x[..n_obs], &z[..n_obs], &y[..n_obs]) {
        Some((beta, r2)) => format!("2SLS β={beta:.4} R²={r2:.4}"),
        None => "IV/2SLS inputs incomplete".to_string(),
    };
    invoke_dual(
        document,
        label,
        "Econ.iv_2sls",
        sketch,
        json!({
            "x_endogenous": x[..n_obs].to_vec(),
            "z_instruments": z[..n_obs].to_vec(),
            "y": y[..n_obs].to_vec(),
            "n_obs": n_obs as u64,
            "n_reg": 1u64,
            "n_instr": 1u64,
        }),
    );
}

/// `Econ.logistic_mle` — x then binary y (split halves) or illustrative.
pub(super) fn run_logistic_mle(document: &Document, label: &str) {
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let (x, y) = if nums.len() >= 4 && nums.len() % 2 == 0 {
        let mid = nums.len() / 2;
        let x = nums[..mid].to_vec();
        let y: Vec<f64> = nums[mid..]
            .iter()
            .map(|v| if *v >= 0.5 { 1.0 } else { 0.0 })
            .collect();
        (x, y)
    } else {
        (
            vec![-2.0, -1.0, 1.0, 2.0],
            vec![0.0, 0.0, 1.0, 1.0],
        )
    };
    let n_obs = x.len().min(y.len());
    let sketch = match local_logistic_mle(&x[..n_obs], &y[..n_obs]) {
        Some(beta) => format!("Logistic β≈{beta:.4}"),
        None => "Logistic MLE inputs incomplete".to_string(),
    };
    invoke_dual(
        document,
        label,
        "Econ.logistic_mle",
        sketch,
        json!({
            "x": x[..n_obs].to_vec(),
            "y": y[..n_obs].to_vec(),
            "n_obs": n_obs as u64,
            "n_reg": 1u64,
        }),
    );
}

/// `Econ.interbank_clearing` — n×n exposures then n capital, or illustrative 2-bank.
pub(super) fn run_interbank_clearing(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let n = numeric_attr(container.as_ref(), "data-n")
        .map(|v| v.round().max(1.0) as usize)
        .unwrap_or(2)
        .min(8);
    let need = n * n + n;
    let (exposures, capital) = if nums.len() >= need {
        (
            nums[..n * n].to_vec(),
            nums[n * n..n * n + n].to_vec(),
        )
    } else {
        (vec![0.0, 100.0, 50.0, 0.0], vec![0.0, 0.0])
    };
    let sketch = match local_interbank_clearing(&exposures, &capital, n) {
        Some(p) => format!(
            "Interbank clearing sketch n={n}: p0≈{:.4}",
            p.first().copied().unwrap_or(0.0)
        ),
        None => format!("Interbank clearing inputs n={n}"),
    };
    invoke_dual(
        document,
        label,
        "Econ.interbank_clearing",
        sketch,
        json!({
            "exposures": exposures,
            "capital": capital,
            "n": n as u64,
            "max_rounds": 100u64,
            "tolerance": 1e-6,
        }),
    );
}

/// `Econ.leontief_inverse` — row-major technical coefficients A, or illustrative 2-sector.
pub(super) fn run_leontief_inverse(document: &Document, label: &str) {
    let (matrix, n) = square_matrix_from_surface(document, vec![0.0, 0.5, 0.5, 0.0]);
    let n = n.min(8);
    let sketch = match local_leontief_inverse(&matrix, n) {
        Some(inv) => format!(
            "Leontief inverse sketch n={n}: L00≈{:.4}",
            inv.first().copied().unwrap_or(0.0)
        ),
        None => format!("Leontief inverse inputs n={n}"),
    };
    invoke_dual(
        document,
        label,
        "Econ.leontief_inverse",
        sketch,
        json!({
            "matrix": matrix,
            "n": n as u64,
            "max_rounds": 100u64,
            "tolerance": 1e-9,
        }),
    );
}

/// `Econ.output_multipliers` — surface as Leontief inverse (or tech A → local inverse).
pub(super) fn run_output_multipliers(document: &Document, label: &str) {
    let (matrix, n) = square_matrix_from_surface(document, vec![0.0, 0.5, 0.5, 0.0]);
    let n = n.min(8);
    let inverse = local_leontief_inverse(&matrix, n).unwrap_or_else(|| {
        // Treat surface as already-inverted when Neumann fails (e.g. user pasted L).
        matrix.clone()
    });
    let sketch = match local_output_multipliers(&inverse, n) {
        Some(m) => format!(
            "Output multipliers sketch n={n}: m0≈{:.4}",
            m.first().copied().unwrap_or(0.0)
        ),
        None => format!("Output multipliers inputs n={n}"),
    };
    invoke_dual(
        document,
        label,
        "Econ.output_multipliers",
        sketch,
        json!({
            "leontief_inverse": inverse,
            "n": n as u64,
        }),
    );
}

/// `Econ.agent_based_aggregate_wealth` — Host metadata status (no Agent structs on surface).
pub(super) fn run_agent_based_aggregate_wealth(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Econ.agent_based_aggregate_wealth",
        "Agent-based aggregate wealth: Host status probe (Agent structs not on surface)."
            .to_string(),
        json!({}),
    );
}

/// `Econ.validate_scalar_constraint` — value/min/max from attrs or three surface numbers.
pub(super) fn run_validate_scalar_constraint(document: &Document, label: &str) {
    let container = selected_container(document);
    let source = selected_source(document).unwrap_or_default();
    let nums = parse_numbers(&source);
    let value = numeric_attr(container.as_ref(), "data-value")
        .or_else(|| nums.first().copied())
        .unwrap_or(0.05);
    let min = numeric_attr(container.as_ref(), "data-min")
        .or_else(|| nums.get(1).copied())
        .unwrap_or(0.0);
    let max = numeric_attr(container.as_ref(), "data-max")
        .or_else(|| nums.get(2).copied())
        .unwrap_or(1.0);
    let sketch = match local_validate_scalar_constraint(value, min, max) {
        Some(true) => format!("Scalar constraint sketch: {value} ∈ [{min}, {max}] ✓"),
        Some(false) => format!("Scalar constraint sketch: {value} ∉ [{min}, {max}]"),
        None => "Scalar constraint inputs incomplete".to_string(),
    };
    invoke_dual(
        document,
        label,
        "Econ.validate_scalar_constraint",
        sketch,
        json!({
            "value": value,
            "min": min,
            "max": max,
        }),
    );
}

/// `Econ.aggregate_paper_fills` — Host metadata status (Fill structs not on surface).
pub(super) fn run_aggregate_paper_fills(document: &Document, label: &str) {
    invoke_dual(
        document,
        label,
        "Econ.aggregate_paper_fills",
        "Paper-trading fill aggregate: Host status probe (Fill structs not on surface)."
            .to_string(),
        json!({}),
    );
}

#[cfg(test)]
mod tests {
    use super::{
        local_abatement_net_benefit, local_atkinson, local_autocorrelation, local_bertrand,
        local_bertrand_with_demand, local_binomial_one_step, local_capm_beta,
        local_ccapm_equity_premium, local_ccapm_sdf, local_check_budget_balance, local_check_ir,
        local_cournot, local_covariance_matrix, local_degree_centrality, local_discount_factor,
        local_distributional_npv, local_efficiency_units, local_endowment_effect,
        local_eigenvector_centrality, local_expected_holding_time, local_fiscal_multiplier,
        local_forward_rate_two_point, local_gbm_drift_endpoint, local_gini, local_gordon,
        local_gravity_flow, local_headcount_poverty, local_historical_cvar, local_historical_var,
        local_household_production_ces, local_hyperbolic_discount, local_interpolate_zero_rate,
        local_labor_supply, local_laffer_curve, local_log_returns_summary, local_malfeasance_delta,
        local_marginal_damage, local_max_drawdown, local_mean_first_passage, local_mean_return,
        local_morans_i, local_multi_period_ddm, local_nash_welfare, local_nearest_facility,
        local_new_keynesian_solve, local_npv, local_olg_steady_state, local_optimal_abatement,
        local_optimal_pollution, local_parametric_var, local_par_yield, local_pearson,
        local_pollution_damage, local_portfolio_max_drawdown, local_portfolio_returns_summary,
        local_portfolio_variance, local_poverty_gap, local_present_biased_utility,
        local_probability_weight, local_progressive_tax, local_prospect_value,
        local_pure_nash_equilibria, local_put_call_parity, local_ramsey_euler_residual,
        local_ramsey_steady_state, local_rawlsian_welfare, local_reference_dependent_utility,
        local_repeated_game_payoff, local_rolling_mean_last, local_rolling_variance_last,
        local_sample_variance, local_simple_returns_summary, local_social_cost_of_carbon,
        local_bellman_update, local_block_bootstrap_mean_sketch, local_interbank_clearing,
        local_iv_2sls, local_leontief_inverse, local_logistic_mle, local_lorenz_curve,
        local_lucas_asset_price, local_ols, local_output_multipliers,
        local_simulate_chain_greedy, local_stackelberg, local_stationary_distribution,
        local_strategy_proofness, local_stress_scenario, local_total_transport_cost,
        local_transfer_payment, local_transition_probability, local_utilitarian_welfare,
        local_validate_scalar_constraint, local_validate_transition_matrix,
        local_value_iteration, local_vcg_payment, local_wls,
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

    #[test]
    fn local_hyperbolic_present_is_one() {
        assert!((local_hyperbolic_discount(0, 0.7, 0.99).unwrap() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn local_hyperbolic_future_is_beta_delta() {
        let d = local_hyperbolic_discount(1, 0.7, 0.99).unwrap();
        assert!((d - 0.7 * 0.99).abs() < 1e-12);
    }

    #[test]
    fn local_fiscal_multiplier_basic() {
        let m = local_fiscal_multiplier(100.0, 0.5, 0.0).unwrap();
        assert!((m - 200.0).abs() < 1e-9);
    }

    #[test]
    fn local_headcount_half_poor() {
        let (c, r) = local_headcount_poverty(&[1.0, 3.0], 2.0).unwrap();
        assert_eq!(c, 1);
        assert!((r - 0.5).abs() < 1e-12);
    }

    #[test]
    fn local_max_drawdown_flat_is_zero() {
        assert!((local_max_drawdown(&[1.0, 1.0, 1.0]).unwrap()).abs() < 1e-12);
    }

    #[test]
    fn local_binomial_one_step_positive() {
        let p = local_binomial_one_step(100.0, 100.0, 1.0, 0.03, 0.2, 0.0, true).unwrap();
        assert!(p > 0.0);
    }

    #[test]
    fn local_gbm_drift_endpoint_grows_with_mu() {
        let end = local_gbm_drift_endpoint(100.0, 0.1, 0.0, 1.0, 1).unwrap();
        assert!((end - 100.0 * 0.1f64.exp()).abs() < 1e-9);
    }

    #[test]
    fn local_forward_rate_two_point_finite() {
        let f = local_forward_rate_two_point(0.03, 1.0, 0.04, 2.0).unwrap();
        assert!(f.is_finite());
        assert!(f > 0.0);
    }

    #[test]
    fn local_covariance_matrix_2x2() {
        // 3 periods × 2 assets, flat: [[1,2],[3,4],[5,6]]
        let returns = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let cov = local_covariance_matrix(&returns, 3, 2).unwrap();
        assert_eq!(cov.len(), 4);
        assert!(cov[0] > 0.0);
        assert!((cov[1] - cov[2]).abs() < 1e-12);
    }

    #[test]
    fn local_capm_beta_unit_when_identical() {
        let r = [0.01, 0.02, -0.01, 0.03];
        let beta = local_capm_beta(&r, &r).unwrap();
        assert!((beta - 1.0).abs() < 1e-9);
    }

    #[test]
    fn local_autocorrelation_rejects_bad_lag() {
        assert!(local_autocorrelation(&[1.0, 2.0, 3.0], 0).is_none());
        assert!(local_autocorrelation(&[1.0, 2.0, 3.0], 3).is_none());
    }

    #[test]
    fn local_pearson_perfect_correlation() {
        let r = local_pearson(&[1.0, 2.0, 3.0], &[2.0, 4.0, 6.0]).unwrap();
        assert!((r - 1.0).abs() < 1e-9);
    }

    #[test]
    fn local_bertrand_with_demand_asymmetric() {
        let (p, q) = local_bertrand_with_demand(100.0, 1.0, 10.0, 20.0).unwrap();
        assert!((p - 20.0).abs() < 1e-9);
        assert!((q - 80.0).abs() < 1e-9);
    }

    #[test]
    fn local_check_budget_balance_surplus() {
        let (ok, s) = local_check_budget_balance(&[1.0, -0.5, 0.25]).unwrap();
        assert!(ok);
        assert!((s - 0.75).abs() < 1e-12);
    }

    #[test]
    fn local_ccapm_equity_premium_positive() {
        let p = local_ccapm_equity_premium(2.0, 0.02, 0.15).unwrap();
        assert!(p > 0.0);
    }

    #[test]
    fn local_mean_return_basic() {
        let m = local_mean_return(&[0.01, 0.03, -0.01]).unwrap();
        assert!((m - 0.01).abs() < 1e-12);
    }

    #[test]
    fn local_poverty_gap_basic() {
        let g = local_poverty_gap(&[5.0, 15.0, 25.0], 10.0).unwrap();
        assert!((g - 1.0 / 6.0).abs() < 1e-12);
    }
    #[test]
    fn local_sample_variance_basic() {
        let v = local_sample_variance(&[1.0, 2.0, 3.0]).unwrap();
        assert!((v - 1.0).abs() < 1e-12);
    }

    #[test]
    fn local_utilitarian_is_sum() {
        let w = local_utilitarian_welfare(&[1.0, 2.0, 3.0]).unwrap();
        assert!((w - 6.0).abs() < 1e-12);
    }

    #[test]
    fn local_rawlsian_is_min() {
        let w = local_rawlsian_welfare(&[3.0, 1.0, 2.0]).unwrap();
        assert!((w - 1.0).abs() < 1e-12);
    }

    #[test]
    fn local_nash_welfare_product() {
        let w = local_nash_welfare(&[2.0, 3.0, 4.0]).unwrap();
        assert!((w - 24.0).abs() < 1e-12);
    }

    #[test]
    fn local_stackelberg_symmetric_costs() {
        let (q1, q2, price) = local_stackelberg(100.0, 1.0, 10.0, 10.0).unwrap();
        assert!(q1 > q2);
        assert!(price > 0.0);
    }

    #[test]
    fn local_put_call_parity_zero_when_matched() {
        let s = 100.0;
        let k = 100.0;
        let r = 0.03;
        let q = 0.0;
        let t = 1.0;
        let put = 5.0;
        let call = put + s * f64::exp(-q * t) - k * f64::exp(-r * t);
        let err = local_put_call_parity(call, put, s, k, r, q, t).unwrap();
        assert!(err.abs() < 1e-9);
    }

    #[test]
    fn local_parametric_var_positive() {
        let v = local_parametric_var(0.0, 0.02, 0.95).unwrap();
        assert!(v > 0.0);
    }

    #[test]
    fn local_laffer_zero_at_extremes() {
        assert!((local_laffer_curve(0.0, 100.0, 1.0).unwrap()).abs() < 1e-12);
        assert!((local_laffer_curve(1.0, 100.0, 1.0).unwrap()).abs() < 1e-12);
    }

    #[test]
    fn local_historical_cvar_positive() {
        let c = local_historical_cvar(&[-0.10, -0.05, 0.0, 0.02, 0.03], 0.8).unwrap();
        assert!(c >= 0.0);
    }

    #[test]
    fn local_endowment_effect_scales() {
        let wta = local_endowment_effect(10.0, 2.25).unwrap();
        assert!((wta - 22.5).abs() < 1e-12);
    }

    #[test]
    fn local_prospect_value_gain() {
        let v = local_prospect_value(100.0, 0.88, 0.88, 2.25).unwrap();
        assert!((v - 100.0f64.powf(0.88)).abs() < 1e-9);
    }

    #[test]
    fn local_probability_weight_rejects_zero() {
        assert!(local_probability_weight(0.0, 0.65).is_none());
    }

    #[test]
    fn local_ccapm_sdf_basic() {
        let m = local_ccapm_sdf(1.02, 2.0, 0.99).unwrap();
        assert!((m - 0.99 * 1.02f64.powf(-2.0)).abs() < 1e-12);
    }

    #[test]
    fn local_gravity_flow_basic() {
        let f = local_gravity_flow(100.0, 80.0, 10.0, 1.0, 1.0, 2.0).unwrap();
        assert!((f - 80.0).abs() < 1e-9);
    }

    #[test]
    fn local_transfer_payment_phaseout() {
        let t = local_transfer_payment(500.0, 600.0, 400.0, 0.5).unwrap();
        assert!((t - 400.0).abs() < 1e-12);
    }

    #[test]
    fn local_efficiency_units_product() {
        let e = local_efficiency_units(40.0, 1.5).unwrap();
        assert!((e - 60.0).abs() < 1e-12);
    }

    #[test]
    fn local_social_cost_of_carbon_product() {
        let s = local_social_cost_of_carbon(100.0, 50.0).unwrap();
        assert!((s - 5000.0).abs() < 1e-12);
    }

    #[test]
    fn local_pollution_damage_quadratic() {
        let d = local_pollution_damage(10.0, 0.5).unwrap();
        assert!((d - 25.0).abs() < 1e-12);
    }

    #[test]
    fn local_marginal_damage_linear() {
        let md = local_marginal_damage(10.0, 0.5).unwrap();
        assert!((md - 5.0).abs() < 1e-12);
    }

    #[test]
    fn local_ramsey_steady_state_positive() {
        let k = local_ramsey_steady_state(0.33, 0.96, 0.08).unwrap();
        assert!(k > 0.0);
        assert!(k.is_finite());
    }

    #[test]
    fn local_simple_returns_summary_basic() {
        let (n, last) = local_simple_returns_summary(&[100.0, 110.0, 121.0]).unwrap();
        assert_eq!(n, 2);
        assert!((last - 0.1).abs() < 1e-12);
    }

    #[test]
    fn local_log_returns_summary_basic() {
        let (n, last) = local_log_returns_summary(&[100.0, 110.0]).unwrap();
        assert_eq!(n, 1);
        assert!((last - (1.1f64).ln()).abs() < 1e-12);
    }

    #[test]
    fn local_rolling_mean_last_window() {
        let (count, mean) = local_rolling_mean_last(&[1.0, 2.0, 3.0, 4.0], 3).unwrap();
        assert_eq!(count, 2);
        assert!((mean - 3.0).abs() < 1e-12);
    }

    #[test]
    fn local_rolling_variance_last_window() {
        let (count, var) = local_rolling_variance_last(&[1.0, 2.0, 3.0], 3).unwrap();
        assert_eq!(count, 1);
        assert!((var - (2.0 / 3.0)).abs() < 1e-12);
    }

    #[test]
    fn local_labor_supply_no_nonlabor() {
        let (h, c) = local_labor_supply(20.0, 24.0, 0.0, 0.5).unwrap();
        assert!((h - 12.0).abs() < 1e-12);
        assert!((c - 240.0).abs() < 1e-12);
    }

    #[test]
    fn local_optimal_pollution_and_abatement() {
        let e = local_optimal_pollution(100.0, 1.0, 1.0).unwrap();
        assert!((e - 50.0).abs() < 1e-12);
        let a = local_optimal_abatement(100.0, 1.0, 1.0).unwrap();
        assert!((a - 50.0).abs() < 1e-12);
    }

    #[test]
    fn local_olg_steady_state_positive() {
        let (k, y) = local_olg_steady_state(0.33, 0.96, 0.0).unwrap();
        assert!(k > 0.0 && y > 0.0);
    }

    #[test]
    fn local_ramsey_euler_residual_finite() {
        let r = local_ramsey_euler_residual(1.0, 1.0, 0.96, 0.33, 0.08, 1.0).unwrap();
        assert!(r.is_finite());
    }

    #[test]
    fn local_present_biased_utility_present() {
        let u = local_present_biased_utility(&[10.0, 10.0], 0.7, 0.99).unwrap();
        assert!((u - (10.0 + 0.7 * 0.99 * 10.0)).abs() < 1e-12);
    }

    #[test]
    fn local_reference_dependent_matches_prospect() {
        let u = local_reference_dependent_utility(110.0, 10.0, 0.88, 0.88, 2.25).unwrap();
        let p = local_prospect_value(100.0, 0.88, 0.88, 2.25).unwrap();
        assert!((u - p).abs() < 1e-12);
    }

    #[test]
    fn local_npv_basic() {
        let npv = local_npv(&[100.0, 0.0], &[50.0, 0.0], 0.0, 2).unwrap();
        assert!((npv - 50.0).abs() < 1e-12);
    }

    #[test]
    fn local_multi_period_ddm_positive() {
        let p = local_multi_period_ddm(&[2.0, 2.0], 0.1, 0.02).unwrap();
        assert!(p > 0.0);
    }

    #[test]
    fn local_portfolio_max_drawdown_signed() {
        let md = local_portfolio_max_drawdown(&[0.1, -0.5, 0.0]).unwrap();
        assert!(md < 0.0);
    }

    #[test]
    fn local_interpolate_zero_rate_midpoint() {
        let r = local_interpolate_zero_rate(&[1.0, 2.0], &[0.03, 0.05], 1.5).unwrap();
        assert!((r - 0.04).abs() < 1e-12);
    }

    #[test]
    fn local_discount_factor_unit_at_zero() {
        let df = local_discount_factor(&[1.0], &[0.05], 0.0, 1).unwrap();
        assert!((df - 1.0).abs() < 1e-12);
    }

    #[test]
    fn local_par_yield_flat_matches_zero() {
        let y = local_par_yield(&[1.0, 2.0], &[0.04, 0.04], 2.0, 1).unwrap();
        assert!((y - 0.04).abs() < 1e-9);
    }

    #[test]
    fn local_progressive_tax_hand() {
        let (total, _) =
            local_progressive_tax(120_000.0, &[0.0, 50_000.0, 100_000.0], &[0.1, 0.2, 0.3])
                .unwrap();
        assert!((total - 21_000.0).abs() < 1e-6);
    }

    #[test]
    fn local_abatement_net_benefit_zero_at_baseline() {
        let nb = local_abatement_net_benefit(100.0, 100.0, 1.0, 1.0).unwrap();
        assert!(nb.abs() < 1e-12);
    }

    #[test]
    fn local_household_production_ces_linear() {
        // ρ = 1 → alpha*time + (1-alpha)*goods
        let out = local_household_production_ces(8.0, 10.0, 0.5, 1.0).unwrap();
        assert!((out - 9.0).abs() < 1e-12);
    }

    #[test]
    fn local_malfeasance_delta_basic() {
        let d = local_malfeasance_delta(100.0, 80.0).unwrap();
        assert!((d - 20.0).abs() < 1e-12);
    }

    #[test]
    fn local_portfolio_variance_equal_weights() {
        let v = local_portfolio_variance(&[0.5, 0.5], &[0.04, 0.0, 0.0, 0.04], 2).unwrap();
        assert!((v - 0.02).abs() < 1e-12);
    }

    #[test]
    fn local_portfolio_returns_summary_last() {
        let (n, last) =
            local_portfolio_returns_summary(&[0.1, 0.0, 0.0, 0.2], &[0.5, 0.5], 2, 2).unwrap();
        assert_eq!(n, 2);
        assert!((last - 0.1).abs() < 1e-12);
    }

    #[test]
    fn local_distributional_npv_weights() {
        let (w, u) =
            local_distributional_npv(&[10.0], &[0.0], &[2.0], 0.0, 1).unwrap();
        assert!((u - 10.0).abs() < 1e-12);
        assert!((w - 20.0).abs() < 1e-12);
    }

    #[test]
    fn local_stress_scenario_scales() {
        let (n, last) = local_stress_scenario(&[1.0, 2.0], 0.5).unwrap();
        assert_eq!(n, 2);
        assert!((last - 1.0).abs() < 1e-12);
    }

    #[test]
    fn local_repeated_game_payoff_geometric() {
        let p = local_repeated_game_payoff(&[1.0, 1.0], 0.5, 2).unwrap();
        assert!((p - 1.5).abs() < 1e-12);
    }

    #[test]
    fn local_total_transport_cost_hand() {
        let c = local_total_transport_cost(&[0.0, 2.0, 1.0, 0.0], &[0.0, 3.0, 4.0, 0.0], 2)
            .unwrap();
        assert!((c - 10.0).abs() < 1e-12);
    }

    #[test]
    fn local_transition_probability_lookup() {
        let p = local_transition_probability(&[0.7, 0.3, 0.4, 0.6], 2, 0, 1).unwrap();
        assert!((p - 0.3).abs() < 1e-12);
    }

    #[test]
    fn local_expected_holding_time_basic() {
        let t = local_expected_holding_time(&[0.5, 0.5, 0.2, 0.8], 2, 0).unwrap();
        assert!((t - 2.0).abs() < 1e-12);
    }

    #[test]
    fn local_check_ir_passes() {
        assert_eq!(local_check_ir(&[10.0, 8.0], &[5.0, 4.0]), Some(true));
        assert_eq!(local_check_ir(&[10.0, 8.0], &[11.0, 4.0]), Some(false));
    }

    #[test]
    fn local_vcg_payment_second_price() {
        let (pay, rev) = local_vcg_payment(&[10.0, 8.0, 6.0]).unwrap();
        assert!((pay - 8.0).abs() < 1e-12);
        assert!((rev - 8.0).abs() < 1e-12);
    }

    #[test]
    fn local_validate_transition_matrix_ok() {
        assert_eq!(
            local_validate_transition_matrix(&[0.7, 0.3, 0.4, 0.6], 2),
            Some(true)
        );
        assert_eq!(
            local_validate_transition_matrix(&[0.7, 0.2, 0.4, 0.6], 2),
            Some(false)
        );
    }

    #[test]
    fn local_stationary_distribution_sums_to_one() {
        let pi = local_stationary_distribution(&[0.7, 0.3, 0.4, 0.6], 2).unwrap();
        assert!((pi.iter().sum::<f64>() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn local_mean_first_passage_target_zero() {
        let m = local_mean_first_passage(&[0.5, 0.5, 0.2, 0.8], 2, 0).unwrap();
        assert!((m[0]).abs() < 1e-12);
        assert!(m[1] > 0.0);
    }

    #[test]
    fn local_degree_centrality_row_sums() {
        let c = local_degree_centrality(&[0.0, 1.0, 2.0, 0.0], 2).unwrap();
        assert!((c[0] - 1.0).abs() < 1e-12);
        assert!((c[1] - 2.0).abs() < 1e-12);
    }

    #[test]
    fn local_eigenvector_centrality_unit_norm() {
        let c = local_eigenvector_centrality(&[0.0, 1.0, 1.0, 0.0], 2).unwrap();
        let norm = (c[0] * c[0] + c[1] * c[1]).sqrt();
        assert!((norm - 1.0).abs() < 1e-6);
    }

    #[test]
    fn local_new_keynesian_solve_finite() {
        let (y, pi, r) =
            local_new_keynesian_solve(0.02, 0.99, 0.1, 1.0, 1.5, 0.5, 0.8, 0.02).unwrap();
        assert!(y.is_finite() && pi.is_finite() && r.is_finite());
    }

    #[test]
    fn local_nearest_facility_assigns_closest() {
        let asg =
            local_nearest_facility(&[0.0, 0.0, 1.0, 1.0], &[0.0, 1.0, 1.0, 0.0], 2, 2).unwrap();
        assert_eq!(asg.len(), 2);
        assert!(asg[0] == 0 || asg[0] == 1);
    }

    #[test]
    fn local_pure_nash_pd_has_defect() {
        let eqs = local_pure_nash_equilibria(
            &[3.0, 0.0, 5.0, 1.0],
            &[3.0, 5.0, 0.0, 1.0],
            2,
            2,
        )
        .unwrap();
        assert!(eqs.contains(&(1, 1)));
    }

    #[test]
    fn local_morans_i_finite() {
        let i = local_morans_i(&[1.0, 2.0], &[0.0, 1.0, 1.0, 0.0], 2).unwrap();
        assert!(i.is_finite());
    }

    #[test]
    fn local_strategy_proofness_basic() {
        let ok = local_strategy_proofness(
            &[10.0, 5.0, 8.0, 4.0],
            &[true, true, true, true],
            &[0.0, 0.0, 0.0, 0.0],
        )
        .unwrap();
        assert!(ok);
        let bad = local_strategy_proofness(
            &[10.0, 5.0, 8.0, 4.0],
            &[true, true, false, false],
            &[5.0, 0.0, 5.0, 0.0],
        )
        .unwrap();
        assert!(!bad);
    }

    #[test]
    fn local_lorenz_curve_endpoint_is_one() {
        let (pop, share) = local_lorenz_curve(&[10.0, 20.0, 30.0]).unwrap();
        assert!((pop - 1.0).abs() < 1e-12);
        assert!((share - 1.0).abs() < 1e-12);
    }

    #[test]
    fn local_ols_recovers_slope() {
        let (beta, r2) = local_ols(&[1.0, 2.0, 3.0], &[2.0, 4.0, 6.0]).unwrap();
        assert!((beta - 2.0).abs() < 1e-12);
        assert!((r2 - 1.0).abs() < 1e-12);
    }

    #[test]
    fn local_wls_equal_weights_matches_ols() {
        let (b_ols, _) = local_ols(&[1.0, 2.0, 3.0], &[2.0, 4.0, 6.0]).unwrap();
        let (b_wls, _) =
            local_wls(&[1.0, 2.0, 3.0], &[2.0, 4.0, 6.0], &[1.0, 1.0, 1.0]).unwrap();
        assert!((b_ols - b_wls).abs() < 1e-12);
    }

    #[test]
    fn local_lucas_asset_price_positive() {
        let p = local_lucas_asset_price(&[1.0, 1.0], &[1.0, 1.0], 1, 2, 0.99, 2.0).unwrap();
        assert!(p > 0.0);
    }

    #[test]
    fn local_bellman_update_good_state() {
        let rewards = [0.0, -0.5, 2.0, 1.5];
        let transitions = [1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0];
        let values = [17.5, 20.0];
        let v = local_bellman_update(&rewards, &transitions, 0.9, &values, 2, 2, 1).unwrap();
        assert!((v - 20.0).abs() < 1e-9);
    }

    #[test]
    fn local_block_bootstrap_mean_sketch_basic() {
        let (mean, n) = local_block_bootstrap_mean_sketch(&[0.0, 0.1, -0.1, 0.2], 2, 4).unwrap();
        assert!((mean - 0.05).abs() < 1e-12);
        assert_eq!(n, 4);
    }

    #[test]
    fn local_simulate_chain_greedy_stays_on_peak() {
        let path = local_simulate_chain_greedy(&[0.9, 0.1, 0.2, 0.8], 2, 0, 3).unwrap();
        assert_eq!(path[0], 0);
        assert_eq!(path.len(), 4);
    }

    #[test]
    fn local_value_iteration_recovers_known() {
        let rewards = [0.0, -0.5, 2.0, 1.5];
        let transitions = [1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0];
        let (values, policy) =
            local_value_iteration(&rewards, &transitions, 0.9, 2, 2, 1000, 1e-12).unwrap();
        assert!((values[0] - 17.5).abs() < 1e-4);
        assert!((values[1] - 20.0).abs() < 1e-4);
        assert_eq!(policy[0], 1);
        assert_eq!(policy[1], 0);
    }

    #[test]
    fn local_iv_2sls_recovers_slope() {
        let (beta, _) =
            local_iv_2sls(&[1.0, 2.0, 3.0, 4.0], &[1.0, 2.0, 3.0, 4.0], &[2.0, 4.0, 6.0, 8.0])
                .unwrap();
        assert!((beta - 2.0).abs() < 1e-9);
    }

    #[test]
    fn local_logistic_mle_separates() {
        let beta = local_logistic_mle(&[-2.0, -1.0, 1.0, 2.0], &[0.0, 0.0, 1.0, 1.0]).unwrap();
        assert!(beta > 0.0);
    }

    #[test]
    fn local_interbank_clearing_mutual() {
        let p = local_interbank_clearing(&[0.0, 100.0, 50.0, 0.0], &[0.0, 0.0], 2).unwrap();
        assert!((p[0] - 50.0).abs() < 1e-3);
        assert!((p[1] - 50.0).abs() < 1e-3);
    }

    #[test]
    fn local_leontief_inverse_two_sector() {
        let inv = local_leontief_inverse(&[0.0, 0.5, 0.5, 0.0], 2).unwrap();
        assert!((inv[0] - 4.0 / 3.0).abs() < 1e-6);
        assert!((inv[1] - 2.0 / 3.0).abs() < 1e-6);
    }

    #[test]
    fn local_output_multipliers_from_known_inverse() {
        let inv = local_leontief_inverse(&[0.0, 0.5, 0.5, 0.0], 2).unwrap();
        let m = local_output_multipliers(&inv, 2).unwrap();
        assert!((m[0] - 2.0).abs() < 1e-6);
        assert!((m[1] - 2.0).abs() < 1e-6);
    }

    #[test]
    fn local_validate_scalar_constraint_bounds() {
        assert_eq!(local_validate_scalar_constraint(0.5, 0.0, 1.0), Some(true));
        assert_eq!(local_validate_scalar_constraint(1.5, 0.0, 1.0), Some(false));
    }
}
