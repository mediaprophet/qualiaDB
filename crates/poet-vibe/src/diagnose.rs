//! Agent diagnostic loop — parse + check, JSON out. No disk. No execute.

use crate::check::{check_cell, check_program};
use crate::error::Diagnostic;
use crate::parse::{parse_cell, parse_program};

/// Result of [`diagnose`].
#[derive(Debug, Clone, PartialEq, Eq)]
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
                "{{\"valid\":true,\"kind\":\"{}\",\"error_code\":null,\"span\":null,\"message\":null,\"suggested_fix\":null,\"shacl_violations\":[]}}",
                self.kind
            )
        }
    }
}

/// Parse and type/effect-check. Cells start with `=`. Does not evaluate.
pub fn diagnose(src: &str) -> DiagnoseReport {
    let trimmed = src.trim_start_matches('\u{feff}').trim_start();
    if trimmed.starts_with('=') {
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
    }
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
}
