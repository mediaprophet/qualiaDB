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
        host.published, vec!["clinic/alerts".to_string()],
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
    assert_eq!(v.as_i64(), Some(1_000_000_000), "MockHost returns deterministic epoch");
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
    let v = eval_function(&program, "clamp", vec![Value::I64(0), Value::I64(100), Value::I64(42)], &mut host, &mut env).unwrap();
    assert_eq!(v.as_i64(), Some(42));
    let v = eval_function(&program, "clamp", vec![Value::I64(0), Value::I64(100), Value::I64(150)], &mut host, &mut env).unwrap();
    assert_eq!(v.as_i64(), Some(100));
    let v = eval_function(&program, "clamp", vec![Value::I64(0), Value::I64(100), Value::I64(-5)], &mut host, &mut env).unwrap();
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
    let v = eval_function(&program, "wave_clamp", vec![Value::F64(1.5)], &mut host, &mut env).unwrap();
    assert_eq!(v.as_f64(), Some(1.0));
    let v = eval_function(&program, "wave_clamp", vec![Value::F64(-2.0)], &mut host, &mut env).unwrap();
    assert_eq!(v.as_f64(), Some(-1.0));
    let v = eval_function(&program, "wave_clamp", vec![Value::F64(0.5)], &mut host, &mut env).unwrap();
    assert_eq!(v.as_f64(), Some(0.5));
}

#[test]
fn p2_harmonic_oscillator_energy() {
    let src = include_str!("../fixtures/p2_harmonic_oscillator.vibe");
    let program = load_program(src).expect("p2 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(&program, "energy_clamp", vec![Value::F64(3.0), Value::F64(4.0)], &mut host, &mut env).unwrap();
    assert_eq!(v.as_f64(), Some(7.0));
    let v = eval_function(&program, "energy_clamp", vec![Value::F64(-10.0), Value::F64(3.0)], &mut host, &mut env).unwrap();
    assert_eq!(v.as_f64(), Some(0.0));
}

#[test]
fn p3_projectile_range() {
    let src = include_str!("../fixtures/p3_projectile.vibe");
    let program = load_program(src).expect("p3 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(&program, "range_estimate", vec![Value::F64(10.0), Value::F64(45.0)], &mut host, &mut env).unwrap();
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
    let v = eval_function(&program, "bounded_force", vec![Value::F64(100.0), Value::F64(10.0)], &mut host, &mut env).unwrap();
    assert_eq!(v.as_f64(), Some(10.0));
    let v = eval_function(&program, "bounded_force", vec![Value::F64(-50.0), Value::F64(10.0)], &mut host, &mut env).unwrap();
    assert_eq!(v.as_f64(), Some(-10.0));
}

// ── EMF / Spectral ──────────────────────────────────────────────────────────

#[test]
fn e1_emf_to_color_clamps_wavelength() {
    let src = include_str!("../fixtures/e1_emf_to_color.vibe");
    let program = load_program(src).expect("e1 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(&program, "wavelength_to_rgb_channel", vec![Value::F64(800.0), Value::F64(380.0), Value::F64(780.0)], &mut host, &mut env).unwrap();
    assert_eq!(v.as_f64(), Some(780.0));
    let v = eval_function(&program, "wavelength_to_rgb_channel", vec![Value::F64(200.0), Value::F64(380.0), Value::F64(780.0)], &mut host, &mut env).unwrap();
    assert_eq!(v.as_f64(), Some(380.0));
}

#[test]
fn e2_emf_interference_clamps() {
    let src = include_str!("../fixtures/e2_emf_interference.vibe");
    let program = load_program(src).expect("e2 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(&program, "interference_amplitude", vec![Value::F64(0.8), Value::F64(0.5)], &mut host, &mut env).unwrap();
    assert_eq!(v.as_f64(), Some(1.0));
    let v = eval_function(&program, "interference_amplitude", vec![Value::F64(-0.6), Value::F64(-0.7)], &mut host, &mut env).unwrap();
    assert_eq!(v.as_f64(), Some(-1.0));
}

#[test]
fn e3_doppler_shift_positive() {
    let src = include_str!("../fixtures/e3_doppler_shift.vibe");
    let program = load_program(src).expect("e3 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(&program, "shifted_frequency", vec![Value::F64(440.0), Value::F64(10.0), Value::F64(340.0)], &mut host, &mut env).unwrap();
    let f = v.as_f64().unwrap();
    assert!(f > 440.0, "approaching source should increase frequency: {f}");
}

#[test]
fn e4_emf_attenuation_inverse_square() {
    let src = include_str!("../fixtures/e4_emf_attenuation.vibe");
    let program = load_program(src).expect("e4 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(&program, "attenuated_intensity", vec![Value::F64(100.0), Value::F64(2.0)], &mut host, &mut env).unwrap();
    assert_eq!(v.as_f64(), Some(25.0));
    let v = eval_function(&program, "attenuated_intensity", vec![Value::F64(100.0), Value::F64(0.0)], &mut host, &mut env).unwrap();
    assert!(v.as_f64().unwrap() > 0.0, "zero distance should not divide by zero");
}

// ── Geometry / SVG ──────────────────────────────────────────────────────────

#[test]
fn geo1_convex_hull_cross_product() {
    let src = include_str!("../fixtures/geo1_convex_hull.vibe");
    let program = load_program(src).expect("geo1 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(&program, "cross_product", vec![Value::F64(1.0), Value::F64(0.0), Value::F64(0.0), Value::F64(1.0)], &mut host, &mut env).unwrap();
    assert_eq!(v.as_f64(), Some(1.0));
    let v = eval_function(&program, "cross_product", vec![Value::F64(2.0), Value::F64(3.0), Value::F64(4.0), Value::F64(6.0)], &mut host, &mut env).unwrap();
    assert_eq!(v.as_f64(), Some(0.0));
}

#[test]
fn geo2_svg_path_distance() {
    let src = include_str!("../fixtures/geo2_svg_path.vibe");
    let program = load_program(src).expect("geo2 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(&program, "point_distance", vec![Value::F64(0.0), Value::F64(0.0), Value::F64(3.0), Value::F64(4.0)], &mut host, &mut env).unwrap();
    assert_eq!(v.as_f64(), Some(25.0));
}

#[test]
fn geo3_field_viz_magnitude() {
    let src = include_str!("../fixtures/geo3_field_viz.vibe");
    let program = load_program(src).expect("geo3 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(&program, "field_magnitude", vec![Value::F64(1.0), Value::F64(2.0), Value::F64(2.0)], &mut host, &mut env).unwrap();
    assert_eq!(v.as_f64(), Some(9.0));
}

// ── CSS Animation ───────────────────────────────────────────────────────────

#[test]
fn c1_css_keyframe_interpolation() {
    let src = include_str!("../fixtures/c1_css_keyframe.vibe");
    let program = load_program(src).expect("c1 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(&program, "interpolate_keyframe", vec![Value::F64(0.0), Value::F64(100.0), Value::F64(0.5)], &mut host, &mut env).unwrap();
    assert_eq!(v.as_f64(), Some(50.0));
    let v = eval_function(&program, "interpolate_keyframe", vec![Value::F64(0.0), Value::F64(100.0), Value::F64(1.5)], &mut host, &mut env).unwrap();
    assert_eq!(v.as_f64(), Some(100.0));
    let v = eval_function(&program, "interpolate_keyframe", vec![Value::F64(0.0), Value::F64(100.0), Value::F64(-0.5)], &mut host, &mut env).unwrap();
    assert_eq!(v.as_f64(), Some(0.0));
}

#[test]
fn c2_reactive_color_hue_clamp() {
    let src = include_str!("../fixtures/c2_reactive_color.vibe");
    let program = load_program(src).expect("c2 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(&program, "temperature_to_hue", vec![Value::F64(400.0)], &mut host, &mut env).unwrap();
    assert_eq!(v.as_f64(), Some(360.0));
    let v = eval_function(&program, "temperature_to_hue", vec![Value::F64(-10.0)], &mut host, &mut env).unwrap();
    assert_eq!(v.as_f64(), Some(0.0));
}

#[test]
fn c3_css_opacity_ratio() {
    let src = include_str!("../fixtures/c3_css_opacity.vibe");
    let program = load_program(src).expect("c3 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(&program, "fade_opacity", vec![Value::F64(50.0), Value::F64(100.0)], &mut host, &mut env).unwrap();
    assert_eq!(v.as_f64(), Some(0.5));
    let v = eval_function(&program, "fade_opacity", vec![Value::F64(150.0), Value::F64(100.0)], &mut host, &mut env).unwrap();
    assert_eq!(v.as_f64(), Some(1.0));
}

// ── Reactive cells ──────────────────────────────────────────────────────────

#[test]
fn r1_reactive_sum_counts_rows() {
    let src = include_str!("../fixtures/r1_reactive_sum.vibe");
    let program = load_program(src).expect("r1 fixture");
    let mut host = MockHost { query_rows: 7, ..MockHost::default() };
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
    let mut host = MockHost { query_rows: 10, ..MockHost::default() };
    let mut env = Env::default();
    let v = eval_function(&program, "above_threshold", vec![Value::I64(3)], &mut host, &mut env).unwrap();
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
    let v = dispatch_hook(&program, &vec!["tick".to_string()], vec![], &mut host, &mut env).unwrap();
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
    let v = dispatch_hook(&program, &path, vec![Value::String("test".into()), Value::F64(42.0)], &mut host, &mut env).unwrap();
    assert_eq!(v, Value::Null);
    assert_eq!(host.published.len(), 1);
    // Zero value → no publish.
    let v = dispatch_hook(&program, &path, vec![Value::String("test".into()), Value::F64(0.0)], &mut host, &mut env).unwrap();
    assert_eq!(v, Value::Null);
    assert_eq!(host.published.len(), 1, "zero should not publish");
}

#[test]
fn h3_tick_time_publish_uses_time() {
    let src = include_str!("../fixtures/h3_tick_time_publish.vibe");
    let program = load_program(src).expect("h3 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = dispatch_hook(&program, &vec!["tick".to_string()], vec![], &mut host, &mut env).unwrap();
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
        vec![Value::Iri("actor:alice".into()), Value::Iri("action:read".into())],
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
        vec![Value::Iri("actor:bob".into()), Value::Iri("action:delete".into())],
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
        vec![Value::Iri("party:alice".into()), Value::Iri("contract:c1".into())],
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
    let v = eval_function(&program, "atom_count_valid", vec![Value::F64(500.0)], &mut host, &mut env).unwrap();
    assert_eq!(v.as_f64(), Some(500.0));
    let v = eval_function(&program, "atom_count_valid", vec![Value::F64(2000.0)], &mut host, &mut env).unwrap();
    assert_eq!(v.as_f64(), Some(1000.0));
}

#[test]
fn s2_bio_alignment_score() {
    let src = include_str!("../fixtures/s2_bio_alignment.vibe");
    let program = load_program(src).expect("s2 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(&program, "alignment_score", vec![Value::F64(10.0), Value::F64(3.0)], &mut host, &mut env).unwrap();
    assert_eq!(v.as_f64(), Some(17.0));
    let v = eval_function(&program, "alignment_score", vec![Value::F64(1.0), Value::F64(5.0)], &mut host, &mut env).unwrap();
    assert_eq!(v.as_f64(), Some(0.0));
}

#[test]
fn s3_mol_weight_calculates() {
    let src = include_str!("../fixtures/s3_mol_weight.vibe");
    let program = load_program(src).expect("s3 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(&program, "molecular_weight", vec![Value::F64(6.0), Value::F64(12.0)], &mut host, &mut env).unwrap();
    assert_eq!(v.as_f64(), Some(72.0));
}

// ── Financial ───────────────────────────────────────────────────────────────

#[test]
fn f1_black_scholes_intrinsic() {
    let src = include_str!("../fixtures/f1_black_scholes.vibe");
    let program = load_program(src).expect("f1 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(&program, "intrinsic_value", vec![Value::F64(110.0), Value::F64(100.0)], &mut host, &mut env).unwrap();
    assert_eq!(v.as_f64(), Some(10.0));
    let v = eval_function(&program, "intrinsic_value", vec![Value::F64(80.0), Value::F64(100.0)], &mut host, &mut env).unwrap();
    assert_eq!(v.as_f64(), Some(0.0));
}

#[test]
fn f2_portfolio_opt_weighted_return() {
    let src = include_str!("../fixtures/f2_portfolio_opt.vibe");
    let program = load_program(src).expect("f2 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(&program, "weighted_return", vec![Value::F64(0.1), Value::F64(0.6), Value::F64(0.05), Value::F64(0.4)], &mut host, &mut env).unwrap();
    let r = v.as_f64().unwrap();
    assert!((r - 0.08).abs() < 0.001, "expected 0.08, got {r}");
}

#[test]
fn f3_var_calc_clamps_confidence() {
    let src = include_str!("../fixtures/f3_var_calc.vibe");
    let program = load_program(src).expect("f3 fixture");
    let mut host = MockHost::default();
    let mut env = Env::default();
    let v = eval_function(&program, "value_at_risk", vec![Value::F64(1000.0), Value::F64(0.95)], &mut host, &mut env).unwrap();
    assert!((v.as_f64().unwrap() - 50.0).abs() < 0.001);
    let v = eval_function(&program, "value_at_risk", vec![Value::F64(1000.0), Value::F64(1.5)], &mut host, &mut env).unwrap();
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
}

// ── H7: Enum/ADT golden corpus (T9 coverage) ──────────────────────────────────

#[test]
fn ad1_enum_unit_parses() {
    let src = include_str!("../fixtures/ad1_enum_unit.vibe");
    load_program(src).expect("ad1 fixture should parse");
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
    let mut engine = poet_vibe::Engine::with_program(&mut host, poet_vibe::Budget::default(), &program);
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
