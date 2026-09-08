//! Host bindings for dense univariate [`Polynomial`] algebra.
//!
//! Wraps `crate::specialized_libs::polynomial_algebra::Polynomial` methods that had
//! no exact `PolynomialAlgebra.*` Host twin (`div_rem`, `derivative`, `monic`, `resultant`).

use super::super::args;
use crate::specialized_libs::polynomial_algebra::Polynomial;
use vibe::{Diagnostic, Span, Value};

fn poly_from(args_v: &Value, key: &str, span: Span) -> Result<Polynomial, Diagnostic> {
    let coeffs = args::rec_f64_list(args_v, key)
        .ok_or_else(|| args::bad(span, format!("PolynomialAlgebra needs {key}: coeffs")))?;
    Ok(Polynomial::new(coeffs))
}

fn coeffs_value(p: &Polynomial) -> Value {
    args::f64_list_value(p.coeffs().to_vec())
}

/// `PolynomialAlgebra.div_rem` — long division `(q, r)` with fail-closed on zero divisor.
/// Args: `{ a, b }` (little-endian coeffs). Out: `{ q, r }` or error.
pub fn div_rem(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = poly_from(args_v, "a", span)?;
    let b = poly_from(args_v, "b", span)?;
    let (q, r) = a
        .div_rem(&b)
        .ok_or_else(|| args::bad(span, "div_rem: division by zero polynomial"))?;
    Ok(args::record([("q", coeffs_value(&q)), ("r", coeffs_value(&r))]))
}

/// `PolynomialAlgebra.derivative` — first derivative. Args: `{ a }`. Out: `{ coeffs }`.
pub fn derivative(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = poly_from(args_v, "a", span)?;
    Ok(args::record([("coeffs", coeffs_value(&a.derivative()))]))
}

/// `PolynomialAlgebra.monic` — scale so leading coeff is 1. Args: `{ a }`. Out: `{ coeffs }`.
pub fn monic(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = poly_from(args_v, "a", span)?;
    Ok(args::record([("coeffs", coeffs_value(&a.monic()))]))
}

/// `PolynomialAlgebra.resultant` — Euclidean resultant. Args: `{ a, b }`. Out: `{ value }`.
pub fn resultant(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = poly_from(args_v, "a", span)?;
    let b = poly_from(args_v, "b", span)?;
    Ok(args::record([("value", Value::F64(a.resultant(&b)))]))
}

/// `PolynomialAlgebra.add` — coefficient-wise sum. Args: `{ a, b }`. Out: `{ coeffs }`.
pub fn add(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = poly_from(args_v, "a", span)?;
    let b = poly_from(args_v, "b", span)?;
    Ok(args::record([("coeffs", coeffs_value(&a.add(&b)))]))
}

/// `PolynomialAlgebra.sub` — `a − b`. Args: `{ a, b }`. Out: `{ coeffs }`.
pub fn sub(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = poly_from(args_v, "a", span)?;
    let b = poly_from(args_v, "b", span)?;
    Ok(args::record([("coeffs", coeffs_value(&a.sub(&b)))]))
}

/// `PolynomialAlgebra.mul` — schoolbook product. Args: `{ a, b }`. Out: `{ coeffs }`.
pub fn mul(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = poly_from(args_v, "a", span)?;
    let b = poly_from(args_v, "b", span)?;
    Ok(args::record([("coeffs", coeffs_value(&a.mul(&b)))]))
}

/// `PolynomialAlgebra.degree` — degree, or `null` for the zero polynomial.
/// Args: `{ a }`. Out: `{ degree }`.
pub fn degree(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = poly_from(args_v, "a", span)?;
    Ok(args::record([(
        "degree",
        match a.degree() {
            Some(d) => Value::U64(d as u64),
            None => Value::Null,
        },
    )]))
}

/// `PolynomialAlgebra.leading` — leading coefficient (`0` if zero poly).
/// Args: `{ a }`. Out: `{ value }`.
pub fn leading(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = poly_from(args_v, "a", span)?;
    Ok(args::record([("value", Value::F64(a.leading()))]))
}

/// `PolynomialAlgebra.is_zero` — true iff the coefficient list is empty after trim.
/// Args: `{ a }`. Out: `{ value }` (bool).
pub fn is_zero(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = poly_from(args_v, "a", span)?;
    Ok(args::record([("value", Value::Bool(a.is_zero()))]))
}

/// `PolynomialAlgebra.gcd` — monic Euclidean gcd. Args: `{ a, b }`. Out: `{ coeffs }`.
pub fn gcd(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = poly_from(args_v, "a", span)?;
    let b = poly_from(args_v, "b", span)?;
    Ok(args::record([("coeffs", coeffs_value(&a.gcd(&b)))]))
}

/// `PolynomialAlgebra.scale` — multiply every coefficient by `s`.
/// Args: `{ a, s }`. Out: `{ coeffs }`.
pub fn scale(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = poly_from(args_v, "a", span)?;
    let s = args::rec_f64(args_v, "s")
        .ok_or_else(|| args::bad(span, "PolynomialAlgebra.scale needs s: f64"))?;
    Ok(args::record([("coeffs", coeffs_value(&a.scale(s)))]))
}

/// `PolynomialAlgebra.eval` — Horner evaluation at `x`.
/// Args: `{ a, x }`. Out: `{ value }`.
pub fn eval(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = poly_from(args_v, "a", span)?;
    let x = args::rec_f64(args_v, "x")
        .ok_or_else(|| args::bad(span, "PolynomialAlgebra.eval needs x: f64"))?;
    Ok(args::record([("value", Value::F64(a.eval(x)))]))
}

/// `PolynomialAlgebra.zero` — the zero polynomial. Args: `{}`. Out: `{ coeffs }`.
pub fn zero(_args_v: &Value, _span: Span) -> Result<Value, Diagnostic> {
    Ok(args::record([("coeffs", coeffs_value(&Polynomial::zero()))]))
}

/// `PolynomialAlgebra.constant` — constant polynomial `c`.
/// Args: `{ c }`. Out: `{ coeffs }`.
pub fn constant(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let c = args::rec_f64(args_v, "c")
        .ok_or_else(|| args::bad(span, "PolynomialAlgebra.constant needs c: f64"))?;
    Ok(args::record([(
        "coeffs",
        coeffs_value(&Polynomial::constant(c)),
    )]))
}

/// `PolynomialAlgebra.coeffs` — borrow trimmed little-endian coefficients.
/// Args: `{ a }`. Out: `{ coeffs }` (same list after trim of near-zero leading terms).
pub fn coeffs(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = poly_from(args_v, "a", span)?;
    Ok(args::record([("coeffs", coeffs_value(&a))]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn span() -> Span {
        Span { start: 0, end: 0 }
    }

    fn f64s(v: &Value, key: &str) -> Vec<f64> {
        args::rec(v, key).and_then(args::f64s).expect("list")
    }

    #[test]
    fn div_rem_exact_x2_minus_1() {
        // (x² − 1) / (x − 1) = x + 1
        let mut m = BTreeMap::new();
        m.insert("a".into(), args::f64_list_value(vec![-1.0, 0.0, 1.0]));
        m.insert("b".into(), args::f64_list_value(vec![-1.0, 1.0]));
        let out = div_rem(&Value::Record(m), span()).unwrap();
        assert_eq!(f64s(&out, "q"), vec![1.0, 1.0]);
        assert!(f64s(&out, "r").is_empty());
    }

    #[test]
    fn div_rem_rejects_zero_divisor() {
        let mut m = BTreeMap::new();
        m.insert("a".into(), args::f64_list_value(vec![1.0, 1.0]));
        m.insert("b".into(), args::f64_list_value(vec![]));
        assert!(div_rem(&Value::Record(m), span()).is_err());
    }

    #[test]
    fn derivative_of_quadratic() {
        // 2 + 3x + x² → 3 + 2x
        let mut m = BTreeMap::new();
        m.insert("a".into(), args::f64_list_value(vec![2.0, 3.0, 1.0]));
        let out = derivative(&Value::Record(m), span()).unwrap();
        assert_eq!(f64s(&out, "coeffs"), vec![3.0, 2.0]);
    }

    #[test]
    fn monic_scales_leading() {
        let mut m = BTreeMap::new();
        m.insert("a".into(), args::f64_list_value(vec![2.0, 4.0]));
        let out = monic(&Value::Record(m), span()).unwrap();
        assert_eq!(f64s(&out, "coeffs"), vec![0.5, 1.0]);
    }

    #[test]
    fn add_sub_mul_ring_ops() {
        // (1 + 2x) + (3 + x) = 4 + 3x
        let mut add_m = BTreeMap::new();
        add_m.insert("a".into(), args::f64_list_value(vec![1.0, 2.0]));
        add_m.insert("b".into(), args::f64_list_value(vec![3.0, 1.0]));
        assert_eq!(
            f64s(&add(&Value::Record(add_m), span()).unwrap(), "coeffs"),
            vec![4.0, 3.0]
        );

        // (1 + 2x) − (3 + x) = −2 + x
        let mut sub_m = BTreeMap::new();
        sub_m.insert("a".into(), args::f64_list_value(vec![1.0, 2.0]));
        sub_m.insert("b".into(), args::f64_list_value(vec![3.0, 1.0]));
        assert_eq!(
            f64s(&sub(&Value::Record(sub_m), span()).unwrap(), "coeffs"),
            vec![-2.0, 1.0]
        );

        // (1 + x)(1 − x) = 1 − x²
        let mut mul_m = BTreeMap::new();
        mul_m.insert("a".into(), args::f64_list_value(vec![1.0, 1.0]));
        mul_m.insert("b".into(), args::f64_list_value(vec![1.0, -1.0]));
        assert_eq!(
            f64s(&mul(&Value::Record(mul_m), span()).unwrap(), "coeffs"),
            vec![1.0, 0.0, -1.0]
        );
    }

    #[test]
    fn resultant_shared_root_is_zero() {
        // (x−1)(x−2) and (x−2)(x−3) share root → resultant 0
        let a = Polynomial::new(vec![-1.0, 1.0])
            .mul(&Polynomial::new(vec![-2.0, 1.0]));
        let b = Polynomial::new(vec![-2.0, 1.0])
            .mul(&Polynomial::new(vec![-3.0, 1.0]));
        let mut m = BTreeMap::new();
        m.insert("a".into(), coeffs_value(&a));
        m.insert("b".into(), coeffs_value(&b));
        let out = resultant(&Value::Record(m), span()).unwrap();
        let v = args::rec_f64(&out, "value").unwrap();
        assert!(v.abs() < 1e-9);
    }

    #[test]
    fn degree_and_leading_of_quadratic() {
        let mut m = BTreeMap::new();
        m.insert("a".into(), args::f64_list_value(vec![2.0, 3.0, 4.0]));
        assert_eq!(
            args::rec_u64(&degree(&Value::Record(m.clone()), span()).unwrap(), "degree"),
            Some(2)
        );
        assert_eq!(
            args::rec_f64(&leading(&Value::Record(m), span()).unwrap(), "value"),
            Some(4.0)
        );
    }

    #[test]
    fn wave5_poly_is_zero() {
        let mut z = BTreeMap::new();
        z.insert("a".into(), args::f64_list_value(vec![]));
        assert_eq!(
            args::rec_bool(&is_zero(&Value::Record(z), span()).unwrap(), "value"),
            Some(true)
        );
        let mut nz = BTreeMap::new();
        nz.insert("a".into(), args::f64_list_value(vec![1.0]));
        assert_eq!(
            args::rec_bool(&is_zero(&Value::Record(nz), span()).unwrap(), "value"),
            Some(false)
        );
    }


    #[test]
    fn degree_of_zero_is_null() {
        let mut m = BTreeMap::new();
        m.insert("a".into(), args::f64_list_value(vec![]));
        let out = degree(&Value::Record(m), span()).unwrap();
        assert_eq!(args::rec(&out, "degree"), Some(&Value::Null));
    }

    #[test]
    fn wave13_poly_coeffs_trims_leading_zeros() {
        let mut m = BTreeMap::new();
        m.insert("a".into(), args::f64_list_value(vec![1.0, 2.0, 0.0, 0.0]));
        let out = coeffs(&Value::Record(m), span()).unwrap();
        assert_eq!(f64s(&out, "coeffs"), vec![1.0, 2.0]);
    }

    #[test]
    fn wave9_poly_gcd_shared_linear() {
        // gcd((x−1)(x−2), (x−2)(x−3)) = x−2 (monic)
        let a = Polynomial::new(vec![-1.0, 1.0]).mul(&Polynomial::new(vec![-2.0, 1.0]));
        let b = Polynomial::new(vec![-2.0, 1.0]).mul(&Polynomial::new(vec![-3.0, 1.0]));
        let mut m = BTreeMap::new();
        m.insert("a".into(), coeffs_value(&a));
        m.insert("b".into(), coeffs_value(&b));
        let out = gcd(&Value::Record(m), span()).unwrap();
        let c = f64s(&out, "coeffs");
        assert_eq!(c.len(), 2);
        assert!((c[0] + 2.0).abs() < 1e-9);
        assert!((c[1] - 1.0).abs() < 1e-9);
    }

    #[test]
    fn wave9_poly_scale() {
        let mut m = BTreeMap::new();
        m.insert("a".into(), args::f64_list_value(vec![1.0, 2.0, 3.0]));
        m.insert("s".into(), Value::F64(2.0));
        let out = scale(&Value::Record(m), span()).unwrap();
        assert_eq!(f64s(&out, "coeffs"), vec![2.0, 4.0, 6.0]);
    }

    #[test]
    fn wave10_poly_eval_horner() {
        // 1 + 2x + 3x² at x=2 → 1+4+12 = 17
        let mut m = BTreeMap::new();
        m.insert("a".into(), args::f64_list_value(vec![1.0, 2.0, 3.0]));
        m.insert("x".into(), Value::F64(2.0));
        let out = eval(&Value::Record(m), span()).unwrap();
        assert_eq!(args::rec_f64(&out, "value"), Some(17.0));
    }

    #[test]
    fn wave10_poly_zero_and_constant() {
        let z = zero(&Value::Record(BTreeMap::new()), span()).unwrap();
        assert!(f64s(&z, "coeffs").is_empty());
        let mut m = BTreeMap::new();
        m.insert("c".into(), Value::F64(5.0));
        let out = constant(&Value::Record(m), span()).unwrap();
        assert_eq!(f64s(&out, "coeffs"), vec![5.0]);
    }
}
