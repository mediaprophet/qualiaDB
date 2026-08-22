//! `NLP.coref_resolve` — multi-pass sieve coreference resolution.

use crate::nlp::coref::{resolve_coreferences, CorefMention, MentionKind};
use crate::nlp::span::DocSpan;
use std::collections::BTreeMap;
use vibe::{DiagCode, Diagnostic, Span, Value};

/// `NLP.coref_resolve` — resolve coreferences over `text` given a list of
/// mentions. Each mention is `{ start, end, text, kind }` where kind is
/// `"pronoun"`, `"proper"`, or `"common"`. Returns a list of chains, each
/// `{ id, mentions: [...] }`.
pub fn coref_resolve(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let (text, mentions) = match args {
        Value::Record(rec) => {
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
            let mentions = match rec.get("mentions") {
                Some(Value::List(list)) => list.clone(),
                _ => Vec::new(),
            };
            (text, mentions)
        }
        _ => {
            return Err(Diagnostic::new(
                DiagCode::E100,
                span,
                "NLP.coref_resolve needs a { text: string, mentions: list } record",
            ))
        }
    };
    if text.len() > 256 * 1024 {
        return Err(Diagnostic::new(
            DiagCode::E400,
            span,
            "NLP.coref_resolve text exceeds 256 KiB",
        ));
    }
    let mut parsed: Vec<CorefMention> = Vec::with_capacity(mentions.len());
    for m in &mentions {
        let rec = match m {
            Value::Record(r) => r,
            _ => {
                return Err(Diagnostic::new(
                    DiagCode::E100,
                    span,
                    "NLP.coref_resolve mentions must be records",
                ))
            }
        };
        let start = match rec.get("start") {
            Some(Value::U64(n)) => *n as u32,
            Some(Value::I64(n)) => *n as u32,
            _ => {
                return Err(Diagnostic::new(
                    DiagCode::E100,
                    span,
                    "mention needs a numeric start",
                ))
            }
        };
        let end = match rec.get("end") {
            Some(Value::U64(n)) => *n as u32,
            Some(Value::I64(n)) => *n as u32,
            _ => {
                return Err(Diagnostic::new(
                    DiagCode::E100,
                    span,
                    "mention needs a numeric end",
                ))
            }
        };
        let mtext = match rec.get("text") {
            Some(Value::String(s)) => s.clone(),
            _ => {
                return Err(Diagnostic::new(
                    DiagCode::E100,
                    span,
                    "mention needs a text string",
                ))
            }
        };
        let kind = match rec.get("kind") {
            Some(Value::String(s)) => match s.as_str() {
                "pronoun" => MentionKind::Pronoun,
                "proper" => MentionKind::Proper,
                _ => MentionKind::Common,
            },
            _ => MentionKind::Common,
        };
        parsed.push(CorefMention {
            span: DocSpan::new(start, end),
            text: mtext,
            kind,
        });
    }
    let chains = resolve_coreferences(text, parsed);
    let list: Vec<Value> = chains
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
                    r.insert(
                        "kind".into(),
                        Value::String(
                            match m.kind {
                                MentionKind::Pronoun => "pronoun",
                                MentionKind::Proper => "proper",
                                MentionKind::Common => "common",
                            }
                            .into(),
                        ),
                    );
                    Value::Record(r)
                })
                .collect();
            let mut rec = BTreeMap::new();
            rec.insert("id".into(), Value::U64(c.id as u64));
            rec.insert("mentions".into(), Value::List(ms));
            Value::Record(rec)
        })
        .collect();
    Ok(Value::List(list))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mention(start: u32, end: u32, text: &str, kind: &str) -> Value {
        let mut r = BTreeMap::new();
        r.insert("start".into(), Value::U64(start as u64));
        r.insert("end".into(), Value::U64(end as u64));
        r.insert("text".into(), Value::String(text.into()));
        r.insert("kind".into(), Value::String(kind.into()));
        Value::Record(r)
    }

    #[test]
    fn resolves_pronoun() {
        let mut rec = BTreeMap::new();
        rec.insert("text".into(), Value::String("John ran. He fell.".into()));
        rec.insert(
            "mentions".into(),
            Value::List(vec![
                mention(0, 4, "John", "proper"),
                mention(11, 13, "He", "pronoun"),
            ]),
        );
        let r = coref_resolve(&Value::Record(rec), Span { start: 0, end: 0 });
        assert!(r.is_ok());
        match r.unwrap() {
            Value::List(chains) => {
                assert_eq!(chains.len(), 1);
            }
            _ => panic!("expected list"),
        }
    }

    #[test]
    fn rejects_non_record() {
        let r = coref_resolve(&Value::I64(1), Span { start: 0, end: 0 });
        assert!(r.is_err());
    }
}
