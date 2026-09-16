//! Integration tests for Prompt Precision Wave 8 (P6: PP-080, PP-081).
//!
//! Validates Host dispatch lockstep, profile registry promotion/rollback,
//! and privacy-redacted inspector traces.

use qualia_core_db::poet_host::invoke::{dispatch, ids};
use qualia_core_db::poet_host::PoetSnapshot;
use std::collections::BTreeMap;
use vibe::{Span, Value};

fn mock_profile_value(pid: &str, valid: bool) -> Value {
    let mut rec = BTreeMap::new();
    rec.insert("schema_version".into(), Value::U64(1));
    rec.insert("profile_id".into(), Value::String(pid.into()));
    rec.insert(
        "objective".into(),
        Value::String("Review code for bounded resources".into()),
    );
    rec.insert(
        "domains".into(),
        Value::List(vec![Value::String("urn:qualia:domain:systems".into())]),
    );

    let mut req1 = BTreeMap::new();
    req1.insert("id".into(), Value::String("R1".into()));
    req1.insert(
        "class".into(),
        Value::String(if valid { "enforced" } else { "enforced" }.into()),
    );
    req1.insert(
        "rule".into(),
        Value::String("Zero heap allocation in hot paths".into()),
    );
    if valid {
        req1.insert(
            "validator".into(),
            Value::String("zero-alloc-suite".into()),
        );
    } else {
        // Enforced requirement missing validator must fail validation
    }
    req1.insert("required".into(), Value::Bool(true));
    req1.insert("priority".into(), Value::U64(255));

    rec.insert("requirements".into(), Value::List(vec![Value::Record(req1)]));

    let mut budget = BTreeMap::new();
    budget.insert("input_tokens".into(), Value::U64(4096));
    budget.insert("output_tokens".into(), Value::U64(1024));
    budget.insert("tool_rounds".into(), Value::U64(4));
    rec.insert("budget".into(), Value::Record(budget));

    Value::Record(rec)
}

#[test]
fn pp080_conditioning_validate_structured_violations() {
    let mut snap = PoetSnapshot::default();
    let span = Span { start: 0, end: 0 };

    // Valid profile
    let valid_arg = mock_profile_value("urn:qualia:profile:rust-review:v1", true);
    let val_res = dispatch(&mut snap, ids::CONDITIONING_VALIDATE, &valid_arg, span)
        .expect("validate must succeed");
    match val_res {
        Value::Record(m) => {
            assert_eq!(m.get("valid"), Some(&Value::Bool(true)));
            assert_eq!(
                m.get("profile_id"),
                Some(&Value::String("urn:qualia:profile:rust-review:v1".into()))
            );
            assert_eq!(m.get("violations"), Some(&Value::List(Vec::new())));
            assert!(matches!(m.get("digest"), Some(Value::U64(h)) if *h > 0));
        }
        other => panic!("expected record, got {other:?}"),
    }

    // Invalid profile (enforced requirement missing validator)
    let invalid_arg = mock_profile_value("urn:qualia:profile:invalid:v1", false);
    let inval_res = dispatch(&mut snap, ids::CONDITIONING_VALIDATE, &invalid_arg, span)
        .expect("validate call must not panic");
    match inval_res {
        Value::Record(m) => {
            assert_eq!(m.get("valid"), Some(&Value::Bool(false)));
            if let Some(Value::List(violations)) = m.get("violations") {
                assert!(!violations.is_empty(), "expected structured violations");
            } else {
                panic!("violations must be a List");
            }
        }
        other => panic!("expected record, got {other:?}"),
    }
}

#[test]
fn pp080_conditioning_compile_receipts_and_budgets() {
    let mut snap = PoetSnapshot::default();
    let span = Span { start: 0, end: 0 };

    let valid_arg = mock_profile_value("urn:qualia:profile:rust-compile:v1", true);
    let compile_res = dispatch(&mut snap, ids::CONDITIONING_COMPILE, &valid_arg, span)
        .expect("compile must succeed");

    match compile_res {
        Value::Record(m) => {
            assert!(matches!(m.get("plan_id"), Some(Value::U64(id)) if *id > 0));
            assert_eq!(
                m.get("profile_id"),
                Some(&Value::String("urn:qualia:profile:rust-compile:v1".into()))
            );
            assert_eq!(m.get("requirements_count"), Some(&Value::U64(1)));
            assert_eq!(m.get("input_token_budget"), Some(&Value::U64(4096)));
            assert_eq!(m.get("output_token_budget"), Some(&Value::U64(1024)));
        }
        other => panic!("expected record, got {other:?}"),
    }
}

#[test]
fn pp081_conditioning_inspector_privacy_redaction() {
    let mut snap = PoetSnapshot::default();
    let span = Span { start: 0, end: 0 };

    let valid_arg = mock_profile_value("urn:qualia:profile:inspect:v1", true);
    let inspect_res = dispatch(&mut snap, ids::CONDITIONING_INSPECT, &valid_arg, span)
        .expect("inspect must succeed");

    match inspect_res {
        Value::Record(m) => {
            assert_eq!(
                m.get("privacy_redaction_applied"),
                Some(&Value::Bool(true)),
                "trace must explicitly confirm privacy redaction"
            );
            assert_eq!(
                m.get("profile_id"),
                Some(&Value::String("urn:qualia:profile:inspect:v1".into()))
            );
            assert_eq!(m.get("evidence_count"), Some(&Value::U64(0)));
            assert_eq!(m.get("disclosure_ceiling"), Some(&Value::U64(0)));

            if let Some(Value::List(reqs)) = m.get("requirements") {
                assert_eq!(reqs.len(), 1);
                if let Value::Record(rm) = &reqs[0] {
                    assert_eq!(rm.get("id"), Some(&Value::String("R1".into())));
                    assert_eq!(rm.get("class"), Some(&Value::String("enforced".into())));
                }
            } else {
                panic!("requirements must be a List");
            }
        }
        other => panic!("expected record, got {other:?}"),
    }
}

#[test]
fn pp081_conditioning_activation_and_rollback() {
    let mut snap = PoetSnapshot::default();
    let span = Span { start: 0, end: 0 };
    let pid = "urn:qualia:profile:lifecycle-test:v1";

    // 1. Activate version 1
    let mut act_arg1 = BTreeMap::new();
    act_arg1.insert("profile_id".into(), Value::String(pid.into()));
    act_arg1.insert("version".into(), Value::U64(1));

    let act_res1 = dispatch(
        &mut snap,
        ids::CONDITIONING_ACTIVATE,
        &Value::Record(act_arg1),
        span,
    )
    .expect("activate v1 must succeed");
    match act_res1 {
        Value::Record(m) => {
            assert_eq!(m.get("active_version"), Some(&Value::U64(1)));
            assert_eq!(m.get("previous_version"), Some(&Value::Null));
            assert_eq!(m.get("activated"), Some(&Value::Bool(true)));
        }
        other => panic!("expected record, got {other:?}"),
    }

    // 2. Promote to version 2
    let mut act_arg2 = BTreeMap::new();
    act_arg2.insert("profile_id".into(), Value::String(pid.into()));
    act_arg2.insert("version".into(), Value::U64(2));

    let act_res2 = dispatch(
        &mut snap,
        ids::CONDITIONING_ACTIVATE,
        &Value::Record(act_arg2),
        span,
    )
    .expect("activate v2 must succeed");
    match act_res2 {
        Value::Record(m) => {
            assert_eq!(m.get("active_version"), Some(&Value::U64(2)));
            assert_eq!(m.get("previous_version"), Some(&Value::U64(1)));
        }
        other => panic!("expected record, got {other:?}"),
    }

    // 3. Rollback to prior approved version
    let mut roll_arg = BTreeMap::new();
    roll_arg.insert("profile_id".into(), Value::String(pid.into()));

    let roll_res = dispatch(
        &mut snap,
        ids::CONDITIONING_ROLLBACK,
        &Value::Record(roll_arg),
        span,
    )
    .expect("rollback must succeed");
    match roll_res {
        Value::Record(m) => {
            assert_eq!(m.get("from_version"), Some(&Value::U64(2)));
            assert_eq!(m.get("active_version"), Some(&Value::U64(1)));
            assert_eq!(m.get("rolled_back"), Some(&Value::Bool(true)));
        }
        other => panic!("expected record, got {other:?}"),
    }
}

#[test]
fn pp080_conditioning_evaluate_bounded_campaign() {
    let mut snap = PoetSnapshot::default();
    let span = Span { start: 0, end: 0 };

    let mut eval_arg = BTreeMap::new();
    eval_arg.insert("split".into(), Value::String("test".into()));
    eval_arg.insert(
        "candidate_output".into(),
        Value::String("fn test_case() -> bool { true }".into()),
    );

    let eval_res = dispatch(
        &mut snap,
        ids::CONDITIONING_EVALUATE,
        &Value::Record(eval_arg),
        span,
    )
    .expect("evaluate must succeed");
    match eval_res {
        Value::Record(m) => {
            assert_eq!(m.get("split"), Some(&Value::String("test".into())));
            assert_eq!(m.get("accuracy"), Some(&Value::F64(1.0)));
            assert_eq!(m.get("tasks_evaluated"), Some(&Value::U64(1)));
        }
        other => panic!("expected record, got {other:?}"),
    }
}
