//! Contradiction isolation. Does not explode.

use crate::modalities::paraconsistent::route_paraconsistent;
use crate::poet_host::PoetSnapshot;
use crate::NQuin;
use poet_vibe::{DiagCode, Diagnostic, Span, Value};
use std::collections::BTreeMap;

const MAX: usize = 64;

pub fn route(snap: &PoetSnapshot, span: Span) -> Result<Value, Diagnostic> {
    snap.with_live_quins(|quins| {
        let mut consistent = [NQuin::default(); MAX];
        let mut isolated = [NQuin::default(); MAX];
        let (c, i) = route_paraconsistent(quins, &mut consistent, &mut isolated)
            .map_err(|_| Diagnostic::new(DiagCode::E400, span, "paraconsistent buffer full"))?;
        let mut rec = BTreeMap::new();
        rec.insert("consistent".into(), Value::U64(c as u64));
        rec.insert("isolated".into(), Value::U64(i as u64));
        Ok(Value::Record(rec))
    })
}
