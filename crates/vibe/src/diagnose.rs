//! Agent diagnostic loop — parse + check, JSON out. No disk. No execute.

use crate::check::{check_cell, check_program};
use crate::error::Diagnostic;
use crate::parse::{parse_cell, parse_program};

/// Result of [`diagnose`].
#[derive(Debug, Clone, PartialEq)]
pub struct DiagnoseReport {
    pub valid: bool,
    pub kind: &'static str,
    pub error: Option<Diagnostic>,
    /// All diagnostics collected for this source (parse/check). The first
    /// entry is the same as [`Self::error`] when present.
    pub errors: Vec<Diagnostic>,
}

impl DiagnoseReport {
    pub fn to_json(&self) -> String {
        if let Some(err) = &self.error {
            format!(
                "{{\"valid\":false,\"kind\":\"{}\",{},\"errors\":{}}}",
                self.kind,
                err.json_body(),
                errors_json(&self.errors)
            )
        } else {
            format!(
                "{{\"valid\":true,\"kind\":\"{}\",\"error_code\":null,\"span\":null,\"message\":null,\"suggested_fix\":null,\"evidential\":null,\"shacl_violations\":[],\"errors\":[]}}",
                self.kind
            )
        }
    }
}

fn errors_json(errors: &[Diagnostic]) -> String {
    let mut s = String::from("[");
    for (i, error) in errors.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push('{');
        s.push_str(&error.json_body());
        s.push('}');
    }
    s.push(']');
    s
}

/// Helper to detect if a diagnostic represents a contradiction or semantic conflict
/// (e.g. conflicting types, conflicting graph assertions, forbidden effect in pure cell).
pub fn is_contradiction(diag: &Diagnostic) -> bool {
    match diag.code {
        crate::error::DiagCode::E100
        | crate::error::DiagCode::E200
        | crate::error::DiagCode::E700 => true,
        _ => {
            let m = diag.message.to_ascii_lowercase();
            m.contains("conflict") || m.contains("contradiction") || m.contains("mismatch")
        }
    }
}

/// Parse and type/effect-check. Cells start with `=`. Does not evaluate.
/// When a contradiction/conflict is detected, attaches evidential annotations (μ = 1.0, λ = 0.0).
pub fn diagnose(src: &str) -> DiagnoseReport {
    let trimmed = src.trim_start_matches('\u{feff}').trim_start();
    let mut report = if trimmed.starts_with('=') {
        match parse_cell(src).and_then(|e| check_cell(&e).map(|_| e)) {
            Ok(_) => DiagnoseReport {
                valid: true,
                kind: "cell",
                error: None,
                errors: Vec::new(),
            },
            Err(error) => DiagnoseReport {
                valid: false,
                kind: "cell",
                errors: vec![error.clone()],
                error: Some(error),
            },
        }
    } else {
        match parse_program(src).and_then(|p| check_program(&p).map(|_| p)) {
            Ok(_) => DiagnoseReport {
                valid: true,
                kind: "module",
                error: None,
                errors: Vec::new(),
            },
            Err(error) => {
                let extra = match parse_program(src) {
                    Ok(p) => {
                        let (_, mut all) = crate::check::check_program_all(&p);
                        if all.is_empty() {
                            all.push(error.clone());
                        }
                        all
                    }
                    Err(_) => collect_module_errors(src, &error),
                };
                DiagnoseReport {
                    valid: false,
                    kind: "module",
                    errors: extra,
                    error: Some(error),
                }
            }
        }
    };

    if let Some(err) = report.error.as_mut() {
        if is_contradiction(err) && err.evidential.is_none() {
            err.evidential = Some((1.0, 0.0));
        }
    }

    report
}

fn collect_module_errors(src: &str, first: &Diagnostic) -> Vec<Diagnostic> {
    let Ok(program) = parse_program(src) else {
        return vec![first.clone()];
    };
    let (_, mut all) = crate::check::check_program_all(&program);
    if all.is_empty() {
        all.push(first.clone());
    }
    all
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DiagCode;

    #[test]
    fn overlay_has_safe_fix() {
        let r = diagnose("fn bad() { return <<[ s p o g prov ]>>; }");
        assert!(!r.valid);
        assert!(r.to_json().contains("suggested_fix"));
        let err = r.error.as_ref().unwrap();
        assert_eq!(err.code, DiagCode::E001);
        assert!(err
            .suggested_fix
            .as_deref()
            .unwrap()
            .contains("quin.statement"));
    }

    #[test]
    fn good_cell_is_valid() {
        let r = diagnose("= math.max(0, 1)");
        assert!(r.valid);
        assert_eq!(r.kind, "cell");
        assert!(r.to_json().contains("\"valid\":true"));
    }

    #[test]
    fn contradiction_diagnostic_carries_evidential() {
        // E200 effect contradiction: Pure cell performing External effect
        let r = diagnose("= pulse.publish(\"t\", 1)");
        assert!(!r.valid);
        let err = r.error.as_ref().unwrap();
        assert!(
            err.evidential.is_some(),
            "Contradiction diagnostic must carry evidential annotation"
        );
        let (mu, lambda) = err.evidential.unwrap();
        assert_eq!(mu, 1.0);
        assert_eq!(lambda, 0.0);
    }

    #[test]
    fn syntax_error_does_not_carry_evidential() {
        // E001 parse error: unclosed parenthesis
        let r = diagnose("= math.max(1, ");
        assert!(!r.valid);
        let err = r.error.as_ref().unwrap();
        assert_eq!(err.code, DiagCode::E001);
        assert!(
            err.evidential.is_none(),
            "Syntax error must not carry evidential annotation"
        );
    }

    #[test]
    fn diagnostic_with_evidential_builder() {
        let diag = Diagnostic::new(DiagCode::E100, crate::Span::point(0), "type conflict")
            .with_evidential(0.85, 0.15);
        assert_eq!(diag.evidential, Some((0.85, 0.15)));
        assert!(diag.to_json().contains("\"evidential\":[0.85,0.15]"));
    }

    #[test]
    fn diagnose_collects_multiple_module_errors() {
        let r = diagnose(
            r#"
            fn a() { return HID.poll(); }
            fn b() { return HID.poll(); }
            fn c() { return HID.poll(); }
            "#,
        );
        assert!(!r.valid);
        assert!(
            r.errors.len() >= 3,
            "diagnose must collect per-item errors, got {}",
            r.errors.len()
        );
        let json = r.to_json();
        assert!(
            json.contains("\"errors\":["),
            "failure JSON must list errors[]"
        );
        assert!(json.contains("\"kind\":\"module\""));
        assert!(json.contains("\"valid\":false"));
        assert!(
            json.matches("\"error_code\":").count() >= 4,
            "primary diagnostic plus errors[] entries, got {json}"
        );
    }

    #[test]
    fn lexicon_missing_pack_helper_matches_held_json_shape() {
        let src = include_str!("../fixtures/lexicon/missing_pack.vibe");
        let expected = include_str!("../fixtures/lexicon/missing_pack.diagnose.json");
        let r = crate::diagnose_lexicon_pin(src, false);
        let json = r.to_json();
        assert!(!r.valid);
        assert_eq!(r.kind, "module");
        let err = r.error.as_ref().unwrap();
        assert_eq!(err.code, DiagCode::E300);
        assert_eq!(err.suggested_fix.as_deref(), Some(crate::HELD_OPEN_PACK));
        assert!(json.contains("\"error_code\":\"E300\""));
        assert!(json.contains("held / not yet"));
        assert!(json.contains("open lexicon pack"));
        assert!(json.contains("\"errors\":["));
        assert!(!json.to_ascii_lowercase().contains("broken"));
        for needle in [
            "\"valid\": false",
            "\"kind\": \"module\"",
            "\"error_code\": \"E300\"",
            "held / not yet — open lexicon pack",
        ] {
            assert!(
                expected.contains(needle),
                "fixture DiagnoseReport JSON must document {needle}"
            );
        }
        // parse+check of the workshop file stays valid (no disk / no invoke)
        let parse_r = diagnose(src);
        assert!(
            parse_r.valid,
            "diagnose() must not invoke lexicon_manifest: {}",
            parse_r.to_json()
        );
    }

    #[test]
    fn lexicon_pin_ok_records_en_core() {
        let src = include_str!("../fixtures/lexicon/pin_ok.vibe");
        let pin = crate::parse_lexicon_pin_from_source(src).expect("pin");
        assert_eq!(pin.as_pin_str(), crate::EXAMPLE_PIN);
        assert_eq!(pin.pack_id, "en-core");
        assert_eq!(pin.pack_semver, "0.1.0");
        let hook = crate::diagnose_lexicon_pin(src, true);
        assert!(hook.valid);
        let r = diagnose(src);
        assert!(r.valid, "pin_ok diagnose: {}", r.to_json());
        assert_eq!(crate::LANGUAGE_VERSION, "vibe-0.1");
        assert_eq!(crate::HOST_VERSION, "vibe-host-0.1");
    }

    #[test]
    fn lexicon_alias_row_round_trip_in_suggested_fix() {
        let src = include_str!("../fixtures/lexicon/alias_rows.json");
        let rows = crate::parse_alias_rows_json(src);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].from, "arrive");
        assert_eq!(rows[0].to, "concept:arrive");
        assert_eq!(rows[0].framing, crate::LexiconFraming::LivingShacl);
        assert_eq!(rows[1].framing, crate::LexiconFraming::Mixed);
        assert_eq!(rows[2].framing, crate::LexiconFraming::ArtifactOwl);
        let report = crate::alias_migrate_report(crate::Span::point(0), &rows);
        let fix = report
            .error
            .as_ref()
            .unwrap()
            .suggested_fix
            .as_deref()
            .unwrap();
        let back = crate::parse_alias_rows_json(fix);
        assert_eq!(back, rows);
        assert!(fix.contains("\"from\":\"arrive\""));
        assert!(fix.contains("\"to\":\"concept:arrive\""));
        assert!(fix.contains("living-SHACL"));
        let json = report.to_json();
        assert!(json.contains("suggested_fix"));
        assert!(json.contains("living-SHACL"));
    }

    #[test]
    fn lexicon_living_upgrade_never_rewritten_as_artifact() {
        let src = include_str!("../fixtures/lexicon/upgrade_living.json");
        let before = crate::parse_alias_rows_json(src);
        let living: Vec<_> = before
            .iter()
            .filter(|r| r.framing == crate::LexiconFraming::LivingShacl)
            .cloned()
            .collect();
        assert!(!living.is_empty());
        let requested = vec![crate::LexiconFraming::ArtifactOwl; before.len()];
        let after = crate::apply_upgrade_map(&before, &requested);
        assert!(!crate::living_rewritten_as_artifact(&before, &after));
        for row in &after {
            if living.iter().any(|l| l.from == row.from) {
                assert_eq!(
                    row.framing,
                    crate::LexiconFraming::LivingShacl,
                    "{} must stay living-SHACL",
                    row.from
                );
            }
        }
    }
}
