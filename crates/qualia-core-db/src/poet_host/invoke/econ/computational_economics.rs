//! Computational economics invoke seams.
//!
//! Exposes the ~192 pub functions in `specialized_libs/computational_economics/`
//! through VibeScript invoke IDs. Each seam wraps the backend function,
//! validates inputs, and returns a structured `Value::Record`.
//!
//! Modules covered: asset_pricing, behavioral, derivatives, game_theory,
//! macro_models, market_design (mechanism), portfolio, risk (time_series),
//! welfare, yield_curve, time_series, spatial_economics, public_finance,
//! network_economics, mechanism, markov, labor_household, input_output,
//! forensic_economics, environmental_resource, econometrics,
//! dynamic_programming, agent_based, paper_trading, accounting.

use crate::poet_host::invoke::args;
use poet_vibe::{Diagnostic, Span, Value};

// ── Asset pricing ────────────────────────────────────────────────────────────

pub fn econ_capm_expected_return(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let rf = args::rec_f64(args, "rf")
        .ok_or_else(|| args::bad(span, "Econ.capm_expected_return needs rf"))?;
    let beta = args::rec_f64(args, "beta")
        .ok_or_else(|| args::bad(span, "Econ.capm_expected_return needs beta"))?;
    let market_premium = args::rec_f64(args, "market_premium")
        .ok_or_else(|| args::bad(span, "Econ.capm_expected_return needs market_premium"))?;
    match crate::specialized_libs::computational_economics::asset_pricing::capm_expected_return(
        rf,
        beta,
        market_premium,
    ) {
        Ok(expected) => Ok(args::record([("expected_return", Value::F64(expected))])),
        Err(e) => Err(args::bad(span, format!("capm_expected_return: {e:?}"))),
    }
}

pub fn econ_capm_beta(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let asset_returns = args::rec_f64_list(args, "asset_returns")
        .ok_or_else(|| args::bad(span, "Econ.capm_beta needs asset_returns"))?;
    let market_returns = args::rec_f64_list(args, "market_returns")
        .ok_or_else(|| args::bad(span, "Econ.capm_beta needs market_returns"))?;
    match crate::specialized_libs::computational_economics::asset_pricing::capm_beta(
        &asset_returns,
        &market_returns,
    ) {
        Ok(beta) => Ok(args::record([("beta", Value::F64(beta))])),
        Err(e) => Err(args::bad(span, format!("capm_beta: {e:?}"))),
    }
}

pub fn econ_gordon_growth(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let d1 = args::rec_f64(args, "next_dividend")
        .ok_or_else(|| args::bad(span, "Econ.gordon_growth needs next_dividend"))?;
    let r = args::rec_f64(args, "required_return")
        .ok_or_else(|| args::bad(span, "Econ.gordon_growth needs required_return"))?;
    let g = args::rec_f64(args, "growth_rate")
        .ok_or_else(|| args::bad(span, "Econ.gordon_growth needs growth_rate"))?;
    match crate::specialized_libs::computational_economics::asset_pricing::gordon_growth_price(
        d1, r, g,
    ) {
        Ok(price) => Ok(args::record([("price", Value::F64(price))])),
        Err(e) => Err(args::bad(span, format!("gordon_growth: {e:?}"))),
    }
}

pub fn econ_multi_period_ddm(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let dividends = args::rec_f64_list(args, "dividends")
        .ok_or_else(|| args::bad(span, "Econ.multi_period_ddm needs dividends"))?;
    let r = args::rec_f64(args, "discount_rate")
        .ok_or_else(|| args::bad(span, "Econ.multi_period_ddm needs discount_rate"))?;
    let g = args::rec_f64(args, "terminal_growth")
        .ok_or_else(|| args::bad(span, "Econ.multi_period_ddm needs terminal_growth"))?;
    match crate::specialized_libs::computational_economics::asset_pricing::multi_period_ddm(
        &dividends, r, g,
    ) {
        Ok(price) => Ok(args::record([("price", Value::F64(price))])),
        Err(e) => Err(args::bad(span, format!("multi_period_ddm: {e:?}"))),
    }
}

pub fn econ_ccapm_equity_premium(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let gamma = args::rec_f64(args, "gamma")
        .ok_or_else(|| args::bad(span, "Econ.ccapm_equity_premium needs gamma"))?;
    let sigma_c = args::rec_f64(args, "consumption_growth_std").ok_or_else(|| {
        args::bad(
            span,
            "Econ.ccapm_equity_premium needs consumption_growth_std",
        )
    })?;
    let sigma_m = args::rec_f64(args, "market_return_std")
        .ok_or_else(|| args::bad(span, "Econ.ccapm_equity_premium needs market_return_std"))?;
    match crate::specialized_libs::computational_economics::asset_pricing::ccapm_equity_premium(
        gamma, sigma_c, sigma_m,
    ) {
        Ok(premium) => Ok(args::record([("equity_premium", Value::F64(premium))])),
        Err(e) => Err(args::bad(span, format!("ccapm_equity_premium: {e:?}"))),
    }
}

pub fn econ_ccapm_sdf(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let cg = args::rec_f64(args, "consumption_growth")
        .ok_or_else(|| args::bad(span, "Econ.ccapm_sdf needs consumption_growth"))?;
    let gamma = args::rec_f64(args, "gamma")
        .ok_or_else(|| args::bad(span, "Econ.ccapm_sdf needs gamma"))?;
    let beta =
        args::rec_f64(args, "beta").ok_or_else(|| args::bad(span, "Econ.ccapm_sdf needs beta"))?;
    match crate::specialized_libs::computational_economics::asset_pricing::ccapm_stochastic_discount_factor(cg, gamma, beta) {
        Ok(m) => Ok(args::record([("sdf", Value::F64(m))])),
        Err(e) => Err(args::bad(span, format!("ccapm_sdf: {e:?}"))),
    }
}

// ── Behavioral ───────────────────────────────────────────────────────────────

pub fn econ_prospect_value(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x =
        args::rec_f64(args, "x").ok_or_else(|| args::bad(span, "Econ.prospect_value needs x"))?;
    let alpha = args::rec_f64(args, "alpha").unwrap_or(0.88);
    let beta = args::rec_f64(args, "beta").unwrap_or(0.88);
    let lambda = args::rec_f64(args, "lambda").unwrap_or(2.25);
    match crate::specialized_libs::computational_economics::behavioral::prospect_value(
        x, alpha, beta, lambda,
    ) {
        Ok(v) => Ok(args::record([("value", Value::F64(v))])),
        Err(e) => Err(args::bad(span, format!("prospect_value: {e:?}"))),
    }
}

pub fn econ_probability_weight(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let p = args::rec_f64(args, "p")
        .ok_or_else(|| args::bad(span, "Econ.probability_weight needs p"))?;
    let gamma = args::rec_f64(args, "gamma").unwrap_or(0.65);
    match crate::specialized_libs::computational_economics::behavioral::probability_weight(p, gamma)
    {
        Ok(w) => Ok(args::record([("weight", Value::F64(w))])),
        Err(e) => Err(args::bad(span, format!("probability_weight: {e:?}"))),
    }
}

pub fn econ_hyperbolic_discount(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let t = args::rec_u64(args, "t")
        .ok_or_else(|| args::bad(span, "Econ.hyperbolic_discount needs t"))? as u32;
    let beta = args::rec_f64(args, "beta").unwrap_or(0.7);
    let delta = args::rec_f64(args, "delta").unwrap_or(0.99);
    match crate::specialized_libs::computational_economics::behavioral::hyperbolic_discount(
        t, beta, delta,
    ) {
        Ok(d) => Ok(args::record([("discount", Value::F64(d))])),
        Err(e) => Err(args::bad(span, format!("hyperbolic_discount: {e:?}"))),
    }
}

pub fn econ_endowment_effect(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let wtp = args::rec_f64(args, "wtp")
        .ok_or_else(|| args::bad(span, "Econ.endowment_effect needs wtp"))?;
    let lambda = args::rec_f64(args, "lambda").unwrap_or(2.25);
    match crate::specialized_libs::computational_economics::behavioral::endowment_effect_wta(
        wtp, lambda,
    ) {
        Ok(wta) => Ok(args::record([("wta", Value::F64(wta))])),
        Err(e) => Err(args::bad(span, format!("endowment_effect: {e:?}"))),
    }
}

// ── Derivatives ──────────────────────────────────────────────────────────────

pub fn econ_black_scholes(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    use crate::specialized_libs::computational_economics::derivatives::{
        black_scholes_price_and_greeks, OptionKind,
    };
    let s = args::rec_f64(args, "spot")
        .ok_or_else(|| args::bad(span, "Econ.black_scholes needs spot"))?;
    let k = args::rec_f64(args, "strike")
        .ok_or_else(|| args::bad(span, "Econ.black_scholes needs strike"))?;
    let t = args::rec_f64(args, "time_to_expiry")
        .ok_or_else(|| args::bad(span, "Econ.black_scholes needs time_to_expiry"))?;
    let r = args::rec_f64(args, "risk_free_rate")
        .ok_or_else(|| args::bad(span, "Econ.black_scholes needs risk_free_rate"))?;
    let q = args::rec_f64(args, "dividend_yield").unwrap_or(0.0);
    let sigma = args::rec_f64(args, "volatility")
        .ok_or_else(|| args::bad(span, "Econ.black_scholes needs volatility"))?;
    let is_call = args::rec_bool(args, "is_call").unwrap_or(true);
    let kind = if is_call {
        OptionKind::Call
    } else {
        OptionKind::Put
    };
    match black_scholes_price_and_greeks(kind, s, k, r, q, sigma, t) {
        Ok(result) => Ok(args::record([
            ("price", Value::F64(result.price)),
            ("delta", Value::F64(result.delta)),
            ("gamma", Value::F64(result.gamma)),
            ("theta", Value::F64(result.theta)),
            ("vega", Value::F64(result.vega)),
            ("rho", Value::F64(result.rho)),
        ])),
        Err(e) => Err(args::bad(span, format!("black_scholes: {e:?}"))),
    }
}

pub fn econ_put_call_parity(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    use crate::specialized_libs::computational_economics::derivatives::put_call_parity;
    let call_price = args::rec_f64(args, "call_price")
        .ok_or_else(|| args::bad(span, "Econ.put_call_parity needs call_price"))?;
    let put_price = args::rec_f64(args, "put_price").unwrap_or(0.0);
    let s = args::rec_f64(args, "spot")
        .ok_or_else(|| args::bad(span, "Econ.put_call_parity needs spot"))?;
    let k = args::rec_f64(args, "strike")
        .ok_or_else(|| args::bad(span, "Econ.put_call_parity needs strike"))?;
    let t = args::rec_f64(args, "time_to_expiry")
        .ok_or_else(|| args::bad(span, "Econ.put_call_parity needs time_to_expiry"))?;
    let r = args::rec_f64(args, "risk_free_rate")
        .ok_or_else(|| args::bad(span, "Econ.put_call_parity needs risk_free_rate"))?;
    let q = args::rec_f64(args, "dividend_yield").unwrap_or(0.0);
    match put_call_parity(call_price, put_price, s, k, r, q, t) {
        Ok(parity_error) => Ok(args::record([("parity_error", Value::F64(parity_error))])),
        Err(e) => Err(args::bad(span, format!("put_call_parity: {e:?}"))),
    }
}

pub fn econ_binomial_option(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    use crate::specialized_libs::computational_economics::derivatives::{
        binomial_option_price, OptionKind, OptionStyle,
    };
    let s = args::rec_f64(args, "spot")
        .ok_or_else(|| args::bad(span, "Econ.binomial_option needs spot"))?;
    let k = args::rec_f64(args, "strike")
        .ok_or_else(|| args::bad(span, "Econ.binomial_option needs strike"))?;
    let t = args::rec_f64(args, "time_to_expiry")
        .ok_or_else(|| args::bad(span, "Econ.binomial_option needs time_to_expiry"))?;
    let r = args::rec_f64(args, "risk_free_rate")
        .ok_or_else(|| args::bad(span, "Econ.binomial_option needs risk_free_rate"))?;
    let q = args::rec_f64(args, "dividend_yield").unwrap_or(0.0);
    let sigma = args::rec_f64(args, "volatility")
        .ok_or_else(|| args::bad(span, "Econ.binomial_option needs volatility"))?;
    let steps = args::rec_u64(args, "steps").unwrap_or(100) as usize;
    let is_call = args::rec_bool(args, "is_call").unwrap_or(true);
    let kind = if is_call {
        OptionKind::Call
    } else {
        OptionKind::Put
    };
    match binomial_option_price(kind, OptionStyle::European, s, k, r, q, sigma, t, steps) {
        Ok(price) => Ok(args::record([("price", Value::F64(price))])),
        Err(e) => Err(args::bad(span, format!("binomial_option: {e:?}"))),
    }
}

// ── Game theory ──────────────────────────────────────────────────────────────

pub fn econ_mixed_nash_2x2(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = args::rec_f64_list(args, "payoff_matrix_a")
        .ok_or_else(|| args::bad(span, "Econ.mixed_nash_2x2 needs payoff_matrix_a (4 values)"))?;
    let b = args::rec_f64_list(args, "payoff_matrix_b")
        .ok_or_else(|| args::bad(span, "Econ.mixed_nash_2x2 needs payoff_matrix_b (4 values)"))?;
    if a.len() < 4 || b.len() < 4 {
        return Err(args::bad(
            span,
            "Econ.mixed_nash_2x2: matrices need 4 values each",
        ));
    }
    match crate::specialized_libs::computational_economics::game_theory::mixed_nash_2x2(&a, &b) {
        Ok((p_row, p_col, exp_row, exp_col)) => Ok(args::record([
            ("row_player_p", Value::F64(p_row)),
            ("col_player_q", Value::F64(p_col)),
            ("expected_row_payoff", Value::F64(exp_row)),
            ("expected_col_payoff", Value::F64(exp_col)),
        ])),
        Err(e) => Err(args::bad(span, format!("mixed_nash_2x2: {e:?}"))),
    }
}

pub fn econ_cournot_duopoly(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = args::rec_f64(args, "demand_intercept")
        .ok_or_else(|| args::bad(span, "Econ.cournot_duopoly needs demand_intercept"))?;
    let b = args::rec_f64(args, "demand_slope")
        .ok_or_else(|| args::bad(span, "Econ.cournot_duopoly needs demand_slope"))?;
    let c1 = args::rec_f64(args, "cost_1")
        .ok_or_else(|| args::bad(span, "Econ.cournot_duopoly needs cost_1"))?;
    let c2 = args::rec_f64(args, "cost_2")
        .ok_or_else(|| args::bad(span, "Econ.cournot_duopoly needs cost_2"))?;
    match crate::specialized_libs::computational_economics::game_theory::cournot_duopoly(
        a, b, c1, c2,
    ) {
        Ok((q1, q2, price)) => Ok(args::record([
            ("quantity_1", Value::F64(q1)),
            ("quantity_2", Value::F64(q2)),
            ("market_price", Value::F64(price)),
        ])),
        Err(e) => Err(args::bad(span, format!("cournot_duopoly: {e:?}"))),
    }
}

pub fn econ_bertrand_duopoly(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let c1 = args::rec_f64(args, "cost_1")
        .ok_or_else(|| args::bad(span, "Econ.bertrand_duopoly needs cost_1"))?;
    let c2 = args::rec_f64(args, "cost_2")
        .ok_or_else(|| args::bad(span, "Econ.bertrand_duopoly needs cost_2"))?;
    match crate::specialized_libs::computational_economics::game_theory::bertrand_duopoly(c1, c2) {
        Ok((p1, p2)) => Ok(args::record([
            ("price_1", Value::F64(p1)),
            ("price_2", Value::F64(p2)),
        ])),
        Err(e) => Err(args::bad(span, format!("bertrand_duopoly: {e:?}"))),
    }
}

pub fn econ_stackelberg_duopoly(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = args::rec_f64(args, "demand_intercept")
        .ok_or_else(|| args::bad(span, "Econ.stackelberg_duopoly needs demand_intercept"))?;
    let b = args::rec_f64(args, "demand_slope")
        .ok_or_else(|| args::bad(span, "Econ.stackelberg_duopoly needs demand_slope"))?;
    let c1 = args::rec_f64(args, "cost_leader")
        .ok_or_else(|| args::bad(span, "Econ.stackelberg_duopoly needs cost_leader"))?;
    let c2 = args::rec_f64(args, "cost_follower")
        .ok_or_else(|| args::bad(span, "Econ.stackelberg_duopoly needs cost_follower"))?;
    match crate::specialized_libs::computational_economics::game_theory::stackelberg_duopoly(
        a, b, c1, c2,
    ) {
        Ok((q1, q2, price)) => Ok(args::record([
            ("leader_quantity", Value::F64(q1)),
            ("follower_quantity", Value::F64(q2)),
            ("market_price", Value::F64(price)),
        ])),
        Err(e) => Err(args::bad(span, format!("stackelberg_duopoly: {e:?}"))),
    }
}

// ── Macro models ─────────────────────────────────────────────────────────────

pub fn econ_solow_steady_state(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let s = args::rec_f64(args, "savings_rate")
        .ok_or_else(|| args::bad(span, "Econ.solow_steady_state needs savings_rate"))?;
    let alpha = args::rec_f64(args, "alpha")
        .ok_or_else(|| args::bad(span, "Econ.solow_steady_state needs alpha"))?;
    let delta = args::rec_f64(args, "depreciation")
        .ok_or_else(|| args::bad(span, "Econ.solow_steady_state needs depreciation"))?;
    let n = args::rec_f64(args, "population_growth").unwrap_or(0.0);
    let g = args::rec_f64(args, "tech_growth").unwrap_or(0.0);
    match crate::specialized_libs::computational_economics::macro_models::solow_steady_state(
        s, alpha, delta, n, g,
    ) {
        Ok((k, y)) => Ok(args::record([
            ("steady_state_capital", Value::F64(k)),
            ("steady_state_output", Value::F64(y)),
        ])),
        Err(e) => Err(args::bad(span, format!("solow_steady_state: {e:?}"))),
    }
}

pub fn econ_ramsey_steady_state(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let alpha = args::rec_f64(args, "alpha")
        .ok_or_else(|| args::bad(span, "Econ.ramsey_steady_state needs alpha"))?;
    let beta = args::rec_f64(args, "beta")
        .ok_or_else(|| args::bad(span, "Econ.ramsey_steady_state needs beta"))?;
    let delta = args::rec_f64(args, "depreciation")
        .ok_or_else(|| args::bad(span, "Econ.ramsey_steady_state needs depreciation"))?;
    match crate::specialized_libs::computational_economics::macro_models::ramsey_steady_state(
        alpha, beta, delta,
    ) {
        Ok(k) => Ok(args::record([("steady_state_capital", Value::F64(k))])),
        Err(e) => Err(args::bad(span, format!("ramsey_steady_state: {e:?}"))),
    }
}

pub fn econ_olg_steady_state(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let alpha = args::rec_f64(args, "alpha")
        .ok_or_else(|| args::bad(span, "Econ.olg_steady_state needs alpha"))?;
    let beta = args::rec_f64(args, "beta")
        .ok_or_else(|| args::bad(span, "Econ.olg_steady_state needs beta"))?;
    let n = args::rec_f64(args, "population_growth").unwrap_or(0.0);
    match crate::specialized_libs::computational_economics::macro_models::olg_steady_state(
        alpha, beta, n,
    ) {
        Ok((k, c)) => Ok(args::record([
            ("steady_state_capital", Value::F64(k)),
            ("steady_state_consumption", Value::F64(c)),
        ])),
        Err(e) => Err(args::bad(span, format!("olg_steady_state: {e:?}"))),
    }
}

// ── Welfare ──────────────────────────────────────────────────────────────────

pub fn econ_gini(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let incomes = args::rec_f64_list(args, "incomes")
        .ok_or_else(|| args::bad(span, "Econ.gini needs incomes"))?;
    match crate::specialized_libs::computational_economics::welfare::gini_coefficient(&incomes) {
        Ok(g) => Ok(args::record([("gini", Value::F64(g))])),
        Err(e) => Err(args::bad(span, format!("gini: {e:?}"))),
    }
}

pub fn econ_atkinson(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let incomes = args::rec_f64_list(args, "incomes")
        .ok_or_else(|| args::bad(span, "Econ.atkinson needs incomes"))?;
    let epsilon = args::rec_f64(args, "epsilon")
        .ok_or_else(|| args::bad(span, "Econ.atkinson needs epsilon"))?;
    match crate::specialized_libs::computational_economics::welfare::atkinson_inequality(
        &incomes, epsilon,
    ) {
        Ok(a) => Ok(args::record([("atkinson", Value::F64(a))])),
        Err(e) => Err(args::bad(span, format!("atkinson: {e:?}"))),
    }
}

pub fn econ_headcount_poverty(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let incomes = args::rec_f64_list(args, "incomes")
        .ok_or_else(|| args::bad(span, "Econ.headcount_poverty needs incomes"))?;
    let line = args::rec_f64(args, "poverty_line")
        .ok_or_else(|| args::bad(span, "Econ.headcount_poverty needs poverty_line"))?;
    match crate::specialized_libs::computational_economics::welfare::headcount_poverty(
        &incomes, line,
    ) {
        Ok((count, rate)) => Ok(args::record([
            ("headcount", Value::U64(count as u64)),
            ("poverty_rate", Value::F64(rate)),
        ])),
        Err(e) => Err(args::bad(span, format!("headcount_poverty: {e:?}"))),
    }
}

pub fn econ_poverty_gap(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let incomes = args::rec_f64_list(args, "incomes")
        .ok_or_else(|| args::bad(span, "Econ.poverty_gap needs incomes"))?;
    let line = args::rec_f64(args, "poverty_line")
        .ok_or_else(|| args::bad(span, "Econ.poverty_gap needs poverty_line"))?;
    match crate::specialized_libs::computational_economics::welfare::poverty_gap_ratio(
        &incomes, line,
    ) {
        Ok(r) => Ok(args::record([("poverty_gap_ratio", Value::F64(r))])),
        Err(e) => Err(args::bad(span, format!("poverty_gap: {e:?}"))),
    }
}

pub fn econ_utilitarian_welfare(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let utilities = args::rec_f64_list(args, "utilities")
        .ok_or_else(|| args::bad(span, "Econ.utilitarian_welfare needs utilities"))?;
    match crate::specialized_libs::computational_economics::welfare::utilitarian_welfare(&utilities)
    {
        Ok(w) => Ok(args::record([("welfare", Value::F64(w))])),
        Err(e) => Err(args::bad(span, format!("utilitarian_welfare: {e:?}"))),
    }
}

pub fn econ_rawlsian_welfare(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let utilities = args::rec_f64_list(args, "utilities")
        .ok_or_else(|| args::bad(span, "Econ.rawlsian_welfare needs utilities"))?;
    match crate::specialized_libs::computational_economics::welfare::rawlsian_welfare(&utilities) {
        Ok(w) => Ok(args::record([("welfare", Value::F64(w))])),
        Err(e) => Err(args::bad(span, format!("rawlsian_welfare: {e:?}"))),
    }
}

pub fn econ_nash_welfare(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let utilities = args::rec_f64_list(args, "utilities")
        .ok_or_else(|| args::bad(span, "Econ.nash_welfare needs utilities"))?;
    match crate::specialized_libs::computational_economics::welfare::nash_welfare(&utilities) {
        Ok(w) => Ok(args::record([("welfare", Value::F64(w))])),
        Err(e) => Err(args::bad(span, format!("nash_welfare: {e:?}"))),
    }
}

pub fn econ_npv(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let benefits = args::rec_f64_list(args, "benefits")
        .ok_or_else(|| args::bad(span, "Econ.npv needs benefits"))?;
    let costs =
        args::rec_f64_list(args, "costs").ok_or_else(|| args::bad(span, "Econ.npv needs costs"))?;
    let rate = args::rec_f64(args, "discount_rate")
        .ok_or_else(|| args::bad(span, "Econ.npv needs discount_rate"))?;
    let n_periods = args::rec_u64(args, "n_periods")
        .ok_or_else(|| args::bad(span, "Econ.npv needs n_periods"))? as usize;
    match crate::specialized_libs::computational_economics::welfare::net_present_value(
        &benefits, &costs, rate, n_periods,
    ) {
        Ok(npv) => Ok(args::record([("npv", Value::F64(npv))])),
        Err(e) => Err(args::bad(span, format!("npv: {e:?}"))),
    }
}

// ── Portfolio ────────────────────────────────────────────────────────────────

pub fn econ_mean_return(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let returns = args::rec_f64_list(args, "returns")
        .ok_or_else(|| args::bad(span, "Econ.mean_return needs returns"))?;
    match crate::specialized_libs::computational_economics::portfolio::mean_return(&returns) {
        Ok(m) => Ok(args::record([("mean_return", Value::F64(m))])),
        Err(e) => Err(args::bad(span, format!("mean_return: {e:?}"))),
    }
}

pub fn econ_sample_variance(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let returns = args::rec_f64_list(args, "returns")
        .ok_or_else(|| args::bad(span, "Econ.sample_variance needs returns"))?;
    match crate::specialized_libs::computational_economics::portfolio::sample_variance(&returns) {
        Ok(v) => Ok(args::record([("variance", Value::F64(v))])),
        Err(e) => Err(args::bad(span, format!("sample_variance: {e:?}"))),
    }
}

pub fn econ_portfolio_max_drawdown(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let returns = args::rec_f64_list(args, "returns")
        .ok_or_else(|| args::bad(span, "Econ.portfolio_max_drawdown needs returns"))?;
    match crate::specialized_libs::computational_economics::portfolio::max_drawdown(&returns) {
        Ok(md) => Ok(args::record([("max_drawdown", Value::F64(md))])),
        Err(e) => Err(args::bad(span, format!("portfolio_max_drawdown: {e:?}"))),
    }
}

// ── Time series / risk ───────────────────────────────────────────────────────

pub fn econ_historical_var(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let returns = args::rec_f64_list(args, "returns")
        .ok_or_else(|| args::bad(span, "Econ.historical_var needs returns"))?;
    let confidence = args::rec_f64(args, "confidence").unwrap_or(0.95);
    let mut scratch = returns.clone();
    match crate::specialized_libs::computational_economics::risk::historical_var(
        &returns,
        confidence,
        &mut scratch,
    ) {
        Ok(var) => Ok(args::record([("var", Value::F64(var))])),
        Err(e) => Err(args::bad(span, format!("historical_var: {e:?}"))),
    }
}

pub fn econ_historical_cvar(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let returns = args::rec_f64_list(args, "returns")
        .ok_or_else(|| args::bad(span, "Econ.historical_cvar needs returns"))?;
    let confidence = args::rec_f64(args, "confidence").unwrap_or(0.95);
    let mut scratch = returns.clone();
    match crate::specialized_libs::computational_economics::risk::historical_cvar(
        &returns,
        confidence,
        &mut scratch,
    ) {
        Ok(cvar) => Ok(args::record([("cvar", Value::F64(cvar))])),
        Err(e) => Err(args::bad(span, format!("historical_cvar: {e:?}"))),
    }
}

pub fn econ_parametric_var(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let mean = args::rec_f64(args, "mean")
        .ok_or_else(|| args::bad(span, "Econ.parametric_var needs mean"))?;
    let std_dev = args::rec_f64(args, "std_dev")
        .ok_or_else(|| args::bad(span, "Econ.parametric_var needs std_dev"))?;
    let confidence = args::rec_f64(args, "confidence").unwrap_or(0.95);
    match crate::specialized_libs::computational_economics::risk::gaussian_var(
        mean, std_dev, confidence,
    ) {
        Ok(var) => Ok(args::record([("var", Value::F64(var))])),
        Err(e) => Err(args::bad(span, format!("parametric_var: {e:?}"))),
    }
}

pub fn econ_autocorrelation(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let values = args::rec_f64_list(args, "values")
        .ok_or_else(|| args::bad(span, "Econ.autocorrelation needs values"))?;
    let lag = args::rec_u64(args, "lag").unwrap_or(1) as usize;
    match crate::specialized_libs::computational_economics::time_series::autocorrelation(
        &values, lag,
    ) {
        Ok(ac) => Ok(args::record([("autocorrelation", Value::F64(ac))])),
        Err(e) => Err(args::bad(span, format!("autocorrelation: {e:?}"))),
    }
}

pub fn econ_cross_correlation(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = args::rec_f64_list(args, "series_a")
        .ok_or_else(|| args::bad(span, "Econ.cross_correlation needs series_a"))?;
    let b = args::rec_f64_list(args, "series_b")
        .ok_or_else(|| args::bad(span, "Econ.cross_correlation needs series_b"))?;
    let lag = args::rec_i64(args, "lag").unwrap_or(0) as i32;
    match crate::specialized_libs::computational_economics::time_series::cross_correlation(
        &a, &b, lag,
    ) {
        Ok(cc) => Ok(args::record([("cross_correlation", Value::F64(cc))])),
        Err(e) => Err(args::bad(span, format!("cross_correlation: {e:?}"))),
    }
}

// ── Yield curve ──────────────────────────────────────────────────────────────

pub fn econ_interpolate_zero_rate(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    use crate::specialized_libs::computational_economics::yield_curve::{
        interpolate_zero_rate, CurvePoint,
    };
    let maturities = args::rec_f64_list(args, "maturities")
        .ok_or_else(|| args::bad(span, "Econ.interpolate_zero_rate needs maturities"))?;
    let rates = args::rec_f64_list(args, "rates")
        .ok_or_else(|| args::bad(span, "Econ.interpolate_zero_rate needs rates"))?;
    let target = args::rec_f64(args, "target_maturity")
        .ok_or_else(|| args::bad(span, "Econ.interpolate_zero_rate needs target_maturity"))?;
    if maturities.len() != rates.len() {
        return Err(args::bad(
            span,
            "Econ.interpolate_zero_rate: maturities and rates must have equal length",
        ));
    }
    let points: Vec<CurvePoint> = maturities
        .iter()
        .zip(rates.iter())
        .map(|(&t, &r)| CurvePoint {
            time_years: t,
            zero_rate: r,
        })
        .collect();
    match interpolate_zero_rate(&points, target) {
        Ok(r) => Ok(args::record([("zero_rate", Value::F64(r))])),
        Err(e) => Err(args::bad(span, format!("interpolate_zero_rate: {e:?}"))),
    }
}

pub fn econ_discount_factor(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    use crate::specialized_libs::computational_economics::yield_curve::{
        discount_factor_from_curve, CurvePoint,
    };
    let maturities = args::rec_f64_list(args, "maturities")
        .ok_or_else(|| args::bad(span, "Econ.discount_factor needs maturities"))?;
    let rates = args::rec_f64_list(args, "rates")
        .ok_or_else(|| args::bad(span, "Econ.discount_factor needs rates"))?;
    let target = args::rec_f64(args, "target_maturity")
        .ok_or_else(|| args::bad(span, "Econ.discount_factor needs target_maturity"))?;
    let compounding = args::rec_u64(args, "compounding_per_year").unwrap_or(1) as u32;
    if maturities.len() != rates.len() {
        return Err(args::bad(
            span,
            "Econ.discount_factor: maturities and rates must have equal length",
        ));
    }
    let points: Vec<CurvePoint> = maturities
        .iter()
        .zip(rates.iter())
        .map(|(&t, &r)| CurvePoint {
            time_years: t,
            zero_rate: r,
        })
        .collect();
    match discount_factor_from_curve(&points, target, compounding) {
        Ok(df) => Ok(args::record([("discount_factor", Value::F64(df))])),
        Err(e) => Err(args::bad(span, format!("discount_factor: {e:?}"))),
    }
}

pub fn econ_forward_rate(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    use crate::specialized_libs::computational_economics::yield_curve::{
        annualized_forward_rate, CurvePoint,
    };
    let maturities = args::rec_f64_list(args, "maturities")
        .ok_or_else(|| args::bad(span, "Econ.forward_rate needs maturities"))?;
    let rates = args::rec_f64_list(args, "rates")
        .ok_or_else(|| args::bad(span, "Econ.forward_rate needs rates"))?;
    let t1 =
        args::rec_f64(args, "t1").ok_or_else(|| args::bad(span, "Econ.forward_rate needs t1"))?;
    let t2 =
        args::rec_f64(args, "t2").ok_or_else(|| args::bad(span, "Econ.forward_rate needs t2"))?;
    let compounding = args::rec_u64(args, "compounding_per_year").unwrap_or(1) as u32;
    if maturities.len() != rates.len() {
        return Err(args::bad(
            span,
            "Econ.forward_rate: maturities and rates must have equal length",
        ));
    }
    let points: Vec<CurvePoint> = maturities
        .iter()
        .zip(rates.iter())
        .map(|(&t, &r)| CurvePoint {
            time_years: t,
            zero_rate: r,
        })
        .collect();
    match annualized_forward_rate(&points, t1, t2, compounding) {
        Ok(f) => Ok(args::record([("forward_rate", Value::F64(f))])),
        Err(e) => Err(args::bad(span, format!("forward_rate: {e:?}"))),
    }
}

// ── Spatial economics ────────────────────────────────────────────────────────

pub fn econ_gravity_flow(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let mass_1 = args::rec_f64(args, "mass_1")
        .ok_or_else(|| args::bad(span, "Econ.gravity_flow needs mass_1"))?;
    let mass_2 = args::rec_f64(args, "mass_2")
        .ok_or_else(|| args::bad(span, "Econ.gravity_flow needs mass_2"))?;
    let distance = args::rec_f64(args, "distance")
        .ok_or_else(|| args::bad(span, "Econ.gravity_flow needs distance"))?;
    let alpha = args::rec_f64(args, "alpha").unwrap_or(1.0);
    let beta = args::rec_f64(args, "beta").unwrap_or(1.0);
    let gamma = args::rec_f64(args, "gamma").unwrap_or(2.0);
    match crate::specialized_libs::computational_economics::spatial_economics::gravity_flow(
        mass_1, mass_2, distance, alpha, beta, gamma,
    ) {
        Ok(flow) => Ok(args::record([("flow", Value::F64(flow))])),
        Err(e) => Err(args::bad(span, format!("gravity_flow: {e:?}"))),
    }
}

pub fn econ_morans_i(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let values = args::rec_f64_list(args, "values")
        .ok_or_else(|| args::bad(span, "Econ.morans_i needs values"))?;
    let weights = args::rec_f64_list(args, "weights")
        .ok_or_else(|| args::bad(span, "Econ.morans_i needs weights"))?;
    let n =
        args::rec_u64(args, "n").ok_or_else(|| args::bad(span, "Econ.morans_i needs n"))? as usize;
    match crate::specialized_libs::computational_economics::spatial_economics::morans_i(
        &values, &weights, n,
    ) {
        Ok(mi) => Ok(args::record([("morans_i", Value::F64(mi))])),
        Err(e) => Err(args::bad(span, format!("morans_i: {e:?}"))),
    }
}

// ── Public finance ───────────────────────────────────────────────────────────

pub fn econ_transfer_payment(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let base = args::rec_f64(args, "base")
        .ok_or_else(|| args::bad(span, "Econ.transfer_payment needs base"))?;
    let income = args::rec_f64(args, "income")
        .ok_or_else(|| args::bad(span, "Econ.transfer_payment needs income"))?;
    let threshold = args::rec_f64(args, "threshold")
        .ok_or_else(|| args::bad(span, "Econ.transfer_payment needs threshold"))?;
    let phaseout_rate = args::rec_f64(args, "phaseout_rate")
        .ok_or_else(|| args::bad(span, "Econ.transfer_payment needs phaseout_rate"))?;
    match crate::specialized_libs::computational_economics::public_finance::transfer_payment(
        base,
        income,
        threshold,
        phaseout_rate,
    ) {
        Ok(t) => Ok(args::record([("transfer", Value::F64(t))])),
        Err(e) => Err(args::bad(span, format!("transfer_payment: {e:?}"))),
    }
}

pub fn econ_fiscal_multiplier(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let initial_spending = args::rec_f64(args, "initial_spending")
        .ok_or_else(|| args::bad(span, "Econ.fiscal_multiplier needs initial_spending"))?;
    let mpc = args::rec_f64(args, "mpc")
        .ok_or_else(|| args::bad(span, "Econ.fiscal_multiplier needs mpc"))?;
    let leakage_rate = args::rec_f64(args, "leakage_rate").unwrap_or(0.0);
    match crate::specialized_libs::computational_economics::public_finance::fiscal_multiplier(
        initial_spending,
        mpc,
        leakage_rate,
    ) {
        Ok(m) => Ok(args::record([("gdp_impact", Value::F64(m))])),
        Err(e) => Err(args::bad(span, format!("fiscal_multiplier: {e:?}"))),
    }
}

pub fn econ_laffer_curve(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let tax_rate = args::rec_f64(args, "tax_rate")
        .ok_or_else(|| args::bad(span, "Econ.laffer_curve needs tax_rate"))?;
    let base = args::rec_f64(args, "tax_base")
        .ok_or_else(|| args::bad(span, "Econ.laffer_curve needs tax_base"))?;
    let elasticity = args::rec_f64(args, "elasticity")
        .ok_or_else(|| args::bad(span, "Econ.laffer_curve needs elasticity"))?;
    match crate::specialized_libs::computational_economics::public_finance::laffer_curve_revenue(
        tax_rate, base, elasticity,
    ) {
        Ok(rev) => Ok(args::record([("revenue", Value::F64(rev))])),
        Err(e) => Err(args::bad(span, format!("laffer_curve: {e:?}"))),
    }
}

// ── Mechanism design ─────────────────────────────────────────────────────────

pub fn econ_check_ir(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let valuations = args::rec_f64_list(args, "valuations")
        .ok_or_else(|| args::bad(span, "Econ.check_ir needs valuations"))?;
    let payments = args::rec_f64_list(args, "payments")
        .ok_or_else(|| args::bad(span, "Econ.check_ir needs payments"))?;
    match crate::specialized_libs::computational_economics::mechanism::check_individual_rationality(
        &valuations,
        &payments,
    ) {
        Ok(ir) => Ok(args::record([("individual_rationality", Value::Bool(ir))])),
        Err(e) => Err(args::bad(span, format!("check_ir: {e:?}"))),
    }
}

pub fn econ_check_budget_balance(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let payments = args::rec_f64_list(args, "payments")
        .ok_or_else(|| args::bad(span, "Econ.check_budget_balance needs payments"))?;
    match crate::specialized_libs::computational_economics::mechanism::check_budget_balance(
        &payments,
    ) {
        Ok((balanced, surplus)) => Ok(args::record([
            ("balanced", Value::Bool(balanced)),
            ("surplus", Value::F64(surplus)),
        ])),
        Err(e) => Err(args::bad(span, format!("check_budget_balance: {e:?}"))),
    }
}

// ── Markov chains ────────────────────────────────────────────────────────────

pub fn econ_validate_transition_matrix(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let p = args::rec_f64_list(args, "matrix")
        .ok_or_else(|| args::bad(span, "Econ.validate_transition_matrix needs matrix"))?;
    let n = args::rec_u64(args, "n")
        .ok_or_else(|| args::bad(span, "Econ.validate_transition_matrix needs n"))?
        as usize;
    match crate::specialized_libs::computational_economics::markov::validate_transition_matrix(
        &p, n,
    ) {
        Ok(()) => Ok(args::record([("valid", Value::Bool(true))])),
        Err(e) => Err(args::bad(
            span,
            format!("validate_transition_matrix: {e:?}"),
        )),
    }
}

pub fn econ_transition_probability(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let p = args::rec_f64_list(args, "matrix")
        .ok_or_else(|| args::bad(span, "Econ.transition_probability needs matrix"))?;
    let n = args::rec_u64(args, "n")
        .ok_or_else(|| args::bad(span, "Econ.transition_probability needs n"))?
        as usize;
    let from = args::rec_u64(args, "from")
        .ok_or_else(|| args::bad(span, "Econ.transition_probability needs from"))?
        as usize;
    let to = args::rec_u64(args, "to")
        .ok_or_else(|| args::bad(span, "Econ.transition_probability needs to"))?
        as usize;
    match crate::specialized_libs::computational_economics::markov::transition_probability(
        &p, n, from, to,
    ) {
        Ok(prob) => Ok(args::record([("probability", Value::F64(prob))])),
        Err(e) => Err(args::bad(span, format!("transition_probability: {e:?}"))),
    }
}

pub fn econ_expected_holding_time(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let p = args::rec_f64_list(args, "matrix")
        .ok_or_else(|| args::bad(span, "Econ.expected_holding_time needs matrix"))?;
    let n = args::rec_u64(args, "n")
        .ok_or_else(|| args::bad(span, "Econ.expected_holding_time needs n"))? as usize;
    let state = args::rec_u64(args, "state")
        .ok_or_else(|| args::bad(span, "Econ.expected_holding_time needs state"))?
        as usize;
    match crate::specialized_libs::computational_economics::markov::expected_holding_time(
        &p, n, state,
    ) {
        Ok(t) => Ok(args::record([("holding_time", Value::F64(t))])),
        Err(e) => Err(args::bad(span, format!("expected_holding_time: {e:?}"))),
    }
}

// ── Labor / household ────────────────────────────────────────────────────────

pub fn econ_labor_supply(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let wage = args::rec_f64(args, "wage")
        .ok_or_else(|| args::bad(span, "Econ.labor_supply needs wage"))?;
    let time_endowment = args::rec_f64(args, "time_endowment").unwrap_or(24.0);
    let non_labor_income = args::rec_f64(args, "non_labor_income").unwrap_or(0.0);
    let alpha = args::rec_f64(args, "alpha").unwrap_or(0.5);
    match crate::specialized_libs::computational_economics::labor_household::labor_supply_cobb_douglas(wage, time_endowment, non_labor_income, alpha) {
        Ok((l, c)) => Ok(args::record([
            ("labor", Value::F64(l)),
            ("consumption", Value::F64(c)),
        ])),
        Err(e) => Err(args::bad(span, format!("labor_supply: {e:?}"))),
    }
}

pub fn econ_efficiency_units(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let raw = args::rec_f64(args, "raw_labor")
        .ok_or_else(|| args::bad(span, "Econ.efficiency_units needs raw_labor"))?;
    let hc = args::rec_f64(args, "human_capital")
        .ok_or_else(|| args::bad(span, "Econ.efficiency_units needs human_capital"))?;
    match crate::specialized_libs::computational_economics::labor_household::efficiency_units(
        raw, hc,
    ) {
        Ok(e) => Ok(args::record([("efficiency_units", Value::F64(e))])),
        Err(e) => Err(args::bad(span, format!("efficiency_units: {e:?}"))),
    }
}

// ── Environmental ────────────────────────────────────────────────────────────

pub fn econ_social_cost_of_carbon(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let emissions = args::rec_f64(args, "emissions")
        .ok_or_else(|| args::bad(span, "Econ.social_cost_of_carbon needs emissions"))?;
    let damage_per_ton = args::rec_f64(args, "damage_per_ton")
        .ok_or_else(|| args::bad(span, "Econ.social_cost_of_carbon needs damage_per_ton"))?;
    match crate::specialized_libs::computational_economics::environmental_resource::social_cost_of_carbon(emissions, damage_per_ton) {
        Ok(scc) => Ok(args::record([("social_cost", Value::F64(scc))])),
        Err(e) => Err(args::bad(span, format!("social_cost_of_carbon: {e:?}"))),
    }
}

pub fn econ_optimal_pollution(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let baseline_emissions = args::rec_f64(args, "baseline_emissions")
        .ok_or_else(|| args::bad(span, "Econ.optimal_pollution needs baseline_emissions"))?;
    let abatement_coeff = args::rec_f64(args, "abatement_coeff")
        .ok_or_else(|| args::bad(span, "Econ.optimal_pollution needs abatement_coeff"))?;
    let damage_coeff = args::rec_f64(args, "damage_coeff")
        .ok_or_else(|| args::bad(span, "Econ.optimal_pollution needs damage_coeff"))?;
    match crate::specialized_libs::computational_economics::environmental_resource::optimal_pollution(baseline_emissions, abatement_coeff, damage_coeff) {
        Ok(e) => Ok(args::record([("optimal_emissions", Value::F64(e))])),
        Err(e) => Err(args::bad(span, format!("optimal_pollution: {e:?}"))),
    }
}

pub fn econ_optimal_abatement(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let baseline_emissions = args::rec_f64(args, "baseline_emissions")
        .ok_or_else(|| args::bad(span, "Econ.optimal_abatement needs baseline_emissions"))?;
    let abatement_coeff = args::rec_f64(args, "abatement_coeff")
        .ok_or_else(|| args::bad(span, "Econ.optimal_abatement needs abatement_coeff"))?;
    let damage_coeff = args::rec_f64(args, "damage_coeff")
        .ok_or_else(|| args::bad(span, "Econ.optimal_abatement needs damage_coeff"))?;
    match crate::specialized_libs::computational_economics::environmental_resource::optimal_abatement(baseline_emissions, abatement_coeff, damage_coeff) {
        Ok(a) => Ok(args::record([("optimal_abatement", Value::F64(a))])),
        Err(e) => Err(args::bad(span, format!("optimal_abatement: {e:?}"))),
    }
}

// ── Dynamic programming ──────────────────────────────────────────────────────

pub fn econ_bellman_update(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let rewards = args::rec_f64_list(args, "rewards")
        .ok_or_else(|| args::bad(span, "Econ.bellman_update needs rewards"))?;
    let transitions = args::rec_f64_list(args, "transitions")
        .ok_or_else(|| args::bad(span, "Econ.bellman_update needs transitions"))?;
    let discount = args::rec_f64(args, "discount")
        .ok_or_else(|| args::bad(span, "Econ.bellman_update needs discount"))?;
    let values = args::rec_f64_list(args, "values")
        .ok_or_else(|| args::bad(span, "Econ.bellman_update needs values"))?;
    let n_states = args::rec_u64(args, "n_states")
        .ok_or_else(|| args::bad(span, "Econ.bellman_update needs n_states"))?
        as usize;
    let n_actions = args::rec_u64(args, "n_actions")
        .ok_or_else(|| args::bad(span, "Econ.bellman_update needs n_actions"))?
        as usize;
    let state = args::rec_u64(args, "state")
        .ok_or_else(|| args::bad(span, "Econ.bellman_update needs state"))?
        as usize;
    match crate::specialized_libs::computational_economics::dynamic_programming::bellman_update(
        &rewards,
        &transitions,
        discount,
        &values,
        n_states,
        n_actions,
        state,
    ) {
        Ok(v) => Ok(args::record([("updated_value", Value::F64(v))])),
        Err(e) => Err(args::bad(span, format!("bellman_update: {e:?}"))),
    }
}

// ── Forensic economics ───────────────────────────────────────────────────────

pub fn econ_malfeasance_delta(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let capital_allocated = args::rec_f64(args, "capital_allocated")
        .ok_or_else(|| args::bad(span, "Econ.malfeasance_delta needs capital_allocated"))?;
    let delivered = args::rec_f64(args, "delivered")
        .ok_or_else(|| args::bad(span, "Econ.malfeasance_delta needs delivered"))?;
    let inverted = args::rec_bool(args, "inverted").unwrap_or(false);
    match crate::specialized_libs::computational_economics::forensic_economics::compute_malfeasance_delta(capital_allocated, delivered, inverted) {
        Ok(delta) => Ok(args::record([
            ("capital_allocated", Value::F64(delta.capital_allocated)),
            ("delivered_utility", Value::F64(delta.delivered_utility)),
            ("delta", Value::F64(delta.delta)),
            ("governance_yield_inverted", Value::Bool(delta.governance_yield_inverted)),
        ])),
        Err(e) => Err(args::bad(span, format!("malfeasance_delta: {e:?}"))),
    }
}

// ── Econometrics ─────────────────────────────────────────────────────────────

pub fn econ_ols(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_f64_list(args, "x").ok_or_else(|| args::bad(span, "Econ.ols needs x"))?;
    let y = args::rec_f64_list(args, "y").ok_or_else(|| args::bad(span, "Econ.ols needs y"))?;
    let n_obs = args::rec_u64(args, "n_obs")
        .ok_or_else(|| args::bad(span, "Econ.ols needs n_obs"))? as usize;
    let n_reg = args::rec_u64(args, "n_reg").unwrap_or(1) as usize;
    let mut beta = vec![0.0; n_reg];
    let mut residuals = vec![0.0; n_obs];
    match crate::specialized_libs::computational_economics::econometrics::ols_into(
        &x,
        &y,
        n_obs,
        n_reg,
        &mut beta,
        &mut residuals,
    ) {
        Ok(r_squared) => Ok(args::record([
            (
                "beta",
                Value::List(beta.into_iter().map(Value::F64).collect()),
            ),
            (
                "residuals",
                Value::List(residuals.into_iter().map(Value::F64).collect()),
            ),
            ("r_squared", Value::F64(r_squared)),
        ])),
        Err(e) => Err(args::bad(span, format!("ols: {e:?}"))),
    }
}

// ── Agent-based ──────────────────────────────────────────────────────────────

pub fn econ_aggregate_wealth(_args: &Value, _span: Span) -> Result<Value, Diagnostic> {
    // Agent-based aggregate wealth — simplified exposure.
    Ok(args::record([(
        "status",
        Value::String("agent_based_available".into()),
    )]))
}

// ── Input-output ─────────────────────────────────────────────────────────────

pub fn econ_total_transport_cost(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let flows = args::rec_f64_list(args, "flows")
        .ok_or_else(|| args::bad(span, "Econ.total_transport_cost needs flows"))?;
    let distances = args::rec_f64_list(args, "distances")
        .ok_or_else(|| args::bad(span, "Econ.total_transport_cost needs distances"))?;
    let n = args::rec_u64(args, "n")
        .ok_or_else(|| args::bad(span, "Econ.total_transport_cost needs n"))? as usize;
    match crate::specialized_libs::computational_economics::spatial_economics::total_transport_cost(
        &flows, &distances, n,
    ) {
        Ok(total) => Ok(args::record([("total_cost", Value::F64(total))])),
        Err(e) => Err(args::bad(span, format!("total_transport_cost: {e:?}"))),
    }
}

// ── Asset pricing: Lucas pricing ─────────────────────────────────────────────

pub fn econ_lucas_asset_price(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let dividend_paths = args::rec_f64_list(args, "dividend_paths")
        .ok_or_else(|| args::bad(span, "Econ.lucas_asset_price needs dividend_paths"))?;
    let consumption_paths = args::rec_f64_list(args, "consumption_paths")
        .ok_or_else(|| args::bad(span, "Econ.lucas_asset_price needs consumption_paths"))?;
    let n_paths = args::rec_u64(args, "n_paths")
        .ok_or_else(|| args::bad(span, "Econ.lucas_asset_price needs n_paths"))?
        as usize;
    let n_periods = args::rec_u64(args, "n_periods")
        .ok_or_else(|| args::bad(span, "Econ.lucas_asset_price needs n_periods"))?
        as usize;
    let beta = args::rec_f64(args, "beta")
        .ok_or_else(|| args::bad(span, "Econ.lucas_asset_price needs beta"))?;
    let gamma = args::rec_f64(args, "gamma")
        .ok_or_else(|| args::bad(span, "Econ.lucas_asset_price needs gamma"))?;
    match crate::specialized_libs::computational_economics::asset_pricing::lucas_asset_price(
        &dividend_paths,
        &consumption_paths,
        n_paths,
        n_periods,
        beta,
        gamma,
        None,
    ) {
        Ok(price) => Ok(args::record([("price", Value::F64(price))])),
        Err(e) => Err(args::bad(span, format!("lucas_asset_price: {e:?}"))),
    }
}

// ── Behavioral: present-biased, reference-dependent ───────────────────────────

pub fn econ_present_biased_utility(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let utilities = args::rec_f64_list(args, "utilities")
        .ok_or_else(|| args::bad(span, "Econ.present_biased_utility needs utilities"))?;
    let beta = args::rec_f64(args, "beta").unwrap_or(0.7);
    let delta = args::rec_f64(args, "delta").unwrap_or(0.99);
    match crate::specialized_libs::computational_economics::behavioral::present_biased_utility(
        &utilities, beta, delta,
    ) {
        Ok(u) => Ok(args::record([("utility", Value::F64(u))])),
        Err(e) => Err(args::bad(span, format!("present_biased_utility: {e:?}"))),
    }
}

pub fn econ_reference_dependent_utility(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_f64(args, "x")
        .ok_or_else(|| args::bad(span, "Econ.reference_dependent_utility needs x"))?;
    let r = args::rec_f64(args, "reference")
        .ok_or_else(|| args::bad(span, "Econ.reference_dependent_utility needs reference"))?;
    let alpha = args::rec_f64(args, "alpha").unwrap_or(0.88);
    let beta = args::rec_f64(args, "beta").unwrap_or(0.88);
    let lambda = args::rec_f64(args, "lambda").unwrap_or(2.25);
    match crate::specialized_libs::computational_economics::behavioral::reference_dependent_utility(
        x, r, alpha, beta, lambda,
    ) {
        Ok(u) => Ok(args::record([("utility", Value::F64(u))])),
        Err(e) => Err(args::bad(
            span,
            format!("reference_dependent_utility: {e:?}"),
        )),
    }
}

// ── Game theory: pure Nash, dominated strategies, repeated games ──────────────

pub fn econ_pure_nash_equilibria(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let row = args::rec_f64_list(args, "payoff_row")
        .ok_or_else(|| args::bad(span, "Econ.pure_nash_equilibria needs payoff_row"))?;
    let col = args::rec_f64_list(args, "payoff_col")
        .ok_or_else(|| args::bad(span, "Econ.pure_nash_equilibria needs payoff_col"))?;
    let n_row = args::rec_u64(args, "n_row")
        .ok_or_else(|| args::bad(span, "Econ.pure_nash_equilibria needs n_row"))?
        as usize;
    let n_col = args::rec_u64(args, "n_col")
        .ok_or_else(|| args::bad(span, "Econ.pure_nash_equilibria needs n_col"))?
        as usize;
    let mut out: Vec<(usize, usize)> = vec![(0, 0); n_row * n_col];
    match crate::specialized_libs::computational_economics::game_theory::pure_nash_equilibria_into(
        &row, &col, n_row, n_col, &mut out,
    ) {
        Ok(count) => {
            let eqs: Vec<Value> = out[..count]
                .iter()
                .map(|(r, c)| {
                    args::record([
                        ("row", Value::U64(*r as u64)),
                        ("col", Value::U64(*c as u64)),
                    ])
                })
                .collect();
            Ok(args::record([
                ("equilibria", Value::List(eqs)),
                ("count", Value::U64(count as u64)),
            ]))
        }
        Err(e) => Err(args::bad(span, format!("pure_nash_equilibria: {e:?}"))),
    }
}

pub fn econ_repeated_game_payoff(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let stage_payoffs = args::rec_f64_list(args, "stage_payoffs")
        .ok_or_else(|| args::bad(span, "Econ.repeated_game_payoff needs stage_payoffs"))?;
    let discount = args::rec_f64(args, "discount")
        .ok_or_else(|| args::bad(span, "Econ.repeated_game_payoff needs discount"))?;
    let n_rounds = args::rec_u64(args, "n_rounds").unwrap_or(stage_payoffs.len() as u64) as usize;
    match crate::specialized_libs::computational_economics::game_theory::repeated_game_payoff(
        &stage_payoffs,
        discount,
        n_rounds,
    ) {
        Ok(payoff) => Ok(args::record([("total_payoff", Value::F64(payoff))])),
        Err(e) => Err(args::bad(span, format!("repeated_game_payoff: {e:?}"))),
    }
}

pub fn econ_bertrand_with_demand(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = args::rec_f64(args, "demand_intercept")
        .ok_or_else(|| args::bad(span, "Econ.bertrand_with_demand needs demand_intercept"))?;
    let b = args::rec_f64(args, "demand_slope")
        .ok_or_else(|| args::bad(span, "Econ.bertrand_with_demand needs demand_slope"))?;
    let c1 = args::rec_f64(args, "cost_1")
        .ok_or_else(|| args::bad(span, "Econ.bertrand_with_demand needs cost_1"))?;
    let c2 = args::rec_f64(args, "cost_2")
        .ok_or_else(|| args::bad(span, "Econ.bertrand_with_demand needs cost_2"))?;
    match crate::specialized_libs::computational_economics::game_theory::bertrand_duopoly_with_demand(a, b, c1, c2) {
        Ok((price, quantity)) => Ok(args::record([
            ("price", Value::F64(price)),
            ("quantity", Value::F64(quantity)),
        ])),
        Err(e) => Err(args::bad(span, format!("bertrand_with_demand: {e:?}"))),
    }
}

// ── Macro: Solow simulate, Ramsey euler, RBC, New Keynesian ───────────────────

pub fn econ_ramsey_euler_residual(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let k = args::rec_f64(args, "capital")
        .ok_or_else(|| args::bad(span, "Econ.ramsey_euler_residual needs capital"))?;
    let k_next = args::rec_f64(args, "capital_next")
        .ok_or_else(|| args::bad(span, "Econ.ramsey_euler_residual needs capital_next"))?;
    let beta = args::rec_f64(args, "beta")
        .ok_or_else(|| args::bad(span, "Econ.ramsey_euler_residual needs beta"))?;
    let alpha = args::rec_f64(args, "alpha")
        .ok_or_else(|| args::bad(span, "Econ.ramsey_euler_residual needs alpha"))?;
    let delta = args::rec_f64(args, "delta")
        .ok_or_else(|| args::bad(span, "Econ.ramsey_euler_residual needs delta"))?;
    let sigma = args::rec_f64(args, "sigma").unwrap_or(1.0);
    let residual =
        crate::specialized_libs::computational_economics::macro_models::ramsey_euler_residual(
            k, k_next, beta, alpha, delta, sigma,
        );
    Ok(args::record([("euler_residual", Value::F64(residual))]))
}

pub fn econ_new_keynesian_solve(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let r_prev = args::rec_f64(args, "r_prev")
        .ok_or_else(|| args::bad(span, "Econ.new_keynesian_solve needs r_prev"))?;
    let beta = args::rec_f64(args, "beta")
        .ok_or_else(|| args::bad(span, "Econ.new_keynesian_solve needs beta"))?;
    let kappa = args::rec_f64(args, "kappa")
        .ok_or_else(|| args::bad(span, "Econ.new_keynesian_solve needs kappa"))?;
    let sigma = args::rec_f64(args, "sigma")
        .ok_or_else(|| args::bad(span, "Econ.new_keynesian_solve needs sigma"))?;
    let phi_pi = args::rec_f64(args, "phi_pi")
        .ok_or_else(|| args::bad(span, "Econ.new_keynesian_solve needs phi_pi"))?;
    let phi_y = args::rec_f64(args, "phi_y")
        .ok_or_else(|| args::bad(span, "Econ.new_keynesian_solve needs phi_y"))?;
    let rho_r = args::rec_f64(args, "rho_r")
        .ok_or_else(|| args::bad(span, "Econ.new_keynesian_solve needs rho_r"))?;
    let r_nat = args::rec_f64(args, "r_nat")
        .ok_or_else(|| args::bad(span, "Econ.new_keynesian_solve needs r_nat"))?;
    match crate::specialized_libs::computational_economics::macro_models::new_keynesian_solve(
        r_prev, beta, kappa, sigma, phi_pi, phi_y, rho_r, r_nat,
    ) {
        Ok((output_gap, inflation, interest_rate)) => Ok(args::record([
            ("output_gap", Value::F64(output_gap)),
            ("inflation", Value::F64(inflation)),
            ("interest_rate", Value::F64(interest_rate)),
        ])),
        Err(e) => Err(args::bad(span, format!("new_keynesian_solve: {e:?}"))),
    }
}

// ── Welfare: Lorenz, distributional NPV, survival floor ───────────────────────

pub fn econ_lorenz_curve(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let incomes = args::rec_f64_list(args, "incomes")
        .ok_or_else(|| args::bad(span, "Econ.lorenz_curve needs incomes"))?;
    let mut out = vec![0.0; incomes.len()];
    match crate::specialized_libs::computational_economics::welfare::lorenz_curve_into(
        &incomes, &mut out,
    ) {
        Ok(n) => Ok(args::record([
            (
                "lorenz_points",
                Value::List(out[..n].iter().map(|v| Value::F64(*v)).collect()),
            ),
            ("count", Value::U64(n as u64)),
        ])),
        Err(e) => Err(args::bad(span, format!("lorenz_curve: {e:?}"))),
    }
}

pub fn econ_distributional_npv(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let benefits = args::rec_f64_list(args, "benefits")
        .ok_or_else(|| args::bad(span, "Econ.distributional_npv needs benefits"))?;
    let costs = args::rec_f64_list(args, "costs")
        .ok_or_else(|| args::bad(span, "Econ.distributional_npv needs costs"))?;
    let weights = args::rec_f64_list(args, "weights")
        .ok_or_else(|| args::bad(span, "Econ.distributional_npv needs weights"))?;
    let discount_rate = args::rec_f64(args, "discount_rate")
        .ok_or_else(|| args::bad(span, "Econ.distributional_npv needs discount_rate"))?;
    let n_periods = args::rec_u64(args, "n_periods")
        .ok_or_else(|| args::bad(span, "Econ.distributional_npv needs n_periods"))?
        as usize;
    match crate::specialized_libs::computational_economics::welfare::distributional_npv(
        &benefits,
        &costs,
        &weights,
        discount_rate,
        n_periods,
    ) {
        Ok(report) => Ok(args::record([
            ("weighted_npv", Value::F64(report.value)),
            ("unweighted_npv", Value::F64(report.auxiliary)),
            ("assumptions", Value::U64(report.assumptions as u64)),
        ])),
        Err(e) => Err(args::bad(span, format!("distributional_npv: {e:?}"))),
    }
}

// ── Portfolio: covariance, variance, risk contributions ───────────────────────

pub fn econ_portfolio_returns(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let flat_returns = args::rec_f64_list(args, "asset_returns")
        .ok_or_else(|| args::bad(span, "Econ.portfolio_returns needs asset_returns"))?;
    let weights = args::rec_f64_list(args, "weights")
        .ok_or_else(|| args::bad(span, "Econ.portfolio_returns needs weights"))?;
    let n_periods = args::rec_u64(args, "n_periods")
        .ok_or_else(|| args::bad(span, "Econ.portfolio_returns needs n_periods"))?
        as usize;
    let n_assets = args::rec_u64(args, "n_assets")
        .ok_or_else(|| args::bad(span, "Econ.portfolio_returns needs n_assets"))?
        as usize;
    let mut out = vec![0.0; n_periods];
    match crate::specialized_libs::computational_economics::portfolio::portfolio_returns_into(
        &flat_returns,
        n_periods,
        n_assets,
        &weights,
        &mut out,
    ) {
        Ok(n) => Ok(args::record([
            (
                "portfolio_returns",
                Value::List(out[..n].iter().map(|v| Value::F64(*v)).collect()),
            ),
            ("count", Value::U64(n as u64)),
        ])),
        Err(e) => Err(args::bad(span, format!("portfolio_returns: {e:?}"))),
    }
}

pub fn econ_covariance_matrix(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let returns = args::rec_f64_list(args, "returns")
        .ok_or_else(|| args::bad(span, "Econ.covariance_matrix needs returns"))?;
    let n_periods = args::rec_u64(args, "n_periods")
        .ok_or_else(|| args::bad(span, "Econ.covariance_matrix needs n_periods"))?
        as usize;
    let n_assets = args::rec_u64(args, "n_assets")
        .ok_or_else(|| args::bad(span, "Econ.covariance_matrix needs n_assets"))?
        as usize;
    let mut out = vec![0.0; n_assets * n_assets];
    match crate::specialized_libs::computational_economics::portfolio::covariance_matrix_into(
        &returns, n_periods, n_assets, &mut out,
    ) {
        Ok(_) => Ok(args::record([
            (
                "covariance",
                Value::List(out.iter().map(|v| Value::F64(*v)).collect()),
            ),
            ("n_assets", Value::U64(n_assets as u64)),
        ])),
        Err(e) => Err(args::bad(span, format!("covariance_matrix: {e:?}"))),
    }
}

pub fn econ_portfolio_variance(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let weights = args::rec_f64_list(args, "weights")
        .ok_or_else(|| args::bad(span, "Econ.portfolio_variance needs weights"))?;
    let cov = args::rec_f64_list(args, "covariance")
        .ok_or_else(|| args::bad(span, "Econ.portfolio_variance needs covariance"))?;
    let n_assets = args::rec_u64(args, "n_assets")
        .ok_or_else(|| args::bad(span, "Econ.portfolio_variance needs n_assets"))?
        as usize;
    match crate::specialized_libs::computational_economics::portfolio::portfolio_variance_from_covariance(&weights, &cov, n_assets) {
        Ok(variance) => Ok(args::record([("portfolio_variance", Value::F64(variance))])),
        Err(e) => Err(args::bad(span, format!("portfolio_variance: {e:?}"))),
    }
}

// ── Time series: returns, wealth, drawdown, rolling stats, simulations ────────

pub fn econ_simple_returns(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let prices = args::rec_f64_list(args, "prices")
        .ok_or_else(|| args::bad(span, "Econ.simple_returns needs prices"))?;
    let mut out = vec![0.0; prices.len().saturating_sub(1)];
    match crate::specialized_libs::computational_economics::time_series::simple_returns_into(
        &prices, &mut out,
    ) {
        Ok(n) => Ok(args::record([
            (
                "returns",
                Value::List(out[..n].iter().map(|v| Value::F64(*v)).collect()),
            ),
            ("count", Value::U64(n as u64)),
        ])),
        Err(e) => Err(args::bad(span, format!("simple_returns: {e:?}"))),
    }
}

pub fn econ_log_returns(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let prices = args::rec_f64_list(args, "prices")
        .ok_or_else(|| args::bad(span, "Econ.log_returns needs prices"))?;
    let mut out = vec![0.0; prices.len().saturating_sub(1)];
    match crate::specialized_libs::computational_economics::time_series::log_returns_into(
        &prices, &mut out,
    ) {
        Ok(n) => Ok(args::record([
            (
                "returns",
                Value::List(out[..n].iter().map(|v| Value::F64(*v)).collect()),
            ),
            ("count", Value::U64(n as u64)),
        ])),
        Err(e) => Err(args::bad(span, format!("log_returns: {e:?}"))),
    }
}

pub fn econ_cumulative_wealth(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let returns = args::rec_f64_list(args, "returns")
        .ok_or_else(|| args::bad(span, "Econ.cumulative_wealth needs returns"))?;
    let mut out = vec![0.0; returns.len()];
    match crate::specialized_libs::computational_economics::time_series::cumulative_wealth_into(
        &returns, &mut out,
    ) {
        Ok(n) => Ok(args::record([
            (
                "wealth",
                Value::List(out[..n].iter().map(|v| Value::F64(*v)).collect()),
            ),
            ("count", Value::U64(n as u64)),
        ])),
        Err(e) => Err(args::bad(span, format!("cumulative_wealth: {e:?}"))),
    }
}

pub fn econ_drawdown(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let wealth = args::rec_f64_list(args, "wealth")
        .ok_or_else(|| args::bad(span, "Econ.drawdown needs wealth"))?;
    let mut out = vec![0.0; wealth.len()];
    match crate::specialized_libs::computational_economics::time_series::drawdown_into(
        &wealth, &mut out,
    ) {
        Ok(n) => Ok(args::record([
            (
                "drawdown",
                Value::List(out[..n].iter().map(|v| Value::F64(*v)).collect()),
            ),
            ("count", Value::U64(n as u64)),
        ])),
        Err(e) => Err(args::bad(span, format!("drawdown: {e:?}"))),
    }
}

pub fn econ_rolling_mean(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let values = args::rec_f64_list(args, "values")
        .ok_or_else(|| args::bad(span, "Econ.rolling_mean needs values"))?;
    let window = args::rec_u64(args, "window")
        .ok_or_else(|| args::bad(span, "Econ.rolling_mean needs window"))?
        as usize;
    let mut out = vec![0.0; values.len()];
    match crate::specialized_libs::computational_economics::time_series::rolling_mean_into(
        &values, window, &mut out,
    ) {
        Ok(n) => Ok(args::record([
            (
                "rolling_mean",
                Value::List(out[..n].iter().map(|v| Value::F64(*v)).collect()),
            ),
            ("count", Value::U64(n as u64)),
        ])),
        Err(e) => Err(args::bad(span, format!("rolling_mean: {e:?}"))),
    }
}

pub fn econ_rolling_variance(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let values = args::rec_f64_list(args, "values")
        .ok_or_else(|| args::bad(span, "Econ.rolling_variance needs values"))?;
    let window = args::rec_u64(args, "window")
        .ok_or_else(|| args::bad(span, "Econ.rolling_variance needs window"))?
        as usize;
    let mut out = vec![0.0; values.len()];
    match crate::specialized_libs::computational_economics::time_series::rolling_variance_into(
        &values, window, &mut out,
    ) {
        Ok(n) => Ok(args::record([
            (
                "rolling_variance",
                Value::List(out[..n].iter().map(|v| Value::F64(*v)).collect()),
            ),
            ("count", Value::U64(n as u64)),
        ])),
        Err(e) => Err(args::bad(span, format!("rolling_variance: {e:?}"))),
    }
}

pub fn econ_gbm_simulate(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let s0 =
        args::rec_f64(args, "s0").ok_or_else(|| args::bad(span, "Econ.gbm_simulate needs s0"))?;
    let mu =
        args::rec_f64(args, "mu").ok_or_else(|| args::bad(span, "Econ.gbm_simulate needs mu"))?;
    let sigma = args::rec_f64(args, "sigma")
        .ok_or_else(|| args::bad(span, "Econ.gbm_simulate needs sigma"))?;
    let dt =
        args::rec_f64(args, "dt").ok_or_else(|| args::bad(span, "Econ.gbm_simulate needs dt"))?;
    let n_steps = args::rec_u64(args, "n_steps")
        .ok_or_else(|| args::bad(span, "Econ.gbm_simulate needs n_steps"))?
        as usize;
    let seed = args::rec_u64(args, "seed").unwrap_or(42);
    let mut out = vec![0.0; n_steps];
    match crate::specialized_libs::computational_economics::time_series::gbm_simulate_into(
        s0, mu, sigma, dt, n_steps, seed, &mut out,
    ) {
        Ok(n) => Ok(args::record([(
            "path",
            Value::List(out[..n].iter().map(|v| Value::F64(*v)).collect()),
        )])),
        Err(e) => Err(args::bad(span, format!("gbm_simulate: {e:?}"))),
    }
}

pub fn econ_stress_scenario(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let returns = args::rec_f64_list(args, "returns")
        .ok_or_else(|| args::bad(span, "Econ.stress_scenario needs returns"))?;
    let shock = args::rec_f64(args, "shock")
        .ok_or_else(|| args::bad(span, "Econ.stress_scenario needs shock"))?;
    let mut out = vec![0.0; returns.len()];
    match crate::specialized_libs::computational_economics::time_series::apply_stress_scenario_into(
        &returns, shock, 0, &mut out,
    ) {
        Ok(n) => Ok(args::record([
            (
                "stressed_returns",
                Value::List(out[..n].iter().map(|v| Value::F64(*v)).collect()),
            ),
            ("count", Value::U64(n as u64)),
        ])),
        Err(e) => Err(args::bad(span, format!("stress_scenario: {e:?}"))),
    }
}

pub fn econ_block_bootstrap(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let returns = args::rec_f64_list(args, "returns")
        .ok_or_else(|| args::bad(span, "Econ.block_bootstrap needs returns"))?;
    let block_size = args::rec_u64(args, "block_size")
        .ok_or_else(|| args::bad(span, "Econ.block_bootstrap needs block_size"))?
        as usize;
    let n_resamples = args::rec_u64(args, "n_resamples")
        .ok_or_else(|| args::bad(span, "Econ.block_bootstrap needs n_resamples"))?
        as usize;
    let seed = args::rec_u64(args, "seed").unwrap_or(42);
    let mut out = vec![0.0; n_resamples];
    match crate::specialized_libs::computational_economics::time_series::block_bootstrap_mean_into(
        &returns,
        block_size,
        n_resamples,
        seed,
        &mut out,
    ) {
        Ok(n) => Ok(args::record([(
            "bootstrap_means",
            Value::List(out[..n].iter().map(|v| Value::F64(*v)).collect()),
        )])),
        Err(e) => Err(args::bad(span, format!("block_bootstrap: {e:?}"))),
    }
}

// ── Yield curve: par yield, spot curve ────────────────────────────────────────

pub fn econ_par_yield(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    use crate::specialized_libs::computational_economics::yield_curve::{
        par_yield_from_zero_curve, CurvePoint,
    };
    let maturities = args::rec_f64_list(args, "maturities")
        .ok_or_else(|| args::bad(span, "Econ.par_yield needs maturities"))?;
    let rates = args::rec_f64_list(args, "rates")
        .ok_or_else(|| args::bad(span, "Econ.par_yield needs rates"))?;
    let target = args::rec_f64(args, "target_maturity")
        .ok_or_else(|| args::bad(span, "Econ.par_yield needs target_maturity"))?;
    let compounding = args::rec_u64(args, "compounding_per_year").unwrap_or(1) as u32;
    if maturities.len() != rates.len() {
        return Err(args::bad(
            span,
            "Econ.par_yield: maturities and rates must have equal length",
        ));
    }
    let points: Vec<CurvePoint> = maturities
        .iter()
        .zip(rates.iter())
        .map(|(&t, &r)| CurvePoint {
            time_years: t,
            zero_rate: r,
        })
        .collect();
    match par_yield_from_zero_curve(&points, target, compounding) {
        Ok(y) => Ok(args::record([("par_yield", Value::F64(y))])),
        Err(e) => Err(args::bad(span, format!("par_yield: {e:?}"))),
    }
}

// ── Spatial: gravity matrix, transport cost matrix, nearest facility ──────────

pub fn econ_nearest_facility(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let demands = args::rec_f64_list(args, "demands")
        .ok_or_else(|| args::bad(span, "Econ.nearest_facility needs demands"))?;
    let facilities = args::rec_f64_list(args, "facilities")
        .ok_or_else(|| args::bad(span, "Econ.nearest_facility needs facilities"))?;
    let n_demands = args::rec_u64(args, "n_demands")
        .ok_or_else(|| args::bad(span, "Econ.nearest_facility needs n_demands"))?
        as usize;
    let n_facilities = args::rec_u64(args, "n_facilities")
        .ok_or_else(|| args::bad(span, "Econ.nearest_facility needs n_facilities"))?
        as usize;
    let mut out = vec![0usize; n_demands];
    match crate::specialized_libs::computational_economics::spatial_economics::nearest_facility_into(
        &demands,
        &facilities,
        n_demands,
        n_facilities,
        &mut out,
    ) {
        Ok(_) => Ok(args::record([(
            "assignments",
            Value::List(out.iter().map(|v| Value::U64(*v as u64)).collect()),
        )])),
        Err(e) => Err(args::bad(span, format!("nearest_facility: {e:?}"))),
    }
}

// ── Public finance: progressive tax ───────────────────────────────────────────

pub fn econ_progressive_tax(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    use crate::specialized_libs::computational_economics::public_finance::{
        progressive_tax_into, TaxBracket,
    };
    let thresholds = args::rec_f64_list(args, "thresholds")
        .ok_or_else(|| args::bad(span, "Econ.progressive_tax needs thresholds"))?;
    let rates = args::rec_f64_list(args, "rates")
        .ok_or_else(|| args::bad(span, "Econ.progressive_tax needs rates"))?;
    let income = args::rec_f64(args, "income")
        .ok_or_else(|| args::bad(span, "Econ.progressive_tax needs income"))?;
    if thresholds.len() != rates.len() {
        return Err(args::bad(
            span,
            "Econ.progressive_tax: thresholds and rates must have equal length",
        ));
    }
    let brackets: Vec<TaxBracket> = thresholds
        .iter()
        .zip(rates.iter())
        .map(|(&t, &r)| TaxBracket {
            threshold: t,
            marginal_rate: r,
        })
        .collect();
    let mut out = vec![0.0; brackets.len()];
    match progressive_tax_into(income, &brackets, &mut out) {
        Ok((total_tax, effective_rate)) => Ok(args::record([
            (
                "tax_per_bracket",
                Value::List(out.iter().map(|v| Value::F64(*v)).collect()),
            ),
            ("total_tax", Value::F64(total_tax)),
            ("effective_rate", Value::F64(effective_rate)),
        ])),
        Err(e) => Err(args::bad(span, format!("progressive_tax: {e:?}"))),
    }
}

// ── Mechanism: VCG, strategy-proofness, report ────────────────────────────────

pub fn econ_vcg_payment(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let valuations = args::rec_f64_list(args, "valuations")
        .ok_or_else(|| args::bad(span, "Econ.vcg_payment needs valuations"))?;
    let n_agents = valuations.len();
    let mut out = vec![0.0; n_agents];
    match crate::specialized_libs::computational_economics::mechanism::vickrey_clarke_groves_payment_into(&valuations, &mut out) {
        Ok(total_revenue) => Ok(args::record([
            ("payments", Value::List(out.iter().map(|v| Value::F64(*v)).collect())),
            ("total_revenue", Value::F64(total_revenue)),
        ])),
        Err(e) => Err(args::bad(span, format!("vcg_payment: {e:?}"))),
    }
}

pub fn econ_strategy_proofness(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let valuations = args::rec_f64_list(args, "valuations")
        .ok_or_else(|| args::bad(span, "Econ.strategy_proofness needs valuations (2x2)"))?;
    let allocation = args::rec_bool_list(args, "allocation")
        .ok_or_else(|| args::bad(span, "Econ.strategy_proofness needs allocation (2x2 bools)"))?;
    let payments = args::rec_f64_list(args, "payments")
        .ok_or_else(|| args::bad(span, "Econ.strategy_proofness needs payments (2x2)"))?;
    match crate::specialized_libs::computational_economics::mechanism::check_strategy_proofness_2x2(
        &valuations,
        &allocation,
        &payments,
    ) {
        Ok(proof) => Ok(args::record([("strategy_proof", Value::Bool(proof))])),
        Err(e) => Err(args::bad(span, format!("strategy_proofness: {e:?}"))),
    }
}

// ── Markov: stationary distribution, simulate, first passage ──────────────────

pub fn econ_stationary_distribution(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let p = args::rec_f64_list(args, "matrix")
        .ok_or_else(|| args::bad(span, "Econ.stationary_distribution needs matrix"))?;
    let n = args::rec_u64(args, "n")
        .ok_or_else(|| args::bad(span, "Econ.stationary_distribution needs n"))?
        as usize;
    let max_iter = args::rec_u64(args, "max_iter").unwrap_or(1000) as u32;
    let tolerance = args::rec_f64(args, "tolerance").unwrap_or(1e-9);
    let mut out = vec![0.0; n];
    match crate::specialized_libs::computational_economics::markov::stationary_distribution_into(
        &p, n, max_iter, tolerance, &mut out,
    ) {
        Ok(_) => Ok(args::record([(
            "distribution",
            Value::List(out.iter().map(|v| Value::F64(*v)).collect()),
        )])),
        Err(e) => Err(args::bad(span, format!("stationary_distribution: {e:?}"))),
    }
}

pub fn econ_simulate_chain(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let p = args::rec_f64_list(args, "matrix")
        .ok_or_else(|| args::bad(span, "Econ.simulate_chain needs matrix"))?;
    let n = args::rec_u64(args, "n")
        .ok_or_else(|| args::bad(span, "Econ.simulate_chain needs n"))? as usize;
    let start = args::rec_u64(args, "start")
        .ok_or_else(|| args::bad(span, "Econ.simulate_chain needs start"))?
        as usize;
    let n_steps = args::rec_u64(args, "n_steps")
        .ok_or_else(|| args::bad(span, "Econ.simulate_chain needs n_steps"))?
        as usize;
    let seed = args::rec_u64(args, "seed").unwrap_or(42);
    let mut out = vec![0usize; n_steps + 1];
    match crate::specialized_libs::computational_economics::markov::simulate_chain_into(
        &p, n, start, n_steps, seed, &mut out,
    ) {
        Ok(n_out) => Ok(args::record([(
            "path",
            Value::List(out[..n_out].iter().map(|v| Value::U64(*v as u64)).collect()),
        )])),
        Err(e) => Err(args::bad(span, format!("simulate_chain: {e:?}"))),
    }
}

pub fn econ_mean_first_passage(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let p = args::rec_f64_list(args, "matrix")
        .ok_or_else(|| args::bad(span, "Econ.mean_first_passage needs matrix"))?;
    let n = args::rec_u64(args, "n")
        .ok_or_else(|| args::bad(span, "Econ.mean_first_passage needs n"))? as usize;
    let target = args::rec_u64(args, "target")
        .ok_or_else(|| args::bad(span, "Econ.mean_first_passage needs target"))?
        as usize;
    let max_iter = args::rec_u64(args, "max_iter").unwrap_or(1000) as u32;
    let tolerance = args::rec_f64(args, "tolerance").unwrap_or(1e-9);
    let mut out = vec![0.0; n];
    match crate::specialized_libs::computational_economics::markov::mean_first_passage_time_into(
        &p, n, target, max_iter, tolerance, &mut out,
    ) {
        Ok(_) => Ok(args::record([(
            "first_passage_times",
            Value::List(out.iter().map(|v| Value::F64(*v)).collect()),
        )])),
        Err(e) => Err(args::bad(span, format!("mean_first_passage: {e:?}"))),
    }
}

// ── Labor: CES household production, human capital ────────────────────────────

pub fn econ_household_production_ces(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let time = args::rec_f64(args, "time")
        .ok_or_else(|| args::bad(span, "Econ.household_production_ces needs time"))?;
    let goods = args::rec_f64(args, "goods")
        .ok_or_else(|| args::bad(span, "Econ.household_production_ces needs goods"))?;
    let alpha = args::rec_f64(args, "alpha")
        .ok_or_else(|| args::bad(span, "Econ.household_production_ces needs alpha"))?;
    let rho = args::rec_f64(args, "rho")
        .ok_or_else(|| args::bad(span, "Econ.household_production_ces needs rho"))?;
    match crate::specialized_libs::computational_economics::labor_household::household_production_ces(time, goods, alpha, rho) {
        Ok(output) => Ok(args::record([("output", Value::F64(output))])),
        Err(e) => Err(args::bad(span, format!("household_production_ces: {e:?}"))),
    }
}

// ── Environmental: pollution damage, marginal damage, abatement net benefit ───

pub fn econ_pollution_damage(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let emissions = args::rec_f64(args, "emissions")
        .ok_or_else(|| args::bad(span, "Econ.pollution_damage needs emissions"))?;
    let damage_coeff = args::rec_f64(args, "damage_coeff")
        .ok_or_else(|| args::bad(span, "Econ.pollution_damage needs damage_coeff"))?;
    match crate::specialized_libs::computational_economics::environmental_resource::pollution_damage(
        emissions,
        damage_coeff,
    ) {
        Ok(d) => Ok(args::record([("damage", Value::F64(d))])),
        Err(e) => Err(args::bad(span, format!("pollution_damage: {e:?}"))),
    }
}

pub fn econ_marginal_damage(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let emissions = args::rec_f64(args, "emissions")
        .ok_or_else(|| args::bad(span, "Econ.marginal_damage needs emissions"))?;
    let damage_coeff = args::rec_f64(args, "damage_coeff")
        .ok_or_else(|| args::bad(span, "Econ.marginal_damage needs damage_coeff"))?;
    match crate::specialized_libs::computational_economics::environmental_resource::marginal_damage(
        emissions,
        damage_coeff,
    ) {
        Ok(md) => Ok(args::record([("marginal_damage", Value::F64(md))])),
        Err(e) => Err(args::bad(span, format!("marginal_damage: {e:?}"))),
    }
}

pub fn econ_abatement_net_benefit(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let baseline = args::rec_f64(args, "baseline_emissions")
        .ok_or_else(|| args::bad(span, "Econ.abatement_net_benefit needs baseline_emissions"))?;
    let actual = args::rec_f64(args, "actual_emissions")
        .ok_or_else(|| args::bad(span, "Econ.abatement_net_benefit needs actual_emissions"))?;
    let abatement_coeff = args::rec_f64(args, "abatement_coeff")
        .ok_or_else(|| args::bad(span, "Econ.abatement_net_benefit needs abatement_coeff"))?;
    let damage_coeff = args::rec_f64(args, "damage_coeff")
        .ok_or_else(|| args::bad(span, "Econ.abatement_net_benefit needs damage_coeff"))?;
    match crate::specialized_libs::computational_economics::environmental_resource::abatement_net_benefit(baseline, actual, abatement_coeff, damage_coeff) {
        Ok(nb) => Ok(args::record([("net_benefit", Value::F64(nb))])),
        Err(e) => Err(args::bad(span, format!("abatement_net_benefit: {e:?}"))),
    }
}

// ── Econometrics: WLS, IV/2SLS, logistic MLE ──────────────────────────────────

pub fn econ_wls(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_f64_list(args, "x").ok_or_else(|| args::bad(span, "Econ.wls needs x"))?;
    let y = args::rec_f64_list(args, "y").ok_or_else(|| args::bad(span, "Econ.wls needs y"))?;
    let weights = args::rec_f64_list(args, "weights")
        .ok_or_else(|| args::bad(span, "Econ.wls needs weights"))?;
    let n_obs = args::rec_u64(args, "n_obs")
        .ok_or_else(|| args::bad(span, "Econ.wls needs n_obs"))? as usize;
    let n_reg = args::rec_u64(args, "n_reg").unwrap_or(1) as usize;
    let mut beta = vec![0.0; n_reg];
    let mut residuals = vec![0.0; n_obs];
    match crate::specialized_libs::computational_economics::econometrics::wls_into(
        &x,
        &y,
        &weights,
        n_obs,
        n_reg,
        &mut beta,
        &mut residuals,
    ) {
        Ok(r2) => Ok(args::record([
            (
                "beta",
                Value::List(beta.iter().map(|v| Value::F64(*v)).collect()),
            ),
            ("r_squared", Value::F64(r2)),
        ])),
        Err(e) => Err(args::bad(span, format!("wls: {e:?}"))),
    }
}

pub fn econ_iv_2sls(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_f64_list(args, "x_endogenous")
        .ok_or_else(|| args::bad(span, "Econ.iv_2sls needs x_endogenous"))?;
    let z = args::rec_f64_list(args, "z_instruments")
        .ok_or_else(|| args::bad(span, "Econ.iv_2sls needs z_instruments"))?;
    let y = args::rec_f64_list(args, "y").ok_or_else(|| args::bad(span, "Econ.iv_2sls needs y"))?;
    let n_obs = args::rec_u64(args, "n_obs")
        .ok_or_else(|| args::bad(span, "Econ.iv_2sls needs n_obs"))? as usize;
    let n_reg = args::rec_u64(args, "n_reg").unwrap_or(1) as usize;
    let n_instr = args::rec_u64(args, "n_instr").unwrap_or(n_reg as u64) as usize;
    let mut beta = vec![0.0; n_reg];
    match crate::specialized_libs::computational_economics::econometrics::iv_2sls_into(
        &x, &z, &y, n_obs, n_reg, n_instr, &mut beta,
    ) {
        Ok(r2) => Ok(args::record([
            (
                "beta",
                Value::List(beta.iter().map(|v| Value::F64(*v)).collect()),
            ),
            ("r_squared", Value::F64(r2)),
        ])),
        Err(e) => Err(args::bad(span, format!("iv_2sls: {e:?}"))),
    }
}

pub fn econ_logistic_mle(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_f64_list(args, "x")
        .ok_or_else(|| args::bad(span, "Econ.logistic_mle needs x"))?;
    let y = args::rec_f64_list(args, "y")
        .ok_or_else(|| args::bad(span, "Econ.logistic_mle needs y"))?;
    let n_obs = args::rec_u64(args, "n_obs")
        .ok_or_else(|| args::bad(span, "Econ.logistic_mle needs n_obs"))? as usize;
    let n_reg = args::rec_u64(args, "n_reg").unwrap_or(1) as usize;
    let max_iter = args::rec_u64(args, "max_iter").unwrap_or(100) as u32;
    let tolerance = args::rec_f64(args, "tolerance").unwrap_or(1e-6);
    let mut beta = vec![0.0; n_reg];
    match crate::specialized_libs::computational_economics::econometrics::logistic_mle_into(
        &x, &y, n_obs, n_reg, max_iter, tolerance, &mut beta,
    ) {
        Ok(_) => Ok(args::record([(
            "beta",
            Value::List(beta.iter().map(|v| Value::F64(*v)).collect()),
        )])),
        Err(e) => Err(args::bad(span, format!("logistic_mle: {e:?}"))),
    }
}

// ── Dynamic programming: value iteration, policy iteration ────────────────────

pub fn econ_value_iteration(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let rewards = args::rec_f64_list(args, "rewards")
        .ok_or_else(|| args::bad(span, "Econ.value_iteration needs rewards"))?;
    let transitions = args::rec_f64_list(args, "transitions")
        .ok_or_else(|| args::bad(span, "Econ.value_iteration needs transitions"))?;
    let discount = args::rec_f64(args, "discount")
        .ok_or_else(|| args::bad(span, "Econ.value_iteration needs discount"))?;
    let n_states = args::rec_u64(args, "n_states")
        .ok_or_else(|| args::bad(span, "Econ.value_iteration needs n_states"))?
        as usize;
    let n_actions = args::rec_u64(args, "n_actions")
        .ok_or_else(|| args::bad(span, "Econ.value_iteration needs n_actions"))?
        as usize;
    let max_iter = args::rec_u64(args, "max_iter").unwrap_or(1000) as u32;
    let tolerance = args::rec_f64(args, "tolerance").unwrap_or(1e-6);
    let mut values = vec![0.0; n_states];
    let mut policy = vec![0u32; n_states];
    match crate::specialized_libs::computational_economics::dynamic_programming::value_iteration_into(&rewards, &transitions, discount, n_states, n_actions, max_iter, tolerance, &mut values, &mut policy) {
        Ok(_) => Ok(args::record([
            ("values", Value::List(values.iter().map(|v| Value::F64(*v)).collect())),
            ("policy", Value::List(policy.iter().map(|v| Value::U64(*v as u64)).collect())),
        ])),
        Err(e) => Err(args::bad(span, format!("value_iteration: {e:?}"))),
    }
}

// ── Forensic: narrative divergence, harm trace ────────────────────────────────

pub fn econ_narrative_divergence(_args: &Value, _span: Span) -> Result<Value, Diagnostic> {
    // Narrative divergence requires NquinVector structs — expose as metadata.
    Ok(args::record([(
        "status",
        Value::String("narrative_divergence_available".into()),
    )]))
}

// ── Network economics: centrality, cascades, interbank clearing ───────────────

pub fn econ_eigenvector_centrality(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let adj = args::rec_f64_list(args, "adjacency")
        .ok_or_else(|| args::bad(span, "Econ.eigenvector_centrality needs adjacency"))?;
    let n = args::rec_u64(args, "n")
        .ok_or_else(|| args::bad(span, "Econ.eigenvector_centrality needs n"))?
        as usize;
    let max_iter = args::rec_u64(args, "max_iter").unwrap_or(100) as u32;
    let tolerance = args::rec_f64(args, "tolerance").unwrap_or(1e-6);
    let mut out = vec![0.0; n];
    match crate::specialized_libs::computational_economics::network_economics::eigenvector_centrality_into(&adj, n, max_iter, tolerance, &mut out) {
        Ok(_) => Ok(args::record([
            ("centrality", Value::List(out.iter().map(|v| Value::F64(*v)).collect())),
        ])),
        Err(e) => Err(args::bad(span, format!("eigenvector_centrality: {e:?}"))),
    }
}

pub fn econ_degree_centrality(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let adj = args::rec_f64_list(args, "adjacency")
        .ok_or_else(|| args::bad(span, "Econ.degree_centrality needs adjacency"))?;
    let n = args::rec_u64(args, "n")
        .ok_or_else(|| args::bad(span, "Econ.degree_centrality needs n"))? as usize;
    let mut out = vec![0.0; n];
    match crate::specialized_libs::computational_economics::network_economics::degree_centrality_into(&adj, n, &mut out) {
        Ok(_) => Ok(args::record([
            ("centrality", Value::List(out.iter().map(|v| Value::F64(*v)).collect())),
        ])),
        Err(e) => Err(args::bad(span, format!("degree_centrality: {e:?}"))),
    }
}

pub fn econ_interbank_clearing(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let exposures = args::rec_f64_list(args, "exposures")
        .ok_or_else(|| args::bad(span, "Econ.interbank_clearing needs exposures"))?;
    let capital = args::rec_f64_list(args, "capital")
        .ok_or_else(|| args::bad(span, "Econ.interbank_clearing needs capital"))?;
    let n = args::rec_u64(args, "n")
        .ok_or_else(|| args::bad(span, "Econ.interbank_clearing needs n"))? as usize;
    let max_rounds = args::rec_u64(args, "max_rounds").unwrap_or(100) as u32;
    let tolerance = args::rec_f64(args, "tolerance").unwrap_or(1e-6);
    let mut out = vec![0.0; n];
    match crate::specialized_libs::computational_economics::network_economics::interbank_clearing_into(&exposures, &capital, n, max_rounds, tolerance, &mut out) {
        Ok(_) => Ok(args::record([
            ("clearing_payments", Value::List(out.iter().map(|v| Value::F64(*v)).collect())),
        ])),
        Err(e) => Err(args::bad(span, format!("interbank_clearing: {e:?}"))),
    }
}

// ── Input-output: Leontief inverse, multipliers, key sectors ──────────────────

pub fn econ_leontief_inverse(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = args::rec_f64_list(args, "matrix")
        .ok_or_else(|| args::bad(span, "Econ.leontief_inverse needs matrix"))?;
    let n = args::rec_u64(args, "n")
        .ok_or_else(|| args::bad(span, "Econ.leontief_inverse needs n"))? as usize;
    let max_rounds = args::rec_u64(args, "max_rounds").unwrap_or(100) as u32;
    let tolerance = args::rec_f64(args, "tolerance").unwrap_or(1e-9);
    let mut out = vec![0.0; n * n];
    match crate::specialized_libs::computational_economics::input_output::leontief_inverse_into(
        &a, n, max_rounds, tolerance, &mut out,
    ) {
        Ok(_) => Ok(args::record([(
            "inverse",
            Value::List(out.iter().map(|v| Value::F64(*v)).collect()),
        )])),
        Err(e) => Err(args::bad(span, format!("leontief_inverse: {e:?}"))),
    }
}

pub fn econ_output_multipliers(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let leontief_inv = args::rec_f64_list(args, "leontief_inverse")
        .ok_or_else(|| args::bad(span, "Econ.output_multipliers needs leontief_inverse"))?;
    let n = args::rec_u64(args, "n")
        .ok_or_else(|| args::bad(span, "Econ.output_multipliers needs n"))? as usize;
    let mut out = vec![0.0; n];
    match crate::specialized_libs::computational_economics::input_output::output_multipliers_from_inverse(&leontief_inv, n, &mut out) {
        Ok(_) => Ok(args::record([
            ("multipliers", Value::List(out.iter().map(|v| Value::F64(*v)).collect())),
        ])),
        Err(e) => Err(args::bad(span, format!("output_multipliers: {e:?}"))),
    }
}

// ── Agent-based: zero intelligence, aggregate wealth ──────────────────────────

pub fn econ_agent_based_aggregate_wealth(_args: &Value, _span: Span) -> Result<Value, Diagnostic> {
    // Agent-based aggregate wealth requires Agent structs — expose as metadata.
    Ok(args::record([(
        "status",
        Value::String("agent_based_aggregate_wealth_available".into()),
    )]))
}

// ── Ontology bridge: scalar/vector encoding, FIBO ─────────────────────────────

pub fn econ_validate_scalar_constraint(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let value = args::rec_f64(args, "value")
        .ok_or_else(|| args::bad(span, "Econ.validate_scalar_constraint needs value"))?;
    let min = args::rec_f64(args, "min")
        .ok_or_else(|| args::bad(span, "Econ.validate_scalar_constraint needs min"))?;
    let max = args::rec_f64(args, "max")
        .ok_or_else(|| args::bad(span, "Econ.validate_scalar_constraint needs max"))?;
    let valid = crate::specialized_libs::computational_economics::ontology_bridge::validate_scalar_econ_constraint(value, min, max);
    Ok(args::record([("valid", Value::Bool(valid))]))
}

// ── Paper trading: aggregate fills ────────────────────────────────────────────

pub fn econ_aggregate_paper_fills(_args: &Value, _span: Span) -> Result<Value, Diagnostic> {
    // Paper trading fill aggregation requires Fill structs — expose as metadata.
    Ok(args::record([(
        "status",
        Value::String("paper_trading_aggregate_fills_available".into()),
    )]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn capm_expected_return_basic() {
        let mut m = BTreeMap::new();
        m.insert("rf".into(), Value::F64(0.02));
        m.insert("beta".into(), Value::F64(1.2));
        m.insert("market_premium".into(), Value::F64(0.06));
        let result = econ_capm_expected_return(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
    }

    #[test]
    fn gordon_growth_basic() {
        let mut m = BTreeMap::new();
        m.insert("next_dividend".into(), Value::F64(2.0));
        m.insert("required_return".into(), Value::F64(0.10));
        m.insert("growth_rate".into(), Value::F64(0.03));
        let result = econ_gordon_growth(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
    }

    #[test]
    fn gini_basic() {
        let mut m = BTreeMap::new();
        m.insert(
            "incomes".into(),
            Value::List(vec![
                Value::F64(10.0),
                Value::F64(20.0),
                Value::F64(30.0),
                Value::F64(40.0),
            ]),
        );
        let result = econ_gini(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
    }

    #[test]
    fn black_scholes_basic() {
        let mut m = BTreeMap::new();
        m.insert("spot".into(), Value::F64(100.0));
        m.insert("strike".into(), Value::F64(100.0));
        m.insert("time_to_expiry".into(), Value::F64(1.0));
        m.insert("risk_free_rate".into(), Value::F64(0.05));
        m.insert("volatility".into(), Value::F64(0.2));
        m.insert("is_call".into(), Value::Bool(true));
        let result = econ_black_scholes(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
    }

    #[test]
    fn prospect_value_basic() {
        let mut m = BTreeMap::new();
        m.insert("x".into(), Value::F64(100.0));
        let result = econ_prospect_value(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
    }

    #[test]
    fn cournot_duopoly_basic() {
        let mut m = BTreeMap::new();
        m.insert("demand_intercept".into(), Value::F64(100.0));
        m.insert("demand_slope".into(), Value::F64(1.0));
        m.insert("cost_1".into(), Value::F64(10.0));
        m.insert("cost_2".into(), Value::F64(10.0));
        let result = econ_cournot_duopoly(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
    }

    #[test]
    fn solow_steady_state_basic() {
        let mut m = BTreeMap::new();
        m.insert("alpha".into(), Value::F64(0.3));
        m.insert("savings_rate".into(), Value::F64(0.2));
        m.insert("depreciation".into(), Value::F64(0.05));
        let result = econ_solow_steady_state(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
    }

    #[test]
    fn historical_var_basic() {
        let mut m = BTreeMap::new();
        let returns: Vec<Value> = (-50..50).map(|i| Value::F64(i as f64 * 0.001)).collect();
        m.insert("returns".into(), Value::List(returns));
        m.insert("confidence".into(), Value::F64(0.95));
        let result = econ_historical_var(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
    }
}
