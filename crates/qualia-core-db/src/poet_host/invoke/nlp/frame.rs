//! `NLP.frame_extract` — frame semantics extraction.

use crate::nlp::frame::extract_frames;
use std::collections::BTreeMap;
use vibe::{DiagCode, Diagnostic, Span, Value};

/// `NLP.frame_extract` — extract frame instances from a text. Returns a list
/// of `{ frame_type, elements: [{ role, text, start, end }] }`.
pub fn frame_extract(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let text = match args {
        Value::String(s) => s.as_str(),
        _ => {
            return Err(Diagnostic::new(
                DiagCode::E100,
                span,
                "NLP.frame_extract needs a string document",
            ))
        }
    };
    if text.len() > 256 * 1024 {
        return Err(Diagnostic::new(
            DiagCode::E400,
            span,
            "NLP.frame_extract exceeds 256 KiB",
        ));
    }
    let frames = extract_frames(text);
    let list: Vec<Value> = frames
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
    Ok(Value::List(list))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_buy_frame() {
        let r = frame_extract(
            &Value::String("John bought a book from Mary".into()),
            Span { start: 0, end: 0 },
        );
        assert!(r.is_ok());
        match r.unwrap() {
            Value::List(frames) => {
                assert_eq!(frames.len(), 1);
            }
            _ => panic!("expected list"),
        }
    }

    #[test]
    fn rejects_non_string() {
        let r = frame_extract(&Value::I64(1), Span { start: 0, end: 0 });
        assert!(r.is_err());
    }
}
