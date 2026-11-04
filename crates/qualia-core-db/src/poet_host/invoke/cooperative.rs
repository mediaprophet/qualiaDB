//! Host bindings for `qualia-cooperative-core` and the 6 Core Semantic Instruments (`WP-CP-02B`).
//!
//! Exposes:
//! - `CooperativeDelegation.permits` ([`permits`])
//! - `CooperativeWork.board_project` ([`board_project`])
//! - `CooperativeLifecycle.evaluate` ([`evaluate_lifecycle`])
//! - `CooperativeAssertions.review` ([`review_assertion`])
//! - `CooperativeInstruments.evaluate` ([`evaluate_instrument_readiness`])
//! - `CooperativePool.clear_bps` ([`clear_pool_bps`])

mod codec;
mod permits;
mod work;

pub use permits::permits;
pub use work::board_project;

use vibe::{Span, Value};

use super::args;
use crate::modalities::socioeconomics::clear_pool_obligations_bps;
use crate::semantic_instruments::{
    evaluate_contextual_assertions, evaluate_contribution_lifecycle,
    evaluate_core_semantic_instrument, execute_reference_instrument, ContextualAssertionsReview,
    ContributionLifecycleCompleteness, CoreInstrumentKind, CoreInstrumentPayload,
    InstrumentEvaluationReport,
};
use codec::{decode_field, encode_json, vibe_to_json};

fn encode_report(
    report: &InstrumentEvaluationReport,
    span: Span,
    what: &str,
) -> Result<Value, vibe::Diagnostic> {
    let findings_v = encode_json(&report.findings, span, what)?;
    Ok(args::record([
        ("instrument", Value::String(report.instrument.slug().into())),
        ("outcome", Value::String(report.outcome.as_str().into())),
        ("satisfied", Value::Bool(report.outcome.is_satisfied())),
        ("findings", findings_v),
        ("receipt_subject", Value::U64(report.receipt_quin.subject)),
        (
            "receipt_parity_ok",
            Value::Bool(report.receipt_quin.verify_ecc_parity()),
        ),
    ]))
}

/// `CooperativeLifecycle.evaluate` — evaluate 9-stage contribution lifecycle completeness
/// (`Activity != Payable Debt` and `Recognition != Settlement`).
pub fn evaluate_lifecycle(args_v: &Value, span: Span) -> Result<Value, vibe::Diagnostic> {
    let input: ContributionLifecycleCompleteness = match args::rec(args_v, "lifecycle") {
        Some(_) => decode_field(args_v, "lifecycle", span, "CooperativeLifecycle.evaluate")?,
        None => {
            let json = vibe_to_json(args_v).ok_or_else(|| {
                args::bad(span, "CooperativeLifecycle.evaluate: invalid argument record")
            })?;
            serde_json::from_value(json).map_err(|e| {
                args::bad(span, format!("CooperativeLifecycle.evaluate: decode failed ({e})"))
            })?
        }
    };
    let now_unix = args::rec_u64(args_v, "now_unix").unwrap_or(0) as u32;
    let report = evaluate_contribution_lifecycle(&input, now_unix);
    encode_report(&report, span, "CooperativeLifecycle.evaluate")
}

/// `CooperativeAssertions.review` — evaluate contextual assertions review
/// (`Observation`, `Inference`, `Allegation`, `Attestation`, `AdjudicatedFinding`).
pub fn review_assertion(args_v: &Value, span: Span) -> Result<Value, vibe::Diagnostic> {
    let input: ContextualAssertionsReview = match args::rec(args_v, "assertion") {
        Some(_) => decode_field(args_v, "assertion", span, "CooperativeAssertions.review")?,
        None => {
            let json = vibe_to_json(args_v).ok_or_else(|| {
                args::bad(span, "CooperativeAssertions.review: invalid argument record")
            })?;
            serde_json::from_value(json).map_err(|e| {
                args::bad(span, format!("CooperativeAssertions.review: decode failed ({e})"))
            })?
        }
    };
    let now_unix = args::rec_u64(args_v, "now_unix").unwrap_or(0) as u32;
    let report = evaluate_contextual_assertions(&input, now_unix);
    encode_report(&report, span, "CooperativeAssertions.review")
}

/// `CooperativeInstruments.evaluate` — evaluate any of the 6 Core Semantic Instruments
/// by `instrument` slug/IRI + `input` record, or via tagged [`CoreInstrumentPayload`].
pub fn evaluate_instrument_readiness(args_v: &Value, span: Span) -> Result<Value, vibe::Diagnostic> {
    let now_unix = args::rec_u64(args_v, "now_unix").unwrap_or(0) as u32;
    if let Some(slug) = args::rec_str(args_v, "instrument") {
        if let Some(input_v) = args::rec(args_v, "input") {
            let kind = CoreInstrumentKind::from_slug_or_id(slug).ok_or_else(|| {
                args::bad(
                    span,
                    format!("CooperativeInstruments.evaluate: unknown instrument `{slug}`"),
                )
            })?;
            let json = vibe_to_json(input_v).ok_or_else(|| {
                args::bad(span, "CooperativeInstruments.evaluate: invalid `input` value")
            })?;
            let bytes = serde_json::to_vec(&json).map_err(|e| {
                args::bad(span, format!("CooperativeInstruments.evaluate: {e}"))
            })?;
            let report = execute_reference_instrument(kind, &bytes, now_unix)
                .map_err(|e| args::bad(span, format!("CooperativeInstruments.evaluate: {e}")))?;
            return encode_report(&report, span, "CooperativeInstruments.evaluate");
        }
    }
    let payload: CoreInstrumentPayload = match args::rec(args_v, "payload") {
        Some(_) => decode_field(args_v, "payload", span, "CooperativeInstruments.evaluate")?,
        None => {
            let json = vibe_to_json(args_v).ok_or_else(|| {
                args::bad(span, "CooperativeInstruments.evaluate: invalid payload record")
            })?;
            serde_json::from_value(json).map_err(|e| {
                args::bad(
                    span,
                    format!("CooperativeInstruments.evaluate: decode failed ({e})"),
                )
            })?
        }
    };
    let report = evaluate_core_semantic_instrument(&payload, now_unix);
    encode_report(&report, span, "CooperativeInstruments.evaluate")
}

/// `CooperativePool.clear_bps` — zero-heap basis-point clearing verifying `Pool Funded != Contributor Paid`.
pub fn clear_pool_bps(args_v: &Value, span: Span) -> Result<Value, vibe::Diagnostic> {
    let gross = args::rec_i64(args_v, "gross_receipt_cents").unwrap_or(0);
    let reserve_bps = args::rec_u64(args_v, "operating_reserve_bps").unwrap_or(0) as u32;
    let fee_bps = args::rec_u64(args_v, "fiduciary_fee_bps").unwrap_or(0) as u32;
    let prior_pool = args::rec_i64(args_v, "prior_pool_funded_cents").unwrap_or(0);
    let target = args::rec_i64(args_v, "target_obligation_cents").unwrap_or(0);
    let settled = args::rec_i64(args_v, "settled_verified_cents").unwrap_or(0);
    let waived = args::rec_i64(args_v, "waived_obligation_cents").unwrap_or(0);
    let res = clear_pool_obligations_bps(
        gross,
        reserve_bps,
        fee_bps,
        prior_pool,
        target,
        settled,
        waived,
    )
    .ok_or_else(|| args::bad(span, "CooperativePool.clear_bps: invalid negative or >10000 bps inputs"))?;
    Ok(args::record([
        ("gross_receipt_cents", Value::I64(res.gross_receipt_cents)),
        ("operating_reserve_cents", Value::I64(res.operating_reserve_cents)),
        ("fiduciary_fee_cents", Value::I64(res.fiduciary_fee_cents)),
        (
            "obligation_pool_allocation_cents",
            Value::I64(res.obligation_pool_allocation_cents),
        ),
        ("pool_funded_cents", Value::I64(res.pool_funded_cents)),
        ("settled_verified_cents", Value::I64(res.settled_verified_cents)),
        ("remaining_obligation_cents", Value::I64(res.remaining_obligation_cents)),
        (
            "contributor_obligations_discharged",
            Value::Bool(res.contributor_obligations_discharged),
        ),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn rec(pairs: &[(&str, Value)]) -> Value {
        let mut m = BTreeMap::new();
        for (k, v) in pairs {
            m.insert((*k).into(), v.clone());
        }
        Value::Record(m)
    }

    #[test]
    fn host_evaluates_lifecycle_and_assertion_and_instrument() {
        let span = Span { start: 0, end: 0 };
        let lc_args = rec(&[
            ("is_commissioned", Value::Bool(false)),
            ("is_accepted", Value::Bool(false)),
            ("has_valuation", Value::Bool(false)),
            ("obligation_cents", Value::I64(10_000)),
            ("settled_verified_cents", Value::I64(0)),
            ("is_recognition_badge_only", Value::Bool(true)),
        ]);
        let lc_out = evaluate_lifecycle(&lc_args, span).unwrap();
        let Value::Record(m) = lc_out else { panic!("expected record") };
        assert_eq!(m.get("outcome"), Some(&Value::String("failed".into())));

        let assert_args = rec(&[
            ("assertion_kind", Value::String("allegation".into())),
            ("claimed_target_kind", Value::String("adjudicated_finding".into())),
            ("distinct_root_sources", Value::U64(1)),
            ("dependent_inference_count", Value::U64(2)),
            ("has_human_adjudicator_did", Value::Bool(false)),
            ("scope_domain", Value::String("project:alpha".into())),
        ]);
        let assert_out = review_assertion(&assert_args, span).unwrap();
        let Value::Record(am) = assert_out else { panic!("expected record") };
        assert_eq!(am.get("outcome"), Some(&Value::String("failed".into())));

        let ready_args = rec(&[
            (
                "instrument",
                Value::String("licence-transition-readiness".into()),
            ),
            (
                "input",
                rec(&[
                    ("project_id", Value::String("proj:1".into())),
                    ("pool_funded_cents", Value::I64(100_000)),
                    ("required_obligation_cents", Value::I64(100_000)),
                    ("settled_verified_cents", Value::I64(40_000)),
                    ("dependencies_compatible", Value::Bool(true)),
                    ("unresolved_material_challenges", Value::U64(0)),
                    (
                        "authorised_human_steward_did",
                        Value::String("did:webizen:person:steward".into()),
                    ),
                ]),
            ),
        ]);
        let ready_out = evaluate_instrument_readiness(&ready_args, span).unwrap();
        let Value::Record(rm) = ready_out else { panic!("expected record") };
        assert_eq!(rm.get("outcome"), Some(&Value::String("held".into())));
    }
}
