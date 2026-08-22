//! Contradiction isolation. Does not explode. Args are scanned as extra NQuins when provided.

use crate::modalities::paraconsistent::route_paraconsistent;
use crate::poet_host::{value_to_quin, PoetSnapshot};
use crate::NQuin;
use std::collections::BTreeMap;
use vibe::{DiagCode, Diagnostic, Span, Value};

const MAX: usize = 64;

pub fn route(snap: &PoetSnapshot, args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let extra = extra_quins(args);
    snap.with_live_quins(|quins| {
        let mut scan = Vec::with_capacity(quins.len() + extra.len());
        scan.extend_from_slice(&extra);
        scan.extend_from_slice(quins);
        let zero = NQuin {
            subject: 0,
            predicate: 0,
            object: 0,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        let mut consistent = [zero; MAX];
        let mut isolated = [zero; MAX];
        let (c, i) = route_paraconsistent(&scan, &mut consistent, &mut isolated)
            .map_err(|_| Diagnostic::new(DiagCode::E400, span, "paraconsistent buffer full"))?;
        let mut rec = BTreeMap::new();
        rec.insert(
            "id".into(),
            Value::String("ParaconsistentLogic.route".into()),
        );
        rec.insert("evaluated".into(), Value::Bool(true));
        rec.insert("honesty".into(), Value::String(snap.honesty().into()));
        rec.insert("consistent".into(), Value::U64(c as u64));
        rec.insert("isolated".into(), Value::U64(i as u64));
        Ok(Value::Record(rec))
    })
}

fn extra_quins(args: &Value) -> Vec<NQuin> {
    match args {
        Value::List(xs) => xs.iter().filter_map(|v| value_to_quin(v, 0)).collect(),
        other => value_to_quin(other, 0).into_iter().collect(),
    }
}
