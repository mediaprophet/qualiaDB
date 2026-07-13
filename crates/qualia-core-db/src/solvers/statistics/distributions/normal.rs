//! Normal (Gaussian) distribution — pdf / cdf / quantile, the canonical engine-wide
//! implementation. The CDF is `½·erfc(−z/√2)` over the shared [`super::special::erfc`]
//! (full precision), and the quantile is Acklam's rational inverse refined by one
//! Halley step (≈ machine precision). Domain libraries (`financial_modeling`'s
//! Black–Scholes, etc.) should call these instead of re-deriving a local `normal_cdf`.

use super::special::erfc;
use std::f64::consts::{PI, SQRT_2};

/// Standard-normal pdf `φ(z)`.
pub fn standard_pdf(z: f64) -> f64 {
    (-(z * z) / 2.0).exp() / (2.0 * PI).sqrt()
}

/// Standard-normal cdf `Φ(z) = ½·erfc(−z/√2)`.
pub fn standard_cdf(z: f64) -> f64 {
    0.5 * erfc(-z / SQRT_2)
}

/// Normal pdf with mean `mu`, std-dev `sigma` (> 0).
pub fn pdf(x: f64, mu: f64, sigma: f64) -> f64 {
    debug_assert!(sigma > 0.0);
    standard_pdf((x - mu) / sigma) / sigma
}

/// Normal cdf with mean `mu`, std-dev `sigma` (> 0).
pub fn cdf(x: f64, mu: f64, sigma: f64) -> f64 {
    standard_cdf((x - mu) / sigma)
}

/// Inverse standard-normal cdf `Φ⁻¹(p)`, `0 < p < 1` (Acklam + one Halley refinement).
/// `±∞` at the endpoints.
pub fn standard_quantile(p: f64) -> f64 {
    if p <= 0.0 {
        return f64::NEG_INFINITY;
    }
    if p >= 1.0 {
        return f64::INFINITY;
    }
    const A: [f64; 6] = [
        -3.969_683_028_665_376e1,
        2.209_460_984_245_205e2,
        -2.759_285_104_469_687e2,
        1.383_577_518_672_690e2,
        -3.066_479_806_614_716e1,
        2.506_628_277_459_239e0,
    ];
    const B: [f64; 5] = [
        -5.447_609_879_822_406e1,
        1.615_858_368_580_409e2,
        -1.556_989_798_598_866e2,
        6.680_131_188_771_972e1,
        -1.328_068_155_288_572e1,
    ];
    const C: [f64; 6] = [
        -7.784_894_002_430_293e-3,
        -3.223_964_580_411_365e-1,
        -2.400_758_277_161_838e0,
        -2.549_732_539_343_734e0,
        4.374_664_141_464_968e0,
        2.938_163_982_698_783e0,
    ];
    const D: [f64; 4] = [
        7.784_695_709_041_462e-3,
        3.224_671_290_700_398e-1,
        2.445_134_137_142_996e0,
        3.754_408_661_907_416e0,
    ];
    const PLOW: f64 = 0.02425;
    const PHIGH: f64 = 1.0 - PLOW;

    let mut x = if p < PLOW {
        let q = (-2.0 * p.ln()).sqrt();
        (((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    } else if p <= PHIGH {
        let q = p - 0.5;
        let r = q * q;
        (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q
            / (((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r + 1.0)
    } else {
        let q = (-2.0 * (1.0 - p).ln()).sqrt();
        -(((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    };
    // One Halley refinement step (uses the full-precision cdf/pdf).
    let e = standard_cdf(x) - p;
    let u = e * (2.0 * PI).sqrt() * (x * x / 2.0).exp();
    x -= u / (1.0 + x * u / 2.0);
    x
}

/// Inverse normal cdf with mean `mu`, std-dev `sigma`.
pub fn quantile(p: f64, mu: f64, sigma: f64) -> f64 {
    mu + sigma * standard_quantile(p)
}

/// Two-sided p-value for a standard-normal (z) statistic: `2·(1 − Φ(|z|))`.
pub fn two_sided_p(z: f64) -> f64 {
    2.0 * (1.0 - standard_cdf(z.abs()))
}

#[cfg(test)]
mod tests {
    use super::*;
    const TOL: f64 = 1e-9;

    #[test]
    fn cdf_known_quantiles() {
        assert!((standard_cdf(0.0) - 0.5).abs() < TOL);
        assert!((standard_cdf(1.0) - 0.841_344_746_068_543).abs() < 1e-9);
        assert!((standard_cdf(1.959_963_984_540_054) - 0.975).abs() < 1e-9);
        assert!((standard_cdf(-2.0) - 0.022_750_131_948_179).abs() < 1e-9);
    }

    #[test]
    fn pdf_peak_and_symmetry() {
        assert!((standard_pdf(0.0) - 1.0 / (2.0 * PI).sqrt()).abs() < TOL);
        assert!((standard_pdf(1.5) - standard_pdf(-1.5)).abs() < TOL);
    }

    #[test]
    fn quantile_inverts_cdf() {
        for &p in &[0.001, 0.025, 0.1, 0.5, 0.84, 0.975, 0.999] {
            let z = standard_quantile(p);
            assert!((standard_cdf(z) - p).abs() < 1e-10, "p={p} z={z}");
        }
        // The canonical 1.96 ≈ Φ⁻¹(0.975).
        assert!((standard_quantile(0.975) - 1.959_963_984_540_054).abs() < 1e-7);
    }

    #[test]
    fn general_params_shift_and_scale() {
        // X ~ N(10, 2): cdf(10)=0.5, quantile(0.5)=10.
        assert!((cdf(10.0, 10.0, 2.0) - 0.5).abs() < TOL);
        assert!((quantile(0.5, 10.0, 2.0) - 10.0).abs() < 1e-9);
        assert!((cdf(12.0, 10.0, 2.0) - standard_cdf(1.0)).abs() < TOL);
    }

    #[test]
    fn two_sided_p_value() {
        // |z| = 1.96 → p ≈ 0.05.
        assert!((two_sided_p(1.959_963_984_540_054) - 0.05).abs() < 1e-9);
        assert!((two_sided_p(0.0) - 1.0).abs() < TOL);
    }
}
