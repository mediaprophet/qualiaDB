use super::*;

fn rec(pairs: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    args::record(pairs)
}

fn span() -> Span {
    Span { start: 0, end: 0 }
}

#[test]
fn bytecode_and_arena_use_native_constants() {
    let compiled = compute(
        &rec([
            ("mode", Value::String("bytecode".into())),
            (
                "source",
                Value::String("<did:alice> <schema:knows> ?who .".into()),
            ),
        ]),
        span(),
    )
    .unwrap();
    let Value::Record(result) = compiled else {
        panic!("expected record")
    };
    assert!(matches!(result.get("matches"), Some(Value::U64(n)) if *n >= 1));

    let arena = compute(
        &rec([
            ("mode", Value::String("slg_arena".into())),
            ("used_slots", Value::U64(917)),
        ]),
        span(),
    )
    .unwrap();
    let Value::Record(result) = arena else {
        panic!("expected record")
    };
    assert_eq!(result.get("max_slots"), Some(&Value::U64(917_504)));
}

#[test]
fn gemm_tokenizer_and_zero_sensitivity_dp_are_native() {
    let gemm = compute(
        &rec([
            ("mode", Value::String("forge".into())),
            ("operation", Value::String("gemm".into())),
            ("m", Value::U64(2)),
            ("k", Value::U64(2)),
            ("n", Value::U64(2)),
            ("a", args::f64_list_value([1.0, 0.0, 0.0, 1.0])),
            ("b", args::f64_list_value([2.0, 3.0, 4.0, 5.0])),
        ]),
        span(),
    )
    .unwrap();
    let Value::Record(result) = gemm else {
        panic!("expected record")
    };
    assert_eq!(
        result.get("data"),
        Some(&args::f64_list_value([2.0, 3.0, 4.0, 5.0]))
    );

    let tokens = compute(
        &rec([
            ("mode", Value::String("gguf_tokenizer".into())),
            ("text", Value::String("Hi".into())),
        ]),
        span(),
    )
    .unwrap();
    let Value::Record(result) = tokens else {
        panic!("expected record")
    };
    assert_eq!(result.get("vocab_size"), Some(&Value::U64(256)));

    let dp = compute(
        &rec([
            ("mode", Value::String("privacy".into())),
            ("operation", Value::String("dp_laplace".into())),
            ("plaintext", args::f64_list_value([4.0, 5.0])),
            ("sensitivity", Value::F64(0.0)),
            ("epsilon", Value::F64(0.5)),
        ]),
        span(),
    )
    .unwrap();
    let Value::Record(result) = dp else {
        panic!("expected record")
    };
    assert_eq!(
        result.get("released"),
        Some(&args::f64_list_value([4.0, 5.0]))
    );
}

#[test]
fn missing_decode_session_and_unknown_mode_fail_closed() {
    assert!(compute(
        &rec([("mode", Value::String("inference_monitor".into()))]),
        span()
    )
    .is_err());
    assert!(compute(&rec([("mode", Value::String("mock".into()))]), span()).is_err());
}
