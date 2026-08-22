//! `NLP.relation_extract` — RDF-Star triple extraction.

use crate::nlp::relation::extract_relations;
use std::collections::BTreeMap;
use vibe::{DiagCode, Diagnostic, Span, Value};

/// `NLP.relation_extract` — extract relations from a text. Returns a list of
/// `{ subject, predicate, object, subject_start, subject_end, object_start,
/// object_end, confidence }`.
pub fn relation_extract(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let text = match args {
        Value::String(s) => s.as_str(),
        _ => {
            return Err(Diagnostic::new(
                DiagCode::E100,
                span,
                "NLP.relation_extract needs a string document",
            ))
        }
    };
    if text.len() > 256 * 1024 {
        return Err(Diagnostic::new(
            DiagCode::E400,
            span,
            "NLP.relation_extract exceeds 256 KiB",
        ));
    }
    let rels = extract_relations(text);
    let list: Vec<Value> = rels
        .iter()
        .map(|r| {
            let mut rec = BTreeMap::new();
            rec.insert("subject".into(), Value::String(r.subject.clone()));
            rec.insert("predicate".into(), Value::String(r.predicate.clone()));
            rec.insert("object".into(), Value::String(r.object.clone()));
            rec.insert(
                "subject_start".into(),
                Value::U64(r.subject_span.start_utf8 as u64),
            );
            rec.insert(
                "subject_end".into(),
                Value::U64(r.subject_span.end_utf8 as u64),
            );
            rec.insert(
                "object_start".into(),
                Value::U64(r.object_span.start_utf8 as u64),
            );
            rec.insert(
                "object_end".into(),
                Value::U64(r.object_span.end_utf8 as u64),
            );
            rec.insert("confidence".into(), Value::F64(r.confidence));
            Value::Record(rec)
        })
        .collect();
    Ok(Value::List(list))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_is_a() {
        let r = relation_extract(
            &Value::String("Socrates is a philosopher".into()),
            Span { start: 0, end: 0 },
        );
        assert!(r.is_ok());
        match r.unwrap() {
            Value::List(rels) => assert_eq!(rels.len(), 1),
            _ => panic!("expected list"),
        }
    }

    #[test]
    fn rejects_non_string() {
        let r = relation_extract(&Value::I64(1), Span { start: 0, end: 0 });
        assert!(r.is_err());
    }
}
