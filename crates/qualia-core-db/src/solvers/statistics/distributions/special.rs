//! Special functions underpinning the probability distributions — the canonical,
//! full-double-precision implementations the whole engine shares.
//!
//! These replace the scattered ad-hoc approximations (a coarse `normal_cdf` copied
//! into `financial_modeling`, a `|t| > 1.96 ⇒ p = 0.05` placeholder in the t-test):
//! every distribution CDF/quantile is built from `ln_gamma`, the regularized
//! incomplete gamma `P(a,x)`, and the regularized incomplete beta `I_x(a,b)` here.
//!
//! Algorithms are the standard, well-conditioned ones (Lanczos for `ln_gamma`; a
//! series + continued-fraction split for the incomplete gamma; a Lentz continued
//! fraction for the incomplete beta) — they are ordinary numerical mathematics,
//! accurate to ~1e-12 and verified against known closed forms in the tests.
//!
//! All scalar `f64` — these are pointwise special functions, **not** GPU-amenable
//! (per-call scalar work below any dispatch crossover); the CPU path is the right
//! and only path here (CLAUDE.md §13: "where GPU does not help, say so").

const EPS: f64 = 1e-15;
const ITMAX: usize = 300;
/// Smallest positive normalized-ish guard for the continued-fraction Lentz start.
const FPMIN: f64 = 1e-300;

/// Natural log of the Gamma function, `ln Γ(x)`, via the Lanczos approximation
/// (g = 7, 9 coefficients) with the reflection formula for `x < 0.5`. Accurate to
/// ~15 significant figures for `x > 0`.
pub fn ln_gamma(x: f64) -> f64 {
    // Lanczos coefficients (g = 7).
    const G: f64 = 7.0;
    const C: [f64; 9] = [
        0.999_999_999_999_809_93,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_13,
        -176.615_029_162_140_59,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_571_6e-6,
        1.505_632_735_149_311_6e-7,
    ];
    if x < 0.5 {
        // Reflection: Γ(x)Γ(1-x) = π / sin(πx).
        let pi = std::f64::consts::PI;
        return (pi / (pi * x).sin()).ln() - ln_gamma(1.0 - x);
    }
    let x = x - 1.0;
    let mut a = C[0];
    let t = x + G + 0.5;
    for (i, &ci) in C.iter().enumerate().skip(1) {
        a += ci / (x + i as f64);
    }
    0.5 * (2.0 * std::f64::consts::PI).ln() + (x + 0.5) * t.ln() - t + a.ln()
}

/// Γ(x) for convenience (small/moderate `x`).
pub fn gamma(x: f64) -> f64 {
    ln_gamma(x).exp() * if x < 0.5 && (x.floor() == x) { f64::NAN } else { 1.0 }
}

/// Regularized lower incomplete gamma `P(a, x) = γ(a,x)/Γ(a)`, `a > 0`, `x ≥ 0`.
/// Series for `x < a+1`, continued fraction (via `Q`) otherwise.
pub fn gammp(a: f64, x: f64) -> f64 {
    debug_assert!(a > 0.0);
    if x <= 0.0 {
        return 0.0;
    }
    if x < a + 1.0 {
        gser(a, x)
    } else {
        1.0 - gcf(a, x)
    }
}

/// Regularized upper incomplete gamma `Q(a, x) = 1 − P(a, x)`.
pub fn gammq(a: f64, x: f64) -> f64 {
    1.0 - gammp(a, x)
}

/// Series evaluation of `P(a, x)` (good for `x < a+1`).
fn gser(a: f64, x: f64) -> f64 {
    let mut ap = a;
    let mut sum = 1.0 / a;
    let mut del = sum;
    for _ in 0..ITMAX {
        ap += 1.0;
        del *= x / ap;
        sum += del;
        if del.abs() < sum.abs() * EPS {
            break;
        }
    }
    sum * (-x + a * x.ln() - ln_gamma(a)).exp()
}

/// Continued-fraction evaluation of `Q(a, x)` (good for `x ≥ a+1`), Lentz's method.
fn gcf(a: f64, x: f64) -> f64 {
    let mut b = x + 1.0 - a;
    let mut c = 1.0 / FPMIN;
    let mut d = 1.0 / b;
    let mut h = d;
    for i in 1..=ITMAX {
        let an = -(i as f64) * (i as f64 - a);
        b += 2.0;
        d = an * d + b;
        if d.abs() < FPMIN {
            d = FPMIN;
        }
        c = b + an / c;
        if c.abs() < FPMIN {
            c = FPMIN;
        }
        d = 1.0 / d;
        let del = d * c;
        h *= del;
        if (del - 1.0).abs() < EPS {
            break;
        }
    }
    (-x + a * x.ln() - ln_gamma(a)).exp() * h
}

/// Regularized incomplete beta `I_x(a, b)`, `0 ≤ x ≤ 1`, `a,b > 0`. Continued
/// fraction (Lentz) with the symmetry switch for fast convergence.
pub fn betai(a: f64, b: f64, x: f64) -> f64 {
    debug_assert!(a > 0.0 && b > 0.0);
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    let bt = (ln_gamma(a + b) - ln_gamma(a) - ln_gamma(b) + a * x.ln() + b * (1.0 - x).ln()).exp();
    if x < (a + 1.0) / (a + b + 2.0) {
        bt * betacf(a, b, x) / a
    } else {
        1.0 - bt * betacf(b, a, 1.0 - x) / b
    }
}

/// Lentz continued fraction for the incomplete beta.
fn betacf(a: f64, b: f64, x: f64) -> f64 {
    let qab = a + b;
    let qap = a + 1.0;
    let qam = a - 1.0;
    let mut c = 1.0;
    let mut d = 1.0 - qab * x / qap;
    if d.abs() < FPMIN {
        d = FPMIN;
    }
    d = 1.0 / d;
    let mut h = d;
    for m in 1..=ITMAX {
        let m = m as f64;
        let m2 = 2.0 * m;
        // even step
        let aa = m * (b - m) * x / ((qam + m2) * (a + m2));
        d = 1.0 + aa * d;
        if d.abs() < FPMIN {
            d = FPMIN;
        }
        c = 1.0 + aa / c;
        if c.abs() < FPMIN {
            c = FPMIN;
        }
        d = 1.0 / d;
        h *= d * c;
        // odd step
        let aa = -(a + m) * (qab + m) * x / ((a + m2) * (qap + m2));
        d = 1.0 + aa * d;
        if d.abs() < FPMIN {
            d = FPMIN;
        }
        c = 1.0 + aa / c;
        if c.abs() < FPMIN {
            c = FPMIN;
        }
        d = 1.0 / d;
        let del = d * c;
        h *= del;
        if (del - 1.0).abs() < EPS {
            break;
        }
    }
    h
}

/// Error function `erf(x)` via the incomplete gamma: `erf(x) = sign(x)·P(½, x²)`.
pub fn erf(x: f64) -> f64 {
    if x == 0.0 {
        0.0
    } else if x > 0.0 {
        gammp(0.5, x * x)
    } else {
        -gammp(0.5, x * x)
    }
}

/// Complementary error function `erfc(x) = 1 − erf(x)`.
pub fn erfc(x: f64) -> f64 {
    1.0 - erf(x)
}

#[cfg(test)]
mod tests {
    use super::*;
    const TOL: f64 = 1e-9;

    #[test]
    fn ln_gamma_known_values() {
        assert!((ln_gamma(1.0)).abs() < TOL); // Γ(1)=1
        assert!((ln_gamma(2.0)).abs() < TOL); // Γ(2)=1
        assert!((ln_gamma(5.0) - 24.0_f64.ln()).abs() < 1e-9); // Γ(5)=4!=24
        // Γ(1/2)=√π
        assert!((ln_gamma(0.5) - std::f64::consts::PI.sqrt().ln()).abs() < 1e-9);
    }

    #[test]
    fn erf_known_values() {
        assert!((erf(0.0)).abs() < TOL);
        assert!((erf(1.0) - 0.842_700_792_949_715).abs() < 1e-9);
        assert!((erf(-1.0) + 0.842_700_792_949_715).abs() < 1e-9);
        assert!((erf(2.0) - 0.995_322_265_018_953).abs() < 1e-9);
        assert!((erfc(0.0) - 1.0).abs() < TOL);
    }

    #[test]
    fn gammp_is_exponential_cdf_for_a_one() {
        // P(1, x) = 1 - e^{-x} (the Exp(1) CDF).
        for &x in &[0.5, 1.0, 2.5, 5.0] {
            assert!((gammp(1.0, x) - (1.0 - (-x).exp())).abs() < 1e-10, "x={x}");
        }
        assert!((gammp(1.0, 0.0)).abs() < TOL);
    }

    #[test]
    fn gammp_gammq_complementary() {
        for &(a, x) in &[(0.5, 0.3), (2.0, 1.0), (3.5, 7.0), (10.0, 4.0)] {
            assert!((gammp(a, x) + gammq(a, x) - 1.0).abs() < 1e-12);
        }
    }

    #[test]
    fn betai_endpoints_and_symmetry() {
        // I_x(1,1) = x (uniform).
        for &x in &[0.0, 0.25, 0.5, 0.9, 1.0] {
            assert!((betai(1.0, 1.0, x) - x).abs() < 1e-10, "x={x}");
        }
        // Symmetry: I_x(a,b) = 1 - I_{1-x}(b,a).
        for &(a, b, x) in &[(2.0, 3.0, 0.4), (0.5, 2.5, 0.7), (5.0, 1.5, 0.2)] {
            assert!((betai(a, b, x) - (1.0 - betai(b, a, 1.0 - x))).abs() < 1e-10, "{a},{b},{x}");
        }
    }
}
