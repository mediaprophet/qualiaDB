//! Document NLP via invoke — not a Vibe keyword.

use crate::nlp::analyze_document;
use crate::nlp::budget::reject_source;
use std::collections::BTreeMap;
use vibe::{DiagCode, Diagnostic, Span, Value};

pub fn analyze(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let text = match args {
        Value::String(s) => s.as_str(),
        _ => {
            return Err(Diagnostic::new(
                DiagCode::E100,
                span,
                "nlp.analyze needs a string document",
            ))
        }
    };
    reject_source(text.len())
        .map_err(|_| Diagnostic::new(DiagCode::E400, span, "nlp.analyze exceeds 256 KiB"))?;
    let analysis = analyze_document(text);
    let mut rec = BTreeMap::new();
    rec.insert("tokens".into(), Value::U64(analysis.token_count as u64));
    rec.insert(
        "sentences".into(),
        Value::U64(analysis.sentence_count as u64),
    );
    rec.insert("plans".into(), Value::U64(analysis.plans.len() as u64));
    rec.insert(
        "source_hash".into(),
        Value::String(format!("{:#x}", analysis.source_hash)),
    );
    Ok(Value::Record(rec))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nlp::budget::MAX_SOURCE_BYTES;

    #[test]
    fn analyze_happy_path() {
        let r = analyze(
            &Value::String("North Spring is the reference site.".into()),
            Span { start: 0, end: 0 },
        )
        .expect("analyze");
        match r {
            Value::Record(rec) => {
                assert!(rec.contains_key("tokens"));
                assert!(rec.contains_key("source_hash"));
            }
            _ => panic!("expected record"),
        }
    }

    #[test]
    fn analyze_rejects_oversize_source() {
        let src = "x".repeat(MAX_SOURCE_BYTES + 1);
        let err = analyze(&Value::String(src), Span { start: 0, end: 0 }).unwrap_err();
        assert_eq!(err.code, DiagCode::E400);
        assert!(err.message.contains("256 KiB"));
    }
}
