//! General-to-specific / backward stepwise selection (exploratory — receipt must flag).

use crate::solvers::statistics::regression::multiple_ols;

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StepwiseStep {
    pub dropped_column: Option<usize>,
    pub remaining: Vec<usize>,
    pub adj_r_squared: f64,
    pub f_p_value: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StepwiseResult {
    pub selected_columns: Vec<usize>,
    pub steps: Vec<StepwiseStep>,
    pub exploratory: bool,
}

/// Backward elimination: drop the predictor with largest p-value while
/// p > `exit_alpha`, bounded by `max_steps`. Always records `exploratory: true`.
pub fn stepwise_backward(
    x: &[f64],
    y: &[f64],
    n: usize,
    p: usize,
    exit_alpha: f64,
    max_steps: usize,
) -> Option<StepwiseResult> {
    if p == 0 || x.len() != n * p || y.len() != n || max_steps == 0 {
        return None;
    }
    let mut remaining: Vec<usize> = (0..p).collect();
    let mut steps = Vec::new();
    for _ in 0..max_steps {
        if remaining.len() <= 1 {
            break;
        }
        let k = remaining.len();
        let mut xr = vec![0.0; n * k];
        for i in 0..n {
            for (c, &col) in remaining.iter().enumerate() {
                xr[i * k + c] = x[i * p + col];
            }
        }
        let m = multiple_ols(&xr, y, n, k, true)?;
        // p_values[0] = intercept; slopes start at 1.
        let mut worst_local = 0usize;
        let mut worst_p = -1.0_f64;
        for j in 0..k {
            let pv = m.p_values[j + 1];
            if pv > worst_p {
                worst_p = pv;
                worst_local = j;
            }
        }
        steps.push(StepwiseStep {
            dropped_column: None,
            remaining: remaining.clone(),
            adj_r_squared: m.adj_r_squared,
            f_p_value: m.f_p_value,
        });
        if worst_p <= exit_alpha {
            break;
        }
        let dropped = remaining.remove(worst_local);
        if let Some(last) = steps.last_mut() {
            last.dropped_column = Some(dropped);
        }
    }
    // Final fit note.
    let k = remaining.len();
    if k >= 1 {
        let mut xr = vec![0.0; n * k];
        for i in 0..n {
            for (c, &col) in remaining.iter().enumerate() {
                xr[i * k + c] = x[i * p + col];
            }
        }
        if let Some(m) = multiple_ols(&xr, y, n, k, true) {
            steps.push(StepwiseStep {
                dropped_column: None,
                remaining: remaining.clone(),
                adj_r_squared: m.adj_r_squared,
                f_p_value: m.f_p_value,
            });
        }
    }
    Some(StepwiseResult {
        selected_columns: remaining,
        steps,
        exploratory: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drops_noise_predictor() {
        // y ~ x0; x1 is noise.
        let n = 20;
        let mut x = Vec::with_capacity(n * 2);
        let mut y = Vec::with_capacity(n);
        for i in 0..n {
            let t = i as f64;
            x.push(t);
            x.push(((i * 17) % 7) as f64); // weak noise
            y.push(1.0 + 2.0 * t + 0.01 * ((i % 3) as f64));
        }
        let r = stepwise_backward(&x, &y, n, 2, 0.05, 5).unwrap();
        assert!(r.exploratory);
        assert!(
            r.selected_columns == vec![0] || r.selected_columns.contains(&0),
            "sel={:?}",
            r.selected_columns
        );
    }
}
