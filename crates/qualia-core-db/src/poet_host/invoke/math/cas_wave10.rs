//! Wave-10 Host binds: remaining CAS surface (`parse`).
//!
//! Host-missing `pub fn` from `specialized_libs::symbolic_algebra` not bound in
//! waves 1–9 (wave 8/9 covered expression constructors).

use super::super::args;
use crate::specialized_libs::symbolic_algebra as sa;
use vibe::{Diagnostic, Span, Value};

/// `SymbolicAlgebra.parse` — parse an expression string into canonical form.
/// Args: `{ expr }`. Out: `{ expr }`.
pub fn parse(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let expr = args::rec_str(args_v, "expr")
        .ok_or_else(|| args::bad(span, "SymbolicAlgebra.parse needs expr: string"))?;
    let e = sa::parse(expr).map_err(|e| args::bad(span, format!("parse error: {e}")))?;
    Ok(args::record([("expr", Value::String(e.to_string()))]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn span() -> Span {
        Span { start: 0, end: 0 }
    }

    #[test]
    fn wave10_parse_sum() {
        let mut m = BTreeMap::new();
        m.insert("expr".into(), Value::String("1 + 2".into()));
        let out = parse(&Value::Record(m), span()).unwrap();
        let s = args::rec_str(&out, "expr").unwrap();
        assert!(s.contains('1') && s.contains('2'), "got {s}");
    }

    #[test]
    fn wave10_parse_rejects_empty() {
        let mut m = BTreeMap::new();
        m.insert("expr".into(), Value::String("".into()));
        assert!(parse(&Value::Record(m), span()).is_err());
    }
}
