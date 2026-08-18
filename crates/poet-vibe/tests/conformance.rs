//! vibe-0.1 §12 / §13 fixtures.

use poet_vibe::{
    check_cell, check_program, eval_cell, eval_function, load_program, parse_cell, parse_program,
    DiagCode, Env, MockHost, Value,
};

const CELL: &str = r#"= math.max(0, math.min(100, score))"#;

const CLINIC: &str = r#"
module <https://qualiadb.org/modules/clinic_alerts>;
prefix clinic: <https://qualiadb.org/clinic/>;
prefix snomed: <http://snomed.info/id/>;

requires [
    capability("graph.read", context: clinic:telemetry),
    capability("graph.write", context: clinic:alerts),
    capability("aura.validate"),
    capability("pulse.publish", topic: "clinic/alerts")
];

effect fn raise_alert(sensor: Iri, value: f64) budget(steps: 20000) -> Result<Receipt, string> {
    if value <= 85.0 {
        return Ok(receipt_empty());
    }

    let proposition = <<( sensor clinic:emitsAlert clinic:Overheat )>>;
    let stated = << sensor clinic:emitsAlert clinic:Overheat ~ clinic:claim_1 >>;

    transaction {
        graph.stage(stated);
        aura.validate(clinic:claim_1, clinic:EmergencyAlertShape)?;
        graph.commit()?;
    };

    effect pulse.publish("clinic/alerts", { sensor: sensor, value: value })?;
    return Ok(receipt_empty());
}

on pulse:message(topic: string, value: f64) budget(steps: 20000) -> Result<Receipt, string> {
    return raise_alert(clinic:sensor_1, value);
}
"#;

const COUNT: &str = r#"
requires [
    capability("graph.read")
];

pure fn count_conditions(kind: Iri) -> Result<i64, string> {
    let rows = graph.query(?s, clinic:hasCondition, kind, take: 64)?;
    let mut n: i64 = 0;
    for row in rows {
        n = n + 1;
    }
    return Ok(n);
}
"#;

#[test]
fn language_version() {
    assert_eq!(poet_vibe::LANGUAGE_VERSION, "vibe-0.1");
}

#[test]
fn section_12_1_cell_parses_and_evals() {
    parse_cell(CELL).expect("parse cell");
    check_cell(&parse_cell(CELL).unwrap()).expect("check cell");
    let mut host = MockHost::default();
    let mut env = Env::default();
    env.vars.insert("score".into(), Value::I64(42));
    let v = eval_cell(CELL, &mut host, &mut env).unwrap();
    assert_eq!(v.as_f64(), Some(42.0));
    env.vars.insert("score".into(), Value::I64(150));
    let v = eval_cell(CELL, &mut host, &mut env).unwrap();
    assert_eq!(v.as_f64(), Some(100.0));
    env.vars.insert("score".into(), Value::I64(-3));
    let v = eval_cell(CELL, &mut host, &mut env).unwrap();
    assert_eq!(v.as_f64(), Some(0.0));
}

#[test]
fn section_12_2_clinic_module_loads() {
    parse_program(CLINIC).unwrap_or_else(|e| panic!("{e}"));
    check_program(&parse_program(CLINIC).unwrap()).unwrap_or_else(|e| panic!("{e}"));
    let program = load_program(CLINIC).unwrap();
    let mut host = MockHost::default();
    let mut env = Env::default();
    let below = eval_function(
        &program,
        "raise_alert",
        vec![Value::Iri("sensor:1".into()), Value::F64(10.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert!(matches!(below, Value::Ok(_)));
    assert_eq!(host.committed, 0);

    let over = eval_function(
        &program,
        "raise_alert",
        vec![Value::Iri("sensor:1".into()), Value::F64(90.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert!(matches!(over, Value::Ok(_)));
    assert_eq!(host.committed, 1);
    assert_eq!(host.published, vec!["clinic/alerts".to_string()]);
}

#[test]
fn section_12_3_bounded_query() {
    let program = load_program(COUNT).unwrap_or_else(|e| panic!("{e}"));
    let mut host = MockHost {
        query_rows: 3,
        ..MockHost::default()
    };
    let mut env = Env::default();
    let v = eval_function(
        &program,
        "count_conditions",
        vec![Value::Iri("snomed:x".into())],
        &mut host,
        &mut env,
    )
    .unwrap();
    match v {
        Value::Ok(inner) => assert_eq!(inner.as_i64(), Some(3)),
        other => panic!("{other:?}"),
    }
}

#[test]
fn n1_relational_lt_without_spaces() {
    let err = parse_program("let x = a<b;").unwrap_err();
    assert_eq!(err.code, DiagCode::E001);
}

#[test]
fn n2_illegal_reifier_pipe() {
    let err = parse_program("let x = << id | s p o >>;").unwrap_err();
    assert_eq!(err.code, DiagCode::E001);
}

#[test]
fn n3_raw_quin_literal() {
    let err = parse_program("let x = <<[ s p o g prov ]>>;").unwrap_err();
    assert_eq!(err.code, DiagCode::E001);
    assert!(err.message.contains("quin.statement"));
}

#[test]
fn n4_external_in_pure_cell() {
    let expr = parse_cell(r#"= pulse.publish("t", 1)"#).unwrap();
    let err = check_cell(&expr).unwrap_err();
    assert_eq!(err.code, DiagCode::E200);
}

#[test]
fn n5_unbounded_while() {
    let err = load_program("fn loop() { while true { } }").unwrap_err();
    assert_eq!(err.code, DiagCode::E400);
}

#[test]
fn n6_commit_without_capability() {
    let err = load_program("effect fn go() { graph.commit(); }").unwrap_err();
    assert_eq!(err.code, DiagCode::E300);
}

#[test]
fn n7_tick_must_not_query() {
    let src = "on tick() { graph.query(?s, ?p, ?o, take: 1); }";
    let err = load_program(src).unwrap_err();
    assert_eq!(err.code, DiagCode::E200);
}

#[test]
fn quin_statement_seals_without_parity_field() {
    let src = r#"
requires [ capability("graph.read") ];
fn make() {
    return quin.statement(
        subject: <https://example.org/s>,
        predicate: <https://example.org/p>,
        object: <https://example.org/o>,
        context: <https://example.org/g>
    );
}
"#;
    let program = load_program(src).unwrap();
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(&program, "make", vec![], &mut host, &mut env).unwrap();
    assert!(matches!(v, Value::Quin { .. }));
}

#[test]
fn capability_resolve_is_pure_and_returns_a_record() {
    let src = r#"
fn peek() {
    return capability.resolve("graph.read");
}
"#;
    let program = load_program(src).unwrap();
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(&program, "peek", vec![], &mut host, &mut env).unwrap();
    match v {
        Value::Record(r) => assert_eq!(r.get("id"), Some(&Value::String("graph.read".into()))),
        other => panic!("{other:?}"),
    }
}

#[test]
fn time_unix_is_external_and_forbidden_in_pure_cell() {
    // Pure cell must not reach the clock (core §11 / §5).
    let expr = parse_cell("= time.unix()").unwrap();
    let err = check_cell(&expr).unwrap_err();
    assert_eq!(err.code, DiagCode::E200);
}

#[test]
fn time_unix_runs_in_effect_fn() {
    let src = "effect fn now() { return time.unix(); }";
    let program = load_program(src).unwrap();
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(&program, "now", vec![], &mut host, &mut env).unwrap();
    assert_eq!(v.as_i64(), Some(0));
}

#[test]
fn fixtures_on_disk_match_section_12_and_13() {
    let cell = include_str!("../fixtures/12_1_cell.vibe");
    parse_cell(cell).expect("12.1 fixture");
    load_program(include_str!("../fixtures/12_2_clinic.vibe")).expect("12.2 fixture");
    load_program(include_str!("../fixtures/12_3_count.vibe")).expect("12.3 fixture");
    assert_eq!(
        parse_program(include_str!("../fixtures/n1_nospace_lt.vibe"))
            .unwrap_err()
            .code,
        DiagCode::E001
    );
    assert_eq!(
        parse_program(include_str!("../fixtures/n3_quin_overlay.vibe"))
            .unwrap_err()
            .code,
        DiagCode::E001
    );
}

#[test]
fn import_alias_resolves_namespace() {
    let src = r#"
import "vibe:0.1/math" as m;
fn add() { return m.abs(-5); }
"#;
    let program = load_program(src).unwrap();
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(&program, "add", vec![], &mut host, &mut env).unwrap();
    assert_eq!(v.as_i64(), Some(5));
}

#[test]
fn import_without_alias_uses_namespace_name() {
    let src = r#"
import "vibe:0.1/math";
fn add() { return math.abs(-3); }
"#;
    let program = load_program(src).unwrap();
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(&program, "add", vec![], &mut host, &mut env).unwrap();
    assert_eq!(v.as_i64(), Some(3));
}

#[test]
fn import_invalid_path_rejected() {
    let src = r#"
import "vibe:0.1/bogus";
fn x() { return 0; }
"#;
    let err = load_program(src).unwrap_err();
    assert_eq!(err.code, DiagCode::E100);
}

#[test]
fn import_non_vibe_path_rejected() {
    let src = r#"
import "https://evil.example/mod";
fn x() { return 0; }
"#;
    let err = load_program(src).unwrap_err();
    assert_eq!(err.code, DiagCode::E100);
}
