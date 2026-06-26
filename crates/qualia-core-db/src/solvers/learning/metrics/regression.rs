//! Regression metrics — error and fit measures over caller-owned prediction slices.
//! Reuses `statistics::descriptive` for the mean (no re-implementation).

use crate::solvers::statistics::descriptive::mean;

/// Mean squared error `Σ(yᵢ−ŷᵢ)²/n`. `None` if lengths differ or are empty.
pub fn mse(y_true: &[f64], y_pred: &[f64]) -> Option<f64> {
    let n = y_true.len();
    if n == 0 || n != y_pred.len() {
        return None;
    }
    let s: f64 = y_true.iter().zip(y_pred).map(|(y, p)| (y - p) * (y - p)).sum();
    Some(s / n as f64)
}

/// Root mean squared error.
pub fn rmse(y_true: &[f64], y_pred: &[f64]) -> Option<f64> {
    mse(y_true, y_pred).map(f64::sqrt)
}

/// Mean absolute error.
pub fn mae(y_true: &[f64], y_pred: &[f64]) -> Option<f64> {
    let n = y_true.len();
    if n == 0 || n != y_pred.len() {
        return None;
    }
    let s: f64 = y_true.iter().zip(y_pred).map(|(y, p)| (y - p).abs()).sum();
    Some(s / n as f64)
}

/// Coefficient of determination `R² = 1 − SSE/SST`. Can be negative for a model
/// worse than the mean. `None` if lengths differ or `SST = 0` (constant target).
pub fn r2_score(y_true: &[f64], y_pred: &[f64]) -> Option<f64> {
    let n = y_true.len();
    if n == 0 || n != y_pred.len() {
        return None;
    }
    let ybar = mean(y_true)?;
    let mut sse = 0.0;
    let mut sst = 0.0;
    for (y, p) in y_true.iter().zip(y_pred) {
        sse += (y - p) * (y - p);
        sst += (y - ybar) * (y - ybar);
    }
    if sst == 0.0 {
        return None;
    }
    Some(1.0 - sse / sst)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perfect_prediction() {
        let y = [1.0, 2.0, 3.0, 4.0];
        assert_eq!(mse(&y, &y), Some(0.0));
        assert_eq!(rmse(&y, &y), Some(0.0));
        assert_eq!(mae(&y, &y), Some(0.0));
        assert!((r2_score(&y, &y).unwrap() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn known_values() {
        let y = [3.0, -0.5, 2.0, 7.0];
        let p = [2.5, 0.0, 2.0, 8.0];
        // errors: 0.5,-0.5,0,-1 → squared 0.25,0.25,0,1 → mse 0.375
        assert!((mse(&y, &p).unwrap() - 0.375).abs() < 1e-12);
        assert!((mae(&y, &p).unwrap() - 0.5).abs() < 1e-12);
        // sklearn r2 for this pair ≈ 0.9486.
        assert!((r2_score(&y, &p).unwrap() - 0.948_608).abs() < 1e-4);
    }

    #[test]
    fn mean_predictor_is_zero_r2() {
        let y = [1.0, 2.0, 3.0, 4.0, 5.0];
        let p = [3.0; 5]; // predicting the mean
        assert!(r2_score(&y, &p).unwrap().abs() < 1e-12);
    }

    #[test]
    fn guards() {
        assert_eq!(mse(&[], &[]), None);
        assert_eq!(mse(&[1.0], &[1.0, 2.0]), None);
        assert_eq!(r2_score(&[5.0, 5.0], &[1.0, 2.0]), None); // constant target
    }
}
