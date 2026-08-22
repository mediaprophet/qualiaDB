//! `NLP.fst_lookup` — FST morphology lookup over a word.

use crate::nlp::fst::FstDict;
use std::collections::BTreeMap;
use vibe::{DiagCode, Diagnostic, Span, Value};

/// `NLP.fst_lookup` — look up a word in a dictionary built from the
/// `entries` argument (a list of `[surface, "lemma|features"]` pairs).
/// Returns a list of `{ lemma, features, start, end }` records.
pub fn fst_lookup(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let (word, entries) = match args {
        Value::Record(rec) => {
            let word = match rec.get("word") {
                Some(Value::String(s)) => s.as_str(),
                _ => {
                    return Err(Diagnostic::new(
                        DiagCode::E100,
                        span,
                        "NLP.fst_lookup needs a { word: string } record",
                    ))
                }
            };
            let entries = match rec.get("entries") {
                Some(Value::List(list)) => list.clone(),
                _ => Vec::new(),
            };
            (word, entries)
        }
        _ => {
            return Err(Diagnostic::new(
                DiagCode::E100,
                span,
                "NLP.fst_lookup needs a { word: string, entries: list } record",
            ))
        }
    };
    if word.len() > 4 * 1024 {
        return Err(Diagnostic::new(
            DiagCode::E400,
            span,
            "NLP.fst_lookup word exceeds 4 KiB",
        ));
    }
    let mut pairs: Vec<(String, String)> = Vec::with_capacity(entries.len());
    for e in &entries {
        match e {
            Value::List(pair) if pair.len() == 2 => {
                let surface = match &pair[0] {
                    Value::String(s) => s.clone(),
                    _ => {
                        return Err(Diagnostic::new(
                            DiagCode::E100,
                            span,
                            "NLP.fst_lookup entries must be [string, string] pairs",
                        ))
                    }
                };
                let payload = match &pair[1] {
                    Value::String(s) => s.clone(),
                    _ => {
                        return Err(Diagnostic::new(
                            DiagCode::E100,
                            span,
                            "NLP.fst_lookup entries must be [string, string] pairs",
                        ))
                    }
                };
                pairs.push((surface, payload));
            }
            _ => {
                return Err(Diagnostic::new(
                    DiagCode::E100,
                    span,
                    "NLP.fst_lookup entries must be [string, string] pairs",
                ))
            }
        }
    }
    let dict = FstDict::from_entries(&pairs);
    let results = dict.lookup(word);
    let list: Vec<Value> = results
        .iter()
        .map(|r| {
            let mut rec = BTreeMap::new();
            rec.insert("lemma".into(), Value::String(r.lemma.clone()));
            rec.insert("features".into(), Value::String(r.features.clone()));
            rec.insert("start".into(), Value::U64(r.span.start_utf8 as u64));
            rec.insert("end".into(), Value::U64(r.span.end_utf8 as u64));
            Value::Record(rec)
        })
        .collect();
    Ok(Value::List(list))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries_value() -> Value {
        Value::List(vec![
            Value::List(vec![
                Value::String("cat".into()),
                Value::String("cat|N".into()),
            ]),
            Value::List(vec![
                Value::String("walk".into()),
                Value::String("walk|V".into()),
            ]),
        ])
    }

    fn args(word: &str) -> Value {
        let mut rec = BTreeMap::new();
        rec.insert("word".into(), Value::String(word.into()));
        rec.insert("entries".into(), entries_value());
        Value::Record(rec)
    }

    #[test]
    fn lookup_known() {
        let r = fst_lookup(&args("cat"), Span { start: 0, end: 0 });
        assert!(r.is_ok());
        match r.unwrap() {
            Value::List(v) => {
                assert_eq!(v.len(), 1);
            }
            _ => panic!("expected list"),
        }
    }

    #[test]
    fn lookup_suffix_plural() {
        let r = fst_lookup(&args("cats"), Span { start: 0, end: 0 });
        assert!(r.is_ok());
        match r.unwrap() {
            Value::List(v) => assert_eq!(v.len(), 1),
            _ => panic!("expected list"),
        }
    }

    #[test]
    fn rejects_non_record() {
        let r = fst_lookup(&Value::I64(1), Span { start: 0, end: 0 });
        assert!(r.is_err());
    }
}
