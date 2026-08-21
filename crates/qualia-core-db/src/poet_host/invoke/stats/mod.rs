//! Statistics invoke seams.
//!
//! Exposes `solvers::statistics` through VibeScript invoke IDs
//! in the `Statistics.*` namespace.

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod correlation;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod correlation_more;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod descriptive;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod descriptive_more;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod extended;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod hypothesis;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod regression;

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use correlation::pearson_r;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use correlation_more::{kendall, spearman};
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use descriptive::arithmetic_mean;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use descriptive_more::{
    covariance, kurtosis, max, median, min, quantile, skewness, std_dev, sum, variance,
};
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use extended::{
    autocorrelation, entropy, exponential_smoothing, iqr, kl_divergence, median_abs_deviation,
    moving_average, trimmed_mean, z_score_outliers,
};
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use hypothesis::{chi_square_gof, one_sample_t, one_way_anova, paired_t, two_sample_t};
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use regression::linear_regression;

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn arithmetic_mean(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "Statistics"))
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn pearson_r(
    args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    arithmetic_mean(args, span)
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn linear_regression(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "Statistics"))
}

// Stub all the new functions for non-scientific builds.
#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
macro_rules! stats_stub {
    ($($name:ident),*) => {
        $(
            pub fn $name(
                _args: &poet_vibe::Value,
                span: poet_vibe::Span,
            ) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
                Err(super::args::need_scientific(span, "Statistics"))
            }
        )*
    };
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
stats_stub!(
    median,
    variance,
    std_dev,
    skewness,
    kurtosis,
    quantile,
    covariance,
    min,
    max,
    sum,
    spearman,
    kendall,
    one_sample_t,
    two_sample_t,
    paired_t,
    chi_square_gof,
    one_way_anova,
    autocorrelation,
    moving_average,
    exponential_smoothing,
    trimmed_mean,
    iqr,
    median_abs_deviation,
    entropy,
    kl_divergence,
    z_score_outliers
);
