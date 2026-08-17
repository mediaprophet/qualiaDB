//! LTL G/F over the live Quin trace.

use crate::modalities::temporal_ltl::{evaluate_ltl_trace, LtlFormula};
use crate::poet_host::{hash_val, PoetSnapshot};
use poet_vibe::{DiagCode, Diagnostic, Span, Value};

pub fn globally(snap: &PoetSnapshot, args: &Value, span: Span) -> Result<Value, Diagnostic> {
    run(snap, args, span, true)
}

pub fn finally(snap: &PoetSnapshot, args: &Value, span: Span) -> Result<Value, Diagnostic> {
    run(snap, args, span, false)
}

fn run(snap: &PoetSnapshot, args: &Value, span: Span, globally: bool) -> Result<Value, Diagnostic> {
    let pred = hash_val(args).ok_or_else(|| {
        Diagnostic::new(DiagCode::E100, span, "ltl needs a predicate IRI or hash")
    })?;
    let formula = if globally {
        LtlFormula::Globally(pred)
    } else {
        LtlFormula::Finally(pred)
    };
    Ok(Value::Bool(
        snap.with_live_quins(|quins| evaluate_ltl_trace(quins, &formula)),
    ))
}
