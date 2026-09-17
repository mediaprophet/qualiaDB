use super::*;

fn rec(pairs: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    args::record(pairs)
}

fn span() -> Span {
    Span { start: 0, end: 0 }
}

#[test]
fn bayesian_update_and_interval_relation_are_real() {
    let posterior = compute(
        &rec([
            ("mode", Value::String("probabilistic".into())),
            ("prior", Value::F64(0.2)),
            ("likelihood_true", Value::F64(0.9)),
            ("likelihood_false", Value::F64(0.1)),
            ("threshold", Value::F64(0.5)),
        ]),
        span(),
    )
    .unwrap();
    match posterior {
        Value::Record(r) => assert_eq!(r.get("meets_threshold"), Some(&Value::Bool(true))),
        _ => panic!(),
    }

    let interval = compute(
        &rec([
            ("mode", Value::String("interval".into())),
            ("a", Value::List(vec![Value::I64(0), Value::I64(10)])),
            ("b", Value::List(vec![Value::I64(5), Value::I64(15)])),
        ]),
        span(),
    )
    .unwrap();
    match interval {
        Value::Record(r) => assert_eq!(
            r.get("allen_relation"),
            Some(&Value::String("Overlaps".into()))
        ),
        _ => panic!(),
    }
}

#[test]
fn modal_frame_is_evaluated() {
    let v = compute(
        &rec([
            ("mode", Value::String("modal".into())),
            ("system", Value::String("K".into())),
            ("operator", Value::String("necessary".into())),
            ("world", Value::String("w0".into())),
            ("proposition", Value::String("integrity".into())),
            (
                "worlds",
                Value::List(vec![Value::String("w0".into()), Value::String("w1".into())]),
            ),
            (
                "accesses",
                Value::List(vec![Value::List(vec![
                    Value::String("w0".into()),
                    Value::String("w1".into()),
                ])]),
            ),
            ("holds_in", Value::List(vec![Value::String("w1".into())])),
        ]),
        span(),
    )
    .unwrap();
    match v {
        Value::Record(r) => assert_eq!(r.get("truth"), Some(&Value::Bool(true))),
        _ => panic!(),
    }
}
