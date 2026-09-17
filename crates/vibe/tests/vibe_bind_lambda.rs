//! P14.9: two-way bind, effect cell, non-capturing lambdas, tween.

use vibe::{parse_program, Budget, Engine, Env, LocalHost, Value};

fn eval_src(src: &str) -> Result<(Value, Env), vibe::Diagnostic> {
    let prog = parse_program(src)?;
    vibe::check_program(&prog)?;
    let mut host = LocalHost::default();
    let mut engine = Engine::with_program(&mut host, Budget::default(), &prog);
    let mut env = Env::default();
    engine.eval_program(&prog, &mut env)?;
    let v = engine.call_function(&prog, "main", vec![], &mut env)?;
    Ok((v, env))
}

#[test]
fn lambda_map_squares() {
    let src = r#"
        fn main() {
            return [1, 2, 3].map(|x| x * x);
        }
    "#;
    let (v, _) = eval_src(src).expect("eval");
    assert_eq!(
        v,
        Value::List(vec![Value::I64(1), Value::I64(4), Value::I64(9)])
    );
}

#[test]
fn lambda_filter_even() {
    let src = r#"
        fn main() {
            return [1, 2, 3, 4].filter(|x| x / 2 * 2 == x);
        }
    "#;
    let (v, _) = eval_src(src).expect("eval");
    assert_eq!(v, Value::List(vec![Value::I64(2), Value::I64(4)]));
}

#[test]
fn lambda_capture_is_e100() {
    let src = r#"
        fn main() {
            let y = 3;
            return [1].map(|x| x + y);
        }
    "#;
    let err = match eval_src(src) {
        Err(e) => e,
        Ok(_) => panic!("capture should fail"),
    };
    assert_eq!(err.code, vibe::DiagCode::E100);
    assert!(err.message.contains("captures"));
}

#[test]
fn tween_linear_settled() {
    let src = r#"
        fn main() {
            return 0.0 ~ 10.0 over 400ms ease linear;
        }
    "#;
    let (v, _) = eval_src(src).expect("eval");
    match v {
        Value::F64(n) => assert!((n - 10.0).abs() < 1e-9, "got {n}"),
        other => panic!("expected f64, got {other}"),
    }
}

#[test]
fn tween_at_t_halfway() {
    let src = r#"
        fn main() {
            let t = 200ms;
            return 0.0 ~ 10.0 over 400ms ease linear;
        }
    "#;
    let (v, _) = eval_src(src).expect("eval");
    match v {
        Value::F64(n) => assert!((n - 5.0).abs() < 1e-9, "got {n}"),
        other => panic!("expected f64, got {other}"),
    }
}

#[test]
fn bind_latest_clamps() {
    let src = r#"
        let volume = 1.5;
        let slider = { value: 0.2 };
        bind slider.value <-> volume using Clamp[0, 1] resolve latest;
    "#;
    let prog = parse_program(src).expect("parse");
    vibe::check_program(&prog).expect("check");
    let mut host = LocalHost::default();
    let mut engine = Engine::with_program(&mut host, Budget::default(), &prog);
    let mut env = Env::default();
    engine.eval_program(&prog, &mut env).expect("eval");
    match env.vars.get("volume") {
        Some(Value::F64(n)) => assert!((n - 1.0).abs() < 1e-9, "got {n}"),
        other => panic!("expected clamped volume, got {other:?}"),
    }
}

#[test]
fn effect_cell_pulse_when() {
    let src = r#"
        requires [ capability("pulse.publish") ];
        cell high := true;
        effect cell shout when high {
            pulse.publish("score/high", { ok: true })
        };
    "#;
    let prog = parse_program(src).expect("parse");
    vibe::check_program(&prog).expect("check");
    let mut host = LocalHost::default();
    let mut engine = Engine::with_program(&mut host, Budget::default(), &prog);
    let mut env = Env::default();
    engine.eval_program(&prog, &mut env).expect("eval");
    assert!(env.vars.contains_key("shout"));
}

#[test]
fn tween_and_lambda_round_trip_cbor() {
    let src = "fn main() { return [1, 2].map(|x| x + 1); }\n";
    let a = parse_program(src).expect("parse");
    let bytes = vibe::encode(&a);
    let b = vibe::decode(&bytes).expect("decode");
    assert_eq!(vibe::encode(&b), bytes);
}
