//! SMILES validation — `domains::chemical::organic_chemistry`.

use super::super::args;
use crate::domains::chemical::organic_chemistry::validate_smiles;
use poet_vibe::{Diagnostic, Span, Value};

pub fn smiles(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let s = args::as_str(args_v)
        .or_else(|| args::rec_str(args_v, "smiles"))
        .ok_or_else(|| args::bad(span, "validate_smiles needs a SMILES string"))?;
    let r = validate_smiles(s);
    Ok(args::record([
        ("valid", Value::Bool(r.is_valid)),
        ("atom_count", Value::U64(r.atom_count as u64)),
        (
            "error",
            r.error.map(Value::String).unwrap_or(Value::Null),
        ),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ethanol_is_valid() {
        match smiles(&Value::String("CCO".into()), Span { start: 0, end: 0 }).unwrap() {
            Value::Record(r) => assert_eq!(r.get("valid"), Some(&Value::Bool(true))),
            other => panic!("{other:?}"),
        }
    }
}
