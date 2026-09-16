//! `NLP.coref_resolve` — bounded exact-string grouping plus an experimental
//! pronoun sieve. Not multi-pass sieve quality. Empty `mentions` skip detection.

use crate::nlp::coref::{
    mention_kind_from_label, offset_from_i64, offset_from_u64, resolve_coreferences_counted,
    CancellationToken, CorefError, CorefLimits, CorefMention,
};
use crate::nlp::span::DocSpan;
use std::collections::BTreeMap;
use vibe::{DiagCode, Diagnostic, Span, Value};

/// `NLP.coref_resolve` — resolve coreferences over `text` given a list of
/// mentions. Each mention is `{ start, end, text, kind }` where kind is
/// `"pronoun"`, `"proper"`, or `"common"`. Returns a list of chains, each
/// `{ id, mentions: [...] }`.
///
/// A missing `mentions` field is the documented empty-list mode and does not
/// perform mention detection. A present but wrongly typed `mentions` field
/// fails. Unknown kinds, negative offsets, and unsorted lists fail before
/// grouping.
pub fn coref_resolve(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let rec = match args {
        Value::Record(rec) => rec,
        _ => {
            return Err(Diagnostic::new(
                DiagCode::E100,
                span,
                "NLP.coref_resolve needs a { text: string, mentions: list } record",
            ))
        }
    };
    let text = match rec.get("text") {
        Some(Value::String(s)) => s.as_str(),
        _ => {
            return Err(Diagnostic::new(
                DiagCode::E100,
                span,
                "NLP.coref_resolve needs a { text: string } record",
            ))
        }
    };
    if text.len() > CorefLimits::DEFAULT.max_source_bytes {
        return Err(coref_diag(
            CorefError::SourceTooLarge {
                bytes: text.len(),
                max: CorefLimits::DEFAULT.max_source_bytes,
            },
            span,
        ));
    }
    let mention_values: &[Value] = match rec.get("mentions") {
        Some(Value::List(list)) => list.as_slice(),
        Some(_) => {
            return Err(Diagnostic::new(
                DiagCode::E100,
                span,
                "NLP.coref_resolve mentions must be a list",
            ))
        }
        None => &[],
    };
    if mention_values.len() > CorefLimits::DEFAULT.max_mentions {
        return Err(coref_diag(
            CorefError::TooManyMentions {
                count: mention_values.len(),
                max: CorefLimits::DEFAULT.max_mentions,
            },
            span,
        ));
    }
    let mut text_bytes = 0usize;
    for item in mention_values {
        let rec = match item {
            Value::Record(r) => r,
            _ => {
                return Err(Diagnostic::new(
                    DiagCode::E100,
                    span,
                    "NLP.coref_resolve mentions must be records",
                ))
            }
        };
        match rec.get("text") {
            Some(Value::String(s)) => {
                text_bytes = text_bytes.saturating_add(s.len());
                if text_bytes > CorefLimits::DEFAULT.max_mention_text_bytes {
                    return Err(coref_diag(
                        CorefError::MentionTextTooLarge {
                            bytes: text_bytes,
                            max: CorefLimits::DEFAULT.max_mention_text_bytes,
                        },
                        span,
                    ));
                }
            }
            _ => {
                return Err(Diagnostic::new(
                    DiagCode::E100,
                    span,
                    "mention needs a text string",
                ))
            }
        }
    }
    let mut parsed: Vec<CorefMention> = Vec::with_capacity(mention_values.len());
    for item in mention_values {
        let rec = match item {
            Value::Record(r) => r,
            _ => {
                return Err(Diagnostic::new(
                    DiagCode::E100,
                    span,
                    "NLP.coref_resolve mentions must be records",
                ))
            }
        };
        let start = required_offset(rec, "start", span)?;
        let end = required_offset(rec, "end", span)?;
        let mtext = match rec.get("text") {
            Some(Value::String(s)) => s.clone(),
            Some(_) | None => {
                return Err(Diagnostic::new(
                    DiagCode::E100,
                    span,
                    "mention needs a text string",
                ))
            }
        };
        let kind = match rec.get("kind") {
            Some(Value::String(s)) => {
                mention_kind_from_label(s).map_err(|e| coref_diag(e, span))?
            }
            Some(_) | None => {
                return Err(Diagnostic::new(
                    DiagCode::E100,
                    span,
                    "mention needs a kind string",
                ))
            }
        };
        parsed.push(CorefMention {
            span: DocSpan::new(start, end),
            text: mtext,
            kind,
        });
    }
    let (chains, _) = resolve_coreferences_counted(
        text,
        &parsed,
        &CorefLimits::DEFAULT,
        &CancellationToken::new(),
    )
    .map_err(|err| coref_diag(err, span))?;
    Ok(Value::List(chains.iter().map(chain_value).collect()))
}

fn required_offset(
    rec: &BTreeMap<String, Value>,
    field: &str,
    span: Span,
) -> Result<u32, Diagnostic> {
    match rec.get(field) {
        Some(Value::U64(n)) => offset_from_u64(*n).map_err(|e| coref_diag(e, span)),
        Some(Value::I64(n)) => offset_from_i64(*n).map_err(|e| coref_diag(e, span)),
        _ => Err(Diagnostic::new(
            DiagCode::E100,
            span,
            format!("mention needs a numeric {field}"),
        )),
    }
}

fn coref_diag(err: CorefError, span: Span) -> Diagnostic {
    let code = if err.is_resource_error() {
        DiagCode::E400
    } else {
        DiagCode::E100
    };
    Diagnostic::new(code, span, err.to_string())
}

fn chain_value(chain: &crate::nlp::coref::CorefChain) -> Value {
    let mentions: Vec<Value> = chain
        .mentions
        .iter()
        .map(|m| {
            let mut rec = BTreeMap::new();
            rec.insert("start".into(), Value::U64(u64::from(m.span.start_utf8)));
            rec.insert("end".into(), Value::U64(u64::from(m.span.end_utf8)));
            rec.insert("text".into(), Value::String(m.text.clone()));
            rec.insert("kind".into(), Value::String(m.kind.as_label().into()));
            Value::Record(rec)
        })
        .collect();
    let mut rec = BTreeMap::new();
    rec.insert("id".into(), Value::U64(u64::from(chain.id)));
    rec.insert("mentions".into(), Value::List(mentions));
    Value::Record(rec)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mention(start: u32, end: u32, text: &str, kind: &str) -> Value {
        let mut r = BTreeMap::new();
        r.insert("start".into(), Value::U64(u64::from(start)));
        r.insert("end".into(), Value::U64(u64::from(end)));
        r.insert("text".into(), Value::String(text.into()));
        r.insert("kind".into(), Value::String(kind.into()));
        Value::Record(r)
    }

    fn record(text: &str, mentions: Vec<Value>) -> Value {
        let mut rec = BTreeMap::new();
        rec.insert("text".into(), Value::String(text.into()));
        rec.insert("mentions".into(), Value::List(mentions));
        Value::Record(rec)
    }

    #[test]
    fn resolves_pronoun() {
        let source = "John ran. He fell.";
        let r = coref_resolve(
            &record(
                source,
                vec![
                    mention(0, 4, "John", "proper"),
                    mention(10, 12, "He", "pronoun"),
                ],
            ),
            Span { start: 0, end: 0 },
        );
        match r.expect("coref") {
            Value::List(chains) => assert_eq!(chains.len(), 1),
            _ => panic!("expected list"),
        }
    }

    #[test]
    fn missing_mentions_is_empty_success() {
        let mut rec = BTreeMap::new();
        rec.insert("text".into(), Value::String("John ran.".into()));
        let r = coref_resolve(&Value::Record(rec), Span { start: 0, end: 0 }).expect("empty mode");
        match r {
            Value::List(chains) => assert!(chains.is_empty()),
            _ => panic!("expected list"),
        }
    }

    #[test]
    fn rejects_non_record() {
        let r = coref_resolve(&Value::I64(1), Span { start: 0, end: 0 });
        assert!(r.is_err());
    }

    #[test]
    fn rejects_negative_offset() {
        let mut rec = BTreeMap::new();
        rec.insert("start".into(), Value::I64(-1));
        rec.insert("end".into(), Value::U64(4));
        rec.insert("text".into(), Value::String("John".into()));
        rec.insert("kind".into(), Value::String("proper".into()));
        let err = coref_resolve(
            &record("John", vec![Value::Record(rec)]),
            Span { start: 0, end: 0 },
        )
        .unwrap_err();
        assert_eq!(err.code, DiagCode::E100);
    }

    #[test]
    fn rejects_unknown_kind() {
        let err = coref_resolve(
            &record("John", vec![mention(0, 4, "John", "entity")]),
            Span { start: 0, end: 0 },
        )
        .unwrap_err();
        assert_eq!(err.code, DiagCode::E100);
        assert!(err.message.contains("kind"));
    }

    #[test]
    fn rejects_wrongly_typed_mentions() {
        let mut rec = BTreeMap::new();
        rec.insert("text".into(), Value::String("John".into()));
        rec.insert("mentions".into(), Value::I64(1));
        let err = coref_resolve(&Value::Record(rec), Span { start: 0, end: 0 }).unwrap_err();
        assert_eq!(err.code, DiagCode::E100);
    }

    #[test]
    fn rejects_unsorted_host_mentions() {
        let source = "John Mary";
        let err = coref_resolve(
            &record(
                source,
                vec![
                    mention(5, 9, "Mary", "proper"),
                    mention(0, 4, "John", "proper"),
                ],
            ),
            Span { start: 0, end: 0 },
        )
        .unwrap_err();
        assert_eq!(err.code, DiagCode::E100);
    }

    #[test]
    fn rejects_overflow_offset() {
        let mut rec = BTreeMap::new();
        rec.insert("start".into(), Value::U64(u64::from(u32::MAX) + 1));
        rec.insert("end".into(), Value::U64(u64::from(u32::MAX) + 2));
        rec.insert("text".into(), Value::String("John".into()));
        rec.insert("kind".into(), Value::String("proper".into()));
        let err = coref_resolve(
            &record("John", vec![Value::Record(rec)]),
            Span { start: 0, end: 0 },
        )
        .unwrap_err();
        assert_eq!(err.code, DiagCode::E100);
    }
}
