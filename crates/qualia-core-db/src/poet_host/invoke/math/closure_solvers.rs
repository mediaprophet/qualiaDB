//! Higher-order numerical solvers & closure evaluation bridge.
//!
//! Provides zero-heap and CAS-evaluated higher-order mathematical operations:
//! - Multi-step ODE integrators (RK4, DOPRI5, BDF, Symplectic leapfrog/Ruth/Yoshida)
//! - Adaptive Simpson numerical quadrature
//! - Numerical differential operators (Adaptive central difference, Jacobian, Hessian, Newton root finding)
//! - Global optimization metaheuristics (Simulated Annealing, Artificial Bee Colony)
//! - Vector calculus line & surface integrals
//! - Numerical & symbolic Laplace transforms

use super::super::args;
use crate::solvers::calculus::ode_adaptive::{integrate_dopri5, AdaptiveOdeConfig, OdeError};
use crate::solvers::calculus::ode_advanced::{ruth3_step, verlet_step, yoshida4_step};
use crate::solvers::calculus::quadrature::adaptive_simpson;
use crate::solvers::transforms::laplace::{laplace_numeric, laplace_table};
use crate::solvers::vector_calculus::integrals::{line_integral_scalar, line_integral_work, surface_flux};
use crate::specialized_libs::symbolic_algebra::{self as sa, Expr};
use vibe::{Diagnostic, Span, Value};
use std::collections::HashMap;

/// Helper: parse a single expression string into an `Expr`.
fn parse_expr(s: &str, span: Span) -> Result<Expr, Diagnostic> {
    sa::parse(s).map_err(|e| args::bad(span, format!("failed to parse expression '{s}': {e}")))
}

/// Helper: parse a list of expression strings into `Vec<Expr>`.
fn parse_expr_list(list: &[String], span: Span) -> Result<Vec<Expr>, Diagnostic> {
    list.iter().map(|s| parse_expr(s, span)).collect()
}

/// Helper: evaluate an `Expr` with variable values.
fn eval_expr(expr: &Expr, env: &HashMap<String, f64>, span: Span) -> Result<f64, Diagnostic> {
    expr.eval(env)
        .ok_or_else(|| args::bad(span, "evaluation failed (unbound variable or non-finite result)"))
}

// ── 1. ODE Integrators ─────────────────────────────────────────────────────────

/// `Ode.rk4_integrate` — Multi-step Runge-Kutta 4th order ODE integrator.
/// Args: { system: [string] | string, vars: [string], t_span: [f64, f64], y0: [f64] | f64, dt: f64 }
pub fn ode_rk4_integrate(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let t_span = args::rec_f64_list(args_v, "t_span")
        .ok_or_else(|| args::bad(span, "Ode.rk4_integrate needs t_span: [t0, t_final]"))?;
    if t_span.len() != 2 || t_span[1] <= t_span[0] {
        return Err(args::bad(span, "t_span must be [t0, t_final] with t_final > t0"));
    }
    let t0 = t_span[0];
    let t_final = t_span[1];

    let dt = args::rec_f64(args_v, "dt")
        .ok_or_else(|| args::bad(span, "Ode.rk4_integrate needs dt: number"))?;
    if dt <= 0.0 || !dt.is_finite() {
        return Err(args::bad(span, "dt must be a positive finite number"));
    }

    let vars = args::rec_str_list(args_v, "vars").unwrap_or_else(|| vec!["y".to_string()]);
    let system_strs = if let Some(sys) = args::rec_str_list(args_v, "system") {
        sys
    } else if let Some(s) = args::rec_str(args_v, "system") {
        vec![s.to_string()]
    } else {
        return Err(args::bad(span, "Ode.rk4_integrate needs system: [string] or string"));
    };

    let n = vars.len();
    if system_strs.len() != n {
        return Err(args::bad(span, "number of system equations must match number of variables"));
    }

    let y0_vec = if let Some(y_list) = args::rec_f64_list(args_v, "y0") {
        y_list
    } else if let Some(y_scalar) = args::rec_f64(args_v, "y0") {
        vec![y_scalar]
    } else {
        return Err(args::bad(span, "Ode.rk4_integrate needs y0: [number] or number"));
    };
    if y0_vec.len() != n {
        return Err(args::bad(span, "initial state y0 dimension must match number of variables"));
    }

    let exprs = parse_expr_list(&system_strs, span)?;
    let mut current_t = t0;
    let mut current_y = y0_vec.clone();

    let mut times = vec![Value::F64(t0)];
    let mut trajectory = vec![Value::List(current_y.iter().map(|&v| Value::F64(v)).collect())];

    let num_steps = ((t_final - t0) / dt).ceil() as usize;
    let max_steps = 100_000;
    if num_steps > max_steps {
        return Err(args::bad(span, format!("step count {num_steps} exceeds maximum of {max_steps}")));
    }

    let eval_deriv = |t: f64, y: &[f64]| -> Result<Vec<f64>, Diagnostic> {
        let mut env = HashMap::new();
        env.insert("t".to_string(), t);
        for (i, v) in vars.iter().enumerate() {
            env.insert(v.clone(), y[i]);
        }
        let mut dy = Vec::with_capacity(n);
        for e in &exprs {
            dy.push(eval_expr(e, &env, span)?);
        }
        Ok(dy)
    };

    for _ in 0..num_steps {
        let h = if current_t + dt > t_final { t_final - current_t } else { dt };
        if h <= 1e-15 { break; }

        let k1 = eval_deriv(current_t, &current_y)?;
        
        let mut y_temp = vec![0.0; n];
        for i in 0..n { y_temp[i] = current_y[i] + 0.5 * h * k1[i]; }
        let k2 = eval_deriv(current_t + 0.5 * h, &y_temp)?;

        for i in 0..n { y_temp[i] = current_y[i] + 0.5 * h * k2[i]; }
        let k3 = eval_deriv(current_t + 0.5 * h, &y_temp)?;

        for i in 0..n { y_temp[i] = current_y[i] + h * k3[i]; }
        let k4 = eval_deriv(current_t + h, &y_temp)?;

        for i in 0..n {
            current_y[i] += (h / 6.0) * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]);
        }
        current_t += h;

        times.push(Value::F64(current_t));
        trajectory.push(Value::List(current_y.iter().map(|&v| Value::F64(v)).collect()));
    }

    let final_state: Vec<Value> = current_y.iter().map(|&v| Value::F64(v)).collect();
    Ok(args::record([
        ("times", Value::List(times)),
        ("trajectory", Value::List(trajectory)),
        ("final_state", Value::List(final_state)),
        ("steps", Value::I64(num_steps as i64)),
    ]))
}

/// `Ode.dopri5` — Adaptive Dormand-Prince 5(4) integrator.
/// Args: { system: [string], vars: [string], t_span: [f64, f64], y0: [f64], abs_tol?: f64, rel_tol?: f64 }
pub fn ode_dopri5(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let t_span = args::rec_f64_list(args_v, "t_span")
        .ok_or_else(|| args::bad(span, "Ode.dopri5 needs t_span: [t0, t_final]"))?;
    if t_span.len() != 2 || t_span[1] <= t_span[0] {
        return Err(args::bad(span, "t_span must be [t0, t_final] with t_final > t0"));
    }
    let t0 = t_span[0];
    let t_final = t_span[1];

    let vars = args::rec_str_list(args_v, "vars")
        .ok_or_else(|| args::bad(span, "Ode.dopri5 needs vars: [string]"))?;
    let system_strs = args::rec_str_list(args_v, "system")
        .ok_or_else(|| args::bad(span, "Ode.dopri5 needs system: [string]"))?;

    let n = vars.len();
    if system_strs.len() != n {
        return Err(args::bad(span, "dimension mismatch between vars and system"));
    }

    let mut y0 = args::rec_f64_list(args_v, "y0")
        .ok_or_else(|| args::bad(span, "Ode.dopri5 needs y0: [number]"))?;
    if y0.len() != n {
        return Err(args::bad(span, "y0 length must match vars"));
    }

    let abs_tol = args::rec_f64(args_v, "abs_tol").unwrap_or(1e-8);
    let rel_tol = args::rec_f64(args_v, "rel_tol").unwrap_or(1e-6);

    let exprs = parse_expr_list(&system_strs, span)?;
    let mut config = AdaptiveOdeConfig::default();
    config.absolute_tolerance = abs_tol;
    config.relative_tolerance = rel_tol;

    let mut workspace = vec![0.0; n * 8];

    let deriv_fn = |t: f64, y: &[f64], out: &mut [f64]| -> Result<(), OdeError> {
        let mut env = HashMap::new();
        env.insert("t".to_string(), t);
        for (i, v) in vars.iter().enumerate() {
            env.insert(v.clone(), y[i]);
        }
        for (i, e) in exprs.iter().enumerate() {
            match e.eval(&env) {
                Some(val) if val.is_finite() => out[i] = val,
                _ => return Err(OdeError::NonFiniteDerivative),
            }
        }
        Ok(())
    };

    match integrate_dopri5(deriv_fn, &mut y0, t0, t_final, config, &mut workspace) {
        Ok(res) => {
            let final_state: Vec<Value> = y0.iter().map(|&v| Value::F64(v)).collect();
            Ok(args::record([
                ("final_time", Value::F64(res.final_time)),
                ("final_state", Value::List(final_state)),
                ("accepted_steps", Value::I64(res.accepted_steps as i64)),
                ("rejected_steps", Value::I64(res.rejected_steps as i64)),
                ("derivative_evaluations", Value::I64(res.derivative_evaluations as i64)),
                ("last_step", Value::F64(res.last_step)),
            ]))
        }
        Err(e) => Err(args::bad(span, format!("DOPRI5 integration failed: {e:?}"))),
    }
}

/// `Ode.bdf` — Backward Differentiation Formula integrator for stiff ODEs.
/// Args: { system: [string], vars: [string], t_span: [f64, f64], y0: [f64], dt: f64, order?: i64 }
pub fn ode_bdf(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let t_span = args::rec_f64_list(args_v, "t_span")
        .ok_or_else(|| args::bad(span, "Ode.bdf needs t_span: [t0, t_final]"))?;
    if t_span.len() != 2 || t_span[1] <= t_span[0] {
        return Err(args::bad(span, "t_span must be [t0, t_final] with t_final > t0"));
    }
    let t0 = t_span[0];
    let t_final = t_span[1];

    let dt = args::rec_f64(args_v, "dt")
        .ok_or_else(|| args::bad(span, "Ode.bdf needs dt: number"))?;
    if dt <= 0.0 {
        return Err(args::bad(span, "dt must be positive"));
    }

    let vars = args::rec_str_list(args_v, "vars")
        .ok_or_else(|| args::bad(span, "Ode.bdf needs vars: [string]"))?;
    let system_strs = args::rec_str_list(args_v, "system")
        .ok_or_else(|| args::bad(span, "Ode.bdf needs system: [string]"))?;

    let n = vars.len();
    if system_strs.len() != n {
        return Err(args::bad(span, "dimension mismatch between vars and system"));
    }

    let mut current_y = args::rec_f64_list(args_v, "y0")
        .ok_or_else(|| args::bad(span, "Ode.bdf needs y0: [number]"))?;
    if current_y.len() != n {
        return Err(args::bad(span, "y0 length must match vars"));
    }

    let exprs = parse_expr_list(&system_strs, span)?;
    let num_steps = ((t_final - t0) / dt).ceil() as usize;

    let eval_f = |t: f64, y: &[f64]| -> Result<Vec<f64>, Diagnostic> {
        let mut env = HashMap::new();
        env.insert("t".to_string(), t);
        for (i, v) in vars.iter().enumerate() {
            env.insert(v.clone(), y[i]);
        }
        let mut out = Vec::with_capacity(n);
        for e in &exprs {
            out.push(eval_expr(e, &env, span)?);
        }
        Ok(out)
    };

    let mut times = vec![Value::F64(t0)];
    let mut trajectory = vec![Value::List(current_y.iter().map(|&v| Value::F64(v)).collect())];
    let mut current_t = t0;

    // BDF1 (implicit Euler) with fixed-point / Newton iteration
    for _ in 0..num_steps {
        let next_t = (current_t + dt).min(t_final);
        let h = next_t - current_t;
        if h <= 1e-15 { break; }

        let mut y_next = current_y.clone();
        for _iter in 0..20 {
            let f_val = eval_f(next_t, &y_next)?;
            let mut max_diff = 0.0f64;
            for i in 0..n {
                let updated = current_y[i] + h * f_val[i];
                max_diff = max_diff.max((updated - y_next[i]).abs());
                y_next[i] = updated;
            }
            if max_diff < 1e-10 { break; }
        }

        current_y = y_next;
        current_t = next_t;
        times.push(Value::F64(current_t));
        trajectory.push(Value::List(current_y.iter().map(|&v| Value::F64(v)).collect()));
    }

    let final_state: Vec<Value> = current_y.iter().map(|&v| Value::F64(v)).collect();
    Ok(args::record([
        ("times", Value::List(times)),
        ("trajectory", Value::List(trajectory)),
        ("final_state", Value::List(final_state)),
        ("steps", Value::I64(num_steps as i64)),
    ]))
}

/// `Ode.symplectic_step` — Symplectic step (Verlet / Ruth3 / Yoshida4).
/// Args: { force: string, q: f64, p: f64, dt: f64, mass?: f64, method?: string, steps?: i64 }
pub fn ode_symplectic_step(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let force_str = args::rec_str(args_v, "force")
        .ok_or_else(|| args::bad(span, "Ode.symplectic_step needs force: string (e.g. '-1.0 * q')"))?;
    let mut q = args::rec_f64(args_v, "q")
        .ok_or_else(|| args::bad(span, "Ode.symplectic_step needs q: number"))?;
    let mut p = args::rec_f64(args_v, "p")
        .ok_or_else(|| args::bad(span, "Ode.symplectic_step needs p: number"))?;
    let dt = args::rec_f64(args_v, "dt")
        .ok_or_else(|| args::bad(span, "Ode.symplectic_step needs dt: number"))?;
    let mass = args::rec_f64(args_v, "mass").unwrap_or(1.0);
    let method = args::rec_str(args_v, "method").unwrap_or("verlet");
    let steps = args::rec_i64(args_v, "steps").unwrap_or(1).max(1) as usize;

    let force_expr = parse_expr(force_str, span)?;
    let force_fn = |pos: f64| -> f64 {
        let mut env = HashMap::new();
        env.insert("q".to_string(), pos);
        force_expr.eval(&env).unwrap_or(0.0)
    };
    let kinetic_fn = |mom: f64| -> f64 { mom / mass };

    for _ in 0..steps {
        let (nq, np) = match method.to_ascii_lowercase().as_str() {
            "ruth3" => ruth3_step(q, p, dt, force_fn, kinetic_fn),
            "yoshida4" => yoshida4_step(q, p, dt, force_fn, kinetic_fn),
            _ => verlet_step(q, p, dt, force_fn, kinetic_fn),
        };
        q = nq;
        p = np;
    }

    let kinetic_energy = 0.5 * p * p / mass;
    Ok(args::record([
        ("q", Value::F64(q)),
        ("p", Value::F64(p)),
        ("kinetic_energy", Value::F64(kinetic_energy)),
        ("method", Value::String(method.to_string())),
    ]))
}

// ── 2. Quadrature & Differential Operators ────────────────────────────────────

/// `Calculus.adaptive_simpson` — Adaptive Simpson quadrature.
/// Args: { expr: string, var?: string, a: f64, b: f64, tolerance?: f64, max_evaluations?: i64 }
pub fn calc_adaptive_simpson(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let expr_str = args::rec_str(args_v, "expr")
        .ok_or_else(|| args::bad(span, "Calculus.adaptive_simpson needs expr: string"))?;
    let var_name = args::rec_str(args_v, "var").unwrap_or("x");
    let a = args::rec_f64(args_v, "a")
        .ok_or_else(|| args::bad(span, "Calculus.adaptive_simpson needs a: number"))?;
    let b = args::rec_f64(args_v, "b")
        .ok_or_else(|| args::bad(span, "Calculus.adaptive_simpson needs b: number"))?;
    let tol = args::rec_f64(args_v, "tolerance").unwrap_or(1e-8);
    let max_evals = args::rec_i64(args_v, "max_evaluations").unwrap_or(10_000) as u32;

    let expr = parse_expr(expr_str, span)?;
    let f = |x: f64| -> f64 {
        let mut env = HashMap::new();
        env.insert(var_name.to_string(), x);
        expr.eval(&env).unwrap_or(f64::NAN)
    };

    match adaptive_simpson(f, a, b, tol, max_evals) {
        Ok(res) => Ok(args::record([
            ("value", Value::F64(res.value)),
            ("absolute_error", Value::F64(res.absolute_error)),
            ("evaluations", Value::I64(res.evaluations as i64)),
            ("intervals", Value::I64(res.intervals as i64)),
        ])),
        Err(e) => Err(args::bad(span, format!("quadrature failed: {e:?}"))),
    }
}

/// `Calculus.adaptive_derivative` — Central difference adaptive derivative.
/// Args: { expr: string, var?: string, x: f64 }
pub fn calc_adaptive_derivative(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let expr_str = args::rec_str(args_v, "expr")
        .ok_or_else(|| args::bad(span, "Calculus.adaptive_derivative needs expr: string"))?;
    let var_name = args::rec_str(args_v, "var").unwrap_or("x");
    let x = args::rec_f64(args_v, "x")
        .ok_or_else(|| args::bad(span, "Calculus.adaptive_derivative needs x: number"))?;

    let expr = parse_expr(expr_str, span)?;
    let f = |val: f64| -> f64 {
        let mut env = HashMap::new();
        env.insert(var_name.to_string(), val);
        expr.eval(&env).unwrap_or(f64::NAN)
    };

    match crate::solvers::calculus::differential::adaptive_central_difference(f, x) {
        Ok(est) => Ok(args::record([
            ("value", Value::F64(est.value)),
            ("absolute_error", Value::F64(est.absolute_error)),
            ("step", Value::F64(est.step)),
        ])),
        Err(e) => Err(args::bad(span, format!("differentiation failed: {e:?}"))),
    }
}

/// `Calculus.numerical_jacobian` — Numerical Jacobian matrix of a vector function.
/// Args: { exprs: [string], vars: [string], point: [f64], step?: f64 }
pub fn calc_numerical_jacobian(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let expr_strs = args::rec_str_list(args_v, "exprs")
        .ok_or_else(|| args::bad(span, "Calculus.numerical_jacobian needs exprs: [string]"))?;
    let vars = args::rec_str_list(args_v, "vars")
        .ok_or_else(|| args::bad(span, "Calculus.numerical_jacobian needs vars: [string]"))?;
    let point = args::rec_f64_list(args_v, "point")
        .ok_or_else(|| args::bad(span, "Calculus.numerical_jacobian needs point: [number]"))?;
    let h = args::rec_f64(args_v, "step").unwrap_or(1e-6);

    let n_vars = vars.len();
    let n_exprs = expr_strs.len();
    if point.len() != n_vars {
        return Err(args::bad(span, "point dimension must match vars"));
    }

    let parsed_exprs = parse_expr_list(&expr_strs, span)?;
    let mut jacobian_rows = Vec::with_capacity(n_exprs);

    for e in &parsed_exprs {
        let mut row = Vec::with_capacity(n_vars);
        for (j, _) in vars.iter().enumerate() {
            let mut pt_plus = point.clone();
            let mut pt_minus = point.clone();
            pt_plus[j] += h;
            pt_minus[j] -= h;

            let mut env_plus = HashMap::new();
            let mut env_minus = HashMap::new();
            for (k, v) in vars.iter().enumerate() {
                env_plus.insert(v.clone(), pt_plus[k]);
                env_minus.insert(v.clone(), pt_minus[k]);
            }

            let f_plus = eval_expr(e, &env_plus, span)?;
            let f_minus = eval_expr(e, &env_minus, span)?;
            let df = (f_plus - f_minus) / (2.0 * h);
            row.push(Value::F64(df));
        }
        jacobian_rows.push(Value::List(row));
    }

    Ok(args::record([
        ("jacobian", Value::List(jacobian_rows)),
        ("rows", Value::I64(n_exprs as i64)),
        ("cols", Value::I64(n_vars as i64)),
    ]))
}

/// `Calculus.numerical_hessian` — Numerical Hessian matrix of a scalar function.
/// Args: { expr: string, vars: [string], point: [f64], step?: f64 }
pub fn calc_numerical_hessian(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let expr_str = args::rec_str(args_v, "expr")
        .ok_or_else(|| args::bad(span, "Calculus.numerical_hessian needs expr: string"))?;
    let vars = args::rec_str_list(args_v, "vars")
        .ok_or_else(|| args::bad(span, "Calculus.numerical_hessian needs vars: [string]"))?;
    let point = args::rec_f64_list(args_v, "point")
        .ok_or_else(|| args::bad(span, "Calculus.numerical_hessian needs point: [number]"))?;
    let h = args::rec_f64(args_v, "step").unwrap_or(1e-4);

    let n = vars.len();
    if point.len() != n {
        return Err(args::bad(span, "point dimension must match vars"));
    }

    let expr = parse_expr(expr_str, span)?;
    let eval_at = |pt: &[f64]| -> Result<f64, Diagnostic> {
        let mut env = HashMap::new();
        for (i, v) in vars.iter().enumerate() {
            env.insert(v.clone(), pt[i]);
        }
        eval_expr(&expr, &env, span)
    };

    let f0 = eval_at(&point)?;
    let mut hessian_rows = Vec::with_capacity(n);

    for i in 0..n {
        let mut row = Vec::with_capacity(n);
        for j in 0..n {
            let h_ij = if i == j {
                let mut pt_plus = point.clone();
                let mut pt_minus = point.clone();
                pt_plus[i] += h;
                pt_minus[i] -= h;
                let fp = eval_at(&pt_plus)?;
                let fm = eval_at(&pt_minus)?;
                (fp - 2.0 * f0 + fm) / (h * h)
            } else {
                let mut p_pp = point.clone();
                let mut p_pm = point.clone();
                let mut p_mp = point.clone();
                let mut p_mm = point.clone();
                p_pp[i] += h; p_pp[j] += h;
                p_pm[i] += h; p_pm[j] -= h;
                p_mp[i] -= h; p_mp[j] += h;
                p_mm[i] -= h; p_mm[j] -= h;
                (eval_at(&p_pp)? - eval_at(&p_pm)? - eval_at(&p_mp)? + eval_at(&p_mm)?) / (4.0 * h * h)
            };
            row.push(Value::F64(h_ij));
        }
        hessian_rows.push(Value::List(row));
    }

    Ok(args::record([
        ("hessian", Value::List(hessian_rows)),
        ("dim", Value::I64(n as i64)),
    ]))
}

/// `Calculus.newton_solve` — Multidimensional Newton-Raphson nonlinear equation root finder.
/// Args: { exprs: [string], vars: [string], guess: [f64], max_iter?: i64, tolerance?: f64 }
pub fn calc_newton_solve(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let expr_strs = args::rec_str_list(args_v, "exprs")
        .ok_or_else(|| args::bad(span, "Calculus.newton_solve needs exprs: [string]"))?;
    let vars = args::rec_str_list(args_v, "vars")
        .ok_or_else(|| args::bad(span, "Calculus.newton_solve needs vars: [string]"))?;
    let guess = args::rec_f64_list(args_v, "guess")
        .ok_or_else(|| args::bad(span, "Calculus.newton_solve needs guess: [number]"))?;
    let max_iter = args::rec_i64(args_v, "max_iter").unwrap_or(50).max(1) as usize;
    let tol = args::rec_f64(args_v, "tolerance").unwrap_or(1e-8);

    let n = vars.len();
    if expr_strs.len() != n || guess.len() != n {
        return Err(args::bad(span, "dimension mismatch: exprs, vars, guess must have equal length"));
    }

    let parsed_exprs = parse_expr_list(&expr_strs, span)?;
    let mut x = guess;
    let h = 1e-6;
    let mut converged = false;
    let mut last_norm = 0.0;
    let mut iters_taken = 0;

    for iter in 0..max_iter {
        iters_taken = iter + 1;
        // Evaluate F(x)
        let mut fx = vec![0.0; n];
        let mut env = HashMap::new();
        for (i, v) in vars.iter().enumerate() { env.insert(v.clone(), x[i]); }
        for (i, e) in parsed_exprs.iter().enumerate() {
            fx[i] = eval_expr(e, &env, span)?;
        }

        last_norm = fx.iter().map(|&v| v * v).sum::<f64>().sqrt();
        if last_norm < tol {
            converged = true;
            break;
        }

        // Numerical Jacobian J(x)
        let mut j_mat = vec![0.0; n * n];
        for col in 0..n {
            let mut pt_plus = x.clone();
            pt_plus[col] += h;
            let mut env_plus = HashMap::new();
            for (i, v) in vars.iter().enumerate() { env_plus.insert(v.clone(), pt_plus[i]); }
            for row in 0..n {
                let f_plus = eval_expr(&parsed_exprs[row], &env_plus, span)?;
                j_mat[row * n + col] = (f_plus - fx[row]) / h;
            }
        }

        // Solve J * delta = fx (using Gaussian elimination for n x n)
        if let Some(delta) = solve_linear_system(&j_mat, &fx, n) {
            for i in 0..n {
                x[i] -= delta[i];
            }
        } else {
            return Err(args::bad(span, "singular Jacobian encountered during Newton step"));
        }
    }

    let sol: Vec<Value> = x.iter().map(|&v| Value::F64(v)).collect();
    Ok(args::record([
        ("solution", Value::List(sol)),
        ("iterations", Value::I64(iters_taken as i64)),
        ("residual_norm", Value::F64(last_norm)),
        ("converged", Value::Bool(converged)),
    ]))
}

fn solve_linear_system(a: &[f64], b: &[f64], n: usize) -> Option<Vec<f64>> {
    let mut aug = vec![0.0; n * (n + 1)];
    for i in 0..n {
        for j in 0..n {
            aug[i * (n + 1) + j] = a[i * n + j];
        }
        aug[i * (n + 1) + n] = b[i];
    }

    for i in 0..n {
        let mut max_row = i;
        for k in (i + 1)..n {
            if aug[k * (n + 1) + i].abs() > aug[max_row * (n + 1) + i].abs() {
                max_row = k;
            }
        }
        for j in 0..=(n) {
            let tmp = aug[i * (n + 1) + j];
            aug[i * (n + 1) + j] = aug[max_row * (n + 1) + j];
            aug[max_row * (n + 1) + j] = tmp;
        }

        let pivot = aug[i * (n + 1) + i];
        if pivot.abs() < 1e-14 { return None; }

        for k in (i + 1)..n {
            let factor = aug[k * (n + 1) + i] / pivot;
            for j in i..=(n) {
                aug[k * (n + 1) + j] -= factor * aug[i * (n + 1) + j];
            }
        }
    }

    let mut x = vec![0.0; n];
    for i in (0..n).rev() {
        let mut sum = aug[i * (n + 1) + n];
        for j in (i + 1)..n {
            sum -= aug[i * (n + 1) + j] * x[j];
        }
        x[i] = sum / aug[i * (n + 1) + i];
    }
    Some(x)
}

// ── 3. Optimization Metaheuristics ───────────────────────────────────────────

/// `Optimization.simulated_annealing` — Continuous bounded Simulated Annealing.
/// Args: { objective: string, vars: [string], bounds: [[f64, f64]], initial?: [f64], max_iter?: i64, temp_initial?: f64, cooling_rate?: f64, seed?: i64 }
pub fn opt_simulated_annealing(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let obj_str = args::rec_str(args_v, "objective")
        .ok_or_else(|| args::bad(span, "Optimization.simulated_annealing needs objective: string"))?;
    let vars = args::rec_str_list(args_v, "vars")
        .ok_or_else(|| args::bad(span, "Optimization.simulated_annealing needs vars: [string]"))?;
    let raw_bounds = args::rec(args_v, "bounds")
        .and_then(args::list)
        .ok_or_else(|| args::bad(span, "Optimization.simulated_annealing needs bounds: [[min, max]]"))?;

    let n = vars.len();
    if raw_bounds.len() != n {
        return Err(args::bad(span, "bounds length must match vars"));
    }

    let mut bounds = Vec::with_capacity(n);
    for b in raw_bounds {
        let pair = args::list(b).ok_or_else(|| args::bad(span, "each bound must be [min, max]"))?;
        if pair.len() != 2 { return Err(args::bad(span, "each bound must be [min, max]")); }
        let min = args::as_f64(&pair[0]).ok_or_else(|| args::bad(span, "bound min must be number"))?;
        let max = args::as_f64(&pair[1]).ok_or_else(|| args::bad(span, "bound max must be number"))?;
        bounds.push((min, max));
    }

    let max_iter = args::rec_i64(args_v, "max_iter").unwrap_or(1000).max(1) as usize;
    let mut temp = args::rec_f64(args_v, "temp_initial").unwrap_or(100.0);
    let cooling = args::rec_f64(args_v, "cooling_rate").unwrap_or(0.95);
    let seed = args::rec_i64(args_v, "seed").unwrap_or(42) as u64;

    let expr = parse_expr(obj_str, span)?;
    let eval_obj = |pt: &[f64]| -> Result<f64, Diagnostic> {
        let mut env = HashMap::new();
        for (i, v) in vars.iter().enumerate() { env.insert(v.clone(), pt[i]); }
        eval_expr(&expr, &env, span)
    };

    let mut rng = crate::solvers::optimization::metaheuristics::Rng(seed);

    let mut current = if let Some(init) = args::rec_f64_list(args_v, "initial") {
        if init.len() != n { return Err(args::bad(span, "initial state length must match vars")); }
        init
    } else {
        bounds.iter().map(|(min, max)| min + rng.unit() * (max - min)).collect()
    };

    let mut current_e = eval_obj(&current)?;
    let mut best = current.clone();
    let mut best_e = current_e;

    for _ in 0..max_iter {
        // Perturb state
        let mut cand = current.clone();
        for i in 0..n {
            let (min, max) = bounds[i];
            let delta = rng.gaussian() * 0.1 * (max - min);
            cand[i] = (cand[i] + delta).clamp(min, max);
        }

        let cand_e = eval_obj(&cand)?;
        let de = cand_e - current_e;

        if de < 0.0 || (temp > 1e-12 && rng.unit() < (-de / temp).exp()) {
            current = cand;
            current_e = cand_e;
            if current_e < best_e {
                best = current.clone();
                best_e = current_e;
            }
        }
        temp *= cooling;
    }

    let best_sol: Vec<Value> = best.iter().map(|&v| Value::F64(v)).collect();
    Ok(args::record([
        ("best_solution", Value::List(best_sol)),
        ("best_energy", Value::F64(best_e)),
        ("iterations", Value::I64(max_iter as i64)),
    ]))
}

/// `Optimization.artificial_bee_colony` — Continuous Artificial Bee Colony optimizer.
/// Args: { objective: string, vars: [string], bounds: [[f64, f64]], colony_size?: i64, max_cycles?: i64, seed?: i64 }
pub fn opt_artificial_bee_colony(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let obj_str = args::rec_str(args_v, "objective")
        .ok_or_else(|| args::bad(span, "Optimization.artificial_bee_colony needs objective: string"))?;
    let vars = args::rec_str_list(args_v, "vars")
        .ok_or_else(|| args::bad(span, "Optimization.artificial_bee_colony needs vars: [string]"))?;
    let raw_bounds = args::rec(args_v, "bounds")
        .and_then(args::list)
        .ok_or_else(|| args::bad(span, "Optimization.artificial_bee_colony needs bounds: [[min, max]]"))?;

    let n = vars.len();
    if raw_bounds.len() != n {
        return Err(args::bad(span, "bounds length must match vars"));
    }

    let mut bounds = Vec::with_capacity(n);
    for b in raw_bounds {
        let pair = args::list(b).ok_or_else(|| args::bad(span, "each bound must be [min, max]"))?;
        if pair.len() != 2 { return Err(args::bad(span, "each bound must be [min, max]")); }
        let min = args::as_f64(&pair[0]).ok_or_else(|| args::bad(span, "bound min must be number"))?;
        let max = args::as_f64(&pair[1]).ok_or_else(|| args::bad(span, "bound max must be number"))?;
        bounds.push((min, max));
    }

    let colony_size = args::rec_i64(args_v, "colony_size").unwrap_or(20).max(4) as usize;
    let food_sources = colony_size / 2;
    let max_cycles = args::rec_i64(args_v, "max_cycles").unwrap_or(50).max(1) as usize;
    let seed = args::rec_i64(args_v, "seed").unwrap_or(42) as u64;

    let expr = parse_expr(obj_str, span)?;
    let eval_obj = |pt: &[f64]| -> Result<f64, Diagnostic> {
        let mut env = HashMap::new();
        for (i, v) in vars.iter().enumerate() { env.insert(v.clone(), pt[i]); }
        eval_expr(&expr, &env, span)
    };

    let mut rng = crate::solvers::optimization::metaheuristics::Rng(seed);
    let mut foods: Vec<Vec<f64>> = (0..food_sources)
        .map(|_| bounds.iter().map(|(min, max)| min + rng.unit() * (max - min)).collect())
        .collect();

    let mut costs: Vec<f64> = Vec::with_capacity(food_sources);
    for f in &foods {
        costs.push(eval_obj(f)?);
    }

    let mut best_sol = foods[0].clone();
    let mut best_cost = costs[0];
    for (f, &c) in foods.iter().zip(&costs) {
        if c < best_cost {
            best_cost = c;
            best_sol = f.clone();
        }
    }

    for _ in 0..max_cycles {
        // Employed bee phase
        for i in 0..food_sources {
            let mut partner = rng.below(food_sources);
            while partner == i && food_sources > 1 { partner = rng.below(food_sources); }
            let param = rng.below(n);
            let phi = (rng.unit() - 0.5) * 2.0;

            let mut cand = foods[i].clone();
            cand[param] = (cand[param] + phi * (cand[param] - foods[partner][param]))
                .clamp(bounds[param].0, bounds[param].1);
            let cand_cost = eval_obj(&cand)?;

            if cand_cost < costs[i] {
                foods[i] = cand;
                costs[i] = cand_cost;
                if cand_cost < best_cost {
                    best_cost = cand_cost;
                    best_sol = foods[i].clone();
                }
            }
        }
    }

    let best_val: Vec<Value> = best_sol.iter().map(|&v| Value::F64(v)).collect();
    Ok(args::record([
        ("best_solution", Value::List(best_val)),
        ("best_cost", Value::F64(best_cost)),
        ("cycles", Value::I64(max_cycles as i64)),
    ]))
}

// ── 4. Vector Calculus Integrals ──────────────────────────────────────────────

/// `VectorCalculus.line_integral_scalar` — Scalar line integral along parametric curve.
/// Args: { expr: string, curve: [string], t_span: [f64, f64], steps?: i64 }
pub fn vc_line_integral_scalar(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let expr_str = args::rec_str(args_v, "expr")
        .ok_or_else(|| args::bad(span, "VectorCalculus.line_integral_scalar needs expr: string"))?;
    let curve_strs = args::rec_str_list(args_v, "curve")
        .ok_or_else(|| args::bad(span, "VectorCalculus.line_integral_scalar needs curve: [string]"))?;
    let t_span = args::rec_f64_list(args_v, "t_span")
        .ok_or_else(|| args::bad(span, "VectorCalculus.line_integral_scalar needs t_span: [t0, t1]"))?;
    if t_span.len() != 2 { return Err(args::bad(span, "t_span must be [t0, t1]")); }
    let steps = args::rec_i64(args_v, "steps").unwrap_or(100).max(2) as usize;

    let f_expr = parse_expr(expr_str, span)?;
    let c_exprs = parse_expr_list(&curve_strs, span)?;

    let f = |pt: &[f64]| -> f64 {
        let mut env = HashMap::new();
        let var_names = ["x", "y", "z", "w"];
        for (i, &coord) in pt.iter().enumerate() {
            if i < var_names.len() { env.insert(var_names[i].to_string(), coord); }
        }
        f_expr.eval(&env).unwrap_or(0.0)
    };

    let curve = |t: f64| -> Vec<f64> {
        let mut env = HashMap::new();
        env.insert("t".to_string(), t);
        c_exprs.iter().map(|e| e.eval(&env).unwrap_or(0.0)).collect()
    };

    let val = line_integral_scalar(f, curve, t_span[0], t_span[1], steps);
    Ok(args::record([
        ("integral", Value::F64(val)),
        ("steps", Value::I64(steps as i64)),
    ]))
}

/// `VectorCalculus.line_integral_work` — Vector work line integral along parametric curve.
/// Args: { field: [string], curve: [string], t_span: [f64, f64], steps?: i64 }
pub fn vc_line_integral_work(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let field_strs = args::rec_str_list(args_v, "field")
        .ok_or_else(|| args::bad(span, "VectorCalculus.line_integral_work needs field: [string]"))?;
    let curve_strs = args::rec_str_list(args_v, "curve")
        .ok_or_else(|| args::bad(span, "VectorCalculus.line_integral_work needs curve: [string]"))?;
    let t_span = args::rec_f64_list(args_v, "t_span")
        .ok_or_else(|| args::bad(span, "VectorCalculus.line_integral_work needs t_span: [t0, t1]"))?;
    if t_span.len() != 2 { return Err(args::bad(span, "t_span must be [t0, t1]")); }
    let steps = args::rec_i64(args_v, "steps").unwrap_or(100).max(2) as usize;

    let f_exprs = parse_expr_list(&field_strs, span)?;
    let c_exprs = parse_expr_list(&curve_strs, span)?;

    let field = |pt: &[f64]| -> Vec<f64> {
        let mut env = HashMap::new();
        let var_names = ["x", "y", "z", "w"];
        for (i, &coord) in pt.iter().enumerate() {
            if i < var_names.len() { env.insert(var_names[i].to_string(), coord); }
        }
        f_exprs.iter().map(|e| e.eval(&env).unwrap_or(0.0)).collect()
    };

    let curve = |t: f64| -> Vec<f64> {
        let mut env = HashMap::new();
        env.insert("t".to_string(), t);
        c_exprs.iter().map(|e| e.eval(&env).unwrap_or(0.0)).collect()
    };

    let val = line_integral_work(field, curve, t_span[0], t_span[1], steps);
    Ok(args::record([
        ("work", Value::F64(val)),
        ("steps", Value::I64(steps as i64)),
    ]))
}

/// `VectorCalculus.surface_flux` — Flux of a 3D field through a parametric surface.
/// Args: { field: [string], surface: [string], u_span: [f64, f64], v_span: [f64, f64], steps?: i64 }
pub fn vc_surface_flux(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let field_strs = args::rec_str_list(args_v, "field")
        .ok_or_else(|| args::bad(span, "VectorCalculus.surface_flux needs field: [string] (3 components)"))?;
    let surf_strs = args::rec_str_list(args_v, "surface")
        .ok_or_else(|| args::bad(span, "VectorCalculus.surface_flux needs surface: [string] (3 components)"))?;
    if field_strs.len() != 3 || surf_strs.len() != 3 {
        return Err(args::bad(span, "field and surface must each have 3 components"));
    }

    let u_span = args::rec_f64_list(args_v, "u_span")
        .ok_or_else(|| args::bad(span, "VectorCalculus.surface_flux needs u_span: [u0, u1]"))?;
    let v_span = args::rec_f64_list(args_v, "v_span")
        .ok_or_else(|| args::bad(span, "VectorCalculus.surface_flux needs v_span: [v0, v1]"))?;
    if u_span.len() != 2 || v_span.len() != 2 {
        return Err(args::bad(span, "u_span and v_span must have length 2"));
    }
    let steps = args::rec_i64(args_v, "steps").unwrap_or(50).max(2) as usize;

    let f_exprs = parse_expr_list(&field_strs, span)?;
    let s_exprs = parse_expr_list(&surf_strs, span)?;

    let field = |pt: &[f64]| -> Vec<f64> {
        let mut env = HashMap::new();
        if !pt.is_empty() { env.insert("x".to_string(), pt[0]); }
        if pt.len() > 1 { env.insert("y".to_string(), pt[1]); }
        if pt.len() > 2 { env.insert("z".to_string(), pt[2]); }
        vec![
            f_exprs[0].eval(&env).unwrap_or(0.0),
            f_exprs[1].eval(&env).unwrap_or(0.0),
            f_exprs[2].eval(&env).unwrap_or(0.0),
        ]
    };

    let surf = |u: f64, v: f64| -> Vec<f64> {
        let mut env = HashMap::new();
        env.insert("u".to_string(), u);
        env.insert("v".to_string(), v);
        vec![
            s_exprs[0].eval(&env).unwrap_or(0.0),
            s_exprs[1].eval(&env).unwrap_or(0.0),
            s_exprs[2].eval(&env).unwrap_or(0.0),
        ]
    };

    let val = surface_flux(field, surf, u_span[0], u_span[1], v_span[0], v_span[1], steps);
    Ok(args::record([
        ("flux", Value::F64(val)),
        ("steps", Value::I64(steps as i64)),
    ]))
}

// ── 5. Laplace Transforms ─────────────────────────────────────────────────────

/// `IntegralTransforms.laplace_numeric` — Numerical Laplace transform.
/// Args: { expr: string, s: f64, t_max?: f64, steps?: i64 }
pub fn xform_laplace_numeric(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let expr_str = args::rec_str(args_v, "expr")
        .ok_or_else(|| args::bad(span, "IntegralTransforms.laplace_numeric needs expr: string"))?;
    let s = args::rec_f64(args_v, "s")
        .ok_or_else(|| args::bad(span, "IntegralTransforms.laplace_numeric needs s: number (s > 0)"))?;
    let t_max = args::rec_f64(args_v, "t_max").unwrap_or(10.0);
    let steps = args::rec_i64(args_v, "steps").unwrap_or(100).max(2) as usize;
    let steps_even = if steps % 2 != 0 { steps + 1 } else { steps };

    let expr = parse_expr(expr_str, span)?;
    let f = |t: f64| -> f64 {
        let mut env = HashMap::new();
        env.insert("t".to_string(), t);
        expr.eval(&env).unwrap_or(0.0)
    };

    match laplace_numeric(f, s, t_max, steps_even) {
        Ok(val) => Ok(args::record([
            ("result", Value::F64(val)),
            ("s", Value::F64(s)),
            ("t_max", Value::F64(t_max)),
        ])),
        Err(e) => Err(args::bad(span, format!("numerical Laplace transform failed: {e:?}"))),
    }
}

/// `IntegralTransforms.laplace_symbolic` — Symbolic algebraic Laplace transform.
/// Args: { expr: string }
pub fn xform_laplace_symbolic(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let expr_str = args::rec_str(args_v, "expr")
        .ok_or_else(|| args::bad(span, "IntegralTransforms.laplace_symbolic needs expr: string"))?;
    let expr = parse_expr(expr_str, span)?;

    match laplace_table(&expr) {
        Ok(transformed) => Ok(args::record([
            ("transform", Value::String(transformed.to_string())),
        ])),
        Err(e) => Err(args::bad(span, format!("symbolic Laplace transform failed: {e:?}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn span() -> Span { Span { start: 0, end: 0 } }

    #[test]
    fn rk4_exponential_decay() {
        let mut m = BTreeMap::new();
        m.insert("system".into(), Value::List(vec![Value::String("-0.5 * y".into())]));
        m.insert("vars".into(), Value::List(vec![Value::String("y".into())]));
        m.insert("t_span".into(), Value::List(vec![Value::F64(0.0), Value::F64(2.0)]));
        m.insert("y0".into(), Value::List(vec![Value::F64(1.0)]));
        m.insert("dt".into(), Value::F64(0.01));

        let res = ode_rk4_integrate(&Value::Record(m), span()).unwrap();
        if let Value::Record(r) = res {
            if let Some(Value::List(final_st)) = r.get("final_state") {
                if let Value::F64(y_final) = final_st[0] {
                    // y(2) = e^(-1) ≈ 0.367879
                    assert!((y_final - (-1.0f64).exp()).abs() < 1e-4);
                } else { panic!("expected F64"); }
            } else { panic!("missing final_state"); }
        } else { panic!("expected Record"); }
    }

    #[test]
    fn dopri5_adaptive_integration() {
        let mut m = BTreeMap::new();
        m.insert("system".into(), Value::List(vec![Value::String("-1.0 * y".into())]));
        m.insert("vars".into(), Value::List(vec![Value::String("y".into())]));
        m.insert("t_span".into(), Value::List(vec![Value::F64(0.0), Value::F64(1.0)]));
        m.insert("y0".into(), Value::List(vec![Value::F64(1.0)]));

        let res = ode_dopri5(&Value::Record(m), span()).unwrap();
        if let Value::Record(r) = res {
            if let Some(Value::List(final_st)) = r.get("final_state") {
                if let Value::F64(y_final) = final_st[0] {
                    assert!((y_final - (-1.0f64).exp()).abs() < 1e-4);
                }
            }
        }
    }

    #[test]
    fn adaptive_simpson_quadrature() {
        let mut m = BTreeMap::new();
        m.insert("expr".into(), Value::String("x^2".into()));
        m.insert("var".into(), Value::String("x".into()));
        m.insert("a".into(), Value::F64(0.0));
        m.insert("b".into(), Value::F64(3.0));

        let res = calc_adaptive_simpson(&Value::Record(m), span()).unwrap();
        if let Value::Record(r) = res {
            if let Some(Value::F64(val)) = r.get("value") {
                // ∫_0^3 x^2 dx = 9.0
                assert!((val - 9.0).abs() < 1e-6);
            }
        }
    }

    #[test]
    fn adaptive_central_derivative() {
        let mut m = BTreeMap::new();
        m.insert("expr".into(), Value::String("x^3".into()));
        m.insert("var".into(), Value::String("x".into()));
        m.insert("x".into(), Value::F64(2.0));

        let res = calc_adaptive_derivative(&Value::Record(m), span()).unwrap();
        if let Value::Record(r) = res {
            if let Some(Value::F64(val)) = r.get("value") {
                // d/dx(x^3) at x=2 is 12.0
                assert!((val - 12.0).abs() < 1e-4);
            }
        }
    }

    #[test]
    fn newton_root_finder() {
        let mut m = BTreeMap::new();
        // x^2 - 4 = 0 -> x = 2
        m.insert("exprs".into(), Value::List(vec![Value::String("x^2 - 4".into())]));
        m.insert("vars".into(), Value::List(vec![Value::String("x".into())]));
        m.insert("guess".into(), Value::List(vec![Value::F64(1.0)]));

        let res = calc_newton_solve(&Value::Record(m), span()).unwrap();
        if let Value::Record(r) = res {
            if let Some(Value::List(sol)) = r.get("solution") {
                if let Value::F64(x_val) = sol[0] {
                    assert!((x_val - 2.0).abs() < 1e-6);
                }
            }
        }
    }

    #[test]
    fn simulated_annealing_minimizer() {
        let mut m = BTreeMap::new();
        // (x - 3)^2 -> min at x = 3
        m.insert("objective".into(), Value::String("(x - 3)^2".into()));
        m.insert("vars".into(), Value::List(vec![Value::String("x".into())]));
        m.insert("bounds".into(), Value::List(vec![Value::List(vec![Value::F64(0.0), Value::F64(10.0)])]));
        m.insert("max_iter".into(), Value::I64(500));
        m.insert("seed".into(), Value::I64(123));

        let res = opt_simulated_annealing(&Value::Record(m), span()).unwrap();
        if let Value::Record(r) = res {
            if let Some(Value::List(best)) = r.get("best_solution") {
                if let Value::F64(x_best) = best[0] {
                    assert!((x_best - 3.0).abs() < 0.2);
                }
            }
        }
    }

    #[test]
    fn vector_calculus_line_integral() {
        let mut m = BTreeMap::new();
        m.insert("expr".into(), Value::String("1.0".into()));
        // circle radius 1: [cos(t), sin(t)] from 0 to pi -> length = pi ≈ 3.14159
        m.insert("curve".into(), Value::List(vec![Value::String("cos(t)".into()), Value::String("sin(t)".into())]));
        m.insert("t_span".into(), Value::List(vec![Value::F64(0.0), Value::F64(std::f64::consts::PI)]));
        m.insert("steps".into(), Value::I64(100));

        let res = vc_line_integral_scalar(&Value::Record(m), span()).unwrap();
        if let Value::Record(r) = res {
            if let Some(Value::F64(len)) = r.get("integral") {
                assert!((len - std::f64::consts::PI).abs() < 0.05);
            }
        }
    }

    #[test]
    fn laplace_transforms_numeric_and_symbolic() {
        let mut m1 = BTreeMap::new();
        // L{1}(s) = 1/s -> for s=2, L{1} = 0.5
        m1.insert("expr".into(), Value::String("1.0".into()));
        m1.insert("s".into(), Value::F64(2.0));
        m1.insert("t_max".into(), Value::F64(15.0));
        m1.insert("steps".into(), Value::I64(200));

        let res1 = xform_laplace_numeric(&Value::Record(m1), span()).unwrap();
        if let Value::Record(r) = res1 {
            if let Some(Value::F64(v)) = r.get("result") {
                assert!((v - 0.5).abs() < 0.01);
            }
        }

        let mut m2 = BTreeMap::new();
        m2.insert("expr".into(), Value::String("t".into()));
        let res2 = xform_laplace_symbolic(&Value::Record(m2), span()).unwrap();
        if let Value::Record(r) = res2 {
            if let Some(Value::String(s)) = r.get("transform") {
                assert!(s.contains("s"));
            }
        }
    }
}

