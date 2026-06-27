//! **Symbolic differential equations** (Gap analysis §3.4) — closed-form solutions for the
//! standard *solvable* classes of ODE, plus first-order-linear PDE (method of
//! characteristics) and second-order-linear PDE type classification.
//!
//! Honest scope (everything outside it returns [`OdeError::NotSupported`] — never a
//! fabricated solution):
//!
//! **ODE**
//! - **Separable** `y' = g(x)·h(y)` → implicit `∫dy/h(y) = ∫g(x)dx + C`, using the CAS
//!   integrator ([`crate::specialized_libs::symbolic_integration`]); fails closed when
//!   either integral is outside the integrator's table.
//! - **Linear first-order, constant coefficients** `y' + a·y = b` → explicit.
//! - **Linear second-order, constant coefficients** `a·y'' + b·y' + c·y = 0` → explicit, via
//!   the characteristic equation (distinct-real / repeated / complex roots).
//!
//! **PDE**
//! - **First-order linear homogeneous** `a·uₓ + b·u_y = 0` → `u = F(b·x − a·y)` (an arbitrary
//!   differentiable `F`; the characteristic invariant is returned).
//! - **Second-order linear** `A·uₓₓ + B·u_xy + C·u_yy + … ` → elliptic / parabolic / hyperbolic
//!   classification by the discriminant `B² − 4AC`.
//!
//! A general nonlinear/variable-coefficient PDE solver is *not* attempted — that is genuinely
//! beyond a bounded module, and the contract here is "solve the supported classes exactly,
//! refuse the rest", not "pretend".

use super::symbolic_algebra::{add, c, cos, div, exp, mul, sin, var, Expr};
use super::symbolic_integration::{integrate, IntegrationError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OdeError {
    /// The equation is outside the supported solvable classes.
    NotSupported,
    /// A required antiderivative is outside the CAS integrator's table.
    NotIntegrable,
}

impl From<IntegrationError> for OdeError {
    fn from(_: IntegrationError) -> Self {
        OdeError::NotIntegrable
    }
}

/// The solution of an ODE.
#[derive(Debug, Clone, PartialEq)]
pub enum OdeSolution {
    /// `y(x)` given explicitly; integration constants appear as variables `C`, `C1`, `C2`.
    Explicit(Expr),
    /// An implicit relation `F(y) = G(x)` (with `G` already including `+ C`).
    Implicit { f_y: Expr, g_x: Expr },
}

/// Solve the **separable** ODE `y' = g(x)·h(y)` as `∫ dy/h(y) = ∫ g(x) dx + C`. The two
/// antiderivatives are computed by the CAS integrator; either being non-integrable yields
/// [`OdeError::NotIntegrable`].
pub fn solve_separable(g_x: &Expr, h_y: &Expr, xvar: &str, yvar: &str) -> Result<OdeSolution, OdeError> {
    let f_y = integrate(&div(c(1.0), h_y.clone()), yvar)?; // ∫ dy / h(y)
    let g_int = integrate(g_x, xvar)?; // ∫ g(x) dx
    Ok(OdeSolution::Implicit { f_y, g_x: add(g_int, var("C")) })
}

/// Solve the **linear first-order constant-coefficient** ODE `y' + a·y = b`.
/// - `a ≠ 0` → `y = b/a + C·e^{−a x}`.
/// - `a = 0` → `y = b·x + C`.
pub fn solve_linear_first_order(a: f64, b: f64, xvar: &str) -> OdeSolution {
    if a == 0.0 {
        OdeSolution::Explicit(add(mul(c(b), var(xvar)), var("C")))
    } else {
        let homogeneous = mul(var("C"), exp(mul(c(-a), var(xvar))));
        OdeSolution::Explicit(add(c(b / a), homogeneous))
    }
}

/// Solve the **linear second-order homogeneous constant-coefficient** ODE
/// `a·y'' + b·y' + c·y = 0` via its characteristic equation `a·r² + b·r + c = 0`. `a = 0`
/// is not second-order → [`OdeError::NotSupported`].
pub fn solve_linear_second_order(a: f64, b: f64, cc: f64, xvar: &str) -> Result<OdeSolution, OdeError> {
    if a == 0.0 {
        return Err(OdeError::NotSupported);
    }
    let disc = b * b - 4.0 * a * cc;
    let x = var(xvar);
    let sol = if disc > 1e-12 {
        // Distinct real roots: y = C1·e^{r1 x} + C2·e^{r2 x}.
        let s = disc.sqrt();
        let r1 = (-b + s) / (2.0 * a);
        let r2 = (-b - s) / (2.0 * a);
        add(
            mul(var("C1"), exp(mul(c(r1), x.clone()))),
            mul(var("C2"), exp(mul(c(r2), x))),
        )
    } else if disc.abs() <= 1e-12 {
        // Repeated root r: y = (C1 + C2·x)·e^{r x}.
        let r = -b / (2.0 * a);
        mul(add(var("C1"), mul(var("C2"), x.clone())), exp(mul(c(r), x)))
    } else {
        // Complex roots α ± βi: y = e^{α x}·(C1·cos(β x) + C2·sin(β x)).
        let alpha = -b / (2.0 * a);
        let beta = (-disc).sqrt() / (2.0 * a);
        mul(
            exp(mul(c(alpha), x.clone())),
            add(
                mul(var("C1"), cos(mul(c(beta), x.clone()))),
                mul(var("C2"), sin(mul(c(beta), x))),
            ),
        )
    };
    Ok(OdeSolution::Explicit(sol))
}

// ── PDE ──────────────────────────────────────────────────────────────────────────

/// The solution of a (supported) PDE.
#[derive(Debug, Clone, PartialEq)]
pub enum PdeSolution {
    /// `u(x, y) = F(invariant)` for an arbitrary differentiable `F` (method of
    /// characteristics for a first-order linear homogeneous PDE).
    GeneralFunctionOf { invariant: Expr },
}

/// The type of a second-order linear PDE.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PdeClass {
    /// `B² − 4AC < 0` (e.g. Laplace `uₓₓ + u_yy = 0`).
    Elliptic,
    /// `B² − 4AC = 0` (e.g. the heat equation).
    Parabolic,
    /// `B² − 4AC > 0` (e.g. the wave equation `uₓₓ − u_yy = 0`).
    Hyperbolic,
}

/// Solve `a·uₓ + b·u_y = 0` by the method of characteristics: `u` is an arbitrary function
/// of the invariant `b·x − a·y`. Requires `(a, b) ≠ (0, 0)`.
pub fn solve_first_order_linear_pde(
    a: f64,
    b: f64,
    xvar: &str,
    yvar: &str,
) -> Result<PdeSolution, OdeError> {
    if a == 0.0 && b == 0.0 {
        return Err(OdeError::NotSupported);
    }
    // Characteristic invariant ξ = b·x − a·y (constant along characteristics).
    let invariant = add(mul(c(b), var(xvar)), mul(c(-a), var(yvar)));
    Ok(PdeSolution::GeneralFunctionOf { invariant })
}

/// Classify the second-order linear PDE `A·uₓₓ + B·u_xy + C·u_yy + … = …` by the
/// discriminant `B² − 4AC`.
pub fn classify_second_order_pde(a_xx: f64, b_xy: f64, c_yy: f64) -> PdeClass {
    let disc = b_xy * b_xy - 4.0 * a_xx * c_yy;
    if disc < -1e-12 {
        PdeClass::Elliptic
    } else if disc <= 1e-12 {
        PdeClass::Parabolic
    } else {
        PdeClass::Hyperbolic
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::symbolic_algebra::{differentiate, pow, simplify, var};
    use std::collections::HashMap;

    fn env(pairs: &[(&str, f64)]) -> HashMap<String, f64> {
        pairs.iter().map(|&(k, v)| (k.to_string(), v)).collect()
    }

    #[test]
    fn separable_growth() {
        // y' = 1·y  (g = 1, h = y)  →  ∫dy/y = ∫dx + C  →  ln(y) = x + C.
        let sol = solve_separable(&c(1.0), &var("y"), "x", "y").unwrap();
        match sol {
            OdeSolution::Implicit { f_y, g_x } => {
                // f_y = ln(y) ; evaluate F(y=e) = 1.
                assert!((f_y.eval(&env(&[("y", std::f64::consts::E)])).unwrap() - 1.0).abs() < 1e-9);
                // g_x = x + C ; at x=2, C=3 → 5.
                assert!((g_x.eval(&env(&[("x", 2.0), ("C", 3.0)])).unwrap() - 5.0).abs() < 1e-9);
            }
            _ => panic!("expected implicit solution"),
        }
    }

    #[test]
    fn separable_fails_closed() {
        // h(y) = sin(y²) → ∫dy/sin(y²) is not in the table → NotIntegrable, not fabricated.
        let h = super::super::symbolic_algebra::sin(pow(var("y"), 2));
        assert_eq!(solve_separable(&c(1.0), &h, "x", "y").unwrap_err(), OdeError::NotIntegrable);
    }

    #[test]
    fn linear_first_order_satisfies_the_ode() {
        // y' + 2y = 6  →  y = 3 + C·e^{−2x}. Check y' + 2y = 6 at samples with C set.
        let OdeSolution::Explicit(y) = solve_linear_first_order(2.0, 6.0, "x") else {
            panic!()
        };
        let yp = simplify(&differentiate(&y, "x"));
        for &(x, cval) in &[(0.0, 1.0), (0.7, -2.0), (1.5, 4.0)] {
            let e = env(&[("x", x), ("C", cval)]);
            let residual = yp.eval(&e).unwrap() + 2.0 * y.eval(&e).unwrap();
            assert!((residual - 6.0).abs() < 1e-7, "residual {residual} at x={x}");
        }
    }

    /// Substitute an explicit solution into `a·y'' + b·y' + c·y` and assert ≈ 0.
    fn verify_second_order(y: &Expr, a: f64, b: f64, cc: f64) {
        let yp = simplify(&differentiate(y, "x"));
        let ypp = simplify(&differentiate(&yp, "x"));
        for &(x, c1, c2) in &[(0.0, 1.0, 0.5), (0.8, -1.0, 2.0), (1.7, 3.0, -1.5)] {
            let e = env(&[("x", x), ("C1", c1), ("C2", c2)]);
            let r = a * ypp.eval(&e).unwrap() + b * yp.eval(&e).unwrap() + cc * y.eval(&e).unwrap();
            assert!(r.abs() < 1e-6, "residual {r} at x={x}");
        }
    }

    #[test]
    fn second_order_distinct_real_roots() {
        // y'' − 3y' + 2y = 0 → roots 1, 2 → C1 e^x + C2 e^{2x}.
        let OdeSolution::Explicit(y) = solve_linear_second_order(1.0, -3.0, 2.0, "x").unwrap() else {
            panic!()
        };
        verify_second_order(&y, 1.0, -3.0, 2.0);
    }

    #[test]
    fn second_order_repeated_root() {
        // y'' − 2y' + y = 0 → repeated root 1 → (C1 + C2 x) e^x.
        let OdeSolution::Explicit(y) = solve_linear_second_order(1.0, -2.0, 1.0, "x").unwrap() else {
            panic!()
        };
        verify_second_order(&y, 1.0, -2.0, 1.0);
    }

    #[test]
    fn second_order_complex_roots() {
        // y'' + y = 0 → roots ±i → C1 cos x + C2 sin x.
        let OdeSolution::Explicit(y) = solve_linear_second_order(1.0, 0.0, 1.0, "x").unwrap() else {
            panic!()
        };
        verify_second_order(&y, 1.0, 0.0, 1.0);
    }

    #[test]
    fn second_order_rejects_non_second_order() {
        assert_eq!(solve_linear_second_order(0.0, 1.0, 1.0, "x").unwrap_err(), OdeError::NotSupported);
    }

    #[test]
    fn transport_pde_invariant_satisfies_equation() {
        // a uₓ + b u_y = 0 with a=2,b=3 → u = F(3x − 2y). Test F=(·)² : u=(3x−2y)².
        let PdeSolution::GeneralFunctionOf { invariant } =
            solve_first_order_linear_pde(2.0, 3.0, "x", "y").unwrap();
        let u = pow(invariant, 2);
        let ux = simplify(&differentiate(&u, "x"));
        let uy = simplify(&differentiate(&u, "y"));
        for &(x, y) in &[(0.0, 0.0), (1.0, 2.0), (-1.0, 0.5)] {
            let e = env(&[("x", x), ("y", y)]);
            let r = 2.0 * ux.eval(&e).unwrap() + 3.0 * uy.eval(&e).unwrap();
            assert!(r.abs() < 1e-7, "transport residual {r}");
        }
    }

    #[test]
    fn second_order_pde_classification() {
        // Laplace uₓₓ + u_yy: A=1,B=0,C=1 → elliptic.
        assert_eq!(classify_second_order_pde(1.0, 0.0, 1.0), PdeClass::Elliptic);
        // Wave uₓₓ − u_yy: A=1,B=0,C=−1 → hyperbolic.
        assert_eq!(classify_second_order_pde(1.0, 0.0, -1.0), PdeClass::Hyperbolic);
        // Heat-like uₓₓ (no u_yy): A=1,B=0,C=0 → parabolic.
        assert_eq!(classify_second_order_pde(1.0, 0.0, 0.0), PdeClass::Parabolic);
    }
}
