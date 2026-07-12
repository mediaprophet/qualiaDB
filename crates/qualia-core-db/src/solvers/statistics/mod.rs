//! Statistics solver — the single, canonical home for numeric statistics.
//!
//! Zero-allocation kernels over caller-owned slices, a sibling to
//! `solvers::linear_algebra`. This is where descriptive statistics, hypothesis
//! tests, correlation, and binning live for the *whole* engine.
//!
//! **Composition rule (Modality-First):** specialized/domain libraries
//! (`specialized_libs::statistical_computing`, `machine_learning`, …) marshal
//! their domain data into a slice and call these functions. They MUST NOT carry
//! their own `mean`/`variance`/`correlation` re-implementations. See
//! `MODALITY_FIRST_CONSOLIDATION.md`.
//!
//! Probabilistic *logic* (Bayesian networks, truth-degree reasoning over quins)
//! is a separate concern and lives in `modalities::probabilistic`; it may call
//! into here for numeric work, but the two are not merged.

pub mod anomaly;
pub mod correlation;
pub mod descriptive;
pub mod distributions;
pub mod histogram;
pub mod hypothesis;
pub mod information;
pub mod regression;
pub mod robust;
pub mod timeseries;

pub use correlation::{correlation_p_value, kendall, pearson, rank_into, spearman};
pub use timeseries::{autocorrelation, exponential_smoothing_into, moving_average_into};
pub use descriptive::{
    covariance, kurtosis, max, mean, median_in_place, median_sorted, min, mode_in_place,
    quantile_in_place, quantile_sorted, skewness, std_dev, sum, variance,
};
pub use histogram::{histogram_into, HistRange};

pub use hypothesis::{
    chi_square_gof, chi_square_independence, friedman, ks_1sample, mann_whitney_u, mcnemar,
    one_sample_t, one_way_anova, paired_t, two_sample_t, AnovaResult, ChiSquareResult,
    FriedmanResult, KolmogorovSmirnovResult, MannWhitneyResult, NonparametricResult, TTest, TwoSampleTTest,
};
pub use information::{cross_entropy, entropy, kl_divergence, mutual_information_discrete};
pub use regression::{simple_linear_regression, LinearRegression};
pub use robust::{iqr, median_abs_deviation, trimmed_mean, winsorized_mean};

/// Basic bootstrap mean (cold bounded, for calibration/validation).
/// Resamples with replacement using provided RNG state (SplitMix style).
/// Writes means for `num_samples` into `out`.
pub fn bootstrap_means(
    data: &[f64],
    num_samples: usize,
    seed: u64,
    out: &mut [f64],
) -> Result<usize, ()> {
    if data.is_empty() || num_samples == 0 || out.len() < num_samples {
        return Err(());
    }
    let mut rng = seed;
    for s in 0..num_samples {
        let mut sum = 0.0;
        for _ in 0..data.len() {
            // simple xorshift for demo
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            let idx = (rng as usize) % data.len();
            sum += data[idx];
        }
        out[s] = sum / data.len() as f64;
    }
    Ok(num_samples)
}

/// Ljung-Box test statistic for autocorrelation up to lag h.
/// acf[0..h] are sample autocorrelations (from lag 1).
pub fn ljung_box(acf: &[f64], n: usize, h: usize) -> f64 {
    if h == 0 || acf.len() < h { return f64::NAN; }
    let mut q = 0.0;
    for k in 1..=h {
        if k-1 < acf.len() {
            q += acf[k-1].powi(2) / (n - k) as f64;
        }
    }
    q * n as f64
}

/// Simple ADF-like stationarity proxy (negative means more stationary tendency).
pub fn adf_proxy(series: &[f64]) -> f64 {
    if series.len() < 3 { return f64::NAN; }
    let mut sum_diff = 0.0;
    let mut sum_lag = 0.0;
    for i in 1..series.len() {
        let diff = series[i] - series[i-1];
        sum_diff += diff * series[i-1];
        sum_lag += series[i-1] * series[i-1];
    }
    if sum_lag.abs() < 1e-12 { return 0.0; }
    sum_diff / sum_lag
}

// Additional distributions (5.1-A progress)
pub use distributions::{
    beta_pdf, binomial_cdf, binomial_pmf, empirical_cdf, exponential_cdf, exponential_pdf,
    gamma_pdf, laplace_cdf, laplace_pdf, lognormal_cdf, lognormal_pdf, poisson_cdf, poisson_pmf,
    uniform_cdf, uniform_pdf, weibull_pdf,
};
