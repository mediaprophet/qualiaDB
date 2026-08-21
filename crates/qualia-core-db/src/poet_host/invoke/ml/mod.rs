//! Machine learning invoke seams.
//!
//! Exposes `solvers::learning` through VibeScript invoke IDs
//! in the `MachineLearning.*` namespace.

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod extended;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod ols;

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use extended::{accuracy, kmeans, mae, mse, r2_score, rmse, roc_auc, train_test_split};
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use ols::fit_ols;

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn fit_ols(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "MachineLearning"))
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
macro_rules! ml_stub {
    ($($name:ident),*) => {
        $(
            pub fn $name(
                _args: &poet_vibe::Value,
                span: poet_vibe::Span,
            ) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
                Err(super::args::need_scientific(span, "MachineLearning"))
            }
        )*
    };
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
ml_stub!(
    mse,
    rmse,
    mae,
    r2_score,
    accuracy,
    roc_auc,
    kmeans,
    train_test_split
);
