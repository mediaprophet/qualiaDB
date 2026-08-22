//! LTL G/F over the live Quin trace.

use crate::modalities::temporal_ltl::{evaluate_ltl_trace, LtlFormula};
use crate::poet_host::{hash_val, PoetSnapshot};
use vibe::{DiagCode, Diagnostic, Span, Value};

pub fn globally(snap: &PoetSnapshot, args: &Value, span: Span) -> Result<Value, Diagnostic> {
    run(snap, args, span, true)
}

pub fn finally(snap: &PoetSnapshot, args: &Value, span: Span) -> Result<Value, Diagnostic> {
    run(snap, args, span, false)
}

fn run(snap: &PoetSnapshot, args: &Value, span: Span, globally: bool) -> Result<Value, Diagnostic> {
    let pred = predicate_hash(args).ok_or_else(|| {
        Diagnostic::new(
            DiagCode::E100,
            span,
            "ltl needs a predicate IRI, hash, or modal body",
        )
    })?;
    let formula = if globally {
        LtlFormula::Globally(pred)
    } else {
        LtlFormula::Finally(pred)
    };
    Ok(Value::Bool(snap.with_live_quins(|quins| {
        evaluate_ltl_trace(quins, &formula)
    })))
}

fn predicate_hash(args: &Value) -> Option<u64> {
    if let Some(h) = hash_val(args) {
        return Some(h);
    }
    match args {
        Value::Record(m) => m
            .get("body")
            .and_then(hash_val)
            .or_else(|| m.get("predicate").and_then(hash_val)),
        _ => None,
    }
}
