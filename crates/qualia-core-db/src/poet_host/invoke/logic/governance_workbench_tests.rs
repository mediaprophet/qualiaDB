use super::*;

fn rec(pairs: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    args::record(pairs)
}

fn span() -> Span {
    Span { start: 0, end: 0 }
}

#[test]
fn value_flow_and_gap_use_native_cores() {
    let flow = compute(
        &rec([
            ("mode", Value::String("value_flow".into())),
            ("production_cost", Value::U64(1000)),
            ("roi_cap_percent", Value::U64(20)),
            ("max_roi_percent", Value::U64(20)),
            ("pool", Value::U64(400)),
        ]),
        span(),
    )
    .unwrap();
    let Value::Record(result) = flow else {
        panic!("expected record")
    };
    assert_eq!(result.get("commons_cost"), Some(&Value::U64(1200)));
    assert_eq!(result.get("outstanding"), Some(&Value::U64(800)));

    let gap = compute(
        &rec([
            ("mode", Value::String("capability_gap".into())),
            (
                "required",
                Value::List(
                    ["rust", "wasm", "sparql"]
                        .into_iter()
                        .map(|name| Value::String(name.into()))
                        .collect(),
                ),
            ),
            (
                "held",
                Value::List(
                    ["rust", "wasm"]
                        .into_iter()
                        .map(|name| Value::String(name.into()))
                        .collect(),
                ),
            ),
        ]),
        span(),
    )
    .unwrap();
    let Value::Record(result) = gap else {
        panic!("expected record")
    };
    assert_eq!(result.get("gap_count"), Some(&Value::U64(1)));
    assert_eq!(result.get("requirements_met"), Some(&Value::Bool(false)));
}

#[test]
fn interaction_and_mens_rea_are_native() {
    let verdict = compute(
        &rec([
            ("mode", Value::String("interaction".into())),
            ("status", Value::String("violated".into())),
            ("non_derogable", Value::Bool(true)),
        ]),
        span(),
    )
    .unwrap();
    let Value::Record(result) = verdict else {
        panic!("expected record")
    };
    assert_eq!(
        result.get("action"),
        Some(&Value::String("DenyRollback".into()))
    );

    let mens = compute(
        &rec([
            ("mode", Value::String("deontic_compose".into())),
            ("opcode", Value::String("forbid".into())),
            ("brought_about", Value::Bool(true)),
            ("knows", Value::Bool(true)),
        ]),
        span(),
    )
    .unwrap();
    let Value::Record(result) = mens else {
        panic!("expected record")
    };
    assert_eq!(
        result.get("mens_rea"),
        Some(&Value::String("Knowing".into()))
    );
}

#[test]
fn unknown_mode_fails_closed() {
    let err = compute(&rec([("mode", Value::String("fabricated".into()))]), span());
    assert!(err.is_err());
}
