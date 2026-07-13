//! Classical orthogonal polynomials by their three-term recurrences. Each evaluates
//! `P_n(x)` in `O(n)` with no allocation.

/// Legendre polynomial `P_n(x)`. `(n+1)P_{n+1} = (2n+1)x P_n − n P_{n-1}`.
pub fn legendre(n: u32, x: f64) -> f64 {
    if n == 0 {
        return 1.0;
    }
    let (mut p0, mut p1) = (1.0, x);
    for k in 1..n {
        let kf = k as f64;
        let p2 = ((2.0 * kf + 1.0) * x * p1 - kf * p0) / (kf + 1.0);
        p0 = p1;
        p1 = p2;
    }
    p1
}

/// Chebyshev polynomial of the first kind `T_n(x)`. `T_{n+1} = 2x T_n − T_{n-1}`.
pub fn chebyshev_t(n: u32, x: f64) -> f64 {
    if n == 0 {
        return 1.0;
    }
    let (mut t0, mut t1) = (1.0, x);
    for _ in 1..n {
        let t2 = 2.0 * x * t1 - t0;
        t0 = t1;
        t1 = t2;
    }
    t1
}

/// Chebyshev polynomial of the second kind `U_n(x)`. `U_0 = 1`, `U_1 = 2x`,
/// `U_{n+1} = 2x U_n − U_{n-1}`.
pub fn chebyshev_u(n: u32, x: f64) -> f64 {
    if n == 0 {
        return 1.0;
    }
    let (mut u0, mut u1) = (1.0, 2.0 * x);
    for _ in 1..n {
        let u2 = 2.0 * x * u1 - u0;
        u0 = u1;
        u1 = u2;
    }
    u1
}

/// Physicists' Hermite polynomial `H_n(x)`. `H_0 = 1`, `H_1 = 2x`,
/// `H_{n+1} = 2x H_n − 2n H_{n-1}`.
pub fn hermite(n: u32, x: f64) -> f64 {
    if n == 0 {
        return 1.0;
    }
    let (mut h0, mut h1) = (1.0, 2.0 * x);
    for k in 1..n {
        let h2 = 2.0 * x * h1 - 2.0 * k as f64 * h0;
        h0 = h1;
        h1 = h2;
    }
    h1
}

/// Laguerre polynomial `L_n(x)`. `L_0 = 1`, `L_1 = 1 − x`,
/// `(n+1)L_{n+1} = (2n+1−x)L_n − n L_{n-1}`.
pub fn laguerre(n: u32, x: f64) -> f64 {
    if n == 0 {
        return 1.0;
    }
    let (mut l0, mut l1) = (1.0, 1.0 - x);
    for k in 1..n {
        let kf = k as f64;
        let l2 = ((2.0 * kf + 1.0 - x) * l1 - kf * l0) / (kf + 1.0);
        l0 = l1;
        l1 = l2;
    }
    l1
}

#[cfg(test)]
mod tests {
    use super::*;
    const EPS: f64 = 1e-12;

    #[test]
    fn legendre_known() {
        // P_2(x) = (3x²−1)/2, P_3(x) = (5x³−3x)/2
        assert!((legendre(2, 0.5) - (3.0 * 0.25 - 1.0) / 2.0).abs() < EPS);
        assert!((legendre(3, 0.5) - (5.0 * 0.125 - 1.5) / 2.0).abs() < EPS);
        assert!((legendre(5, 1.0) - 1.0).abs() < EPS); // P_n(1) = 1
        assert!((legendre(0, 7.0) - 1.0).abs() < EPS);
    }

    #[test]
    fn chebyshev_known() {
        // T_n(cos θ) = cos(n θ)
        let theta = 0.7_f64;
        for n in 0..6u32 {
            assert!((chebyshev_t(n, theta.cos()) - (n as f64 * theta).cos()).abs() < 1e-10);
        }
        // T_2 = 2x²−1, U_2 = 4x²−1
        assert!((chebyshev_t(2, 0.3) - (2.0 * 0.09 - 1.0)).abs() < EPS);
        assert!((chebyshev_u(2, 0.3) - (4.0 * 0.09 - 1.0)).abs() < EPS);
    }

    #[test]
    fn hermite_known() {
        // H_2 = 4x²−2, H_3 = 8x³−12x
        assert!((hermite(2, 1.5) - (4.0 * 2.25 - 2.0)).abs() < EPS);
        assert!((hermite(3, 1.5) - (8.0 * 3.375 - 18.0)).abs() < 1e-10);
    }

    #[test]
    fn laguerre_known() {
        // L_2 = (x²−4x+2)/2, L_n(0)=1
        assert!((laguerre(2, 1.0) - (1.0 - 4.0 + 2.0) / 2.0).abs() < EPS);
        assert!((laguerre(4, 0.0) - 1.0).abs() < EPS);
    }
}
