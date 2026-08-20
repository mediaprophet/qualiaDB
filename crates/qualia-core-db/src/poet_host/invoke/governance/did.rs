//! `did:q42` topological pointer — future seam `identity/` / `qualia-governance`.

use super::super::args;
use crate::identifier::parse_did_q42;
use poet_vibe::{Diagnostic, Span, Value};

pub fn parse(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let s = args::as_str(args_v).ok_or_else(|| args::bad(span, "parse_did_q42 needs a string"))?;
    match parse_did_q42(s.as_bytes()) {
        Ok(ptr) => Ok(Value::U64(ptr)),
        Err(e) => Err(args::bad(span, format!("did:q42 rejected: {e:?}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_q42() {
        let v = parse(
            &Value::String("did:q42:abc".into()),
            Span { start: 0, end: 0 },
        )
        .unwrap();
        match v {
            Value::U64(n) => assert!(n & (1u64 << 63) != 0),
            other => panic!("{other:?}"),
        }
    }
}
