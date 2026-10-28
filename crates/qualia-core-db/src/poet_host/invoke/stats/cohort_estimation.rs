//! VibeScript invoke seam for User-Selectable & User-Defined Cohort Graph Estimation (C2–C3).
#![allow(dead_code)]

use super::super::args;
use crate::q_hash;
use crate::solvers::statistics::cohort_estimator::{
    estimate_invalid_observation_from_cohort, CohortEstimationError, CohortEstimationMethod,
    CohortObservation, CohortQuerySpec,
};
use crate::NQuin;
use vibe::{Diagnostic, Span, Value};

fn parse_u64_or_hash(v: Option<&Value>) -> u64 {
    match v {
        Some(val) => args::as_u64(val).unwrap_or_else(|| args::as_str(val).map(q_hash).unwrap_or(0)),
        None => 0,
    }
}

fn parse_obs(v: &Value, default_valid: bool) -> CohortObservation {
    CohortObservation {
        subject_hash: parse_u64_or_hash(args::rec(v, "subject")),
        predicate_hash: parse_u64_or_hash(args::rec(v, "predicate")),
        taxon_or_domain_hash: parse_u64_or_hash(args::rec(v, "taxon_or_domain")),
        organ_or_feature_hash: parse_u64_or_hash(args::rec(v, "organ_or_feature")),
        stage_or_season_hash: parse_u64_or_hash(args::rec(v, "stage_or_season")),
        unit_hash: parse_u64_or_hash(args::rec(v, "unit")),
        crs_or_frame_hash: parse_u64_or_hash(args::rec(v, "crs_or_frame")),
        epoch_bucket: args::rec_u64(v, "epoch_bucket").unwrap_or(0),
        context: args::rec_u64(v, "context").unwrap_or(0),
        value: args::rec_f64(v, "value").unwrap_or(0.0),
        is_valid: args::rec_bool(v, "is_valid").unwrap_or(default_valid),
        raw_lexical_hash: parse_u64_or_hash(args::rec(v, "raw_lexical")),
    }
}

/// Invokes cohort estimation over comparable observations.
pub fn stats_estimate_cohort(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let target = args::rec(args_v, "target")
        .map(|t| parse_obs(t, false))
        .unwrap_or_default();

    let mut candidates = Vec::new();
    if let Some(cand_list) = args::rec(args_v, "candidates").and_then(args::list) {
        for c in cand_list {
            candidates.push(parse_obs(c, true));
        }
    } else if let Some(vals) = args::rec_f64_list(args_v, "values") {
        for (i, v) in vals.into_iter().enumerate() {
            let mut obs = target;
            obs.subject_hash = (i + 100) as u64;
            obs.value = v;
            obs.is_valid = true;
            candidates.push(obs);
        }
    } else {
        return Err(args::bad(span, "Statistics.estimate_cohort needs candidates or values"));
    }

    let method = match args::rec(args_v, "method") {
        Some(m) => {
            if let Some(s) = args::as_str(m) {
                match s {
                    "median" => CohortEstimationMethod::Median,
                    "mean" => CohortEstimationMethod::Mean,
                    "trimmed_mean" => {
                        let tf = args::rec_f64(args_v, "trim_fraction").unwrap_or(0.1);
                        CohortEstimationMethod::TrimmedMean { trim_fraction: tf }
                    }
                    "winsorized_mean" => {
                        let lf = args::rec_f64(args_v, "limit_fraction").unwrap_or(0.1);
                        CohortEstimationMethod::WinsorizedMean { limit_fraction: lf }
                    }
                    user_defined => CohortEstimationMethod::UserDefinedWeighted {
                        weights_hash: q_hash(user_defined),
                    },
                }
            } else if let Some(u) = args::as_u64(m) {
                CohortEstimationMethod::UserDefinedWeighted { weights_hash: u }
            } else {
                CohortEstimationMethod::Median
            }
        }
        None => CohortEstimationMethod::Median,
    };

    let spec = CohortQuerySpec {
        min_cohort_size: args::rec_u64(args_v, "min_cohort_size").unwrap_or(3) as usize,
        max_sensitivity_class: args::rec_u64(args_v, "max_sensitivity_class").unwrap_or(0) as u8,
        match_taxon_or_domain: args::rec_bool(args_v, "match_taxon_or_domain").unwrap_or(true),
        match_organ_or_feature: args::rec_bool(args_v, "match_organ_or_feature").unwrap_or(true),
        match_stage_or_season: args::rec_bool(args_v, "match_stage_or_season").unwrap_or(true),
        match_epoch_bucket: args::rec_bool(args_v, "match_epoch_bucket").unwrap_or(false),
        strict_units: args::rec_bool(args_v, "strict_units").unwrap_or(true),
        strict_frames: args::rec_bool(args_v, "strict_frames").unwrap_or(true),
    };

    let scratch_size = (candidates.len().max(spec.min_cohort_size)) * 2 + 16;
    let mut scratch = vec![0.0f64; scratch_size];
    let mut out_quins = [NQuin::default(); 3];

    match estimate_invalid_observation_from_cohort(&target, &candidates, &spec, method, &mut scratch, &mut out_quins) {
        Ok(res) => Ok(args::record([
            ("estimate", Value::F64(res.estimate)),
            ("uncertainty_mad", Value::F64(res.uncertainty_mad)),
            ("uncertainty_iqr", Value::F64(res.uncertainty_iqr)),
            ("cohort_size", Value::U64(res.cohort_size as u64)),
            ("method_hash", Value::U64(res.method_hash)),
            ("inherited_sensitivity", Value::U64(res.inherited_sensitivity as u64)),
        ])),
        Err(err) => {
            let msg = match err {
                CohortEstimationError::InsufficientCohort => "CohortEstimation: InsufficientCohort",
                CohortEstimationError::IncompatibleUnits => "CohortEstimation: IncompatibleUnits",
                CohortEstimationError::IncompatibleFrames => "CohortEstimation: IncompatibleFrames",
                CohortEstimationError::SensitivityDenied => "CohortEstimation: SensitivityDenied",
                CohortEstimationError::ScratchBufferTooSmall => "CohortEstimation: ScratchBufferTooSmall",
                CohortEstimationError::InvalidParameter => "CohortEstimation: InvalidParameter",
                CohortEstimationError::InvalidEstimateValue => "CohortEstimation: InvalidEstimateValue",
                CohortEstimationError::OutputBufferTooSmall => "CohortEstimation: OutputBufferTooSmall",
            };
            Err(args::bad(span, msg))
        }
    }
}

/// Alias for `stats_estimate_cohort` matching `Statistics.estimate_cohort`.
pub fn estimate_cohort(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    stats_estimate_cohort(args_v, span)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vibe_median_estimation() {
        let args = args::record([
            ("values", args::f64_list_value(vec![10.0, 12.0, 14.0, 16.0, 18.0])),
            ("method", Value::String("median".into())),
        ]);
        let res = estimate_cohort(&args, Span { start: 0, end: 0 }).expect("invoke should pass");
        assert_eq!(args::rec_f64(&res, "estimate"), Some(14.0));
        assert_eq!(args::rec_u64(&res, "cohort_size"), Some(5));
    }

    #[test]
    fn test_vibe_user_defined_and_trimmed_mean() {
        let args = args::record([
            ("values", args::f64_list_value(vec![10.0, 20.0, 30.0, 1000.0])),
            ("method", Value::String("trimmed_mean".into())),
            ("trim_fraction", Value::F64(0.25)),
        ]);
        let res = estimate_cohort(&args, Span { start: 0, end: 0 }).expect("trimmed_mean should pass");
        assert!(args::rec_f64(&res, "estimate").unwrap() < 50.0);

        let args_user = args::record([
            ("values", args::f64_list_value(vec![10.0, 20.0, 30.0])),
            ("method", Value::String("custom_weights_v1".into())),
        ]);
        let res_user = estimate_cohort(&args_user, Span { start: 0, end: 0 }).expect("user method pass");
        assert_eq!(args::rec_u64(&res_user, "method_hash"), Some(q_hash("custom_weights_v1")));
    }

    #[test]
    fn test_vibe_insufficient_cohort_error() {
        let args = args::record([
            ("values", args::f64_list_value(vec![10.0, 12.0])),
            ("min_cohort_size", Value::U64(3)),
        ]);
        let err = estimate_cohort(&args, Span { start: 0, end: 0 }).unwrap_err();
        assert!(err.message.contains("InsufficientCohort"));
    }
}
