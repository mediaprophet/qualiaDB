//! Deterministic stdlib kernels for LocalHost / vibe-wasm.
//!
//! Poet overrides `Host::capability_invoke` to reach the engine. When Vibe
//! runs under `LocalHost` (CLI fixtures, WASM interpreter, Node CI), these
//! pure kernels supply the Civics-facing subset: welfare, OLS, and units.
//! Formulas match `qualia-core-db` welfare / simple OLS / quantity convert.

use std::collections::BTreeMap;

use crate::error::{DiagCode, Diagnostic};
use crate::quantity::{lookup_unit, Unit};
use crate::span::Span;
use crate::value::Value;

/// Attempt a local stdlib invoke. Returns `None` when `id` is not handled here.
pub fn try_invoke(id: &str, args: &Value, span: Span) -> Option<Result<Value, Diagnostic>> {
    match id {
        "Econ.gini" => Some(econ_gini(args, span)),
        "Econ.utilitarian_welfare" | "Econ.welfare" => Some(econ_utilitarian_welfare(args, span)),
        "Econ.atkinson" => Some(econ_atkinson(args, span)),
        "Statistics.linear_regression" | "Statistics.ols" => Some(stats_ols(args, span)),
        "Statistics.mean" => Some(stats_mean(args, span)),
        "Statistics.median" => Some(stats_median(args, span)),
        "Statistics.variance" => Some(stats_variance(args, span)),
        "Statistics.std_dev" => Some(stats_std_dev(args, span)),
        "Statistics.pearson" => Some(stats_pearson(args, span)),
        "PhysicalUnits.convert" => Some(units_convert(args, span)),
        "SymbolicLogic.sat" | "SymbolicAndDefeasibleLogic.sat" => Some(sat_solve(args, span)),
        _ => None,
    }
}

fn econ_gini(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let incomes = f64_list(args, "incomes")
        .ok_or_else(|| Diagnostic::new(DiagCode::E100, span, "Econ.gini needs incomes: [f64]"))?;
    let g = gini_coefficient(&incomes).map_err(|e| Diagnostic::new(DiagCode::E100, span, e))?;
    Ok(record([("gini", Value::F64(g))]))
}

fn econ_utilitarian_welfare(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let utilities = f64_list(args, "utilities")
        .or_else(|| f64_list(args, "incomes"))
        .ok_or_else(|| {
            Diagnostic::new(
                DiagCode::E100,
                span,
                "Econ.utilitarian_welfare needs utilities: [f64]",
            )
        })?;
    if utilities.is_empty() {
        return Err(Diagnostic::new(
            DiagCode::E100,
            span,
            "utilitarian_welfare: empty input",
        ));
    }
    if utilities.iter().any(|v| !v.is_finite()) {
        return Err(Diagnostic::new(
            DiagCode::E100,
            span,
            "utilitarian_welfare: non-finite value",
        ));
    }
    let w: f64 = utilities.iter().sum();
    Ok(record([("welfare", Value::F64(w))]))
}

fn econ_atkinson(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let incomes = f64_list(args, "incomes").ok_or_else(|| {
        Diagnostic::new(DiagCode::E100, span, "Econ.atkinson needs incomes: [f64]")
    })?;
    let epsilon = f64_field(args, "epsilon").ok_or_else(|| {
        Diagnostic::new(DiagCode::E100, span, "Econ.atkinson needs epsilon: f64")
    })?;
    let a = atkinson_inequality(&incomes, epsilon)
        .map_err(|e| Diagnostic::new(DiagCode::E100, span, e))?;
    Ok(record([("atkinson", Value::F64(a))]))
}

fn stats_ols(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = f64_list(args, "x")
        .ok_or_else(|| Diagnostic::new(DiagCode::E100, span, "OLS needs x: [f64]"))?;
    let y = f64_list(args, "y")
        .ok_or_else(|| Diagnostic::new(DiagCode::E100, span, "OLS needs y: [f64]"))?;
    if x.len() != y.len() {
        return Err(Diagnostic::new(
            DiagCode::E100,
            span,
            "OLS: x and y length mismatch",
        ));
    }
    let fit = simple_ols(&x, &y).ok_or_else(|| {
        Diagnostic::new(
            DiagCode::E100,
            span,
            "OLS: degenerate input (n < 3 or zero variance)",
        )
    })?;
    Ok(record([
        ("slope", Value::F64(fit.slope)),
        ("intercept", Value::F64(fit.intercept)),
        ("r_squared", Value::F64(fit.r_squared)),
        ("residual_std_error", Value::F64(fit.residual_std_error)),
        ("n", Value::U64(fit.n as u64)),
    ]))
}

fn units_convert(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let value = f64_field(args, "value").ok_or_else(|| {
        Diagnostic::new(
            DiagCode::E100,
            span,
            "PhysicalUnits.convert needs value: f64",
        )
    })?;
    let from_name = string_field(args, "from").ok_or_else(|| {
        Diagnostic::new(
            DiagCode::E100,
            span,
            "PhysicalUnits.convert needs from: unit name",
        )
    })?;
    let to_name = string_field(args, "to").ok_or_else(|| {
        Diagnostic::new(
            DiagCode::E100,
            span,
            "PhysicalUnits.convert needs to: unit name",
        )
    })?;
    let from = resolve_unit(from_name).ok_or_else(|| {
        Diagnostic::new(
            DiagCode::E100,
            span,
            format!("unknown unit: {from_name}"),
        )
    })?;
    let to = resolve_unit(to_name).ok_or_else(|| {
        Diagnostic::new(DiagCode::E100, span, format!("unknown unit: {to_name}"))
    })?;
    let converted = from
        .convert(value, &to)
        .map_err(|e| Diagnostic::new(DiagCode::E100, span, e))?;
    Ok(record([
        ("value", Value::F64(converted)),
        ("from", Value::String(from.symbol.clone())),
        ("to", Value::String(to.symbol.clone())),
    ]))
}

fn stats_mean(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let xs = f64_list(args, "values")
        .or_else(|| f64_list(args, "x"))
        .ok_or_else(|| Diagnostic::new(DiagCode::E100, span, "Statistics.mean needs values: [f64]"))?;
    if xs.is_empty() {
        return Err(Diagnostic::new(DiagCode::E100, span, "Statistics.mean: empty"));
    }
    let m = xs.iter().sum::<f64>() / xs.len() as f64;
    Ok(record([("mean", Value::F64(m)), ("n", Value::U64(xs.len() as u64))]))
}

fn stats_median(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let mut xs = f64_list(args, "values")
        .or_else(|| f64_list(args, "x"))
        .ok_or_else(|| {
            Diagnostic::new(DiagCode::E100, span, "Statistics.median needs values: [f64]")
        })?;
    if xs.is_empty() {
        return Err(Diagnostic::new(DiagCode::E100, span, "Statistics.median: empty"));
    }
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = xs.len();
    let med = if n % 2 == 1 {
        xs[n / 2]
    } else {
        (xs[n / 2 - 1] + xs[n / 2]) / 2.0
    };
    Ok(record([("median", Value::F64(med)), ("n", Value::U64(n as u64))]))
}

fn stats_variance(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let xs = f64_list(args, "values")
        .or_else(|| f64_list(args, "x"))
        .ok_or_else(|| {
            Diagnostic::new(DiagCode::E100, span, "Statistics.variance needs values: [f64]")
        })?;
    if xs.len() < 2 {
        return Err(Diagnostic::new(
            DiagCode::E100,
            span,
            "Statistics.variance needs n ≥ 2",
        ));
    }
    let mean = xs.iter().sum::<f64>() / xs.len() as f64;
    let var = xs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (xs.len() - 1) as f64;
    Ok(record([
        ("variance", Value::F64(var)),
        ("n", Value::U64(xs.len() as u64)),
    ]))
}

fn stats_std_dev(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let v = stats_variance(args, span)?;
    let Value::Record(m) = &v else {
        return Ok(v);
    };
    let var = match m.get("variance") {
        Some(Value::F64(x)) => *x,
        _ => 0.0,
    };
    Ok(record([
        ("std_dev", Value::F64(var.sqrt())),
        ("n", m.get("n").cloned().unwrap_or(Value::U64(0))),
    ]))
}

fn stats_pearson(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = f64_list(args, "x")
        .ok_or_else(|| Diagnostic::new(DiagCode::E100, span, "Statistics.pearson needs x: [f64]"))?;
    let y = f64_list(args, "y")
        .ok_or_else(|| Diagnostic::new(DiagCode::E100, span, "Statistics.pearson needs y: [f64]"))?;
    if x.len() != y.len() || x.len() < 2 {
        return Err(Diagnostic::new(
            DiagCode::E100,
            span,
            "Statistics.pearson: length mismatch or n < 2",
        ));
    }
    let n = x.len() as f64;
    let mx = x.iter().sum::<f64>() / n;
    let my = y.iter().sum::<f64>() / n;
    let mut num = 0.0;
    let mut dx = 0.0;
    let mut dy = 0.0;
    for i in 0..x.len() {
        let a = x[i] - mx;
        let b = y[i] - my;
        num += a * b;
        dx += a * a;
        dy += b * b;
    }
    if dx == 0.0 || dy == 0.0 {
        return Err(Diagnostic::new(
            DiagCode::E100,
            span,
            "Statistics.pearson: zero variance",
        ));
    }
    Ok(record([
        ("r", Value::F64(num / (dx * dy).sqrt())),
        ("n", Value::U64(x.len() as u64)),
    ]))
}

/// Tiny bounded DPLL for Civics LocalHost (mirrors `solve_sat_wasm` clause shape).
fn sat_solve(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let clauses = i32_list_of_lists(args, "clauses").ok_or_else(|| {
        Diagnostic::new(
            DiagCode::E100,
            span,
            "SymbolicLogic.sat needs clauses: [[i32],…] (signed literals)",
        )
    })?;
    if clauses.len() > 32 {
        return Err(Diagnostic::new(
            DiagCode::E400,
            span,
            "SymbolicLogic.sat: >32 clauses — use solve_sat_wasm / Poet",
        ));
    }
    let (sat, assignment) = dpll_sat(&clauses);
    let mut assign_rec = BTreeMap::new();
    for (k, v) in assignment {
        assign_rec.insert(k.to_string(), Value::Bool(v));
    }
    Ok(record([
        ("satisfiable", Value::Bool(sat)),
        ("assignment", Value::Record(assign_rec)),
        ("honesty", Value::String("local-bounded-dpll".into())),
    ]))
}

fn i32_list_of_lists(args: &Value, key: &str) -> Option<Vec<Vec<i32>>> {
    let Value::Record(map) = args else {
        return None;
    };
    let Value::List(outer) = map.get(key)? else {
        return None;
    };
    let mut out = Vec::new();
    for clause in outer {
        let Value::List(lits) = clause else {
            return None;
        };
        let mut c = Vec::new();
        for lit in lits {
            match lit {
                Value::I64(n) => c.push(*n as i32),
                Value::U64(n) => c.push(*n as i32),
                Value::F64(n) if n.fract() == 0.0 => c.push(*n as i32),
                _ => return None,
            }
        }
        out.push(c);
    }
    Some(out)
}

fn dpll_sat(clauses: &[Vec<i32>]) -> (bool, BTreeMap<i32, bool>) {
    let mut assignment = BTreeMap::new();
    let mut vars = BTreeMap::new();
    for c in clauses {
        for &lit in c {
            vars.insert(lit.unsigned_abs() as i32, ());
        }
    }
    fn sat(
        clauses: &[Vec<i32>],
        vars: &[i32],
        assignment: &mut BTreeMap<i32, bool>,
    ) -> bool {
        // Unit / conflict check
        for c in clauses {
            let mut unresolved = 0;
            let mut satisfied = false;
            for &lit in c {
                let v = lit.unsigned_abs() as i32;
                match assignment.get(&v) {
                    Some(true) if lit > 0 => {
                        satisfied = true;
                        break;
                    }
                    Some(false) if lit < 0 => {
                        satisfied = true;
                        break;
                    }
                    Some(_) => {}
                    None => unresolved += 1,
                }
            }
            if !satisfied && unresolved == 0 {
                return false;
            }
        }
        if let Some(&v) = vars.iter().find(|v| !assignment.contains_key(v)) {
            assignment.insert(v, true);
            if sat(clauses, vars, assignment) {
                return true;
            }
            assignment.insert(v, false);
            if sat(clauses, vars, assignment) {
                return true;
            }
            assignment.remove(&v);
            false
        } else {
            true
        }
    }
    let var_list: Vec<i32> = vars.keys().copied().collect();
    let ok = sat(clauses, &var_list, &mut assignment);
    (ok, assignment)
}

fn resolve_unit(name: &str) -> Option<Unit> {
    if let Some(u) = lookup_unit(name) {
        return Some(u);
    }
    // ASCII aliases aligned with poet_host PhysicalUnits.convert.
    match name {
        "C" | "celsius" => lookup_unit("degC"),
        "F" | "fahrenheit" => lookup_unit("degF"),
        "metre" | "meter" => lookup_unit("m"),
        "kilometre" | "kilometer" => lookup_unit("km"),
        "sec" | "second" => lookup_unit("s"),
        _ => None,
    }
}

/// Relative mean absolute difference Gini (matches core welfare::gini_coefficient).
fn gini_coefficient(incomes: &[f64]) -> Result<f64, &'static str> {
    let n = incomes.len();
    if n == 0 {
        return Err("gini: empty incomes");
    }
    if incomes.iter().any(|v| !v.is_finite() || *v < 0.0) {
        return Err("gini: non-finite or negative income");
    }
    let mut sum = 0.0f64;
    let mut total = 0.0f64;
    for &yi in incomes {
        total += yi;
        for &yj in incomes {
            sum += (yi - yj).abs();
        }
    }
    if total <= 0.0 {
        return Err("gini: all-zero incomes");
    }
    let mean = total / n as f64;
    let denom = 2.0 * (n as f64).powi(2) * mean;
    Ok(sum / denom)
}

fn atkinson_inequality(incomes: &[f64], epsilon: f64) -> Result<f64, &'static str> {
    let n = incomes.len();
    if n == 0 {
        return Err("atkinson: empty incomes");
    }
    if !epsilon.is_finite() || epsilon <= 0.0 {
        return Err("atkinson: epsilon must be finite and > 0");
    }
    if incomes.iter().any(|v| !v.is_finite() || *v <= 0.0) {
        return Err("atkinson: incomes must be strictly positive");
    }
    let arithmetic_mean: f64 = incomes.iter().sum::<f64>() / n as f64;
    if (epsilon - 1.0).abs() < 1e-15 {
        let log_mean = incomes.iter().map(|v| v.ln()).sum::<f64>() / n as f64;
        let geometric_mean = log_mean.exp();
        return Ok(1.0 - geometric_mean / arithmetic_mean);
    }
    let mean_power: f64 = incomes.iter().map(|v| v.powf(1.0 - epsilon)).sum::<f64>() / n as f64;
    let equally_distributed = mean_power.powf(1.0 / (1.0 - epsilon));
    Ok(1.0 - equally_distributed / arithmetic_mean)
}

struct OlsFit {
    slope: f64,
    intercept: f64,
    r_squared: f64,
    residual_std_error: f64,
    n: usize,
}

fn simple_ols(x: &[f64], y: &[f64]) -> Option<OlsFit> {
    let n = x.len();
    if n != y.len() || n < 3 {
        return None;
    }
    if x.iter().chain(y.iter()).any(|v| !v.is_finite()) {
        return None;
    }
    let mx = x.iter().sum::<f64>() / n as f64;
    let my = y.iter().sum::<f64>() / n as f64;
    let mut sxx = 0.0f64;
    let mut sxy = 0.0f64;
    for i in 0..n {
        let dx = x[i] - mx;
        sxx += dx * dx;
        sxy += dx * (y[i] - my);
    }
    if sxx <= 0.0 {
        return None;
    }
    let slope = sxy / sxx;
    let intercept = my - slope * mx;
    let mut ss_tot = 0.0f64;
    let mut ss_res = 0.0f64;
    for i in 0..n {
        let yhat = intercept + slope * x[i];
        ss_tot += (y[i] - my).powi(2);
        ss_res += (y[i] - yhat).powi(2);
    }
    let r_squared = if ss_tot > 0.0 {
        1.0 - ss_res / ss_tot
    } else {
        1.0
    };
    let df = (n - 2) as f64;
    let residual_std_error = (ss_res / df).sqrt();
    Some(OlsFit {
        slope,
        intercept,
        r_squared,
        residual_std_error,
        n,
    })
}

fn record(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    let mut rec = BTreeMap::new();
    for (k, v) in fields {
        rec.insert(k.into(), v);
    }
    Value::Record(rec)
}

fn string_field<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    match args {
        Value::Record(map) => match map.get(key) {
            Some(Value::String(s)) => Some(s.as_str()),
            _ => None,
        },
        _ => None,
    }
}

fn f64_field(args: &Value, key: &str) -> Option<f64> {
    match args {
        Value::Record(map) => match map.get(key) {
            Some(Value::F64(n)) => Some(*n),
            Some(Value::I64(n)) => Some(*n as f64),
            Some(Value::U64(n)) => Some(*n as f64),
            Some(Value::Quantity(q)) => Some(q.value),
            _ => None,
        },
        _ => None,
    }
}

fn f64_list(args: &Value, key: &str) -> Option<Vec<f64>> {
    match args {
        Value::Record(map) => match map.get(key) {
            Some(Value::List(items)) => {
                let mut out = Vec::with_capacity(items.len());
                for item in items {
                    match item {
                        Value::F64(n) => out.push(*n),
                        Value::I64(n) => out.push(*n as f64),
                        Value::U64(n) => out.push(*n as f64),
                        Value::Quantity(q) => out.push(q.value),
                        _ => return None,
                    }
                }
                Some(out)
            }
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::span::Span;

    fn empty_span() -> Span {
        Span::new(0, 0)
    }

    #[test]
    fn gini_two_person_split_is_half() {
        let args = record([("incomes", Value::List(vec![Value::F64(0.0), Value::F64(1.0)]))]);
        let v = econ_gini(&args, empty_span()).unwrap();
        match v {
            Value::Record(m) => {
                let g = match m.get("gini") {
                    Some(Value::F64(g)) => *g,
                    _ => panic!("missing gini"),
                };
                assert!((g - 0.5).abs() < 1e-12);
            }
            _ => panic!("expected record"),
        }
    }

    #[test]
    fn utilitarian_sums() {
        let args = record([(
            "utilities",
            Value::List(vec![Value::F64(1.0), Value::F64(2.0), Value::F64(3.0)]),
        )]);
        let v = econ_utilitarian_welfare(&args, empty_span()).unwrap();
        match v {
            Value::Record(m) => match m.get("welfare") {
                Some(Value::F64(w)) => assert!((*w - 6.0).abs() < 1e-12),
                _ => panic!("missing welfare"),
            },
            _ => panic!("expected record"),
        }
    }

    #[test]
    fn ols_perfect_line() {
        let args = record([
            (
                "x",
                Value::List(vec![Value::F64(1.0), Value::F64(2.0), Value::F64(3.0)]),
            ),
            (
                "y",
                Value::List(vec![Value::F64(2.0), Value::F64(4.0), Value::F64(6.0)]),
            ),
        ]);
        let v = stats_ols(&args, empty_span()).unwrap();
        match v {
            Value::Record(m) => {
                assert!((match m.get("slope") {
                    Some(Value::F64(s)) => *s,
                    _ => panic!(),
                } - 2.0)
                    .abs()
                    < 1e-12);
                assert!((match m.get("r_squared") {
                    Some(Value::F64(r)) => *r,
                    _ => panic!(),
                } - 1.0)
                    .abs()
                    < 1e-12);
            }
            _ => panic!("expected record"),
        }
    }

    #[test]
    fn celsius_to_fahrenheit() {
        let args = record([
            ("value", Value::F64(100.0)),
            ("from", Value::String("°C".into())),
            ("to", Value::String("°F".into())),
        ]);
        let v = units_convert(&args, empty_span()).unwrap();
        match v {
            Value::Record(m) => {
                let f = match m.get("value") {
                    Some(Value::F64(f)) => *f,
                    _ => panic!("missing value"),
                };
                assert!((f - 212.0).abs() < 1e-9);
            }
            _ => panic!("expected record"),
        }
    }

    #[test]
    fn mean_median_pearson_and_sat() {
        let mean_args = record([(
            "values",
            Value::List(vec![Value::F64(1.0), Value::F64(2.0), Value::F64(3.0)]),
        )]);
        let mv = stats_mean(&mean_args, empty_span()).unwrap();
        let Value::Record(m) = mv else { panic!() };
        assert_eq!(m.get("mean"), Some(&Value::F64(2.0)));

        let pearson_args = record([
            (
                "x",
                Value::List(vec![Value::F64(1.0), Value::F64(2.0), Value::F64(3.0)]),
            ),
            (
                "y",
                Value::List(vec![Value::F64(2.0), Value::F64(4.0), Value::F64(6.0)]),
            ),
        ]);
        let pv = stats_pearson(&pearson_args, empty_span()).unwrap();
        let Value::Record(pm) = pv else { panic!() };
        let r = match pm.get("r") {
            Some(Value::F64(r)) => *r,
            _ => panic!(),
        };
        assert!((r - 1.0).abs() < 1e-12);

        // (x ∨ y) ∧ (¬x ∨ y) is satisfiable with y=true
        let sat_args = record([(
            "clauses",
            Value::List(vec![
                Value::List(vec![Value::I64(1), Value::I64(2)]),
                Value::List(vec![Value::I64(-1), Value::I64(2)]),
            ]),
        )]);
        let sv = sat_solve(&sat_args, empty_span()).unwrap();
        let Value::Record(sm) = sv else { panic!() };
        assert_eq!(sm.get("satisfiable"), Some(&Value::Bool(true)));
    }
}
