//! Natural cubic spline interpolation (the tridiagonal system for the second
//! derivatives is solved with the Thomas algorithm) and piecewise-linear interpolation.

use super::InterpolationError;

/// Piecewise-linear interpolation of `(xs, ys)` (xs strictly increasing) at `x`.
/// Clamps to the endpoints outside the range.
pub fn linear_interp(xs: &[f64], ys: &[f64], x: f64) -> Result<f64, InterpolationError> {
    if xs.len() < 2 || xs.len() != ys.len() {
        return Err(InterpolationError::InsufficientData);
    }
    if x <= xs[0] {
        return Ok(ys[0]);
    }
    if x >= xs[xs.len() - 1] {
        return Ok(ys[ys.len() - 1]);
    }
    let i = xs.partition_point(|&xi| xi <= x) - 1;
    let t = (x - xs[i]) / (xs[i + 1] - xs[i]);
    Ok(ys[i] * (1.0 - t) + ys[i + 1] * t)
}

/// A natural cubic spline: second derivatives zero at both ends.
#[derive(Debug, Clone)]
pub struct CubicSpline {
    xs: Vec<f64>,
    ys: Vec<f64>,
    m: Vec<f64>, // second derivatives at the nodes
}

impl CubicSpline {
    /// Fit a natural cubic spline through `(xs, ys)`. `xs` must be strictly increasing
    /// (duplicate/unsorted nodes fail closed). Needs ≥ 2 points.
    pub fn natural(xs: &[f64], ys: &[f64]) -> Result<Self, InterpolationError> {
        let n = xs.len();
        if n < 2 || n != ys.len() {
            return Err(InterpolationError::InsufficientData);
        }
        for i in 1..n {
            if xs[i] <= xs[i - 1] {
                return Err(InterpolationError::DuplicateNodes);
            }
        }
        let mut m = vec![0.0; n];
        if n >= 3 {
            // Interior tridiagonal system for M_1..M_{n-2} (M_0 = M_{n-1} = 0).
            let h: Vec<f64> = (0..n - 1).map(|i| xs[i + 1] - xs[i]).collect();
            let sz = n - 2;
            let mut sub = vec![0.0; sz]; // sub-diagonal
            let mut diag = vec![0.0; sz];
            let mut sup = vec![0.0; sz]; // super-diagonal
            let mut rhs = vec![0.0; sz];
            for k in 0..sz {
                let i = k + 1; // interior node index
                sub[k] = h[i - 1];
                diag[k] = 2.0 * (h[i - 1] + h[i]);
                sup[k] = h[i];
                rhs[k] = 6.0 * ((ys[i + 1] - ys[i]) / h[i] - (ys[i] - ys[i - 1]) / h[i - 1]);
            }
            let sol = thomas(&sub, &diag, &sup, &rhs).ok_or(InterpolationError::Singular)?;
            for k in 0..sz {
                m[k + 1] = sol[k];
            }
        }
        Ok(Self {
            xs: xs.to_vec(),
            ys: ys.to_vec(),
            m,
        })
    }

    /// Evaluate the spline at `x` (clamped to the node range).
    pub fn eval(&self, x: f64) -> f64 {
        let n = self.xs.len();
        if x <= self.xs[0] {
            return self.ys[0];
        }
        if x >= self.xs[n - 1] {
            return self.ys[n - 1];
        }
        let i = self.xs.partition_point(|&xi| xi <= x) - 1;
        let h = self.xs[i + 1] - self.xs[i];
        let a = self.xs[i + 1] - x;
        let b = x - self.xs[i];
        self.m[i] * a.powi(3) / (6.0 * h)
            + self.m[i + 1] * b.powi(3) / (6.0 * h)
            + (self.ys[i] - self.m[i] * h * h / 6.0) * a / h
            + (self.ys[i + 1] - self.m[i + 1] * h * h / 6.0) * b / h
    }
}

/// Thomas algorithm: solve a tridiagonal system with sub/diag/super-diagonals. `None`
/// on a zero pivot (singular).
fn thomas(sub: &[f64], diag: &[f64], sup: &[f64], rhs: &[f64]) -> Option<Vec<f64>> {
    let n = diag.len();
    let mut c = vec![0.0; n];
    let mut d = vec![0.0; n];
    if diag[0] == 0.0 {
        return None;
    }
    c[0] = sup[0] / diag[0];
    d[0] = rhs[0] / diag[0];
    for i in 1..n {
        let denom = diag[i] - sub[i] * c[i - 1];
        if denom == 0.0 {
            return None;
        }
        c[i] = sup[i] / denom;
        d[i] = (rhs[i] - sub[i] * d[i - 1]) / denom;
    }
    let mut x = vec![0.0; n];
    x[n - 1] = d[n - 1];
    for i in (0..n - 1).rev() {
        x[i] = d[i] - c[i] * x[i + 1];
    }
    Some(x)
}

#[cfg(test)]
mod tests {
    use super::*;
    const EPS: f64 = 1e-9;

    #[test]
    fn spline_passes_through_nodes() {
        let xs = [0.0, 1.0, 2.0, 3.0, 4.0];
        let ys = [0.0, 1.0, 0.0, 1.0, 0.0];
        let s = CubicSpline::natural(&xs, &ys).unwrap();
        for i in 0..xs.len() {
            assert!((s.eval(xs[i]) - ys[i]).abs() < EPS);
        }
    }

    #[test]
    fn natural_spline_reproduces_a_line() {
        // A natural spline of a linear function is that line (zero curvature).
        let f = |x: f64| 2.0 * x - 1.0;
        let xs = [0.0, 1.0, 2.5, 4.0];
        let ys = xs.map(f);
        let s = CubicSpline::natural(&xs, &ys).unwrap();
        for &q in &[0.5, 1.7, 3.2] {
            assert!((s.eval(q) - f(q)).abs() < 1e-9);
        }
    }

    #[test]
    fn linear_interpolation_midpoints() {
        let xs = [0.0, 2.0, 4.0];
        let ys = [0.0, 10.0, 0.0];
        assert!((linear_interp(&xs, &ys, 1.0).unwrap() - 5.0).abs() < EPS);
        assert!((linear_interp(&xs, &ys, 3.0).unwrap() - 5.0).abs() < EPS);
        assert!((linear_interp(&xs, &ys, -1.0).unwrap() - 0.0).abs() < EPS); // clamp
    }

    #[test]
    fn fails_closed() {
        assert_eq!(
            CubicSpline::natural(&[1.0], &[2.0]).unwrap_err(),
            InterpolationError::InsufficientData
        );
        assert_eq!(
            CubicSpline::natural(&[0.0, 0.0, 1.0], &[1.0, 2.0, 3.0]).unwrap_err(),
            InterpolationError::DuplicateNodes
        );
    }
}
