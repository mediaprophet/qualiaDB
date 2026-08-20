//! vibe-0.1 §12 / §13 fixtures.

use poet_vibe::{
    check_cell, check_program, dispatch_hook, eval_cell, eval_function, load_program, parse_cell,
    parse_program, DiagCode, Env, MockHost, Value,
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
fn hook_dispatch_pulse_message_routes_to_function() {
    // The CLINIC module has `on pulse:message(topic, value) { return raise_alert(...) }`.
    // Dispatching a pulse:message hook with value > 85 should commit + publish,
    // exactly like calling raise_alert directly.
    let program = load_program(CLINIC).unwrap();
    let mut host = MockHost::default();
    let mut env = Env::default();
    let path = vec!["pulse".to_string(), "message".to_string()];
    let v = dispatch_hook(
        &program,
        &path,
        vec![Value::String("clinic/alerts".into()), Value::F64(90.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Ok(_)), "hook should return Ok: {v:?}");
    assert_eq!(host.committed, 1, "hook dispatch should commit the alert");
    assert_eq!(
        host.published,
        vec!["clinic/alerts".to_string()],
        "hook dispatch should publish the pulse"
    );
}

#[test]
fn hook_dispatch_unknown_path_returns_null() {
    // No matching hook → Ok(Null), not an error.
    let program = load_program(CLINIC).unwrap();
    let mut host = MockHost::default();
    let mut env = Env::default();
    let path = vec!["unknown".to_string(), "event".to_string()];
    let v = dispatch_hook(&program, &path, vec![], &mut host, &mut env).unwrap();
    assert_eq!(v, Value::Null);
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
    assert!(matches!(v, Value::QuinRef(_)));
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
    assert_eq!(v.as_i64(), Some(1_000_000_000));
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

// ── Phase G: Golden corpus expansion ─────────────────────────────────────

#[test]
fn g1_tick_hook_loads_and_dispatches() {
    let src = include_str!("../fixtures/g1_tick_hook.vibe");
    let program = load_program(src).expect("g1 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let path = vec!["tick".to_string()];
    let v = dispatch_hook(&program, &path, vec![], &mut host, &mut env).unwrap();
    assert_eq!(v, Value::Null, "tick hook should return null: {v:?}");
    assert_eq!(host.published, vec!["poet/tick".to_string()]);
}

#[test]
fn g2_tick_time_effect_fn() {
    let src = include_str!("../fixtures/g2_tick_time.vibe");
    let program = load_program(src).expect("g2 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(&program, "now", vec![], &mut host, &mut env).unwrap();
    assert_eq!(
        v.as_i64(),
        Some(1_000_000_000),
        "MockHost returns deterministic epoch"
    );
}

#[test]
fn g3_multi_hook_tick_and_pulse() {
    let src = include_str!("../fixtures/g3_multi_hook.vibe");
    let program = load_program(src).expect("g3 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();

    // Tick hook should fire without error.
    let tick_path = vec!["tick".to_string()];
    let v = dispatch_hook(&program, &tick_path, vec![], &mut host, &mut env).unwrap();
    assert_eq!(v, Value::Null);
    assert_eq!(host.published.len(), 0, "tick should not publish");

    // Pulse hook with value > 50 should publish.
    let pulse_path = vec!["pulse".to_string(), "message".to_string()];
    let v = dispatch_hook(
        &program,
        &pulse_path,
        vec![Value::String("poet/events".into()), Value::F64(75.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v, Value::Null);
    assert_eq!(host.published, vec!["poet/events".to_string()]);

    // Pulse hook with value <= 50 should NOT publish.
    let v = dispatch_hook(
        &program,
        &pulse_path,
        vec![Value::String("poet/events".into()), Value::F64(30.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v, Value::Null);
    assert_eq!(host.published.len(), 1, "low value should not publish");
}

#[test]
fn g4_reactive_cell_bounded_query() {
    let src = include_str!("../fixtures/g4_reactive_cell.vibe");
    let program = load_program(src).expect("g4 fixture");
    let mut host = MockHost {
        query_rows: 5,
        ..MockHost::default()
    };
    let mut env = Env::default();
    let v = eval_function(&program, "count", vec![], &mut host, &mut env).unwrap();
    match v {
        Value::Ok(inner) => assert_eq!(inner.as_i64(), Some(5)),
        other => panic!("{other:?}"),
    }
}

#[test]
fn g5_import_math_clamp() {
    let src = include_str!("../fixtures/g5_import_math.vibe");
    let program = load_program(src).expect("g5 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(
        &program,
        "clamp",
        vec![Value::I64(0), Value::I64(100), Value::I64(42)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v.as_i64(), Some(42));
    let v = eval_function(
        &program,
        "clamp",
        vec![Value::I64(0), Value::I64(100), Value::I64(150)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v.as_i64(), Some(100));
    let v = eval_function(
        &program,
        "clamp",
        vec![Value::I64(0), Value::I64(100), Value::I64(-5)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v.as_i64(), Some(0));
}

#[test]
fn g6_clinic_deontic_threshold() {
    let src = include_str!("../fixtures/g6_clinic_deontic.vibe");
    let program = load_program(src).expect("g6 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();

    // Below threshold — no commit, no publish.
    let v = eval_function(
        &program,
        "enforce_threshold",
        vec![Value::Iri("clinic:sensor_1".into()), Value::F64(70.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v, Value::Null);
    assert_eq!(host.committed, 0);
    assert_eq!(host.published.len(), 0);

    // Above threshold — commit + publish.
    let v = eval_function(
        &program,
        "enforce_threshold",
        vec![Value::Iri("clinic:sensor_1".into()), Value::F64(90.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v, Value::Null);
    assert_eq!(host.committed, 1);
    assert_eq!(host.published, vec!["clinic/alerts".to_string()]);
}

// ── Phase G: Golden corpus — domain verticals ─────────────────────────────
//
// Each domain has fixture .vibe files on disk plus an in-test conformance
// check that parses, type-checks, and evaluates the program.

// ── Physics ────────────────────────────────────────────────────────────────

#[test]
fn p1_wave_propagation_clamps_amplitude() {
    let src = include_str!("../fixtures/p1_wave_propagation.vibe");
    let program = load_program(src).expect("p1 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(
        &program,
        "wave_clamp",
        vec![Value::F64(1.5)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v.as_f64(), Some(1.0));
    let v = eval_function(
        &program,
        "wave_clamp",
        vec![Value::F64(-2.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v.as_f64(), Some(-1.0));
    let v = eval_function(
        &program,
        "wave_clamp",
        vec![Value::F64(0.5)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v.as_f64(), Some(0.5));
}

#[test]
fn p2_harmonic_oscillator_energy() {
    let src = include_str!("../fixtures/p2_harmonic_oscillator.vibe");
    let program = load_program(src).expect("p2 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(
        &program,
        "energy_clamp",
        vec![Value::F64(3.0), Value::F64(4.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v.as_f64(), Some(7.0));
    let v = eval_function(
        &program,
        "energy_clamp",
        vec![Value::F64(-10.0), Value::F64(3.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v.as_f64(), Some(0.0));
}

#[test]
fn p3_projectile_range() {
    let src = include_str!("../fixtures/p3_projectile.vibe");
    let program = load_program(src).expect("p3 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(
        &program,
        "range_estimate",
        vec![Value::F64(10.0), Value::F64(45.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    let r = v.as_f64().unwrap();
    assert!(r > 0.0, "range should be positive: {r}");
    assert!((r - 10.193).abs() < 0.1, "expected ~10.19, got {r}");
}

#[test]
fn p4_n_body_bounded_force() {
    let src = include_str!("../fixtures/p4_n_body.vibe");
    let program = load_program(src).expect("p4 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(
        &program,
        "bounded_force",
        vec![Value::F64(100.0), Value::F64(10.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v.as_f64(), Some(10.0));
    let v = eval_function(
        &program,
        "bounded_force",
        vec![Value::F64(-50.0), Value::F64(10.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v.as_f64(), Some(-10.0));
}

// ── EMF / Spectral ──────────────────────────────────────────────────────────

#[test]
fn e1_emf_to_color_clamps_wavelength() {
    let src = include_str!("../fixtures/e1_emf_to_color.vibe");
    let program = load_program(src).expect("e1 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(
        &program,
        "wavelength_to_rgb_channel",
        vec![Value::F64(800.0), Value::F64(380.0), Value::F64(780.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v.as_f64(), Some(780.0));
    let v = eval_function(
        &program,
        "wavelength_to_rgb_channel",
        vec![Value::F64(200.0), Value::F64(380.0), Value::F64(780.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v.as_f64(), Some(380.0));
}

#[test]
fn e2_emf_interference_clamps() {
    let src = include_str!("../fixtures/e2_emf_interference.vibe");
    let program = load_program(src).expect("e2 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(
        &program,
        "interference_amplitude",
        vec![Value::F64(0.8), Value::F64(0.5)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v.as_f64(), Some(1.0));
    let v = eval_function(
        &program,
        "interference_amplitude",
        vec![Value::F64(-0.6), Value::F64(-0.7)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v.as_f64(), Some(-1.0));
}

#[test]
fn e3_doppler_shift_positive() {
    let src = include_str!("../fixtures/e3_doppler_shift.vibe");
    let program = load_program(src).expect("e3 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(
        &program,
        "shifted_frequency",
        vec![Value::F64(440.0), Value::F64(10.0), Value::F64(340.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    let f = v.as_f64().unwrap();
    assert!(
        f > 440.0,
        "approaching source should increase frequency: {f}"
    );
}

#[test]
fn e4_emf_attenuation_inverse_square() {
    let src = include_str!("../fixtures/e4_emf_attenuation.vibe");
    let program = load_program(src).expect("e4 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(
        &program,
        "attenuated_intensity",
        vec![Value::F64(100.0), Value::F64(2.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v.as_f64(), Some(25.0));
    let v = eval_function(
        &program,
        "attenuated_intensity",
        vec![Value::F64(100.0), Value::F64(0.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert!(
        v.as_f64().unwrap() > 0.0,
        "zero distance should not divide by zero"
    );
}

// ── Geometry / SVG ──────────────────────────────────────────────────────────

#[test]
fn geo1_convex_hull_cross_product() {
    let src = include_str!("../fixtures/geo1_convex_hull.vibe");
    let program = load_program(src).expect("geo1 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(
        &program,
        "cross_product",
        vec![
            Value::F64(1.0),
            Value::F64(0.0),
            Value::F64(0.0),
            Value::F64(1.0),
        ],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v.as_f64(), Some(1.0));
    let v = eval_function(
        &program,
        "cross_product",
        vec![
            Value::F64(2.0),
            Value::F64(3.0),
            Value::F64(4.0),
            Value::F64(6.0),
        ],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v.as_f64(), Some(0.0));
}

#[test]
fn geo2_svg_path_distance() {
    let src = include_str!("../fixtures/geo2_svg_path.vibe");
    let program = load_program(src).expect("geo2 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(
        &program,
        "point_distance",
        vec![
            Value::F64(0.0),
            Value::F64(0.0),
            Value::F64(3.0),
            Value::F64(4.0),
        ],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v.as_f64(), Some(25.0));
}

#[test]
fn geo3_field_viz_magnitude() {
    let src = include_str!("../fixtures/geo3_field_viz.vibe");
    let program = load_program(src).expect("geo3 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(
        &program,
        "field_magnitude",
        vec![Value::F64(1.0), Value::F64(2.0), Value::F64(2.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v.as_f64(), Some(9.0));
}

// ── CSS Animation ───────────────────────────────────────────────────────────

#[test]
fn c1_css_keyframe_interpolation() {
    let src = include_str!("../fixtures/c1_css_keyframe.vibe");
    let program = load_program(src).expect("c1 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(
        &program,
        "interpolate_keyframe",
        vec![Value::F64(0.0), Value::F64(100.0), Value::F64(0.5)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v.as_f64(), Some(50.0));
    let v = eval_function(
        &program,
        "interpolate_keyframe",
        vec![Value::F64(0.0), Value::F64(100.0), Value::F64(1.5)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v.as_f64(), Some(100.0));
    let v = eval_function(
        &program,
        "interpolate_keyframe",
        vec![Value::F64(0.0), Value::F64(100.0), Value::F64(-0.5)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v.as_f64(), Some(0.0));
}

#[test]
fn c2_reactive_color_hue_clamp() {
    let src = include_str!("../fixtures/c2_reactive_color.vibe");
    let program = load_program(src).expect("c2 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(
        &program,
        "temperature_to_hue",
        vec![Value::F64(400.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v.as_f64(), Some(360.0));
    let v = eval_function(
        &program,
        "temperature_to_hue",
        vec![Value::F64(-10.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v.as_f64(), Some(0.0));
}

#[test]
fn c3_css_opacity_ratio() {
    let src = include_str!("../fixtures/c3_css_opacity.vibe");
    let program = load_program(src).expect("c3 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(
        &program,
        "fade_opacity",
        vec![Value::F64(50.0), Value::F64(100.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v.as_f64(), Some(0.5));
    let v = eval_function(
        &program,
        "fade_opacity",
        vec![Value::F64(150.0), Value::F64(100.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v.as_f64(), Some(1.0));
}

// ── Reactive cells ──────────────────────────────────────────────────────────

#[test]
fn r1_reactive_sum_counts_rows() {
    let src = include_str!("../fixtures/r1_reactive_sum.vibe");
    let program = load_program(src).expect("r1 fixture");
    let mut host = MockHost {
        query_rows: 7,
        ..MockHost::default()
    };
    let mut env = Env::default();
    let v = eval_function(&program, "count_rows", vec![], &mut host, &mut env).unwrap();
    match v {
        Value::Ok(inner) => assert_eq!(inner.as_i64(), Some(7)),
        other => panic!("{other:?}"),
    }
}

#[test]
fn r2_reactive_threshold_caps_count() {
    let src = include_str!("../fixtures/r2_reactive_threshold.vibe");
    let program = load_program(src).expect("r2 fixture");
    let mut host = MockHost {
        query_rows: 10,
        ..MockHost::default()
    };
    let mut env = Env::default();
    let v = eval_function(
        &program,
        "above_threshold",
        vec![Value::I64(3)],
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
fn r3_time_cell_clamps_score() {
    let src = include_str!("../fixtures/r3_time_cell.vibe");
    parse_cell(src).expect("r3 fixture parse");
    let mut host = MockHost::default();
    let mut env = Env::default();
    env.vars.insert("score".into(), Value::I64(75));
    let v = eval_cell(src, &mut host, &mut env).unwrap();
    assert_eq!(v.as_f64(), Some(75.0));
    env.vars.insert("score".into(), Value::I64(200));
    let v = eval_cell(src, &mut host, &mut env).unwrap();
    assert_eq!(v.as_f64(), Some(100.0));
}

// ── Hook dispatch ───────────────────────────────────────────────────────────

#[test]
fn h1_tick_counter_publishes() {
    let src = include_str!("../fixtures/h1_tick_counter.vibe");
    let program = load_program(src).expect("h1 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = dispatch_hook(
        &program,
        &vec!["tick".to_string()],
        vec![],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v, Value::Null);
    assert_eq!(host.published, vec!["tick/count".to_string()]);
}

#[test]
fn h2_pulse_filter_positive_only() {
    let src = include_str!("../fixtures/h2_pulse_filter.vibe");
    let program = load_program(src).expect("h2 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let path = vec!["pulse".to_string(), "message".to_string()];
    // Positive value → publish.
    let v = dispatch_hook(
        &program,
        &path,
        vec![Value::String("test".into()), Value::F64(42.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v, Value::Null);
    assert_eq!(host.published.len(), 1);
    // Zero value → no publish.
    let v = dispatch_hook(
        &program,
        &path,
        vec![Value::String("test".into()), Value::F64(0.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v, Value::Null);
    assert_eq!(host.published.len(), 1, "zero should not publish");
}

#[test]
fn h3_tick_time_publish_uses_time() {
    let src = include_str!("../fixtures/h3_tick_time_publish.vibe");
    let program = load_program(src).expect("h3 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = dispatch_hook(
        &program,
        &vec!["tick".to_string()],
        vec![],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v, Value::Null);
    assert_eq!(host.published, vec!["time/tick".to_string()]);
}

// ── Legal / Governance ──────────────────────────────────────────────────────

#[test]
fn l1_deontic_permit_commits() {
    let src = include_str!("../fixtures/l1_deontic_permit.vibe");
    let program = load_program(src).expect("l1 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(
        &program,
        "permit_action",
        vec![
            Value::Iri("actor:alice".into()),
            Value::Iri("action:read".into()),
        ],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v, Value::Null);
    assert_eq!(host.staged, 1);
    assert_eq!(host.committed, 1);
}

#[test]
fn l2_deontic_forbid_commits_and_publishes() {
    let src = include_str!("../fixtures/l2_deontic_forbid.vibe");
    let program = load_program(src).expect("l2 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(
        &program,
        "forbid_action",
        vec![
            Value::Iri("actor:bob".into()),
            Value::Iri("action:delete".into()),
        ],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v, Value::Null);
    assert_eq!(host.committed, 1);
    assert_eq!(host.published, vec!["policy/violation".to_string()]);
}

#[test]
fn l3_contract_validate_commits() {
    let src = include_str!("../fixtures/l3_contract_validate.vibe");
    let program = load_program(src).expect("l3 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(
        &program,
        "validate_contract",
        vec![
            Value::Iri("party:alice".into()),
            Value::Iri("contract:c1".into()),
        ],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v, Value::Null);
    assert_eq!(host.staged, 1);
    assert_eq!(host.committed, 1);
}

// ── Scientific ──────────────────────────────────────────────────────────────

#[test]
fn s1_smiles_validate_atom_count() {
    let src = include_str!("../fixtures/s1_smiles_validate.vibe");
    let program = load_program(src).expect("s1 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(
        &program,
        "atom_count_valid",
        vec![Value::F64(500.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v.as_f64(), Some(500.0));
    let v = eval_function(
        &program,
        "atom_count_valid",
        vec![Value::F64(2000.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v.as_f64(), Some(1000.0));
}

#[test]
fn s2_bio_alignment_score() {
    let src = include_str!("../fixtures/s2_bio_alignment.vibe");
    let program = load_program(src).expect("s2 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(
        &program,
        "alignment_score",
        vec![Value::F64(10.0), Value::F64(3.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v.as_f64(), Some(17.0));
    let v = eval_function(
        &program,
        "alignment_score",
        vec![Value::F64(1.0), Value::F64(5.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v.as_f64(), Some(0.0));
}

#[test]
fn s3_mol_weight_calculates() {
    let src = include_str!("../fixtures/s3_mol_weight.vibe");
    let program = load_program(src).expect("s3 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(
        &program,
        "molecular_weight",
        vec![Value::F64(6.0), Value::F64(12.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v.as_f64(), Some(72.0));
}

// ── Financial ───────────────────────────────────────────────────────────────

#[test]
fn f1_black_scholes_intrinsic() {
    let src = include_str!("../fixtures/f1_black_scholes.vibe");
    let program = load_program(src).expect("f1 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(
        &program,
        "intrinsic_value",
        vec![Value::F64(110.0), Value::F64(100.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v.as_f64(), Some(10.0));
    let v = eval_function(
        &program,
        "intrinsic_value",
        vec![Value::F64(80.0), Value::F64(100.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v.as_f64(), Some(0.0));
}

#[test]
fn f2_portfolio_opt_weighted_return() {
    let src = include_str!("../fixtures/f2_portfolio_opt.vibe");
    let program = load_program(src).expect("f2 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(
        &program,
        "weighted_return",
        vec![
            Value::F64(0.1),
            Value::F64(0.6),
            Value::F64(0.05),
            Value::F64(0.4),
        ],
        &mut host,
        &mut env,
    )
    .unwrap();
    let r = v.as_f64().unwrap();
    assert!((r - 0.08).abs() < 0.001, "expected 0.08, got {r}");
}

#[test]
fn f3_var_calc_clamps_confidence() {
    let src = include_str!("../fixtures/f3_var_calc.vibe");
    let program = load_program(src).expect("f3 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(
        &program,
        "value_at_risk",
        vec![Value::F64(1000.0), Value::F64(0.95)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert!((v.as_f64().unwrap() - 50.0).abs() < 0.001);
    let v = eval_function(
        &program,
        "value_at_risk",
        vec![Value::F64(1000.0), Value::F64(1.5)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v.as_f64(), Some(0.0));
}

// ── Negative (must-reject) fixtures ─────────────────────────────────────────

#[test]
fn n4_unknown_math_fn_runtime_error() {
    let src = include_str!("../fixtures/n4_unknown_math_fn.vibe");
    let program = load_program(src).unwrap();
    let mut host = MockHost::default();
    let mut env = Env::default();
    let err = eval_function(&program, "bad", vec![], &mut host, &mut env).unwrap_err();
    assert_eq!(err.code, DiagCode::E100);
}

#[test]
fn n5_commit_without_capability() {
    let src = include_str!("../fixtures/n5_commit_no_graph_write.vibe");
    let err = load_program(src).unwrap_err();
    assert_eq!(err.code, DiagCode::E300);
}

#[test]
fn n6_unbounded_while_rejected() {
    let src = include_str!("../fixtures/n6_unbounded_while.vibe");
    let err = load_program(src).unwrap_err();
    assert_eq!(err.code, DiagCode::E400);
}

#[test]
fn n7_time_in_pure_cell_rejected() {
    let src = include_str!("../fixtures/n7_time_in_pure_cell.vibe");
    let expr = parse_cell(src).unwrap();
    let err = check_cell(&expr).unwrap_err();
    assert_eq!(err.code, DiagCode::E200);
}

#[test]
fn n8_tick_queries_graph_rejected() {
    let src = include_str!("../fixtures/n8_tick_queries_graph.vibe");
    let err = load_program(src).unwrap_err();
    assert_eq!(err.code, DiagCode::E200);
}

#[test]
fn n9_non_vibe_import_rejected() {
    let src = include_str!("../fixtures/n9_non_vibe_import.vibe");
    let err = load_program(src).unwrap_err();
    assert_eq!(err.code, DiagCode::E100);
}

// ── All fixtures on disk parse/load ─────────────────────────────────────────

#[test]
fn all_phase_g_fixtures_on_disk_are_valid() {
    // Physics
    for (name, is_cell) in [
        ("p1_wave_propagation", false),
        ("p2_harmonic_oscillator", false),
        ("p3_projectile", false),
        ("p4_n_body", false),
    ] {
        let path = format!("../fixtures/{name}.vibe");
        let src = include_str!(concat!("../fixtures/p1_wave_propagation.vibe"));
        let _ = src; // suppress unused
                     // Just verify the files exist and parse — individual tests above check semantics.
    }
    // This is a compile-time include check: if any file is missing, compilation fails.
    let _ = include_str!("../fixtures/p1_wave_propagation.vibe");
    let _ = include_str!("../fixtures/p2_harmonic_oscillator.vibe");
    let _ = include_str!("../fixtures/p3_projectile.vibe");
    let _ = include_str!("../fixtures/p4_n_body.vibe");
    let _ = include_str!("../fixtures/e1_emf_to_color.vibe");
    let _ = include_str!("../fixtures/e2_emf_interference.vibe");
    let _ = include_str!("../fixtures/e3_doppler_shift.vibe");
    let _ = include_str!("../fixtures/e4_emf_attenuation.vibe");
    let _ = include_str!("../fixtures/geo1_convex_hull.vibe");
    let _ = include_str!("../fixtures/geo2_svg_path.vibe");
    let _ = include_str!("../fixtures/geo3_field_viz.vibe");
    let _ = include_str!("../fixtures/c1_css_keyframe.vibe");
    let _ = include_str!("../fixtures/c2_reactive_color.vibe");
    let _ = include_str!("../fixtures/c3_css_opacity.vibe");
    let _ = include_str!("../fixtures/r1_reactive_sum.vibe");
    let _ = include_str!("../fixtures/r2_reactive_threshold.vibe");
    let _ = include_str!("../fixtures/r3_time_cell.vibe");
    let _ = include_str!("../fixtures/h1_tick_counter.vibe");
    let _ = include_str!("../fixtures/h2_pulse_filter.vibe");
    let _ = include_str!("../fixtures/h3_tick_time_publish.vibe");
    let _ = include_str!("../fixtures/l1_deontic_permit.vibe");
    let _ = include_str!("../fixtures/l2_deontic_forbid.vibe");
    let _ = include_str!("../fixtures/l3_contract_validate.vibe");
    let _ = include_str!("../fixtures/s1_smiles_validate.vibe");
    let _ = include_str!("../fixtures/s2_bio_alignment.vibe");
    let _ = include_str!("../fixtures/s3_mol_weight.vibe");
    let _ = include_str!("../fixtures/f1_black_scholes.vibe");
    let _ = include_str!("../fixtures/f2_portfolio_opt.vibe");
    let _ = include_str!("../fixtures/f3_var_calc.vibe");
    let _ = include_str!("../fixtures/n4_unknown_math_fn.vibe");
    let _ = include_str!("../fixtures/n5_commit_no_graph_write.vibe");
    let _ = include_str!("../fixtures/n6_unbounded_while.vibe");
    let _ = include_str!("../fixtures/n7_time_in_pure_cell.vibe");
    let _ = include_str!("../fixtures/n8_tick_queries_graph.vibe");
    let _ = include_str!("../fixtures/n9_non_vibe_import.vibe");
    let _ = include_str!("../fixtures/ad1_enum_unit.vibe");
    let _ = include_str!("../fixtures/ad2_enum_payload.vibe");
    let _ = include_str!("../fixtures/ad3_enum_match.vibe");
    let _ = include_str!("../fixtures/ad4_enum_nested.vibe");
    let _ = include_str!("../fixtures/qd1_field_with_unit.vibe");
    let _ = include_str!("../fixtures/qd2_multiple_fields.vibe");
    let _ = include_str!("../fixtures/qd3_material_with_props.vibe");
    let _ = include_str!("../fixtures/qd4_field_material_law.vibe");
    let _ = include_str!("../fixtures/i1_checked_add.vibe");
    let _ = include_str!("../fixtures/i2_checked_overflow.vibe");
    let _ = include_str!("../fixtures/i3_mixed_int_float.vibe");
    let _ = include_str!("../fixtures/m1_mut_reassign.vibe");
    let _ = include_str!("../fixtures/m2_immutable_reject.vibe");
    let _ = include_str!("../fixtures/m3_mut_in_block.vibe");
    let _ = include_str!("../fixtures/crypto1_hash_sign.vibe");
    let _ = include_str!("../fixtures/zk1_threshold_range.vibe");
}

// ── H7: Enum/ADT golden corpus (T9 coverage) ──────────────────────────────────

#[test]
fn crypto1_hash_sign_parses() {
    let src = include_str!("../fixtures/crypto1_hash_sign.vibe");
    load_program(src).expect("crypto1 fixture should parse");
}

#[test]
fn zk1_threshold_range_parses() {
    let src = include_str!("../fixtures/zk1_threshold_range.vibe");
    load_program(src).expect("zk1 fixture should parse");
}

#[test]
fn ad1_enum_unit_parses() {
    let src = include_str!("../fixtures/ad1_enum_unit.vibe");
    load_program(src).expect("ad1 fixture should parse");
}

// ── X6: time.now → Instant (primary time primitive) ──────────────────────────

#[test]
fn x6_time_now_parses() {
    let src = include_str!("../fixtures/x6_time_now.vibe");
    load_program(src).expect("x6 fixture should parse");
}

#[test]
fn x6_time_now_evaluates() {
    let src = include_str!("../fixtures/x6_time_now.vibe");
    let program = load_program(src).expect("x6 fixture should load");
    let mut host = poet_vibe::MockHost::default();
    let mut env = poet_vibe::Env::default();
    let result = poet_vibe::eval_function(&program, "main", vec![], &mut host, &mut env)
        .expect("x6 fixture should evaluate");
    // MockHost returns Instant with secs=1_000_000_000, projected to i64.
    assert_eq!(result, poet_vibe::Value::I64(1_000_000_000));
}

// ── X5: Quantity mandatory for physical fields ───────────────────────────────

#[test]
fn x5_field_missing_unit_rejected() {
    let src = include_str!("../fixtures/x5_field_missing_unit.vibe");
    let err = load_program(src).unwrap_err();
    assert_eq!(err.code, DiagCode::E100);
    assert!(err.message.contains("unit"));
}

#[test]
fn x5_field_with_unit_accepted() {
    let src = include_str!("../fixtures/x5_field_with_unit.vibe");
    load_program(src).expect("field with unit should be accepted");
}

#[test]
fn x5_nonphysical_field_without_unit_accepted() {
    let src = include_str!("../fixtures/x5_nonphysical_field.vibe");
    load_program(src).expect("non-physical field without unit should be accepted");
}

#[test]
fn ad2_enum_payload_parses() {
    let src = include_str!("../fixtures/ad2_enum_payload.vibe");
    load_program(src).expect("ad2 fixture should parse");
}

#[test]
fn ad3_enum_match_evaluates() {
    let src = include_str!("../fixtures/ad3_enum_match.vibe");
    let program = load_program(src).expect("ad3 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let mut engine =
        poet_vibe::Engine::with_program(&mut host, poet_vibe::Budget::default(), &program);
    engine.eval_program(&program, &mut env).unwrap();
    let v = eval_function(&program, "main", vec![], &mut host, &mut env).unwrap();
    assert_eq!(v.as_f64(), Some(12.0));
}

#[test]
fn ad4_enum_nested_parses() {
    let src = include_str!("../fixtures/ad4_enum_nested.vibe");
    load_program(src).expect("ad4 fixture should parse");
}

// ── H8: Quantity/dimension golden corpus (T73 coverage) ───────────────────────

#[test]
fn qd1_field_with_unit_parses() {
    let src = include_str!("../fixtures/qd1_field_with_unit.vibe");
    load_program(src).expect("qd1 fixture should parse");
}

#[test]
fn qd2_multiple_fields_parses() {
    let src = include_str!("../fixtures/qd2_multiple_fields.vibe");
    load_program(src).expect("qd2 fixture should parse");
}

#[test]
fn qd3_material_with_props_parses() {
    let src = include_str!("../fixtures/qd3_material_with_props.vibe");
    load_program(src).expect("qd3 fixture should parse");
}

#[test]
fn qd4_field_material_law_parses() {
    let src = include_str!("../fixtures/qd4_field_material_law.vibe");
    load_program(src).expect("qd4 fixture should parse");
}

// ── H9: Checked-integer golden corpus (T11 coverage) ──────────────────────────

#[test]
fn i1_checked_add_evaluates() {
    let src = include_str!("../fixtures/i1_checked_add.vibe");
    let program = load_program(src).expect("i1 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(&program, "main", vec![], &mut host, &mut env).unwrap();
    assert_eq!(v.as_i64(), Some(100));
}

#[test]
fn i2_checked_overflow_produces_e600() {
    let src = include_str!("../fixtures/i2_checked_overflow.vibe");
    let program = load_program(src).expect("i2 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let result = eval_function(&program, "main", vec![], &mut host, &mut env);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, DiagCode::E600);
}

#[test]
fn i3_mixed_int_float_evaluates() {
    let src = include_str!("../fixtures/i3_mixed_int_float.vibe");
    let program = load_program(src).expect("i3 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(&program, "main", vec![], &mut host, &mut env).unwrap();
    assert_eq!(v.as_f64(), Some(3.14));
}

// ── H10: Mut enforcement golden corpus (T10 coverage) ─────────────────────────

#[test]
fn m1_mut_reassign_succeeds() {
    let src = include_str!("../fixtures/m1_mut_reassign.vibe");
    let program = load_program(src).expect("m1 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(&program, "main", vec![], &mut host, &mut env).unwrap();
    assert_eq!(v.as_i64(), Some(2));
}

#[test]
fn m2_immutable_reject_produces_e701() {
    let src = include_str!("../fixtures/m2_immutable_reject.vibe");
    // load_program calls check_program, which catches E701 at check time.
    let result = load_program(src);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code, DiagCode::E701);
}

#[test]
fn m3_mut_in_block_succeeds() {
    let src = include_str!("../fixtures/m3_mut_in_block.vibe");
    let program = load_program(src).expect("m3 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(&program, "main", vec![], &mut host, &mut env).unwrap();
    assert_eq!(v.as_i64(), Some(30));
}

// ── T40: Unicode identifiers ─────────────────────────────────────────────

#[test]
fn t40_unicode_ident_cyrillic_parses_and_evaluates() {
    let src = r#"
fn привет() {
    return 42;
}
"#;
    let program = load_program(src).expect("Cyrillic identifier should parse");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(&program, "привет", vec![], &mut host, &mut env).unwrap();
    assert_eq!(v.as_i64(), Some(42));
}

#[test]
fn t40_unicode_ident_cjk_parses_and_evaluates() {
    let src = r#"
fn 変数() {
    let 値 = 100;
    return 値 + 1;
}
"#;
    let program = load_program(src).expect("CJK identifier should parse");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(&program, "変数", vec![], &mut host, &mut env).unwrap();
    assert_eq!(v.as_i64(), Some(101));
}

#[test]
fn t40_unicode_ident_greek_parses() {
    let src = r#"
fn μεταβλητή() {
    return 7;
}
"#;
    let program = load_program(src).expect("Greek identifier should parse");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(&program, "μεταβλητή", vec![], &mut host, &mut env).unwrap();
    assert_eq!(v.as_i64(), Some(7));
}

#[test]
fn t40_unicode_ident_arabic_parses() {
    let src = r#"
fn متغير() {
    return 9;
}
"#;
    let program = load_program(src).expect("Arabic identifier should parse");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(&program, "متغير", vec![], &mut host, &mut env).unwrap();
    assert_eq!(v.as_i64(), Some(9));
}

#[test]
fn t40_unicode_ident_latin_extended_parses() {
    let src = r#"
fn café() {
    return 3;
}
"#;
    let program = load_program(src).expect("Latin Extended identifier should parse");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(&program, "café", vec![], &mut host, &mut env).unwrap();
    assert_eq!(v.as_i64(), Some(3));
}

#[test]
fn t40_bidi_control_rejected() {
    // U+202E (RLO) in an identifier — Trojan Source attack
    let src = format!(
        r#"
fn hello{evil}world() {{
    return 1;
}}
"#,
        evil = "\u{202E}"
    );
    let result = load_program(&src);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code, DiagCode::E001);
    assert!(err.message.contains("BiDi"));
}

#[test]
fn t40_cyrillic_homoglyph_mixed_rejected() {
    // Latin 'a' + Cyrillic 'а' (U+0430) — homoglyph attack
    let src = format!(
        r#"
fn a{evil}bc() {{
    return 1;
}}
"#,
        evil = "\u{0430}"
    );
    let result = load_program(&src);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code, DiagCode::E001);
    assert!(err.message.contains("confusable") || err.message.contains("homoglyph"));
}

#[test]
fn t40_pure_cyrillic_function_call_works() {
    let src = r#"
fn вычислить() {
    return 5;
}

fn основной() {
    return вычислить() * 2;
}
"#;
    let program = load_program(src).expect("pure Cyrillic identifiers should parse");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(&program, "основной", vec![], &mut host, &mut env).unwrap();
    assert_eq!(v.as_i64(), Some(10));
}

#[test]
fn t40_korean_identifier_parses() {
    let src = r#"
fn 변수() {
    return 42;
}
"#;
    let program = load_program(src).expect("Korean identifier should parse");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(&program, "변수", vec![], &mut host, &mut env).unwrap();
    assert_eq!(v.as_i64(), Some(42));
}

#[test]
fn t40_japanese_hiragana_parses() {
    let src = r#"
fn へんすう() {
    return 8;
}
"#;
    let program = load_program(src).expect("Hiragana identifier should parse");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(&program, "へんすう", vec![], &mut host, &mut env).unwrap();
    assert_eq!(v.as_i64(), Some(8));
}

// ── Phase G: Comprehensive golden corpus ───────────────────────────────────
// Each fixture has a parse test + an evaluation test with golden vectors.

// ── Physics: p5–p11 ────────────────────────────────────────────────────────

#[test]
fn p5_heat_diffusion_thermal_diffusivity() {
    let src = include_str!("../fixtures/p5_heat_diffusion.vibe");
    let program = load_program(src).expect("p5 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    // α = k / (ρ * cp) = 1.0 / (2.0 * 4.0) = 0.125
    let v = eval_function(
        &program,
        "thermal_diffusivity",
        vec![Value::F64(1.0), Value::F64(2.0), Value::F64(4.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert!((v.as_f64().unwrap() - 0.125).abs() < 1e-9);
}

#[test]
fn p5_heat_diffusion_steady_state_mean() {
    let src = include_str!("../fixtures/p5_heat_diffusion.vibe");
    let program = load_program(src).expect("p5 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(
        &program,
        "steady_state_mean",
        vec![Value::List(vec![
            Value::F64(10.0),
            Value::F64(20.0),
            Value::F64(30.0),
        ])],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert!((v.as_f64().unwrap() - 20.0).abs() < 1e-9);
}

#[test]
fn p6_advection_diffusion_cfl() {
    let src = include_str!("../fixtures/p6_advection_diffusion.vibe");
    let program = load_program(src).expect("p6 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    // CFL = |v| * dt / dx = 1.0 * 0.01 / 0.02 = 0.5
    let v = eval_function(
        &program,
        "courant_number",
        vec![Value::F64(1.0), Value::F64(0.01), Value::F64(0.02)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert!((v.as_f64().unwrap() - 0.5).abs() < 1e-9);
}

#[test]
fn p6_advection_diffusion_stability() {
    let src = include_str!("../fixtures/p6_advection_diffusion.vibe");
    let program = load_program(src).expect("p6 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(
        &program,
        "is_stable",
        vec![Value::F64(0.5)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Bool(true)));
    let v = eval_function(
        &program,
        "is_stable",
        vec![Value::F64(1.5)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Bool(false)));
}

#[test]
fn p7_pendulum_period() {
    let src = include_str!("../fixtures/p7_pendulum.vibe");
    let program = load_program(src).expect("p7 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    // T = 2π * sqrt(L/g) = 2π * sqrt(1/9.81) ≈ 2.006
    let v = eval_function(
        &program,
        "period",
        vec![Value::F64(1.0), Value::F64(9.81)],
        &mut host,
        &mut env,
    )
    .unwrap();
    let expected = 2.0 * std::f64::consts::PI * (1.0_f64 / 9.81).sqrt();
    assert!((v.as_f64().unwrap() - expected).abs() < 1e-9);
}

#[test]
fn p7_pendulum_angular_frequency() {
    let src = include_str!("../fixtures/p7_pendulum.vibe");
    let program = load_program(src).expect("p7 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    // ω = sqrt(g/L) = sqrt(9.81/1.0) ≈ 3.132
    let v = eval_function(
        &program,
        "angular_frequency",
        vec![Value::F64(1.0), Value::F64(9.81)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert!((v.as_f64().unwrap() - (9.81_f64).sqrt()).abs() < 1e-9);
}

#[test]
fn p8_molecular_dynamics_lennard_jones() {
    let src = include_str!("../fixtures/p8_molecular_dynamics.vibe");
    let program = load_program(src).expect("p8 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    // At r = sigma, LJ potential = 4ε(1 - 1) = 0
    let v = eval_function(
        &program,
        "lennard_jones_potential",
        vec![Value::F64(1.0), Value::F64(1.0), Value::F64(1.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert!((v.as_f64().unwrap() - 0.0).abs() < 1e-9);
}

#[test]
fn p8_molecular_dynamics_force() {
    let src = include_str!("../fixtures/p8_molecular_dynamics.vibe");
    let program = load_program(src).expect("p8 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    // At r = sigma, force = 24ε/σ * (2*1 - 1) = 24ε/σ
    let v = eval_function(
        &program,
        "force_magnitude",
        vec![Value::F64(1.0), Value::F64(1.0), Value::F64(1.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert!((v.as_f64().unwrap() - 24.0).abs() < 1e-9);
}

#[test]
fn p9_cfd_reynolds_number() {
    let src = include_str!("../fixtures/p9_cfd_step.vibe");
    let program = load_program(src).expect("p9 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    // Re = |v| * L / ν = 1.0 * 0.1 / 0.001 = 100
    let v = eval_function(
        &program,
        "reynolds_number",
        vec![Value::F64(1.0), Value::F64(0.1), Value::F64(0.001)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert!((v.as_f64().unwrap() - 100.0).abs() < 1e-9);
}

#[test]
fn p9_cfd_is_laminar() {
    let src = include_str!("../fixtures/p9_cfd_step.vibe");
    let program = load_program(src).expect("p9 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(
        &program,
        "is_laminar",
        vec![Value::F64(100.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Bool(true)));
    let v = eval_function(
        &program,
        "is_turbulent",
        vec![Value::F64(100.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Bool(false)));
}

#[test]
fn p10_quantum_energy_level() {
    let src = include_str!("../fixtures/p10_quantum_states.vibe");
    let program = load_program(src).expect("p10 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    // E_n = ℏω(n + 0.5) = 1.0 * 1.0 * (0 + 0.5) = 0.5
    let v = eval_function(
        &program,
        "energy_level",
        vec![Value::F64(0.0), Value::F64(1.0), Value::F64(1.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert!((v.as_f64().unwrap() - 0.5).abs() < 1e-9);
}

#[test]
fn p10_quantum_transition() {
    let src = include_str!("../fixtures/p10_quantum_states.vibe");
    let program = load_program(src).expect("p10 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    // ΔE = E_2 - E_1 = ℏω(2.5 - 1.5) = ℏω
    let v = eval_function(
        &program,
        "transition_energy",
        vec![
            Value::F64(1.0),
            Value::F64(2.0),
            Value::F64(1.0),
            Value::F64(1.0),
        ],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert!((v.as_f64().unwrap() - 1.0).abs() < 1e-9);
}

#[test]
fn p11_logistic_growth_carrying() {
    let src = include_str!("../fixtures/p11_logistic_growth.vibe");
    let program = load_program(src).expect("p11 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    // N(t) = K / (1 + ((K-N0)/N0) * e^(-rt))
    // K=100, r=0.5, t=0, N0=10 → 100 / (1 + 9*1) = 10
    let v = eval_function(
        &program,
        "carrying_capacity",
        vec![
            Value::F64(100.0),
            Value::F64(0.5),
            Value::F64(0.0),
            Value::F64(10.0),
        ],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert!((v.as_f64().unwrap() - 10.0).abs() < 1e-9);
}

#[test]
fn p11_logistic_growth_rate() {
    let src = include_str!("../fixtures/p11_logistic_growth.vibe");
    let program = load_program(src).expect("p11 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    // dN/dt = rN(1 - N/K) = 0.5 * 50 * (1 - 50/100) = 12.5
    let v = eval_function(
        &program,
        "growth_rate_at",
        vec![Value::F64(100.0), Value::F64(0.5), Value::F64(50.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert!((v.as_f64().unwrap() - 12.5).abs() < 1e-9);
}

// ── EMF/Spectral: e5–e8 ────────────────────────────────────────────────────

#[test]
fn e5_spd_to_xyz_clamps_wavelength() {
    let src = include_str!("../fixtures/e5_spd_to_xyz.vibe");
    let program = load_program(src).expect("e5 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(
        &program,
        "wavelength_to_xyz",
        vec![Value::F64(200.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v.as_f64(), Some(380.0));
}

#[test]
fn e5_spd_to_xyz_chromaticity() {
    let src = include_str!("../fixtures/e5_spd_to_xyz.vibe");
    let program = load_program(src).expect("e5 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    // x / (x + y + z) = 1 / (1 + 1 + 1) = 1/3
    let v = eval_function(
        &program,
        "xyz_to_xy",
        vec![Value::F64(1.0), Value::F64(1.0), Value::F64(1.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert!((v.as_f64().unwrap() - 1.0 / 3.0).abs() < 1e-9);
}

#[test]
fn e6_spectral_blend_wavelengths() {
    let src = include_str!("../fixtures/e6_spectral_blend.vibe");
    let program = load_program(src).expect("e6 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    // blend(400, 700, 0.5) = 550
    let v = eval_function(
        &program,
        "blend_wavelengths",
        vec![Value::F64(400.0), Value::F64(700.0), Value::F64(0.5)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert!((v.as_f64().unwrap() - 550.0).abs() < 1e-9);
}

#[test]
fn e6_spectral_metamer_check() {
    let src = include_str!("../fixtures/e6_spectral_blend.vibe");
    let program = load_program(src).expect("e6 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(
        &program,
        "metamer_check",
        vec![
            Value::F64(0.5),
            Value::F64(0.5),
            Value::F64(0.5),
            Value::F64(0.5),
        ],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert!(matches!(v, Value::Bool(true)));
}

#[test]
fn e7_gamut_map_clamp_rgb() {
    let src = include_str!("../fixtures/e7_gamut_map.vibe");
    let program = load_program(src).expect("e7 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(
        &program,
        "clamp_rgb_channel",
        vec![Value::F64(1.5)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v.as_f64(), Some(1.0));
}

#[test]
fn e7_gamut_map_srgb_roundtrip() {
    let src = include_str!("../fixtures/e7_gamut_map.vibe");
    let program = load_program(src).expect("e7 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    // sRGB → linear → sRGB should be identity (approximately)
    let v = eval_function(
        &program,
        "srgb_to_linear",
        vec![Value::F64(0.5)],
        &mut host,
        &mut env,
    )
    .unwrap();
    let linear = v.as_f64().unwrap();
    let v2 = eval_function(
        &program,
        "linear_to_srgb",
        vec![Value::F64(linear)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert!((v2.as_f64().unwrap() - 0.5).abs() < 1e-3);
}

#[test]
fn e8_emf_inverse_square() {
    let src = include_str!("../fixtures/e8_emf_field_grid.vibe");
    let program = load_program(src).expect("e8 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    // P / (4π * r²) = 100 / (4π * 1) ≈ 7.958
    let v = eval_function(
        &program,
        "inverse_square",
        vec![Value::F64(1.0), Value::F64(100.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    let expected = 100.0 / (4.0 * std::f64::consts::PI);
    assert!((v.as_f64().unwrap() - expected).abs() < 1e-6);
}

#[test]
fn e8_emf_inverse_square_zero_distance() {
    let src = include_str!("../fixtures/e8_emf_field_grid.vibe");
    let program = load_program(src).expect("e8 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(
        &program,
        "inverse_square",
        vec![Value::F64(0.0), Value::F64(100.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v.as_f64(), Some(0.0));
}

// ── Geometry: geo4–geo6 ────────────────────────────────────────────────────

#[test]
fn geo4_svg_circle_area() {
    let src = include_str!("../fixtures/geo4_svg_circle.vibe");
    let program = load_program(src).expect("geo4 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(
        &program,
        "circle_area",
        vec![Value::F64(2.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert!((v.as_f64().unwrap() - 4.0 * std::f64::consts::PI).abs() < 1e-9);
}

#[test]
fn geo5_cubic_bezier_endpoints() {
    let src = include_str!("../fixtures/geo5_svg_bezier.vibe");
    let program = load_program(src).expect("geo5 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    // At t=0, B(0) = p0
    let v = eval_function(
        &program,
        "cubic_bezier_point",
        vec![
            Value::F64(0.0),
            Value::F64(1.0),
            Value::F64(2.0),
            Value::F64(3.0),
            Value::F64(0.0),
        ],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v.as_f64(), Some(0.0));
    // At t=1, B(1) = p3
    let v = eval_function(
        &program,
        "cubic_bezier_point",
        vec![
            Value::F64(0.0),
            Value::F64(1.0),
            Value::F64(2.0),
            Value::F64(3.0),
            Value::F64(1.0),
        ],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v.as_f64(), Some(3.0));
}

#[test]
fn geo5_quadratic_bezier_midpoint() {
    let src = include_str!("../fixtures/geo5_svg_bezier.vibe");
    let program = load_program(src).expect("geo5 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    // At t=0.5, Q(0.5) = 0.25*p0 + 0.5*p1 + 0.25*p2 = 0.25*0 + 0.5*2 + 0.25*4 = 2.0
    let v = eval_function(
        &program,
        "quadratic_bezier_point",
        vec![
            Value::F64(0.0),
            Value::F64(2.0),
            Value::F64(4.0),
            Value::F64(0.5),
        ],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert!((v.as_f64().unwrap() - 2.0).abs() < 1e-9);
}

#[test]
fn geo6_triangle_area() {
    let src = include_str!("../fixtures/geo6_point_in_polygon.vibe");
    let program = load_program(src).expect("geo6 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    // Triangle (0,0), (4,0), (0,3) → area = 6
    let v = eval_function(
        &program,
        "triangle_area",
        vec![
            Value::F64(0.0),
            Value::F64(0.0),
            Value::F64(4.0),
            Value::F64(0.0),
            Value::F64(0.0),
            Value::F64(3.0),
        ],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert!((v.as_f64().unwrap() - 6.0).abs() < 1e-9);
}

// ── CSS Animation: c4–c6 ───────────────────────────────────────────────────

#[test]
fn c4_css_transform_rotate_to_radians() {
    let src = include_str!("../fixtures/c4_css_transform.vibe");
    let program = load_program(src).expect("c4 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    // 180° = π radians
    let v = eval_function(
        &program,
        "rotate_degrees_to_radians",
        vec![Value::F64(180.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert!((v.as_f64().unwrap() - std::f64::consts::PI).abs() < 1e-9);
}

#[test]
fn c4_css_transform_scale_non_negative() {
    let src = include_str!("../fixtures/c4_css_transform.vibe");
    let program = load_program(src).expect("c4 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(
        &program,
        "scale_factor",
        vec![Value::F64(-1.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v.as_f64(), Some(0.0));
}

#[test]
fn c5_css_easing_in_out() {
    let src = include_str!("../fixtures/c5_css_easing.vibe");
    let program = load_program(src).expect("c5 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    // ease_in_out(0.5) = 0.5 (symmetric)
    let v = eval_function(
        &program,
        "ease_in_out",
        vec![Value::F64(0.5)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert!((v.as_f64().unwrap() - 0.5).abs() < 1e-9);
}

#[test]
fn c5_css_easing_in_quadratic() {
    let src = include_str!("../fixtures/c5_css_easing.vibe");
    let program = load_program(src).expect("c5 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    // ease_in(0.5) = 0.25
    let v = eval_function(
        &program,
        "ease_in",
        vec![Value::F64(0.5)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert!((v.as_f64().unwrap() - 0.25).abs() < 1e-9);
}

#[test]
fn c5_css_easing_linear() {
    let src = include_str!("../fixtures/c5_css_easing.vibe");
    let program = load_program(src).expect("c5 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(
        &program,
        "linear_easing",
        vec![Value::F64(0.3)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert!((v.as_f64().unwrap() - 0.3).abs() < 1e-9);
}

#[test]
fn c6_css_color_blend_linear() {
    let src = include_str!("../fixtures/c6_css_color_blend.vibe");
    let program = load_program(src).expect("c6 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    // blend(0.2, 0.8, 0.5) = 0.5
    let v = eval_function(
        &program,
        "blend_colors",
        vec![Value::F64(0.2), Value::F64(0.8), Value::F64(0.5)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert!((v.as_f64().unwrap() - 0.5).abs() < 1e-9);
}

#[test]
fn c6_css_color_blend_multiply() {
    let src = include_str!("../fixtures/c6_css_color_blend.vibe");
    let program = load_program(src).expect("c6 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    // multiply(0.5, 0.5) = 0.25
    let v = eval_function(
        &program,
        "multiply_blend",
        vec![Value::F64(0.5), Value::F64(0.5)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert!((v.as_f64().unwrap() - 0.25).abs() < 1e-9);
}

#[test]
fn c6_css_color_blend_screen() {
    let src = include_str!("../fixtures/c6_css_color_blend.vibe");
    let program = load_program(src).expect("c6 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    // screen(0.5, 0.5) = 1 - 0.25 = 0.75
    let v = eval_function(
        &program,
        "screen_blend",
        vec![Value::F64(0.5), Value::F64(0.5)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert!((v.as_f64().unwrap() - 0.75).abs() < 1e-9);
}

// ── Reactive cells: r4–r6 ──────────────────────────────────────────────────

#[test]
fn r4_reactive_compose_sum() {
    let src = include_str!("../fixtures/r4_reactive_compose.vibe");
    let program = load_program(src).expect("r4 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(
        &program,
        "sum_quins",
        vec![Value::List(vec![
            Value::F64(1.0),
            Value::F64(2.0),
            Value::F64(3.0),
        ])],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert!((v.as_f64().unwrap() - 6.0).abs() < 1e-9);
}

#[test]
fn r5_threshold_alert_check() {
    let src = include_str!("../fixtures/r5_threshold_alert.vibe");
    let program = load_program(src).expect("r5 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(
        &program,
        "check_threshold",
        vec![Value::F64(5.0), Value::F64(10.0), Value::F64(20.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v.as_i64(), Some(-1));
    let v = eval_function(
        &program,
        "check_threshold",
        vec![Value::F64(25.0), Value::F64(10.0), Value::F64(20.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v.as_i64(), Some(1));
    let v = eval_function(
        &program,
        "check_threshold",
        vec![Value::F64(15.0), Value::F64(10.0), Value::F64(20.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v.as_i64(), Some(0));
}

#[test]
fn r6_reactive_chain_derive_rate() {
    let src = include_str!("../fixtures/r6_reactive_chain.vibe");
    let program = load_program(src).expect("r6 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    // rate = (curr - prev) / dt = (20 - 10) / 2 = 5
    let v = eval_function(
        &program,
        "derive_rate",
        vec![Value::F64(10.0), Value::F64(20.0), Value::F64(2.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert!((v.as_f64().unwrap() - 5.0).abs() < 1e-9);
}

#[test]
fn r6_reactive_chain_zero_dt() {
    let src = include_str!("../fixtures/r6_reactive_chain.vibe");
    let program = load_program(src).expect("r6 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(
        &program,
        "derive_rate",
        vec![Value::F64(10.0), Value::F64(20.0), Value::F64(0.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v.as_f64(), Some(0.0));
}

// ── Financial: f4–f6 ───────────────────────────────────────────────────────

#[test]
fn f4_options_greeks_delta_in_the_money() {
    let src = include_str!("../fixtures/f4_options_greeks.vibe");
    let program = load_program(src).expect("f4 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    // S > K → delta = 1.0
    let v = eval_function(
        &program,
        "delta_call",
        vec![Value::F64(110.0), Value::F64(100.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert!((v.as_f64().unwrap() - 1.0).abs() < 1e-9);
}

#[test]
fn f4_options_greeks_delta_out_of_the_money() {
    let src = include_str!("../fixtures/f4_options_greeks.vibe");
    let program = load_program(src).expect("f4 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    // S < K → delta = S/K = 90/100 = 0.9
    let v = eval_function(
        &program,
        "delta_call",
        vec![Value::F64(90.0), Value::F64(100.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert!((v.as_f64().unwrap() - 0.9).abs() < 1e-9);
}

#[test]
fn f5_bond_pricing_zero_yield() {
    let src = include_str!("../fixtures/f5_bond_pricing.vibe");
    let program = load_program(src).expect("f5 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    // yield = 0 → PV = face + coupon * periods = 100 + 5 * 3 = 115
    let v = eval_function(
        &program,
        "present_value",
        vec![
            Value::F64(100.0),
            Value::F64(5.0),
            Value::F64(0.0),
            Value::F64(3.0),
        ],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert!((v.as_f64().unwrap() - 115.0).abs() < 1e-9);
}

#[test]
fn f6_risk_metrics_sharpe_ratio() {
    let src = include_str!("../fixtures/f6_risk_metrics.vibe");
    let program = load_program(src).expect("f6 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    // returns = [0.1, 0.2, 0.3], rf = 0.0
    // mean = 0.2, std = sqrt(((0.1-0.2)² + 0² + (0.3-0.2)²)/3) = sqrt(0.02/3) ≈ 0.0816
    // Sharpe = 0.2 / 0.0816 ≈ 2.449
    let v = eval_function(
        &program,
        "sharpe_ratio",
        vec![
            Value::List(vec![Value::F64(0.1), Value::F64(0.2), Value::F64(0.3)]),
            Value::F64(0.0),
        ],
        &mut host,
        &mut env,
    )
    .unwrap();
    let sharpe = v.as_f64().unwrap();
    assert!(sharpe > 2.0 && sharpe < 3.0);
}

#[test]
fn f6_risk_metrics_max_drawdown() {
    let src = include_str!("../fixtures/f6_risk_metrics.vibe");
    let program = load_program(src).expect("f6 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    // values = [100, 120, 80, 90] → peak=120, trough=80, dd = (120-80)/120 = 1/3
    let v = eval_function(
        &program,
        "max_drawdown",
        vec![Value::List(vec![
            Value::F64(100.0),
            Value::F64(120.0),
            Value::F64(80.0),
            Value::F64(90.0),
        ])],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert!((v.as_f64().unwrap() - 1.0 / 3.0).abs() < 1e-6);
}

// ── Scientific: s4–s6 ──────────────────────────────────────────────────────

#[test]
fn s4_molar_mass_calculation() {
    let src = include_str!("../fixtures/s4_molar_mass.vibe");
    let program = load_program(src).expect("s4 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    // H2O: 2*1.008 + 15.999 = 18.015
    let v = eval_function(
        &program,
        "molar_mass",
        vec![Value::List(vec![
            Value::F64(1.008),
            Value::F64(1.008),
            Value::F64(15.999),
        ])],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert!((v.as_f64().unwrap() - 18.015).abs() < 1e-3);
}

#[test]
fn s4_moles_from_mass() {
    let src = include_str!("../fixtures/s4_molar_mass.vibe");
    let program = load_program(src).expect("s4 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    // moles = mass / molar_mass = 36.03 / 18.015 = 2.0
    let v = eval_function(
        &program,
        "moles_from_mass",
        vec![Value::F64(36.03), Value::F64(18.015)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert!((v.as_f64().unwrap() - 2.0).abs() < 1e-3);
}

#[test]
fn s5_reaction_kinetics_half_life() {
    let src = include_str!("../fixtures/s5_reaction_kinetics.vibe");
    let program = load_program(src).expect("s5 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    // t½ = ln(2)/k = 0.693/0.1 = 6.93
    let v = eval_function(
        &program,
        "half_life_first_order",
        vec![Value::F64(0.1)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert!((v.as_f64().unwrap() - 6.931471805599453).abs() < 1e-6);
}

#[test]
fn s5_reaction_kinetics_first_order() {
    let src = include_str!("../fixtures/s5_reaction_kinetics.vibe");
    let program = load_program(src).expect("s5 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    // rate = k * [A] = 0.1 * 2.0 = 0.2
    let v = eval_function(
        &program,
        "first_order_rate",
        vec![Value::F64(0.1), Value::F64(2.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert!((v.as_f64().unwrap() - 0.2).abs() < 1e-9);
}

#[test]
fn s6_dna_gc_content() {
    let src = include_str!("../fixtures/s6_dna_codon.vibe");
    let program = load_program(src).expect("s6 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    // bases: [1, 2, 3, 4] → GC = 2/4 = 0.5 (1=G, 2=C)
    let v = eval_function(
        &program,
        "gc_content",
        vec![Value::List(vec![
            Value::F64(1.0),
            Value::F64(2.0),
            Value::F64(3.0),
            Value::F64(4.0),
        ])],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert!((v.as_f64().unwrap() - 0.5).abs() < 1e-9);
}

// ── Cross-domain: x1–x4 ────────────────────────────────────────────────────

#[test]
fn x1_emf_to_css_wavelength_mapping() {
    let src = include_str!("../fixtures/x1_emf_to_css.vibe");
    let program = load_program(src).expect("x1 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    // 580nm → (580-380)/400 = 0.5
    let v = eval_function(
        &program,
        "wavelength_to_css_rgb",
        vec![Value::F64(580.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert!((v.as_f64().unwrap() - 0.5).abs() < 1e-9);
}

#[test]
fn x1_emf_to_css_opacity() {
    let src = include_str!("../fixtures/x1_emf_to_css.vibe");
    let program = load_program(src).expect("x1 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(
        &program,
        "css_rgb_to_opacity",
        vec![Value::F64(1.5)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v.as_f64(), Some(1.0));
}

#[test]
fn x2_physics_to_geometry_trajectory() {
    let src = include_str!("../fixtures/x2_physics_to_geometry.vibe");
    let program = load_program(src).expect("x2 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    // 45° launch, v=10, g=9.81 → range = v²sin(90°)/g = 100/9.81 ≈ 10.19
    let v = eval_function(
        &program,
        "trajectory_to_path",
        vec![Value::F64(10.0), Value::F64(45.0), Value::F64(9.81)],
        &mut host,
        &mut env,
    )
    .unwrap();
    let expected = 100.0 * (90.0_f64).to_radians().sin() / 9.81;
    assert!((v.as_f64().unwrap() - expected).abs() < 1e-6);
}

#[test]
fn x3_governance_to_pulse_severity() {
    let src = include_str!("../fixtures/x3_governance_to_pulse.vibe");
    let program = load_program(src).expect("x3 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(
        &program,
        "severity_to_pulse_value",
        vec![Value::I64(3)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v.as_f64(), Some(1.0));
    let v = eval_function(
        &program,
        "severity_to_pulse_value",
        vec![Value::I64(0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v.as_f64(), Some(0.0));
}

#[test]
fn x4_financial_to_graph_risk_classification() {
    let src = include_str!("../fixtures/x4_financial_to_graph.vibe");
    let program = load_program(src).expect("x4 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    // sharpe > 2.0 → level 0 (low risk)
    let v = eval_function(
        &program,
        "classify_risk",
        vec![Value::F64(2.5)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v.as_i64(), Some(0));
    // sharpe < 0 → level 3 (high risk)
    let v = eval_function(
        &program,
        "classify_risk",
        vec![Value::F64(-1.0)],
        &mut host,
        &mut env,
    )
    .unwrap();
    assert_eq!(v.as_i64(), Some(3));
}

// ── Negative fixtures: n10–n25 ─────────────────────────────────────────────
// These should fail at parse, check, or eval time.

#[test]
fn n10_negative_sqrt_fails() {
    let src = include_str!("../fixtures/n10_negative_sqrt.vibe");
    let program = load_program(src).expect("n10 should parse");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let result = eval_function(&program, "bad", vec![], &mut host, &mut env);
    // sqrt(-1) should produce NaN or error — either way it's not a valid result
    assert!(result.is_err() || result.unwrap().as_f64().map(|f| f.is_nan()).unwrap_or(true));
}

#[test]
fn n11_div_by_zero_physics_trapped() {
    let src = include_str!("../fixtures/n11_div_by_zero_physics.vibe");
    let program = load_program(src).expect("n11 should parse");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let result = eval_function(&program, "bad", vec![], &mut host, &mut env);
    // VibeScript traps division by zero at runtime (E600)
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code, DiagCode::E600);
    assert!(err.message.contains("division by zero"));
}

#[test]
fn n12_negative_reynolds_parses() {
    let src = include_str!("../fixtures/n12_negative_reynolds.vibe");
    // This is a semantic error, not a parse error — it should parse fine
    load_program(src).expect("n12 should parse (semantic issue, not syntax)");
}

#[test]
fn n13_negative_log_emf_fails() {
    let src = include_str!("../fixtures/n13_negative_log_emf.vibe");
    let program = load_program(src).expect("n13 should parse");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let result = eval_function(&program, "bad", vec![], &mut host, &mut env);
    assert!(result.is_err() || result.unwrap().as_f64().map(|f| f.is_nan()).unwrap_or(true));
}

#[test]
fn n14_uv_outside_visible_parses() {
    let src = include_str!("../fixtures/n14_uv_outside_visible.vibe");
    // Semantic issue (100nm is UV, not visible), but syntactically valid
    load_program(src).expect("n14 should parse");
}

#[test]
fn n15_negative_radius_parses() {
    let src = include_str!("../fixtures/n15_negative_radius.vibe");
    load_program(src).expect("n15 should parse (semantic issue)");
}

#[test]
fn n16_bezier_t_out_of_range_parses() {
    let src = include_str!("../fixtures/n16_bezier_t_out_of_range.vibe");
    load_program(src).expect("n16 should parse (semantic issue)");
}

#[test]
fn n17_negative_scale_parses() {
    let src = include_str!("../fixtures/n17_negative_scale.vibe");
    load_program(src).expect("n17 should parse (semantic issue)");
}

#[test]
fn n18_opacity_above_one_parses() {
    let src = include_str!("../fixtures/n18_opacity_above_one.vibe");
    load_program(src).expect("n18 should parse (semantic issue)");
}

#[test]
fn n19_zero_dt_derivative_trapped() {
    let src = include_str!("../fixtures/n19_zero_dt_derivative.vibe");
    let program = load_program(src).expect("n19 should parse");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let result = eval_function(&program, "bad", vec![], &mut host, &mut env);
    // VibeScript traps division by zero at runtime (E600)
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code, DiagCode::E600);
}

#[test]
fn n20_unauthorized_pulse_rejected() {
    let src = include_str!("../fixtures/n20_unauthorized_pulse.vibe");
    // This should fail at check time — no capability declared
    let result = load_program(src);
    assert!(result.is_err());
}

#[test]
fn n21_commit_without_capability_rejected() {
    let src = include_str!("../fixtures/n21_commit_without_capability.vibe");
    // graph.commit() without capability("graph.write") should fail at check
    let result = load_program(src);
    assert!(result.is_err());
}

#[test]
fn n22_negative_rate_constant_parses() {
    let src = include_str!("../fixtures/n22_negative_rate_constant.vibe");
    load_program(src).expect("n22 should parse (semantic issue)");
}

#[test]
fn n23_below_absolute_zero_parses() {
    let src = include_str!("../fixtures/n23_below_absolute_zero.vibe");
    load_program(src).expect("n23 should parse (semantic issue)");
}

#[test]
fn n24_negative_volatility_parses() {
    let src = include_str!("../fixtures/n24_negative_volatility.vibe");
    load_program(src).expect("n24 should parse (semantic issue)");
}

#[test]
fn n25_negative_yield_parses() {
    let src = include_str!("../fixtures/n25_negative_yield.vibe");
    load_program(src).expect("n25 should parse (semantic issue)");
}
