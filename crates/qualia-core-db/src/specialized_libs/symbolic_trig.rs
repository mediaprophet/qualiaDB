//! **Trigonometric simplification** (Gap analysis §3.3) — identity-driven rewrites over the
//! CAS's `Sin`/`Cos`/`Tan` nodes that plain [`simplify`](super::symbolic_algebra::simplify)
//! cannot do (it knows only constant folding and algebraic identities).
//!
//! Implemented identities, each value-preserving for all real inputs:
//! - **Pythagorean:** `k·sin²(u) + k·cos²(u) → k` (any common scalar `k`, either order),
//!   and `1 − sin²(u) → cos²(u)`, `1 − cos²(u) → sin²(u)`.
//! - **Parity:** `sin(−u) → −sin(u)`, `cos(−u) → cos(u)`, `tan(−u) → −tan(u)`.
//! - **Quotient:** `sin(u)/cos(u) → tan(u)`.
//!
//! Applied bottom-up to a bounded fixpoint and interleaved with the base `simplify`, so the
//! Pythagorean collapse also fires on nested sums after the algebra normalises them.

use super::symbolic_algebra::{add, c, cos, div, mul, neg, pow, simplify, sin, sub, tan, Expr};

/// Simplify trigonometric structure in `expr`, layered on the always-sound base `simplify`.
pub fn simplify_trig(expr: &Expr) -> Expr {
    let mut cur = simplify(expr);
    for _ in 0..16 {
        let next = simplify(&rewrite(&cur));
        if next == cur {
            break;
        }
        cur = next;
    }
    cur
}

/// If `e` is `k·trig(u)²` (or `trig(u)²` with `k = 1`), return `(k, is_sin, u)`.
fn as_scaled_square(e: &Expr) -> Option<(Expr, bool, Expr)> {
    let classify = |b: &Expr| -> Option<(bool, Expr)> {
        match b {
            Expr::Sin(u) => Some((true, (**u).clone())),
            Expr::Cos(u) => Some((false, (**u).clone())),
            _ => None,
        }
    };
    match e {
        Expr::Pow(base, 2) => classify(base).map(|(is_sin, u)| (c(1.0), is_sin, u)),
        Expr::Mul(a, b) => {
            if let (Expr::Const(_), Expr::Pow(base, 2)) = (&**a, &**b) {
                classify(base).map(|(is_sin, u)| ((**a).clone(), is_sin, u))
            } else if let (Expr::Pow(base, 2), Expr::Const(_)) = (&**a, &**b) {
                classify(base).map(|(is_sin, u)| ((**b).clone(), is_sin, u))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// `k·sin²(u) + k·cos²(u) → k` when the two summands share a coefficient and argument.
fn pythagorean(a: &Expr, b: &Expr) -> Option<Expr> {
    let (ka, sin_a, ua) = as_scaled_square(a)?;
    let (kb, sin_b, ub) = as_scaled_square(b)?;
    if ka == kb && ua == ub && sin_a != sin_b {
        Some(ka)
    } else {
        None
    }
}

fn rewrite(e: &Expr) -> Expr {
    // Bottom-up.
    let e = match e {
        Expr::Add(a, b) => add(rewrite(a), rewrite(b)),
        Expr::Sub(a, b) => sub(rewrite(a), rewrite(b)),
        Expr::Mul(a, b) => mul(rewrite(a), rewrite(b)),
        Expr::Div(a, b) => div(rewrite(a), rewrite(b)),
        Expr::Pow(a, n) => pow(rewrite(a), *n),
        Expr::Neg(a) => neg(rewrite(a)),
        Expr::Sqrt(a) => Expr::Sqrt(Box::new(rewrite(a))),
        Expr::Exp(a) => Expr::Exp(Box::new(rewrite(a))),
        Expr::Ln(a) => Expr::Ln(Box::new(rewrite(a))),
        Expr::Sin(a) => sin(rewrite(a)),
        Expr::Cos(a) => cos(rewrite(a)),
        Expr::Tan(a) => tan(rewrite(a)),
        Expr::Const(_) | Expr::Var(_) => e.clone(),
    };

    match &e {
        // Parity.
        Expr::Sin(u) => {
            if let Expr::Neg(inner) = &**u {
                return neg(sin((**inner).clone()));
            }
            e
        }
        Expr::Cos(u) => {
            if let Expr::Neg(inner) = &**u {
                return cos((**inner).clone());
            }
            e
        }
        Expr::Tan(u) => {
            if let Expr::Neg(inner) = &**u {
                return neg(tan((**inner).clone()));
            }
            e
        }
        // sin(u)/cos(u) → tan(u).
        Expr::Div(n, d) => {
            if let (Expr::Sin(un), Expr::Cos(ud)) = (&**n, &**d) {
                if un == ud {
                    return tan((**un).clone());
                }
            }
            e
        }
        // Pythagorean collapse on a sum (either order).
        Expr::Add(a, b) => {
            if let Some(r) = pythagorean(a, b).or_else(|| pythagorean(b, a)) {
                return r;
            }
            e
        }
        // 1 − sin²(u) → cos²(u) ; 1 − cos²(u) → sin²(u).
        Expr::Sub(a, b) => {
            if let Expr::Const(one) = &**a {
                if *one == 1.0 {
                    if let Expr::Pow(base, 2) = &**b {
                        match &**base {
                            Expr::Sin(u) => return pow(cos((**u).clone()), 2),
                            Expr::Cos(u) => return pow(sin((**u).clone()), 2),
                            _ => {}
                        }
                    }
                }
            }
            e
        }
        _ => e,
    }
}

#[cfg(test)]
mod tests {
    use super::super::symbolic_algebra::{var, Expr};
    use super::*;
    use std::collections::HashMap;

    fn val(e: &Expr, x: f64) -> f64 {
        let mut env = HashMap::new();
        env.insert("x".to_string(), x);
        e.eval(&env).unwrap()
    }

    #[test]
    fn pythagorean_identity_collapses() {
        // sin²(x) + cos²(x) → 1.
        let e = add(pow(sin(var("x")), 2), pow(cos(var("x")), 2));
        assert_eq!(simplify_trig(&e), c(1.0));
        // cos²(x) + sin²(x) → 1 (other order).
        let e2 = add(pow(cos(var("x")), 2), pow(sin(var("x")), 2));
        assert_eq!(simplify_trig(&e2), c(1.0));
        // 3·sin²(x) + 3·cos²(x) → 3.
        let e3 = add(
            mul(c(3.0), pow(sin(var("x")), 2)),
            mul(c(3.0), pow(cos(var("x")), 2)),
        );
        assert_eq!(simplify_trig(&e3), c(3.0));
    }

    #[test]
    fn pythagorean_in_a_larger_sum() {
        // sin²(x) + cos²(x) + x  →  1 + x (value-checked; structure may be (1 + x)).
        let e = add(add(pow(sin(var("x")), 2), pow(cos(var("x")), 2)), var("x"));
        let s = simplify_trig(&e);
        for &x in &[0.3, 1.2, 2.7] {
            assert!((val(&s, x) - (1.0 + x)).abs() < 1e-9);
        }
    }

    #[test]
    fn one_minus_square() {
        // 1 − sin²(x) → cos²(x) (value-checked).
        let s = simplify_trig(&sub(c(1.0), pow(sin(var("x")), 2)));
        for &x in &[0.3, 1.2, 2.7] {
            assert!((val(&s, x) - x.cos().powi(2)).abs() < 1e-9);
        }
    }

    #[test]
    fn parity_and_quotient() {
        // cos(−x) → cos(x) ; sin(−x) → −sin(x).
        assert_eq!(simplify_trig(&cos(neg(var("x")))), cos(var("x")));
        assert_eq!(simplify_trig(&sin(neg(var("x")))), neg(sin(var("x"))));
        // sin(x)/cos(x) → tan(x).
        assert_eq!(
            simplify_trig(&div(sin(var("x")), cos(var("x")))),
            tan(var("x"))
        );
    }
}
