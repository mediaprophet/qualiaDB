//! Riemann zeta function `ζ(s)` for real `s > 1`, via Euler–Maclaurin acceleration:
//! sum the first `N−1` terms directly, then add the integral tail and Bernoulli
//! corrections. `None` for `s ≤ 1` (the series there needs analytic continuation —
//! out of this function's honest domain).

const N: u32 = 10;
// Bernoulli numbers B₂, B₄, B₆, B₈ and the factorials (2k)!.
const BERNOULLI: [f64; 4] = [1.0 / 6.0, -1.0 / 30.0, 1.0 / 42.0, -1.0 / 30.0];
const FACT_2K: [f64; 4] = [2.0, 24.0, 720.0, 40320.0];

/// `ζ(s)` for real `s > 1`. `None` otherwise (domain).
pub fn zeta(s: f64) -> Option<f64> {
    if s <= 1.0 {
        return None;
    }
    let nf = N as f64;
    // Σ_{n=1}^{N-1} n^{-s}
    let mut sum: f64 = (1..N).map(|n| (n as f64).powf(-s)).sum();
    // ∫ tail + ½ f(N)
    sum += nf.powf(1.0 - s) / (s - 1.0);
    sum += 0.5 * nf.powf(-s);
    // Bernoulli corrections: Σ_k B_{2k}/(2k)! · (s)_{2k-1} · N^{-s-2k+1}
    for k in 1..=4usize {
        let mut poch = 1.0; // rising factorial (s)(s+1)…(s+2k-2), length 2k-1
        for j in 0..(2 * k - 1) {
            poch *= s + j as f64;
        }
        sum += BERNOULLI[k - 1] / FACT_2K[k - 1] * poch * nf.powf(-s - (2 * k - 1) as f64);
    }
    Some(sum)
}

#[cfg(test)]
mod tests {
    use super::*;
    const TOL: f64 = 1e-9;

    #[test]
    fn exact_even_values() {
        // ζ(2) = π²/6, ζ(4) = π⁴/90
        let pi = core::f64::consts::PI;
        assert!((zeta(2.0).unwrap() - pi * pi / 6.0).abs() < TOL);
        assert!((zeta(4.0).unwrap() - pi.powi(4) / 90.0).abs() < TOL);
    }

    #[test]
    fn apery_and_large_s() {
        // ζ(3) = Apéry's constant
        assert!((zeta(3.0).unwrap() - 1.202_056_903_159_594_3).abs() < 1e-8);
        // ζ(s) → 1 as s → ∞
        assert!((zeta(30.0).unwrap() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn domain_fails_closed() {
        assert!(zeta(1.0).is_none());
        assert!(zeta(0.5).is_none());
    }
}
