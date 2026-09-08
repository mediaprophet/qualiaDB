//! Wave-5 CAS / ODE / LinAlg Host binds over specialized_libs pure helpers.
//!
//! Host-missing `pub fn`s not bound in waves 1–4:
//! `solve_separable`, `solve_first_order_linear_pde`, `solve_polynomial_expr`,
//! `simplify_with_assumptions`, `roots`, `solve_linear_system`, `expr_citation_hash`.

use super::super::args;
use crate::specialized_libs::symbolic_algebra as sa;
use crate::specialized_libs::symbolic_assumptions::{Assumptions, Sign};
use crate::specialized_libs::symbolic_ode::{
    solve_first_order_linear_pde as ode_pde1, solve_separable as ode_sep, OdeSolution,
    PdeSolution,
};
use crate::specialized_libs::symbolic_solve::{
    roots as complex_roots, solve_linear_system as lin_solve, solve_polynomial_expr as poly_expr,
};
use vibe::{Diagnostic, Span, Value};

fn ode_value(sol: &OdeSolution) -> Value {
    match sol {
        OdeSolution::Explicit(e) => args::record([
            ("kind", Value::String("explicit".into())),
            ("y", Value::String(e.to_string())),
        ]),
        OdeSolution::Implicit { f_y, g_x } => args::record([
            ("kind", Value::String("implicit".into())),
            ("f_y", Value::String(f_y.to_string())),
            ("g_x", Value::String(g_x.to_string())),
        ]),
    }
}

fn parse_sign(s: &str) -> Option<Sign> {
    match s.trim().to_ascii_lowercase().as_str() {
        "positive" | "pos" | ">" => Some(Sign::Positive),
        "nonnegative" | "non_negative" | "non-negative" | ">=" => Some(Sign::NonNegative),
        "negative" | "neg" | "<" => Some(Sign::Negative),
        "nonpositive" | "non_positive" | "non-positive" | "<=" => Some(Sign::NonPositive),
        "nonzero" | "non_zero" | "non-zero" | "!=" => Some(Sign::Nonzero),
        _ => None,
    }
}

fn assumptions_from(args_v: &Value, span: Span) -> Result<Assumptions, Diagnostic> {
    let mut asm = Assumptions::new();
    let Some(list) = args::rec(args_v, "assumptions") else {
        return Ok(asm);
    };
    let Value::List(items) = list else {
        return Err(args::bad(
            span,
            "assumptions must be a list of { var, sign } records",
        ));
    };
    for item in items {
        let var = args::rec_str(item, "var")
            .ok_or_else(|| args::bad(span, "each assumption needs var: string"))?;
        let sign_s = args::rec_str(item, "sign")
            .ok_or_else(|| args::bad(span, "each assumption needs sign: string"))?;
        let sign = parse_sign(sign_s).ok_or_else(|| {
            args::bad(
                span,
                format!(
                    "unknown sign '{sign_s}' (use positive/nonnegative/negative/nonpositive/nonzero)"
                ),
            )
        })?;
        asm = asm.assume(var, sign);
    }
    Ok(asm)
}

/// `SymbolicODE.solve_separable` — `y' = g(x)·h(y)` → implicit ∫dy/h = ∫g dx + C.
/// Args: `{ g, h, x?, y? }`. Out: `{ kind, f_y, g_x }` or error.
pub fn solve_separable(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let g = args::rec_str(args_v, "g")
        .ok_or_else(|| args::bad(span, "SymbolicODE.solve_separable needs g: string"))?;
    let h = args::rec_str(args_v, "h")
        .ok_or_else(|| args::bad(span, "SymbolicODE.solve_separable needs h: string"))?;
    let xvar = args::rec_str(args_v, "x").unwrap_or("x");
    let yvar = args::rec_str(args_v, "y").unwrap_or("y");
    let g_e = sa::parse(g).map_err(|e| args::bad(span, format!("parse g: {e}")))?;
    let h_e = sa::parse(h).map_err(|e| args::bad(span, format!("parse h: {e}")))?;
    let sol = ode_sep(&g_e, &h_e, xvar, yvar)
        .map_err(|e| args::bad(span, format!("solve_separable: {e:?}")))?;
    Ok(ode_value(&sol))
}

/// `SymbolicODE.solve_first_order_linear_pde` — `a·uₓ + b·u_y = 0` characteristics.
/// Args: `{ a, b, x?, y? }`. Out: `{ kind, invariant }`.
pub fn solve_first_order_linear_pde(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = args::rec_f64(args_v, "a")
        .ok_or_else(|| args::bad(span, "solve_first_order_linear_pde needs a: number"))?;
    let b = args::rec_f64(args_v, "b")
        .ok_or_else(|| args::bad(span, "solve_first_order_linear_pde needs b: number"))?;
    let xvar = args::rec_str(args_v, "x").unwrap_or("x");
    let yvar = args::rec_str(args_v, "y").unwrap_or("y");
    let sol = ode_pde1(a, b, xvar, yvar)
        .map_err(|e| args::bad(span, format!("solve_first_order_linear_pde: {e:?}")))?;
    match sol {
        PdeSolution::GeneralFunctionOf { invariant } => Ok(args::record([
            ("kind", Value::String("general_function_of".into())),
            ("invariant", Value::String(invariant.to_string())),
        ])),
    }
}

/// `SymbolicAlgebra.solve_polynomial_expr` — real roots of a CAS polynomial expression.
/// Args: `{ expr, degree, var?, tol? }`. Out: `{ roots }`.
pub fn solve_polynomial_expr(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let expr = args::rec_str(args_v, "expr")
        .ok_or_else(|| args::bad(span, "SymbolicAlgebra.solve_polynomial_expr needs expr"))?;
    let degree = args::rec_u64(args_v, "degree")
        .ok_or_else(|| args::bad(span, "solve_polynomial_expr needs degree: number"))?
        as usize;
    let var = args::rec_str(args_v, "var").unwrap_or("x");
    let tol = args::rec_f64(args_v, "tol").unwrap_or(1e-8);
    let e = sa::parse(expr).map_err(|e| args::bad(span, format!("parse error: {e}")))?;
    let roots = poly_expr(&e, var, degree, tol)
        .map_err(|e| args::bad(span, format!("solve_polynomial_expr: {e:?}")))?;
    Ok(args::record([("roots", args::f64_list_value(roots))]))
}

/// `SymbolicAlgebra.simplify_with_assumptions` — sign-gated CAS rewrites.
/// Args: `{ expr, assumptions? }` where assumptions is `[{ var, sign }, ...]`.
/// Out: `{ simplified }`.
pub fn simplify_with_assumptions(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let expr = args::rec_str(args_v, "expr").ok_or_else(|| {
        args::bad(span, "SymbolicAlgebra.simplify_with_assumptions needs expr")
    })?;
    let asm = assumptions_from(args_v, span)?;
    let e = sa::parse(expr).map_err(|e| args::bad(span, format!("parse error: {e}")))?;
    let s = crate::specialized_libs::symbolic_assumptions::simplify_with_assumptions(&e, &asm);
    Ok(args::record([("simplified", Value::String(s.to_string()))]))
}

/// `SymbolicAlgebra.roots` — all complex roots (descending coeffs).
/// Args: `{ coeffs }`. Out: `{ roots: [{ re, im }, ...] }`.
pub fn roots(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let coeffs = args::rec_f64_list(args_v, "coeffs")
        .ok_or_else(|| args::bad(span, "SymbolicAlgebra.roots needs coeffs: [f64]"))?;
    let zs = complex_roots(&coeffs).map_err(|e| args::bad(span, format!("roots: {e:?}")))?;
    let list: Vec<Value> = zs
        .into_iter()
        .map(|z| args::record([("re", Value::F64(z.re)), ("im", Value::F64(z.im))]))
        .collect();
    Ok(args::record([("roots", Value::List(list))]))
}

/// `LinearAlgebra.solve_linear_system` — Gaussian elimination with partial pivoting.
/// Args: `{ a, b, n }` (row-major `a`). Out: `{ x }` or error if singular.
pub fn solve_linear_system(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = args::rec_f64_list(args_v, "a")
        .ok_or_else(|| args::bad(span, "solve_linear_system needs a: [f64]"))?;
    let b = args::rec_f64_list(args_v, "b")
        .ok_or_else(|| args::bad(span, "solve_linear_system needs b: [f64]"))?;
    let n = args::rec_u64(args_v, "n")
        .ok_or_else(|| args::bad(span, "solve_linear_system needs n: number"))?
        as usize;
    let x = lin_solve(&a, &b, n)
        .ok_or_else(|| args::bad(span, "solve_linear_system: singular or shape mismatch"))?;
    Ok(args::record([("x", args::f64_list_value(x))]))
}

/// `SymbolicAlgebra.expr_citation_hash` — FNV-1a `q_hash` of the expression display form.
/// Args: `{ expr }`. Out: `{ hash }` (u64).
pub fn expr_citation_hash(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let expr = args::rec_str(args_v, "expr").ok_or_else(|| {
        args::bad(span, "SymbolicAlgebra.expr_citation_hash needs expr: string")
    })?;
    let e = sa::parse(expr).map_err(|e| args::bad(span, format!("parse error: {e}")))?;
    let hash = sa::expr_citation_hash(&e);
    Ok(args::record([("hash", Value::U64(hash))]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn span() -> Span {
        Span { start: 0, end: 0 }
    }

    #[test]
    fn wave5_solve_separable_growth() {
        let mut m = BTreeMap::new();
        m.insert("g".into(), Value::String("1".into()));
        m.insert("h".into(), Value::String("y".into()));
        let out = solve_separable(&Value::Record(m), span()).unwrap();
        assert_eq!(args::rec_str(&out, "kind"), Some("implicit"));
        let f_y = args::rec_str(&out, "f_y").unwrap();
        assert!(f_y.contains("ln") || f_y.contains("log"), "got {f_y}");
    }

    #[test]
    fn wave5_solve_separable_fails_closed() {
        let mut m = BTreeMap::new();
        m.insert("g".into(), Value::String("1".into()));
        m.insert("h".into(), Value::String("sin(y^2)".into()));
        assert!(solve_separable(&Value::Record(m), span()).is_err());
    }

    #[test]
    fn wave5_first_order_linear_pde_invariant() {
        let mut m = BTreeMap::new();
        m.insert("a".into(), Value::F64(1.0));
        m.insert("b".into(), Value::F64(2.0));
        let out = solve_first_order_linear_pde(&Value::Record(m), span()).unwrap();
        assert_eq!(args::rec_str(&out, "kind"), Some("general_function_of"));
        let inv = args::rec_str(&out, "invariant").unwrap();
        assert!(inv.contains('x') && inv.contains('y'), "got {inv}");
    }

    #[test]
    fn wave5_pde_rejects_zero_coeffs() {
        let mut m = BTreeMap::new();
        m.insert("a".into(), Value::F64(0.0));
        m.insert("b".into(), Value::F64(0.0));
        assert!(solve_first_order_linear_pde(&Value::Record(m), span()).is_err());
    }

    #[test]
    fn wave5_solve_polynomial_expr_quadratic() {
        let mut m = BTreeMap::new();
        m.insert("expr".into(), Value::String("x^2 - 5*x + 6".into()));
        m.insert("degree".into(), Value::U64(2));
        let out = solve_polynomial_expr(&Value::Record(m), span()).unwrap();
        let roots = args::rec(&out, "roots").and_then(args::f64s).unwrap();
        assert_eq!(roots.len(), 2);
        assert!((roots[0] - 2.0).abs() < 1e-5);
        assert!((roots[1] - 3.0).abs() < 1e-5);
    }

    #[test]
    fn wave5_simplify_with_assumptions_sqrt_x2() {
        let mut m = BTreeMap::new();
        m.insert("expr".into(), Value::String("sqrt(x^2)".into()));
        let mut asm = BTreeMap::new();
        asm.insert("var".into(), Value::String("x".into()));
        asm.insert("sign".into(), Value::String("nonnegative".into()));
        m.insert("assumptions".into(), Value::List(vec![Value::Record(asm)]));
        let out = simplify_with_assumptions(&Value::Record(m), span()).unwrap();
        let s = args::rec_str(&out, "simplified").unwrap();
        assert_eq!(s, "x");
    }

    #[test]
    fn wave5_complex_roots_of_x2_plus_1() {
        let mut m = BTreeMap::new();
        m.insert("coeffs".into(), args::f64_list_value(vec![1.0, 0.0, 1.0]));
        let out = roots(&Value::Record(m), span()).unwrap();
        let Value::List(zs) = args::rec(&out, "roots").unwrap() else {
            panic!("expected list");
        };
        assert_eq!(zs.len(), 2);
        let re0 = args::rec_f64(&zs[0], "re").unwrap();
        let im0 = args::rec_f64(&zs[0], "im").unwrap();
        assert!(re0.abs() < 1e-6);
        assert!((im0.abs() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn wave5_solve_linear_system_2x2() {
        let mut m = BTreeMap::new();
        m.insert("a".into(), args::f64_list_value(vec![2.0, 1.0, 1.0, 3.0]));
        m.insert("b".into(), args::f64_list_value(vec![3.0, 5.0]));
        m.insert("n".into(), Value::U64(2));
        let out = solve_linear_system(&Value::Record(m), span()).unwrap();
        let x = args::rec(&out, "x").and_then(args::f64s).unwrap();
        assert!((x[0] - 0.8).abs() < 1e-9);
        assert!((x[1] - 1.4).abs() < 1e-9);
    }

    #[test]
    fn wave5_solve_linear_system_singular() {
        let mut m = BTreeMap::new();
        m.insert("a".into(), args::f64_list_value(vec![1.0, 2.0, 2.0, 4.0]));
        m.insert("b".into(), args::f64_list_value(vec![1.0, 2.0]));
        m.insert("n".into(), Value::U64(2));
        assert!(solve_linear_system(&Value::Record(m), span()).is_err());
    }

    #[test]
    fn wave5_expr_citation_hash_stable() {
        let mut m = BTreeMap::new();
        m.insert("expr".into(), Value::String("x + 1".into()));
        let out = expr_citation_hash(&Value::Record(m.clone()), span()).unwrap();
        let h = match args::rec(&out, "hash") {
            Some(Value::U64(v)) => *v,
            _ => panic!("expected u64 hash"),
        };
        assert_ne!(h, 0);
        let out2 = expr_citation_hash(&Value::Record(m), span()).unwrap();
        assert_eq!(args::rec(&out2, "hash"), Some(&Value::U64(h)));
    }
}
