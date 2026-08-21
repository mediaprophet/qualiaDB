//! Probability distribution invoke seams — `solvers::statistics::distributions`.

use super::super::args;
use crate::solvers::statistics::distributions;
use poet_vibe::{Diagnostic, Span, Value};

// ── Normal ──────────────────────────────────────────────────────────

/// `Statistics.normal_pdf` — Normal PDF.
/// Args: { x: f64, mu: f64, sigma: f64 }
pub fn normal_pdf(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_f64(args, "x").ok_or_else(|| args::bad(span, "normal_pdf needs x"))?;
    let mu = args::rec_f64(args, "mu").unwrap_or(0.0);
    let sigma = args::rec_f64(args, "sigma").unwrap_or(1.0);
    Ok(Value::F64(distributions::normal::pdf(x, mu, sigma)))
}

/// `Statistics.normal_cdf` — Normal CDF.
pub fn normal_cdf(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_f64(args, "x").ok_or_else(|| args::bad(span, "normal_cdf needs x"))?;
    let mu = args::rec_f64(args, "mu").unwrap_or(0.0);
    let sigma = args::rec_f64(args, "sigma").unwrap_or(1.0);
    Ok(Value::F64(distributions::normal::cdf(x, mu, sigma)))
}

/// `Statistics.normal_quantile` — Normal inverse CDF (quantile).
pub fn normal_quantile(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let p = args::rec_f64(args, "p").ok_or_else(|| args::bad(span, "normal_quantile needs p"))?;
    let mu = args::rec_f64(args, "mu").unwrap_or(0.0);
    let sigma = args::rec_f64(args, "sigma").unwrap_or(1.0);
    Ok(Value::F64(distributions::normal::quantile(p, mu, sigma)))
}

/// `Statistics.standard_normal_cdf` — Standard normal CDF (z-score → p).
pub fn standard_normal_cdf(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let z =
        args::rec_f64(args, "z").ok_or_else(|| args::bad(span, "standard_normal_cdf needs z"))?;
    Ok(Value::F64(distributions::normal::standard_cdf(z)))
}

/// `Statistics.two_sided_p` — Two-sided p-value from a z-score.
pub fn two_sided_p(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let z = args::rec_f64(args, "z").ok_or_else(|| args::bad(span, "two_sided_p needs z"))?;
    Ok(Value::F64(distributions::normal::two_sided_p(z)))
}

// ── Student's t ─────────────────────────────────────────────────────

/// `Statistics.students_t_pdf` — Student's t PDF.
pub fn students_t_pdf(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let t = args::rec_f64(args, "t").ok_or_else(|| args::bad(span, "students_t_pdf needs t"))?;
    let nu = args::rec_f64(args, "nu").ok_or_else(|| args::bad(span, "students_t_pdf needs nu"))?;
    Ok(Value::F64(distributions::students_t::pdf(t, nu)))
}

/// `Statistics.students_t_cdf` — Student's t CDF.
pub fn students_t_cdf(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let t = args::rec_f64(args, "t").ok_or_else(|| args::bad(span, "students_t_cdf needs t"))?;
    let nu = args::rec_f64(args, "nu").ok_or_else(|| args::bad(span, "students_t_cdf needs nu"))?;
    Ok(Value::F64(distributions::students_t::cdf(t, nu)))
}

/// `Statistics.students_t_two_sided_p` — Two-sided p-value from t and df.
pub fn students_t_two_sided_p(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let t = args::rec_f64(args, "t")
        .ok_or_else(|| args::bad(span, "students_t_two_sided_p needs t"))?;
    let nu = args::rec_f64(args, "nu")
        .ok_or_else(|| args::bad(span, "students_t_two_sided_p needs nu"))?;
    Ok(Value::F64(distributions::students_t::two_sided_p(t, nu)))
}

// ── Chi-squared ─────────────────────────────────────────────────────

/// `Statistics.chi_squared_pdf` — Chi-squared PDF.
pub fn chi_squared_pdf(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_f64(args, "x").ok_or_else(|| args::bad(span, "chi_squared_pdf needs x"))?;
    let k = args::rec_f64(args, "k").ok_or_else(|| args::bad(span, "chi_squared_pdf needs k"))?;
    Ok(Value::F64(distributions::chi_squared::pdf(x, k)))
}

/// `Statistics.chi_squared_cdf` — Chi-squared CDF.
pub fn chi_squared_cdf(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_f64(args, "x").ok_or_else(|| args::bad(span, "chi_squared_cdf needs x"))?;
    let k = args::rec_f64(args, "k").ok_or_else(|| args::bad(span, "chi_squared_cdf needs k"))?;
    Ok(Value::F64(distributions::chi_squared::cdf(x, k)))
}

/// `Statistics.chi_squared_upper_p` — Chi-squared upper-tail p-value.
pub fn chi_squared_upper_p(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x =
        args::rec_f64(args, "x").ok_or_else(|| args::bad(span, "chi_squared_upper_p needs x"))?;
    let k =
        args::rec_f64(args, "k").ok_or_else(|| args::bad(span, "chi_squared_upper_p needs k"))?;
    Ok(Value::F64(distributions::chi_squared::upper_p(x, k)))
}

// ── Fisher F ────────────────────────────────────────────────────────

/// `Statistics.fisher_f_pdf` — F-distribution PDF.
pub fn fisher_f_pdf(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_f64(args, "x").ok_or_else(|| args::bad(span, "fisher_f_pdf needs x"))?;
    let d1 = args::rec_f64(args, "d1").ok_or_else(|| args::bad(span, "fisher_f_pdf needs d1"))?;
    let d2 = args::rec_f64(args, "d2").ok_or_else(|| args::bad(span, "fisher_f_pdf needs d2"))?;
    Ok(Value::F64(distributions::fisher_f::pdf(x, d1, d2)))
}

/// `Statistics.fisher_f_cdf` — F-distribution CDF.
pub fn fisher_f_cdf(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_f64(args, "x").ok_or_else(|| args::bad(span, "fisher_f_cdf needs x"))?;
    let d1 = args::rec_f64(args, "d1").ok_or_else(|| args::bad(span, "fisher_f_cdf needs d1"))?;
    let d2 = args::rec_f64(args, "d2").ok_or_else(|| args::bad(span, "fisher_f_cdf needs d2"))?;
    Ok(Value::F64(distributions::fisher_f::cdf(x, d1, d2)))
}

/// `Statistics.fisher_f_upper_p` — F-distribution upper-tail p-value.
pub fn fisher_f_upper_p(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_f64(args, "x").ok_or_else(|| args::bad(span, "fisher_f_upper_p needs x"))?;
    let d1 =
        args::rec_f64(args, "d1").ok_or_else(|| args::bad(span, "fisher_f_upper_p needs d1"))?;
    let d2 =
        args::rec_f64(args, "d2").ok_or_else(|| args::bad(span, "fisher_f_upper_p needs d2"))?;
    Ok(Value::F64(distributions::fisher_f::upper_p(x, d1, d2)))
}

// ── Discrete distributions ──────────────────────────────────────────

/// `Statistics.binomial_pmf` — Binomial PMF.
/// Args: { k: u64, n: u64, p: f64 }
pub fn binomial_pmf(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let k = args::rec_u64(args, "k").ok_or_else(|| args::bad(span, "binomial_pmf needs k"))? as u32;
    let n = args::rec_u64(args, "n").ok_or_else(|| args::bad(span, "binomial_pmf needs n"))? as u32;
    let p = args::rec_f64(args, "p").ok_or_else(|| args::bad(span, "binomial_pmf needs p"))?;
    Ok(Value::F64(distributions::binomial_pmf(k, n, p)))
}

/// `Statistics.binomial_cdf` — Binomial CDF.
pub fn binomial_cdf(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let k = args::rec_u64(args, "k").ok_or_else(|| args::bad(span, "binomial_cdf needs k"))? as u32;
    let n = args::rec_u64(args, "n").ok_or_else(|| args::bad(span, "binomial_cdf needs n"))? as u32;
    let p = args::rec_f64(args, "p").ok_or_else(|| args::bad(span, "binomial_cdf needs p"))?;
    Ok(Value::F64(distributions::binomial_cdf(k, n, p)))
}

/// `Statistics.poisson_pmf` — Poisson PMF.
/// Args: { k: u64, lambda: f64 }
pub fn poisson_pmf(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let k = args::rec_u64(args, "k").ok_or_else(|| args::bad(span, "poisson_pmf needs k"))? as u32;
    let lambda =
        args::rec_f64(args, "lambda").ok_or_else(|| args::bad(span, "poisson_pmf needs lambda"))?;
    Ok(Value::F64(distributions::poisson_pmf(k, lambda)))
}

/// `Statistics.poisson_cdf` — Poisson CDF.
pub fn poisson_cdf(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let k = args::rec_u64(args, "k").ok_or_else(|| args::bad(span, "poisson_cdf needs k"))? as u32;
    let lambda =
        args::rec_f64(args, "lambda").ok_or_else(|| args::bad(span, "poisson_cdf needs lambda"))?;
    Ok(Value::F64(distributions::poisson_cdf(k, lambda)))
}

// ── Continuous distributions ────────────────────────────────────────

/// `Statistics.exponential_pdf` — Exponential PDF.
/// Args: { x: f64, rate: f64 }
pub fn exponential_pdf(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_f64(args, "x").ok_or_else(|| args::bad(span, "exponential_pdf needs x"))?;
    let rate =
        args::rec_f64(args, "rate").ok_or_else(|| args::bad(span, "exponential_pdf needs rate"))?;
    Ok(Value::F64(distributions::exponential_pdf(x, rate)))
}

/// `Statistics.exponential_cdf` — Exponential CDF.
pub fn exponential_cdf(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_f64(args, "x").ok_or_else(|| args::bad(span, "exponential_cdf needs x"))?;
    let rate =
        args::rec_f64(args, "rate").ok_or_else(|| args::bad(span, "exponential_cdf needs rate"))?;
    Ok(Value::F64(distributions::exponential_cdf(x, rate)))
}

/// `Statistics.gamma_pdf` — Gamma PDF.
/// Args: { x: f64, shape: f64, scale: f64 }
pub fn gamma_pdf(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_f64(args, "x").ok_or_else(|| args::bad(span, "gamma_pdf needs x"))?;
    let shape =
        args::rec_f64(args, "shape").ok_or_else(|| args::bad(span, "gamma_pdf needs shape"))?;
    let scale =
        args::rec_f64(args, "scale").ok_or_else(|| args::bad(span, "gamma_pdf needs scale"))?;
    Ok(Value::F64(distributions::gamma_pdf(x, shape, scale)))
}

/// `Statistics.beta_pdf` — Beta PDF.
/// Args: { x: f64, alpha: f64, beta: f64 }
pub fn beta_pdf(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_f64(args, "x").ok_or_else(|| args::bad(span, "beta_pdf needs x"))?;
    let alpha =
        args::rec_f64(args, "alpha").ok_or_else(|| args::bad(span, "beta_pdf needs alpha"))?;
    let beta = args::rec_f64(args, "beta").ok_or_else(|| args::bad(span, "beta_pdf needs beta"))?;
    Ok(Value::F64(distributions::beta_pdf(x, alpha, beta)))
}

/// `Statistics.weibull_pdf` — Weibull PDF.
/// Args: { x: f64, shape: f64, scale: f64 }
pub fn weibull_pdf(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_f64(args, "x").ok_or_else(|| args::bad(span, "weibull_pdf needs x"))?;
    let shape =
        args::rec_f64(args, "shape").ok_or_else(|| args::bad(span, "weibull_pdf needs shape"))?;
    let scale =
        args::rec_f64(args, "scale").ok_or_else(|| args::bad(span, "weibull_pdf needs scale"))?;
    Ok(Value::F64(distributions::weibull_pdf(x, shape, scale)))
}

/// `Statistics.lognormal_pdf` — Lognormal PDF.
/// Args: { x: f64, mu: f64, sigma: f64 }
pub fn lognormal_pdf(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_f64(args, "x").ok_or_else(|| args::bad(span, "lognormal_pdf needs x"))?;
    let mu = args::rec_f64(args, "mu").ok_or_else(|| args::bad(span, "lognormal_pdf needs mu"))?;
    let sigma =
        args::rec_f64(args, "sigma").ok_or_else(|| args::bad(span, "lognormal_pdf needs sigma"))?;
    Ok(Value::F64(distributions::lognormal_pdf(x, mu, sigma)))
}

/// `Statistics.uniform_pdf` — Uniform PDF on [a, b].
/// Args: { x: f64, a: f64, b: f64 }
pub fn uniform_pdf(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_f64(args, "x").ok_or_else(|| args::bad(span, "uniform_pdf needs x"))?;
    let a = args::rec_f64(args, "a").ok_or_else(|| args::bad(span, "uniform_pdf needs a"))?;
    let b = args::rec_f64(args, "b").ok_or_else(|| args::bad(span, "uniform_pdf needs b"))?;
    Ok(Value::F64(distributions::uniform_pdf(x, a, b)))
}

/// `Statistics.laplace_pdf` — Laplace (double exponential) PDF.
/// Args: { x: f64, mu: f64, b: f64 }
pub fn laplace_pdf(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_f64(args, "x").ok_or_else(|| args::bad(span, "laplace_pdf needs x"))?;
    let mu = args::rec_f64(args, "mu").ok_or_else(|| args::bad(span, "laplace_pdf needs mu"))?;
    let b = args::rec_f64(args, "b").ok_or_else(|| args::bad(span, "laplace_pdf needs b"))?;
    Ok(Value::F64(distributions::laplace_pdf(x, mu, b)))
}

// ── Special functions ───────────────────────────────────────────────

/// `Statistics.ln_gamma` — Log-gamma function.
pub fn ln_gamma(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_f64(args, "x").ok_or_else(|| args::bad(span, "ln_gamma needs x"))?;
    Ok(Value::F64(distributions::special::ln_gamma(x)))
}

/// `Statistics.gamma_fn` — Gamma function.
pub fn gamma_fn(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_f64(args, "x").ok_or_else(|| args::bad(span, "gamma_fn needs x"))?;
    Ok(Value::F64(distributions::special::gamma(x)))
}

/// `Statistics.erf` — Error function.
pub fn erf(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_f64(args, "x").ok_or_else(|| args::bad(span, "erf needs x"))?;
    Ok(Value::F64(distributions::special::erf(x)))
}

/// `Statistics.erfc` — Complementary error function.
pub fn erfc(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let x = args::rec_f64(args, "x").ok_or_else(|| args::bad(span, "erfc needs x"))?;
    Ok(Value::F64(distributions::special::erfc(x)))
}

/// `Statistics.empirical_cdf` — Empirical CDF from sorted samples.
/// Args: { samples: [f64], x: f64 }
pub fn empirical_cdf(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let samples = args::rec_f64_list(args, "samples")
        .ok_or_else(|| args::bad(span, "empirical_cdf needs samples"))?;
    let x = args::rec_f64(args, "x").ok_or_else(|| args::bad(span, "empirical_cdf needs x"))?;
    Ok(Value::F64(distributions::empirical_cdf(&samples, x)))
}
