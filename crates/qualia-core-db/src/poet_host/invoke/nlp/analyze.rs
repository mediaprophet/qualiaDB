//! Document NLP via invoke — not a Vibe keyword.

use crate::nlp::analyze_document;
use poet_vibe::{DiagCode, Diagnostic, Span, Value};
use std::collections::BTreeMap;

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
    if text.len() > 256 * 1024 {
        return Err(Diagnostic::new(DiagCode::E400, span, "nlp.analyze exceeds 256 KiB"));
    }
    let analysis = analyze_document(text);
    let mut rec = BTreeMap::new();
    rec.insert("tokens".into(), Value::U64(analysis.token_count as u64));
    rec.insert("sentences".into(), Value::U64(analysis.sentence_count as u64));
    rec.insert("plans".into(), Value::U64(analysis.plans.len() as u64));
    rec.insert(
        "source_hash".into(),
        Value::String(format!("{:#x}", analysis.source_hash)),
    );
    Ok(Value::Record(rec))
}
