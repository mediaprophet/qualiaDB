//! Laplace transform `L{f}(s) = ∫₀^∞ e^{−st} f(t) dt`.
//!
//! Two paths: a **numerical** transform by Simpson quadrature for any closure (general,
//! always available), and a **symbolic** table transform over the CAS expression type.
//! The current `Expr` algebra has no exp/trig variants, so the symbolic table covers the
//! cases it *can* represent — constants, integer powers `tⁿ`, and their linear
//! combinations — and **fails closed** (`NotTransformable`) on anything else rather than
//! returning a wrong transform.

use crate::specialized_libs::symbolic_algebra::{add, c, div, neg, pow, sub, var, Expr};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaplaceError {
    /// The expression is outside the symbolic table (e.g. needs exp/trig the CAS lacks).
    NotTransformable,
    /// Domain error in the numeric transform (`s ≤ 0`, non-positive horizon, …).
    OutOfDomain,
}

/// Numerical Laplace transform of `f` at `s`, integrating to `t_max` with `steps` (even)
/// Simpson panels. Requires `s > 0`, `t_max > 0`, `steps ≥ 2` even.
pub fn laplace_numeric<F: Fn(f64) -> f64>(
    f: F,
    s: f64,
    t_max: f64,
    steps: usize,
) -> Result<f64, LaplaceError> {
    if s <= 0.0 || t_max <= 0.0 || steps < 2 || steps % 2 != 0 {
        return Err(LaplaceError::OutOfDomain);
    }
    let h = t_max / steps as f64;
    let g = |t: f64| (-s * t).exp() * f(t);
    let mut sum = g(0.0) + g(t_max);
    for i in 1..steps {
        let t = i as f64 * h;
        sum += if i % 2 == 1 { 4.0 } else { 2.0 } * g(t);
    }
    Ok(sum * h / 3.0)
}

fn factorial(n: i32) -> f64 {
    (1..=n).fold(1.0, |a, k| a * k as f64)
}

/// Symbolic Laplace transform of `expr` in the time variable `t`, returning an `Expr` in
/// the complex frequency variable `s`. Handles constants (`c → c/s`), powers
/// (`tⁿ → n!/s^{n+1}`), negation, sums/differences, and scalar multiples; everything
/// else fails closed.
pub fn laplace_table(expr: &Expr) -> Result<Expr, LaplaceError> {
    transform(expr, "t")
}

fn transform(expr: &Expr, t: &str) -> Result<Expr, LaplaceError> {
    match expr {
        Expr::Const(k) => Ok(div(c(*k), var("s"))), // L{k} = k/s
        Expr::Var(name) if name == t => Ok(div(c(1.0), pow(var("s"), 2))), // L{t} = 1/s²
        Expr::Pow(base, n) if is_var(base, t) && *n >= 0 => {
            // L{tⁿ} = n!/s^{n+1}
            Ok(div(c(factorial(*n)), pow(var("s"), n + 1)))
        }
        Expr::Neg(a) => Ok(neg(transform(a, t)?)),
        Expr::Add(a, b) => Ok(add(transform(a, t)?, transform(b, t)?)),
        Expr::Sub(a, b) => Ok(sub(transform(a, t)?, transform(b, t)?)),
        Expr::Mul(a, b) => {
            // Scalar · f(t): L is the scalar times L{f}.
            if let Expr::Const(_) = **a {
                Ok(crate::specialized_libs::symbolic_algebra::mul((**a).clone(), transform(b, t)?))
            } else if let Expr::Const(_) = **b {
                Ok(crate::specialized_libs::symbolic_algebra::mul((**b).clone(), transform(a, t)?))
            } else {
                Err(LaplaceError::NotTransformable)
            }
        }
        _ => Err(LaplaceError::NotTransformable),
    }
}

fn is_var(e: &Expr, name: &str) -> bool {
    matches!(e, Expr::Var(v) if v == name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn eval_at_s(e: &Expr, s: f64) -> f64 {
        let mut env = HashMap::new();
        env.insert("s".to_string(), s);
        e.eval(&env).unwrap()
    }

    #[test]
    fn numeric_transforms_match_closed_forms() {
        // L{1}(s) = 1/s
        assert!((laplace_numeric(|_| 1.0, 2.0, 60.0, 4000).unwrap() - 0.5).abs() < 1e-4);
        // L{e^{-t}}(s=1) = 1/(s+1) = 1/2
        assert!((laplace_numeric(|t| (-t).exp(), 1.0, 60.0, 4000).unwrap() - 0.5).abs() < 1e-4);
        // L{t}(s=1) = 1/s² = 1
        assert!((laplace_numeric(|t| t, 1.0, 80.0, 8000).unwrap() - 1.0).abs() < 1e-3);
    }

    #[test]
    fn symbolic_table_powers_and_linearity() {
        // L{t²} = 2/s³ ; at s=2 → 2/8 = 0.25
        let l = laplace_table(&pow(var("t"), 2)).unwrap();
        assert!((eval_at_s(&l, 2.0) - 0.25).abs() < 1e-12);
        // L{3} = 3/s ; at s=3 → 1
        let l2 = laplace_table(&c(3.0)).unwrap();
        assert!((eval_at_s(&l2, 3.0) - 1.0).abs() < 1e-12);
        // L{t + 5} = 1/s² + 5/s ; at s=1 → 6
        let l3 = laplace_table(&add(var("t"), c(5.0))).unwrap();
        assert!((eval_at_s(&l3, 1.0) - 6.0).abs() < 1e-12);
    }

    #[test]
    fn fails_closed_on_unrepresentable() {
        // sqrt(t) is not in the polynomial table.
        let e = crate::specialized_libs::symbolic_algebra::sqrt(var("t"));
        assert_eq!(laplace_table(&e).unwrap_err(), LaplaceError::NotTransformable);
        assert_eq!(laplace_numeric(|_| 1.0, -1.0, 10.0, 100).unwrap_err(), LaplaceError::OutOfDomain);
    }
}
