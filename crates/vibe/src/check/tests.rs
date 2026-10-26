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
