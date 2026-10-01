//! Regression verification WASM exports (multiple OLS, diagnostics, influence,
//! selection, design, discrete Y, spurious guards). Sibling of `stats.rs` so that
//! file stays reviewable.
#![cfg(target_arch = "wasm32")]

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use super::jserr;

fn flat_x(rows: &[Vec<f64>]) -> Result<(Vec<f64>, usize, usize), JsValue> {
    if rows.is_empty() {
        return Err(JsValue::from_str("x must be a non-empty matrix"));
    }
    let n = rows.len();
    let p = rows[0].len();
    if p == 0 {
        return Err(JsValue::from_str("x must have at least one column"));
    }
    let mut flat = Vec::with_capacity(n * p);
    for r in rows {
        if r.len() != p {
            return Err(JsValue::from_str("x rows must be rectangular"));
        }
        flat.extend_from_slice(r);
    }
    Ok((flat, n, p))
}

/// Multiple OLS. Input `{ x:number[][], y:number[], fit_intercept?:bool }` →
/// coefficients, SE, t, p, R², adj-R², F, residuals, fitted, n, k.
#[wasm_bindgen]
pub fn ols_multiple_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        x: Vec<Vec<f64>>,
        y: Vec<f64>,
        #[serde(default = "default_true")]
        fit_intercept: bool,
    }
    fn default_true() -> bool {
        true
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let (x, n, k) = flat_x(&p.x)?;
    if p.y.len() != n {
        return Err(JsValue::from_str("y length must equal x rows"));
    }
    let m = crate::solvers::statistics::regression::multiple_ols(&x, &p.y, n, k, p.fit_intercept)
        .ok_or_else(|| JsValue::from_str("OLS failed (singular or insufficient data)"))?;
    #[derive(Serialize)]
    struct Out {
        coefficients: Vec<f64>,
        std_errors: Vec<f64>,
        t_values: Vec<f64>,
        p_values: Vec<f64>,
        r_squared: f64,
        adj_r_squared: f64,
        f_statistic: Option<f64>,
        f_p_value: Option<f64>,
        residual_std_error: f64,
        residuals: Vec<f64>,
        fitted: Vec<f64>,
        n: usize,
        k_predictors: usize,
        df_residual: usize,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        coefficients: m.coefficients,
        std_errors: m.std_errors,
        t_values: m.t_values,
        p_values: m.p_values,
        r_squared: m.r_squared,
        adj_r_squared: m.adj_r_squared,
        f_statistic: m.f_statistic,
        f_p_value: m.f_p_value,
        residual_std_error: m.residual_std_error,
        residuals: m.residuals,
        fitted: m.fitted,
        n: m.n,
        k_predictors: m.k_predictors,
        df_residual: m.df_residual,
    })?)
}

/// Jarque–Bera on a residual vector. `{ residuals:[..] }` → `{ statistic, p_value, skewness, excess_kurtosis }`.
#[wasm_bindgen]
pub fn stats_jarque_bera_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        residuals: Vec<f64>,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let r = crate::solvers::statistics::regression::jarque_bera(&p.residuals)
        .ok_or_else(|| JsValue::from_str("jarque_bera requires n>=4"))?;
    Ok(serde_wasm_bindgen::to_value(&r)?)
}

/// Breusch–Pagan. `{ residuals, x:number[][] }` (x = original predictors).
#[wasm_bindgen]
pub fn stats_breusch_pagan_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        residuals: Vec<f64>,
        x: Vec<Vec<f64>>,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let (x, n, kpred) = flat_x(&p.x)?;
    if p.residuals.len() != n {
        return Err(JsValue::from_str("residuals length must equal x rows"));
    }
    let k = kpred + 1;
    let mut design = vec![0.0; n * k];
    for i in 0..n {
        design[i * k] = 1.0;
        design[i * k + 1..i * k + k].copy_from_slice(&x[i * kpred..(i + 1) * kpred]);
    }
    let r = crate::solvers::statistics::regression::breusch_pagan(&p.residuals, &design, n, k)
        .ok_or_else(|| JsValue::from_str("breusch_pagan failed"))?;
    Ok(serde_wasm_bindgen::to_value(&r)?)
}

/// Durbin–Watson. `{ residuals:[..] }` → `{ statistic, approx_p_value }`.
#[wasm_bindgen]
pub fn stats_durbin_watson_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        residuals: Vec<f64>,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let r = crate::solvers::statistics::regression::durbin_watson(&p.residuals)
        .ok_or_else(|| JsValue::from_str("durbin_watson requires n>=3"))?;
    Ok(serde_wasm_bindgen::to_value(&r)?)
}

/// VIF per predictor column. `{ x:number[][] }` → `{ vif:[..] }`.
#[wasm_bindgen]
pub fn stats_vif_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        x: Vec<Vec<f64>>,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let (x, n, k) = flat_x(&p.x)?;
    let r = crate::solvers::statistics::regression::vif_columns(&x, n, k)
        .ok_or_else(|| JsValue::from_str("vif needs >=2 columns and n>p"))?;
    Ok(serde_wasm_bindgen::to_value(&r)?)
}

/// Full verification report + soft/hard flags. `{ x, y, alpha?, strict? }`.
#[wasm_bindgen]
pub fn verify_regression_model_wasm(val: JsValue) -> Result<JsValue, JsValue> {
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
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let (x, n, k) = flat_x(&p.x)?;
    if p.y.len() != n {
        return Err(JsValue::from_str("y length must equal x rows"));
    }
    let m = crate::solvers::statistics::regression::multiple_ols(&x, &p.y, n, k, true)
        .ok_or_else(|| JsValue::from_str("OLS failed"))?;
    let opts = crate::solvers::statistics::regression::VerifyOptions {
        alpha: p.alpha,
        strict: p.strict,
        ..Default::default()
    };
    let report = crate::solvers::statistics::regression::verify_regression_model(
        &m,
        Some((&x, k)),
        Some(&p.y),
        &opts,
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
        alpha: f64,
        f_p_value: Option<f64>,
        jarque_bera_p: Option<f64>,
        breusch_pagan_p: Option<f64>,
        durbin_watson: Option<f64>,
        max_vif: Option<f64>,
        ramsey_reset_p: Option<f64>,
        symmetry_p: Option<f64>,
        runs_p: Option<f64>,
        arch_lm_p: Option<f64>,
        box_pearce_p: Option<f64>,
        spurious_warn: bool,
        coefficients: Vec<f64>,
        residuals: Vec<f64>,
        fitted: Vec<f64>,
        r_squared: f64,
        n: usize,
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
        alpha: report.alpha,
        f_p_value: report.f_p_value,
        jarque_bera_p: report.jarque_bera_p,
        breusch_pagan_p: report.breusch_pagan_p,
        durbin_watson: report.durbin_watson,
        max_vif: report.max_vif,
        ramsey_reset_p: report.ramsey_reset_p,
        symmetry_p: report.symmetry_p,
        runs_p: report.runs_p,
        arch_lm_p: report.arch_lm_p,
        box_pearce_p: report.box_pearce_p,
        spurious_warn: report.spurious_warn,
        coefficients: m.coefficients,
        residuals: m.residuals,
        fitted: m.fitted,
        r_squared: m.r_squared,
        n: m.n,
    })?)
}

/// Univariate outlier screen. `{ data:[..] }` → mean/median + 2σ/3σ indices.
#[wasm_bindgen]
pub fn stats_outlier_screen_univariate_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        data: Vec<f64>,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let r = crate::solvers::statistics::regression::univariate_outlier_screen(&p.data)
        .ok_or_else(|| JsValue::from_str("need n>=4"))?;
    Ok(serde_wasm_bindgen::to_value(&r)?)
}

/// Influence (leverage / Cook / studentized). `{ x, y }` — flags only, never drops.
#[wasm_bindgen]
pub fn stats_influence_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        x: Vec<Vec<f64>>,
        y: Vec<f64>,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let (x, n, k) = flat_x(&p.x)?;
    let m = crate::solvers::statistics::regression::multiple_ols(&x, &p.y, n, k, true)
        .ok_or_else(|| JsValue::from_str("OLS failed"))?;
    let inf = crate::solvers::statistics::regression::influence_measures(&m)
        .ok_or_else(|| JsValue::from_str("influence failed"))?;
    Ok(serde_wasm_bindgen::to_value(&inf)?)
}

/// Mahalanobis outliers. `{ x:number[][], alpha?:number }`.
#[wasm_bindgen]
pub fn stats_mahalanobis_outliers_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        x: Vec<Vec<f64>>,
        #[serde(default = "default_alpha")]
        alpha: f64,
    }
    fn default_alpha() -> f64 {
        0.01
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let (x, n, d) = flat_x(&p.x)?;
    let r = crate::solvers::statistics::regression::mahalanobis_outliers(&x, n, d, p.alpha)
        .ok_or_else(|| JsValue::from_str("mahalanobis failed"))?;
    Ok(serde_wasm_bindgen::to_value(&r)?)
}

/// Ramsey RESET. `{ x, y, power_max?:number }`.
#[wasm_bindgen]
pub fn stats_ramsey_reset_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        x: Vec<Vec<f64>>,
        y: Vec<f64>,
        #[serde(default = "default_power")]
        power_max: usize,
    }
    fn default_power() -> usize {
        3
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let (x, n, k) = flat_x(&p.x)?;
    let r = crate::solvers::statistics::regression::ramsey_reset(&x, &p.y, n, k, p.power_max)
        .ok_or_else(|| JsValue::from_str("ramsey_reset failed"))?;
    Ok(serde_wasm_bindgen::to_value(&r)?)
}

/// Residual symmetry. `{ residuals:[..] }`.
#[wasm_bindgen]
pub fn stats_residual_symmetry_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        residuals: Vec<f64>,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let r = crate::solvers::statistics::regression::residual_symmetry(&p.residuals)
        .ok_or_else(|| JsValue::from_str("symmetry requires n>=5"))?;
    Ok(serde_wasm_bindgen::to_value(&r)?)
}

/// Residual runs test. `{ residuals:[..] }`.
#[wasm_bindgen]
pub fn stats_residual_runs_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        residuals: Vec<f64>,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let r = crate::solvers::statistics::regression::residual_runs(&p.residuals)
        .ok_or_else(|| JsValue::from_str("runs requires n>=8"))?;
    Ok(serde_wasm_bindgen::to_value(&r)?)
}

/// Chow structural break. `{ x, y, break_index }`.
#[wasm_bindgen]
pub fn stats_chow_test_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        x: Vec<Vec<f64>>,
        y: Vec<f64>,
        break_index: usize,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let (x, n, k) = flat_x(&p.x)?;
    let r = crate::solvers::statistics::regression::chow_test(&x, &p.y, n, k, p.break_index)
        .ok_or_else(|| JsValue::from_str("chow_test failed (check break_index / n)"))?;
    Ok(serde_wasm_bindgen::to_value(&r)?)
}

/// Backward stepwise (exploratory). `{ x, y, exit_alpha?, max_steps? }`.
#[wasm_bindgen]
pub fn stats_stepwise_backward_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        x: Vec<Vec<f64>>,
        y: Vec<f64>,
        #[serde(default = "default_alpha")]
        exit_alpha: f64,
        #[serde(default = "default_steps")]
        max_steps: usize,
    }
    fn default_alpha() -> f64 {
        0.05
    }
    fn default_steps() -> usize {
        8
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let (x, n, k) = flat_x(&p.x)?;
    let r = crate::solvers::statistics::regression::stepwise_backward(
        &x,
        &p.y,
        n,
        k,
        p.exit_alpha,
        p.max_steps,
    )
    .ok_or_else(|| JsValue::from_str("stepwise failed"))?;
    Ok(serde_wasm_bindgen::to_value(&r)?)
}

/// Dummy design. `{ categories:number[] }` → matrix + labels.
#[wasm_bindgen]
pub fn design_dummies_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        categories: Vec<u32>,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let r = crate::solvers::statistics::regression::design_dummies(&p.categories)
        .ok_or_else(|| JsValue::from_str("need ≥2 distinct categories"))?;
    Ok(serde_wasm_bindgen::to_value(&r)?)
}

/// Series transform. `{ values, kind: "log"|"log1p"|"sqrt"|"square"|"reciprocal"|"exp" }`.
#[wasm_bindgen]
pub fn transform_series_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        values: Vec<f64>,
        kind: String,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    use crate::solvers::statistics::regression::TransformKind;
    let kind = match p.kind.as_str() {
        "log" => TransformKind::Log,
        "log1p" => TransformKind::Log1p,
        "sqrt" => TransformKind::Sqrt,
        "square" => TransformKind::Square,
        "reciprocal" => TransformKind::Reciprocal,
        "exp" => TransformKind::Exp,
        _ => return Err(JsValue::from_str("unknown transform kind")),
    };
    let out = crate::solvers::statistics::regression::transform_series(&p.values, kind)
        .ok_or_else(|| JsValue::from_str("transform domain error"))?;
    #[derive(Serialize)]
    struct Out {
        values: Vec<f64>,
    }
    Ok(serde_wasm_bindgen::to_value(&Out { values: out })?)
}

/// Binary logit. `{ x:number[][], y:number[] (0/1), fit_intercept?:bool }`.
#[wasm_bindgen]
pub fn stats_logit_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        x: Vec<Vec<f64>>,
        y: Vec<f64>,
        #[serde(default = "default_true")]
        fit_intercept: bool,
    }
    fn default_true() -> bool {
        true
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let (x, n, k) = flat_x(&p.x)?;
    let r = crate::solvers::statistics::regression::fit_logit(&x, &p.y, n, k, p.fit_intercept)
        .ok_or_else(|| JsValue::from_str("logit failed"))?;
    Ok(serde_wasm_bindgen::to_value(&r)?)
}

/// LDA. `{ x:number[][], y:number[] (int class labels) }` → classes + predictions.
#[wasm_bindgen]
pub fn stats_lda_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        x: Vec<Vec<f64>>,
        y: Vec<usize>,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let (x, n, k) = flat_x(&p.x)?;
    if p.y.len() != n {
        return Err(JsValue::from_str("y length must equal x rows"));
    }
    let r = crate::solvers::statistics::regression::fit_lda_2class(&x, &p.y, n, k)
        .ok_or_else(|| JsValue::from_str("lda failed"))?;
    Ok(serde_wasm_bindgen::to_value(&r)?)
}

/// Spurious-regression guard. `{ y, x?:number[] }`.
#[wasm_bindgen]
pub fn stats_spurious_guard_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        y: Vec<f64>,
        #[serde(default)]
        x: Option<Vec<f64>>,
    }
    let p: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let r = crate::solvers::statistics::regression::spurious_regression_guard(
        &p.y,
        p.x.as_deref(),
    )
    .ok_or_else(|| JsValue::from_str("spurious guard needs n>=8"))?;
    Ok(serde_wasm_bindgen::to_value(&r)?)
}
