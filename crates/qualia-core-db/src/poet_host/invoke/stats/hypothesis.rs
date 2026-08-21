//! Hypothesis testing invoke seams — `solvers::statistics::hypothesis`.

use super::super::args;
use crate::solvers::statistics::hypothesis;
use poet_vibe::{Diagnostic, Span, Value};

/// `Statistics.one_sample_t` — one-sample t-test.
/// Args: { values: [f64], mu: f64 }
pub fn one_sample_t(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let xs = args::rec_f64_list(args, "values")
        .ok_or_else(|| args::bad(span, "Statistics.one_sample_t needs values"))?;
    let mu = args::rec_f64(args, "mu")
        .ok_or_else(|| args::bad(span, "Statistics.one_sample_t needs mu"))?;
    match hypothesis::t_tests::one_sample_t(&xs, mu) {
        Some(r) => Ok(args::record([
            ("t_statistic", Value::F64(r.t_statistic)),
            ("p_value", Value::F64(r.p_value)),
            (
                "degrees_of_freedom",
                Value::U64(r.degrees_of_freedom as u64),
            ),
            ("ci_lower", Value::F64(r.confidence_interval.0)),
            ("ci_upper", Value::F64(r.confidence_interval.1)),
        ])),
        None => Err(args::bad(span, "one_sample_t: insufficient data")),
    }
}

/// `Statistics.two_sample_t` — two-sample t-test.
/// Args: { a: [f64], b: [f64], equal_var: bool }
pub fn two_sample_t(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = args::rec_f64_list(args, "a")
        .ok_or_else(|| args::bad(span, "Statistics.two_sample_t needs a"))?;
    let b = args::rec_f64_list(args, "b")
        .ok_or_else(|| args::bad(span, "Statistics.two_sample_t needs b"))?;
    let equal_var = args::rec_bool(args, "equal_var").unwrap_or(true);
    match hypothesis::t_tests::two_sample_t(&a, &b, equal_var) {
        Some(r) => Ok(args::record([
            ("t_statistic", Value::F64(r.t_statistic)),
            ("p_value", Value::F64(r.p_value)),
            ("degrees_of_freedom", Value::F64(r.degrees_of_freedom)),
            ("mean_diff", Value::F64(r.mean_difference)),
        ])),
        None => Err(args::bad(span, "two_sample_t: insufficient data")),
    }
}

/// `Statistics.paired_t` — paired t-test.
/// Args: { a: [f64], b: [f64] }
pub fn paired_t(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let a = args::rec_f64_list(args, "a")
        .ok_or_else(|| args::bad(span, "Statistics.paired_t needs a"))?;
    let b = args::rec_f64_list(args, "b")
        .ok_or_else(|| args::bad(span, "Statistics.paired_t needs b"))?;
    match hypothesis::t_tests::paired_t(&a, &b) {
        Some(r) => Ok(args::record([
            ("t_statistic", Value::F64(r.t_statistic)),
            ("p_value", Value::F64(r.p_value)),
            (
                "degrees_of_freedom",
                Value::U64(r.degrees_of_freedom as u64),
            ),
            ("ci_lower", Value::F64(r.confidence_interval.0)),
            ("ci_upper", Value::F64(r.confidence_interval.1)),
        ])),
        None => Err(args::bad(span, "paired_t: insufficient data")),
    }
}

/// `Statistics.chi_square_gof` — chi-square goodness-of-fit.
/// Args: { observed: [f64], expected: [f64] }
pub fn chi_square_gof(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let observed = args::rec_f64_list(args, "observed")
        .ok_or_else(|| args::bad(span, "Statistics.chi_square_gof needs observed"))?;
    let expected = args::rec_f64_list(args, "expected")
        .ok_or_else(|| args::bad(span, "Statistics.chi_square_gof needs expected"))?;
    match hypothesis::chi_square::chi_square_gof(&observed, &expected) {
        Some(r) => Ok(args::record([
            ("chi_square", Value::F64(r.statistic)),
            ("p_value", Value::F64(r.p_value)),
            ("degrees_of_freedom", Value::F64(r.dof)),
        ])),
        None => Err(args::bad(span, "chi_square_gof: invalid input")),
    }
}

/// `Statistics.one_way_anova` — one-way ANOVA across groups.
/// Args: { groups: [[f64]] }
pub fn one_way_anova(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let groups_val = args::rec(args, "groups")
        .ok_or_else(|| args::bad(span, "Statistics.one_way_anova needs groups"))?;
    let groups_list = match groups_val {
        Value::List(l) => l,
        _ => {
            return Err(args::bad(
                span,
                "Statistics.one_way_anova: groups must be a list of lists",
            ))
        }
    };
    let mut groups: Vec<Vec<f64>> = Vec::new();
    for g in groups_list {
        let vals = args::f64s(&g).ok_or_else(|| {
            args::bad(
                span,
                "Statistics.one_way_anova: each group must be a number list",
            )
        })?;
        groups.push(vals);
    }
    let refs: Vec<&[f64]> = groups.iter().map(|g| g.as_slice()).collect();
    match hypothesis::anova::one_way_anova(&refs) {
        Some(r) => Ok(args::record([
            ("f_statistic", Value::F64(r.f_statistic)),
            ("p_value", Value::F64(r.p_value)),
            ("df_between", Value::F64(r.df_between)),
            ("df_within", Value::F64(r.df_within)),
        ])),
        None => Err(args::bad(span, "one_way_anova: insufficient data")),
    }
}
