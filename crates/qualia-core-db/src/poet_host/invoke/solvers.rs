//! Solver expose seams — number theory, special functions, interpolation,
//! calculus (simple-argument functions), transforms, geometric algebra,
//! and fuzzy query membership functions.

use super::args;
use crate::solvers;
use poet_vibe::{Diagnostic, Span, Value};

// ── Number theory ───────────────────────────────────────────────────

/// `NumberTheory.next_prime` — next prime ≥ n.
pub fn next_prime(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let n = args::rec_u64(args, "n").ok_or_else(|| args::bad(span, "next_prime needs n"))?;
    Ok(Value::U64(solvers::number_theory::primes::next_prime(n)))
}

/// `NumberTheory.prime_factors` — prime factorization.
pub fn prime_factors(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let n = args::rec_u64(args, "n").ok_or_else(|| args::bad(span, "prime_factors needs n"))?;
    let factors = solvers::number_theory::primes::prime_factors(n);
    let records: Vec<Value> = factors
        .iter()
        .map(|(p, e)| {
            args::record([
                ("prime", Value::U64(*p)),
                ("exponent", Value::U64(*e as u64)),
            ])
        })
        .collect();
    Ok(Value::List(records))
}

/// `NumberTheory.divisors` — all divisors of n.
pub fn divisors(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let n = args::rec_u64(args, "n").ok_or_else(|| args::bad(span, "divisors needs n"))?;
    let divs = solvers::number_theory::primes::divisors(n);
    Ok(Value::List(divs.iter().map(|&d| Value::U64(d)).collect()))
}

/// `NumberTheory.euler_totient` — Euler's totient φ(n).
pub fn euler_totient(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let n = args::rec_u64(args, "n").ok_or_else(|| args::bad(span, "euler_totient needs n"))?;
    Ok(Value::U64(
        solvers::number_theory::arithmetic_functions::euler_totient(n),
    ))
}

/// `NumberTheory.mobius` — Möbius function μ(n).
pub fn mobius(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let n = args::rec_u64(args, "n").ok_or_else(|| args::bad(span, "mobius needs n"))?;
    Ok(Value::I64(
        solvers::number_theory::arithmetic_functions::mobius(n) as i64,
    ))
}

/// `NumberTheory.divisor_count` — number of divisors d(n).
pub fn divisor_count(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let n = args::rec_u64(args, "n").ok_or_else(|| args::bad(span, "divisor_count needs n"))?;
    Ok(Value::U64(
        solvers::number_theory::arithmetic_functions::divisor_count(n),
    ))
}

/// `NumberTheory.divisor_sum` — sum of divisors σ(n).
pub fn divisor_sum(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let n = args::rec_u64(args, "n").ok_or_else(|| args::bad(span, "divisor_sum needs n"))?;
    Ok(Value::U64(
        solvers::number_theory::arithmetic_functions::divisor_sum(n),
    ))
}

/// `NumberTheory.mod_pow` — modular exponentiation (base^exp mod m).
pub fn mod_pow(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let base = args::rec_u64(args, "base").ok_or_else(|| args::bad(span, "mod_pow needs base"))?;
    let exp = args::rec_u64(args, "exp").ok_or_else(|| args::bad(span, "mod_pow needs exp"))?;
    let m =
        args::rec_u64(args, "modulus").ok_or_else(|| args::bad(span, "mod_pow needs modulus"))?;
    Ok(Value::U64(solvers::number_theory::modular::mod_pow(
        base, exp, m,
    )))
}

/// `NumberTheory.mod_inverse` — modular multiplicative inverse.
pub fn mod_inverse(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = args::rec_u64(args, "a").ok_or_else(|| args::bad(span, "mod_inverse needs a"))?;
    let m = args::rec_u64(args, "modulus")
        .ok_or_else(|| args::bad(span, "mod_inverse needs modulus"))?;
    match solvers::number_theory::modular::mod_inverse(a, m) {
        Some(r) => Ok(Value::U64(r)),
        None => Ok(Value::Null),
    }
}

/// `NumberTheory.factorial` — n! (returns f64 to accommodate large values).
pub fn factorial(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let n = args::rec_u64(args, "n").ok_or_else(|| args::bad(span, "factorial needs n"))?;
    match solvers::number_theory::combinatorics::factorial(n) {
        Some(f) => Ok(Value::F64(f as f64)),
        None => Ok(Value::Null),
    }
}

/// `NumberTheory.binomial` — C(n, k) (returns f64 to accommodate large values).
pub fn binomial(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let n = args::rec_u64(args, "n").ok_or_else(|| args::bad(span, "binomial needs n"))?;
    let k = args::rec_u64(args, "k").ok_or_else(|| args::bad(span, "binomial needs k"))?;
    match solvers::number_theory::combinatorics::binomial(n, k) {
        Some(c) => Ok(Value::F64(c as f64)),
        None => Ok(Value::Null),
    }
}

/// `NumberTheory.partitions` — partition function p(n).
pub fn partitions(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let n = args::rec_u64(args, "n").ok_or_else(|| args::bad(span, "partitions needs n"))?;
    Ok(Value::U64(
        solvers::number_theory::combinatorics::partitions(n),
    ))
}

/// `NumberTheory.catalan` — Catalan number C(n).
pub fn catalan(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let n = args::rec_u64(args, "n").ok_or_else(|| args::bad(span, "catalan needs n"))?;
    match solvers::number_theory::combinatorics::catalan(n) {
        Some(c) => Ok(Value::F64(c as f64)),
        None => Ok(Value::Null),
    }
}

/// `NumberTheory.stirling_second` — Stirling number of the second kind S(n,k).
pub fn stirling_second(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let n = args::rec_u64(args, "n").ok_or_else(|| args::bad(span, "stirling_second needs n"))?;
    let k = args::rec_u64(args, "k").ok_or_else(|| args::bad(span, "stirling_second needs k"))?;
    match solvers::number_theory::combinatorics::stirling_second(n, k) {
        Some(s) => Ok(Value::F64(s as f64)),
        None => Ok(Value::Null),
    }
}

/// `NumberTheory.stirling_first` — Stirling number of the first kind s(n,k).
pub fn stirling_first(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let n = args::rec_u64(args, "n").ok_or_else(|| args::bad(span, "stirling_first needs n"))?;
    let k = args::rec_u64(args, "k").ok_or_else(|| args::bad(span, "stirling_first needs k"))?;
    match solvers::number_theory::combinatorics::stirling_first(n, k) {
        Some(s) => Ok(Value::F64(s as f64)),
        None => Ok(Value::Null),
    }
}

// ── Special functions ───────────────────────────────────────────────

/// `SpecialFunctions.airy_ai` — Airy Ai function.
pub fn airy_ai(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_f64(args, "x").ok_or_else(|| args::bad(span, "airy_ai needs x"))?;
    Ok(Value::F64(solvers::special_functions::airy::airy_ai(x)))
}

/// `SpecialFunctions.airy_bi` — Airy Bi function.
pub fn airy_bi(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_f64(args, "x").ok_or_else(|| args::bad(span, "airy_bi needs x"))?;
    Ok(Value::F64(solvers::special_functions::airy::airy_bi(x)))
}

/// `SpecialFunctions.zeta` — Riemann zeta function ζ(s).
pub fn zeta(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let s = args::rec_f64(args, "s").ok_or_else(|| args::bad(span, "zeta needs s"))?;
    match solvers::special_functions::zeta::zeta(s) {
        Some(z) => Ok(Value::F64(z)),
        None => Ok(Value::Null),
    }
}

/// `SpecialFunctions.legendre` — Legendre polynomial P_n(x).
pub fn legendre(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let n = args::rec_u64(args, "n").ok_or_else(|| args::bad(span, "legendre needs n"))? as u32;
    let x = args::rec_f64(args, "x").ok_or_else(|| args::bad(span, "legendre needs x"))?;
    Ok(Value::F64(
        solvers::special_functions::orthogonal::legendre(n, x),
    ))
}

/// `SpecialFunctions.chebyshev_t` — Chebyshev polynomial of the first kind T_n(x).
pub fn chebyshev_t(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let n = args::rec_u64(args, "n").ok_or_else(|| args::bad(span, "chebyshev_t needs n"))? as u32;
    let x = args::rec_f64(args, "x").ok_or_else(|| args::bad(span, "chebyshev_t needs x"))?;
    Ok(Value::F64(
        solvers::special_functions::orthogonal::chebyshev_t(n, x),
    ))
}

/// `SpecialFunctions.chebyshev_u` — Chebyshev polynomial of the second kind U_n(x).
pub fn chebyshev_u(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let n = args::rec_u64(args, "n").ok_or_else(|| args::bad(span, "chebyshev_u needs n"))? as u32;
    let x = args::rec_f64(args, "x").ok_or_else(|| args::bad(span, "chebyshev_u needs x"))?;
    Ok(Value::F64(
        solvers::special_functions::orthogonal::chebyshev_u(n, x),
    ))
}

/// `SpecialFunctions.hermite` — Hermite polynomial H_n(x).
pub fn hermite(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let n = args::rec_u64(args, "n").ok_or_else(|| args::bad(span, "hermite needs n"))? as u32;
    let x = args::rec_f64(args, "x").ok_or_else(|| args::bad(span, "hermite needs x"))?;
    Ok(Value::F64(solvers::special_functions::orthogonal::hermite(
        n, x,
    )))
}

/// `SpecialFunctions.laguerre` — Laguerre polynomial L_n(x).
pub fn laguerre(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let n = args::rec_u64(args, "n").ok_or_else(|| args::bad(span, "laguerre needs n"))? as u32;
    let x = args::rec_f64(args, "x").ok_or_else(|| args::bad(span, "laguerre needs x"))?;
    Ok(Value::F64(
        solvers::special_functions::orthogonal::laguerre(n, x),
    ))
}

/// `SpecialFunctions.bessel_j` — Bessel function of the first kind J_n(x).
pub fn bessel_j(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let n = args::rec_i64(args, "n").ok_or_else(|| args::bad(span, "bessel_j needs n"))? as i32;
    let x = args::rec_f64(args, "x").ok_or_else(|| args::bad(span, "bessel_j needs x"))?;
    Ok(Value::F64(solvers::special_functions::bessel::bessel_j(
        n, x,
    )))
}

/// `SpecialFunctions.bessel_i` — Modified Bessel function I_n(x).
pub fn bessel_i(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let n = args::rec_i64(args, "n").ok_or_else(|| args::bad(span, "bessel_i needs n"))? as i32;
    let x = args::rec_f64(args, "x").ok_or_else(|| args::bad(span, "bessel_i needs x"))?;
    Ok(Value::F64(solvers::special_functions::bessel::bessel_i(
        n, x,
    )))
}

/// `SpecialFunctions.bessel_y` — Bessel function of the second kind Y_n(x).
pub fn bessel_y(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let n = args::rec_u64(args, "n").ok_or_else(|| args::bad(span, "bessel_y needs n"))? as u32;
    let x = args::rec_f64(args, "x").ok_or_else(|| args::bad(span, "bessel_y needs x"))?;
    match solvers::special_functions::bessel::bessel_y(n, x) {
        Some(y) => Ok(Value::F64(y)),
        None => Ok(Value::Null),
    }
}

/// `SpecialFunctions.bessel_k` — Modified Bessel function K_n(x).
pub fn bessel_k(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let n = args::rec_u64(args, "n").ok_or_else(|| args::bad(span, "bessel_k needs n"))? as u32;
    let x = args::rec_f64(args, "x").ok_or_else(|| args::bad(span, "bessel_k needs x"))?;
    match solvers::special_functions::bessel::bessel_k(n, x) {
        Some(k) => Ok(Value::F64(k)),
        None => Ok(Value::Null),
    }
}

// ── Interpolation ───────────────────────────────────────────────────

/// `Interpolation.linear_interp` — linear interpolation at x.
pub fn linear_interp(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let xs =
        args::rec_f64_list(args, "xs").ok_or_else(|| args::bad(span, "linear_interp needs xs"))?;
    let ys =
        args::rec_f64_list(args, "ys").ok_or_else(|| args::bad(span, "linear_interp needs ys"))?;
    let x = args::rec_f64(args, "x").ok_or_else(|| args::bad(span, "linear_interp needs x"))?;
    match solvers::interpolation::spline::linear_interp(&xs, &ys, x) {
        Ok(v) => Ok(Value::F64(v)),
        Err(e) => Err(args::bad(span, format!("linear_interp: {e:?}"))),
    }
}

/// `Interpolation.lagrange_eval` — Lagrange interpolation at x.
pub fn lagrange_eval(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let xs =
        args::rec_f64_list(args, "xs").ok_or_else(|| args::bad(span, "lagrange_eval needs xs"))?;
    let ys =
        args::rec_f64_list(args, "ys").ok_or_else(|| args::bad(span, "lagrange_eval needs ys"))?;
    let x = args::rec_f64(args, "x").ok_or_else(|| args::bad(span, "lagrange_eval needs x"))?;
    match solvers::interpolation::lagrange::lagrange_eval(&xs, &ys, x) {
        Ok(v) => Ok(Value::F64(v)),
        Err(e) => Err(args::bad(span, format!("lagrange_eval: {e:?}"))),
    }
}

/// `Interpolation.newton_coefficients` — Newton interpolation coefficients.
pub fn newton_coefficients(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let xs = args::rec_f64_list(args, "xs")
        .ok_or_else(|| args::bad(span, "newton_coefficients needs xs"))?;
    let ys = args::rec_f64_list(args, "ys")
        .ok_or_else(|| args::bad(span, "newton_coefficients needs ys"))?;
    match solvers::interpolation::lagrange::newton_coefficients(&xs, &ys) {
        Ok(coef) => Ok(args::f64_list_value(coef)),
        Err(e) => Err(args::bad(span, format!("newton_coefficients: {e:?}"))),
    }
}

/// `Interpolation.newton_eval` — Newton interpolation eval at x.
pub fn newton_eval(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let xs =
        args::rec_f64_list(args, "xs").ok_or_else(|| args::bad(span, "newton_eval needs xs"))?;
    let coef = args::rec_f64_list(args, "coef")
        .ok_or_else(|| args::bad(span, "newton_eval needs coef"))?;
    let x = args::rec_f64(args, "x").ok_or_else(|| args::bad(span, "newton_eval needs x"))?;
    Ok(Value::F64(solvers::interpolation::lagrange::newton_eval(
        &xs, &coef, x,
    )))
}

/// `Interpolation.poly_fit` — polynomial least-squares fit.
pub fn poly_fit(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let xs = args::rec_f64_list(args, "xs").ok_or_else(|| args::bad(span, "poly_fit needs xs"))?;
    let ys = args::rec_f64_list(args, "ys").ok_or_else(|| args::bad(span, "poly_fit needs ys"))?;
    let degree = args::rec_u64(args, "degree")
        .ok_or_else(|| args::bad(span, "poly_fit needs degree"))? as usize;
    match solvers::interpolation::least_squares::poly_fit(&xs, &ys, degree) {
        Ok(coef) => Ok(args::f64_list_value(coef)),
        Err(e) => Err(args::bad(span, format!("poly_fit: {e:?}"))),
    }
}

/// `Interpolation.poly_eval` — evaluate polynomial coefficients at x.
pub fn poly_eval(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let coef =
        args::rec_f64_list(args, "coef").ok_or_else(|| args::bad(span, "poly_eval needs coef"))?;
    let x = args::rec_f64(args, "x").ok_or_else(|| args::bad(span, "poly_eval needs x"))?;
    Ok(Value::F64(
        solvers::interpolation::least_squares::poly_eval(&coef, x),
    ))
}

// ── Fuzzy query membership ──────────────────────────────────────────

/// `FuzzyQuery.triangular` — triangular membership function.
pub fn fuzzy_triangular(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_f64(args, "x").ok_or_else(|| args::bad(span, "fuzzy_triangular needs x"))?;
    let a = args::rec_f64(args, "a").ok_or_else(|| args::bad(span, "fuzzy_triangular needs a"))?;
    let m = args::rec_f64(args, "m").ok_or_else(|| args::bad(span, "fuzzy_triangular needs m"))?;
    let b = args::rec_f64(args, "b").ok_or_else(|| args::bad(span, "fuzzy_triangular needs b"))?;
    Ok(Value::F64(solvers::fuzzy_query::membership::triangular(
        x, a, m, b,
    )))
}

/// `FuzzyQuery.trapezoidal` — trapezoidal membership function.
pub fn fuzzy_trapezoidal(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_f64(args, "x").ok_or_else(|| args::bad(span, "fuzzy_trapezoidal needs x"))?;
    let a = args::rec_f64(args, "a").ok_or_else(|| args::bad(span, "fuzzy_trapezoidal needs a"))?;
    let b = args::rec_f64(args, "b").ok_or_else(|| args::bad(span, "fuzzy_trapezoidal needs b"))?;
    let c = args::rec_f64(args, "c").ok_or_else(|| args::bad(span, "fuzzy_trapezoidal needs c"))?;
    let d = args::rec_f64(args, "d").ok_or_else(|| args::bad(span, "fuzzy_trapezoidal needs d"))?;
    Ok(Value::F64(solvers::fuzzy_query::membership::trapezoidal(
        x, a, b, c, d,
    )))
}

/// `FuzzyQuery.approximately` — "approximately x" membership.
pub fn fuzzy_approximately(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x =
        args::rec_f64(args, "x").ok_or_else(|| args::bad(span, "fuzzy_approximately needs x"))?;
    let target = args::rec_f64(args, "target")
        .ok_or_else(|| args::bad(span, "fuzzy_approximately needs target"))?;
    let tol = args::rec_f64(args, "tol")
        .ok_or_else(|| args::bad(span, "fuzzy_approximately needs tol"))?;
    Ok(Value::F64(solvers::fuzzy_query::membership::approximately(
        x, target, tol,
    )))
}

/// `FuzzyQuery.ramp_up` — ramp-up membership.
pub fn fuzzy_ramp_up(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_f64(args, "x").ok_or_else(|| args::bad(span, "fuzzy_ramp_up needs x"))?;
    let a = args::rec_f64(args, "a").ok_or_else(|| args::bad(span, "fuzzy_ramp_up needs a"))?;
    let b = args::rec_f64(args, "b").ok_or_else(|| args::bad(span, "fuzzy_ramp_up needs b"))?;
    Ok(Value::F64(solvers::fuzzy_query::membership::ramp_up(
        x, a, b,
    )))
}

/// `FuzzyQuery.ramp_down` — ramp-down membership.
pub fn fuzzy_ramp_down(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_f64(args, "x").ok_or_else(|| args::bad(span, "fuzzy_ramp_down needs x"))?;
    let a = args::rec_f64(args, "a").ok_or_else(|| args::bad(span, "fuzzy_ramp_down needs a"))?;
    let b = args::rec_f64(args, "b").ok_or_else(|| args::bad(span, "fuzzy_ramp_down needs b"))?;
    Ok(Value::F64(solvers::fuzzy_query::membership::ramp_down(
        x, a, b,
    )))
}

/// `FuzzyQuery.much_greater_than` — "much greater than" membership.
pub fn fuzzy_much_greater_than(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_f64(args, "x")
        .ok_or_else(|| args::bad(span, "fuzzy_much_greater_than needs x"))?;
    let reference = args::rec_f64(args, "reference")
        .ok_or_else(|| args::bad(span, "fuzzy_much_greater_than needs reference"))?;
    let spread = args::rec_f64(args, "spread")
        .ok_or_else(|| args::bad(span, "fuzzy_much_greater_than needs spread"))?;
    Ok(Value::F64(
        solvers::fuzzy_query::membership::much_greater_than(x, reference, spread),
    ))
}

/// `FuzzyQuery.much_less_than` — "much less than" membership.
pub fn fuzzy_much_less_than(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x =
        args::rec_f64(args, "x").ok_or_else(|| args::bad(span, "fuzzy_much_less_than needs x"))?;
    let reference = args::rec_f64(args, "reference")
        .ok_or_else(|| args::bad(span, "fuzzy_much_less_than needs reference"))?;
    let spread = args::rec_f64(args, "spread")
        .ok_or_else(|| args::bad(span, "fuzzy_much_less_than needs spread"))?;
    Ok(Value::F64(
        solvers::fuzzy_query::membership::much_less_than(x, reference, spread),
    ))
}

// ── Linear algebra (decompositions not already in math module) ─────

/// `LinearAlgebra.lu_decompose` — LU decomposition with partial pivoting.
/// Args: { a: [[f64]] }
pub fn lu_decompose(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let (data, n, _) = parse_square_matrix(args, "a", "lu_decompose", span)?;
    match solvers::linear_algebra::lu::lu_decompose(n, &data) {
        Ok(lu) => Ok(args::record([
            ("lu", args::f64_list_value(lu.lu)),
            (
                "pivots",
                Value::List(lu.pivots.iter().map(|&p| Value::U64(p as u64)).collect()),
            ),
            ("sign", Value::F64(lu.sign)),
            ("singular", Value::Bool(lu.singular)),
            ("n", Value::U64(n as u64)),
        ])),
        Err(e) => Err(args::bad(span, format!("lu_decompose: {e:?}"))),
    }
}

/// `LinearAlgebra.lu_solve` — solve Ax=b using LU decomposition.
/// Args: { a: [[f64]], b: [f64] }
pub fn lu_solve(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let (data, n, _) = parse_square_matrix(args, "a", "lu_solve", span)?;
    let b = args::rec_f64_list(args, "b").ok_or_else(|| args::bad(span, "lu_solve needs b"))?;
    match solvers::linear_algebra::lu::lu_solve(n, &data, &b) {
        Some(x) => Ok(args::f64_list_value(x)),
        None => Err(args::bad(span, "lu_solve: singular matrix")),
    }
}

/// `LinearAlgebra.cholesky_factor` — Cholesky decomposition of SPD matrix.
/// Args: { a: [[f64]] }
pub fn cholesky_factor(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let (data, n, _) = parse_square_matrix(args, "a", "cholesky_factor", span)?;
    let mut l = vec![0.0; n * n];
    match solvers::linear_algebra::cholesky::cholesky_factor(n, &data, &mut l) {
        Ok(()) => Ok(args::record([
            ("l", args::f64_list_value(l)),
            ("n", Value::U64(n as u64)),
        ])),
        Err(e) => Err(args::bad(span, format!("cholesky_factor: {e:?}"))),
    }
}

/// `LinearAlgebra.cholesky_determinant` — determinant via Cholesky factor.
/// Args: { l: [f64], n: u64 }
pub fn cholesky_determinant(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let l = args::rec_f64_list(args, "l")
        .ok_or_else(|| args::bad(span, "cholesky_determinant needs l"))?;
    let n = args::rec_u64(args, "n")
        .ok_or_else(|| args::bad(span, "cholesky_determinant needs n"))? as usize;
    Ok(Value::F64(
        solvers::linear_algebra::cholesky::cholesky_determinant(n, &l),
    ))
}

/// `LinearAlgebra.characteristic_polynomial` — coefficients of char poly.
/// Args: { a: [[f64]] }
pub fn characteristic_polynomial(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let (data, n, _) = parse_square_matrix(args, "a", "characteristic_polynomial", span)?;
    match solvers::linear_algebra::spectral::characteristic_polynomial(n, &data) {
        Ok(coeffs) => Ok(args::f64_list_value(coeffs)),
        Err(e) => Err(args::bad(span, format!("characteristic_polynomial: {e:?}"))),
    }
}

/// `LinearAlgebra.eigenvalues_general` — eigenvalues of a general matrix.
/// Args: { a: [[f64]] }
pub fn eigenvalues_general(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let (data, n, _) = parse_square_matrix(args, "a", "eigenvalues_general", span)?;
    match solvers::linear_algebra::spectral::eigenvalues_general(n, &data) {
        Ok(eigs) => {
            let records: Vec<Value> = eigs
                .iter()
                .map(|c| args::record([("re", Value::F64(c.re)), ("im", Value::F64(c.im))]))
                .collect();
            Ok(Value::List(records))
        }
        Err(e) => Err(args::bad(span, format!("eigenvalues_general: {e:?}"))),
    }
}

/// Parse a square matrix from a VibeScript list-of-lists record field.
fn parse_square_matrix(
    v: &Value,
    key: &str,
    fn_name: &str,
    span: Span,
) -> Result<(Vec<f64>, usize, usize), Diagnostic> {
    let list_val = args::rec(v, key)
        .ok_or_else(|| args::bad(span, format!("{fn_name} needs {key}: [[f64]]")))?;
    let rows = args::list(&list_val)
        .ok_or_else(|| args::bad(span, format!("{fn_name}: {key} must be a list")))?;
    let n = rows.len();
    if n == 0 {
        return Err(args::bad(span, format!("{fn_name}: empty matrix")));
    }
    let p = args::f64s(&rows[0])
        .ok_or_else(|| args::bad(span, format!("{fn_name}: invalid row")))?
        .len();
    if p != n {
        return Err(args::bad(span, format!("{fn_name}: matrix must be square")));
    }
    let mut data = Vec::with_capacity(n * n);
    for row in rows {
        let cells =
            args::f64s(row).ok_or_else(|| args::bad(span, format!("{fn_name}: invalid row")))?;
        if cells.len() != p {
            return Err(args::bad(span, format!("{fn_name}: ragged matrix")));
        }
        data.extend_from_slice(&cells);
    }
    Ok((data, n, p))
}
