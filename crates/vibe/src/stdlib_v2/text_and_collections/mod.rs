//! VibeScript v0.02 (`vibe-0.2`) Text Processing, Linear-Time Thompson NFA Regex,
//! Immutable Collections & Core Data Structures (`str.*`, `regex.*`, `list.*`, `deque.*`,
//! `pqueue.*`, `set.*`).
//!
//! Directory-backed library structure (`AGENTS.md` §0-B):
//! - `grapheme_str`: UAX #29 extended grapheme cluster strings (`str.*`)
//! - `linear_regex`: Stateless $O(n)$ non-backtracking Thompson NFA regex (`regex.*`)
//! - `collections`: Immutable collections, natural sort, seeded shuffle, deque, priority queue,
//!   and structural-value sets (`list.*`, `deque.*`, `pqueue.*`, `set.*`)

pub mod collections;
pub mod grapheme_str;
pub mod linear_regex;

pub use grapheme_str::split_graphemes;

use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;
use crate::value::Value;

/// All stdlib v2 binding paths handled by `text_and_collections`.
pub const TEXT_AND_COLLECTIONS_BINDINGS: &[&str] = &[
    "str.graphemes",
    "str.grapheme_len",
    "str.grapheme_at",
    "str.grapheme_slice",
    "str.code_points",
    "str.bytes",
    "str.trim",
    "str.trim_start",
    "str.trim_end",
    "str.pad_start",
    "str.pad_end",
    "str.split",
    "str.join",
    "str.replace_literal",
    "str.starts_with",
    "str.ends_with",
    "str.contains",
    "str.to_upper",
    "str.to_lower",
    "regex.compile",
    "regex.is_match",
    "regex.find",
    "regex.find_all",
    "regex.replace_all",
    "list.range",
    "list.at",
    "list.slice",
    "list.sort_natural",
    "list.chunk",
    "list.windows",
    "list.zip",
    "list.unzip",
    "list.enumerate",
    "list.partition",
    "list.uniq",
    "list.uniq_by",
    "list.intersperse",
    "list.flatten",
    "list.shuffle_seeded",
    "deque.new",
    "deque.push_front",
    "deque.push_back",
    "deque.pop_front",
    "deque.pop_back",
    "pqueue.new",
    "pqueue.push",
    "pqueue.pop_min",
    "pqueue.pop_max",
    "set.from_list",
    "set.union",
    "set.intersection",
    "set.difference",
    "set.symmetric_difference",
    "set.is_subset",
    "set.is_disjoint",
];

/// Route a `str.*`, `regex.*`, `list.*`, `deque.*`, `pqueue.*`, or `set.*` call.
pub fn call_text_and_collections(
    path: &str,
    args: &[Value],
    span: Span,
) -> Result<Option<Value>, Diagnostic> {
    if path.starts_with("str.") {
        return grapheme_str::call_grapheme_str(path, args, span);
    }
    if path.starts_with("regex.") {
        return linear_regex::call_linear_regex(path, args, span);
    }
    if path.starts_with("list.")
        || path.starts_with("deque.")
        || path.starts_with("pqueue.")
        || path.starts_with("set.")
    {
        return collections::call_collections(path, args, span);
    }
    Ok(None)
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

pub(crate) fn extract_opt_i64(args: &[Value], idx: usize) -> Option<i64> {
    match args.get(idx) {
        Some(Value::Null) | None => None,
        Some(v) => v.as_i64(),
    }
}

pub(crate) fn extract_list(
    args: &[Value],
    idx: usize,
    ctx: &str,
    span: Span,
) -> Result<Vec<Value>, Diagnostic> {
    match args.get(idx) {
        Some(Value::List(xs)) => Ok(xs.clone()),
        _ => Err(Diagnostic::new(
            DiagCode::E100,
            span,
            format!("{ctx}: argument #{idx} must be a list"),
        )),
    }
}

/// Resolve a signed index (supporting `-1` = last element) to an in-bounds `usize`.
pub(crate) fn resolve_element_index(idx: i64, len: usize) -> Option<usize> {
    let n = len as i64;
    let resolved = if idx < 0 { n + idx } else { idx };
    if resolved >= 0 && resolved < n {
        Some(resolved as usize)
    } else {
        None
    }
}

/// Clamp and resolve a slice bound (supporting negative indices) into `0..=len`.
pub(crate) fn resolve_slice_bound(bound: i64, len: usize) -> usize {
    let n = len as i64;
    let resolved = if bound < 0 { n + bound } else { bound };
    resolved.clamp(0, n) as usize
}
