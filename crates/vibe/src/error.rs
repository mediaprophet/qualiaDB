//! Stable diagnostic codes (vibescript-core.md §9).

use crate::span::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagCode {
    /// Parse / lex
    E001,
    /// Type
    E100,
    /// Effect
    E200,
    /// Capability
    E300,
    /// Budget / unbounded loop
    E400,
    /// Policy
    E500,
    /// Evaluation
    E600,
    /// Deontic phase violation (R2)
    E700,
    /// Assignment to immutable binding (T10)
    E701,
    /// Clock unavailable on this host (T18)
    E702,
    /// Disclosure denied — a credentialed refusal (R10).
    /// The agent/host refused to disclose content because the requester
    /// lacks the required capability, consent, or authority. This is not
    /// a "file not found" error — it is a first-class rights enforcement
    /// value.
    E800,
}

impl DiagCode {
    pub fn as_str(self) -> &'static str {
        match self {
            DiagCode::E001 => "E001",
            DiagCode::E100 => "E100",
            DiagCode::E200 => "E200",
            DiagCode::E300 => "E300",
            DiagCode::E400 => "E400",
            DiagCode::E500 => "E500",
            DiagCode::E600 => "E600",
            DiagCode::E700 => "E700",
            DiagCode::E701 => "E701",
            DiagCode::E702 => "E702",
            DiagCode::E800 => "E800",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub code: DiagCode,
    pub span: Span,
    pub message: String,
    /// Safe rewrite hint. MUST NOT grant new authority (core §9).
    pub suggested_fix: Option<String>,
    /// Evidential (μ, λ) annotation: degrees of positive belief and refutation (G9).
    pub evidential: Option<(f32, f32)>,
}

impl Diagnostic {
    pub fn new(code: DiagCode, span: Span, message: impl Into<String>) -> Self {
        let message = message.into();
        let suggested_fix = infer_fix(code, &message);
        Self {
            code,
            span,
            message,
            suggested_fix,
            evidential: None,
        }
    }

    pub fn with_fix(mut self, fix: impl Into<String>) -> Self {
        self.suggested_fix = Some(fix.into());
        self
    }

    /// Create a disclosure-denied diagnostic (R10).
    pub fn disclosure_denied(span: Span, capability: &str, reason: &str) -> Self {
        Diagnostic::new(
            DiagCode::E800,
            span,
            format!(
                "disclosure denied: capability '{}' — {}",
                capability, reason
            ),
        )
    }

    /// Set the evidential (μ, λ) annotation: degrees of positive belief and refutation.
    pub fn with_evidential(mut self, mu: f32, lambda: f32) -> Self {
        self.evidential = Some((mu, lambda));
        self
    }

    /// Agent-facing JSON (no serde). Always includes `valid: false`.
    pub fn to_json(&self) -> String {
        let mut s = String::from("{\"valid\":false,");
        push_kv(&mut s, "error_code", self.code.as_str());
        s.push_str(&format!(
            "\"span\":[{},{}],",
            self.span.start, self.span.end
        ));
        push_kv(&mut s, "message", &self.message);
        match &self.suggested_fix {
            Some(fix) => push_kv(&mut s, "suggested_fix", fix),
            None => s.push_str("\"suggested_fix\":null,"),
        }
        match self.evidential {
            Some((mu, lambda)) => s.push_str(&format!("\"evidential\":[{},{}],", mu, lambda)),
            None => s.push_str("\"evidential\":null,"),
        }
        s.push_str("\"shacl_violations\":[]}");
        s
    }
}

fn infer_fix(code: DiagCode, message: &str) -> Option<String> {
    let m = message.to_ascii_lowercase();
    if m.contains("<<[") || m.contains("raw quin") {
        return Some(
            "use quin.statement(subject, predicate, object, context) — <<[ overlay is illegal"
                .into(),
        );
    }
    if m.contains("without space") || (m.contains("relational") && m.contains('<')) {
        return Some("put spaces around < > <= >=".into());
    }
    if m.contains("unclosed") && m.contains("triple") {
        return Some("close the term with )>> or >>".into());
    }
    if m.contains("unclosed block comment") {
        return Some("close the comment with */".into());
    }
    if code == DiagCode::E200 && m.contains("pure cell") {
        return Some("move the External call into an effect fn; cells stay Pure".into());
    }
    if code == DiagCode::E300 {
        if m.contains("graph.write") {
            return Some("add requires [ capability(\"graph.write\") ];".into());
        }
        return Some("add `using Family;` or requires [ capability(\"id\") ];".into());
    }
    if m.contains("take") && (m.contains("query") || m.contains("unbounded")) {
        return Some("add take: N to graph.query".into());
    }
    if code == DiagCode::E400 {
        return Some("add budget(steps: N) or a loop bound".into());
    }
    if code == DiagCode::E800 {
        return Some("request the required capability or consent from the principal".into());
    }
    None
}

fn push_kv(out: &mut String, key: &str, value: &str) {
    out.push('"');
    out.push_str(key);
    out.push_str("\":\"");
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push_str("\",");
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}@{}..{}: {}",
            self.code.as_str(),
            self.span.start,
            self.span.end,
            self.message
        )
    }
}

impl std::error::Error for Diagnostic {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disclosure_denied_creates_e800() {
        let diag = Diagnostic::disclosure_denied(
            Span::point(12),
            "graph.read.classified",
            "principal consent token missing",
        );
        assert_eq!(diag.code, DiagCode::E800);
        assert_eq!(diag.code.as_str(), "E800");
        assert!(diag.message.contains("graph.read.classified"));
        assert!(diag.message.contains("principal consent token missing"));
    }

    #[test]
    fn disclosure_denied_has_suggested_fix() {
        let diag = Diagnostic::disclosure_denied(
            Span::point(0),
            "sensor.biometric",
            "unauthorized requester",
        );
        assert!(diag.suggested_fix.is_some());
        assert!(diag
            .suggested_fix
            .as_deref()
            .unwrap()
            .contains("request the required capability or consent"));
    }
}
