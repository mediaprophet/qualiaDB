//! Catalog desugaring, using, color, present, locale opt-in.

use vibe::{parse_program, Budget, Engine, Env, LocalHost, Value};

#[test]
fn using_animation_glass_reveal_is_not_orbit_spin() {
    let src = r#"
        using Animation;
        effect fn run(t: f64) -> Record {
            return Animation.glass_reveal(t);
        }
    "#;
    let orbit = r#"
        using Animation;
        effect fn run(t: f64) -> Record {
            return Animation.orbit_spin(t);
        }
    "#;
    let prog_g = parse_program(src).expect("parse glass");
    let prog_o = parse_program(orbit).expect("parse orbit");
    vibe::check_program(&prog_g).expect("check glass");
    vibe::check_program(&prog_o).expect("check orbit");
    let t = vec![Value::F64(0.2)];
    let mut host = LocalHost::default();
    let mut engine = Engine::new(&mut host, Budget::default());
    let mut env = Env::default();
    let glass = engine
        .call_function(&prog_g, "run", t.clone(), &mut env)
        .expect("eval glass");
    let orbit_v = engine
        .call_function(&prog_o, "run", t, &mut env)
        .expect("eval orbit");
    assert_ne!(
        glass, orbit_v,
        "preset alias must not collapse to orbit_spin"
    );
}

#[test]
fn using_animation_orbit_spin_runs() {
    let src = r#"
        using Animation;
        effect fn run(t: f64) -> Record {
            return Animation.orbit_spin(t);
        }
    "#;
    let prog = parse_program(src).expect("parse");
    vibe::check_program(&prog).expect("check");
    let mut host = LocalHost::default();
    let mut engine = Engine::new(&mut host, Budget::default());
    let mut env = Env::default();
    let val = engine
        .call_function(&prog, "run", vec![Value::F64(1.0)], &mut env)
        .expect("eval");
    let Value::Record(map) = val else {
        panic!("expected record, got {val}");
    };
    assert!(map.contains_key("scalar"));
}

#[test]
fn workshop_invoke_fixtures_parse_and_check() {
    for (name, src) in [
        ("econ1", include_str!("../fixtures/econ1_portfolio.vibe")),
        ("orch1", include_str!("../fixtures/orch1_session.vibe")),
        (
            "asset1",
            include_str!("../fixtures/asset1_aspect_graph.vibe"),
        ),
        ("asset2", include_str!("../fixtures/asset2_persist.vibe")),
        (
            "solvers5",
            include_str!("../fixtures/solvers5_higher_order.vibe"),
        ),
    ] {
        assert!(
            !src.contains("capability.invoke"),
            "{name}: workshop dialect must not teach capability.invoke"
        );
        let prog = parse_program(src).unwrap_or_else(|e| panic!("{name} parse: {e}"));
        vibe::check_program(&prog).unwrap_or_else(|e| panic!("{name} check: {e}"));
    }
}

#[test]
fn unknown_catalog_method_is_compile_error() {
    let src = r#"
        using Animation;
        effect fn run() {
            return Animation.not_a_preset(1.0);
        }
    "#;
    let prog = parse_program(src).expect("parse");
    let err = vibe::check_program(&prog).expect_err("should fail closed");
    assert!(err.message.contains("unknown capability") || err.message.contains("did you mean"));
}

#[test]
fn missing_using_is_e300() {
    let src = r#"
        effect fn run() {
            return HID.poll();
        }
    "#;
    let prog = parse_program(src).expect("parse");
    let err = vibe::check_program(&prog).expect_err("lease required");
    assert!(err.message.contains("missing capability"));
}

#[test]
fn color_literal_and_oklch() {
    let src = r#"
        pure fn palette() -> List {
            let neon = #ff2d95;
            let sky = oklch(0.75, 0.12, 230.0);
            return [neon, sky];
        }
    "#;
    let prog = parse_program(src).expect("parse");
    let mut host = LocalHost::default();
    let mut engine = Engine::new(&mut host, Budget::default());
    let mut env = Env::default();
    let val = engine
        .call_function(&prog, "palette", vec![], &mut env)
        .expect("eval");
    let Value::List(xs) = val else {
        panic!("expected list");
    };
    assert_eq!(xs.len(), 2);
}

#[test]
fn present_block_parses() {
    let src = r#"
        present lamp {
            color: #ff8800
            opacity: 0.9
        }
    "#;
    let prog = parse_program(src).expect("parse");
    vibe::check_program(&prog).expect("check");
    assert!(matches!(prog.items.first(), Some(vibe::Item::Present(_))));
}

#[test]
fn locale_opt_in_does_not_steal_english_fun() {
    let src = r#"
        fn fun() -> i64 {
            return 1;
        }
    "#;
    parse_program(src).expect("fun is a valid English identifier");
}

#[test]
fn locale_zh_if_keyword() {
    let src = "locale zh;\n函数 f() -> i64 { 返回 1; }\n";
    parse_program(src).expect("zh locale keywords after locale zh");
}

#[test]
fn modal_term_is_not_stamped_active() {
    let src = r#"
        pure fn term() -> Record {
            return obligate { "sign" };
        }
    "#;
    let prog = parse_program(src).expect("parse");
    let mut host = LocalHost::default();
    let mut engine = Engine::new(&mut host, Budget::default());
    let mut env = Env::default();
    let val = engine
        .call_function(&prog, "term", vec![], &mut env)
        .expect("eval");
    let Value::Record(map) = val else {
        panic!("expected record");
    };
    assert_eq!(map.get("kind"), Some(&Value::String("term".into())));
    assert_eq!(map.get("evaluated"), Some(&Value::Bool(false)));
    assert_ne!(map.get("status"), Some(&Value::String("Active".into())));
}

#[test]
fn animation_fixture_parses_and_checks() {
    let src = include_str!("../fixtures/animation_presets.vibe");
    let prog = parse_program(src).expect("parse fixture");
    vibe::check_program(&prog).expect("check fixture");
}

#[test]
fn gpu_portal_fixture_parses_and_checks() {
    let src = include_str!("../fixtures/gpu1_portal.vibe");
    let prog = parse_program(src).expect("parse gpu fixture");
    vibe::check_program(&prog).expect("check gpu fixture");
}

#[test]
fn lamp_fixture_parses_checks_and_evals() {
    let src = include_str!("../fixtures/lamp.vibe");
    let prog = parse_program(src).unwrap_or_else(|e| panic!("parse lamp: {e}"));
    vibe::check_program(&prog).unwrap_or_else(|e| panic!("check lamp: {e}"));
    let mut host = LocalHost::default();
    let mut engine = Engine::new(&mut host, Budget::default());
    let mut env = Env::default();
    engine
        .eval_program(&prog, &mut env)
        .unwrap_or_else(|e| panic!("eval lamp: {e}"));
    assert!(env.vars.contains_key("pose"));
    assert!(env.vars.contains_key("glow"));
    assert!(env.vars.contains_key("lamp"));
}

#[test]
fn locale_zh_cbor_ast_hash_round_trip() {
    let src = "locale zh;\n函数 f() -> i64 { 返回 1; }\n";
    let a = parse_program(src).expect("parse");
    let bytes = vibe::encode(&a);
    let b = vibe::decode(&bytes).expect("decode");
    assert_eq!(vibe::encode(&b), bytes);
    assert_eq!(a.locales[0].code, "zh");
}

#[test]
fn locale_ast_round_trips() {
    let src = "locale zh;\n函数 f() -> i64 { 返回 1; }\n";
    let prog = parse_program(src).expect("parse zh");
    assert_eq!(prog.locales.len(), 1);
    assert_eq!(prog.locales[0].code, "zh");
    let bytes = vibe::encode(&prog);
    let back = vibe::decode(&bytes).expect("cbor");
    assert_eq!(back.locales[0].code, "zh");
    let text =
        vibe::decompiler::decompile_program(&back, &vibe::decompiler::DecompileOptions::default());
    assert!(text.contains("locale zh;"));
}

#[test]
fn present_evaluates_to_sheaf() {
    let src = r#"
        present lamp {
            color: #ff8800
            speech: "lamp on"
        }
    "#;
    let prog = parse_program(src).expect("parse");
    vibe::check_program(&prog).expect("check");
    let mut host = LocalHost::default();
    let mut engine = Engine::new(&mut host, Budget::default());
    let mut env = Env::default();
    engine.eval_program(&prog, &mut env).expect("eval");
    let Value::Record(map) = env.vars.get("lamp").expect("lamp bound") else {
        panic!("expected sheaf record");
    };
    assert!(map.contains_key("presentations"));
    assert!(map.contains_key("condition"));
}

#[test]
fn named_cells_eval_in_dag_order() {
    let src = r#"
        cell a := 10;
        cell b := a * 2;
        cell c := b + 5;
    "#;
    let prog = parse_program(src).expect("parse");
    let mut host = LocalHost::default();
    let mut engine = Engine::new(&mut host, Budget::default());
    let mut env = Env::default();
    engine.eval_program(&prog, &mut env).expect("eval");
    assert_eq!(env.vars.get("a"), Some(&Value::I64(10)));
    assert_eq!(env.vars.get("b"), Some(&Value::I64(20)));
    assert_eq!(env.vars.get("c"), Some(&Value::I64(25)));
}

#[test]
fn quantity_add_unit_mismatch_is_check_error() {
    let src = r#"
        pure fn bad() -> f64 {
            return 1.0m + 2.0s;
        }
    "#;
    let prog = parse_program(src).expect("parse");
    let err = vibe::check_program(&prog).expect_err("unit mismatch");
    assert_eq!(err.code, vibe::DiagCode::E100);
    assert!(err.message.contains("unit mismatch"));
}

#[test]
fn graph_query_without_grant_is_e300() {
    let src = r#"
        pure fn q() {
            return graph? { ?s ?p ?o };
        }
    "#;
    let prog = parse_program(src).expect("parse");
    let err = vibe::check_program(&prog).expect_err("lease");
    assert_eq!(err.code, vibe::DiagCode::E300);
}

#[test]
fn modal_lowers_when_family_granted() {
    let src = r#"
        using DeonticLogic;
        pure fn term() -> Record {
            return obligate { "sign" };
        }
    "#;
    let prog = parse_program(src).expect("parse");
    vibe::check_program(&prog).expect("check");
    let mut host = LocalHost::default();
    let mut engine = Engine::with_program(&mut host, Budget::default(), &prog);
    let mut env = Env::default();
    let val = engine
        .call_function(&prog, "term", vec![], &mut env)
        .expect("eval");
    let Value::Record(map) = val else {
        panic!("expected record, got {val}");
    };
    assert_eq!(
        map.get("id"),
        Some(&Value::String("DeonticLogic.evaluate".into()))
    );
    assert_eq!(map.get("evaluated"), Some(&Value::Bool(true)));
    assert_ne!(map.get("status"), Some(&Value::String("Active".into())));
}

#[test]
fn locale_zh_en_projected_ast_hash_equal() {
    let zh = parse_program("locale zh;\n函数 f() -> i64 { 返回 1; }\n").expect("zh");
    let en = parse_program("fn f() -> i64 { return 1; }\n").expect("en");
    let mut zh_items = zh.clone();
    zh_items.locales.clear();
    let projected_zh = vibe::project_program(&zh_items, &vibe::ProjectOptions::default());
    let projected_en = vibe::project_program(&en, &vibe::ProjectOptions::default());
    assert_eq!(
        projected_zh, projected_en,
        "locale is a view; AST is English"
    );
    let a = parse_program(&projected_zh).expect("reparse zh projection");
    let b = parse_program(&projected_en).expect("reparse en projection");
    assert_eq!(vibe::encode(&a), vibe::encode(&b));
}

#[test]
fn workspace_budget_charges_strings() {
    let mut host = LocalHost::default();
    let mut engine = Engine::new(
        &mut host,
        Budget {
            steps_left: 1_000,
            workspace_left: 4,
        },
    );
    let expr = vibe::parse_cell(r#"= "hello""#).expect("parse");
    let mut env = Env::default();
    let err = engine.eval_expr(&expr, &mut env).expect_err("budget");
    assert_eq!(err.code, vibe::DiagCode::E400);
    assert!(err.message.contains("workspace budget"));
}

#[test]
fn eval_quantity_add_keeps_unit() {
    let expr = vibe::parse_cell("= 500ms + 20ms").expect("parse");
    let mut host = LocalHost::default();
    let mut engine = Engine::new(&mut host, Budget::default());
    let mut env = Env::default();
    let val = engine.eval_expr(&expr, &mut env).expect("eval");
    let Value::Quantity(q) = val else {
        panic!("expected Quantity, got {val}");
    };
    assert_eq!(q.unit, "ms");
    assert!((q.value - 520.0).abs() < 1e-9);
}
