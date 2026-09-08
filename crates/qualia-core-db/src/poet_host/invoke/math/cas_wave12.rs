//! Wave-12 Host binds: CAS quin round-trip (`to_quins` / `from_quins`).
//!
//! Host-missing `pub fn`s from `specialized_libs::symbolic_algebra` not bound in
//! waves 1–11.

use super::super::args;
use crate::specialized_libs::symbolic_algebra as sa;
use crate::NQuin;
use vibe::{Diagnostic, Span, Value};

fn quin_record(q: &NQuin) -> Value {
    args::record([
        ("s", Value::U64(q.subject)),
        ("p", Value::U64(q.predicate)),
        ("o", Value::U64(q.object)),
        ("c", Value::U64(q.context)),
        ("m", Value::U64(q.metadata)),
    ])
}

fn quin_from_value(v: &Value, span: Span) -> Result<NQuin, Diagnostic> {
    let subject = args::rec_u64(v, "s")
        .or_else(|| args::rec_u64(v, "subject"))
        .ok_or_else(|| args::bad(span, "quin.s missing"))?;
    let predicate = args::rec_u64(v, "p")
        .or_else(|| args::rec_u64(v, "predicate"))
        .ok_or_else(|| args::bad(span, "quin.p missing"))?;
    let object = args::rec_u64(v, "o")
        .or_else(|| args::rec_u64(v, "object"))
        .ok_or_else(|| args::bad(span, "quin.o missing"))?;
    let context = args::rec_u64(v, "c")
        .or_else(|| args::rec_u64(v, "context"))
        .unwrap_or(0);
    let metadata = args::rec_u64(v, "m")
        .or_else(|| args::rec_u64(v, "metadata"))
        .unwrap_or(0);
    let parity = NQuin::calculate_parity(subject, predicate, object, context, metadata);
    Ok(NQuin {
        subject,
        predicate,
        object,
        context,
        metadata,
        parity,
    })
}

/// `SymbolicAlgebra.to_quins` — serialise expression text to post-order quin records.
/// Args: `{ expr }`. Out: `{ quins, count }`.
pub fn to_quins(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let expr = args::rec_str(args_v, "expr")
        .ok_or_else(|| args::bad(span, "SymbolicAlgebra.to_quins needs expr: string"))?;
    let e = sa::parse(expr).map_err(|e| args::bad(span, format!("parse error: {e}")))?;
    let quins = sa::to_quins(&e);
    let count = quins.len() as u64;
    let list = Value::List(quins.iter().map(quin_record).collect());
    Ok(args::record([("quins", list), ("count", Value::U64(count))]))
}

/// `SymbolicAlgebra.from_quins` — reconstruct expression from `to_quins` records.
/// Args: `{ quins }`. Out: `{ expr }`.
pub fn from_quins(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let list = args::rec(args_v, "quins")
        .ok_or_else(|| args::bad(span, "SymbolicAlgebra.from_quins needs quins: list"))?;
    let items = match list {
        Value::List(xs) => xs,
        _ => return Err(args::bad(span, "quins must be a list")),
    };
    let mut quins = Vec::with_capacity(items.len());
    for item in items {
        quins.push(quin_from_value(item, span)?);
    }
    let e = sa::from_quins(&quins).map_err(|e| args::bad(span, format!("from_quins: {e}")))?;
    Ok(args::record([("expr", Value::String(e.to_string()))]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn span() -> Span {
        Span { start: 0, end: 0 }
    }

    #[test]
    fn wave12_to_from_quins_roundtrip() {
        let mut m = BTreeMap::new();
        m.insert("expr".into(), Value::String("x + 1".into()));
        let encoded = to_quins(&Value::Record(m), span()).unwrap();
        let count = args::rec_u64(&encoded, "count").unwrap();
        assert!(count >= 2, "expected const+var+add nodes, got {count}");

        let mut back = BTreeMap::new();
        back.insert(
            "quins".into(),
            args::rec(&encoded, "quins").unwrap().clone(),
        );
        let decoded = from_quins(&Value::Record(back), span()).unwrap();
        let s = args::rec_str(&decoded, "expr").unwrap();
        assert!(
            s.contains('x') && (s.contains('1') || s.contains('+')),
            "got {s}"
        );
    }

    #[test]
    fn wave12_from_quins_rejects_empty() {
        let mut m = BTreeMap::new();
        m.insert("quins".into(), Value::List(vec![]));
        assert!(from_quins(&Value::Record(m), span()).is_err());
    }
}
