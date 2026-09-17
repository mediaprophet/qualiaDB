//! LWW CRDT merge — the sync kernel social/collab edits ride on.
//! Peer roster persistence lives in `qualia-client-core` (not this crate).

use super::super::args;
use crate::foundation::crdt::CrdtResolver;
use crate::NQuin;
use vibe::{Diagnostic, Span, Value};

pub fn merge(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let mut local = quin_at(args_v, "local", span)?;
    let mut remote = quin_at(args_v, "remote", span)?;
    if let Some(c) = args::rec(args_v, "local").and_then(|v| args::rec_u64(v, "clock")) {
        local.set_lamport_clock(c as u32);
    }
    if let Some(c) = args::rec(args_v, "remote").and_then(|v| args::rec_u64(v, "clock")) {
        remote.set_lamport_clock(c as u32);
    }
    let selfhood = args::rec_bool(args_v, "selfhood").unwrap_or(false);
    let win = CrdtResolver::resolve_lww(&local, &remote, selfhood);
    Ok(args::record([
        ("subject", Value::U64(win.subject)),
        ("predicate", Value::U64(win.predicate)),
        ("object", Value::U64(win.object)),
        ("context", Value::U64(win.context)),
        ("clock", Value::U64(win.extract_lamport_clock() as u64)),
    ]))
}

fn quin_at(args_v: &Value, key: &str, span: Span) -> Result<NQuin, Diagnostic> {
    let v =
        args::rec(args_v, key).ok_or_else(|| args::bad(span, format!("{key} quin required")))?;
    if let Value::QuinRef(qr) = v {
        let [subject, predicate, object, context, metadata, _] = qr.raw_fields();
        let parity = NQuin::calculate_parity(subject, predicate, object, context, metadata);
        return Ok(NQuin {
            subject,
            predicate,
            object,
            context,
            metadata,
            parity,
        });
    }
    let subject = args::rec_u64(v, "s")
        .or_else(|| args::rec_u64(v, "subject"))
        .ok_or_else(|| args::bad(span, format!("{key}.s missing")))?;
    let predicate = args::rec_u64(v, "p")
        .or_else(|| args::rec_u64(v, "predicate"))
        .ok_or_else(|| args::bad(span, format!("{key}.p missing")))?;
    let object = args::rec_u64(v, "o")
        .or_else(|| args::rec_u64(v, "object"))
        .ok_or_else(|| args::bad(span, format!("{key}.o missing")))?;
    let context = args::rec_u64(v, "c")
        .or_else(|| args::rec_u64(v, "context"))
        .unwrap_or(0);
    let metadata = 0;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn q(obj: u64, clock: u64) -> Value {
        let mut m = BTreeMap::new();
        m.insert("s".into(), Value::U64(1));
        m.insert("p".into(), Value::U64(2));
        m.insert("o".into(), Value::U64(obj));
        m.insert("clock".into(), Value::U64(clock));
        Value::Record(m)
    }

    #[test]
    fn later_clock_wins() {
        let mut m = BTreeMap::new();
        m.insert("local".into(), q(10, 1));
        m.insert("remote".into(), q(20, 5));
        match merge(&Value::Record(m), Span { start: 0, end: 0 }).unwrap() {
            Value::Record(r) => assert_eq!(r.get("object"), Some(&Value::U64(20))),
            other => panic!("{other:?}"),
        }
    }
}
