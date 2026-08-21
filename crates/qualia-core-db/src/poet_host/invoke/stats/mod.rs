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
mod distributions;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod extended;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod extra;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod hypothesis;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod more;
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
pub use distributions::*;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use extended::{
    autocorrelation, entropy, exponential_smoothing, iqr, kl_divergence, median_abs_deviation,
    moving_average, trimmed_mean, z_score_outliers,
};
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use extra::*;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use hypothesis::{chi_square_gof, one_sample_t, one_way_anova, paired_t, two_sample_t};
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use more::*;
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
    z_score_outliers,
    // distributions
    normal_pdf,
    normal_cdf,
    normal_quantile,
    standard_normal_cdf,
    two_sided_p,
    students_t_pdf,
    students_t_cdf,
    students_t_two_sided_p,
    chi_squared_pdf,
    chi_squared_cdf,
    chi_squared_upper_p,
    fisher_f_pdf,
    fisher_f_cdf,
    fisher_f_upper_p,
    binomial_pmf,
    binomial_cdf,
    poisson_pmf,
    poisson_cdf,
    exponential_pdf,
    exponential_cdf,
    gamma_pdf,
    beta_pdf,
    weibull_pdf,
    lognormal_pdf,
    uniform_pdf,
    laplace_pdf,
    ln_gamma,
    gamma_fn,
    erf,
    erfc,
    empirical_cdf,
    // extra
    mode,
    winsorized_mean,
    cross_entropy,
    mutual_information,
    histogram,
    correlation_p_value,
    chi_square_independence,
    modified_z_score_outliers,
    iqr_outliers,
    grubbs_test,
    mann_whitney_u,
    ks_1sample,
    friedman,
    mcnemar,
    bootstrap_means,
    ljung_box,
    adf_proxy,
    // more
    argmax,
    standard_pdf,
    standard_quantile,
    lognormal_cdf,
    uniform_cdf,
    laplace_cdf,
    students_t_quantile,
    students_t_upper_p,
    chi_squared_quantile,
    fisher_f_quantile,
    gammp,
    gammq,
    betai,
    entropy_from_counts,
    tukey_fences,
    mahalanobis_sq,
    mvn_log_pdf,
    mvn_pdf,
    mvn_sample,
    mvn_mle
);
