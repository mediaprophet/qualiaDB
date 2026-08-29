use super::*;

fn rec(pairs: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    args::record(pairs)
}

fn span() -> Span {
    Span { start: 0, end: 0 }
}

#[test]
fn crdt_owl_and_qubo_use_native_cores() {
    let lww = compute(
        &rec([
            ("mode", Value::String("crdt".into())),
            ("local_clock", Value::U64(1)),
            ("remote_clock", Value::U64(2)),
            ("local_object", Value::U64(10)),
            ("remote_object", Value::U64(20)),
            ("expiry", Value::U64(100)),
            ("now", Value::U64(1)),
        ]),
        span(),
    )
    .unwrap();
    let Value::Record(result) = lww else {
        panic!("expected record")
    };
    assert_eq!(result.get("winner_clock"), Some(&Value::U64(2)));
    assert_eq!(result.get("delegation_valid"), Some(&Value::Bool(true)));

    let owl = compute(
        &rec([
            ("mode", Value::String("owl".into())),
            (
                "triples",
                Value::List(
                    ["Student:subClassOf:Person", "Person:subClassOf:Agent"]
                        .into_iter()
                        .map(|triple| Value::String(triple.into()))
                        .collect(),
                ),
            ),
        ]),
        span(),
    )
    .unwrap();
    let Value::Record(result) = owl else {
        panic!("expected record")
    };
    assert_eq!(result.get("consistent"), Some(&Value::Bool(true)));
    assert!(matches!(result.get("inferred"), Some(Value::U64(n)) if *n >= 1));

    let qubo = compute(
        &rec([
            ("mode", Value::String("qubo".into())),
            ("edges", Value::List(vec![Value::String("a:b".into())])),
        ]),
        span(),
    )
    .unwrap();
    let Value::Record(result) = qubo else {
        panic!("expected record")
    };
    assert_eq!(result.get("variables"), Some(&Value::U64(2)));
}

#[test]
fn likeliness_pid_and_carrier_are_native() {
    let like = compute(
        &rec([
            ("mode", Value::String("likeliness".into())),
            ("premises", args::f64_list_value([2.0, 1.0, -1.0])),
        ]),
        span(),
    )
    .unwrap();
    let Value::Record(result) = like else {
        panic!("expected record")
    };
    assert_eq!(result.get("level"), Some(&Value::I64(-1)));

    let pid = compute(
        &rec([
            ("mode", Value::String("control_feedback".into())),
            ("setpoint", Value::F64(100.0)),
            ("measured", Value::F64(95.0)),
            ("t", Value::U64(1)),
        ]),
        span(),
    )
    .unwrap();
    let Value::Record(result) = pid else {
        panic!("expected record")
    };
    assert!(matches!(result.get("output"), Some(Value::F64(v)) if v.is_finite()));

    let bound = compute(
        &rec([
            ("mode", Value::String("carrier".into())),
            ("payload", Value::String("hello".into())),
        ]),
        span(),
    )
    .unwrap();
    let Value::Record(result) = bound else {
        panic!("expected record")
    };
    assert_eq!(result.get("binding_valid"), Some(&Value::Bool(true)));
}

#[test]
fn unknown_mode_fails_closed() {
    assert!(compute(&rec([("mode", Value::String("mock".into()))]), span()).is_err());
}
