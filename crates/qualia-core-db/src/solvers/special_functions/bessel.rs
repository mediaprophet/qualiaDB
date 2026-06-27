//! Bessel functions of integer order: `J_n`, `Y_n` (first/second kind) and the modified
//! `I_n`, `K_n`. The order-0 functions come from their convergent power series (with the
//! log + harmonic-number terms for the second kinds); order 1 from the Wronskian
//! relations; higher orders from the standard upward recurrences. Accurate for moderate
//! `|x|` (the series converge for all `x` but lose digits for large argument — documented).

const EULER_GAMMA: f64 = 0.577_215_664_901_532_9;
const MAX_TERMS: usize = 400;

fn factorial_f(n: u32) -> f64 {
    (1..=n).fold(1.0, |a, k| a * k as f64)
}

/// `H_k = 1 + 1/2 + … + 1/k`.
fn harmonic(k: u32) -> f64 {
    (1..=k).fold(0.0, |a, j| a + 1.0 / j as f64)
}

fn j_nonneg(n: u32, x: f64) -> f64 {
    // Σ_{m≥0} (−1)^m (x/2)^{2m+n} / (m! (m+n)!)
    let h2 = (x / 2.0) * (x / 2.0);
    let mut a = (x / 2.0).powi(n as i32) / factorial_f(n); // m = 0
    let mut sum = 0.0;
    let mut sign = 1.0;
    for m in 0..MAX_TERMS {
        sum += sign * a;
        a *= h2 / (((m + 1) as f64) * ((m + 1 + n as usize) as f64));
        sign = -sign;
        if a.abs() < 1e-18 && m as u32 > n {
            break;
        }
    }
    sum
}

fn i_nonneg(n: u32, x: f64) -> f64 {
    // Σ_{m≥0} (x/2)^{2m+n} / (m! (m+n)!)  — same as J but all-positive.
    let h2 = (x / 2.0) * (x / 2.0);
    let mut a = (x / 2.0).powi(n as i32) / factorial_f(n);
    let mut sum = 0.0;
    for m in 0..MAX_TERMS {
        sum += a;
        a *= h2 / (((m + 1) as f64) * ((m + 1 + n as usize) as f64));
        if a.abs() < 1e-18 && m as u32 > n {
            break;
        }
    }
    sum
}

/// Bessel function of the first kind `J_n(x)`, integer order (any sign). Defined for all
/// real `x`. `J_{-n} = (−1)^n J_n`.
pub fn bessel_j(n: i32, x: f64) -> f64 {
    let m = n.unsigned_abs();
    let v = j_nonneg(m, x);
    if n < 0 && m % 2 == 1 {
        -v
    } else {
        v
    }
}

/// Modified Bessel function of the first kind `I_n(x)`, integer order. `I_{-n} = I_n`.
pub fn bessel_i(n: i32, x: f64) -> f64 {
    i_nonneg(n.unsigned_abs(), x)
}

fn y0(x: f64) -> f64 {
    // Y_0 = (2/π)(ln(x/2)+γ)J_0 + (2/π) Σ_{k≥1} (−1)^{k+1} H_k/(k!)^2 (x/2)^{2k}
    let h2 = (x / 2.0) * (x / 2.0);
    let mut series = 0.0;
    let mut term = 1.0; // (x/2)^{2k}/(k!)^2 accumulator, starts at k=0 value 1
    let mut sign = 1.0; // (−1)^{k+1} for k=1 is +1
    for k in 1..MAX_TERMS as u32 {
        term *= h2 / (k as f64 * k as f64); // now (x/2)^{2k}/(k!)^2
        series += sign * harmonic(k) * term;
        sign = -sign;
        if term * harmonic(k) < 1e-18 {
            break;
        }
    }
    let two_over_pi = 2.0 / core::f64::consts::PI;
    two_over_pi * ((x / 2.0).ln() + EULER_GAMMA) * j_nonneg(0, x) + two_over_pi * series
}

fn k0(x: f64) -> f64 {
    // K_0 = −(ln(x/2)+γ)I_0 + Σ_{k≥1} H_k/(k!)^2 (x/2)^{2k}
    let h2 = (x / 2.0) * (x / 2.0);
    let mut series = 0.0;
    let mut term = 1.0;
    for k in 1..MAX_TERMS as u32 {
        term *= h2 / (k as f64 * k as f64);
        series += harmonic(k) * term;
        if term * harmonic(k) < 1e-18 {
            break;
        }
    }
    -((x / 2.0).ln() + EULER_GAMMA) * i_nonneg(0, x) + series
}

/// Bessel function of the second kind `Y_n(x)`, integer order `n ≥ 0`. Requires `x > 0`
/// (singular at the origin) → `None` otherwise. Order 1 via the Wronskian
/// `J_1 Y_0 − J_0 Y_1 = 2/(πx)`; higher via `Y_{n+1} = (2n/x)Y_n − Y_{n-1}`.
pub fn bessel_y(n: u32, x: f64) -> Option<f64> {
    if x <= 0.0 {
        return None;
    }
    let y0v = y0(x);
    if n == 0 {
        return Some(y0v);
    }
    let j0 = j_nonneg(0, x);
    if j0.abs() < 1e-300 {
        return None; // at a zero of J_0 the Wronskian solve is ill-posed
    }
    let y1 = (bessel_j(1, x) * y0v - 2.0 / (core::f64::consts::PI * x)) / j0;
    if n == 1 {
        return Some(y1);
    }
    let (mut ym1, mut yn) = (y0v, y1);
    for k in 1..n {
        let ynext = (2.0 * k as f64 / x) * yn - ym1;
        ym1 = yn;
        yn = ynext;
    }
    Some(yn)
}

/// Modified Bessel function of the second kind `K_n(x)`, integer order `n ≥ 0`. Requires
/// `x > 0` → `None` otherwise. Order 1 via the Wronskian `I_0 K_1 + I_1 K_0 = 1/x`;
/// higher via `K_{n+1} = (2n/x)K_n + K_{n-1}`.
pub fn bessel_k(n: u32, x: f64) -> Option<f64> {
    if x <= 0.0 {
        return None;
    }
    let k0v = k0(x);
    if n == 0 {
        return Some(k0v);
    }
    let i0 = i_nonneg(0, x);
    let k1 = (1.0 / x - bessel_i(1, x) * k0v) / i0;
    if n == 1 {
        return Some(k1);
    }
    let (mut km1, mut kn) = (k0v, k1);
    for k in 1..n {
        let knext = (2.0 * k as f64 / x) * kn + km1;
        km1 = kn;
        kn = knext;
    }
    Some(kn)
}

#[cfg(test)]
mod tests {
    use super::*;
    const TOL: f64 = 1e-7;

    #[test]
    fn first_kind_table() {
        assert!((bessel_j(0, 0.0) - 1.0).abs() < TOL);
        assert!((bessel_j(1, 0.0)).abs() < TOL);
        assert!((bessel_j(0, 1.0) - 0.765_197_686_557_966_5).abs() < TOL);
        assert!((bessel_j(1, 1.0) - 0.440_050_585_744_933_5).abs() < TOL);
        assert!((bessel_j(2, 2.0) - 0.352_834_028_615_815_5).abs() < TOL);
        // J_{-1} = −J_1
        assert!((bessel_j(-1, 1.0) + bessel_j(1, 1.0)).abs() < TOL);
    }

    #[test]
    fn modified_first_kind_table() {
        assert!((bessel_i(0, 0.0) - 1.0).abs() < TOL);
        assert!((bessel_i(0, 1.0) - 1.266_065_877_752_008_4).abs() < TOL);
        assert!((bessel_i(1, 1.0) - 0.565_159_103_992_485_0).abs() < TOL);
    }

    #[test]
    fn second_kind_table_and_domain() {
        assert!(bessel_y(0, -1.0).is_none()); // x ≤ 0 fails closed
        assert!((bessel_y(0, 1.0).unwrap() - 0.088_256_964_215_676_96).abs() < TOL);
        assert!((bessel_y(1, 1.0).unwrap() + 0.781_212_821_300_288_7).abs() < TOL);
        assert!((bessel_k(0, 1.0).unwrap() - 0.421_024_438_240_708_3).abs() < TOL);
        assert!((bessel_k(1, 1.0).unwrap() - 0.601_907_230_197_234_6).abs() < TOL);
        assert!(bessel_k(2, 0.0).is_none());
    }
}
