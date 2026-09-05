//! Sprint B Stage 1: graph/volume fixtures parse+check; diagnose loops carry fixes.

use vibe::{check_program, diagnose, parse_program, DiagCode};

#[test]
fn graph_and_volume_fixtures_parse_and_check() {
    for (name, src) in [
        (
            "graph_sparql",
            include_str!("../fixtures/graph_sparql.vibe"),
        ),
        (
            "volume_sanctuary",
            include_str!("../fixtures/volume_sanctuary.vibe"),
        ),
        (
            "inference_grounding",
            include_str!("../fixtures/inference_grounding.vibe"),
        ),
        (
            "render_preview_handles",
            include_str!("../fixtures/render_preview_handles.vibe"),
        ),
        (
            "g_coord_realms",
            include_str!("../fixtures/g_coord_realms.vibe"),
        ),
    ] {
        let code: String = src
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect();
        assert!(
            !code.contains("capability.invoke"),
            "{name}: workshop dialect must not teach capability.invoke"
        );
        assert!(
            !code.contains("qualia."),
            "{name}: must not invent dotted qualia.* ids"
        );
        let prog = parse_program(src).unwrap_or_else(|e| panic!("{name} parse: {e}"));
        check_program(&prog).unwrap_or_else(|e| panic!("{name} check: {e}"));
    }
}

#[test]
fn diagnose_loop_fixtures_are_invalid_with_suggested_fix() {
    let cases = [
        (
            "n1_nospace_lt",
            include_str!("../fixtures/n1_nospace_lt.vibe"),
            DiagCode::E001,
        ),
        (
            "n3_quin_overlay",
            include_str!("../fixtures/n3_quin_overlay.vibe"),
            DiagCode::E001,
        ),
        (
            "n7_time_in_pure_cell",
            include_str!("../fixtures/n7_time_in_pure_cell.vibe"),
            DiagCode::E200,
        ),
    ];
    for (name, src, code) in cases {
        let report = diagnose(src);
        assert!(!report.valid, "{name} should fail diagnose");
        let err = report.error.as_ref().expect(name);
        assert_eq!(err.code, code, "{name} code");
        assert!(
            err.suggested_fix.is_some(),
            "{name} must carry suggested_fix"
        );
        let json = report.to_json();
        assert!(json.contains("\"errors\":["), "{name} JSON errors[]");
        assert!(json.contains("suggested_fix"), "{name} JSON suggested_fix");
    }
}
