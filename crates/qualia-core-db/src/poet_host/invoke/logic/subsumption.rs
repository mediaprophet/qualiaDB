//! Description-logic subsumption.

use crate::modalities::dl::check_subsumption_quin;
use crate::poet_host::{hash_val, PoetSnapshot};
use poet_vibe::{DiagCode, Diagnostic, Span, Value};

pub fn check(snap: &PoetSnapshot, args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let Value::List(xs) = args else {
        return Err(Diagnostic::new(
            DiagCode::E100,
            span,
            "subsumption needs [sub_class, super_class]",
        ));
    };
    let sub = xs
        .first()
        .and_then(hash_val)
        .ok_or_else(|| Diagnostic::new(DiagCode::E100, span, "subsumption missing sub_class"))?;
    let sup = xs
        .get(1)
        .and_then(hash_val)
        .ok_or_else(|| Diagnostic::new(DiagCode::E100, span, "subsumption missing super_class"))?;
    Ok(Value::Bool(snap.with_live_quins(|tbox| {
        check_subsumption_quin(sub, sup, tbox)
    })))
}
