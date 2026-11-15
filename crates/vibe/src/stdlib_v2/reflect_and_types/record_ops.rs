//! Null-prototype record operations (`record.*`) with prototype-pollution
//! elimination and pure copy-on-write semantics (`vibe-0.2`).

use std::collections::BTreeMap;

use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;
use crate::value::Value;

/// Forbidden JavaScript prototype-pollution keys that are stripped/rejected at all boundaries.
pub const FORBIDDEN_PROTOTYPE_KEYS: &[&str] = &["__proto__", "prototype", "constructor"];

/// Returns `true` if `key` is a forbidden prototype-pollution vector.
pub fn is_prototype_pollution_key(key: &str) -> bool {
    FORBIDDEN_PROTOTYPE_KEYS.contains(&key)
}

/// Dispatches `record.*` standard library calls.
pub fn call_record_ops(
    path: &str,
    args: &[Value],
    span: Span,
) -> Result<Option<Value>, Diagnostic> {
    match path {
        "record.keys" => {
            let map = expect_record(args.first(), "record.keys", span)?;
            let keys = map
                .keys()
                .filter(|k| !is_prototype_pollution_key(k))
                .map(|k| Value::String(k.clone()))
                .collect();
            Ok(Some(Value::List(keys)))
        }
        "record.values" => {
            let map = expect_record(args.first(), "record.values", span)?;
            let vals = map
                .iter()
                .filter(|(k, _)| !is_prototype_pollution_key(k))
                .map(|(_, v)| v.clone())
                .collect();
            Ok(Some(Value::List(vals)))
        }
        "record.entries" => {
            let map = expect_record(args.first(), "record.entries", span)?;
            let entries = map
                .iter()
                .filter(|(k, _)| !is_prototype_pollution_key(k))
                .map(|(k, v)| Value::List(vec![Value::String(k.clone()), v.clone()]))
                .collect();
            Ok(Some(Value::List(entries)))
        }
        "record.from_entries" => {
            let Some(Value::List(entries)) = args.first() else {
                return Err(Diagnostic::new(
                    DiagCode::E600,
                    span,
                    "record.from_entries expects a List of [key, value] pairs or {key, value} records",
                ));
            };
            let mut out = BTreeMap::new();
            for entry in entries {
                match entry {
                    Value::List(pair) if pair.len() >= 2 => {
                        let k = scalar_key_string(&pair[0]);
                        if !is_prototype_pollution_key(&k) {
                            out.insert(k, pair[1].clone());
                        }
                    }
                    Value::Record(rec) => {
                        if let (Some(k_val), Some(v_val)) = (rec.get("key"), rec.get("value")) {
                            let k = scalar_key_string(k_val);
                            if !is_prototype_pollution_key(&k) {
                                out.insert(k, v_val.clone());
                            }
                        }
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagCode::E600,
                            span,
                            "record.from_entries entry must be [key, value] or {key, value}",
                        ))
                    }
                }
            }
            Ok(Some(Value::Record(out)))
        }
        "record.merge" => {
            if args.len() < 2 {
                return Err(Diagnostic::new(
                    DiagCode::E600,
                    span,
                    "record.merge requires 2 Record arguments",
                ));
            }
            let a = expect_record(Some(&args[0]), "record.merge", span)?;
            let b = expect_record(Some(&args[1]), "record.merge", span)?;
            Ok(Some(Value::Record(merge_records_clean(a, b))))
        }
        "record.group_by" => {
            if args.len() < 2 {
                return Err(Diagnostic::new(
                    DiagCode::E600,
                    span,
                    "record.group_by requires (list_of_records, key_field)",
                ));
            }
            let Value::List(items) = &args[0] else {
                return Err(Diagnostic::new(
                    DiagCode::E600,
                    span,
                    "record.group_by expects a List as argument 0",
                ));
            };
            let key_field = match &args[1] {
                Value::String(s) | Value::Iri(s) => s.as_str(),
                _ => {
                    return Err(Diagnostic::new(
                        DiagCode::E600,
                        span,
                        "record.group_by expects a String key_field as argument 1",
                    ))
                }
            };
            let mut groups: BTreeMap<String, Vec<Value>> = BTreeMap::new();
            for item in items {
                if let Value::Record(rec) = item {
                    let group_key = rec
                        .get(key_field)
                        .map(scalar_key_string)
                        .unwrap_or_else(|| "null".to_string());
                    if !is_prototype_pollution_key(&group_key) {
                        groups.entry(group_key).or_default().push(item.clone());
                    }
                }
            }
            let out = groups
                .into_iter()
                .map(|(k, v)| (k, Value::List(v)))
                .collect();
            Ok(Some(Value::Record(out)))
        }
        "record.has_own" => {
            if args.len() < 2 {
                return Err(Diagnostic::new(
                    DiagCode::E600,
                    span,
                    "record.has_own requires (record, key)",
                ));
            }
            let map = expect_record(Some(&args[0]), "record.has_own", span)?;
            let key = scalar_key_string(&args[1]);
            let has = !is_prototype_pollution_key(&key) && map.contains_key(&key);
            Ok(Some(Value::Bool(has)))
        }
        "record.freeze_cow" => {
            let map = expect_record(args.first(), "record.freeze_cow", span)?;
            let mut frozen = BTreeMap::new();
            for (k, v) in map {
                if !is_prototype_pollution_key(k) {
                    frozen.insert(k.clone(), v.clone());
                }
            }
            frozen.insert("__frozen".to_string(), Value::Bool(true));
            Ok(Some(Value::Record(frozen)))
        }
        _ => Ok(None),
    }
}

fn expect_record<'a>(
    val: Option<&'a Value>,
    op: &str,
    span: Span,
) -> Result<&'a BTreeMap<String, Value>, Diagnostic> {
    match val {
        Some(Value::Record(map)) => Ok(map),
        _ => Err(Diagnostic::new(
            DiagCode::E600,
            span,
            format!("{op} expects a Record argument"),
        )),
    }
}

fn merge_records_clean(
    a: &BTreeMap<String, Value>,
    b: &BTreeMap<String, Value>,
) -> BTreeMap<String, Value> {
    let mut out = BTreeMap::new();
    for (k, v) in a {
        if !is_prototype_pollution_key(k) {
            out.insert(k.clone(), v.clone());
        }
    }
    for (k, v) in b {
        if !is_prototype_pollution_key(k) {
            match (out.get(k), v) {
                (Some(Value::Record(sub_a)), Value::Record(sub_b)) => {
                    out.insert(k.clone(), Value::Record(merge_records_clean(sub_a, sub_b)));
                }
                _ => {
                    out.insert(k.clone(), v.clone());
                }
            }
        }
    }
    out
}

fn scalar_key_string(val: &Value) -> String {
    match val {
        Value::String(s) | Value::Iri(s) => s.clone(),
        Value::I64(n) => n.to_string(),
        Value::U64(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
        other => other.to_string(),
    }
}
