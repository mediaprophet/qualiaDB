//! Resident graph size.

use crate::poet_host::PoetSnapshot;
use std::collections::BTreeMap;
use vibe::Value;

pub fn stats(snap: &PoetSnapshot) -> Value {
    let mut rec = BTreeMap::new();
    rec.insert("quin_count".into(), Value::U64(snap.visible_count() as u64));
    rec.insert("attached".into(), Value::Bool(snap.attached));
    rec.insert("revision".into(), Value::U64(snap.revision));
    rec.insert("honesty".into(), Value::String(snap.honesty().into()));
    Value::Record(rec)
}
