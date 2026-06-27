//! General-dimension numerical methods (Calculus plan §4.4).
//!
//! The original solvers in [`super`] (`RungeKutta4Static`, `ShootingMethodBVP`,
//! `SimpsonsIntegratorChunked`) are fixed at `[f64; 4]` / scalar — the same toy-sizing the
//! linear-algebra and optimisation solvers once had. This module generalises them to
//! **arbitrary state dimension** on heap `Vec<f64>`, reusing the engine's canonical dense LU
//! solve ([`crate::solvers::linear_algebra::lu::lu_solve`]) for the BVP Newton correction —
//! no re-implemented linear algebra. These are the allocate-friendly *authoring-path*
//! versions; the zero-heap `[f64; 4]` solvers remain for the hot path.

use crate::solvers::linear_algebra::lu::lu_solve;

/// One classical RK4 step of `y' = f(t, y)` for a state of any dimension. `f` returns the
/// derivative vector, which must have the same length as `y`.
pub fn rk4_step<F>(f: &F, t: f64, y: &[f64], h: f64) -> Vec<f64>
where
    F: Fn(f64, &[f64]) -> Vec<f64>,
{
    let n = y.len();
    let k1 = f(t, y);
    let y2: Vec<f64> = (0..n).map(|i| y[i] + 0.5 * h * k1[i]).collect();
    let k2 = f(t + 0.5 * h, &y2);
    let y3: Vec<f64> = (0..n).map(|i| y[i] + 0.5 * h * k2[i]).collect();
    let k3 = f(t + 0.5 * h, &y3);
    let y4: Vec<f64> = (0..n).map(|i| y[i] + h * k3[i]).collect();
    let k4 = f(t + h, &y4);
    (0..n)
        .map(|i| y[i] + h * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]) / 6.0)
        .collect()
}

/// Integrate `y' = f(t, y)` from `t0` to `t1` in `steps` equal RK4 steps; returns the final
/// state. `steps` is clamped to `≥ 1`.
pub fn rk4_integrate<F>(f: &F, t0: f64, y0: &[f64], t1: f64, steps: usize) -> Vec<f64>
where
    F: Fn(f64, &[f64]) -> Vec<f64>,
{
    let steps = steps.max(1);
    let h = (t1 - t0) / steps as f64;
    let mut y = y0.to_vec();
    let mut t = t0;
    for _ in 0..steps {
        y = rk4_step(f, t, &y, h);
        t += h;
    }
    y
}

/// Full trajectory `[(t, y)]`, including the initial point, over `steps` RK4 steps.
pub fn rk4_solve<F>(f: &F, t0: f64, y0: &[f64], t1: f64, steps: usize) -> Vec<(f64, Vec<f64>)>
where
    F: Fn(f64, &[f64]) -> Vec<f64>,
{
    let steps = steps.max(1);
    let h = (t1 - t0) / steps as f64;
    let mut out = Vec::with_capacity(steps + 1);
    let mut y = y0.to_vec();
    let mut t = t0;
    out.push((t, y.clone()));
    for _ in 0..steps {
        y = rk4_step(f, t, &y, h);
        t += h;
        out.push((t, y.clone()));
    }
    out
}

/// Composite Simpson's rule for a scalar integrand over `[a, b]` with `panels` subintervals
/// (forced even, `≥ 2`). The general-`N` version of the fixed 100-chunk solver in [`super`].
pub fn simpson<F: Fn(f64) -> f64>(f: &F, a: f64, b: f64, panels: usize) -> f64 {
    let n = even_panels(panels);
    let h = (b - a) / n as f64;
    let mut sum = f(a) + f(b);
    for i in 1..n {
        let x = a + i as f64 * h;
        sum += if i % 2 == 1 { 4.0 } else { 2.0 } * f(x);
    }
    sum * h / 3.0
}

/// Composite Simpson's rule for a **vector-valued** integrand `g: t → ℝᵏ`, integrated
/// component-wise (e.g. a vector field along a parameter, or a state trajectory). The output
/// length is the length of `g(a)`; `panels` is forced even and `≥ 2`.
pub fn simpson_vec<F: Fn(f64) -> Vec<f64>>(g: &F, a: f64, b: f64, panels: usize) -> Vec<f64> {
    let n = even_panels(panels);
    let h = (b - a) / n as f64;
    let ga = g(a);
    let gb = g(b);
    let k = ga.len();
    let mut acc: Vec<f64> = (0..k).map(|j| ga[j] + gb[j]).collect();
    for i in 1..n {
        let x = a + i as f64 * h;
        let w = if i % 2 == 1 { 4.0 } else { 2.0 };
        let gx = g(x);
        for j in 0..k {
            acc[j] += w * gx[j];
        }
    }
    acc.iter().map(|v| v * h / 3.0).collect()
}

fn even_panels(panels: usize) -> usize {
    let n = panels.max(2);
    if n % 2 == 0 {
        n
    } else {
        n + 1
    }
}

/// Shooting-method boundary-value solver for a first-order system `y' = f(t, y)` of dimension
/// `n`. The components of the initial state listed in `free` are the unknowns; they are chosen
/// by Newton's method so the user `residual(y(t1))` (length = `free.len()`) is driven to zero.
/// The Jacobian is built by forward finite differences and solved with the canonical
/// [`lu_solve`]. Returns the converged **initial** state, or `None` if it fails to converge
/// within `max_iter` (or the Jacobian is singular).
///
/// This generalises the fixed `[f64; 4]` damped-update shooting solver to arbitrary state
/// dimension and an arbitrary number of free initial conditions, with a real Newton step.
pub fn shooting_bvp<F, R>(
    f: &F,
    t0: f64,
    t1: f64,
    steps: usize,
    y0_init: &[f64],
    free: &[usize],
    residual: &R,
    tol: f64,
    max_iter: usize,
) -> Option<Vec<f64>>
where
    F: Fn(f64, &[f64]) -> Vec<f64>,
    R: Fn(&[f64]) -> Vec<f64>,
{
    let m = free.len();
    if m == 0 || free.iter().any(|&i| i >= y0_init.len()) {
        return None;
    }
    let mut y0 = y0_init.to_vec();

    for _ in 0..max_iter {
        let yf = rk4_integrate(f, t0, &y0, t1, steps);
        let r = residual(&yf);
        if r.len() != m {
            return None;
        }
        let rnorm = r.iter().fold(0.0_f64, |a, &v| a.max(v.abs()));
        if rnorm < tol {
            return Some(y0);
        }

        // Finite-difference Jacobian J[i][j] = ∂rᵢ/∂(free param j).
        let mut jac = vec![0.0; m * m];
        for (j, &idx) in free.iter().enumerate() {
            let h = 1e-6 * y0[idx].abs().max(1e-3);
            let mut yp = y0.clone();
            yp[idx] += h;
            let yfp = rk4_integrate(f, t0, &yp, t1, steps);
            let rp = residual(&yfp);
            if rp.len() != m {
                return None;
            }
            for i in 0..m {
                jac[i * m + j] = (rp[i] - r[i]) / h;
            }
        }

        // Solve J·Δ = −r and update the free components.
        let neg_r: Vec<f64> = r.iter().map(|v| -v).collect();
        let delta = lu_solve(m, &jac, &neg_r)?;
        for (j, &idx) in free.iter().enumerate() {
            y0[idx] += delta[j];
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f64::consts::PI;

    #[test]
    fn rk4_solves_a_3d_linear_system() {
        // y₀' = y₁, y₁' = -y₀ (harmonic), y₂' = -y₂ (decay). Start [0,1,1] at t=0.
        // Exact at t = π/2: [sin, cos, e^{-t}] = [1, 0, e^{-π/2}].
        let f = |_t: f64, y: &[f64]| vec![y[1], -y[0], -y[2]];
        let yf = rk4_integrate(&f, 0.0, &[0.0, 1.0, 1.0], PI / 2.0, 2000);
        assert!((yf[0] - 1.0).abs() < 1e-6, "y0 = {}", yf[0]);
        assert!(yf[1].abs() < 1e-6, "y1 = {}", yf[1]);
        assert!((yf[2] - (-PI / 2.0).exp()).abs() < 1e-6, "y2 = {}", yf[2]);
    }

    #[test]
    fn rk4_trajectory_has_all_points() {
        let f = |_t: f64, y: &[f64]| vec![-y[0]];
        let traj = rk4_solve(&f, 0.0, &[1.0], 1.0, 10);
        assert_eq!(traj.len(), 11);
        assert_eq!(traj[0].0, 0.0);
        assert!((traj.last().unwrap().1[0] - (-1.0_f64).exp()).abs() < 1e-6);
    }

    #[test]
    fn simpson_scalar_and_vector() {
        // ∫₀^π sin = 2 ; arbitrary (odd) panel count gets bumped to even.
        assert!((simpson(&|x: f64| x.sin(), 0.0, PI, 999) - 2.0).abs() < 1e-6);
        // Vector: ∫₀¹ [1, x, x²] = [1, 1/2, 1/3].
        let v = simpson_vec(&|x: f64| vec![1.0, x, x * x], 0.0, 1.0, 100);
        assert!((v[0] - 1.0).abs() < 1e-9);
        assert!((v[1] - 0.5).abs() < 1e-9);
        assert!((v[2] - 1.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn shooting_bvp_recovers_sine() {
        // y'' = -y, y(0) = 0, y(π/2) = 1 → y(t) = sin(t), so the initial slope is 1.
        // System: y₀' = y₁, y₁' = -y₀. Unknown = y₁(0) (index 1). Residual = y₀(π/2) − 1.
        let f = |_t: f64, y: &[f64]| vec![y[1], -y[0]];
        let residual = |yf: &[f64]| vec![yf[0] - 1.0];
        let y0 = shooting_bvp(&f, 0.0, PI / 2.0, 400, &[0.0, 0.0], &[1], &residual, 1e-10, 50)
            .expect("BVP should converge");
        assert!((y0[1] - 1.0).abs() < 1e-6, "initial slope = {}", y0[1]);
        // The recovered trajectory hits the right boundary.
        let yf = rk4_integrate(&f, 0.0, &y0, PI / 2.0, 400);
        assert!((yf[0] - 1.0).abs() < 1e-8);
    }

    #[test]
    fn shooting_bvp_rejects_bad_free_index() {
        let f = |_t: f64, y: &[f64]| vec![y[1], -y[0]];
        let residual = |yf: &[f64]| vec![yf[0] - 1.0];
        assert!(shooting_bvp(&f, 0.0, 1.0, 10, &[0.0, 0.0], &[5], &residual, 1e-9, 10).is_none());
    }
}
