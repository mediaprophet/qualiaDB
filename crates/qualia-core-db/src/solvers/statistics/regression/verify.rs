//! Compose OLS verification flags for Civics admission (Welc Ch.4.11).

use super::diagnostics::{
    arch_lm, box_pearce_ac, breusch_pagan, durbin_watson, jarque_bera, ramsey_reset,
    residual_runs, residual_symmetry, vif_columns,
};
use super::multiple::MultipleOls;
use super::spurious::spurious_regression_guard;

/// Named verification failure / warning for a fitted model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationFlag {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VerificationReport {
    /// `true` iff no hard flags (warnings-only still sets ok=true unless `strict`).
    pub ok: bool,
    pub flags: Vec<VerificationFlag>,
    pub alpha: f64,
    pub f_p_value: Option<f64>,
    pub jarque_bera_p: Option<f64>,
    pub breusch_pagan_p: Option<f64>,
    pub durbin_watson: Option<f64>,
    pub max_vif: Option<f64>,
    pub ramsey_reset_p: Option<f64>,
    pub symmetry_p: Option<f64>,
    pub runs_p: Option<f64>,
    pub arch_lm_p: Option<f64>,
    pub box_pearce_p: Option<f64>,
    pub spurious_warn: bool,
}

/// Thresholds: α for tests; VIF soft/hard.
pub struct VerifyOptions {
    pub alpha: f64,
    pub vif_soft: f64,
    pub vif_hard: f64,
    pub check_spurious: bool,
    /// When true, soft warnings also set `ok = false`.
    pub strict: bool,
}

impl Default for VerifyOptions {
    fn default() -> Self {
        Self {
            alpha: 0.05,
            vif_soft: 5.0,
            vif_hard: 10.0,
            check_spurious: true,
            strict: false,
        }
    }
}

/// Run REG-03..07 (+ RESET, symmetry, runs, ARCH-LM, optional spurious) on a fit.
///
/// `x_raw` is the original `n × p` predictor matrix (no intercept); required for
/// VIF / RESET / spurious. Pass `None` to skip those.
pub fn verify_regression_model(
    model: &MultipleOls,
    x_raw: Option<(&[f64], usize)>,
    y: Option<&[f64]>,
    opts: &VerifyOptions,
) -> VerificationReport {
    let mut flags = Vec::new();
    let mut hard = false;
    let mut soft = false;

    let f_p = model.f_p_value;
    if let Some(p) = f_p {
        if p > opts.alpha {
            flags.push(VerificationFlag {
                code: "overall_f_fail".into(),
                message: format!("overall F p={p:.4} > α={}", opts.alpha),
            });
            hard = true;
        }
    }

    let jb_p = jarque_bera(&model.residuals).map(|r| {
        if r.p_value < opts.alpha {
            flags.push(VerificationFlag {
                code: "non_normal_residuals".into(),
                message: format!("Jarque–Bera p={:.4}", r.p_value),
            });
            hard = true;
        }
        r.p_value
    });

    let bp_p = breusch_pagan(&model.residuals, &model.design, model.n, model.n_params).map(
        |r| {
            if r.p_value < opts.alpha {
                flags.push(VerificationFlag {
                    code: "heteroscedasticity".into(),
                    message: format!("Breusch–Pagan p={:.4}", r.p_value),
                });
                hard = true;
            }
            r.p_value
        },
    );

    let dw = durbin_watson(&model.residuals).map(|r| {
        // Classical rule of thumb: DW far from 2 indicates residual AC.
        if r.statistic < 1.5 || r.statistic > 2.5 {
            flags.push(VerificationFlag {
                code: "autocorrelation".into(),
                message: format!("Durbin–Watson={:.3} (approx p={:.4})", r.statistic, r.approx_p_value),
            });
            hard = true;
        }
        r.statistic
    });

    let bp_ac = box_pearce_ac(&model.residuals).map(|r| {
        if r.p_value < opts.alpha {
            flags.push(VerificationFlag {
                code: "residual_acf".into(),
                message: format!("Box–Pearce lag-1 p={:.4}", r.p_value),
            });
            soft = true;
        }
        r.p_value
    });

    let arch_p = arch_lm(&model.residuals, 1).map(|r| {
        if r.p_value < opts.alpha {
            flags.push(VerificationFlag {
                code: "arch_lm".into(),
                message: format!("ARCH-LM(1) p={:.4}", r.p_value),
            });
            soft = true;
        }
        r.p_value
    });

    let mut max_vif = None;
    let mut reset_p = None;
    let mut spurious_warn = false;

    if let Some((x, p)) = x_raw {
        if let Some(v) = vif_columns(x, model.n, p) {
            let mx = v.vif.iter().cloned().fold(0.0_f64, f64::max);
            max_vif = Some(mx);
            if mx >= opts.vif_hard {
                flags.push(VerificationFlag {
                    code: "high_vif".into(),
                    message: format!("max VIF={mx:.2} ≥ {}", opts.vif_hard),
                });
                hard = true;
            } else if mx >= opts.vif_soft {
                flags.push(VerificationFlag {
                    code: "elevated_vif".into(),
                    message: format!("max VIF={mx:.2} ≥ {}", opts.vif_soft),
                });
                soft = true;
            }
        }
        if let Some(y) = y {
            if let Some(r) = ramsey_reset(x, y, model.n, p, 3) {
                reset_p = Some(r.p_value);
                if r.p_value < opts.alpha {
                    flags.push(VerificationFlag {
                        code: "reset_fail".into(),
                        message: format!("Ramsey RESET p={:.4}", r.p_value),
                    });
                    hard = true;
                }
            }
            if opts.check_spurious && p >= 1 {
                // Use first predictor column as x series for bivariate guard.
                let mut x0 = vec![0.0; model.n];
                for i in 0..model.n {
                    x0[i] = x[i * p];
                }
                if let Some(g) = spurious_regression_guard(y, Some(&x0)) {
                    spurious_warn = g.warn_spurious;
                    if g.warn_spurious {
                        flags.push(VerificationFlag {
                            code: "spurious_risk".into(),
                            message: g.detail,
                        });
                        soft = true;
                    }
                }
            }
        }
    }

    let sym_p = residual_symmetry(&model.residuals).map(|r| {
        if r.p_value < opts.alpha {
            flags.push(VerificationFlag {
                code: "asymmetry".into(),
                message: format!("residual symmetry t p={:.4}", r.p_value),
            });
            soft = true;
        }
        r.p_value
    });

    let runs_p = residual_runs(&model.residuals).map(|r| {
        if r.p_value < opts.alpha {
            flags.push(VerificationFlag {
                code: "non_random_runs".into(),
                message: format!("runs test p={:.4}", r.p_value),
            });
            soft = true;
        }
        r.p_value
    });

    let ok = if opts.strict {
        !hard && !soft
    } else {
        !hard
    };

    VerificationReport {
        ok,
        flags,
        alpha: opts.alpha,
        f_p_value: f_p,
        jarque_bera_p: jb_p,
        breusch_pagan_p: bp_p,
        durbin_watson: dw,
        max_vif,
        ramsey_reset_p: reset_p,
        symmetry_p: sym_p,
        runs_p,
        arch_lm_p: arch_p,
        box_pearce_p: bp_ac,
        spurious_warn,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solvers::statistics::regression::multiple_ols;

    #[test]
    fn clean_fit_passes() {
        // Cross-section-ish predictors (not a pure time index) + iid noise.
        let x = [
            1.2, 3.5, 2.1, 4.8, 0.5, 5.5, 3.0, 6.2, 1.8, 4.1, 2.7, 5.0, 0.9, 3.8, 4.4, 1.5,
        ];
        let y: Vec<f64> = x
            .iter()
            .enumerate()
            .map(|(i, &xi)| 1.0 + 1.5 * xi + 0.05 * ((i % 5) as f64 - 2.0))
            .collect();
        let m = multiple_ols(&x, &y, 16, 1, true).unwrap();
        let opts = VerifyOptions {
            check_spurious: false,
            ..Default::default()
        };
        let r = verify_regression_model(&m, Some((&x, 1)), Some(&y), &opts);
        assert!(r.ok, "flags={:?}", r.flags);
    }
}
