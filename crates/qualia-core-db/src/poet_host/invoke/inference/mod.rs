//! Inference invoke seam — exposes semantic skills, grounding, and post-turn verification.
//!
//! Future crate: `qualia-inference`.

use super::args;
use crate::inference::{post_turn_verify, quant_graph_grounding};
use poet_vibe::{DiagCode, Diagnostic, Span, Value};

/// `Inference.embed` — embed text into a vector using the default TextEmbedder.
pub fn embed(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let text =
        args::as_str(args).ok_or_else(|| args::bad(span, "Inference.embed needs a string"))?;
    if text.len() > 64 * 1024 {
        return Err(Diagnostic::new(
            DiagCode::E400,
            span,
            "Inference.embed exceeds 64 KiB",
        ));
    }
    let embedder = crate::inference::semantic_skills::TextEmbedder::default();
    let vec = embedder.embed(text);
    Ok(args::f64_list_value(vec.dims.iter().map(|&x| x as f64)))
}

/// `Inference.grounding` — check whether a generation is grounded against
/// the quant graph fact store. Returns a record with `text`, `repaired`,
/// and optional `reason`.
pub fn grounding(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let prompt = args::rec_str(args, "prompt")
        .ok_or_else(|| args::bad(span, "Inference.grounding needs prompt"))?;
    let text = args::rec_str(args, "text")
        .ok_or_else(|| args::bad(span, "Inference.grounding needs text"))?;
    let result = quant_graph_grounding::maybe_ground_generation(prompt, text);
    let mut rec = std::collections::BTreeMap::new();
    rec.insert("text".into(), Value::String(result.text));
    rec.insert("repaired".into(), Value::Bool(result.repaired));
    if let Some(reason) = result.reason {
        rec.insert("reason".into(), Value::String(reason));
    }
    if let Some(hash) = result.object_hash {
        rec.insert("object_hash".into(), Value::U64(hash));
    }
    Ok(Value::Record(rec))
}

/// `Inference.verify_turn` — verify and heal a completed generation turn.
/// Returns the final text, repair status, and individual checks.
pub fn verify_turn(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let prompt = args::rec_str(args, "prompt")
        .ok_or_else(|| args::bad(span, "Inference.verify_turn needs prompt"))?;
    let draft = args::rec_str(args, "draft")
        .ok_or_else(|| args::bad(span, "Inference.verify_turn needs draft"))?;
    let result = post_turn_verify::maybe_verify_turn(prompt, draft);
    let mut rec = std::collections::BTreeMap::new();
    rec.insert("final_text".into(), Value::String(result.final_text));
    rec.insert("repaired".into(), Value::Bool(result.repaired));
    let checks: Vec<Value> = result
        .checks
        .iter()
        .map(|c| {
            let mut r = std::collections::BTreeMap::new();
            r.insert("id".into(), Value::String(c.id.clone()));
            r.insert("ok".into(), Value::Bool(c.ok));
            r.insert("detail".into(), Value::String(c.detail.clone()));
            Value::Record(r)
        })
        .collect();
    rec.insert("checks".into(), Value::List(checks));
    if let Some(reason) = result.grounding_reason {
        rec.insert("grounding_reason".into(), Value::String(reason));
    }
    Ok(Value::Record(rec))
}

/// `Inference.detect_ungrounded` — check if a generation output is ungrounded.
/// Returns a boolean and optional reason.
pub fn detect_ungrounded(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let prompt = args::rec_str(args, "prompt")
        .ok_or_else(|| args::bad(span, "Inference.detect_ungrounded needs prompt"))?;
    let draft = args::rec_str(args, "draft")
        .ok_or_else(|| args::bad(span, "Inference.detect_ungrounded needs draft"))?;
    let result = post_turn_verify::maybe_verify_turn(prompt, draft);
    let ungrounded = result.repaired || result.grounding_reason.is_some();
    let mut rec = std::collections::BTreeMap::new();
    rec.insert("ungrounded".into(), Value::Bool(ungrounded));
    if let Some(reason) = result.grounding_reason {
        rec.insert("reason".into(), Value::String(reason));
    }
    Ok(Value::Record(rec))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn embed_returns_vector() {
        let result = embed(
            &Value::String("hello world".into()),
            Span { start: 0, end: 0 },
        );
        assert!(result.is_ok());
        match result.unwrap() {
            Value::List(dims) => assert!(!dims.is_empty()),
            _ => panic!("expected list"),
        }
    }

    #[test]
    fn embed_rejects_non_string() {
        let result = embed(&Value::I64(42), Span { start: 0, end: 0 });
        assert!(result.is_err());
    }

    #[test]
    fn grounding_returns_record() {
        let mut m = BTreeMap::new();
        m.insert(
            "prompt".into(),
            Value::String("What is the capital of France?".into()),
        );
        m.insert(
            "text".into(),
            Value::String("The capital of France is Paris.".into()),
        );
        let result = grounding(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Record(rec) => {
                assert!(rec.contains_key("text"));
                assert!(rec.contains_key("repaired"));
            }
            _ => panic!("expected record"),
        }
    }

    #[test]
    fn verify_turn_returns_checks() {
        let mut m = BTreeMap::new();
        m.insert("prompt".into(), Value::String("Hello".into()));
        m.insert("draft".into(), Value::String("Hi there".into()));
        let result = verify_turn(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Record(rec) => {
                assert!(rec.contains_key("final_text"));
                assert!(rec.contains_key("checks"));
            }
            _ => panic!("expected record"),
        }
    }

    #[test]
    fn detect_ungrounded_returns_bool() {
        let mut m = BTreeMap::new();
        m.insert("prompt".into(), Value::String("Hello".into()));
        m.insert("draft".into(), Value::String("Hi".into()));
        let result = detect_ungrounded(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Record(rec) => {
                assert!(rec.contains_key("ungrounded"));
            }
            _ => panic!("expected record"),
        }
    }
}
