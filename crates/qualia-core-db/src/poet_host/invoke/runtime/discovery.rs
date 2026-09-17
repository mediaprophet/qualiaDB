//! List engine families and bound invoke ids.

use super::super::ids;
use crate::CAPABILITY_DESCRIPTORS;
use std::collections::BTreeMap;
use vibe::Value;

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
                Value::List(
                    d.mcp_tools
                        .iter()
                        .map(|t| Value::String((*t).into()))
                        .collect(),
                ),
            );
            rec.insert(
                "surfaces".into(),
                Value::List(
                    d.surfaces
                        .iter()
                        .map(|t| Value::String((*t).into()))
                        .collect(),
                ),
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

    #[test]
    fn list_includes_experimental_nlp_family() {
        match list() {
            Value::Record(r) => match r.get("families") {
                Some(Value::List(xs)) => {
                    let nlp = xs.iter().find(|v| {
                        matches!(
                            v,
                            Value::Record(m)
                                if m.get("name") == Some(&Value::String("NLP".into()))
                        )
                    });
                    let Some(Value::Record(m)) = nlp else {
                        panic!("NLP family missing from capability discovery");
                    };
                    assert_eq!(
                        m.get("maturity"),
                        Some(&Value::String("experimental".into()))
                    );
                    assert_eq!(m.get("domain"), Some(&Value::String("language".into())));
                    match m.get("mcp_tools") {
                        Some(Value::List(tools)) => assert!(tools.is_empty()),
                        other => panic!("NLP mcp_tools must be empty, got {other:?}"),
                    }
                    match m.get("surfaces") {
                        Some(Value::List(surfaces)) => {
                            assert!(
                                surfaces.contains(&Value::String("native".into())),
                                "NLP surfaces must include native, got {surfaces:?}"
                            );
                            assert!(
                                surfaces.contains(&Value::String("library".into())),
                                "NLP surfaces must include library (compiles; invoke host is native), got {surfaces:?}"
                            );
                            assert!(
                                !surfaces.iter().any(|s| matches!(
                                    s,
                                    Value::String(t) if t == "wasm-ontology"
                                )),
                                "NLP invoke host is cfg'd out of wasm-ontology; do not claim that surface, got {surfaces:?}"
                            );
                        }
                        other => panic!("NLP surfaces missing, got {other:?}"),
                    }
                }
                other => panic!("{other:?}"),
            },
            other => panic!("{other:?}"),
        }
    }
}
