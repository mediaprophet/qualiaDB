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
}

impl DiagnoseReport {
    pub fn to_json(&self) -> String {
        if let Some(err) = &self.error {
            let mut json = err.to_json();
            // insert kind after opening brace
            json.insert_str(1, &format!("\"kind\":\"{}\",", self.kind));
            json
        } else {
            format!(
                "{{\"valid\":true,\"kind\":\"{}\",\"error_code\":null,\"span\":null,\"message\":null,\"suggested_fix\":null,\"evidential\":null,\"shacl_violations\":[]}}",
                self.kind
            )
        }
    }
}

/// Helper to detect if a diagnostic represents a contradiction or semantic conflict
/// (e.g. conflicting types, conflicting graph assertions, forbidden effect in pure cell).
pub fn is_contradiction(diag: &Diagnostic) -> bool {
    match diag.code {
        crate::error::DiagCode::E100 | crate::error::DiagCode::E200 | crate::error::DiagCode::E700 => true,
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
            },
            Err(error) => DiagnoseReport {
                valid: false,
                kind: "cell",
                error: Some(error),
            },
        }
    } else {
        match parse_program(src).and_then(|p| check_program(&p).map(|_| p)) {
            Ok(_) => DiagnoseReport {
                valid: true,
                kind: "module",
                error: None,
            },
            Err(error) => DiagnoseReport {
                valid: false,
                kind: "module",
                error: Some(error),
            },
        }
    };

    if let Some(err) = report.error.as_mut() {
        if is_contradiction(err) && err.evidential.is_none() {
            err.evidential = Some((1.0, 0.0));
        }
    }

    report
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
        assert!(err.suggested_fix.as_deref().unwrap().contains("quin.statement"));
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
        assert!(err.evidential.is_some(), "Contradiction diagnostic must carry evidential annotation");
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
        assert!(err.evidential.is_none(), "Syntax error must not carry evidential annotation");
    }

    #[test]
    fn diagnostic_with_evidential_builder() {
        let diag = Diagnostic::new(DiagCode::E100, crate::Span::point(0), "type conflict")
            .with_evidential(0.85, 0.15);
        assert_eq!(diag.evidential, Some((0.85, 0.15)));
        assert!(diag.to_json().contains("\"evidential\":[0.85,0.15]"));
    }
}
