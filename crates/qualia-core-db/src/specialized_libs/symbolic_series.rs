//! **Taylor / Maclaurin series** (Calculus plan §4.2) — the series of a CAS expression
//! about a point, by repeated symbolic differentiation. `cₖ = f⁽ᵏ⁾(a)/k!`.

use super::symbolic_algebra::{differentiate, simplify, Expr};

/// Taylor coefficients `[c₀, …, c_order]` of `f` about `x = a`. `None` if any derivative
/// fails to evaluate at `a` (e.g. a singularity there).
pub fn taylor_coefficients(f: &Expr, x: &str, a: f64, order: usize) -> Option<Vec<f64>> {
    let mut coeffs = Vec::with_capacity(order + 1);
    let mut deriv = f.clone();
    let mut factorial = 1.0;
    for k in 0..=order {
        if k > 0 {
            deriv = simplify(&differentiate(&deriv, x));
            factorial *= k as f64;
        }
        let mut env = std::collections::HashMap::new();
        env.insert(x.to_string(), a);
        coeffs.push(deriv.eval(&env)? / factorial);
    }
    Some(coeffs)
}

/// Evaluate the truncated Taylor polynomial `Σ cₖ (x−a)ᵏ` at `x`.
pub fn taylor_eval(coeffs: &[f64], a: f64, x: f64) -> f64 {
    let d = x - a;
    coeffs.iter().rev().fold(0.0, |acc, &c| acc * d + c)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::symbolic_algebra::{add, c, mul, pow, sqrt, var};

    #[test]
    fn series_of_polynomial_is_exact() {
        // f = x³ − 2x + 1 about 0 → coeffs [1, −2, 0, 1], higher are 0.
        let f = add(super::super::symbolic_algebra::sub(pow(var("x"), 3), mul(c(2.0), var("x"))), c(1.0));
        let coeffs = taylor_coefficients(&f, "x", 0.0, 5).unwrap();
        assert!((coeffs[0] - 1.0).abs() < 1e-9);
        assert!((coeffs[1] + 2.0).abs() < 1e-9);
        assert!(coeffs[2].abs() < 1e-9);
        assert!((coeffs[3] - 1.0).abs() < 1e-9);
        assert!(coeffs[4].abs() < 1e-9 && coeffs[5].abs() < 1e-9);
    }

    #[test]
    fn series_of_sqrt_about_one() {
        // √x about 1: c₀=1, c₁=1/2, c₂=−1/8, c₃=1/16.
        let coeffs = taylor_coefficients(&sqrt(var("x")), "x", 1.0, 3).unwrap();
        assert!((coeffs[0] - 1.0).abs() < 1e-9);
        assert!((coeffs[1] - 0.5).abs() < 1e-9);
        assert!((coeffs[2] + 0.125).abs() < 1e-9);
        assert!((coeffs[3] - 0.0625).abs() < 1e-9);
        // The truncated series approximates √x near 1.
        let approx = taylor_eval(&coeffs, 1.0, 1.1);
        assert!((approx - 1.1_f64.sqrt()).abs() < 1e-4);
    }

    #[test]
    fn singularity_fails_closed() {
        // 1/x has no Taylor series about 0.
        let f = super::super::symbolic_algebra::div(c(1.0), var("x"));
        assert!(taylor_coefficients(&f, "x", 0.0, 3).is_none());
    }
}
