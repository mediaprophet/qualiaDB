//! Airy functions `Ai(x)` and `Bi(x)` via their Maclaurin series. Accurate for moderate
//! `|x|` (the series converge for all `x` but lose digits for large argument).
//!
//! `Ai = α·f − β·g`, `Bi = √3·(α·f + β·g)`, where `α = Ai(0) = 3^{-2/3}/Γ(2/3)`,
//! `β = −Ai'(0) = 3^{-1/3}/Γ(1/3)`, and `f`, `g` are the two hypergeometric series.

const ALPHA: f64 = 0.355_028_053_887_817_24; // Ai(0)
const BETA: f64 = 0.258_819_403_792_806_8; // −Ai'(0)
const MAX_TERMS: usize = 200;

/// `f(x) = Σ_k [∏(3j+1)] x^{3k}/(3k)!`, ratio `t_k/t_{k-1} = x³/((3k)(3k−1))`.
fn series_f(x: f64) -> f64 {
    let x3 = x * x * x;
    let mut t = 1.0;
    let mut sum = 0.0;
    for k in 0..MAX_TERMS {
        sum += t;
        let k1 = (k + 1) as f64;
        t *= x3 / ((3.0 * k1) * (3.0 * k1 - 1.0));
        if t.abs() < 1e-18 {
            break;
        }
    }
    sum
}

/// `g(x) = Σ_k [∏(3j+2)] x^{3k+1}/(3k+1)!`, ratio `t_k/t_{k-1} = x³/((3k+1)(3k))`.
fn series_g(x: f64) -> f64 {
    let x3 = x * x * x;
    let mut t = x;
    let mut sum = 0.0;
    for k in 0..MAX_TERMS {
        sum += t;
        let k1 = (k + 1) as f64;
        t *= x3 / ((3.0 * k1 + 1.0) * (3.0 * k1));
        if t.abs() < 1e-18 {
            break;
        }
    }
    sum
}

/// Airy function of the first kind `Ai(x)`.
pub fn airy_ai(x: f64) -> f64 {
    ALPHA * series_f(x) - BETA * series_g(x)
}

/// Airy function of the second kind `Bi(x)`.
pub fn airy_bi(x: f64) -> f64 {
    3.0_f64.sqrt() * (ALPHA * series_f(x) + BETA * series_g(x))
}

#[cfg(test)]
mod tests {
    use super::*;
    const TOL: f64 = 1e-9;

    #[test]
    fn airy_at_origin() {
        assert!((airy_ai(0.0) - ALPHA).abs() < TOL);
        assert!((airy_bi(0.0) - 3.0_f64.sqrt() * ALPHA).abs() < TOL);
    }

    #[test]
    fn airy_table_values() {
        assert!((airy_ai(1.0) - 0.135_292_416_312_881_4).abs() < 1e-7);
        assert!((airy_bi(1.0) - 1.207_423_594_952_871_3).abs() < 1e-7);
        assert!((airy_ai(-1.0) - 0.535_560_883_292_352_2).abs() < 1e-7);
    }
}
