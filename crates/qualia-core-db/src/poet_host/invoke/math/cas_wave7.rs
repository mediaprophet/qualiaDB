//! Wave-7 Host binds: remaining constructibility + CAS pure helpers.
//!
//! Host-missing `pub fn`s not bound in waves 1–6:
//! `is_power_of_two`, `is_central_angle_constructible`,
//! `doubling_the_cube_constructible`, `trisecting_general_angle_constructible`,
//! `squaring_the_circle_constructible`, `is_constructible_number`,
//! `solve_quadratic_symbolic`, `factor_quadratic`.

use super::super::args;
use crate::specialized_libs::constructibility as constr;
use crate::specialized_libs::constructibility::ConstructibilityVerdict;
use crate::specialized_libs::symbolic_algebra as sa;
use vibe::{Diagnostic, Span, Value};

fn parse_expr(s: &str, span: Span) -> Result<sa::Expr, Diagnostic> {
    sa::parse(s).map_err(|e| args::bad(span, format!("parse error: {e}")))
}

fn verdict_record(v: ConstructibilityVerdict) -> Value {
    match v {
        ConstructibilityVerdict::Constructible { degree_bound } => args::record([
            ("verdict", Value::String("constructible".into())),
            ("degree_bound", Value::U64(degree_bound)),
            ("value", Value::Bool(true)),
        ]),
        ConstructibilityVerdict::NotRealNumber => args::record([
            ("verdict", Value::String("not_real".into())),
            ("value", Value::Bool(false)),
        ]),
        ConstructibilityVerdict::Undefined => args::record([
            ("verdict", Value::String("undefined".into())),
            ("value", Value::Bool(false)),
        ]),
        ConstructibilityVerdict::Transcendental => args::record([
            ("verdict", Value::String("transcendental".into())),
            ("value", Value::Bool(false)),
        ]),
    }
}

/// `Constructibility.is_power_of_two` — `n ≥ 1` and exactly one bit set.
/// Args: `{ n }`. Out: `{ value: bool }`.
pub fn is_power_of_two(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let n = args::rec_u64(args_v, "n")
        .ok_or_else(|| args::bad(span, "Constructibility.is_power_of_two needs n"))?;
    Ok(args::record([(
        "value",
        Value::Bool(constr::is_power_of_two(n)),
    )]))
}

/// `Constructibility.is_central_angle_constructible` — `2π/n` via Gauss–Wantzel.
/// Args: `{ n }`. Out: `{ value: bool }`.
pub fn is_central_angle_constructible(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let n = args::rec_u64(args_v, "n").ok_or_else(|| {
        args::bad(span, "Constructibility.is_central_angle_constructible needs n")
    })?;
    Ok(args::record([(
        "value",
        Value::Bool(constr::is_central_angle_constructible(n)),
    )]))
}

/// `Constructibility.doubling_the_cube_constructible` — classical impossibility (false).
/// Args: `{}`. Out: `{ value: bool }`.
pub fn doubling_the_cube_constructible(
    _args_v: &Value,
    _span: Span,
) -> Result<Value, Diagnostic> {
    Ok(args::record([(
        "value",
        Value::Bool(constr::doubling_the_cube_constructible()),
    )]))
}

/// `Constructibility.trisecting_general_angle_constructible` — classical impossibility.
/// Args: `{}`. Out: `{ value: bool }`.
pub fn trisecting_general_angle_constructible(
    _args_v: &Value,
    _span: Span,
) -> Result<Value, Diagnostic> {
    Ok(args::record([(
        "value",
        Value::Bool(constr::trisecting_general_angle_constructible()),
    )]))
}

/// `Constructibility.squaring_the_circle_constructible` — classical impossibility.
/// Args: `{}`. Out: `{ value: bool }`.
pub fn squaring_the_circle_constructible(
    _args_v: &Value,
    _span: Span,
) -> Result<Value, Diagnostic> {
    Ok(args::record([(
        "value",
        Value::Bool(constr::squaring_the_circle_constructible()),
    )]))
}

/// `Constructibility.is_constructible_number` — CAS expression constructibility verdict.
/// Args: `{ expr }`. Out: `{ verdict, value, degree_bound? }`.
pub fn is_constructible_number(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let expr = args::rec_str(args_v, "expr").ok_or_else(|| {
        args::bad(span, "Constructibility.is_constructible_number needs expr: string")
    })?;
    let e = parse_expr(expr, span)?;
    Ok(verdict_record(constr::is_constructible_number(&e)))
}

/// `SymbolicAlgebra.solve_quadratic_symbolic` — exact `Expr` roots of `a x² + b x + c`.
/// Args: `{ a, b, c }`. Out: `{ roots: [string] }`.
pub fn solve_quadratic_symbolic(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = args::rec_f64(args_v, "a").ok_or_else(|| {
        args::bad(span, "SymbolicAlgebra.solve_quadratic_symbolic needs a")
    })?;
    let b = args::rec_f64(args_v, "b").ok_or_else(|| {
        args::bad(span, "SymbolicAlgebra.solve_quadratic_symbolic needs b")
    })?;
    let c = args::rec_f64(args_v, "c").ok_or_else(|| {
        args::bad(span, "SymbolicAlgebra.solve_quadratic_symbolic needs c")
    })?;
    let roots = sa::solve_quadratic_symbolic(a, b, c);
    Ok(args::record([(
        "roots",
        Value::List(
            roots
                .into_iter()
                .map(|e| Value::String(e.to_string()))
                .collect(),
        ),
    )]))
}

/// `SymbolicAlgebra.factor_quadratic` — `a (x − r₁)(x − r₂)` when real roots exist.
/// Args: `{ a, b, c, var? }`. Out: `{ factor }` or error when no real factorisation.
pub fn factor_quadratic(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = args::rec_f64(args_v, "a")
        .ok_or_else(|| args::bad(span, "SymbolicAlgebra.factor_quadratic needs a"))?;
    let b = args::rec_f64(args_v, "b")
        .ok_or_else(|| args::bad(span, "SymbolicAlgebra.factor_quadratic needs b"))?;
    let c = args::rec_f64(args_v, "c")
        .ok_or_else(|| args::bad(span, "SymbolicAlgebra.factor_quadratic needs c"))?;
    let var = args::rec_str(args_v, "var").unwrap_or("x");
    let factored = sa::factor_quadratic(a, b, c, var).ok_or_else(|| {
        args::bad(span, "factor_quadratic: no real factorisation")
    })?;
    Ok(args::record([(
        "factor",
        Value::String(factored.to_string()),
    )]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn span() -> Span {
        Span { start: 0, end: 0 }
    }

    #[test]
    fn wave7_is_power_of_two() {
        let mut m8 = BTreeMap::new();
        m8.insert("n".into(), Value::U64(8));
        let out = is_power_of_two(&Value::Record(m8), span()).unwrap();
        assert_eq!(args::rec_bool(&out, "value"), Some(true));
        let mut m6 = BTreeMap::new();
        m6.insert("n".into(), Value::U64(6));
        let out6 = is_power_of_two(&Value::Record(m6), span()).unwrap();
        assert_eq!(args::rec_bool(&out6, "value"), Some(false));
    }

    #[test]
    fn wave7_central_angle_hexagon() {
        let mut m6 = BTreeMap::new();
        m6.insert("n".into(), Value::U64(6));
        let out = is_central_angle_constructible(&Value::Record(m6), span()).unwrap();
        assert_eq!(args::rec_bool(&out, "value"), Some(true));
        let mut m9 = BTreeMap::new();
        m9.insert("n".into(), Value::U64(9));
        let out9 = is_central_angle_constructible(&Value::Record(m9), span()).unwrap();
        assert_eq!(args::rec_bool(&out9, "value"), Some(false));
    }

    #[test]
    fn wave7_classical_impossibilities_are_false() {
        let empty = Value::Record(BTreeMap::new());
        assert_eq!(
            args::rec_bool(
                &doubling_the_cube_constructible(&empty, span()).unwrap(),
                "value"
            ),
            Some(false)
        );
        assert_eq!(
            args::rec_bool(
                &trisecting_general_angle_constructible(&empty, span()).unwrap(),
                "value"
            ),
            Some(false)
        );
        assert_eq!(
            args::rec_bool(
                &squaring_the_circle_constructible(&empty, span()).unwrap(),
                "value"
            ),
            Some(false)
        );
    }

    #[test]
    fn wave7_constructible_sqrt2() {
        let mut m = BTreeMap::new();
        m.insert("expr".into(), Value::String("sqrt(2)".into()));
        let out = is_constructible_number(&Value::Record(m), span()).unwrap();
        assert_eq!(args::rec_bool(&out, "value"), Some(true));
        assert_eq!(args::rec_str(&out, "verdict"), Some("constructible"));
        assert_eq!(args::rec_u64(&out, "degree_bound"), Some(2));
    }

    #[test]
    fn wave7_transcendental_exp() {
        let mut m = BTreeMap::new();
        m.insert("expr".into(), Value::String("exp(1)".into()));
        let out = is_constructible_number(&Value::Record(m), span()).unwrap();
        assert_eq!(args::rec_bool(&out, "value"), Some(false));
        assert_eq!(args::rec_str(&out, "verdict"), Some("transcendental"));
    }

    #[test]
    fn wave7_solve_quadratic_symbolic_x2_minus_1() {
        let mut m = BTreeMap::new();
        m.insert("a".into(), Value::F64(1.0));
        m.insert("b".into(), Value::F64(0.0));
        m.insert("c".into(), Value::F64(-1.0));
        let out = solve_quadratic_symbolic(&Value::Record(m), span()).unwrap();
        let Value::List(roots) = args::rec(&out, "roots").unwrap() else {
            panic!("expected roots list");
        };
        assert_eq!(roots.len(), 2);
    }

    #[test]
    fn wave7_factor_quadratic_x2_minus_1() {
        let mut m = BTreeMap::new();
        m.insert("a".into(), Value::F64(1.0));
        m.insert("b".into(), Value::F64(0.0));
        m.insert("c".into(), Value::F64(-1.0));
        m.insert("var".into(), Value::String("x".into()));
        let out = factor_quadratic(&Value::Record(m), span()).unwrap();
        let f = args::rec_str(&out, "factor").unwrap();
        assert!(f.contains('x'), "got {f}");
    }
}
