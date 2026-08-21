//! Gazetteer invoke — Aho-Corasick matching with byte spans.

use crate::nlp::gazetteer::Gazetteer;
use poet_vibe::{DiagCode, Diagnostic, Span, Value};
use std::collections::BTreeMap;

/// `NLP.gazetteer_run` — run the default gazetteer over a text, returning hits
/// with exact byte spans and matched IRIs.
pub fn gazetteer_run(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let text = match args {
        Value::String(s) => s.as_str(),
        _ => {
            return Err(Diagnostic::new(
                DiagCode::E100,
                span,
                "NLP.gazetteer_run needs a string document",
            ))
        }
    };
    if text.len() > 256 * 1024 {
        return Err(Diagnostic::new(
            DiagCode::E400,
            span,
            "NLP.gazetteer_run exceeds 256 KiB",
        ));
    }
    let g = Gazetteer::default();
    let hits = g.find(text);
    let list: Vec<Value> = hits
        .iter()
        .map(|h| {
            let mut rec = BTreeMap::new();
            rec.insert("start".into(), Value::U64(h.span.start_utf8 as u64));
            rec.insert("end".into(), Value::U64(h.span.end_utf8 as u64));
            rec.insert("iri".into(), Value::String(h.iri.to_string()));
            rec.insert("surface".into(), Value::String(h.surface.to_string()));
            Value::Record(rec)
        })
        .collect();
    Ok(Value::List(list))
}

/// `NLP.gazetteer_build` — report the default gazetteer's lexicon size and
/// pattern surfaces. The engine uses a compiled default lexicon; this returns
/// metadata about it rather than accepting arbitrary entries (which would
/// require a lexicon-loading host capability).
pub fn gazetteer_build(_args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let g = Gazetteer::default();
    let count = g.pattern_count();
    let mut rec = BTreeMap::new();
    rec.insert("patterns".into(), Value::U64(count as u64));
    rec.insert("status".into(), Value::String("default_lexicon".into()));
    Ok(Value::Record(rec))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gazetteer_run_finds_known_entities() {
        let src = "North Spring is the reference catchment.";
        let result = gazetteer_run(&Value::String(src.into()), Span { start: 0, end: 0 });
        assert!(result.is_ok());
        let val = result.unwrap();
        match val {
            Value::List(hits) => {
                assert!(!hits.is_empty());
                // At least one hit should mention North Spring
                let found = hits.iter().any(|h| match h {
                    Value::Record(rec) => rec.get("surface").map_or(false, |v| match v {
                        Value::String(s) => s.contains("North Spring"),
                        _ => false,
                    }),
                    _ => false,
                });
                assert!(found, "should find North Spring in hits");
            }
            _ => panic!("expected list"),
        }
    }

    #[test]
    fn gazetteer_run_rejects_non_string() {
        let result = gazetteer_run(&Value::I64(42), Span { start: 0, end: 0 });
        assert!(result.is_err());
    }

    #[test]
    fn gazetteer_build_returns_metadata() {
        let result = gazetteer_build(&Value::Null, Span { start: 0, end: 0 });
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Record(rec) => {
                assert!(rec.contains_key("patterns"));
                assert!(rec.contains_key("status"));
            }
            _ => panic!("expected record"),
        }
    }
}

