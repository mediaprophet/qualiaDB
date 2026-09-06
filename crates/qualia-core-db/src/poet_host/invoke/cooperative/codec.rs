//! Bounded vibe `Value` ↔ JSON helpers for cooperative record decode/encode.

use std::collections::BTreeMap;

use serde::de::DeserializeOwned;
use serde::Serialize;
use vibe::{Span, Value};

use super::super::args;

pub(super) fn vibe_to_json(v: &Value) -> Option<serde_json::Value> {
    match v {
        Value::Null => Some(serde_json::Value::Null),
        Value::Bool(b) => Some(serde_json::Value::Bool(*b)),
        Value::I64(n) => Some(serde_json::json!(*n)),
        Value::U64(n) => Some(serde_json::json!(*n)),
        Value::F64(n) => serde_json::Number::from_f64(*n).map(serde_json::Value::Number),
        Value::String(s) | Value::Iri(s) => Some(serde_json::Value::String(s.clone())),
        Value::Prefixed(p, l) => Some(serde_json::Value::String(format!("{p}:{l}"))),
        Value::List(xs) => {
            let mut out = Vec::with_capacity(xs.len());
            for x in xs {
                out.push(vibe_to_json(x)?);
            }
            Some(serde_json::Value::Array(out))
        }
        Value::Record(m) => {
            let mut obj = serde_json::Map::new();
            for (k, val) in m {
                obj.insert(k.clone(), vibe_to_json(val)?);
            }
            Some(serde_json::Value::Object(obj))
        }
        _ => None,
    }
}

pub(super) fn json_to_vibe(v: &serde_json::Value) -> Value {
    match v {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::I64(i)
            } else if let Some(u) = n.as_u64() {
                Value::U64(u)
            } else {
                Value::F64(n.as_f64().unwrap_or(0.0))
            }
        }
        serde_json::Value::String(s) => Value::String(s.clone()),
        serde_json::Value::Array(items) => Value::List(items.iter().map(json_to_vibe).collect()),
        serde_json::Value::Object(map) => {
            let mut rec = BTreeMap::new();
            for (k, val) in map {
                rec.insert(k.clone(), json_to_vibe(val));
            }
            Value::Record(rec)
        }
    }
}

pub(super) fn decode_field<T: DeserializeOwned>(
    args_v: &Value,
    key: &str,
    span: Span,
    what: &str,
) -> Result<T, vibe::Diagnostic> {
    let field = args::rec(args_v, key)
        .ok_or_else(|| args::bad(span, format!("{what} needs `{key}`")))?;
    let json = vibe_to_json(field).ok_or_else(|| {
        args::bad(
            span,
            format!("{what}: `{key}` contains unsupported Value variants"),
        )
    })?;
    serde_json::from_value(json).map_err(|e| {
        args::bad(
            span,
            format!("{what}: failed to decode `{key}` ({e})"),
        )
    })
}

pub(super) fn encode_json<T: Serialize>(
    value: &T,
    span: Span,
    what: &str,
) -> Result<Value, vibe::Diagnostic> {
    let json = serde_json::to_value(value)
        .map_err(|e| args::bad(span, format!("{what}: encode failed ({e})")))?;
    Ok(json_to_vibe(&json))
}
