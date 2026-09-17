//! quin.statement — host seals parity. Scripts never write overlays.

use super::Host;
use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;
use crate::value::Value;

pub fn call_quin<H: Host>(
    host: &mut H,
    path: &str,
    named: &[(String, Value)],
    span: Span,
) -> Result<Option<Value>, Diagnostic> {
    if path != "quin.statement" {
        return Ok(None);
    }
    let get = |k: &str| -> Result<u64, Diagnostic> {
        let v = named.iter().find(|(n, _)| n == k).ok_or_else(|| {
            Diagnostic::new(DiagCode::E100, span, format!("quin.statement missing {k}"))
        })?;
        match &v.1 {
            Value::U64(n) => Ok(*n),
            Value::I64(n) => Ok(*n as u64),
            Value::Iri(s) | Value::String(s) => Ok(host.hash_iri(s)),
            Value::Prefixed(p, l) => Ok(host.hash_iri(&format!("{p}:{l}"))),
            _ => Err(Diagnostic::new(
                DiagCode::E100,
                span,
                format!("quin.statement {k} must be Iri/u64"),
            )),
        }
    };
    let subject = get("subject")?;
    let predicate = get("predicate")?;
    let object = get("object")?;
    let context = get("context")?;
    Ok(Some(
        host.quin_seal(subject, predicate, object, context, span)?,
    ))
}
