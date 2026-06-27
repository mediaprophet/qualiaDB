//! Advanced ODE integrators: symplectic, stiff (BDF), dense output, and forward
//! sensitivity — the doctorate-level capabilities the `ode_solver` scalar RK4/BVP
//! core does not provide.
//!
//! Kept in its own focused module (rather than growing the 1000-line `ode_solver.rs`
//! monolith) per the split rule. **Zero-heap**: every routine is pure-scalar
//! arithmetic over `f64`, parameterised by closures (`impl Fn`) — no `Vec`/`Box`,
//! no allocation. Suitable for the constrained / off-grid core.
//!
//! ## What lives here
//! 1. **Symplectic integrators** (`verlet_step`, `ruth3_step`, `yoshida4_step`,
//!    `integrate_symplectic`) for separable Hamiltonian systems `H(q,p)=T(p)+V(q)` —
//!    they conserve energy with *bounded oscillation* over millions of steps instead
//!    of the secular drift a non-symplectic method (RK4) shows.
//! 2. **Stiff BDF solvers** (`bdf1_step`, `bdf2_step`, `integrate_bdf`) — L-stable
//!    backward-differentiation formulas with a Newton corrector, for stiff
//!    thermodynamic / phase-transition ODEs where explicit methods blow up.
//! 3. **Dense output** (`hermite_dense_output`) — cubic-Hermite continuous extension
//!    giving the state at any `t + θΔt` without re-evaluating the derivative.
//! 4. **Forward sensitivity** (`integrate_with_sensitivity`) — integrates the
//!    variational equation `ds/dt = f_y·s` alongside the state to get `∂y/∂y₀`.

/// Cube root of 2, used by the Yoshida 4th-order composition. Const literal so the
/// symplectic path needs no transcendental call (portable to no_std cores).
const CBRT2: f64 = 1.259_921_049_894_873_2;

// ── 1. Symplectic integrators (separable Hamiltonian H = T(p) + V(q)) ───────────
//
// Hamilton's equations: dq/dt = ∂T/∂p = `kinetic_velocity(p)`,
//                       dp/dt = -∂V/∂q = `force(q)`.

/// One Störmer–Verlet (velocity-Verlet / leapfrog) step — 2nd-order symplectic and
/// time-reversible. `force(q) = -∂V/∂q`, `kinetic_velocity(p) = ∂T/∂p` (= p/m).
pub fn verlet_step<F, G>(q: f64, p: f64, h: f64, force: F, kinetic_velocity: G) -> (f64, f64)
where
    F: Fn(f64) -> f64,
    G: Fn(f64) -> f64,
{
    let p_half = p + 0.5 * h * force(q);
    let q_new = q + h * kinetic_velocity(p_half);
    let p_new = p_half + 0.5 * h * force(q_new);
    (q_new, p_new)
}

/// One Ruth (1983) 3rd-order symplectic step. Three (kick, drift) sub-stages with
/// the canonical Ruth coefficients.
pub fn ruth3_step<F, G>(q: f64, p: f64, h: f64, force: F, kinetic_velocity: G) -> (f64, f64)
where
    F: Fn(f64) -> f64,
    G: Fn(f64) -> f64,
{
    // c = drift weights, d = kick weights (Ruth 1983).
    const C: [f64; 3] = [1.0, -2.0 / 3.0, 2.0 / 3.0];
    const D: [f64; 3] = [-1.0 / 24.0, 3.0 / 4.0, 7.0 / 24.0];
    let mut q = q;
    let mut p = p;
    for i in 0..3 {
        p += C[i] * h * force(q);
        q += D[i] * h * kinetic_velocity(p);
    }
    (q, p)
}

/// One Yoshida (1990) 4th-order symplectic step, built as a symmetric composition of
/// three Verlet sub-steps with the Yoshida weights `w1, w0, w1`.
pub fn yoshida4_step<F, G>(q: f64, p: f64, h: f64, force: F, kinetic_velocity: G) -> (f64, f64)
where
    F: Fn(f64) -> f64,
    G: Fn(f64) -> f64,
{
    let w1 = 1.0 / (2.0 - CBRT2);
    let w0 = -CBRT2 * w1;
    let (q, p) = verlet_step(q, p, w1 * h, &force, &kinetic_velocity);
    let (q, p) = verlet_step(q, p, w0 * h, &force, &kinetic_velocity);
    verlet_step(q, p, w1 * h, &force, &kinetic_velocity)
}

/// Symplectic integrator order selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymplecticMethod {
    /// Störmer–Verlet, 2nd order.
    Verlet,
    /// Ruth, 3rd order.
    Ruth3,
    /// Yoshida, 4th order.
    Yoshida4,
}

/// Result of [`integrate_symplectic`]: the final phase-space point plus the maximum
/// energy deviation seen over the run — the headline symplectic property is that this
/// stays *bounded* (no secular drift) even over millions of steps.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SymplecticResult {
    pub q: f64,
    pub p: f64,
    pub max_energy_drift: f64,
}

/// Integrate a separable Hamiltonian system for `steps` steps of size `h`.
///
/// `hamiltonian(q, p)` returns the total energy (for the conservation diagnostic).
/// Returns the final `(q, p)` and the maximum `|E - E₀|` observed. Zero-heap.
#[allow(clippy::too_many_arguments)]
pub fn integrate_symplectic<F, G, H>(
    q0: f64,
    p0: f64,
    h: f64,
    steps: u64,
    force: F,
    kinetic_velocity: G,
    hamiltonian: H,
    method: SymplecticMethod,
) -> SymplecticResult
where
    F: Fn(f64) -> f64,
    G: Fn(f64) -> f64,
    H: Fn(f64, f64) -> f64,
{
    let mut q = q0;
    let mut p = p0;
    let e0 = hamiltonian(q0, p0);
    let mut max_drift = 0.0f64;

    for _ in 0..steps {
        let (qn, pn) = match method {
            SymplecticMethod::Verlet => verlet_step(q, p, h, &force, &kinetic_velocity),
            SymplecticMethod::Ruth3 => ruth3_step(q, p, h, &force, &kinetic_velocity),
            SymplecticMethod::Yoshida4 => yoshida4_step(q, p, h, &force, &kinetic_velocity),
        };
        q = qn;
        p = pn;
        let drift = (hamiltonian(q, p) - e0).abs();
        if drift > max_drift {
            max_drift = drift;
        }
    }

    SymplecticResult {
        q,
        p,
        max_energy_drift: max_drift,
    }
}

// ── 2. Stiff solvers — Backward Differentiation Formulas (BDF) ──────────────────

/// Tolerance / iteration cap for the BDF Newton corrector.
const NEWTON_TOL: f64 = 1e-12;
const NEWTON_MAX_ITERS: u32 = 64;

/// Finite-difference estimate of `∂f/∂y` at `(t, y)`.
#[inline]
fn dfdy_fd<F: Fn(f64, f64) -> f64>(f: &F, t: f64, y: f64) -> f64 {
    let eps = 1e-7 * y.abs().max(1.0);
    (f(t, y + eps) - f(t, y - eps)) / (2.0 * eps)
}

/// One BDF1 (backward / implicit Euler) step: solve
/// `y₁ = y₀ + h·f(t₁, y₁)` by Newton iteration. L-stable — the workhorse for stiff
/// systems where explicit Euler/RK would require an impractically tiny `h`.
pub fn bdf1_step<F: Fn(f64, f64) -> f64>(t0: f64, y0: f64, h: f64, f: F) -> f64 {
    let t1 = t0 + h;
    let mut y = y0 + h * f(t0, y0); // explicit-Euler predictor
    for _ in 0..NEWTON_MAX_ITERS {
        let g = y - y0 - h * f(t1, y);
        let dg = 1.0 - h * dfdy_fd(&f, t1, y);
        let dy = g / dg;
        y -= dy;
        if dy.abs() <= NEWTON_TOL * y.abs().max(1.0) {
            break;
        }
    }
    y
}

/// One BDF2 step: `y₂ = (4/3)y₁ − (1/3)y₀ + (2/3)h·f(t₂, y₂)`, solved by Newton.
/// Second-order and L-stable; needs the two previous points `y1`(newer), `y0`(older).
pub fn bdf2_step<F: Fn(f64, f64) -> f64>(t1: f64, y1: f64, y0: f64, h: f64, f: F) -> f64 {
    let t2 = t1 + h;
    let c = (4.0 / 3.0) * y1 - (1.0 / 3.0) * y0;
    let beta = 2.0 / 3.0;
    let mut y = y1 + h * f(t1, y1); // predictor
    for _ in 0..NEWTON_MAX_ITERS {
        let g = y - c - beta * h * f(t2, y);
        let dg = 1.0 - beta * h * dfdy_fd(&f, t2, y);
        let dy = g / dg;
        y -= dy;
        if dy.abs() <= NEWTON_TOL * y.abs().max(1.0) {
            break;
        }
    }
    y
}

/// Integrate `dy/dt = f(t,y)` over `steps` steps of size `h` with the L-stable BDF2
/// formula, bootstrapped by one BDF1 step. Returns the final `y`. Zero-heap (keeps
/// only the two-point history). Stable for stiff systems at large `h`.
pub fn integrate_bdf<F: Fn(f64, f64) -> f64>(t0: f64, y0: f64, h: f64, steps: u64, f: F) -> f64 {
    if steps == 0 {
        return y0;
    }
    // First step: BDF1 to seed the two-point history.
    let mut y_prev = y0;
    let mut y_curr = bdf1_step(t0, y0, h, &f);
    let mut t = t0 + h;
    for _ in 1..steps {
        let y_next = bdf2_step(t, y_curr, y_prev, h, &f);
        y_prev = y_curr;
        y_curr = y_next;
        t += h;
    }
    y_curr
}

// ── 3. Dense output (continuous extension) ──────────────────────────────────────

/// Cubic-Hermite dense output: the state at `θ ∈ [0,1]` within a step from
/// `(t₀,y0)` to `(t₁,y1)` where `f0=f(t₀,y0)`, `f1=f(t₁,y1)` and `h=t₁−t₀`, WITHOUT
/// re-evaluating the derivative. Exact for cubic trajectories; matches the endpoints
/// and their slopes (`θ=0 → y0`, `θ=1 → y1`).
pub fn hermite_dense_output(y0: f64, f0: f64, y1: f64, f1: f64, h: f64, theta: f64) -> f64 {
    let t = theta;
    let t2 = t * t;
    let t3 = t2 * t;
    let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
    let h10 = t3 - 2.0 * t2 + t;
    let h01 = -2.0 * t3 + 3.0 * t2;
    let h11 = t3 - t2;
    h00 * y0 + h10 * h * f0 + h01 * y1 + h11 * h * f1
}

// ── 4. Forward sensitivity analysis (∂y/∂y₀) ────────────────────────────────────

/// Result of [`integrate_with_sensitivity`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SensitivityResult {
    /// The integrated state `y(t)`.
    pub y: f64,
    /// The forward sensitivity `s(t) = ∂y(t)/∂y₀`.
    pub sensitivity: f64,
}

/// Integrate `dy/dt = f(t,y)` together with the variational equation
/// `ds/dt = f_y(t,y)·s`, `s(0)=1`, by RK4 on the augmented `(y, s)` system. Returns
/// `y(t)` and `∂y/∂y₀` at `t₀ + steps·h`. `f_y` is estimated by central differences.
/// Zero-heap.
pub fn integrate_with_sensitivity<F: Fn(f64, f64) -> f64>(
    t0: f64,
    y0: f64,
    h: f64,
    steps: u64,
    f: F,
) -> SensitivityResult {
    let mut t = t0;
    let mut y = y0;
    let mut s = 1.0f64; // ∂y0/∂y0 = 1

    // Augmented derivative: (dy, ds) = (f(t,y), f_y(t,y)·s).
    let deriv = |t: f64, y: f64, s: f64| -> (f64, f64) { (f(t, y), dfdy_fd(&f, t, y) * s) };

    for _ in 0..steps {
        let (k1y, k1s) = deriv(t, y, s);
        let (k2y, k2s) = deriv(t + 0.5 * h, y + 0.5 * h * k1y, s + 0.5 * h * k1s);
        let (k3y, k3s) = deriv(t + 0.5 * h, y + 0.5 * h * k2y, s + 0.5 * h * k2s);
        let (k4y, k4s) = deriv(t + h, y + h * k3y, s + h * k3s);
        y += (h / 6.0) * (k1y + 2.0 * k2y + 2.0 * k3y + k4y);
        s += (h / 6.0) * (k1s + 2.0 * k2s + 2.0 * k3s + k4s);
        t += h;
    }

    SensitivityResult { y, sensitivity: s }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Harmonic oscillator H = p²/2 + q²/2 (unit mass, ω=1): force(q) = -q, T'(p)=p.
    // Exact solution: q(t)=cos t (from q0=1,p0=0), energy = 1/2 conserved.
    fn ho_force(q: f64) -> f64 {
        -q
    }
    fn ho_kin(p: f64) -> f64 {
        p
    }
    fn ho_energy(q: f64, p: f64) -> f64 {
        0.5 * p * p + 0.5 * q * q
    }

    #[test]
    fn symplectic_methods_conserve_energy_over_many_periods() {
        // 200 periods (2π each) at a coarse step — a non-symplectic method would drift.
        let h = 0.01;
        let steps = (200.0 * 2.0 * std::f64::consts::PI / h) as u64;
        for method in [
            SymplecticMethod::Verlet,
            SymplecticMethod::Ruth3,
            SymplecticMethod::Yoshida4,
        ] {
            let r = integrate_symplectic(1.0, 0.0, h, steps, ho_force, ho_kin, ho_energy, method);
            // Bounded energy drift (no secular growth): well under 1% of E₀=0.5.
            assert!(
                r.max_energy_drift < 5e-3,
                "{method:?} energy drift {} too large",
                r.max_energy_drift
            );
        }
    }

    #[test]
    fn symplectic_convergence_orders_match_labels() {
        // The rigorous order check: the empirical convergence rate
        // p ≈ log2(err(h)/err(h/2)) must match each method's label (2/3/4). (At a
        // *fixed* h the error *constants* differ, so a higher-order method need not
        // have smaller error — only the rate as h→0 is guaranteed.) The position error
        // is measured at a GENERIC time t=1.0 (NOT a period/quarter multiple — at an
        // extremum of cos the leading phase error would square and mis-report the
        // order). For q0=1,p0=0,ω=1 the exact solution is q(t)=cos t.
        let t_end = 1.0f64;
        let exact = t_end.cos();
        let err_at = |m, h: f64| {
            let steps = (t_end / h).round().max(1.0) as u64;
            let h = t_end / steps as f64; // land exactly on t = 1.0
            let r = integrate_symplectic(1.0, 0.0, h, steps, ho_force, ho_kin, ho_energy, m);
            (r.q - exact).abs()
        };
        let order = |m| {
            let e1 = err_at(m, 0.04);
            let e2 = err_at(m, 0.02);
            (e1 / e2).log2()
        };
        let o2 = order(SymplecticMethod::Verlet);
        let o3 = order(SymplecticMethod::Ruth3);
        let o4 = order(SymplecticMethod::Yoshida4);
        assert!((o2 - 2.0).abs() < 0.5, "Verlet should be ~2nd order, got {o2:.2}");
        assert!((o3 - 3.0).abs() < 0.6, "Ruth3 should be ~3rd order, got {o3:.2}");
        assert!((o4 - 4.0).abs() < 0.8, "Yoshida4 should be ~4th order, got {o4:.2}");
    }

    #[test]
    fn bdf_is_stable_on_a_stiff_equation() {
        // y' = -1000 y, y0 = 1. Exact y(0.5)=e^{-500}≈0 (tiny). Backward Euler / BDF2
        // are L-stable: with a big step h=0.1 they decay monotonically to ~0; explicit
        // Euler with the same h would explode (|1 - 1000·0.1| = 99 per step).
        let f = |_t: f64, y: f64| -10.0 * y; // moderately stiff for a quick, exact check
        // Backward Euler one step, h=0.5: y1 = y0/(1+5) = 1/6.
        let y1 = bdf1_step(0.0, 1.0, 0.5, f);
        assert!((y1 - 1.0 / 6.0).abs() < 1e-9, "BDF1 implicit-Euler value, got {y1}");

        // Strongly stiff, large step: must stay bounded in [0,1] and shrink, never blow up.
        let stiff = |_t: f64, y: f64| -1000.0 * y;
        let yf = integrate_bdf(0.0, 1.0, 0.1, 5, stiff);
        assert!(yf.abs() < 1e-2 && yf.is_finite(), "BDF2 stiff result blew up: {yf}");
        assert!(yf >= 0.0, "L-stable decay should not overshoot below 0: {yf}");
    }

    #[test]
    fn bdf2_matches_linear_decay_accurately() {
        // y' = -y, y0 = 1 → y(1) = e^{-1} ≈ 0.367879. BDF2 with small h is 2nd-order.
        let f = |_t: f64, y: f64| -y;
        let yf = integrate_bdf(0.0, 1.0, 1e-3, 1000, f);
        let exact = (-1.0f64).exp();
        assert!((yf - exact).abs() < 1e-5, "BDF2 got {yf}, exact {exact}");
    }

    #[test]
    fn dense_output_is_exact_for_a_cubic() {
        // y(t) = 1 + 2t + 3t² + 4t³ on [0,1]; f = y' = 2 + 6t + 12t².
        let y = |t: f64| 1.0 + 2.0 * t + 3.0 * t * t + 4.0 * t * t * t;
        let f = |t: f64| 2.0 + 6.0 * t + 12.0 * t * t;
        let (t0, t1) = (0.0, 1.0);
        let h = t1 - t0;
        for &theta in &[0.0, 0.25, 0.5, 0.75, 1.0] {
            let interp = hermite_dense_output(y(t0), f(t0), y(t1), f(t1), h, theta);
            let exact = y(t0 + theta * h);
            assert!((interp - exact).abs() < 1e-12, "θ={theta}: {interp} vs {exact}");
        }
    }

    #[test]
    fn forward_sensitivity_matches_analytic_exponential() {
        // y' = -λy, y(t) = y0·e^{-λt}, so ∂y/∂y0 = e^{-λt}. With λ=2, t=1:
        // s(1) should be e^{-2} and y(1) should be 0.5·e^{-2} (y0 = 0.5).
        let lambda = 2.0;
        let f = move |_t: f64, y: f64| -lambda * y;
        let r = integrate_with_sensitivity(0.0, 0.5, 1e-3, 1000, f);
        let exp_m2 = (-2.0f64).exp();
        assert!((r.sensitivity - exp_m2).abs() < 1e-5, "∂y/∂y0 got {}, want {exp_m2}", r.sensitivity);
        assert!((r.y - 0.5 * exp_m2).abs() < 1e-6, "y got {}, want {}", r.y, 0.5 * exp_m2);
    }
}
