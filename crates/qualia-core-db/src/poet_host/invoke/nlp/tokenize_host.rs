//! `NLP.tokenize` / `NLP.split_sentences` Host binds.

use super::super::args;
use crate::nlp::budget::reject_source;
use crate::nlp::tokenize::{split_sentences, tokenize, TokenKind};
use vibe::{DiagCode, Diagnostic, Span, Value};

fn kind_name(k: TokenKind) -> &'static str {
    match k {
        TokenKind::Word => "word",
        TokenKind::Number => "number",
        TokenKind::Punct => "punct",
        TokenKind::Other => "other",
    }
}

/// `NLP.tokenize` — Args: `{ text: string }` or bare string. Out: `{ tokens: [...] }`.
pub fn tokenize_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let text = match args_v {
        Value::String(s) => s.as_str(),
        _ => args::rec_str(args_v, "text")
            .ok_or_else(|| args::bad(span, "NLP.tokenize needs text: string"))?,
    };
    reject_source(text.len())
        .map_err(|_| Diagnostic::new(DiagCode::E400, span, "NLP.tokenize exceeds 256 KiB"))?;
    let tokens: Vec<Value> = tokenize(text)
        .into_iter()
        .map(|t| {
            args::record([
                ("kind", Value::String(kind_name(t.kind).into())),
                ("text", Value::String(t.text.into())),
                ("start", Value::U64(t.span.start_utf8 as u64)),
                ("end", Value::U64(t.span.end_utf8 as u64)),
            ])
        })
        .collect();
    Ok(args::record([("tokens", Value::List(tokens))]))
}

/// `NLP.split_sentences` — Args: `{ text: string }` or bare string. Out: `{ sentences: [...] }`.
pub fn split_sentences_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let text = match args_v {
        Value::String(s) => s.as_str(),
        _ => args::rec_str(args_v, "text")
            .ok_or_else(|| args::bad(span, "NLP.split_sentences needs text: string"))?,
    };
    reject_source(text.len()).map_err(|_| {
        Diagnostic::new(DiagCode::E400, span, "NLP.split_sentences exceeds 256 KiB")
    })?;
    let sentences: Vec<Value> = split_sentences(text)
        .into_iter()
        .map(|s| {
            args::record([
                ("start", Value::U64(s.span.start_utf8 as u64)),
                ("end", Value::U64(s.span.end_utf8 as u64)),
            ])
        })
        .collect();
    Ok(args::record([("sentences", Value::List(sentences))]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn wave18_tokenize_words_and_number() {
        let mut m = BTreeMap::new();
        m.insert("text".into(), Value::String("Hello 42 .".into()));
        let out = tokenize_host(&Value::Record(m), Span { start: 0, end: 0 }).unwrap();
        let Value::List(tokens) = args::rec(&out, "tokens").unwrap() else {
            panic!("tokens");
        };
        assert!(
            tokens.len() >= 2,
            "expected >=2 tokens, got {}",
            tokens.len()
        );
        assert_eq!(args::rec_str(&tokens[0], "kind").unwrap(), "word");
        assert_eq!(args::rec_str(&tokens[0], "text").unwrap(), "Hello");
        assert_eq!(args::rec_str(&tokens[1], "kind").unwrap(), "number");
        assert_eq!(args::rec_str(&tokens[1], "text").unwrap(), "42");
    }

    #[test]
    fn wave18_split_sentences_two() {
        let mut m = BTreeMap::new();
        m.insert("text".into(), Value::String("One. Two!".into()));
        let out = split_sentences_host(&Value::Record(m), Span { start: 0, end: 0 }).unwrap();
        let Value::List(sents) = args::rec(&out, "sentences").unwrap() else {
            panic!("sentences");
        };
        assert!(sents.len() >= 2);
    }

    #[test]
    fn split_sentences_host_decimal_and_unicode() {
        let mut m = BTreeMap::new();
        m.insert(
            "text".into(),
            Value::String("Recorded 12.5 mm. 你好。世界！".into()),
        );
        let out = split_sentences_host(&Value::Record(m), Span { start: 0, end: 0 }).unwrap();
        let Value::List(sents) = args::rec(&out, "sentences").unwrap() else {
            panic!("sentences");
        };
        assert_eq!(
            sents.len(),
            3,
            "decimal must not split; Unicode 。！ must split"
        );
        for s in sents {
            let start = args::rec_u64(s, "start").expect("start");
            let end = args::rec_u64(s, "end").expect("end");
            assert!(end > start, "sentence span must be non-empty");
        }
    }

    #[test]
    fn tokenize_rejects_oversize_source() {
        let mut m = BTreeMap::new();
        m.insert(
            "text".into(),
            Value::String("x".repeat(crate::nlp::budget::MAX_SOURCE_BYTES + 1)),
        );
        let err = tokenize_host(&Value::Record(m), Span { start: 0, end: 0 }).unwrap_err();
        assert_eq!(err.code, DiagCode::E400);
        assert!(err.message.contains("256 KiB"));
    }

    #[test]
    fn split_sentences_rejects_oversize_source() {
        let mut m = BTreeMap::new();
        m.insert(
            "text".into(),
            Value::String("x".repeat(crate::nlp::budget::MAX_SOURCE_BYTES + 1)),
        );
        let err = split_sentences_host(&Value::Record(m), Span { start: 0, end: 0 }).unwrap_err();
        assert_eq!(err.code, DiagCode::E400);
    }
}
