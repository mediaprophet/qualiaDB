//! Wave-6 CAS / constructibility Host binds over specialized_libs pure helpers.
//!
//! Host-missing `pub fn`s not bound in waves 1–5:
//! `partial`, `jacobian`, `hessian`, `gradient_at`, `hessian_at`,
//! `is_regular_polygon_constructible`, `is_fermat_prime`,
//! `constructible_from_min_poly_degree`.

use super::super::args;
use crate::specialized_libs::constructibility as constr;
use crate::specialized_libs::multivar_calculus as mvc;
use crate::specialized_libs::symbolic_algebra as sa;
use crate::specialized_libs::symbolic_algebra::Expr;
use std::collections::HashMap;
use vibe::{Diagnostic, Span, Value};

fn parse_expr(s: &str, span: Span) -> Result<Expr, Diagnostic> {
    sa::parse(s).map_err(|e| args::bad(span, format!("parse error: {e}")))
}

fn vars_from(args_v: &Value, span: Span) -> Result<Vec<String>, Diagnostic> {
    args::rec_str_list(args_v, "vars")
        .ok_or_else(|| args::bad(span, "needs vars: [string]"))
}

fn point_from(args_v: &Value, span: Span) -> Result<HashMap<String, f64>, Diagnostic> {
    let Some(point) = args::rec(args_v, "point") else {
        return Err(args::bad(span, "needs point: { var: number, ... }"));
    };
    let Value::Record(m) = point else {
        return Err(args::bad(span, "point must be a record of number fields"));
    };
    let mut out = HashMap::new();
    for (k, v) in m {
        let n = args::as_f64(v).ok_or_else(|| {
            args::bad(span, format!("point.{k} must be a number"))
        })?;
        out.insert(k.clone(), n);
    }
    Ok(out)
}

fn expr_matrix(rows: &[Vec<Expr>]) -> Value {
    Value::List(
        rows.iter()
            .map(|row| {
                Value::List(
                    row.iter()
                        .map(|e| Value::String(e.to_string()))
                        .collect(),
                )
            })
            .collect(),
    )
}

fn f64_matrix(rows: &[Vec<f64>]) -> Value {
    Value::List(
        rows.iter()
            .map(|row| args::f64_list_value(row.clone()))
            .collect(),
    )
}

/// `SymbolicAlgebra.partial` — simplified ∂expr/∂var.
/// Args: `{ expr, var }`. Out: `{ partial }`.
pub fn partial(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let expr = args::rec_str(args_v, "expr")
        .ok_or_else(|| args::bad(span, "SymbolicAlgebra.partial needs expr: string"))?;
    let var = args::rec_str(args_v, "var")
        .ok_or_else(|| args::bad(span, "SymbolicAlgebra.partial needs var: string"))?;
    let e = parse_expr(expr, span)?;
    let d = mvc::partial(&e, var);
    Ok(args::record([("partial", Value::String(d.to_string()))]))
}

/// `SymbolicAlgebra.jacobian` — symbolic Jacobian of a vector of expressions.
/// Args: `{ exprs: [string], vars: [string] }`. Out: `{ jacobian: [[string]] }`.
pub fn jacobian(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let exprs = args::rec_str_list(args_v, "exprs")
        .ok_or_else(|| args::bad(span, "SymbolicAlgebra.jacobian needs exprs: [string]"))?;
    let vars = vars_from(args_v, span)?;
    let mut parsed = Vec::with_capacity(exprs.len());
    for s in &exprs {
        parsed.push(parse_expr(s, span)?);
    }
    let var_refs: Vec<&str> = vars.iter().map(|s| s.as_str()).collect();
    let j = mvc::jacobian(&parsed, &var_refs);
    Ok(args::record([("jacobian", expr_matrix(&j))]))
}

/// `SymbolicAlgebra.hessian` — symbolic Hessian matrix of second partials.
/// Args: `{ expr, vars }`. Out: `{ hessian: [[string]] }`.
pub fn hessian(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let expr = args::rec_str(args_v, "expr")
        .ok_or_else(|| args::bad(span, "SymbolicAlgebra.hessian needs expr: string"))?;
    let vars = vars_from(args_v, span)?;
    let e = parse_expr(expr, span)?;
    let var_refs: Vec<&str> = vars.iter().map(|s| s.as_str()).collect();
    let h = mvc::hessian(&e, &var_refs);
    Ok(args::record([("hessian", expr_matrix(&h))]))
}

/// `SymbolicAlgebra.gradient_at` — numeric gradient at a point.
/// Args: `{ expr, vars, point }`. Out: `{ gradient: [f64] }` or error if undefined.
pub fn gradient_at(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let expr = args::rec_str(args_v, "expr")
        .ok_or_else(|| args::bad(span, "SymbolicAlgebra.gradient_at needs expr: string"))?;
    let vars = vars_from(args_v, span)?;
    let point = point_from(args_v, span)?;
    let e = parse_expr(expr, span)?;
    let var_refs: Vec<&str> = vars.iter().map(|s| s.as_str()).collect();
    let g = mvc::gradient_at(&e, &var_refs, &point)
        .ok_or_else(|| args::bad(span, "gradient_at: expression undefined at point"))?;
    Ok(args::record([("gradient", args::f64_list_value(g))]))
}

/// `SymbolicAlgebra.hessian_at` — numeric Hessian at a point.
/// Args: `{ expr, vars, point }`. Out: `{ hessian: [[f64]] }` or error if undefined.
pub fn hessian_at(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let expr = args::rec_str(args_v, "expr")
        .ok_or_else(|| args::bad(span, "SymbolicAlgebra.hessian_at needs expr: string"))?;
    let vars = vars_from(args_v, span)?;
    let point = point_from(args_v, span)?;
    let e = parse_expr(expr, span)?;
    let var_refs: Vec<&str> = vars.iter().map(|s| s.as_str()).collect();
    let h = mvc::hessian_at(&e, &var_refs, &point)
        .ok_or_else(|| args::bad(span, "hessian_at: expression undefined at point"))?;
    Ok(args::record([("hessian", f64_matrix(&h))]))
}

/// `Constructibility.is_regular_polygon_constructible` — Gauss–Wantzel test.
/// Args: `{ n }`. Out: `{ value: bool }`.
pub fn is_regular_polygon_constructible(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let n = args::rec_u64(args_v, "n").ok_or_else(|| {
        args::bad(span, "Constructibility.is_regular_polygon_constructible needs n")
    })?;
    Ok(args::record([(
        "value",
        Value::Bool(constr::is_regular_polygon_constructible(n)),
    )]))
}

/// `Constructibility.is_fermat_prime` — prime of form `2^(2^k)+1`.
/// Args: `{ n }`. Out: `{ value: bool }`.
pub fn is_fermat_prime(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let n = args::rec_u64(args_v, "n")
        .ok_or_else(|| args::bad(span, "Constructibility.is_fermat_prime needs n"))?;
    Ok(args::record([(
        "value",
        Value::Bool(constr::is_fermat_prime(n)),
    )]))
}

/// `Constructibility.constructible_from_min_poly_degree` — Wantzel degree gate.
/// Args: `{ degree }`. Out: `{ value: bool }`.
pub fn constructible_from_min_poly_degree(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let degree = args::rec_u64(args_v, "degree").ok_or_else(|| {
        args::bad(
            span,
            "Constructibility.constructible_from_min_poly_degree needs degree",
        )
    })?;
    Ok(args::record([(
        "value",
        Value::Bool(constr::constructible_from_min_poly_degree(degree)),
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
    fn wave6_partial_of_x2() {
        let mut m = BTreeMap::new();
        m.insert("expr".into(), Value::String("x^2".into()));
        m.insert("var".into(), Value::String("x".into()));
        let out = partial(&Value::Record(m), span()).unwrap();
        let p = args::rec_str(&out, "partial").unwrap();
        assert!(p.contains('x') || p.contains('2'), "got {p}");
    }

    #[test]
    fn wave6_jacobian_xy_sum() {
        let mut m = BTreeMap::new();
        m.insert(
            "exprs".into(),
            Value::List(vec![
                Value::String("x*y".into()),
                Value::String("x + y".into()),
            ]),
        );
        m.insert(
            "vars".into(),
            Value::List(vec![Value::String("x".into()), Value::String("y".into())]),
        );
        let out = jacobian(&Value::Record(m), span()).unwrap();
        let Value::List(rows) = args::rec(&out, "jacobian").unwrap() else {
            panic!("expected list");
        };
        assert_eq!(rows.len(), 2);
        let Value::List(r0) = &rows[0] else {
            panic!("row0");
        };
        assert_eq!(r0.len(), 2);
    }

    #[test]
    fn wave6_hessian_quadratic_constant() {
        let mut m = BTreeMap::new();
        m.insert("expr".into(), Value::String("x^2 + x*y + y^2".into()));
        m.insert(
            "vars".into(),
            Value::List(vec![Value::String("x".into()), Value::String("y".into())]),
        );
        let out = hessian(&Value::Record(m), span()).unwrap();
        let Value::List(rows) = args::rec(&out, "hessian").unwrap() else {
            panic!("expected list");
        };
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn wave6_gradient_at_3x2() {
        let mut m = BTreeMap::new();
        m.insert("expr".into(), Value::String("3*x^2".into()));
        m.insert("vars".into(), Value::List(vec![Value::String("x".into())]));
        let mut pt = BTreeMap::new();
        pt.insert("x".into(), Value::F64(4.0));
        m.insert("point".into(), Value::Record(pt));
        let out = gradient_at(&Value::Record(m), span()).unwrap();
        let g = args::rec(&out, "gradient").and_then(args::f64s).unwrap();
        assert_eq!(g.len(), 1);
        assert!((g[0] - 24.0).abs() < 1e-9);
    }

    #[test]
    fn wave6_hessian_at_quadratic() {
        let mut m = BTreeMap::new();
        m.insert("expr".into(), Value::String("x^2 + x*y + y^2".into()));
        m.insert(
            "vars".into(),
            Value::List(vec![Value::String("x".into()), Value::String("y".into())]),
        );
        let mut pt = BTreeMap::new();
        pt.insert("x".into(), Value::F64(0.0));
        pt.insert("y".into(), Value::F64(0.0));
        m.insert("point".into(), Value::Record(pt));
        let out = hessian_at(&Value::Record(m), span()).unwrap();
        let Value::List(rows) = args::rec(&out, "hessian").unwrap() else {
            panic!("expected list");
        };
        let r0 = args::f64s(&rows[0]).unwrap();
        let r1 = args::f64s(&rows[1]).unwrap();
        assert!((r0[0] - 2.0).abs() < 1e-9);
        assert!((r0[1] - 1.0).abs() < 1e-9);
        assert!((r1[0] - 1.0).abs() < 1e-9);
        assert!((r1[1] - 2.0).abs() < 1e-9);
    }

    #[test]
    fn wave6_regular_17_gon_constructible() {
        let mut m17 = BTreeMap::new();
        m17.insert("n".into(), Value::U64(17));
        let out = is_regular_polygon_constructible(&Value::Record(m17), span()).unwrap();
        assert_eq!(args::rec_bool(&out, "value"), Some(true));
        let mut m7 = BTreeMap::new();
        m7.insert("n".into(), Value::U64(7));
        let out7 = is_regular_polygon_constructible(&Value::Record(m7), span()).unwrap();
        assert_eq!(args::rec_bool(&out7, "value"), Some(false));
    }

    #[test]
    fn wave6_fermat_prime_17() {
        let mut m17 = BTreeMap::new();
        m17.insert("n".into(), Value::U64(17));
        let out = is_fermat_prime(&Value::Record(m17), span()).unwrap();
        assert_eq!(args::rec_bool(&out, "value"), Some(true));
        let mut m9 = BTreeMap::new();
        m9.insert("n".into(), Value::U64(9));
        let out9 = is_fermat_prime(&Value::Record(m9), span()).unwrap();
        assert_eq!(args::rec_bool(&out9, "value"), Some(false));
    }

    #[test]
    fn wave6_wantzel_degree_gate() {
        let mut m4 = BTreeMap::new();
        m4.insert("degree".into(), Value::U64(4));
        let out = constructible_from_min_poly_degree(&Value::Record(m4), span()).unwrap();
        assert_eq!(args::rec_bool(&out, "value"), Some(true));
        let mut m3 = BTreeMap::new();
        m3.insert("degree".into(), Value::U64(3));
        let out3 = constructible_from_min_poly_degree(&Value::Record(m3), span()).unwrap();
        assert_eq!(args::rec_bool(&out3, "value"), Some(false));
    }
}
