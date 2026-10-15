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

// ── 7. Value-Producing Conditional Expressions (`if ... else ...`) ─────

#[test]
fn test_if_expr_walk_speed_and_lazy_evaluation() {
    let src = r#"
        module <https://civics.au/game/modules/character_arcs>;

        pure fn walk_speed(trait_id: string, running: bool) -> f64 {
            let base = if running { 5.0 } else { 2.6 };
            if trait_id == "trailwise" && !running { return base * 1.25; }
            if trait_id == "steadfast" && running { return base * 1.15; }
            return base;
        }

        pure fn lazy_branch(flag: bool) -> i64 {
            return if flag { 42 } else { 1 / 0 };
        }
    "#;
    let prog = vibe::load_program(src).expect("should parse and check character_arcs");
    let mut host = LocalHost::default();
    let mut engine = Engine::with_program(&mut host, Budget::default(), &prog);
    let mut env = Env::default();

    let v1 = engine
        .call_function(
            &prog,
            "walk_speed",
            vec![Value::String("trailwise".into()), Value::Bool(false)],
            &mut env,
        )
        .expect("eval trailwise walk");
    assert_eq!(v1, Value::F64(3.25));

    let v2 = engine
        .call_function(
            &prog,
            "walk_speed",
            vec![Value::String("steadfast".into()), Value::Bool(true)],
            &mut env,
        )
        .expect("eval steadfast run");
    assert_eq!(v2, Value::F64(5.75));

    let v3 = engine
        .call_function(
            &prog,
            "walk_speed",
            vec![Value::String("ordinary".into()), Value::Bool(false)],
            &mut env,
        )
        .expect("eval ordinary walk");
    assert_eq!(v3, Value::F64(2.6));

    let v4 = engine
        .call_function(
            &prog,
            "walk_speed",
            vec![Value::String("ordinary".into()), Value::Bool(true)],
            &mut env,
        )
        .expect("eval ordinary run");
    assert_eq!(v4, Value::F64(5.0));

    // Unselected branch with division-by-zero must not execute
    let lazy = engine
        .call_function(&prog, "lazy_branch", vec![Value::Bool(true)], &mut env)
        .expect("lazy branch should not evaluate unselected division by zero");
    assert_eq!(lazy, Value::I64(42));
}

#[test]
fn test_if_expr_chained_records_and_bytecode_parity() {
    let src = r#"
        pure fn classify(labour: bool, coins: f64, is_commons: bool) -> Record {
            let action = if labour {
                "connectMastLabour"
            } else if coins >= 3.0 {
                "connectMastCoins"
            } else {
                "endTurn"
            };
            return {
                title: if is_commons { "The commons are open" } else { "The canal is connected" },
                action: action,
                bonus: (if is_commons { 10 } else { 20 }) + 5
            };
        }
    "#;
    let prog = vibe::load_program(src).expect("load_program");

    // Round-trip CBOR AST
    let bytes = vibe::encode(&prog);
    let decoded = vibe::decode(&bytes).expect("decode CBOR AST");
    assert_eq!(vibe::encode(&decoded), bytes);

    // Round-trip decompile & projectional
    let decompiled =
        vibe::decompiler::decompile_program(&prog, &vibe::decompiler::DecompileOptions::default());
    let reparsed = parse_program(&decompiled).expect("reparse decompiled");
    vibe::check_program(&reparsed).expect("check reparsed");

    let projected =
        vibe::projectional::project_program(&prog, &vibe::projectional::ProjectOptions::default());
    let reparsed_proj = parse_program(&projected).expect("reparse projected");
    vibe::check_program(&reparsed_proj).expect("check reparsed projected");

    // Interpreter vs Bytecode VM parity
    let chunk = vibe::bytecode::compile(&prog).expect("compile bytecode");
    for (labour, coins, is_commons, exp_action, exp_title, exp_bonus) in [
        (
            true,
            0.0,
            true,
            "connectMastLabour",
            "The commons are open",
            15,
        ),
        (
            false,
            4.0,
            false,
            "connectMastCoins",
            "The canal is connected",
            25,
        ),
        (false, 1.0, true, "endTurn", "The commons are open", 15),
    ] {
        let args = vec![
            Value::Bool(labour),
            Value::F64(coins),
            Value::Bool(is_commons),
        ];
        let mut host = LocalHost::default();
        let mut engine = Engine::with_program(&mut host, Budget::default(), &prog);
        let mut env = Env::default();
        let interp_val = engine
            .call_function(&prog, "classify", args.clone(), &mut env)
            .expect("interp eval");

        let mut vm_host = LocalHost::default();
        let mut vm = vibe::bytecode::Vm::new(&chunk, &mut vm_host, Budget::default());
        let vm_val = vm.call_function(0, &args).expect("bytecode vm eval");

        assert_eq!(interp_val, vm_val);
        let Value::Record(map) = interp_val else {
            panic!("expected record");
        };
        assert_eq!(map.get("action"), Some(&Value::String(exp_action.into())));
        assert_eq!(map.get("title"), Some(&Value::String(exp_title.into())));
        assert_eq!(map.get("bonus"), Some(&Value::I64(exp_bonus)));
    }
}

#[test]
fn test_if_expr_static_checks_and_parse_diagnostics() {
    // Missing else in expression position is rejected
    let err = parse_program("pure fn f(x: bool) -> i64 { return if x { 1 }; }")
        .expect_err("missing else should fail");
    assert_eq!(err.code, vibe::DiagCode::E001);
    assert!(err.message.contains("else"));

    // Empty branch is rejected
    let err = parse_program("pure fn f(x: bool) -> i64 { return if x {} else { 1 }; }")
        .expect_err("empty branch should fail");
    assert_eq!(err.code, vibe::DiagCode::E001);

    // Trailing semicolon inside branch is rejected
    let err = parse_program("pure fn f(x: bool) -> i64 { return if x { 1; } else { 2 }; }")
        .expect_err("semicolon in branch should fail");
    assert_eq!(err.code, vibe::DiagCode::E001);

    // Unparenthesized postfix on conditional is rejected
    let err = parse_program(
        "pure fn f(x: bool) -> i64 { return if x { { a: 1 } } else { { a: 2 } }.a; }",
    )
    .expect_err("unparenthesized postfix should fail");
    assert_eq!(err.code, vibe::DiagCode::E001);

    // Static capability check inspects both branches even if condition is constant true
    let prog = parse_program(
        r#"
        effect fn sneaky() {
            let x = if true { 1 } else { pulse.publish("t", {}) };
        }
        "#,
    )
    .expect("parse sneaky");
    let check_err =
        vibe::check_program(&prog).expect_err("missing capability in else branch must fail");
    assert_eq!(check_err.code, vibe::DiagCode::E300);
}

#[test]
fn test_game_demo_campaign_rules_pattern_loads_and_evaluates() {
    let src = r#"
        module <https://civics.au/game/modules/campaign_rules>;

        pure fn campaign_lead(st: Record) -> Record {
            if st.finale != "" {
                let is_commons = st.finale == "\"commons\"" || st.finale == "commons";
                return {
                    title: if is_commons { "The commons are open" } else { "The canal is connected" },
                    copy: if is_commons {
                        "You chose a local gathering and opened a shared place to celebrate the harvest."
                    } else {
                        "You chose a working trade link, carrying Saltwind’s produce to neighbouring communities."
                    },
                    label: "Campaign complete · explore the map",
                    action: "",
                    territory: "saltwind"
                };
            }

            let held_panel = st.held.contains("rc:partPanel");
            let fitted_panel = st.parts.contains("rc:partPanel");
            let mut action = "";
            if !st.committeeFormed {
                action = if st.supporters > 0.0 { "formCommittee" } else { "canvassMara" };
            } else if st.fundingFailed {
                action = if st.supporters < 2.0 {
                    "canvassSecondResident"
                } else if st.coins >= 3.0 {
                    "selfFundProject"
                } else {
                    "raiseCommunityFunds"
                };
            } else if !fitted_panel && !held_panel {
                action = if st.labour { "salvagePanel" } else if st.coins >= 4.0 { "buyPanel" } else { "endTurn" };
            } else {
                action = "switchPowerOn";
            }

            let label = if action != "" { "Continue" } else { "Choose a funding, procurement, or challenge path" };
            return {
                title: if st.day <= 4.0 { "Reach the far bank" } else { "Cross the high canal" },
                label: label,
                action: action,
                territory: "kestrel"
            };
        }
    "#;
    let prog = vibe::load_program(src).expect("should load campaign_rules");
    let mut host = LocalHost::default();
    let mut engine = Engine::with_program(&mut host, Budget::default(), &prog);
    let mut env = Env::default();

    let mut st = std::collections::BTreeMap::new();
    st.insert("finale".into(), Value::String("".into()));
    st.insert("held".into(), Value::List(vec![]));
    st.insert("parts".into(), Value::List(vec![]));
    st.insert("committeeFormed".into(), Value::Bool(true));
    st.insert("fundingFailed".into(), Value::Bool(true));
    st.insert("supporters".into(), Value::F64(2.0));
    st.insert("coins".into(), Value::F64(3.5));
    st.insert("labour".into(), Value::Bool(false));
    st.insert("day".into(), Value::F64(5.0));

    let res = engine
        .call_function(&prog, "campaign_lead", vec![Value::Record(st)], &mut env)
        .expect("eval campaign_lead");
    let Value::Record(map) = res else {
        panic!("expected Record");
    };
    assert_eq!(
        map.get("action"),
        Some(&Value::String("selfFundProject".into()))
    );
    assert_eq!(
        map.get("title"),
        Some(&Value::String("Cross the high canal".into()))
    );
    assert_eq!(map.get("label"), Some(&Value::String("Continue".into())));
}
