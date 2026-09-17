//! Historical VaR / Sharpe from a price series — collab finance start.

use super::super::args;
use crate::specialized_libs::financial_modeling::portfolio_risk::compute_risk_metrics;
use crate::specialized_libs::financial_modeling::{
    Asset, AssetType, InvestmentStrategy, LiquidityNeeds, Portfolio, RiskProfile, RiskTolerance,
    TimeHorizon,
};
use vibe::{Diagnostic, Span, Value};

pub fn risk(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let prices = args::rec(args_v, "prices")
        .and_then(args::f64s)
        .or_else(|| args::f64s(args_v))
        .ok_or_else(|| args::bad(span, "FinancialModeling.portfolio_risk needs prices"))?;
    if prices.len() < 3 {
        return Err(args::bad(span, "need ≥ 3 prices"));
    }
    let last = *prices.last().unwrap();
    let asset = Asset {
        asset_id: "a0".into(),
        symbol: args::rec_str(args_v, "symbol").unwrap_or("PORT").into(),
        asset_type: AssetType::Stock,
        quantity: 1.0,
        average_cost: prices[0],
        current_price: last,
        market_value: last,
        currency: "AUD".into(),
        exchange: "local".into(),
        last_updated: 0,
        price_history: prices,
    };
    let portfolio = Portfolio {
        portfolio_id: "vibe".into(),
        portfolio_name: "vibe".into(),
        owner_id: "local".into(),
        assets: vec![asset],
        cash_balance: 0.0,
        total_value: last,
        created_at: 0,
        last_updated: 0,
        risk_profile: RiskProfile {
            risk_tolerance: RiskTolerance::Moderate,
            risk_capacity: 1.0,
            time_horizon: TimeHorizon::MediumTerm,
            liquidity_needs: LiquidityNeeds::Medium,
        },
        investment_strategy: InvestmentStrategy::Balanced,
    };
    let m = compute_risk_metrics(&portfolio, None)
        .map_err(|e| args::bad(span, format!("portfolio_risk: {e:?}")))?;
    Ok(args::record([
        ("var_95", Value::F64(m.var_95)),
        ("cvar_95", Value::F64(m.cvar_95)),
        ("volatility", Value::F64(m.volatility)),
        ("sharpe", Value::F64(m.sharpe_ratio)),
        ("sortino", Value::F64(m.sortino_ratio)),
        ("max_drawdown", Value::F64(m.max_drawdown)),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rising_series_has_finite_vol() {
        let args = args::f64_list_value([100.0, 101.0, 102.0, 103.0, 104.0]);
        match risk(&args, Span { start: 0, end: 0 }).unwrap() {
            Value::Record(r) => match r.get("volatility") {
                Some(Value::F64(v)) => assert!(v.is_finite() && *v >= 0.0),
                other => panic!("{other:?}"),
            },
            other => panic!("{other:?}"),
        }
    }
}
