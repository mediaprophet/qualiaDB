use super::*;
    use crate::parse_program;

    #[test]
    fn check_program_all_collects_function_and_cell_errors() {
        let src = r#"
            fn a() { return HID.poll(); }
            fn b() { return HID.poll(); }
            cell c := HID.poll();
        "#;
        let program = parse_program(src).expect("parse");
        let (_, errors) = check_program_all(&program);
        assert!(
            errors.len() >= 3,
            "expected function + cell diagnostics, got {}",
            errors.len()
        );
        assert!(errors.len() <= MAX_DIAGNOSTICS);
    }

    #[test]
    fn check_program_all_caps_at_eight() {
        let mut src = String::new();
        for i in 0..12 {
            src.push_str(&format!("fn f{i}() {{ return HID.poll(); }}\n"));
        }
        let program = parse_program(&src).expect("parse");
        let (_, errors) = check_program_all(&program);
        assert_eq!(errors.len(), MAX_DIAGNOSTICS);
    }

    #[test]
    fn vocab_unknown_term_is_check_error() {
        let chunk = crate::vocab::parse_chunk(include_bytes!("../../fixtures/vocab/clinic.n3"))
            .expect("chunk");
        let program = parse_program(
            r#"
            prefix snomed: <http://snomed.info/id/>;
            pure fn f() { return snomed:not_in_chunk; }
            "#,
        )
        .expect("parse");
        let err = check_program_with_vocab(&program, &[chunk]).expect_err("unknown");
        assert_eq!(err.code, DiagCode::E100);
        assert!(err.message.contains("snomed:not_in_chunk"));
    }

    #[test]
    fn vocab_known_clinic_term_checks() {
        let chunk = crate::vocab::parse_chunk(include_bytes!("../../fixtures/vocab/clinic.n3"))
            .expect("chunk");
        let program = parse_program(
            r#"
            prefix snomed: <http://snomed.info/id/>;
            pure fn f() { return snomed:386661006; }
            "#,
        )
        .expect("parse");
        check_program_with_vocab(&program, &[chunk]).expect("known term");
    }

    #[test]
    fn return_type_mismatch_is_rejected() {
        let program = parse_program(
            r#"
            pure fn f() -> i64 { return "not_an_int"; }
            "#,
        )
        .expect("parse");
        let err = check_program(&program).expect_err("return mismatch");
        assert_eq!(err.code, DiagCode::E100);
    }

    #[test]
    fn let_annotation_mismatch_is_rejected() {
        let program = parse_program(
            r#"
            pure fn f() -> i64 {
                let x: i64 = true;
                return 0;
            }
            "#,
        )
        .expect("parse");
        let err = check_program(&program).expect_err("let mismatch");
        assert_eq!(err.code, DiagCode::E100);
    }

    #[test]
    fn non_bool_if_condition_is_rejected() {
        let program = parse_program(
            r#"
            pure fn f() -> i64 {
                if 42 { return 1; }
                return 0;
            }
            "#,
        )
        .expect("parse");
        let err = check_program(&program).expect_err("non-bool if");
        assert_eq!(err.code, DiagCode::E100);
    }

    #[test]
    fn call_arity_and_param_type_mismatch_are_rejected() {
        let arity_prog = parse_program(
            r#"
            pure fn add1(x: i64) -> i64 { return x + 1; }
            pure fn main() -> i64 { return add1(1, 2); }
            "#,
        )
        .expect("parse");
        assert_eq!(
            check_program(&arity_prog).expect_err("arity").code,
            DiagCode::E100
        );

        let type_prog = parse_program(
            r#"
            pure fn add1(x: i64) -> i64 { return x + 1; }
            pure fn main() -> i64 { return add1(true); }
            "#,
        )
        .expect("parse");
        assert_eq!(
            check_program(&type_prog).expect_err("param type").code,
            DiagCode::E100
        );
    }

    #[test]
    fn match_arm_bindings_do_not_leak_to_outer_scope() {
        let program = parse_program(
            r#"
            pure fn f(v: i64) -> i64 {
                let x: i64 = 10;
                match v {
                    1 => { let x: bool = true; }
                    _ => {}
                }
                return x + 1;
            }
            "#,
        )
        .expect("parse");
        check_program(&program).expect("outer x: i64 should remain i64");
    }
