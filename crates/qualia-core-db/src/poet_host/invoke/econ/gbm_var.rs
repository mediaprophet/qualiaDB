//! Bounded deterministic GBM path and Monte Carlo Value-at-Risk composition.

use super::super::args;
use crate::domains::financial::economics::{
    run_monte_carlo_var_seeded_into, simulate_gbm_steps_into,
};
use vibe::{Diagnostic, Span, Value};

const MAX_STEPS: usize = 4096;
const MAX_PATHS: usize = 4096;

pub fn simulate(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let s0 = args::rec_f64(args_v, "s0").ok_or_else(|| args::bad(span, "gbm_var needs s0"))?;
    let drift = args::rec_f64(args_v, "mu").ok_or_else(|| args::bad(span, "gbm_var needs mu"))?;
    let volatility =
        args::rec_f64(args_v, "sigma").ok_or_else(|| args::bad(span, "gbm_var needs sigma"))?;
    let time_horizon = args::rec_f64(args_v, "time_horizon")
        .ok_or_else(|| args::bad(span, "gbm_var needs time_horizon"))?;
    let dt = args::rec_f64(args_v, "dt").ok_or_else(|| args::bad(span, "gbm_var needs dt"))?;
    let portfolio_value = args::rec_f64(args_v, "portfolio_value")
        .ok_or_else(|| args::bad(span, "gbm_var needs portfolio_value"))?;
    let confidence = args::rec_f64(args_v, "confidence")
        .ok_or_else(|| args::bad(span, "gbm_var needs confidence"))?;
    let paths = args::rec_u64(args_v, "paths").unwrap_or(2048) as usize;
    let seed = args::rec_u64(args_v, "seed").unwrap_or(42);
    if !(s0 > 0.0
        && volatility >= 0.0
        && time_horizon > 0.0
        && dt > 0.0
        && portfolio_value >= 0.0
        && (0.5..1.0).contains(&confidence))
        || paths == 0
        || paths > MAX_PATHS
    {
        return Err(args::bad(span, "invalid or out-of-bounds GBM/VaR inputs"));
    }
    let steps = (time_horizon / dt).ceil() as usize;
    if steps == 0 || steps > MAX_STEPS {
        return Err(args::bad(
            span,
            format!("GBM steps must be between 1 and {MAX_STEPS}"),
        ));
    }

    let mut path = vec![0.0; steps];
    let written =
        simulate_gbm_steps_into(s0, drift, volatility, time_horizon, steps, seed, &mut path)
            .map_err(|error| args::bad(span, format!("GBM path failed: {error:?}")))?;

    let mut final_prices = vec![0.0; paths];
    let (_, mean_final_price, _) = run_monte_carlo_var_seeded_into(
        s0,
        drift,
        volatility,
        time_horizon,
        steps,
        paths,
        seed,
        &mut final_prices,
    )
    .map_err(|error| args::bad(span, format!("Monte Carlo VaR failed: {error:?}")))?;
    let quantile_index = ((paths as f64 * (1.0 - confidence)).floor() as usize).min(paths - 1);
    let price_var = (s0 - final_prices[quantile_index]).max(0.0);
    let portfolio_var = price_var / s0 * portfolio_value;

    Ok(args::record([
        (
            "path",
            Value::List(path[..written].iter().copied().map(Value::F64).collect()),
        ),
        ("steps", Value::U64(written as u64)),
        ("paths", Value::U64(paths as u64)),
        ("seed", Value::U64(seed)),
        ("confidence", Value::F64(confidence)),
        ("mean_final_price", Value::F64(mean_final_price)),
        ("price_var", Value::F64(price_var)),
        ("portfolio_var", Value::F64(portfolio_var)),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn seeded_gbm_var_is_reproducible_and_bounded() {
        let args = Value::Record(BTreeMap::from([
            ("s0".into(), Value::F64(100.0)),
            ("mu".into(), Value::F64(0.05)),
            ("sigma".into(), Value::F64(0.2)),
            ("time_horizon".into(), Value::F64(1.0)),
            ("dt".into(), Value::F64(0.01)),
            ("portfolio_value".into(), Value::F64(1_000_000.0)),
            ("confidence".into(), Value::F64(0.95)),
            ("paths".into(), Value::U64(256)),
            ("seed".into(), Value::U64(42)),
        ]));
        let first = simulate(&args, Span::new(0, 0)).unwrap();
        let second = simulate(&args, Span::new(0, 0)).unwrap();
        assert_eq!(first, second);
        let Value::Record(result) = first else {
            panic!("expected GBM/VaR record");
        };
        assert_eq!(result.get("steps"), Some(&Value::U64(100)));
        assert!(
            matches!(result.get("portfolio_var"), Some(Value::F64(value)) if value.is_finite())
        );
    }
}
