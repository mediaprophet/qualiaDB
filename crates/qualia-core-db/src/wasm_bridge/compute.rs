//! WASM-bindgen API — compute domain (split from wasm_bridge.rs; verbatim, no behaviour change).
//! WASM-bindgen API surface — exposes Qualia engine functions to JavaScript.
//!
//! All functions are `#[cfg(target_arch = "wasm32")]` and only compiled into
//! the browser/OPFS build.  Native desktop builds use direct Rust FFI.

#[cfg(target_arch = "wasm32")]
use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

// ─── Economics: Monte Carlo VaR ──────────────────────────────────────────────
#[cfg(target_arch = "wasm32")]
use super::*;


#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn run_semantic_simulation(val: JsValue) -> Result<JsValue, JsValue> {
    let params: SimulationParams = serde_wasm_bindgen::from_value(val)?;
    let (mean, value_at_risk) = crate::domains::financial::economics::run_monte_carlo_var(
        params.initial_price,
        params.drift,
        params.volatility,
        params.time_horizon as f64,
        params.simulation_steps as usize,
        252,
    );
    #[derive(Serialize)]
    struct SimResult {
        mean: f64,
        value_at_risk: f64,
    }
    Ok(serde_wasm_bindgen::to_value(&SimResult {
        mean,
        value_at_risk,
    })?)
}

// ─── Bioinformatics: sequence alignment ──────────────────────────────────────

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct AlignmentParams {
    pub query: String,
    pub target: String,
    /// "nucleotide" or "protein"
    pub mode: String,
}

/// Stateless PID controller step.
/// Returns { output, new_error, new_integral } for chaining into the next step.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn compute_pid_step_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    let p: PidStepParams = serde_wasm_bindgen::from_value(val)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    let error = p.setpoint - p.current_value;
    let derivative = if p.dt > 0.0 { (error - p.prev_error) / p.dt } else { 0.0 };
    let new_integral = p.integral + error * p.dt;
    let output = p.kp * error + p.ki * new_integral + p.kd * derivative;

    #[derive(Serialize)]
    struct PidOut { output: f64, new_error: f64, new_integral: f64 }
    Ok(serde_wasm_bindgen::to_value(&PidOut { output, new_error: error, new_integral })?)
}

// ─── GBM Path ────────────────────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct GbmPathParams {
    pub initial_price: f64,
    pub drift: f64,
    pub volatility: f64,
    pub time_horizon: f64,
    pub steps: usize,
}

/// Simulates a GBM price path and returns the full series together with
/// min_price, max_price, and final_price.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn simulate_gbm_path_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    use rand_distr::{Distribution, StandardNormal};
    let p: GbmPathParams = serde_wasm_bindgen::from_value(val)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let steps = p.steps.min(252);
    let dt = p.time_horizon / steps as f64;
    let mut price = p.initial_price;
    let mut rng = rand::rng();
    let mut path = Vec::with_capacity(steps + 1);
    path.push(p.initial_price);
    let mut min_price = p.initial_price;
    let mut max_price = p.initial_price;
    for _ in 0..steps {
        let z: f64 = StandardNormal.sample(&mut rng);
        price *= f64::exp((p.drift - 0.5 * p.volatility * p.volatility) * dt
                          + p.volatility * f64::sqrt(dt) * z);
        path.push(price);
        if price < min_price { min_price = price; }
        if price > max_price { max_price = price; }
    }
    #[derive(Serialize)]
    struct GbmOut { final_price: f64, min_price: f64, max_price: f64, path: Vec<f64> }
    Ok(serde_wasm_bindgen::to_value(&GbmOut {
        final_price: price, min_price, max_price, path,
    })?)
}

/// Black-Scholes European option pricing with full Greeks.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn black_scholes_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    let p: BlackScholesParams = serde_wasm_bindgen::from_value(val)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    if p.vol <= 0.0 || p.time_years <= 0.0 || p.spot <= 0.0 || p.strike <= 0.0 {
        return Err(JsValue::from_str("spot, strike, vol, time_years must be positive"));
    }
    let sqrt_t = p.time_years.sqrt();
    let d1 = (f64::ln(p.spot / p.strike) + (p.rate + 0.5 * p.vol * p.vol) * p.time_years)
             / (p.vol * sqrt_t);
    let d2 = d1 - p.vol * sqrt_t;
    let disc = f64::exp(-p.rate * p.time_years);
    let (price, delta) = if p.is_call {
        (p.spot * phi_norm(d1) - p.strike * disc * phi_norm(d2), phi_norm(d1))
    } else {
        (p.strike * disc * phi_norm(-d2) - p.spot * phi_norm(-d1), phi_norm(d1) - 1.0)
    };
    let nd1 = f64::exp(-0.5 * d1 * d1) / f64::sqrt(2.0 * std::f64::consts::PI);
    let gamma = nd1 / (p.spot * p.vol * sqrt_t);
    let vega  = p.spot * nd1 * sqrt_t / 100.0;
    let theta = if p.is_call {
        (-(p.spot * nd1 * p.vol) / (2.0 * sqrt_t) - p.rate * p.strike * disc * phi_norm(d2)) / 365.0
    } else {
        (-(p.spot * nd1 * p.vol) / (2.0 * sqrt_t) + p.rate * p.strike * disc * phi_norm(-d2)) / 365.0
    };
    let rho = if p.is_call {
        p.strike * p.time_years * disc * phi_norm(d2) / 100.0
    } else {
        -p.strike * p.time_years * disc * phi_norm(-d2) / 100.0
    };
    #[derive(Serialize)]
    struct BsOut { price: f64, delta: f64, gamma: f64, vega: f64, theta: f64, rho: f64 }
    Ok(serde_wasm_bindgen::to_value(&BsOut { price, delta, gamma, vega, theta, rho })?)
}

// ─── SAT Solver ──────────────────────────────────────────────────────────────

/// Bounded DPLL SAT solver.
/// Input: `{ clauses: [[1, 2, -3], [-1, 3], ...] }` (signed literal convention).
/// Output: `{ satisfiable: bool, assignment: { "1": true, "2": false, ... } }`
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn solve_sat_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    use crate::solvers::symbolic_logic::{
        BoundedSatSolver, Clause, Literal,
    };
    use crate::solvers::SolverConfig;
    use std::collections::HashMap;

    // Deserialize input: { clauses: Vec<Vec<i32>> }
    #[derive(Deserialize)]
    struct SatInput { clauses: Vec<Vec<i32>> }
    let input: SatInput = serde_wasm_bindgen::from_value(val)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    let mut solver = BoundedSatSolver::new(SolverConfig::default());

    for (clause_id, raw_clause) in input.clauses.iter().enumerate() {
        let mut clause = Clause::default();
        clause.id = (clause_id as u32) + 1;
        clause.num_literals = raw_clause.len().min(5) as u8;
        for (i, &lit) in raw_clause.iter().take(5).enumerate() {
            clause.literals[i] = Literal {
                variable: (lit.unsigned_abs() as u8).saturating_sub(1),
                negated: lit < 0,
            };
        }
        solver.add_clause(clause).map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;
    }

    let state = match solver.solve() {
        Ok(s) => s,
        Err(crate::solvers::SolversError::Unsatisfiable) => {
            #[derive(Serialize)]
            struct SatOut { satisfiable: bool, assignment: HashMap<String, bool> }
            return Ok(serde_wasm_bindgen::to_value(&SatOut {
                satisfiable: false,
                assignment: HashMap::new(),
            })?);
        }
        Err(e) => return Err(JsValue::from_str(&format!("{:?}", e))),
    };

    // Collect variable assignments (variable 0 = JS literal 1)
    let mut assignment = HashMap::new();
    for (i, a) in solver.assignments.iter().enumerate() {
        use crate::solvers::symbolic_logic::AssignmentValue;
        let val_bool = match a.value {
            AssignmentValue::True  => Some(true),
            AssignmentValue::False => Some(false),
            AssignmentValue::Unassigned => None,
        };
        if let Some(v) = val_bool {
            assignment.insert(format!("{}", i + 1), v);
        }
    }

    #[derive(Serialize)]
    struct SatOut { satisfiable: bool, assignment: HashMap<String, bool> }
    Ok(serde_wasm_bindgen::to_value(&SatOut {
        satisfiable: state.satisfiable.unwrap_or(false),
        assignment,
    })?)
}

// ─── RK4 ODE: exponential decay ──────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct OdeDecayParams {
    pub k: f64,
    pub y0: f64,
    pub t0: f64,
    pub t_final: f64,
    pub dt: f64,
}

/// Solves dy/dt = -k·y via classical RK4, returning t_values, y_values, and final_y.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn solve_ode_exponential_decay_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    let p: OdeDecayParams = serde_wasm_bindgen::from_value(val)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    if p.k <= 0.0 { return Err(JsValue::from_str("k must be positive")); }
    if p.dt <= 0.0 { return Err(JsValue::from_str("dt must be positive")); }

    // RK4 step for dy/dt = -k*y
    let rk4_step = |t: f64, y: f64, h: f64| -> f64 {
        let _ = t;  // autonomous ODE — t unused
        let f = |yy: f64| -p.k * yy;
        let k1 = f(y);
        let k2 = f(y + 0.5 * h * k1);
        let k3 = f(y + 0.5 * h * k2);
        let k4 = f(y + h * k3);
        y + (h / 6.0) * (k1 + 2.0 * k2 + 2.0 * k3 + k4)
    };

    let max_steps = 10_000usize;
    let mut t_values = Vec::new();
    let mut y_values = Vec::new();
    let mut t = p.t0;
    let mut y = p.y0;
    t_values.push(t);
    y_values.push(y);

    let mut steps = 0;
    while t < p.t_final && steps < max_steps {
        let h = f64::min(p.dt, p.t_final - t);
        y = rk4_step(t, y, h);
        t += h;
        t_values.push(t);
        y_values.push(y);
        steps += 1;
    }

    #[derive(Serialize)]
    struct OdeOut { t_values: Vec<f64>, y_values: Vec<f64>, final_y: f64 }
    Ok(serde_wasm_bindgen::to_value(&OdeOut { t_values, y_values, final_y: y })?)
}
