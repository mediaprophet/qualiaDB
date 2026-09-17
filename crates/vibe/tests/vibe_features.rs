//! VibeScript 0.1 feature verification test suite.
//! Tests Pipeline operator, Quantity literals, Record punning, Vector constructors,
//! Embedded Graph queries, Logic Modalities, and 8-Locale bijective completeness.

use vibe::locale::LocaleRegistry;
use vibe::{parse_cell, parse_program, Budget, Engine, Env, ExprKind, Literal, LocalHost, Value};

// ── 1. Pipeline Operator (|>) ──────────────────────────────────────────

#[test]
fn test_pipeline_operator_evaluation() {
    let src = r#"
        pure fn double(x: f64) -> f64 {
            return x * 2.0;
        }
        pure fn add_ten(x: f64) -> f64 {
            return x + 10.0;
        }
        pure fn compute(val: f64) -> f64 {
            return val |> double() |> add_ten();
        }
    "#;
    let prog = parse_program(src).expect("should parse pipeline program");
    let mut host = LocalHost::default();
    let mut engine = Engine::new(&mut host, Budget::default());
    let mut env = Env::default();
    let res = engine
        .call_function(&prog, "compute", vec![Value::F64(5.0)], &mut env)
        .expect("eval");
    assert_eq!(res, Value::F64(20.0)); // (5 * 2) + 10 = 20
}

#[test]
fn test_pipeline_operator_with_multi_args() {
    let src = r#"
        pure fn scale_offset(x: f64, scale: f64, offset: f64) -> f64 {
            return (x * scale) + offset;
        }
        pure fn run(x: f64) -> f64 {
            return x |> scale_offset(3.0, 5.0);
        }
    "#;
    let prog = parse_program(src).expect("should parse");
    let mut host = LocalHost::default();
    let mut engine = Engine::new(&mut host, Budget::default());
    let mut env = Env::default();
    let res = engine
        .call_function(&prog, "run", vec![Value::F64(10.0)], &mut env)
        .expect("eval");
    assert_eq!(res, Value::F64(35.0)); // (10 * 3) + 5 = 35
}

// ── 2. Quantity & Physical Unit Literals ────────────────────────────────

#[test]
fn test_quantity_literals_parse_and_eval() {
    let cell_src = "= 500ms";
    let expr = parse_cell(cell_src).expect("should parse 500ms");
    assert!(matches!(
        expr.kind,
        ExprKind::Literal(Literal::Quantity { .. })
    ));

    let mut host = LocalHost::default();
    let mut engine = Engine::new(&mut host, Budget::default());
    let mut env = Env::default();
    let val = engine.eval_expr(&expr, &mut env).expect("eval");
    if let Value::Quantity(q) = val {
        assert_eq!(q.value, 500.0);
        assert_eq!(q.unit, "ms");
    } else {
        panic!("expected Value::Quantity, got {:?}", val);
    }
}

#[test]
fn test_various_physical_units() {
    let test_cases = vec![
        ("= 60fps", 60.0, "fps"),
        ("= 2.4GHz", 2.4, "GHz"),
        ("= 101.325kPa", 101.325, "kPa"),
        ("= 90deg", 90.0, "deg"),
        ("= 550nm", 550.0, "nm"),
        ("= 15.0[m/s]", 15.0, "m/s"),
    ];

    let mut host = LocalHost::default();
    let mut engine = Engine::new(&mut host, Budget::default());
    for (src, expected_val, expected_unit) in test_cases {
        let expr = parse_cell(src).unwrap_or_else(|e| panic!("failed parsing `{src}`: {e:?}"));
        let mut env = Env::default();
        let val = engine.eval_expr(&expr, &mut env).expect("eval");
        if let Value::Quantity(q) = val {
            assert_eq!(q.value, expected_val);
            assert_eq!(q.unit, expected_unit);
        } else {
            panic!("expected quantity for `{src}`, got {:?}", val);
        }
    }
}

// ── 3. Record Punning & Vector Constructors ────────────────────────────

#[test]
fn test_record_punning() {
    let src = r#"
        pure fn make_point(x: f64, y: f64) -> Record {
            return { x, y };
        }
    "#;
    let prog = parse_program(src).expect("should parse punning");
    let mut host = LocalHost::default();
    let mut engine = Engine::new(&mut host, Budget::default());
    let mut env = Env::default();
    let val = engine
        .call_function(
            &prog,
            "make_point",
            vec![Value::F64(10.0), Value::F64(20.0)],
            &mut env,
        )
        .expect("eval");
    if let Value::Record(map) = val {
        assert_eq!(map.get("x"), Some(&Value::F64(10.0)));
        assert_eq!(map.get("y"), Some(&Value::F64(20.0)));
    } else {
        panic!("expected record, got {:?}", val);
    }
}

#[test]
fn test_vector_constructors() {
    let src = r#"
        pure fn test_vectors() -> List {
            let v2 = vec2(1.0, 2.0);
            let v3 = vec3(1.0, 2.0, 3.0);
            let v4 = vec4(1.0, 2.0, 3.0, 4.0);
            return [v2, v3, v4];
        }
    "#;
    let prog = parse_program(src).expect("should parse vectors");
    let mut host = LocalHost::default();
    let mut engine = Engine::new(&mut host, Budget::default());
    let mut env = Env::default();
    let val = engine
        .call_function(&prog, "test_vectors", vec![], &mut env)
        .expect("eval");
    if let Value::List(vecs) = val {
        assert_eq!(vecs.len(), 3);
        assert_eq!(vecs[0], Value::List(vec![Value::F64(1.0), Value::F64(2.0)]));
        assert_eq!(
            vecs[1],
            Value::List(vec![Value::F64(1.0), Value::F64(2.0), Value::F64(3.0)])
        );
        assert_eq!(
            vecs[2],
            Value::List(vec![
                Value::F64(1.0),
                Value::F64(2.0),
                Value::F64(3.0),
                Value::F64(4.0)
            ])
        );
    } else {
        panic!("expected list of vectors, got {:?}", val);
    }
}

// ── 4. Embedded Graph / SPARQL Query Parsing ───────────────────────────

#[test]
fn test_embedded_graph_pattern_query() {
    let cell_src = "= graph? { ?agent a :AutonomousAgent ; :trustScore ?score }";
    let expr = parse_cell(cell_src).expect("should parse graph query");
    if let ExprKind::GraphQuery {
        is_ask,
        pattern,
        variables,
    } = expr.kind
    {
        assert!(is_ask);
        assert!(pattern.contains("?agent a :AutonomousAgent"));
        assert!(variables.contains(&"agent".to_string()));
        assert!(variables.contains(&"score".to_string()));
    } else {
        panic!("expected ExprKind::GraphQuery, got {:?}", expr.kind);
    }
}

// ── 5. First-Class Logic Modalities ────────────────────────────────────

#[test]
fn test_first_class_modal_logic_blocks() {
    let src = r#"
        pure fn test_modalities() -> List {
            let k = knows("did:q42:alice", "claim_alpha", 0.95);
            let b = believes("did:q42:bob", "claim_beta");
            let obl = obligate { "did:q42:agent_must_sign" };
            let ltl_g = always { "safe_temperature" };
            return [k, b, obl, ltl_g];
        }
    "#;
    let prog = parse_program(src).expect("should parse modal logic");
    let mut host = LocalHost::default();
    let mut engine = Engine::new(&mut host, Budget::default());
    let mut env = Env::default();
    let val = engine
        .call_function(&prog, "test_modalities", vec![], &mut env)
        .expect("eval");
    if let Value::List(modals) = val {
        assert_eq!(modals.len(), 4);
        for m in modals {
            let Value::Record(map) = m else {
                panic!("expected modal term record");
            };
            assert_eq!(map.get("kind"), Some(&Value::String("term".into())));
            assert_eq!(map.get("evaluated"), Some(&Value::Bool(false)));
        }
    } else {
        panic!("expected list of modal logic records, got {:?}", val);
    }
}

// ── 6. 8-Locale Bijective Keyword Completeness ─────────────────────────

#[test]
fn test_all_8_locales_have_full_cardinality() {
    let reg = LocaleRegistry::default();
    assert_eq!(reg.locale_count(), 8);

    // Verify all 8 tables have exact canonical keywords
    for locale in [
        vibe::locale::Locale::EN,
        vibe::locale::Locale::ZH,
        vibe::locale::Locale::ES,
        vibe::locale::Locale::JA,
        vibe::locale::Locale::AR,
        vibe::locale::Locale::HI,
        vibe::locale::Locale::FR,
        vibe::locale::Locale::DE,
    ] {
        let table = reg
            .table_for(locale)
            .unwrap_or_else(|| panic!("missing table for {locale}"));
        assert_eq!(
            table.keywords.len(),
            vibe::locale::ENGLISH_KEYWORDS.len(),
            "table for locale `{locale}` has {} keywords, expected {}",
            table.keywords.len(),
            vibe::locale::ENGLISH_KEYWORDS.len()
        );
    }
}
