//! Cold-path Value extractors for capability.invoke. Not a hot kernel.
//!
//! Helpers are used by scientific seams that are cfg'd off on wasm-ontology.

#![allow(dead_code)]

use vibe::{DiagCode, Diagnostic, Span, Value};
use std::collections::BTreeMap;

pub fn bad(span: Span, msg: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagCode::E100, span, msg.into())
}

pub fn as_f64(v: &Value) -> Option<f64> {
    match v {
        Value::F64(n) => Some(*n),
        Value::I64(n) => Some(*n as f64),
        Value::U64(n) => Some(*n as f64),
        _ => None,
    }
}

pub fn as_u64(v: &Value) -> Option<u64> {
    match v {
        Value::U64(n) => Some(*n),
        Value::I64(n) if *n >= 0 => Some(*n as u64),
        Value::F64(n) if *n >= 0.0 && n.fract() == 0.0 => Some(*n as u64),
        _ => None,
    }
}

pub fn as_i64(v: &Value) -> Option<i64> {
    match v {
        Value::I64(n) => Some(*n),
        Value::U64(n) if *n <= i64::MAX as u64 => Some(*n as i64),
        Value::F64(n) if n.fract() == 0.0 => Some(*n as i64),
        _ => None,
    }
}

pub fn as_bool(v: &Value) -> Option<bool> {
    match v {
        Value::Bool(b) => Some(*b),
        _ => None,
    }
}

pub fn as_str(v: &Value) -> Option<&str> {
    match v {
        Value::String(s) | Value::Iri(s) => Some(s.as_str()),
        Value::Prefixed(p, l) => Some(l.as_str())
            .filter(|_| p.is_empty())
            .or(Some(l.as_str())),
        _ => None,
    }
}

pub fn rec<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    match v {
        Value::Record(m) => m.get(key),
        _ => None,
    }
}

pub fn rec_f64(v: &Value, key: &str) -> Option<f64> {
    rec(v, key).and_then(as_f64)
}

pub fn rec_u64(v: &Value, key: &str) -> Option<u64> {
    rec(v, key).and_then(as_u64)
}

pub fn rec_bool(v: &Value, key: &str) -> Option<bool> {
    rec(v, key).and_then(as_bool)
}

pub fn rec_i64(v: &Value, key: &str) -> Option<i64> {
    rec(v, key).and_then(as_i64)
}

pub fn rec_str<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    rec(v, key).and_then(as_str)
}

/// Extract a record field as `Vec<f64>` (field must be a `List` of numbers).
pub fn rec_f64_list(v: &Value, key: &str) -> Option<Vec<f64>> {
    rec(v, key).and_then(f64s)
}

/// Extract a record field as `Vec<String>` (field must be a `List` of strings).
pub fn rec_str_list(v: &Value, key: &str) -> Option<Vec<String>> {
    rec(v, key).and_then(|val| {
        list(val)?
            .iter()
            .map(|x| as_str(x).map(|s| s.to_string()))
            .collect()
    })
}

/// Extract a record field as `Vec<u64>` (field must be a `List` of integers).
pub fn rec_u64_list(v: &Value, key: &str) -> Option<Vec<u64>> {
    rec(v, key).and_then(|val| list(val)?.iter().map(as_u64).collect())
}

/// Extract a record field as `Vec<u8>` (field must be a `List` of integers 0-255).
pub fn rec_u8_list(v: &Value, key: &str) -> Option<Vec<u8>> {
    rec(v, key).and_then(u8s)
}

/// Extract a record field as `Vec<bool>` (field must be a `List` of booleans).
pub fn rec_bool_list(v: &Value, key: &str) -> Option<Vec<bool>> {
    rec(v, key).and_then(|val| {
        list(val)?
            .iter()
            .map(|v| match v {
                Value::Bool(b) => Some(*b),
                _ => None,
            })
            .collect()
    })
}

pub fn list(v: &Value) -> Option<&[Value]> {
    match v {
        Value::List(xs) => Some(xs.as_slice()),
        _ => None,
    }
}

pub fn f64s(v: &Value) -> Option<Vec<f64>> {
    list(v)?.iter().map(as_f64).collect()
}

pub fn u8s(v: &Value) -> Option<Vec<u8>> {
    list(v)?
        .iter()
        .map(|x| as_u64(x).and_then(|n| u8::try_from(n).ok()))
        .collect()
}

pub fn pair_f64(args: &Value, span: Span, what: &str) -> Result<(f64, f64), Diagnostic> {
    let xs = list(args).ok_or_else(|| bad(span, format!("{what} needs [a, b]")))?;
    let a = xs
        .first()
        .and_then(as_f64)
        .ok_or_else(|| bad(span, format!("{what} needs two numbers")))?;
    let b = xs
        .get(1)
        .and_then(as_f64)
        .ok_or_else(|| bad(span, format!("{what} needs two numbers")))?;
    Ok((a, b))
}

pub fn pair_u64(args: &Value, span: Span, what: &str) -> Result<(u64, u64), Diagnostic> {
    let xs = list(args).ok_or_else(|| bad(span, format!("{what} needs [a, b]")))?;
    let a = xs
        .first()
        .and_then(as_u64)
        .ok_or_else(|| bad(span, format!("{what} needs two integers")))?;
    let b = xs
        .get(1)
        .and_then(as_u64)
        .ok_or_else(|| bad(span, format!("{what} needs two integers")))?;
    Ok((a, b))
}

pub fn record(pairs: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    let mut m = BTreeMap::new();
    for (k, v) in pairs {
        m.insert(k.into(), v);
    }
    Value::Record(m)
}

pub fn f64_list_value(xs: impl IntoIterator<Item = f64>) -> Value {
    Value::List(xs.into_iter().map(Value::F64).collect())
}

pub fn need_scientific(span: Span, family: &str) -> Diagnostic {
    Diagnostic::new(
        DiagCode::E300,
        span,
        format!("{family} needs native or wasm-scientific"),
    )
}
