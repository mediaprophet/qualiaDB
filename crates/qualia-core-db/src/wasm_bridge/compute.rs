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
    let p: PidStepParams =
        serde_wasm_bindgen::from_value(val).map_err(|e| JsValue::from_str(&e.to_string()))?;

    let error = p.setpoint - p.current_value;
    let derivative = if p.dt > 0.0 {
        (error - p.prev_error) / p.dt
    } else {
        0.0
    };
    let new_integral = p.integral + error * p.dt;
    let output = p.kp * error + p.ki * new_integral + p.kd * derivative;

    #[derive(Serialize)]
    struct PidOut {
        output: f64,
        new_error: f64,
        new_integral: f64,
    }
    Ok(serde_wasm_bindgen::to_value(&PidOut {
        output,
        new_error: error,
        new_integral,
    })?)
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
    let p: GbmPathParams =
        serde_wasm_bindgen::from_value(val).map_err(|e| JsValue::from_str(&e.to_string()))?;
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
        price *= f64::exp(
            (p.drift - 0.5 * p.volatility * p.volatility) * dt + p.volatility * f64::sqrt(dt) * z,
        );
        path.push(price);
        if price < min_price {
            min_price = price;
        }
        if price > max_price {
            max_price = price;
        }
    }
    #[derive(Serialize)]
    struct GbmOut {
        final_price: f64,
        min_price: f64,
        max_price: f64,
        path: Vec<f64>,
    }
    Ok(serde_wasm_bindgen::to_value(&GbmOut {
        final_price: price,
        min_price,
        max_price,
        path,
    })?)
}

/// Black-Scholes European option pricing with full Greeks.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn black_scholes_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    let p: BlackScholesParams =
        serde_wasm_bindgen::from_value(val).map_err(|e| JsValue::from_str(&e.to_string()))?;
    if p.vol <= 0.0 || p.time_years <= 0.0 || p.spot <= 0.0 || p.strike <= 0.0 {
        return Err(JsValue::from_str(
            "spot, strike, vol, time_years must be positive",
        ));
    }
    let sqrt_t = p.time_years.sqrt();
    let d1 = (f64::ln(p.spot / p.strike) + (p.rate + 0.5 * p.vol * p.vol) * p.time_years)
        / (p.vol * sqrt_t);
    let d2 = d1 - p.vol * sqrt_t;
    let disc = f64::exp(-p.rate * p.time_years);
    let (price, delta) = if p.is_call {
        (
            p.spot * phi_norm(d1) - p.strike * disc * phi_norm(d2),
            phi_norm(d1),
        )
    } else {
        (
            p.strike * disc * phi_norm(-d2) - p.spot * phi_norm(-d1),
            phi_norm(d1) - 1.0,
        )
    };
    let nd1 = f64::exp(-0.5 * d1 * d1) / f64::sqrt(2.0 * std::f64::consts::PI);
    let gamma = nd1 / (p.spot * p.vol * sqrt_t);
    let vega = p.spot * nd1 * sqrt_t / 100.0;
    let theta = if p.is_call {
        (-(p.spot * nd1 * p.vol) / (2.0 * sqrt_t) - p.rate * p.strike * disc * phi_norm(d2)) / 365.0
    } else {
        (-(p.spot * nd1 * p.vol) / (2.0 * sqrt_t) + p.rate * p.strike * disc * phi_norm(-d2))
            / 365.0
    };
    let rho = if p.is_call {
        p.strike * p.time_years * disc * phi_norm(d2) / 100.0
    } else {
        -p.strike * p.time_years * disc * phi_norm(-d2) / 100.0
    };
    #[derive(Serialize)]
    struct BsOut {
        price: f64,
        delta: f64,
        gamma: f64,
        vega: f64,
        theta: f64,
        rho: f64,
    }
    Ok(serde_wasm_bindgen::to_value(&BsOut {
        price,
        delta,
        gamma,
        vega,
        theta,
        rho,
    })?)
}

// ─── SAT Solver ──────────────────────────────────────────────────────────────

/// Bounded DPLL SAT solver.
/// Input: `{ clauses: [[1, 2, -3], [-1, 3], ...] }` (signed literal convention).
/// Output: `{ satisfiable: bool, assignment: { "1": true, "2": false, ... } }`
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn solve_sat_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    use crate::solvers::symbolic_logic::{BoundedSatSolver, Clause, Literal};
    use crate::solvers::SolverConfig;
    use std::collections::HashMap;

    // Deserialize input: { clauses: Vec<Vec<i32>> }
    #[derive(Deserialize)]
    struct SatInput {
        clauses: Vec<Vec<i32>>,
    }
    let input: SatInput =
        serde_wasm_bindgen::from_value(val).map_err(|e| JsValue::from_str(&e.to_string()))?;

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
        solver
            .add_clause(clause)
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;
    }

    let state = match solver.solve() {
        Ok(s) => s,
        Err(crate::solvers::SolversError::Unsatisfiable) => {
            #[derive(Serialize)]
            struct SatOut {
                satisfiable: bool,
                assignment: HashMap<String, bool>,
            }
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
            AssignmentValue::True => Some(true),
            AssignmentValue::False => Some(false),
            AssignmentValue::Unassigned => None,
        };
        if let Some(v) = val_bool {
            assignment.insert(format!("{}", i + 1), v);
        }
    }

    #[derive(Serialize)]
    struct SatOut {
        satisfiable: bool,
        assignment: HashMap<String, bool>,
    }
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
    let p: OdeDecayParams =
        serde_wasm_bindgen::from_value(val).map_err(|e| JsValue::from_str(&e.to_string()))?;

    if p.k <= 0.0 {
        return Err(JsValue::from_str("k must be positive"));
    }
    if p.dt <= 0.0 {
        return Err(JsValue::from_str("dt must be positive"));
    }

    // RK4 step for dy/dt = -k*y
    let rk4_step = |t: f64, y: f64, h: f64| -> f64 {
        let _ = t; // autonomous ODE — t unused
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
    struct OdeOut {
        t_values: Vec<f64>,
        y_values: Vec<f64>,
        final_y: f64,
    }
    Ok(serde_wasm_bindgen::to_value(&OdeOut {
        t_values,
        y_values,
        final_y: y,
    })?)
}

// ─── Computational Economics & Distributional Metrics with Receipts (QW-07) ─

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct WelfareParams {
    pub incomes: Vec<f64>,
    pub epsilon: Option<f64>,
}

#[cfg(target_arch = "wasm32")]
#[derive(Serialize)]
pub struct CalculationReceipt {
    pub engine_version: &'static str,
    pub algorithm: String,
    pub sample_size: usize,
    pub seed: Option<u64>,
    pub converged: bool,
    pub tolerances: Option<f64>,
    pub warnings: Vec<String>,
    pub receipt_hash: String,
}

/// Evaluates distributional and welfare metrics (Gini, Atkinson index, Palma ratio,
/// mean, median, P10, P90) and emits an auditable `CalculationReceipt`.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn calculate_welfare_metrics_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    let p: WelfareParams = serde_wasm_bindgen::from_value(val)
        .map_err(|e| JsValue::from_str(&format!("invalid welfare params: {e}")))?;
    if p.incomes.is_empty() {
        return Err(JsValue::from_str("incomes must be non-empty"));
    }

    let gini =
        crate::specialized_libs::computational_economics::welfare::gini_coefficient(&p.incomes)
            .map_err(|e| JsValue::from_str(&format!("gini calculation error: {e:?}")))?;

    let eps = p.epsilon.unwrap_or(0.5);
    let atkinson = crate::specialized_libs::computational_economics::welfare::atkinson_inequality(
        &p.incomes, eps,
    )
    .ok();

    let mut sorted = p.incomes.clone();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let n = sorted.len();
    let sum: f64 = sorted.iter().sum();
    let mean = sum / (n as f64);
    let median = if n % 2 == 1 {
        sorted[n / 2]
    } else {
        0.5 * (sorted[n / 2 - 1] + sorted[n / 2])
    };
    let p10 = sorted[((n as f64) * 0.10).floor() as usize];
    let p90 = sorted[(((n as f64) * 0.90).floor() as usize).min(n - 1)];

    let bottom_40_count = ((n as f64) * 0.40).ceil() as usize;
    let top_10_start = ((n as f64) * 0.90).floor() as usize;
    let bottom_40_sum: f64 = sorted[..bottom_40_count].iter().sum();
    let top_10_sum: f64 = sorted[top_10_start..].iter().sum();
    let palma_ratio = if bottom_40_sum > 0.0 {
        Some(top_10_sum / bottom_40_sum)
    } else {
        None
    };

    let receipt_hash = format!(
        "{:016x}",
        crate::q_hash(&format!("welfare:{n}:{gini}:{mean}"))
    );

    #[derive(Serialize)]
    struct WelfareOut {
        gini: f64,
        atkinson: Option<f64>,
        mean: f64,
        median: f64,
        p10: f64,
        p90: f64,
        palma_ratio: Option<f64>,
        receipt: CalculationReceipt,
    }

    Ok(serde_wasm_bindgen::to_value(&WelfareOut {
        gini,
        atkinson,
        mean,
        median,
        p10,
        p90,
        palma_ratio,
        receipt: CalculationReceipt {
            engine_version: "0.0.39",
            algorithm: "Sen-Gini / Atkinson-CES".to_string(),
            sample_size: n,
            seed: None,
            converged: true,
            tolerances: Some(1e-9),
            warnings: Vec::new(),
            receipt_hash,
        },
    })?)
}

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct LeontiefParams {
    pub technical_matrix: Vec<f64>,
    pub sectors: usize,
    pub final_demand: Vec<f64>,
}

/// Evaluates input-output multipliers and total requirements via the Leontief inverse (I - A)^(-1).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn calculate_leontief_multipliers_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    let p: LeontiefParams = serde_wasm_bindgen::from_value(val)
        .map_err(|e| JsValue::from_str(&format!("invalid Leontief params: {e}")))?;
    let n = p.sectors;
    if n == 0 || n > 32 || p.technical_matrix.len() < n * n || p.final_demand.len() < n {
        return Err(JsValue::from_str(
            "invalid dimensions for Leontief input-output (max 32 sectors)",
        ));
    }

    let mut inv = vec![0.0f64; n * n];
    crate::specialized_libs::computational_economics::input_output::leontief_inverse_into(
        &p.technical_matrix[..n * n],
        n,
        1000,
        1e-12,
        &mut inv,
    )
    .map_err(|e| JsValue::from_str(&format!("Leontief inverse error: {e:?}")))?;

    let mut total_output = vec![0.0f64; n];
    for i in 0..n {
        let mut row_sum = 0.0f64;
        for j in 0..n {
            row_sum += inv[i * n + j] * p.final_demand[j];
        }
        total_output[i] = row_sum;
    }

    let mut output_multipliers = vec![0.0f64; n];
    for j in 0..n {
        let mut col_sum = 0.0f64;
        for i in 0..n {
            col_sum += inv[i * n + j];
        }
        output_multipliers[j] = col_sum;
    }

    let receipt_hash = format!(
        "{:016x}",
        crate::q_hash(&format!("leontief:{n}:{}", total_output[0]))
    );

    #[derive(Serialize)]
    struct LeontiefOut {
        total_output: Vec<f64>,
        output_multipliers: Vec<f64>,
        leontief_inverse: Vec<f64>,
        receipt: CalculationReceipt,
    }

    Ok(serde_wasm_bindgen::to_value(&LeontiefOut {
        total_output,
        output_multipliers,
        leontief_inverse: inv,
        receipt: CalculationReceipt {
            engine_version: "0.0.39",
            algorithm: "Leontief Neumann Series / Direct Inversion".to_string(),
            sample_size: n,
            seed: None,
            converged: true,
            tolerances: Some(1e-10),
            warnings: Vec::new(),
            receipt_hash,
        },
    })?)
}

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct OlsParams {
    pub x: Vec<f64>,
    pub y: Vec<f64>,
}

/// Evaluates ordinary least squares regression with complete diagnostics and receipt.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn compute_ols_diagnostics_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    let p: OlsParams = serde_wasm_bindgen::from_value(val)
        .map_err(|e| JsValue::from_str(&format!("invalid OLS params: {e}")))?;
    if p.x.len() != p.y.len() {
        return Err(JsValue::from_str("x and y must have equal length"));
    }
    if p.x.len() < 3 {
        return Err(JsValue::from_str("OLS requires at least 3 observations"));
    }

    let r = crate::solvers::statistics::regression::simple_linear_regression(&p.x, &p.y)
        .ok_or_else(|| {
            JsValue::from_str("OLS regression error: non-finite or zero-variance predictor")
        })?;

    let receipt_hash = format!(
        "{:016x}",
        crate::q_hash(&format!("ols:{}:{}:{}", r.n, r.slope, r.r_squared))
    );

    #[derive(Serialize)]
    struct OlsOut {
        slope: f64,
        intercept: f64,
        r_squared: f64,
        residual_std_error: f64,
        slope_std_error: f64,
        slope_t: f64,
        slope_p_value: f64,
        intercept_std_error: f64,
        intercept_p_value: f64,
        n: usize,
        receipt: CalculationReceipt,
    }

    Ok(serde_wasm_bindgen::to_value(&OlsOut {
        slope: r.slope,
        intercept: r.intercept,
        r_squared: r.r_squared,
        residual_std_error: r.residual_std_error,
        slope_std_error: r.slope_std_error,
        slope_t: r.slope_t,
        slope_p_value: r.slope_p_value,
        intercept_std_error: r.intercept_std_error,
        intercept_p_value: r.intercept_p_value,
        n: r.n,
        receipt: CalculationReceipt {
            engine_version: "0.0.39",
            algorithm: "Ordinary Least Squares (Bessel-Corrected)".to_string(),
            sample_size: r.n,
            seed: None,
            converged: true,
            tolerances: Some(1e-12),
            warnings: Vec::new(),
            receipt_hash,
        },
    })?)
}

/// Multiple OLS + verification report with Civics calculation receipt.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn verify_regression_model_receipt_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        x: Vec<Vec<f64>>,
        y: Vec<f64>,
        #[serde(default = "default_alpha")]
        alpha: f64,
        #[serde(default)]
        strict: bool,
    }
    fn default_alpha() -> f64 {
        0.05
    }
    let p: In = serde_wasm_bindgen::from_value(val)
        .map_err(|e| JsValue::from_str(&format!("invalid verify params: {e}")))?;
    if p.x.is_empty() {
        return Err(JsValue::from_str("x must be non-empty"));
    }
    let n = p.x.len();
    let k = p.x[0].len();
    let mut flat = Vec::with_capacity(n * k);
    for row in &p.x {
        if row.len() != k {
            return Err(JsValue::from_str("x must be rectangular"));
        }
        flat.extend_from_slice(row);
    }
    if p.y.len() != n {
        return Err(JsValue::from_str("y length must equal x rows"));
    }
    let m = crate::solvers::statistics::regression::multiple_ols(&flat, &p.y, n, k, true)
        .ok_or_else(|| JsValue::from_str("OLS failed"))?;
    let opts = crate::solvers::statistics::regression::VerifyOptions {
        alpha: p.alpha,
        strict: p.strict,
        ..Default::default()
    };
    let report = crate::solvers::statistics::regression::verify_regression_model(
        &m,
        Some((&flat, k)),
        Some(&p.y),
        &opts,
    );
    let mut warnings: Vec<String> = report
        .flags
        .iter()
        .map(|f| format!("{}: {}", f.code, f.message))
        .collect();
    if !report.ok {
        warnings.insert(0, "verification_failed".into());
    }
    let receipt_hash = format!(
        "{:016x}",
        crate::q_hash(&format!(
            "regverify:{}:{}:{:.6}:{}",
            m.n,
            m.r_squared,
            report.ok,
            warnings.len()
        ))
    );
    #[derive(Serialize)]
    struct FlagOut {
        code: String,
        message: String,
    }
    #[derive(Serialize)]
    struct Out {
        ok: bool,
        flags: Vec<FlagOut>,
        coefficients: Vec<f64>,
        residuals: Vec<f64>,
        fitted: Vec<f64>,
        r_squared: f64,
        f_p_value: Option<f64>,
        jarque_bera_p: Option<f64>,
        breusch_pagan_p: Option<f64>,
        durbin_watson: Option<f64>,
        max_vif: Option<f64>,
        ramsey_reset_p: Option<f64>,
        spurious_warn: bool,
        n: usize,
        receipt: CalculationReceipt,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        ok: report.ok,
        flags: report
            .flags
            .into_iter()
            .map(|f| FlagOut {
                code: f.code,
                message: f.message,
            })
            .collect(),
        coefficients: m.coefficients,
        residuals: m.residuals,
        fitted: m.fitted,
        r_squared: m.r_squared,
        f_p_value: report.f_p_value,
        jarque_bera_p: report.jarque_bera_p,
        breusch_pagan_p: report.breusch_pagan_p,
        durbin_watson: report.durbin_watson,
        max_vif: report.max_vif,
        ramsey_reset_p: report.ramsey_reset_p,
        spurious_warn: report.spurious_warn,
        n: m.n,
        receipt: CalculationReceipt {
            engine_version: "0.0.39",
            algorithm: "Multiple OLS + Ch.4 Verification Battery".to_string(),
            sample_size: m.n,
            seed: None,
            converged: report.ok,
            tolerances: Some(p.alpha),
            warnings,
            receipt_hash,
        },
    })?)
}
