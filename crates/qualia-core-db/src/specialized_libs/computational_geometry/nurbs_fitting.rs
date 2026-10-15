//! P20.3 — Non-uniform B-spline / NURBS curve & patch evaluation, chord-length
//! parameterization, Piegl & Tiller (§9.4.1) least-squares curve fitting with
//! pinned endpoints, and curvature-adaptive polyline discretization.
//!
//! All evaluation and parameterization hot paths are Tier-1 `HotZeroHeap`
//! (stack-bounded up to degree `MAX_SPLINE_DEGREE = 15`).

use super::parametric_cad::NurbsControlPoint;
use super::primitives::Point3;

/// Maximum supported spline degree for zero-heap Cox–de Boor evaluation.
pub const MAX_SPLINE_DEGREE: usize = 15;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NurbsFitError {
    InvalidInput,
    DegreeTooHigh { degree: usize, max: usize },
    InvalidKnotVector,
    SingularSystem,
    OutputTooSmall { required: usize },
}

#[inline]
fn add(a: Point3, b: Point3) -> Point3 {
    Point3::new(a.x + b.x, a.y + b.y, a.z + b.z)
}

#[inline]
fn sub(a: Point3, b: Point3) -> Point3 {
    Point3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}

#[inline]
fn scale(a: Point3, s: f64) -> Point3 {
    Point3::new(a.x * s, a.y * s, a.z * s)
}

#[inline]
fn dist(a: Point3, b: Point3) -> f64 {
    let d = sub(a, b);
    (d.x * d.x + d.y * d.y + d.z * d.z).sqrt()
}

/// Find the knot span index $i \in [p, n]$ such that $u \in [U_i, U_{i+1})$.
/// For $u = U_{n+1}$, returns $n$.
///
/// Tier-1 `HotZeroHeap`.
pub fn find_knot_span(
    n_ctrl: usize,
    degree: usize,
    u: f64,
    knots: &[f64],
) -> Result<usize, NurbsFitError> {
    if n_ctrl == 0
        || degree == 0
        || degree >= n_ctrl
        || degree > MAX_SPLINE_DEGREE
        || knots.len() != n_ctrl + degree + 1
        || !u.is_finite()
    {
        return Err(NurbsFitError::InvalidKnotVector);
    }
    for w in knots.windows(2) {
        if w[1] < w[0] || !w[0].is_finite() || !w[1].is_finite() {
            return Err(NurbsFitError::InvalidKnotVector);
        }
    }
    let n = n_ctrl - 1;
    let u_clamped = u.clamp(knots[degree], knots[n + 1]);
    if (u_clamped - knots[n + 1]).abs() <= 1e-14 {
        return Ok(n);
    }
    let mut low = degree;
    let mut high = n + 1;
    let mut mid = (low + high) / 2;
    while u_clamped < knots[mid] || u_clamped >= knots[mid + 1] {
        if u_clamped < knots[mid] {
            high = mid;
        } else {
            low = mid;
        }
        mid = (low + high) / 2;
    }
    Ok(mid)
}

/// Compute the `degree + 1` non-vanishing B-spline basis functions
/// $N_{i-p, p}(u), \dots, N_{i, p}(u)$ at span `span` (Piegl & Tiller Alg. A2.2).
///
/// Tier-1 `HotZeroHeap`.
pub fn basis_functions(
    span: usize,
    u: f64,
    degree: usize,
    knots: &[f64],
    out_basis: &mut [f64],
) -> Result<(), NurbsFitError> {
    if degree > MAX_SPLINE_DEGREE || out_basis.len() <= degree {
        return Err(NurbsFitError::DegreeTooHigh {
            degree,
            max: MAX_SPLINE_DEGREE,
        });
    }
    let mut left = [0.0f64; MAX_SPLINE_DEGREE + 1];
    let mut right = [0.0f64; MAX_SPLINE_DEGREE + 1];
    out_basis[0] = 1.0;

    for j in 1..=degree {
        left[j] = u - knots[span + 1 - j];
        right[j] = knots[span + j] - u;
        let mut saved = 0.0;
        for r in 0..j {
            let denom = right[r + 1] + left[j - r];
            let temp = if denom.abs() > 1e-15 {
                out_basis[r] / denom
            } else {
                0.0
            };
            out_basis[r] = saved + right[r + 1] * temp;
            saved = left[j - r] * temp;
        }
        out_basis[j] = saved;
    }
    Ok(())
}

/// Evaluate a non-uniform B-spline curve at parameter `u` using Cox–de Boor.
///
/// Tier-1 `HotZeroHeap`.
pub fn bspline_eval_knots(
    control: &[Point3],
    degree: usize,
    knots: &[f64],
    u: f64,
) -> Result<Point3, NurbsFitError> {
    let span = find_knot_span(control.len(), degree, u, knots)?;
    let u_clamped = u.clamp(knots[degree], knots[control.len()]);
    let mut n_basis = [0.0f64; MAX_SPLINE_DEGREE + 1];
    basis_functions(span, u_clamped, degree, knots, &mut n_basis)?;

    let mut pt = Point3::new(0.0, 0.0, 0.0);
    for j in 0..=degree {
        pt = add(pt, scale(control[span - degree + j], n_basis[j]));
    }
    Ok(pt)
}

/// Evaluate a rational non-uniform B-spline (NURBS) curve at parameter `u`.
///
/// Tier-1 `HotZeroHeap`.
pub fn nurbs_eval_knots(
    control: &[NurbsControlPoint],
    degree: usize,
    knots: &[f64],
    u: f64,
) -> Result<Point3, NurbsFitError> {
    let span = find_knot_span(control.len(), degree, u, knots)?;
    let u_clamped = u.clamp(knots[degree], knots[control.len()]);
    let mut n_basis = [0.0f64; MAX_SPLINE_DEGREE + 1];
    basis_functions(span, u_clamped, degree, knots, &mut n_basis)?;

    let mut weighted = Point3::new(0.0, 0.0, 0.0);
    let mut w_sum = 0.0f64;
    for j in 0..=degree {
        let cp = control[span - degree + j];
        let bw = n_basis[j] * cp.weight;
        weighted = add(weighted, scale(cp.point, bw));
        w_sum += bw;
    }
    if w_sum.abs() <= 1e-15 {
        return Err(NurbsFitError::InvalidInput);
    }
    Ok(scale(weighted, 1.0 / w_sum))
}

/// Evaluate a rational tensor-product NURBS surface patch at `(u, v)`.
///
/// Tier-1 `HotZeroHeap`.
pub fn nurbs_patch_eval(
    grid: &[NurbsControlPoint],
    u_count: usize,
    v_count: usize,
    u_degree: usize,
    v_degree: usize,
    u_knots: &[f64],
    v_knots: &[f64],
    u: f64,
    v: f64,
) -> Result<Point3, NurbsFitError> {
    if grid.len() != u_count.saturating_mul(v_count) {
        return Err(NurbsFitError::InvalidInput);
    }
    let u_span = find_knot_span(u_count, u_degree, u, u_knots)?;
    let v_span = find_knot_span(v_count, v_degree, v, v_knots)?;
    let u_c = u.clamp(u_knots[u_degree], u_knots[u_count]);
    let v_c = v.clamp(v_knots[v_degree], v_knots[v_count]);

    let mut nu = [0.0f64; MAX_SPLINE_DEGREE + 1];
    let mut nv = [0.0f64; MAX_SPLINE_DEGREE + 1];
    basis_functions(u_span, u_c, u_degree, u_knots, &mut nu)?;
    basis_functions(v_span, v_c, v_degree, v_knots, &mut nv)?;

    let mut sum = Point3::new(0.0, 0.0, 0.0);
    let mut w_sum = 0.0f64;
    for l in 0..=v_degree {
        let row = v_span - v_degree + l;
        for k in 0..=u_degree {
            let col = u_span - u_degree + k;
            let cp = grid[row * u_count + col];
            let w = nu[k] * nv[l] * cp.weight;
            sum = add(sum, scale(cp.point, w));
            w_sum += w;
        }
    }
    if w_sum.abs() <= 1e-15 {
        return Err(NurbsFitError::InvalidInput);
    }
    Ok(scale(sum, 1.0 / w_sum))
}

/// Chord-length parameterization of an ordered 3-D point sequence $Q_0, \dots, Q_{m-1}$
/// (PlantGL `Fit::chordLengthParam`, Piegl & Tiller eq. 9.4–9.5).
///
/// Writes normalized parameters $\bar{u}_0 = 0, \dots, \bar{u}_{m-1} = 1$ to `out_u`
/// and returns total chord length $d$.
///
/// Tier-1 `HotZeroHeap`.
pub fn chord_length_parameterize(
    points: &[Point3],
    out_u: &mut [f64],
) -> Result<f64, NurbsFitError> {
    let m = points.len();
    if m < 2 {
        return Err(NurbsFitError::InvalidInput);
    }
    if out_u.len() < m {
        return Err(NurbsFitError::OutputTooSmall { required: m });
    }
    out_u[0] = 0.0;
    let mut total = 0.0f64;
    for k in 1..m {
        let d = dist(points[k], points[k - 1]);
        if !d.is_finite() {
            return Err(NurbsFitError::InvalidInput);
        }
        total += d;
        out_u[k] = total;
    }
    if total <= 1e-14 {
        for k in 1..m {
            out_u[k] = (k as f64) / ((m - 1) as f64);
        }
        return Ok(0.0);
    }
    for k in 1..m - 1 {
        out_u[k] /= total;
    }
    out_u[m - 1] = 1.0;
    Ok(total)
}

/// Least-squares B-spline curve approximation with pinned endpoints
/// $C(0) = Q_0$ and $C(1) = Q_{m-1}$ (PlantGL `Fit::leastSquares`, Piegl & Tiller
/// §9.4.1, pp. 410–413).
///
/// Computes `n_ctrl` control points and `n_ctrl + degree + 1` clamped knots
/// minimizing $\sum_{k=1}^{m-2} \|Q_k - C(\bar{u}_k)\|^2$.
pub fn fit_bspline_least_squares(
    points: &[Point3],
    degree: usize,
    n_ctrl: usize,
    out_ctrl: &mut [Point3],
    out_knots: &mut [f64],
) -> Result<f64, NurbsFitError> {
    let m = points.len();
    if degree == 0 || degree > MAX_SPLINE_DEGREE || n_ctrl <= degree || n_ctrl > m || m < 2 {
        return Err(NurbsFitError::InvalidInput);
    }
    let req_knots = n_ctrl + degree + 1;
    if out_ctrl.len() < n_ctrl || out_knots.len() < req_knots {
        return Err(NurbsFitError::OutputTooSmall {
            required: req_knots,
        });
    }

    let mut u_bar = vec![0.0f64; m];
    chord_length_parameterize(points, &mut u_bar)?;

    // Build clamped knot vector by Piegl & Tiller eq. (9.68)
    for j in 0..=degree {
        out_knots[j] = 0.0;
        out_knots[req_knots - 1 - j] = 1.0;
    }
    let n = n_ctrl - 1;
    let m_last = m - 1;
    let d = ((m_last + 1) as f64) / ((n - degree + 1) as f64);
    for j in 1..=(n - degree) {
        let i = (j as f64 * d).floor() as usize;
        let alpha = (j as f64 * d) - (i as f64);
        out_knots[degree + j] = (1.0 - alpha) * u_bar[i - 1] + alpha * u_bar[i];
    }

    out_ctrl[0] = points[0];
    out_ctrl[n] = points[m_last];

    if n_ctrl == 2 {
        return max_approximation_error(
            points,
            &u_bar,
            &out_ctrl[..n_ctrl],
            degree,
            &out_knots[..req_knots],
        );
    }

    // Solve normal equations (N^T N) P = R for interior control points P_1 .. P_{n-1}
    let dim = n - 1;
    let mut ntn = vec![0.0f64; dim * dim];
    let mut rhs = vec![Point3::new(0.0, 0.0, 0.0); dim];
    let mut row_n = vec![0.0f64; n_ctrl];
    let mut basis_buf = [0.0f64; MAX_SPLINE_DEGREE + 1];

    for k in 1..m_last {
        row_n.fill(0.0);
        let span = find_knot_span(n_ctrl, degree, u_bar[k], &out_knots[..req_knots])?;
        basis_functions(
            span,
            u_bar[k],
            degree,
            &out_knots[..req_knots],
            &mut basis_buf,
        )?;
        for j in 0..=degree {
            row_n[span - degree + j] = basis_buf[j];
        }

        let rk = sub(
            points[k],
            add(scale(points[0], row_n[0]), scale(points[m_last], row_n[n])),
        );
        for i in 1..=dim {
            let ni = row_n[i];
            if ni == 0.0 {
                continue;
            }
            rhs[i - 1] = add(rhs[i - 1], scale(rk, ni));
            for j in 1..=dim {
                ntn[(i - 1) * dim + (j - 1)] += ni * row_n[j];
            }
        }
    }

    // Small diagonal Tikhonov regularization for rank-deficient point distributions
    for i in 0..dim {
        ntn[i * dim + i] += 1e-12;
    }

    solve_spd_point3(&mut ntn, &mut rhs, dim)?;
    for i in 0..dim {
        out_ctrl[i + 1] = rhs[i];
    }

    max_approximation_error(
        points,
        &u_bar,
        &out_ctrl[..n_ctrl],
        degree,
        &out_knots[..req_knots],
    )
}

fn solve_spd_point3(a: &mut [f64], b: &mut [Point3], n: usize) -> Result<(), NurbsFitError> {
    for k in 0..n {
        let mut pivot = k;
        let mut max_val = a[k * n + k].abs();
        for i in (k + 1)..n {
            let v = a[i * n + k].abs();
            if v > max_val {
                max_val = v;
                pivot = i;
            }
        }
        if max_val <= 1e-15 {
            return Err(NurbsFitError::SingularSystem);
        }
        if pivot != k {
            for j in k..n {
                a.swap(k * n + j, pivot * n + j);
            }
            b.swap(k, pivot);
        }
        let diag = a[k * n + k];
        for i in (k + 1)..n {
            let factor = a[i * n + k] / diag;
            a[i * n + k] = 0.0;
            for j in (k + 1)..n {
                a[i * n + j] -= factor * a[k * n + j];
            }
            b[i] = sub(b[i], scale(b[k], factor));
        }
    }
    for i in (0..n).rev() {
        let mut sum = b[i];
        for j in (i + 1)..n {
            sum = sub(sum, scale(b[j], a[i * n + j]));
        }
        b[i] = scale(sum, 1.0 / a[i * n + i]);
    }
    Ok(())
}

fn max_approximation_error(
    points: &[Point3],
    u_bar: &[f64],
    ctrl: &[Point3],
    degree: usize,
    knots: &[f64],
) -> Result<f64, NurbsFitError> {
    let mut max_err = 0.0f64;
    for (q, &u) in points.iter().zip(u_bar.iter()) {
        let c = bspline_eval_knots(ctrl, degree, knots, u)?;
        max_err = max_err.max(dist(*q, c));
    }
    Ok(max_err)
}

/// Discretize a B-spline curve into `sample_count` points concentrated in
/// regions of higher discrete curvature (PlantGL `discretizeWithCurvature`).
pub fn discretize_bspline_by_curvature(
    control: &[Point3],
    degree: usize,
    knots: &[f64],
    sample_count: usize,
    out_points: &mut [Point3],
) -> Result<usize, NurbsFitError> {
    if sample_count < 2 {
        return Err(NurbsFitError::InvalidInput);
    }
    if out_points.len() < sample_count {
        return Err(NurbsFitError::OutputTooSmall {
            required: sample_count,
        });
    }

    const DENSE: usize = 128;
    let mut dense_pts = [Point3::default(); DENSE];
    for (i, slot) in dense_pts.iter_mut().enumerate() {
        let u = (i as f64) / ((DENSE - 1) as f64);
        *slot = bspline_eval_knots(control, degree, knots, u)?;
    }

    let mut cdf = [0.0f64; DENSE];
    for i in 1..DENSE {
        let ds = dist(dense_pts[i], dense_pts[i - 1]);
        let turn = if i + 1 < DENSE {
            let v0 = sub(dense_pts[i], dense_pts[i - 1]);
            let v1 = sub(dense_pts[i + 1], dense_pts[i]);
            dist(v1, v0)
        } else {
            0.0
        };
        cdf[i] = cdf[i - 1] + ds + 2.0 * turn;
    }
    let total_w = cdf[DENSE - 1].max(1e-12);

    out_points[0] = dense_pts[0];
    out_points[sample_count - 1] = dense_pts[DENSE - 1];
    for k in 1..sample_count - 1 {
        let target = total_w * (k as f64) / ((sample_count - 1) as f64);
        let idx = cdf.partition_point(|&w| w < target).min(DENSE - 1);
        let prev = idx.saturating_sub(1);
        let span = (cdf[idx] - cdf[prev]).max(1e-12);
        let frac = ((target - cdf[prev]) / span).clamp(0.0, 1.0);
        let u = ((prev as f64) + frac) / ((DENSE - 1) as f64);
        out_points[k] = bspline_eval_knots(control, degree, knots, u)?;
    }
    Ok(sample_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn least_squares_bspline_fits_parabola_accurately() {
        let mut samples = [Point3::default(); 21];
        for (i, slot) in samples.iter_mut().enumerate() {
            let t = (i as f64) / 20.0;
            *slot = Point3::new(t, t * t, 0.5 * t);
        }
        let mut ctrl = [Point3::default(); 6];
        let mut knots = [0.0f64; 10];
        let max_err = fit_bspline_least_squares(&samples, 3, 6, &mut ctrl, &mut knots).unwrap();
        assert!(
            max_err < 1e-2,
            "expected tight cubic B-spline fit, max_err={max_err}"
        );

        let mut adaptive = [Point3::default(); 12];
        let n = discretize_bspline_by_curvature(&ctrl, 3, &knots, 12, &mut adaptive).unwrap();
        assert_eq!(n, 12);
        assert!(dist(adaptive[0], samples[0]) < 1e-12);
        assert!(dist(adaptive[11], samples[20]) < 1e-12);
    }
}
