//! Model-evaluation metrics for statistical learning (ISL ch 2, 4, 9).
//!
//! Regression (MSE/RMSE/MAE/R²) and classification (accuracy, confusion matrix,
//! ROC AUC, log-loss) measures over caller-owned slices. Reuses
//! `statistics::descriptive` (mean) and `statistics::correlation` (ranking for AUC)
//! — no re-implementation.

pub mod classification;
pub mod regression;

pub use classification::{accuracy, confusion_binary, log_loss, roc_auc, ConfusionBinary};
pub use regression::{mae, mse, r2_score, rmse};
