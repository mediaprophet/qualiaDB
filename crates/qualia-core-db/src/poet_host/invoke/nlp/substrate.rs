//! `NLP.substrate_extract` — full symbolic pipeline.

use crate::nlp::substrate::extract_substrate;
use poet_vibe::{DiagCode, Diagnostic, Span, Value};
use std::collections::BTreeMap;

/// `NLP.substrate_extract` — run the full pipeline (tokenize → gazetteer →
/// normalize → relations → frames → coref) and return a summary record with
/// counts and the extracted frames/relations/coref chains.
pub fn substrate_extract(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let text = match args {
        Value::String(s) => s.as_str(),
        _ => {
            return Err(Diagnostic::new(
                DiagCode::E100,
                span,
                "NLP.substrate_extract needs a string document",
            ))
        }
    };
    if text.len() > 256 * 1024 {
        return Err(Diagnostic::new(
            DiagCode::E400,
            span,
            "NLP.substrate_extract exceeds 256 KiB",
        ));
    }
    let sub = extract_substrate(text);

    let frames: Vec<Value> = sub
        .frames
        .iter()
        .map(|f| {
            let els: Vec<Value> = f
                .elements
                .iter()
                .map(|e| {
                    let mut r = BTreeMap::new();
                    r.insert("role".into(), Value::String(e.role.clone()));
                    r.insert("text".into(), Value::String(e.text.clone()));
                    r.insert("start".into(), Value::U64(e.span.start_utf8 as u64));
                    r.insert("end".into(), Value::U64(e.span.end_utf8 as u64));
                    Value::Record(r)
                })
                .collect();
            let mut rec = BTreeMap::new();
            rec.insert("frame_type".into(), Value::String(f.frame_type.clone()));
            rec.insert("elements".into(), Value::List(els));
            Value::Record(rec)
        })
        .collect();

    let relations: Vec<Value> = sub
        .relations
        .iter()
        .map(|r| {
            let mut rec = BTreeMap::new();
            rec.insert("subject".into(), Value::String(r.subject.clone()));
            rec.insert("predicate".into(), Value::String(r.predicate.clone()));
            rec.insert("object".into(), Value::String(r.object.clone()));
            rec.insert("confidence".into(), Value::F64(r.confidence));
            Value::Record(rec)
        })
        .collect();

    let coref_chains: Vec<Value> = sub
        .coref_chains
        .iter()
        .map(|c| {
            let ms: Vec<Value> = c
                .mentions
                .iter()
                .map(|m| {
                    let mut r = BTreeMap::new();
                    r.insert("start".into(), Value::U64(m.span.start_utf8 as u64));
                    r.insert("end".into(), Value::U64(m.span.end_utf8 as u64));
                    r.insert("text".into(), Value::String(m.text.clone()));
                    Value::Record(r)
                })
                .collect();
            let mut rec = BTreeMap::new();
            rec.insert("id".into(), Value::U64(c.id as u64));
            rec.insert("mentions".into(), Value::List(ms));
            Value::Record(rec)
        })
        .collect();

    let mut rec = BTreeMap::new();
    rec.insert("tokens".into(), Value::U64(sub.tokens.len() as u64));
    rec.insert("hits".into(), Value::U64(sub.hits.len() as u64));
    rec.insert("norms".into(), Value::U64(sub.norms.len() as u64));
    rec.insert("frames".into(), Value::List(frames));
    rec.insert("relations".into(), Value::List(relations));
    rec.insert("coref_chains".into(), Value::List(coref_chains));
    Ok(Value::Record(rec))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_substrate() {
        let r = substrate_extract(
            &Value::String("John bought a book from Mary. She gave it to John.".into()),
            Span { start: 0, end: 0 },
        );
        assert!(r.is_ok());
        match r.unwrap() {
            Value::Record(rec) => {
                assert!(rec.contains_key("tokens"));
                assert!(rec.contains_key("frames"));
                assert!(rec.contains_key("relations"));
                assert!(rec.contains_key("coref_chains"));
            }
            _ => panic!("expected record"),
        }
    }

    #[test]
    fn rejects_non_string() {
        let r = substrate_extract(&Value::I64(1), Span { start: 0, end: 0 });
        assert!(r.is_err());
    }
}
