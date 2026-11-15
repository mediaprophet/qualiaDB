//! Input formatting, stepping, masking, typed parsing, and text selection primitives
//! (`input.*` and `selection.*` stdlib bindings for VibeScript v0.02 Zero-JS UI).

pub mod mask_and_step;
pub mod typed_parsers;

use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;
use crate::value::Value;

/// Dispatch an `input.*` or `selection.*` call in the VibeScript engine.
pub fn call_input_format(
    path: &str,
    args: &[Value],
    span: Span,
) -> Result<Option<Value>, Diagnostic> {
    match path {
        "input.step" => mask_and_step::eval_input_step(args, span).map(Some),
        "input.apply_mask" => mask_and_step::eval_apply_mask(args, span).map(Some),
        "selection.set_range" => mask_and_step::eval_selection_set_range(args, span).map(Some),
        "selection.replace_range" => {
            mask_and_step::eval_selection_replace_range(args, span).map(Some)
        }
        "input.parse_quantity" => typed_parsers::eval_parse_quantity(args, span).map(Some),
        "input.parse_currency" => typed_parsers::eval_parse_currency(args, span).map(Some),
        "input.parse_uncertainty" => typed_parsers::eval_parse_uncertainty(args, span).map(Some),
        "input.parse_coordinate" => typed_parsers::eval_parse_coordinate(args, span).map(Some),
        "input.parse_instant" => typed_parsers::eval_parse_instant(args, span).map(Some),
        "input.parse_duration" => typed_parsers::eval_parse_duration(args, span).map(Some),
        _ => Ok(None),
    }
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

pub(crate) fn extract_opt_str(args: &[Value], idx: usize) -> Option<String> {
    match args.get(idx) {
        Some(Value::String(s)) | Some(Value::Iri(s)) if !s.is_empty() => Some(s.clone()),
        _ => None,
    }
}

pub(crate) fn extract_i64(
    args: &[Value],
    idx: usize,
    ctx: &str,
    span: Span,
) -> Result<i64, Diagnostic> {
    match args.get(idx) {
        Some(Value::I64(n)) => Ok(*n),
        Some(Value::U64(n)) => Ok(*n as i64),
        Some(Value::F64(n)) => Ok(*n as i64),
        _ => Err(Diagnostic::new(
            DiagCode::E100,
            span,
            format!("{ctx}: argument #{idx} must be an integer"),
        )),
    }
}

pub(crate) fn extract_f64(
    args: &[Value],
    idx: usize,
    ctx: &str,
    span: Span,
) -> Result<f64, Diagnostic> {
    match args.get(idx).and_then(Value::as_f64) {
        Some(n) => Ok(n),
        None => Err(Diagnostic::new(
            DiagCode::E100,
            span,
            format!("{ctx}: argument #{idx} must be a number"),
        )),
    }
}
