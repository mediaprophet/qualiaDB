//! 60-bit FNV IRI hash — same as lexicon.

use crate::lexicon::generate_60bit_token;
use vibe::{DiagCode, Diagnostic, Span, Value};

pub fn iri(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let s = match args {
        Value::String(s) | Value::Iri(s) => s.as_str(),
        Value::Prefixed(p, l) => {
            return Ok(Value::U64(generate_60bit_token(
                format!("{p}:{l}").as_bytes(),
            )))
        }
        _ => {
            return Err(Diagnostic::new(
                DiagCode::E100,
                span,
                "hash.iri needs a string or IRI",
            ))
        }
    };
    Ok(Value::U64(generate_60bit_token(s.as_bytes())))
}
