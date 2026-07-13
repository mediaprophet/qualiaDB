//! Symbolic differential operators over the CAS, built on `differentiate`/`simplify`.

use super::VecCalcError;
use crate::specialized_libs::symbolic_algebra::{add, differentiate, simplify, sub, Expr};

fn partial(f: &Expr, v: &str) -> Expr {
    simplify(&differentiate(f, v))
}

/// `∇f = [∂f/∂x₁, …, ∂f/∂xₙ]`.
pub fn gradient(f: &Expr, vars: &[&str]) -> Vec<Expr> {
    vars.iter().map(|v| partial(f, v)).collect()
}

/// Divergence `∇·F = Σ ∂Fᵢ/∂xᵢ`. Requires `field.len() == vars.len()`.
pub fn divergence(field: &[Expr], vars: &[&str]) -> Result<Expr, VecCalcError> {
    if field.len() != vars.len() || field.is_empty() {
        return Err(VecCalcError::DimensionMismatch);
    }
    let mut acc = partial(&field[0], vars[0]);
    for i in 1..field.len() {
        acc = add(acc, partial(&field[i], vars[i]));
    }
    Ok(simplify(&acc))
}

/// Curl `∇×F` of a 3-D field. Requires exactly 3 components and 3 variables `[x,y,z]`.
/// Returns `[ ∂Fz/∂y−∂Fy/∂z , ∂Fx/∂z−∂Fz/∂x , ∂Fy/∂x−∂Fx/∂y ]`.
pub fn curl(field: &[Expr], vars: &[&str]) -> Result<[Expr; 3], VecCalcError> {
    if field.len() != 3 || vars.len() != 3 {
        return Err(VecCalcError::DimensionMismatch);
    }
    let (fx, fy, fz) = (&field[0], &field[1], &field[2]);
    let (x, y, z) = (vars[0], vars[1], vars[2]);
    Ok([
        simplify(&sub(partial(fz, y), partial(fy, z))),
        simplify(&sub(partial(fx, z), partial(fz, x))),
        simplify(&sub(partial(fy, x), partial(fx, y))),
    ])
}

/// Laplacian `∇²f = Σ ∂²f/∂xᵢ²`.
pub fn laplacian(f: &Expr, vars: &[&str]) -> Result<Expr, VecCalcError> {
    if vars.is_empty() {
        return Err(VecCalcError::DimensionMismatch);
    }
    let second = |v: &str| partial(&partial(f, v), v);
    let mut acc = second(vars[0]);
    for v in &vars[1..] {
        acc = add(acc, second(v));
    }
    Ok(simplify(&acc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::symbolic_algebra::{add as eadd, mul, pow, var};
    use std::collections::HashMap;

    fn at(e: &Expr, p: &[(&str, f64)]) -> f64 {
        let env: HashMap<String, f64> = p.iter().map(|&(k, v)| (k.to_string(), v)).collect();
        e.eval(&env).unwrap()
    }

    #[test]
    fn divergence_of_position_field_is_three() {
        // F = (x, y, z) → ∇·F = 3
        let field = [var("x"), var("y"), var("z")];
        let d = divergence(&field, &["x", "y", "z"]).unwrap();
        assert!((at(&d, &[("x", 1.0), ("y", 2.0), ("z", 3.0)]) - 3.0).abs() < 1e-9);
    }

    #[test]
    fn curl_of_a_gradient_is_zero() {
        // f = x²y + z ; ∇f then curl(∇f) = 0.
        let f = eadd(mul(pow(var("x"), 2), var("y")), var("z"));
        let g = gradient(&f, &["x", "y", "z"]);
        let cc = curl(&g, &["x", "y", "z"]).unwrap();
        for comp in &cc {
            assert!(at(comp, &[("x", 1.3), ("y", -0.7), ("z", 2.0)]).abs() < 1e-9);
        }
    }

    #[test]
    fn divergence_of_a_curl_is_zero() {
        // F = (x²z, x y², y z²) ; ∇·(∇×F) = 0.
        let field = [
            mul(pow(var("x"), 2), var("z")),
            mul(var("x"), pow(var("y"), 2)),
            mul(var("y"), pow(var("z"), 2)),
        ];
        let c = curl(&field, &["x", "y", "z"]).unwrap();
        let d = divergence(&c, &["x", "y", "z"]).unwrap();
        assert!(at(&d, &[("x", 0.9), ("y", 1.1), ("z", -0.4)]).abs() < 1e-8);
    }

    #[test]
    fn laplacian_of_r_squared_is_six() {
        // f = x²+y²+z² → ∇²f = 6
        let f = eadd(eadd(pow(var("x"), 2), pow(var("y"), 2)), pow(var("z"), 2));
        let l = laplacian(&f, &["x", "y", "z"]).unwrap();
        assert!((at(&l, &[("x", 5.0), ("y", -2.0), ("z", 1.0)]) - 6.0).abs() < 1e-9);
    }

    #[test]
    fn curl_dimension_guard() {
        assert_eq!(
            curl(&[var("x"), var("y")], &["x", "y"]).unwrap_err(),
            VecCalcError::DimensionMismatch
        );
    }
}
