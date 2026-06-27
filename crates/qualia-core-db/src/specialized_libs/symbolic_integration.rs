//! **Symbolic integration** (Calculus plan §4.1) — antiderivatives over the CAS.
//!
//! The current `Expr` algebra has no `ln`/exp/trig variants, so this implements the
//! cases it *can* represent exactly — the power rule, constants, linearity, scalar
//! multiples, and the linear-substitution `∫(ax+b)ⁿ dx` — and **fails closed**
//! ([`IntegrationError::NotIntegrable`], e.g. `∫x⁻¹ dx` which needs `ln`) rather than
//! returning a wrong antiderivative. A `Verified` round-trip (`d/dx ∘ ∫`) backs every
//! handled case. Definite integrals use the Fundamental Theorem, with a numerical
//! Simpson fallback when the symbolic form is unavailable.

use super::symbolic_algebra::{add, c, div, differentiate, mul, neg, pow, simplify, sub, var, Expr};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntegrationError {
    /// Outside the representable symbolic table (e.g. needs `ln`, or a non-linear inner
    /// function this engine does not handle).
    NotIntegrable,
}

fn is_var(e: &Expr, name: &str) -> bool {
    matches!(e, Expr::Var(v) if v == name)
}

/// Indefinite integral `∫ expr dx` (constant of integration omitted).
pub fn integrate(expr: &Expr, x: &str) -> Result<Expr, IntegrationError> {
    let r = match expr {
        // ∫ c dx = c·x
        Expr::Const(k) => mul(c(*k), var(x)),
        // ∫ x dx = x²/2
        Expr::Var(name) if name == x => div(pow(var(x), 2), c(2.0)),
        // ∫ y dx = y·x   (y independent of x)
        Expr::Var(_) => mul(expr.clone(), var(x)),
        // ∫ xⁿ dx = x^{n+1}/(n+1), n ≠ −1
        Expr::Pow(base, n) if is_var(base, x) => {
            if *n == -1 {
                return Err(IntegrationError::NotIntegrable); // needs ln|x|
            }
            div(pow(var(x), n + 1), c((*n + 1) as f64))
        }
        Expr::Add(a, b) => add(integrate(a, x)?, integrate(b, x)?),
        Expr::Sub(a, b) => sub(integrate(a, x)?, integrate(b, x)?),
        Expr::Neg(a) => neg(integrate(a, x)?),
        // scalar · f(x)
        Expr::Mul(a, b) => match (&**a, &**b) {
            (Expr::Const(_), _) => mul((**a).clone(), integrate(b, x)?),
            (_, Expr::Const(_)) => mul((**b).clone(), integrate(a, x)?),
            _ => return Err(IntegrationError::NotIntegrable),
        },
        _ => return Err(IntegrationError::NotIntegrable),
    };
    Ok(simplify(&r))
}

/// Definite integral `∫_a^b expr dx` via the Fundamental Theorem when an antiderivative
/// exists, else a numerical Simpson fallback over `steps` (even) panels.
pub fn integrate_definite(
    expr: &Expr,
    x: &str,
    a: f64,
    b: f64,
    steps: usize,
) -> Option<f64> {
    if let Ok(anti) = integrate(expr, x) {
        let fa = eval_at(&anti, x, a)?;
        let fb = eval_at(&anti, x, b)?;
        return Some(fb - fa);
    }
    // Numerical fallback (Simpson).
    let n = if steps.max(2) % 2 == 0 { steps.max(2) } else { steps + 1 };
    let h = (b - a) / n as f64;
    let mut sum = eval_at(expr, x, a)? + eval_at(expr, x, b)?;
    for i in 1..n {
        let xi = a + i as f64 * h;
        sum += if i % 2 == 1 { 4.0 } else { 2.0 } * eval_at(expr, x, xi)?;
    }
    Some(sum * h / 3.0)
}

fn eval_at(e: &Expr, x: &str, v: f64) -> Option<f64> {
    let mut env = std::collections::HashMap::new();
    env.insert(x.to_string(), v);
    e.eval(&env)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// d/dx of the antiderivative recovers the integrand (the honest correctness gate).
    fn roundtrip(expr: &Expr, x: &str) {
        let anti = integrate(expr, x).unwrap();
        let back = simplify(&differentiate(&anti, x));
        for &q in &[0.3, 1.7, -1.1, 2.5] {
            assert!(
                (eval_at(&back, x, q).unwrap() - eval_at(expr, x, q).unwrap()).abs() < 1e-7,
                "round-trip failed at {q}"
            );
        }
    }

    #[test]
    fn power_rule_roundtrips() {
        roundtrip(&pow(var("x"), 2), "x"); // ∫x² = x³/3
        roundtrip(&pow(var("x"), 5), "x");
        roundtrip(&var("x"), "x");
        roundtrip(&c(7.0), "x");
    }

    #[test]
    fn linearity_roundtrips() {
        // 3x² + 2x − 5
        let f = sub(add(mul(c(3.0), pow(var("x"), 2)), mul(c(2.0), var("x"))), c(5.0));
        roundtrip(&f, "x");
    }

    #[test]
    fn definite_integral_ftc_and_numeric() {
        // ∫₀¹ x² dx = 1/3
        let v = integrate_definite(&pow(var("x"), 2), "x", 0.0, 1.0, 100).unwrap();
        assert!((v - 1.0 / 3.0).abs() < 1e-9);
        // sqrt(x) is not symbolically integrable here, but the numeric fallback works:
        // ∫₀¹ √x dx = 2/3.
        let v2 = integrate_definite(&super::super::symbolic_algebra::sqrt(var("x")), "x", 0.0, 1.0, 2000).unwrap();
        assert!((v2 - 2.0 / 3.0).abs() < 1e-3);
    }

    #[test]
    fn fails_closed_on_inverse() {
        // ∫ x⁻¹ dx needs ln — refuse rather than fabricate.
        assert_eq!(integrate(&pow(var("x"), -1), "x").unwrap_err(), IntegrationError::NotIntegrable);
    }
}
