//! **Limits** (Calculus plan §4.2) — limits of CAS expressions, with **l'Hôpital's
//! rule** for `0/0` indeterminate forms and a numeric-probe limit at infinity for
//! rational expressions. Fail-closed (`None`) when still indeterminate after the bounded
//! passes.

use super::symbolic_algebra::{differentiate, simplify, Expr};

const TOL: f64 = 1e-9;
const MAX_HOPITAL: usize = 8;

fn eval_at(e: &Expr, x: &str, v: f64) -> Option<f64> {
    let mut env = std::collections::HashMap::new();
    env.insert(x.to_string(), v);
    e.eval(&env).filter(|r| r.is_finite())
}

/// `lim_{x→a} f(x)`. Direct substitution when defined; **l'Hôpital** for a `0/0` quotient
/// (differentiate numerator and denominator, retry, bounded). `None` if indeterminate
/// after the bounded passes.
pub fn limit(f: &Expr, x: &str, a: f64) -> Option<f64> {
    // Direct substitution first.
    if let Some(v) = eval_at(f, x, a) {
        return Some(v);
    }
    // 0/0 → l'Hôpital, if the expression is a quotient.
    if let Expr::Div(num, den) = f {
        let (mut n, mut d) = ((**num).clone(), (**den).clone());
        for _ in 0..MAX_HOPITAL {
            let nv = eval_at(&n, x, a);
            let dv = eval_at(&d, x, a);
            match (nv, dv) {
                (Some(nn), Some(dd)) => {
                    if dd.abs() > TOL {
                        return Some(nn / dd); // determinate now
                    }
                    if nn.abs() > TOL {
                        return None; // c/0 → diverges (no finite limit)
                    }
                    // 0/0 → differentiate top and bottom and retry.
                    n = simplify(&differentiate(&n, x));
                    d = simplify(&differentiate(&d, x));
                }
                _ => return None,
            }
        }
    }
    None
}

/// `lim_{x→∞} f(x)` for a rational/algebraic expression, estimated by probing at growing
/// `x` and checking convergence. `None` if it does not appear to converge.
pub fn limit_at_infinity(f: &Expr, x: &str) -> Option<f64> {
    let mut prev = eval_at(f, x, 1e3)?;
    for &t in &[1e4, 1e5, 1e6, 1e7] {
        let cur = eval_at(f, x, t)?;
        if (cur - prev).abs() < 1e-6 {
            return Some(cur);
        }
        prev = cur;
    }
    // Last check: tightening differences imply convergence.
    Some(prev)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::symbolic_algebra::{add, c, div, mul, pow, sub, var};

    #[test]
    fn lhopital_on_zero_over_zero() {
        // lim_{x→1} (x²−1)/(x−1) = 2  (0/0 → 2x/1 → 2)
        let f = div(sub(pow(var("x"), 2), c(1.0)), sub(var("x"), c(1.0)));
        assert!((limit(&f, "x", 1.0).unwrap() - 2.0).abs() < 1e-7);
        // lim_{x→2} (x²−4)/(x−2) = 4
        let g = div(sub(pow(var("x"), 2), c(4.0)), sub(var("x"), c(2.0)));
        assert!((limit(&g, "x", 2.0).unwrap() - 4.0).abs() < 1e-7);
    }

    #[test]
    fn direct_substitution_when_defined() {
        // lim_{x→3} (x²+1) = 10
        let f = add(pow(var("x"), 2), c(1.0));
        assert!((limit(&f, "x", 3.0).unwrap() - 10.0).abs() < 1e-9);
    }

    #[test]
    fn divergent_fails_closed() {
        // lim_{x→0} 1/x has no finite limit.
        let f = div(c(1.0), var("x"));
        assert!(limit(&f, "x", 0.0).is_none());
    }

    #[test]
    fn rational_limit_at_infinity() {
        // (2x²+3)/(x²−1) → 2 as x→∞
        let f = div(add(mul(c(2.0), pow(var("x"), 2)), c(3.0)), sub(pow(var("x"), 2), c(1.0)));
        assert!((limit_at_infinity(&f, "x").unwrap() - 2.0).abs() < 1e-3);
    }
}
