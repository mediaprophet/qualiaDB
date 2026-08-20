//! Model-free text ingest — `hypermedia::TextProcessor`. Word-processor start.

use super::super::args;
use crate::hypermedia::{
    content_digest, AssetRef, AssetRole, HypermediaContainer, Processor, TextProcessor,
};
use poet_vibe::{Diagnostic, Span, Value};

pub fn ingest(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let text = args::as_str(args_v)
        .or_else(|| args::rec_str(args_v, "text"))
        .ok_or_else(|| args::bad(span, "Document.ingest needs text"))?;
    let uri = args::rec_str(args_v, "uri").unwrap_or("urn:qualia:doc:untitled");
    let bytes = text.as_bytes();
    let primary = AssetRef::new(uri, content_digest(bytes), "text/plain", AssetRole::Primary);
    let container = HypermediaContainer::new(uri, primary);
    let out = TextProcessor::default().process(uri, bytes, "text/plain");
    let topics: Vec<Value> = out
        .descriptors
        .topics
        .iter()
        .map(|t| Value::String(t.clone()))
        .collect();
    Ok(args::record([
        ("uri", Value::String(container.uri.clone())),
        ("subject", Value::U64(container.subject())),
        ("topics", Value::List(topics)),
        ("derived", Value::U64(out.derived.len() as u64)),
        ("bytes", Value::U64(bytes.len() as u64)),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finance_topic_from_invoice() {
        match ingest(
            &Value::String("Please attach the invoice and tax receipt.".into()),
            Span { start: 0, end: 0 },
        )
        .unwrap()
        {
            Value::Record(r) => match r.get("topics") {
                Some(Value::List(ts)) => {
                    assert!(ts
                        .iter()
                        .any(|t| matches!(t, Value::String(s) if s == "finance")))
                }
                other => panic!("{other:?}"),
            },
            other => panic!("{other:?}"),
        }
    }
}
