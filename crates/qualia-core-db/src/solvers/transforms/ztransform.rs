//! Z-transform `X(z) = Σ_{n≥0} x[n] z^{−n}` of a finite causal sequence, evaluated at a
//! complex `z`, plus the standard closed forms for the unit step and the geometric
//! sequence.

use super::fourier::Cplx;

#[inline]
fn cmul(a: Cplx, b: Cplx) -> Cplx {
    (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
}
/// Complex reciprocal `1/z`. `None` at `z = 0`.
fn cinv(z: Cplx) -> Option<Cplx> {
    let d = z.0 * z.0 + z.1 * z.1;
    if d == 0.0 {
        return None;
    }
    Some((z.0 / d, -z.1 / d))
}
fn csub(a: Cplx, b: Cplx) -> Cplx {
    (a.0 - b.0, a.1 - b.1)
}

/// `X(z) = Σ_{n=0}^{N−1} x[n] z^{−n}` for a finite real sequence. `None` at `z = 0`.
pub fn z_transform_finite(x: &[f64], z: Cplx) -> Option<Cplx> {
    let zinv = cinv(z)?;
    let mut acc = (0.0, 0.0);
    let mut zpow = (1.0, 0.0); // z^{-n}, starts at n=0
    for &xn in x {
        acc = (acc.0 + xn * zpow.0, acc.1 + xn * zpow.1);
        zpow = cmul(zpow, zinv);
    }
    Some(acc)
}

/// Closed form for the unit step `u[n]`: `X(z) = 1/(1 − z^{−1}) = z/(z−1)`, valid for
/// `|z| > 1`. `None` at `z = 0` or `z = 1`.
pub fn unit_step_z(z: Cplx) -> Option<Cplx> {
    let zinv = cinv(z)?;
    let denom = csub((1.0, 0.0), zinv); // 1 − z^{-1}
    let id = cinv(denom)?;
    Some(id)
}

/// Closed form for `a^n u[n]`: `X(z) = 1/(1 − a·z^{−1})`, valid for `|z| > |a|`.
pub fn geometric_z(a: f64, z: Cplx) -> Option<Cplx> {
    let zinv = cinv(z)?;
    let azinv = (a * zinv.0, a * zinv.1);
    let denom = csub((1.0, 0.0), azinv);
    cinv(denom)
}

#[cfg(test)]
mod tests {
    use super::*;
    const EPS: f64 = 1e-9;

    #[test]
    fn finite_sequence_evaluates() {
        // x = [1,2,3] at z = 2 (real): 1 + 2/2 + 3/4 = 2.75
        let v = z_transform_finite(&[1.0, 2.0, 3.0], (2.0, 0.0)).unwrap();
        assert!((v.0 - 2.75).abs() < EPS && v.1.abs() < EPS);
        // delta[n] = [1] → X(z) = 1 everywhere
        let d = z_transform_finite(&[1.0], (3.0, -1.0)).unwrap();
        assert!((d.0 - 1.0).abs() < EPS && d.1.abs() < EPS);
        assert!(z_transform_finite(&[1.0], (0.0, 0.0)).is_none());
    }

    #[test]
    fn closed_forms_match_truncated_sums() {
        // Geometric a=0.5: closed form ≈ truncated finite sum for |z|>|a|.
        let z = (2.0, 0.0);
        let closed = geometric_z(0.5, z).unwrap();
        let seq: Vec<f64> = (0..60).map(|n| 0.5_f64.powi(n)).collect();
        let approx = z_transform_finite(&seq, z).unwrap();
        assert!((closed.0 - approx.0).abs() < 1e-6 && (closed.1 - approx.1).abs() < 1e-6);
        // Unit step closed form vs truncated.
        let us = unit_step_z(z).unwrap();
        let ones = vec![1.0; 60];
        let ua = z_transform_finite(&ones, z).unwrap();
        assert!((us.0 - ua.0).abs() < 1e-6);
    }
}
