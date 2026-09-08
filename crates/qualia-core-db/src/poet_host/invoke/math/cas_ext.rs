//! Wave-4 CAS / ODE Host binds over specialized_libs pure numeric helpers.
//!
//! Covers Host-missing `pub fn`s not bound in waves 1–3:
//! `integrate_definite`, `limit_at_infinity`, `real_roots`,
//! `solve_linear_first_order`, `solve_linear_second_order`, `classify_second_order_pde`.

use super::super::args;
use crate::specialized_libs::symbolic_algebra as sa;
use crate::specialized_libs::symbolic_ode::{
    classify_second_order_pde as classify_pde, solve_linear_first_order as ode_lin1,
    solve_linear_second_order as ode_lin2, OdeSolution, PdeClass,
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

fn pde_class_str(c: PdeClass) -> &'static str {
    match c {
        PdeClass::Elliptic => "elliptic",
        PdeClass::Parabolic => "parabolic",
        PdeClass::Hyperbolic => "hyperbolic",
    }
}

/// `SymbolicAlgebra.integrate_definite` — ∫_a^b expr dx (FTC or Simpson fallback).
/// Args: `{ expr, a, b, var?, steps? }`. Out: `{ value }`.
pub fn integrate_definite(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let expr = args::rec_str(args_v, "expr").ok_or_else(|| {
        args::bad(span, "SymbolicAlgebra.integrate_definite needs expr: string")
    })?;
    let a = args::rec_f64(args_v, "a")
        .ok_or_else(|| args::bad(span, "integrate_definite needs a: number"))?;
    let b = args::rec_f64(args_v, "b")
        .ok_or_else(|| args::bad(span, "integrate_definite needs b: number"))?;
    let var = args::rec_str(args_v, "var").unwrap_or("x");
    let steps = args::rec_u64(args_v, "steps").unwrap_or(64) as usize;
    let e = sa::parse(expr).map_err(|e| args::bad(span, format!("parse error: {e}")))?;
    let value = crate::specialized_libs::symbolic_integration::integrate_definite(&e, var, a, b, steps)
        .ok_or_else(|| args::bad(span, "integrate_definite: non-finite evaluation"))?;
    Ok(args::record([("value", Value::F64(value))]))
}

/// `SymbolicAlgebra.limit_at_infinity` — lim_{x→∞} f(x) via numeric probe.
/// Args: `{ expr, var? }`. Out: `{ value }`.
pub fn limit_at_infinity(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let expr = args::rec_str(args_v, "expr").ok_or_else(|| {
        args::bad(span, "SymbolicAlgebra.limit_at_infinity needs expr: string")
    })?;
    let var = args::rec_str(args_v, "var").unwrap_or("x");
    let e = sa::parse(expr).map_err(|e| args::bad(span, format!("parse error: {e}")))?;
    let value = crate::specialized_libs::symbolic_limits::limit_at_infinity(&e, var)
        .ok_or_else(|| args::bad(span, "limit_at_infinity: does not appear to converge"))?;
    Ok(args::record([("value", Value::F64(value))]))
}

/// `SymbolicAlgebra.real_roots` — real roots of a descending-coeff polynomial.
/// Args: `{ coeffs, tol? }`. Out: `{ roots }`.
pub fn real_roots(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let coeffs = args::rec_f64_list(args_v, "coeffs")
        .ok_or_else(|| args::bad(span, "real_roots needs coeffs: [f64]"))?;
    let tol = args::rec_f64(args_v, "tol").unwrap_or(1e-8);
    let roots = crate::specialized_libs::symbolic_solve::real_roots(&coeffs, tol)
        .map_err(|e| args::bad(span, format!("real_roots: {e:?}")))?;
    Ok(args::record([("roots", args::f64_list_value(roots))]))
}

/// `SymbolicODE.solve_linear_first_order` — `y' + a·y = b` closed form.
/// Args: `{ a, b, var? }`. Out: `{ kind, y }`.
pub fn solve_linear_first_order(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = args::rec_f64(args_v, "a")
        .ok_or_else(|| args::bad(span, "solve_linear_first_order needs a: number"))?;
    let b = args::rec_f64(args_v, "b")
        .ok_or_else(|| args::bad(span, "solve_linear_first_order needs b: number"))?;
    let var = args::rec_str(args_v, "var").unwrap_or("x");
    Ok(ode_value(&ode_lin1(a, b, var)))
}

/// `SymbolicODE.solve_linear_second_order` — `a·y'' + b·y' + c·y = 0`.
/// Args: `{ a, b, c, var? }`. Out: `{ kind, y }` or error if `a == 0`.
pub fn solve_linear_second_order(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = args::rec_f64(args_v, "a")
        .ok_or_else(|| args::bad(span, "solve_linear_second_order needs a: number"))?;
    let b = args::rec_f64(args_v, "b")
        .ok_or_else(|| args::bad(span, "solve_linear_second_order needs b: number"))?;
    let c = args::rec_f64(args_v, "c")
        .ok_or_else(|| args::bad(span, "solve_linear_second_order needs c: number"))?;
    let var = args::rec_str(args_v, "var").unwrap_or("x");
    let sol = ode_lin2(a, b, c, var)
        .map_err(|e| args::bad(span, format!("solve_linear_second_order: {e:?}")))?;
    Ok(ode_value(&sol))
}

/// `SymbolicODE.classify_second_order_pde` — elliptic / parabolic / hyperbolic by `B²−4AC`.
/// Args: `{ a_xx, b_xy, c_yy }`. Out: `{ class }`.
pub fn classify_second_order_pde(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a_xx = args::rec_f64(args_v, "a_xx")
        .ok_or_else(|| args::bad(span, "classify_second_order_pde needs a_xx"))?;
    let b_xy = args::rec_f64(args_v, "b_xy")
        .ok_or_else(|| args::bad(span, "classify_second_order_pde needs b_xy"))?;
    let c_yy = args::rec_f64(args_v, "c_yy")
        .ok_or_else(|| args::bad(span, "classify_second_order_pde needs c_yy"))?;
    let class = classify_pde(a_xx, b_xy, c_yy);
    Ok(args::record([(
        "class",
        Value::String(pde_class_str(class).into()),
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
    fn integrate_definite_x_squared_zero_to_one() {
        // ∫_0^1 x² dx = 1/3
        let mut m = BTreeMap::new();
        m.insert("expr".into(), Value::String("x^2".into()));
        m.insert("a".into(), Value::F64(0.0));
        m.insert("b".into(), Value::F64(1.0));
        let out = integrate_definite(&Value::Record(m), span()).unwrap();
        assert!((args::rec_f64(&out, "value").unwrap() - 1.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn limit_at_infinity_rational() {
        // (2x²+3)/(x²−1) → 2
        let mut m = BTreeMap::new();
        m.insert(
            "expr".into(),
            Value::String("(2*x^2 + 3) / (x^2 - 1)".into()),
        );
        let out = limit_at_infinity(&Value::Record(m), span()).unwrap();
        assert!((args::rec_f64(&out, "value").unwrap() - 2.0).abs() < 1e-3);
    }

    #[test]
    fn real_roots_quadratic() {
        // x² − 3x + 2 = 0 → 1, 2
        let mut m = BTreeMap::new();
        m.insert("coeffs".into(), args::f64_list_value(vec![1.0, -3.0, 2.0]));
        let out = real_roots(&Value::Record(m), span()).unwrap();
        let roots = args::rec(&out, "roots").and_then(args::f64s).unwrap();
        assert_eq!(roots.len(), 2);
        assert!((roots[0] - 1.0).abs() < 1e-6);
        assert!((roots[1] - 2.0).abs() < 1e-6);
    }

    #[test]
    fn lin1_constant_coeff() {
        // y' + 2y = 4 → y = 2 + C·e^{−2x}
        let mut m = BTreeMap::new();
        m.insert("a".into(), Value::F64(2.0));
        m.insert("b".into(), Value::F64(4.0));
        let out = solve_linear_first_order(&Value::Record(m), span()).unwrap();
        assert_eq!(
            args::rec_str(&out, "kind"),
            Some("explicit")
        );
        let y = args::rec_str(&out, "y").unwrap();
        assert!(y.contains('C') || y.contains('e'), "got {y}");
    }

    #[test]
    fn lin2_distinct_real() {
        // y'' − 3y' + 2y = 0 → roots 1, 2
        let mut m = BTreeMap::new();
        m.insert("a".into(), Value::F64(1.0));
        m.insert("b".into(), Value::F64(-3.0));
        m.insert("c".into(), Value::F64(2.0));
        let out = solve_linear_second_order(&Value::Record(m), span()).unwrap();
        assert_eq!(args::rec_str(&out, "kind"), Some("explicit"));
        let y = args::rec_str(&out, "y").unwrap();
        assert!(y.contains("C1") && y.contains("C2"), "got {y}");
    }

    #[test]
    fn lin2_rejects_zero_a() {
        let mut m = BTreeMap::new();
        m.insert("a".into(), Value::F64(0.0));
        m.insert("b".into(), Value::F64(1.0));
        m.insert("c".into(), Value::F64(1.0));
        assert!(solve_linear_second_order(&Value::Record(m), span()).is_err());
    }

    #[test]
    fn classify_laplace_elliptic() {
        // u_xx + u_yy = 0 → A=1, B=0, C=1 → elliptic
        let mut m = BTreeMap::new();
        m.insert("a_xx".into(), Value::F64(1.0));
        m.insert("b_xy".into(), Value::F64(0.0));
        m.insert("c_yy".into(), Value::F64(1.0));
        let out = classify_second_order_pde(&Value::Record(m), span()).unwrap();
        assert_eq!(args::rec_str(&out, "class"), Some("elliptic"));
    }

    #[test]
    fn classify_wave_hyperbolic() {
        // u_xx − u_yy = 0 → A=1, B=0, C=−1 → hyperbolic
        let mut m = BTreeMap::new();
        m.insert("a_xx".into(), Value::F64(1.0));
        m.insert("b_xy".into(), Value::F64(0.0));
        m.insert("c_yy".into(), Value::F64(-1.0));
        let out = classify_second_order_pde(&Value::Record(m), span()).unwrap();
        assert_eq!(args::rec_str(&out, "class"), Some("hyperbolic"));
    }
}
