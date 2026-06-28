//! **Multivariable symbolic differentiation** — gradient, Jacobian, Hessian (Calculus
//! plan §3, the ★★ standout). Built on the CAS's existing single-variable
//! [`differentiate`](super::symbolic_algebra::differentiate), so every partial is a
//! *symbolic, provenance-bearing* derivative (citable via the CAS's `to_quins`/
//! `expr_citation_hash`) — honest math, not a black-box autodiff number.
//!
//! Why this is the highest-demand gap: the learning spine needs it *now* — IRLS
//! (logistic/Poisson GLM) needs the gradient + Hessian, the Bayesian Laplace
//! approximation needs the Hessian of the log-posterior, and second-order optimisers
//! need a Hessian. The symbolic forms here are differentiated once, then evaluated
//! numerically at a point ([`gradient_at`]/[`hessian_at`]) for those consumers.
//!
//! No hot kernel (symbolic); the *numeric evaluation* of a gradient at scale is the
//! bridge's `DenseLinear`/`ElementwiseMap` case.

use std::collections::HashMap;

use super::symbolic_algebra::{differentiate, simplify, Expr};

/// `∂expr/∂var` — a single partial derivative (simplified). Thin alias over the CAS.
pub fn partial(expr: &Expr, var: &str) -> Expr {
    simplify(&differentiate(expr, var))
}

/// The **gradient** `∇f = [∂f/∂x₁, …, ∂f/∂xₙ]` as one simplified expression per variable.
pub fn gradient(expr: &Expr, vars: &[&str]) -> Vec<Expr> {
    vars.iter().map(|v| partial(expr, v)).collect()
}

/// The **Jacobian** of a vector of expressions: row `i` is `∇fᵢ`. Shape
/// `exprs.len() × vars.len()`.
pub fn jacobian(exprs: &[Expr], vars: &[&str]) -> Vec<Vec<Expr>> {
    exprs.iter().map(|f| gradient(f, vars)).collect()
}

/// The **Hessian** `H[i][j] = ∂²f/∂xᵢ∂xⱼ` as an `n×n` matrix of simplified expressions.
/// Symmetric by Clairaut's theorem (computed both ways implicitly via repeated
/// differentiation).
pub fn hessian(expr: &Expr, vars: &[&str]) -> Vec<Vec<Expr>> {
    let grad = gradient(expr, vars);
    grad.iter().map(|gi| gradient(gi, vars)).collect()
}

/// Evaluate the gradient numerically at `point` (variable → value). `None` if any
/// partial fails to evaluate there (e.g. a division by zero in the domain).
pub fn gradient_at(expr: &Expr, vars: &[&str], point: &HashMap<String, f64>) -> Option<Vec<f64>> {
    gradient(expr, vars).iter().map(|g| g.eval(point)).collect()
}

/// Evaluate the Hessian numerically at `point`. `None` if any entry fails to evaluate.
pub fn hessian_at(
    expr: &Expr,
    vars: &[&str],
    point: &HashMap<String, f64>,
) -> Option<Vec<Vec<f64>>> {
    hessian(expr, vars)
        .iter()
        .map(|row| {
            row.iter()
                .map(|h| h.eval(point))
                .collect::<Option<Vec<f64>>>()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::symbolic_algebra::{add, c, mul, pow, var};

    fn env(pairs: &[(&str, f64)]) -> HashMap<String, f64> {
        pairs.iter().map(|&(k, v)| (k.to_string(), v)).collect()
    }

    #[test]
    fn gradient_of_a_quadratic_form() {
        // f = x² + x·y + y²  →  ∇f = [2x + y, x + 2y]
        let f = add(
            add(pow(var("x"), 2), mul(var("x"), var("y"))),
            pow(var("y"), 2),
        );
        let g = gradient(&f, &["x", "y"]);
        let p = env(&[("x", 3.0), ("y", 5.0)]);
        // ∂f/∂x = 2·3 + 5 = 11 ; ∂f/∂y = 3 + 2·5 = 13
        assert!((g[0].eval(&p).unwrap() - 11.0).abs() < 1e-9);
        assert!((g[1].eval(&p).unwrap() - 13.0).abs() < 1e-9);
    }

    #[test]
    fn hessian_of_a_quadratic_is_constant() {
        // f = x² + x·y + y²  →  H = [[2, 1], [1, 2]], symmetric & constant.
        let f = add(
            add(pow(var("x"), 2), mul(var("x"), var("y"))),
            pow(var("y"), 2),
        );
        let h = hessian_at(&f, &["x", "y"], &env(&[("x", 0.0), ("y", 0.0)])).unwrap();
        assert!((h[0][0] - 2.0).abs() < 1e-9);
        assert!((h[0][1] - 1.0).abs() < 1e-9);
        assert!((h[1][0] - 1.0).abs() < 1e-9); // symmetry
        assert!((h[1][1] - 2.0).abs() < 1e-9);
    }

    #[test]
    fn jacobian_shape_and_values() {
        // F = [x·y, x + y] → J = [[y, x], [1, 1]]
        let f1 = mul(var("x"), var("y"));
        let f2 = add(var("x"), var("y"));
        let j = jacobian(&[f1, f2], &["x", "y"]);
        let p = env(&[("x", 2.0), ("y", 7.0)]);
        assert_eq!(j.len(), 2);
        assert!((j[0][0].eval(&p).unwrap() - 7.0).abs() < 1e-9); // ∂(xy)/∂x = y = 7
        assert!((j[0][1].eval(&p).unwrap() - 2.0).abs() < 1e-9); // ∂(xy)/∂y = x = 2
        assert!((j[1][0].eval(&p).unwrap() - 1.0).abs() < 1e-9);
        assert!((j[1][1].eval(&p).unwrap() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn gradient_at_evaluates_numerically() {
        // f = 3·x²  →  ∂f/∂x = 6x ; at x=4 → 24
        let f = mul(c(3.0), pow(var("x"), 2));
        let g = gradient_at(&f, &["x"], &env(&[("x", 4.0)])).unwrap();
        assert!((g[0] - 24.0).abs() < 1e-9);
    }
}
