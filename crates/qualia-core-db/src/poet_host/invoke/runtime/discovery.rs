//! List engine families and bound invoke ids.

use super::super::ids;
use crate::CAPABILITY_DESCRIPTORS;
use poet_vibe::Value;
use std::collections::BTreeMap;

pub fn list() -> Value {
    let families: Vec<Value> = CAPABILITY_DESCRIPTORS
        .iter()
        .map(|d| {
            let mut rec = BTreeMap::new();
            rec.insert("name".into(), Value::String(d.name.into()));
            rec.insert("domain".into(), Value::String(d.domain.into()));
            rec.insert("maturity".into(), Value::String(d.maturity.into()));
            rec.insert(
                "mcp_tools".into(),
                Value::List(d.mcp_tools.iter().map(|t| Value::String((*t).into())).collect()),
            );
            rec.insert(
                "surfaces".into(),
                Value::List(d.surfaces.iter().map(|t| Value::String((*t).into())).collect()),
            );
            Value::Record(rec)
        })
        .collect();
    let bound: Vec<Value> = ids::ALL_BOUND
        .iter()
        .map(|id| {
            let mut rec = BTreeMap::new();
            rec.insert("id".into(), Value::String((*id).into()));
            rec.insert("seam".into(), Value::String(ids::seam_for(id).into()));
            Value::Record(rec)
        })
        .collect();
    let mut out = BTreeMap::new();
    out.insert("families".into(), Value::List(families));
    out.insert("invoke".into(), Value::List(bound));
    out.insert("surface".into(), Value::String("vibe".into()));
    Value::Record(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_includes_seams() {
        match list() {
            Value::Record(r) => match r.get("invoke") {
                Some(Value::List(xs)) => {
                    assert!(xs.iter().any(|v| matches!(v, Value::Record(m) if m.get("seam") == Some(&Value::String("logic".into())))));
                }
                other => panic!("{other:?}"),
            },
            other => panic!("{other:?}"),
        }
    }
}
