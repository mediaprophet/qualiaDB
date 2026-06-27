//! **Constructibility** — compass-and-straightedge feasibility decisions.
//!
//! A length, angle, or figure is *constructible* iff it can be produced with compass
//! and straightedge from a unit segment. This is the **feasibility gate** for the
//! NL→3D-fabrication pipeline: "can this geometric feature be made by this method?"
//! ([[project-nl-to-3d-fabrication-purpose]]). It also settles the three classical
//! impossibilities (doubling the cube, trisecting a general angle, squaring the circle)
//! and decides which regular polygons are constructible (Gauss–Wantzel).
//!
//! ## The decision procedures
//!
//! * **Degree criterion (Wantzel).** A constructible number is algebraic of degree a
//!   **power of two** over ℚ. So [`constructible_from_min_poly_degree`] decides every
//!   classical case from the minimal-polynomial degree alone: ∛2 (degree 3) → no
//!   (doubling the cube); `cos(20°)` (degree 3) → no (trisecting 60°).
//! * **Gauss–Wantzel** for the regular `n`-gon: constructible iff
//!   `n = 2^a · (product of *distinct* Fermat primes)`. See
//!   [`is_regular_polygon_constructible`]; the heptadecagon (`n = 17`) is the showcase.
//! * **Within the CAS** ([`super::symbolic_algebra::Expr`]): any well-formed *real*
//!   expression over rationals, the field operations, integer powers and **square
//!   roots** is constructible by construction — square roots are exactly the
//!   degree-2 tower. [`is_constructible_number`] confirms real-validity and reports the
//!   field-extension degree bound; it fails closed on a non-real `√(negative)` or a
//!   division by zero.
//!
//! Everything here is decidable and verifiable; nothing is fabricated.

use super::symbolic_algebra::Expr;
use std::collections::HashMap;

/// `n` is a power of two (`n ≥ 1`).
pub fn is_power_of_two(n: u64) -> bool {
    n != 0 && (n & (n - 1)) == 0
}

/// Wantzel's degree criterion: a number of minimal-polynomial degree `degree` over ℚ
/// is constructible **only if** `degree` is a power of two. (Necessary and, with the
/// quadratic-tower construction, the operative test for the classical problems.)
pub fn constructible_from_min_poly_degree(degree: u64) -> bool {
    is_power_of_two(degree)
}

/// Trial-division primality test (small inputs; polygon sides / Fermat candidates).
fn is_prime(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    if n % 2 == 0 {
        return n == 2;
    }
    let mut d = 3u64;
    while d.saturating_mul(d) <= n {
        if n % d == 0 {
            return false;
        }
        d += 2;
    }
    true
}

/// A Fermat prime is a prime of the form `2^(2^k) + 1` (3, 5, 17, 257, 65537, …). The
/// constructible odd-prime polygon sides are exactly these.
pub fn is_fermat_prime(n: u64) -> bool {
    if !is_prime(n) || n < 3 {
        return false;
    }
    let m = n - 1; // must be 2^(2^k)
    if !is_power_of_two(m) {
        return false;
    }
    // The exponent of m (= 2^k) must itself be a power of two.
    let exp = m.trailing_zeros() as u64;
    is_power_of_two(exp)
}

/// **Gauss–Wantzel**: the regular `n`-gon is constructible iff `n = 2^a · (product of
/// distinct Fermat primes)` — i.e. after stripping factors of two, the odd part is a
/// squarefree product of Fermat primes. (`n ≥ 3`.)
pub fn is_regular_polygon_constructible(n: u64) -> bool {
    if n < 3 {
        return false;
    }
    // Strip factors of two.
    let mut odd = n;
    while odd % 2 == 0 {
        odd /= 2;
    }
    if odd == 1 {
        return true; // a power-of-two-gon (square, octagon, …)
    }
    // Factor the odd part: each prime factor must be a Fermat prime appearing once.
    let mut p = 3u64;
    let mut rem = odd;
    while p.saturating_mul(p) <= rem {
        if rem % p == 0 {
            if !is_fermat_prime(p) {
                return false;
            }
            rem /= p;
            if rem % p == 0 {
                return false; // repeated factor → not squarefree → not constructible
            }
        }
        p += 2;
    }
    // Whatever remains is a prime factor > sqrt(rem); it must be a Fermat prime.
    rem == 1 || is_fermat_prime(rem)
}

/// The angle `2π/n` (a regular-`n`-gon central angle) is constructible iff the regular
/// `n`-gon is. So a 60° angle (`n = 6`) is constructible; 40° (`n = 9`) is not.
pub fn is_central_angle_constructible(n: u64) -> bool {
    is_regular_polygon_constructible(n)
}

/// The classical impossibilities, decided from the degree criterion (documented facts,
/// not hardcoded opinions):
/// **Doubling the cube** needs ∛2 — degree 3, not a power of two.
pub fn doubling_the_cube_constructible() -> bool {
    constructible_from_min_poly_degree(3)
}
/// **Trisecting a general angle** needs a root of `4x³ − 3x − cos θ` — degree 3.
pub fn trisecting_general_angle_constructible() -> bool {
    constructible_from_min_poly_degree(3)
}
/// **Squaring the circle** needs √π; π is transcendental (no finite minimal polynomial),
/// so it is not algebraic of any finite degree, let alone a power of two.
pub fn squaring_the_circle_constructible() -> bool {
    false
}

/// The verdict for a CAS expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstructibilityVerdict {
    /// Constructible; `degree_bound` is an upper bound on the field-extension degree
    /// (`2^(number of square roots)`), always a power of two.
    Constructible { degree_bound: u64 },
    /// `√(negative)` — not a real number, so not a constructible real.
    NotRealNumber,
    /// Division by zero — undefined.
    Undefined,
    /// A transcendental function (`exp`/`ln`/`sin`/`cos`/`tan`) appears: by
    /// Lindemann–Weierstrass its value at a nonzero algebraic argument is transcendental,
    /// hence not constructible (no finite-degree minimal polynomial).
    Transcendental,
}

/// Count `Sqrt` nodes (the field-extension tower height bound).
fn sqrt_count(expr: &Expr) -> u32 {
    match expr {
        Expr::Const(_) | Expr::Var(_) => 0,
        Expr::Neg(a) | Expr::Pow(a, _) => sqrt_count(a),
        Expr::Add(a, b) | Expr::Sub(a, b) | Expr::Mul(a, b) | Expr::Div(a, b) => {
            sqrt_count(a) + sqrt_count(b)
        }
        Expr::Sqrt(a) => 1 + sqrt_count(a),
        // Transcendental nodes contribute no square-root tower; count any sqrts in the arg.
        Expr::Exp(a) | Expr::Ln(a) | Expr::Sin(a) | Expr::Cos(a) | Expr::Tan(a) => sqrt_count(a),
    }
}

/// Detect a non-real (`√` of a constant-negative) or undefined (`÷0`) subexpression.
/// Returns the offending verdict, or `None` if numerically valid (or symbolic).
fn invalidity(expr: &Expr) -> Option<ConstructibilityVerdict> {
    let empty = HashMap::new();
    match expr {
        Expr::Const(_) | Expr::Var(_) => None,
        Expr::Neg(a) | Expr::Pow(a, _) => invalidity(a),
        Expr::Add(a, b) | Expr::Sub(a, b) | Expr::Mul(a, b) => {
            invalidity(a).or_else(|| invalidity(b))
        }
        Expr::Div(a, b) => {
            if let Some(0.0) = b.eval(&empty).filter(|v| *v == 0.0) {
                return Some(ConstructibilityVerdict::Undefined);
            }
            invalidity(a).or_else(|| invalidity(b))
        }
        Expr::Sqrt(a) => {
            if let Some(v) = a.eval(&empty) {
                if v < 0.0 {
                    return Some(ConstructibilityVerdict::NotRealNumber);
                }
            }
            invalidity(a)
        }
        // exp/ln/sin/cos/tan: transcendental unless the argument is the trivial 0
        // (e.g. sin 0 = 0, cos 0 = 1, exp 0 = 1 are constructible). Otherwise non-constructible.
        Expr::Exp(a) | Expr::Ln(a) | Expr::Sin(a) | Expr::Cos(a) | Expr::Tan(a) => {
            if let Some(v) = a.eval(&empty) {
                if v == 0.0 {
                    return invalidity(a);
                }
            }
            Some(ConstructibilityVerdict::Transcendental)
        }
    }
}

/// Decide constructibility of the number denoted by a CAS expression. Within `Expr`
/// (rationals + field ops + integer powers + square roots) every well-formed **real**
/// number is constructible; this confirms real-validity and reports the degree bound,
/// failing closed on `√(negative)` or `÷0`.
pub fn is_constructible_number(expr: &Expr) -> ConstructibilityVerdict {
    if let Some(bad) = invalidity(expr) {
        return bad;
    }
    let n = sqrt_count(expr).min(62);
    ConstructibilityVerdict::Constructible { degree_bound: 1u64 << n }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::symbolic_algebra::{add, c, div, sqrt};

    #[test]
    fn power_of_two() {
        for n in [1, 2, 4, 8, 16, 65536] {
            assert!(is_power_of_two(n));
        }
        for n in [0, 3, 6, 12, 17] {
            assert!(!is_power_of_two(n));
        }
    }

    #[test]
    fn fermat_primes_are_the_known_five() {
        for p in [3, 5, 17, 257, 65537] {
            assert!(is_fermat_prime(p), "{p} should be a Fermat prime");
        }
        for n in [2, 7, 9, 11, 13, 15, 65539] {
            assert!(!is_fermat_prime(n), "{n} should not be a Fermat prime");
        }
    }

    #[test]
    fn gauss_wantzel_regular_polygons() {
        // Constructible: powers of two, Fermat primes, products of *distinct* ones.
        for n in [3, 4, 5, 6, 8, 10, 12, 15, 16, 17, 20, 257] {
            assert!(is_regular_polygon_constructible(n), "{n}-gon should be constructible");
        }
        // Not: non-Fermat odd primes, repeated Fermat factor (9 = 3²), their multiples.
        for n in [7, 9, 11, 13, 14, 18, 19, 21, 23, 25] {
            assert!(!is_regular_polygon_constructible(n), "{n}-gon should NOT be constructible");
        }
    }

    #[test]
    fn classical_impossibilities() {
        assert!(!doubling_the_cube_constructible());
        assert!(!trisecting_general_angle_constructible());
        assert!(!squaring_the_circle_constructible());
        // The 60° angle (n=6) is constructible; trisecting it to 20° (n=18) is not.
        assert!(is_central_angle_constructible(6));
        assert!(!is_central_angle_constructible(18));
    }

    #[test]
    fn degree_criterion_decides_quadratics_and_cubics() {
        assert!(constructible_from_min_poly_degree(2)); // √2, golden ratio
        assert!(constructible_from_min_poly_degree(4)); // nested square roots
        assert!(!constructible_from_min_poly_degree(3)); // ∛2
        assert!(!constructible_from_min_poly_degree(0));
    }

    #[test]
    fn expression_constructibility_and_degree_bound() {
        // √2 → constructible, degree bound 2.
        match is_constructible_number(&sqrt(c(2.0))) {
            ConstructibilityVerdict::Constructible { degree_bound } => assert_eq!(degree_bound, 2),
            v => panic!("√2 should be constructible, got {v:?}"),
        }
        // √(1+√2) → two nested roots → degree bound 4.
        let nested = sqrt(add(c(1.0), sqrt(c(2.0))));
        match is_constructible_number(&nested) {
            ConstructibilityVerdict::Constructible { degree_bound } => assert_eq!(degree_bound, 4),
            v => panic!("nested root should be constructible, got {v:?}"),
        }
    }

    #[test]
    fn transcendental_subexpression_is_not_constructible() {
        use crate::specialized_libs::symbolic_algebra::{cos, sin, var};
        // sin(1) is transcendental → not constructible.
        assert_eq!(is_constructible_number(&sin(c(1.0))), ConstructibilityVerdict::Transcendental);
        // A symbolic cos(x) (unknown argument) is treated transcendental, not fabricated constructible.
        assert_eq!(is_constructible_number(&cos(var("x"))), ConstructibilityVerdict::Transcendental);
        // But cos(0) = 1 (trivial argument) remains constructible.
        assert!(matches!(
            is_constructible_number(&cos(c(0.0))),
            ConstructibilityVerdict::Constructible { .. }
        ));
    }

    #[test]
    fn fails_closed_on_non_real_and_undefined() {
        assert_eq!(is_constructible_number(&sqrt(c(-1.0))), ConstructibilityVerdict::NotRealNumber);
        assert_eq!(is_constructible_number(&div(c(1.0), c(0.0))), ConstructibilityVerdict::Undefined);
    }
}
