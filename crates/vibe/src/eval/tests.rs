    use super::*;
    use crate::bind::LocalHost;
    use crate::deontic_interrupt::{Phase, PhaseLeaser};

    #[test]
    fn r2_no_phase_leaser_backward_compatible() {
        // Without a phase leaser, the engine behaves exactly as before.
        let mut host = LocalHost::default();
        let mut env = Env::default();
        let mut engine = Engine::new(&mut host, Budget::default());
        let expr = crate::parse::parse_cell("= math.max(1, 2)").unwrap();
        let result = engine.eval_expr(&expr, &mut env);
        assert!(result.is_ok());
    }

    #[test]
    fn r2_phase_leaser_allows_leased_capability() {
        let mut host = LocalHost::default();
        let mut leaser = PhaseLeaser::new();
        leaser
            .register_phase(Phase::new("execute").allow("math"))
            .unwrap();
        leaser.enter_phase("execute").unwrap();

        let mut env = Env::default();
        let mut engine = Engine::with_phase_leaser(&mut host, Budget::default(), &mut leaser);
        let expr = crate::parse::parse_cell("= math.max(1, 2)").unwrap();
        let result = engine.eval_expr(&expr, &mut env);
        assert!(result.is_ok(), "math should be allowed: {:?}", result);
    }

    #[test]
    fn r2_phase_leaser_blocks_unleased_capability() {
        let mut host = LocalHost::default();
        let mut leaser = PhaseLeaser::new();
        // Only allow "math" in this phase â€” "graph" is not leased.
        leaser
            .register_phase(Phase::new("execute").allow("math"))
            .unwrap();
        leaser.enter_phase("execute").unwrap();

        let mut env = Env::default();
        let mut engine = Engine::with_phase_leaser(&mut host, Budget::default(), &mut leaser);
        // graph.query is not allowed in this phase.
        let expr = crate::parse::parse_cell("= graph.query(?s, ?p, ?o, take: 10)").unwrap();
        let result = engine.eval_expr(&expr, &mut env);
        assert!(result.is_err(), "graph should be blocked");
        let err = result.unwrap_err();
        assert_eq!(err.code, DiagCode::E700);
        assert!(err.message.contains("graph"));
    }

    #[test]
    fn r2_phase_leaser_blocks_after_interrupt() {
        let mut host = LocalHost::default();
        let mut leaser = PhaseLeaser::new();
        leaser
            .register_phase(Phase::new("execute").allow("math"))
            .unwrap();
        leaser.enter_phase("execute").unwrap();

        // Trigger an interrupt â€” all capabilities should be revoked.
        let interrupt = crate::deontic_interrupt::DeonticInterrupt::prohibition_breach(
            "graph", "execute", None,
        );
        leaser.trigger_interrupt(interrupt);

        let mut env = Env::default();
        let mut engine = Engine::with_phase_leaser(&mut host, Budget::default(), &mut leaser);
        let expr = crate::parse::parse_cell("= math.max(1, 2)").unwrap();
        let result = engine.eval_expr(&expr, &mut env);
        assert!(result.is_err(), "math should be blocked after interrupt");
        let err = result.unwrap_err();
        assert_eq!(err.code, DiagCode::E700);
        assert!(err.message.contains("halted"));
    }

    // â”€â”€ T9: User-defined enum / match ADT tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    fn eval_program_src(src: &str) -> Result<Value, Diagnostic> {
        let program = crate::parse::parse_program(src)?;
        crate::check::check_program(&program)?;
        let mut host = LocalHost::default();
        let mut env = Env::default();
        // Register enums and consts first.
        let mut engine = Engine::with_program(&mut host, Budget::default(), &program);
        engine.eval_program(&program, &mut env)?;
        // Then call main().
        crate::eval_function(&program, "main", Vec::new(), &mut host, &mut env)
    }

    #[test]
    fn t9_enum_unit_variant_construction() {
        let src = r#"
            enum Shape {
                Point,
                Circle(f64),
            }
            fn main() {
                return Shape.Point;
            }
        "#;
        let result = eval_program_src(src).unwrap();
        let e = result.as_enum().unwrap();
        assert_eq!(e.enum_name, "Shape");
        assert_eq!(e.variant_name, "Point");
        assert!(e.payload.is_empty());
    }

    #[test]
    fn t9_enum_payload_variant_construction() {
        let src = r#"
            enum Shape {
                Point,
                Circle(f64),
            }
            fn main() {
                return Shape.Circle(3.14);
            }
        "#;
        let result = eval_program_src(src).unwrap();
        let e = result.as_enum().unwrap();
        assert_eq!(e.enum_name, "Shape");
        assert_eq!(e.variant_name, "Circle");
        assert_eq!(e.payload.len(), 1);
        assert_eq!(e.payload[0], Value::F64(3.14));
    }

    #[test]
    fn t9_enum_match_unit_variant() {
        let src = r#"
            enum Shape {
                Point,
                Circle(f64),
            }
            fn main() {
                let s = Shape.Point;
                match s {
                    Shape.Point => 0,
                    Shape.Circle(r) => 1,
                }
            }
        "#;
        let result = eval_program_src(src).unwrap();
        assert_eq!(result, Value::I64(0));
    }

    #[test]
    fn t9_enum_match_payload_variant_binds_inner() {
        let src = r#"
            enum Shape {
                Point,
                Circle(f64),
            }
            fn main() {
                let s = Shape.Circle(5.0);
                match s {
                    Shape.Point => 0,
                    Shape.Circle(r) => r,
                }
            }
        "#;
        let result = eval_program_src(src).unwrap();
        assert_eq!(result, Value::F64(5.0));
    }

    #[test]
    fn t9_enum_match_multi_payload_variant() {
        let src = r#"
            enum Shape {
                Point,
                Circle(f64),
                Rect(f64, f64),
            }
            fn main() {
                let s = Shape.Rect(3.0, 4.0);
                match s {
                    Shape.Point => 0,
                    Shape.Circle(r) => r,
                    Shape.Rect(w, h) => w * h,
                }
            }
        "#;
        let result = eval_program_src(src).unwrap();
        assert_eq!(result, Value::F64(12.0));
    }

    #[test]
    fn t9_enum_match_wildcard_fallback() {
        let src = r#"
            enum Color {
                Red,
                Green,
                Blue,
            }
            fn main() {
                let c = Color.Green;
                match c {
                    Color.Red => 0xFF0000,
                    _ => 0x00FF00,
                }
            }
        "#;
        let result = eval_program_src(src).unwrap();
        assert_eq!(result, Value::I64(0x00FF00));
    }

    #[test]
    fn t9_enum_match_wrong_variant_does_not_match() {
        let src = r#"
            enum Shape {
                Point,
                Circle(f64),
            }
            fn main() {
                let s = Shape.Circle(1.0);
                match s {
                    Shape.Point => 0,
                    Shape.Circle(r) => 1,
                }
            }
        "#;
        let result = eval_program_src(src).unwrap();
        assert_eq!(result, Value::I64(1));
    }

    #[test]
    fn mut_let_allows_reassignment() {
        let src = r#"
            fn main() {
                let mut x = 1;
                x = 2;
                return x;
            }
        "#;
        let result = eval_program_src(src).unwrap();
        assert_eq!(result, Value::I64(2));
    }

    #[test]
    fn let_rejects_reassignment() {
        let src = r#"
            fn main() {
                let x = 1;
                x = 2;
                return x;
            }
        "#;
        let err = eval_program_src(src).unwrap_err();
        assert_eq!(err.code, DiagCode::E701);
    }

    #[test]
    fn mut_check_catches_reassignment_in_block() {
        let src = r#"
            fn main() {
                let mut x = 1;
                {
                    let y = 10;
                    y = 20;
                }
                return x;
            }
        "#;
        let err = eval_program_src(src).unwrap_err();
        assert_eq!(err.code, DiagCode::E701);
    }

    #[test]
    fn i64_add_overflow() {
        let src = r#"
            fn main() {
                let x = 9223372036854775807 + 1;
                return x;
            }
        "#;
        let err = eval_program_src(src).unwrap_err();
        assert_eq!(err.code, DiagCode::E600);
    }

    #[test]
    fn i64_sub_overflow() {
        let src = r#"
            fn main() {
                let x = -9223372036854775808 - 1;
                return x;
            }
        "#;
        let err = eval_program_src(src).unwrap_err();
        assert_eq!(err.code, DiagCode::E600);
    }

    #[test]
    fn i64_mul_overflow() {
        let src = r#"
            fn main() {
                let x = 9223372036854775807 * 2;
                return x;
            }
        "#;
        let err = eval_program_src(src).unwrap_err();
        assert_eq!(err.code, DiagCode::E600);
    }

    #[test]
    fn u64_add_overflow() {
        let src = r#"
            fn main() {
                let x = 18446744073709551615 + 1;
                return x;
            }
        "#;
        let err = eval_program_src(src).unwrap_err();
        assert_eq!(err.code, DiagCode::E600);
    }

    #[test]
    fn i64_normal_add_still_works() {
        let src = r#"
            fn main() {
                let x = 1 + 2;
                return x;
            }
        "#;
        let result = eval_program_src(src).unwrap();
        assert_eq!(result, Value::I64(3));
    }

    #[test]
    fn f64_overflow_not_checked() {
        let src = r#"
            fn main() {
                let x = 1e308 * 1e308;
                return x;
            }
        "#;
        let result = eval_program_src(src).unwrap();
        assert_eq!(result, Value::F64(f64::INFINITY));
    }

    #[test]
    fn math_abs_i64_returns_i64() {
        let src = r#"
            import "vibe:0.1/math" as math;
            fn main() {
                return math.abs(-5);
            }
        "#;
        let result = eval_program_src(src).unwrap();
        assert_eq!(result, Value::I64(5));
    }

    #[test]
    fn math_abs_f64_returns_f64() {
        let src = r#"
            import "vibe:0.1/math" as math;
            fn main() {
                return math.abs(-5.0);
            }
        "#;
        let result = eval_program_src(src).unwrap();
        assert_eq!(result, Value::F64(5.0));
    }

    #[test]
    fn math_min_i64_returns_i64() {
        let src = r#"
            import "vibe:0.1/math" as math;
            fn main() {
                return math.min(3, 7);
            }
        "#;
        let result = eval_program_src(src).unwrap();
        assert_eq!(result, Value::I64(3));
    }

    #[test]
    fn math_max_i64_returns_i64() {
        let src = r#"
            import "vibe:0.1/math" as math;
            fn main() {
                return math.max(3, 7);
            }
        "#;
        let result = eval_program_src(src).unwrap();
        assert_eq!(result, Value::I64(7));
    }

    #[test]
    fn math_floor_i64_returns_i64() {
        let src = r#"
            import "vibe:0.1/math" as math;
            fn main() {
                return math.floor(5);
            }
        "#;
        let result = eval_program_src(src).unwrap();
        assert_eq!(result, Value::I64(5));
    }

    #[test]
    fn math_sqrt_always_f64() {
        let src = r#"
            import "vibe:0.1/math" as math;
            fn main() {
                return math.sqrt(9);
            }
        "#;
        let result = eval_program_src(src).unwrap();
        assert_eq!(result, Value::F64(3.0));
    }

    #[test]
    fn import_resolves_alias() {
        let src = r#"
            import <vibe:0.1/math> as math;
            fn main() {
                return math.sin(0.0);
            }
        "#;
        let result = eval_program_src(src).unwrap();
        assert_eq!(result, Value::F64(0.0));
    }

    #[test]
    fn import_unknown_iri_fails() {
        let src = r#"
            import <vibe:0.1/nonexistent> as foo;
            fn main() {
                return 1;
            }
        "#;
        let err = eval_program_src(src).unwrap_err();
        assert_eq!(err.code, DiagCode::E100);
    }

    #[test]
    fn import_dupe_alias_fails() {
        let src = r#"
            import <vibe:0.1/math> as m;
            import <vibe:0.1/time> as m;
            fn main() {
                return 1;
            }
        "#;
        let err = eval_program_src(src).unwrap_err();
        assert_eq!(err.code, DiagCode::E100);
    }

    // â”€â”€ T28: FieldDecl tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn t28_field_decl_parses_and_evaluates() {
        let src = r#"
            field pressure_ambient: Pressure
                unit: "qudt:KiloPascal"
                support: region
                representation: grid;
            fn main() {
                return pressure_ambient;
            }
        "#;
        let result = eval_program_src(src).unwrap();
        match result {
            Value::Record(r) => {
                assert_eq!(
                    r.get("name"),
                    Some(&Value::String("pressure_ambient".into()))
                );
                assert_eq!(r.get("ty"), Some(&Value::String("Pressure".into())));
                assert_eq!(
                    r.get("unit"),
                    Some(&Value::String("qudt:KiloPascal".into()))
                );
                assert_eq!(r.get("support"), Some(&Value::String("region".into())));
                assert_eq!(r.get("representation"), Some(&Value::String("grid".into())));
            }
            other => panic!("expected record, got {other:?}"),
        }
    }

    #[test]
    fn t28_field_decl_without_unit() {
        let src = r#"
            field temperature: Temperature
                support: point
                representation: sampled;
            fn main() {
                return temperature;
            }
        "#;
        let result = eval_program_src(src).unwrap();
        match result {
            Value::Record(r) => {
                assert_eq!(r.get("name"), Some(&Value::String("temperature".into())));
                assert!(r.get("unit").is_none());
                assert_eq!(r.get("support"), Some(&Value::String("point".into())));
                assert_eq!(
                    r.get("representation"),
                    Some(&Value::String("sampled".into()))
                );
            }
            other => panic!("expected record, got {other:?}"),
        }
    }

    #[test]
    fn t28_field_decl_defaults() {
        let src = r#"
            field velocity: Velocity;
            fn main() {
                return velocity;
            }
        "#;
        let result = eval_program_src(src).unwrap();
        match result {
            Value::Record(r) => {
                assert_eq!(r.get("support"), Some(&Value::String("region".into())));
                assert_eq!(r.get("representation"), Some(&Value::String("grid".into())));
            }
            other => panic!("expected record, got {other:?}"),
        }
    }

    // â”€â”€ T29: MaterialDecl tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn t29_material_decl_parses_and_evaluates() {
        let src = r#"
            material sucrose_cube: Material
                yield: 50.0,
                density: 1580.0;
            fn main() {
                return sucrose_cube;
            }
        "#;
        let result = eval_program_src(src).unwrap();
        match result {
            Value::Record(r) => {
                assert_eq!(r.get("name"), Some(&Value::String("sucrose_cube".into())));
                assert_eq!(r.get("yield"), Some(&Value::F64(50.0)));
                assert_eq!(r.get("density"), Some(&Value::F64(1580.0)));
            }
            other => panic!("expected record, got {other:?}"),
        }
    }

    #[test]
    fn t29_material_decl_no_properties() {
        let src = r#"
            material empty_mat: Material;
            fn main() {
                return empty_mat;
            }
        "#;
        let result = eval_program_src(src).unwrap();
        match result {
            Value::Record(r) => {
                assert_eq!(r.get("name"), Some(&Value::String("empty_mat".into())));
            }
            other => panic!("expected record, got {other:?}"),
        }
    }

    // â”€â”€ T30: LawDecl tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn t29_part_signature_facet_resolves_on_one_stack() {
        let src = r#"
            material tank_shell: Material
                part: "did:q42:part:polymer-shell",
                signature: "hdpe_tank_shell",
                reading: "colour";
            fn main() {
                return tank_shell;
            }
        "#;
        let result = eval_program_src(src).unwrap();
        match result {
            Value::Record(r) => {
                assert_eq!(
                    r.get("part"),
                    Some(&Value::String("did:q42:part:polymer-shell".into()))
                );
                assert_eq!(
                    r.get("signature"),
                    Some(&Value::String("did:q42:material:hdpe-tank-shell-v1".into()))
                );
                assert_eq!(r.get("facet"), Some(&Value::String("optical".into())));
                assert_eq!(r.get("reading"), Some(&Value::String("colour".into())));
                assert_eq!(r.get("spectrum_axis"), Some(&Value::String("emf".into())));
                assert_eq!(r.get("writes_spatial"), Some(&Value::Bool(false)));
                assert_eq!(r.get("baked_frame"), Some(&Value::Bool(false)));
                assert_eq!(r.get("subsumes_whole"), Some(&Value::Bool(false)));
                assert_eq!(r.get("construct"), Some(&Value::String("editable".into())));
                match r.get("fields") {
                    Some(Value::Record(fields)) => {
                        assert!(fields.contains_key("albedo"));
                    }
                    other => panic!("fields {other:?}"),
                }
            }
            other => panic!("expected record, got {other:?}"),
        }
    }

    #[test]
    fn t29_water_form_is_not_liquid_water_signature() {
        let src = r#"
            material bad: Material
                part: "did:q42:part:someone",
                signature: "grey",
                facet: "chemical";
            fn main() {
                return bad;
            }
        "#;
        let err = eval_program_src(src).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("liquid_water") || msg.contains("water form"),
            "{msg}"
        );
    }

    #[test]
    fn t29_flora_is_not_a_material_signature() {
        let src = r#"
            material kind: Material
                part: "did:q42:part:organism",
                signature: "flora",
                reading: "colour";
            fn main() {
                return kind;
            }
        "#;
        let err = eval_program_src(src).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("flora"), "{msg}");
        assert!(!msg.contains("Sucrose"));
    }

    #[test]
    fn t30_law_decl_parses_and_evaluates() {
        let src = r#"
            field pressure_ambient: Pressure
                support: region
                representation: grid;
            material sucrose_cube: Material
                yield: 50.0;
            law crush
                when pressure_ambient > sucrose_cube.yield
                => sucrose_cube;
            fn main() {
                return crush;
            }
        "#;
        let result = eval_program_src(src).unwrap();
        match result {
            Value::Record(r) => {
                assert_eq!(r.get("name"), Some(&Value::String("crush".into())));
                assert_eq!(r.get("has_condition"), Some(&Value::Bool(true)));
                assert_eq!(r.get("has_consequence"), Some(&Value::Bool(true)));
            }
            other => panic!("expected record, got {other:?}"),
        }
    }

    #[test]
    fn t30_law_decl_check_passes() {
        // Law with a condition and consequence should pass the checker.
        let src = r#"
            law simple_law
                when 1 > 0
                => 42;
        "#;
        let program = crate::parse::parse_program(src).unwrap();
        let result = crate::check::check_program(&program);
        assert!(result.is_ok(), "checker should accept law: {:?}", result);
    }

    #[test]
    fn t28_t29_t30_all_three_together() {
        let src = r#"
            field pressure_ambient: Pressure
                unit: "qudt:KiloPascal"
                support: region
                representation: grid;
            material sucrose_cube: Material
                yield: 50.0,
                density: 1580.0;
            law crush
                when 1 > 0
                => 42;
            fn main() {
                let p = pressure_ambient;
                let m = sucrose_cube;
                let l = crush;
                return 0;
            }
        "#;
        let result = eval_program_src(src).unwrap();
        assert_eq!(result, Value::I64(0));
    }

    // â”€â”€ T34-T35: Conservation + causal dispatch tests â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn t34_conservation_check_dispatches_to_host() {
        // LocalHost doesn't override conservation_check, so it returns E702.
        let src = r#"
            import "vibe:0.1/conservation" as conservation;
            fn main() {
                return conservation.check("mass", 100.0, 100.0);
            }
        "#;
        let result = eval_program_src(src);
        assert!(result.is_err(), "expected error, got {:?}", result.ok());
        let err = result.unwrap_err();
        assert_eq!(err.code, DiagCode::E702, "got message: {}", err.message);
    }

    #[test]
    fn t35_causal_relation_dispatches_to_host() {
        // LocalHost doesn't override causal_relation, so it returns E702.
        let src = r#"
            import "vibe:0.1/causal" as causal;
            fn main() {
                return causal.relation(0, 1);
            }
        "#;
        let result = eval_program_src(src);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, DiagCode::E702);
    }

    #[test]
    fn t34_conservation_check_with_tolerance() {
        let src = r#"
            import "vibe:0.1/conservation" as conservation;
            fn main() {
                return conservation.check("energy", 500.0, 499.5, tolerance: 0.001);
            }
        "#;
        let result = eval_program_src(src);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, DiagCode::E702);
    }

    #[test]
    fn t34_conservation_check_unknown_quantity() {
        let src = r#"
            import "vibe:0.1/conservation" as conservation;
            fn main() {
                return conservation.check("unknown_quantity", 1.0, 1.0);
            }
        "#;
        let result = eval_program_src(src);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, DiagCode::E100);
    }

    // â”€â”€ T64: Optional vibe:0.1/ prefix â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn t64_import_without_prefix_works() {
        let src = r#"
            import "math" as math;
            fn main() {
                return math.abs(-5);
            }
        "#;
        let result = eval_program_src(src).unwrap();
        assert_eq!(result, Value::I64(5));
    }

    #[test]
    fn t64_import_with_prefix_still_works() {
        let src = r#"
            import "vibe:0.1/math" as math;
            fn main() {
                return math.abs(-5);
            }
        "#;
        let result = eval_program_src(src).unwrap();
        assert_eq!(result, Value::I64(5));
    }

    #[test]
    fn t64_import_unknown_namespace_fails() {
        let src = r#"
            import "nonexistent" as foo;
            fn main() {
                return 0;
            }
        "#;
        let result = eval_program_src(src);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, DiagCode::E100);
    }

    #[test]
    fn t64_import_without_prefix_no_alias() {
        let src = r#"
            import "math";
            fn main() {
                return math.abs(-3);
            }
        "#;
        let result = eval_program_src(src).unwrap();
        assert_eq!(result, Value::I64(3));
    }

    // â”€â”€ T24: DAG execution dispatch â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn t24_dag_execute_dispatches_to_host() {
        let src = r#"
            import "dag" as dag;
            effect fn main() {
                return dag.execute({});
            }
        "#;
        let result = eval_program_src(src);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, DiagCode::E702);
    }

    #[test]
    fn t24_dag_validate_dispatches_to_host() {
        let src = r#"
            import "dag" as dag;
            fn main() {
                return dag.validate({});
            }
        "#;
        let result = eval_program_src(src);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, DiagCode::E702);
    }

    // â”€â”€ T25: Deontic check dispatch â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn t25_deontic_check_dispatches_to_host() {
        let src = r#"
            import "deontic" as deontic;
            fn main() {
                return deontic.check("graph.query", "execute");
            }
        "#;
        let result = eval_program_src(src);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, DiagCode::E702);
    }

    #[test]
    fn t25_deontic_check_needs_capability_string() {
        let src = r#"
            import "deontic" as deontic;
            fn main() {
                return deontic.check(42, "execute");
            }
        "#;
        let result = eval_program_src(src);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, DiagCode::E100);
    }

    // â”€â”€ T42: HID dispatch â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn t42_hid_poll_dispatches_to_host() {
        let src = r#"
            import "hid" as hid;
            effect fn main() {
                return hid.poll();
            }
        "#;
        let result = eval_program_src(src);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, DiagCode::E702);
    }

    #[test]
    fn t42_hid_wait_dispatches_to_host() {
        let src = r#"
            import "hid" as hid;
            effect fn main() {
                return hid.wait(1000000);
            }
        "#;
        let result = eval_program_src(src);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, DiagCode::E702);
    }

    // â”€â”€ T45: Outbound cues â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn t45_cue_post_dispatches_to_host() {
        let src = r#"
            import "cue" as cue;
            effect fn main() {
                return cue.post("Haptic.buzz", {});
            }
        "#;
        let result = eval_program_src(src);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, DiagCode::E702);
    }

    #[test]
    fn t45_cue_post_needs_string_id() {
        let src = r#"
            import "cue" as cue;
            effect fn main() {
                return cue.post(42, {});
            }
        "#;
        let result = eval_program_src(src);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, DiagCode::E100);
    }

    // â”€â”€ T-crypto: Crypto dispatch â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn crypto_sha256_dispatches_to_host() {
        let src = r#"
            import "crypto" as crypto;
            fn main() {
                return crypto.sha256("hello");
            }
        "#;
        let result = eval_program_src(src);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, DiagCode::E702);
    }

    #[test]
    fn crypto_sha512_dispatches_to_host() {
        let src = r#"
            import "crypto" as crypto;
            fn main() {
                return crypto.sha512("hello");
            }
        "#;
        let result = eval_program_src(src);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, DiagCode::E702);
    }

    #[test]
    fn crypto_blake3_dispatches_to_host() {
        let src = r#"
            import "crypto" as crypto;
            fn main() {
                return crypto.blake3("hello");
            }
        "#;
        let result = eval_program_src(src);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, DiagCode::E702);
    }

    #[test]
    fn crypto_hkdf_dispatches_to_host() {
        let src = r#"
            import "crypto" as crypto;
            fn main() {
                return crypto.hkdf_sha256("ikm", "info", 32);
            }
        "#;
        let result = eval_program_src(src);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, DiagCode::E702);
    }

    #[test]
    fn crypto_aead_encrypt_dispatches_to_host() {
        let src = r#"
            import "crypto" as crypto;
            fn main() {
                return crypto.aead_encrypt("AES-256-GCM", "key", "nonce", "plaintext", "aad");
            }
        "#;
        let result = eval_program_src(src);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, DiagCode::E702);
    }

    #[test]
    fn crypto_aead_decrypt_dispatches_to_host() {
        let src = r#"
            import "crypto" as crypto;
            fn main() {
                return crypto.aead_decrypt("AES-256-GCM", "key", "nonce", "ct", "tag", "aad");
            }
        "#;
        let result = eval_program_src(src);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, DiagCode::E702);
    }

    #[test]
    fn crypto_sign_dispatches_to_host() {
        let src = r#"
            import "crypto" as crypto;
            effect fn main() {
                return crypto.sign("key:ed25519:0", "data");
            }
        "#;
        let result = eval_program_src(src);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, DiagCode::E702);
    }

    #[test]
    fn crypto_verify_dispatches_to_host() {
        let src = r#"
            import "crypto" as crypto;
            fn main() {
                return crypto.verify("key:ed25519:0", "data", "sig");
            }
        "#;
        let result = eval_program_src(src);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, DiagCode::E702);
    }

    #[test]
    fn crypto_generate_key_dispatches_to_host() {
        let src = r#"
            import "crypto" as crypto;
            effect fn main() {
                return crypto.generate_key("Ed25519");
            }
        "#;
        let result = eval_program_src(src);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, DiagCode::E702);
    }

    #[test]
    fn crypto_sha256_needs_string_arg() {
        let src = r#"
            import "crypto" as crypto;
            fn main() {
                return crypto.sha256(42);
            }
        "#;
        let result = eval_program_src(src);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, DiagCode::E100);
    }

    #[test]
    fn crypto_sign_needs_string_args() {
        let src = r#"
            import "crypto" as crypto;
            effect fn main() {
                return crypto.sign(42, "data");
            }
        "#;
        let result = eval_program_src(src);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, DiagCode::E100);
    }

    // â”€â”€ ZK proof dispatch â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn zk_prove_threshold_dispatches_to_host() {
        let src = r#"
            import "zk" as zk;
            fn main() {
                return zk.prove_threshold(21, 18);
            }
        "#;
        let result = eval_program_src(src);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, DiagCode::E702);
    }

    #[test]
    fn zk_verify_threshold_dispatches_to_host() {
        let src = r#"
            import "zk" as zk;
            fn main() {
                return zk.verify_threshold("proof", "vk", 18);
            }
        "#;
        let result = eval_program_src(src);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, DiagCode::E702);
    }

    #[test]
    fn zk_prove_range_dispatches_to_host() {
        let src = r#"
            import "zk" as zk;
            fn main() {
                return zk.prove_range(25, 18, 65);
            }
        "#;
        let result = eval_program_src(src);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, DiagCode::E702);
    }

    #[test]
    fn zk_verify_range_dispatches_to_host() {
        let src = r#"
            import "zk" as zk;
            fn main() {
                return zk.verify_range("proof", "vk", 18, 65);
            }
        "#;
        let result = eval_program_src(src);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, DiagCode::E702);
    }

    #[test]
    fn zk_prove_matmul_dispatches_to_host() {
        let src = r#"
            import "zk" as zk;
            fn main() {
                return zk.prove_matmul(2, 2, 2, [1, 2, 3, 4], [5, 6, 7, 8]);
            }
        "#;
        let result = eval_program_src(src);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, DiagCode::E702);
    }

    #[test]
    fn zk_list_circuits_dispatches_to_host() {
        let src = r#"
            import "zk" as zk;
            fn main() {
                return zk.list_circuits();
            }
        "#;
        let result = eval_program_src(src);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, DiagCode::E702);
    }

    #[test]
    fn zk_prove_threshold_needs_u64_args() {
        let src = r#"
            import "zk" as zk;
            fn main() {
                return zk.prove_threshold("not a number", 18);
            }
        "#;
        let result = eval_program_src(src);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, DiagCode::E100);
    }

    #[test]
    fn test_collection_and_string_methods() {
        let src = r#"
            fn test_all() -> bool {
                let xs = [1.0, 2.0, 3.0];
                let l = xs.len;
                let c = xs.contains(2.0);
                let p = xs.push(4.0);
                let s = "hello world";
                let sw = s.starts_with("hello");
                let sc = s.contains("world");
                let sl = s.len;
                return l == 3.0 && c && p.len == 4.0 && sw && sc && sl == 11.0;
            }
        "#;
        let prog = crate::parse_program(src).expect("parses");
        crate::check_program(&prog).expect("checks");
        let mut host = crate::bind::LocalHost::default();
        let mut env = Env::default();
        let res = crate::eval_function(&prog, "test_all", vec![], &mut host, &mut env)
            .expect("evaluates");
        assert_eq!(res, Value::Bool(true));
    }
