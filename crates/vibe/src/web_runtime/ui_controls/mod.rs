//! Zero-JavaScript UI Controls & Composite Widgets (`ui.*`) for VibeScript v0.02.
//!
//! Directory-backed library (`AGENTS.md` §0-B) routing standard HTML control
//! equivalents (`standard.rs`) and composite uplift widgets (`composite.rs`).

pub mod composite;
pub mod standard;

use std::collections::BTreeMap;

use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;
use crate::value::Value;

pub use standard::VALID_CONTROL_KINDS;

/// Dispatch a `ui.*` control or composite widget function call.
pub fn call_ui_controls(
    path: &str,
    args: &[Value],
    span: Span,
) -> Result<Option<Value>, Diagnostic> {
    if let Some(val) = standard::call_standard_control(path, args, span)? {
        return Ok(Some(val));
    }
    composite::call_composite_control(path, args, span)
}

pub(crate) fn prop_bool(props: &BTreeMap<String, Value>, key: &str, default: bool) -> bool {
    match props.get(key) {
        Some(Value::Bool(b)) => *b,
        _ => default,
    }
}

pub(crate) fn opt_bool(args: &[Value], idx: usize, default: bool) -> bool {
    match args.get(idx) {
        Some(Value::Bool(b)) => *b,
        _ => default,
    }
}

pub(crate) fn opt_str(args: &[Value], idx: usize, default: &str) -> String {
    match args.get(idx) {
        Some(Value::String(s)) | Some(Value::Iri(s)) => s.clone(),
        _ => default.to_string(),
    }
}

pub(crate) fn opt_i64(args: &[Value], idx: usize, default: i64) -> i64 {
    args.get(idx).and_then(Value::as_i64).unwrap_or(default)
}

pub(crate) fn opt_f64(args: &[Value], idx: usize, default: f64) -> f64 {
    args.get(idx).and_then(Value::as_f64).unwrap_or(default)
}

pub(crate) fn extract_str(
    args: &[Value],
    idx: usize,
    ctx: &str,
    span: Span,
) -> Result<String, Diagnostic> {
    match args.get(idx) {
        Some(Value::String(s)) | Some(Value::Iri(s)) => Ok(s.clone()),
        _ => Err(Diagnostic::new(
            DiagCode::E100,
            span,
            format!("{ctx}: argument #{idx} must be a string"),
        )),
    }
}

pub(crate) fn extract_i64(
    args: &[Value],
    idx: usize,
    ctx: &str,
    span: Span,
) -> Result<i64, Diagnostic> {
    args.get(idx).and_then(Value::as_i64).ok_or_else(|| {
        Diagnostic::new(
            DiagCode::E100,
            span,
            format!("{ctx}: argument #{idx} must be an integer"),
        )
    })
}

pub(crate) fn extract_f64(
    args: &[Value],
    idx: usize,
    ctx: &str,
    span: Span,
) -> Result<f64, Diagnostic> {
    args.get(idx).and_then(Value::as_f64).ok_or_else(|| {
        Diagnostic::new(
            DiagCode::E100,
            span,
            format!("{ctx}: argument #{idx} must be a number"),
        )
    })
}

pub(crate) fn extract_list(
    args: &[Value],
    idx: usize,
    ctx: &str,
    span: Span,
) -> Result<Vec<Value>, Diagnostic> {
    match args.get(idx) {
        Some(Value::List(items)) => Ok(items.clone()),
        _ => Err(Diagnostic::new(
            DiagCode::E100,
            span,
            format!("{ctx}: argument #{idx} must be a List"),
        )),
    }
}
