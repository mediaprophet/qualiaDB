//! `CooperativeDelegation.permits` — fail-closed ABAC over `delegation_permits`.

use qualia_cooperative_core::agency_delegation::{
    delegation_permits, AccessDecision, AccessRequest, AgencyDelegation,
};
use qualia_cooperative_core::agency_domain::agency_domain_taxonomy;
use qualia_cooperative_core::trigger::TriggerContext;
use vibe::{Span, Value};

use super::super::args;
use super::codec::{decode_field, encode_json};

/// Evaluate whether a delegation permits an access request.
///
/// Args (record):
/// - `delegation` — full [`AgencyDelegation`] JSON record
/// - `request` — [`AccessRequest`] JSON record
/// - `context` — optional [`TriggerContext`] (defaults to `now_unix: 0`)
///
/// Result: `{ permitted: bool, decision: "permit"|"deny", reason?: string }`
pub fn permits(args_v: &Value, span: Span) -> Result<Value, vibe::Diagnostic> {
    let delegation: AgencyDelegation =
        decode_field(args_v, "delegation", span, "CooperativeDelegation.permits")?;
    let request: AccessRequest =
        decode_field(args_v, "request", span, "CooperativeDelegation.permits")?;
    let ctx: TriggerContext = match args::rec(args_v, "context") {
        Some(_) => decode_field(args_v, "context", span, "CooperativeDelegation.permits")?,
        None => TriggerContext::at(0),
    };

    let domains = agency_domain_taxonomy();
    let decision = delegation_permits(&delegation, &domains, &request, &ctx);
    decision_value(&decision, span)
}

fn decision_value(decision: &AccessDecision, _span: Span) -> Result<Value, vibe::Diagnostic> {
    match decision {
        AccessDecision::Permit => Ok(args::record([
            ("permitted", Value::Bool(true)),
            ("decision", Value::String("permit".into())),
        ])),
        AccessDecision::Deny(reason) => Ok(args::record([
            ("permitted", Value::Bool(false)),
            ("decision", Value::String("deny".into())),
            ("reason", Value::String(reason.clone())),
        ])),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qualia_cooperative_core::agency_delegation::ConsentState;
    use qualia_cooperative_core::agency_domain::ids as dom;
    use std::collections::BTreeMap;

    fn rec(pairs: &[(&str, Value)]) -> Value {
        let mut m = BTreeMap::new();
        for (k, v) in pairs {
            m.insert((*k).into(), v.clone());
        }
        Value::Record(m)
    }

    fn granted_delegation(domain: &str) -> Value {
        let mut d = AgencyDelegation::new("did:wf:alice", domain, "urn:un:hr:udhr", 100);
        d.consent = ConsentState::Granted;
        d.id = "test-delegation".into();
        encode_json(&d, Span { start: 0, end: 0 }, "test").expect("encode delegation")
    }

    fn read_request(domain: &str) -> Value {
        let req = AccessRequest {
            domain: domain.into(),
            data_class: "x".into(),
            action: "read".into(),
            sphere: Default::default(),
            jurisdiction: None,
            provenance: None,
        };
        encode_json(&req, Span { start: 0, end: 0 }, "test").expect("encode request")
    }

    #[test]
    fn permits_when_valid_and_consented() {
        let args_v = rec(&[
            ("delegation", granted_delegation(dom::PERSONAL_WELFARE)),
            ("request", read_request(dom::PERSONAL_WELFARE)),
            (
                "context",
                rec(&[("now_unix", Value::U64(200))]),
            ),
        ]);
        let out = permits(&args_v, Span { start: 0, end: 0 }).expect("ok");
        let Value::Record(m) = out else {
            panic!("expected record");
        };
        assert_eq!(m.get("permitted"), Some(&Value::Bool(true)));
        assert_eq!(
            m.get("decision"),
            Some(&Value::String("permit".into()))
        );
    }

    #[test]
    fn denies_wrong_domain_fail_closed() {
        let args_v = rec(&[
            ("delegation", granted_delegation(dom::FINANCIAL)),
            ("request", read_request(dom::MEDICAL)),
            (
                "context",
                rec(&[("now_unix", Value::U64(200))]),
            ),
        ]);
        let out = permits(&args_v, Span { start: 0, end: 0 }).expect("ok");
        let Value::Record(m) = out else {
            panic!("expected record");
        };
        assert_eq!(m.get("permitted"), Some(&Value::Bool(false)));
        assert_eq!(m.get("decision"), Some(&Value::String("deny".into())));
        assert!(matches!(m.get("reason"), Some(Value::String(s)) if !s.is_empty()));
    }

    #[test]
    fn missing_delegation_fails_closed() {
        let args_v = rec(&[("request", read_request(dom::PERSONAL_WELFARE))]);
        let err = permits(&args_v, Span { start: 0, end: 0 }).expect_err("needs delegation");
        assert!(err.message.contains("delegation"));
    }
}
