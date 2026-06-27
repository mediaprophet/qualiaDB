//! Discrete Fourier transform and its inverse over complex samples.

/// A complex number as `(real, imaginary)`.
pub type Cplx = (f64, f64);

#[inline]
fn cadd(a: Cplx, b: Cplx) -> Cplx {
    (a.0 + b.0, a.1 + b.1)
}
#[inline]
fn cmul(a: Cplx, b: Cplx) -> Cplx {
    (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
}

/// Forward DFT: `X[k] = Σ_n x[n] · e^{−2πi kn/N}`.
pub fn dft(x: &[Cplx]) -> Vec<Cplx> {
    let n = x.len();
    let mut out = vec![(0.0, 0.0); n];
    if n == 0 {
        return out;
    }
    let w = -2.0 * core::f64::consts::PI / n as f64;
    for (k, ok) in out.iter_mut().enumerate() {
        let mut acc = (0.0, 0.0);
        for (j, &xj) in x.iter().enumerate() {
            let ang = w * (k * j) as f64;
            acc = cadd(acc, cmul(xj, (ang.cos(), ang.sin())));
        }
        *ok = acc;
    }
    out
}

/// Inverse DFT: `x[n] = (1/N) Σ_k X[k] · e^{+2πi kn/N}`.
pub fn idft(spectrum: &[Cplx]) -> Vec<Cplx> {
    let n = spectrum.len();
    let mut out = vec![(0.0, 0.0); n];
    if n == 0 {
        return out;
    }
    let w = 2.0 * core::f64::consts::PI / n as f64;
    let inv = 1.0 / n as f64;
    for (j, oj) in out.iter_mut().enumerate() {
        let mut acc = (0.0, 0.0);
        for (k, &xk) in spectrum.iter().enumerate() {
            let ang = w * (k * j) as f64;
            acc = cadd(acc, cmul(xk, (ang.cos(), ang.sin())));
        }
        *oj = (acc.0 * inv, acc.1 * inv);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    const EPS: f64 = 1e-9;

    fn re(v: &[f64]) -> Vec<Cplx> {
        v.iter().map(|&r| (r, 0.0)).collect()
    }

    #[test]
    fn dft_of_constant_is_an_impulse() {
        // DFT([1,1,1,1]) = [4,0,0,0]
        let x = dft(&re(&[1.0, 1.0, 1.0, 1.0]));
        assert!((x[0].0 - 4.0).abs() < EPS && x[0].1.abs() < EPS);
        for k in 1..4 {
            assert!(x[k].0.abs() < EPS && x[k].1.abs() < EPS);
        }
    }

    #[test]
    fn dft_of_impulse_is_constant() {
        // DFT([1,0,0,0]) = [1,1,1,1]
        let x = dft(&re(&[1.0, 0.0, 0.0, 0.0]));
        for k in 0..4 {
            assert!((x[k].0 - 1.0).abs() < EPS && x[k].1.abs() < EPS);
        }
    }

    #[test]
    fn inverse_round_trips() {
        let x = re(&[3.0, 1.0, 4.0, 1.0, 5.0, 9.0, 2.0, 6.0]);
        let back = idft(&dft(&x));
        for (a, b) in x.iter().zip(&back) {
            assert!((a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9);
        }
    }
}
