//! VibeScript Form State Lifecycle, Transactional Undo/Redo, Lamport LWW CRDT Merge,
//! Four-Boundary Disclosure Filtering, Zero-Knowledge Field Proofs, and Semantic Serialization
//! (`form.*` stdlib bindings).
//!
//! Directory-backed module structure (`AGENTS.md` §0-B):
//! - `lifecycle`: `form.create`, `form.get_value`, `form.set_value`, `form.get_group_value`,
//!   `form.reset`, `form.is_dirty`, `form.field_is_dirty`, `form.diff`, `form.commit_defaults`,
//!   `form.request_submit`
//! - `history_crdt`: `form.checkpoint`, `form.undo`, `form.redo`, `form.can_undo`,
//!   `form.can_redo`, `form.rollback_to`, `form.crdt_merge`
//! - `serialize_privacy`: `form.to_record`, `form.to_url_params`, `form.to_rdf_turtle`,
//!   `form.to_quins`, `form.filter_disclosure`, `form.zk_prove_field`, `form.from_provider_json`

pub mod history_crdt;
pub mod lifecycle;
pub mod serialize_privacy;

use std::collections::BTreeMap;

use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;
use crate::value::Value;

/// Maximum number of transactional undo/redo snapshots retained per form.
pub(crate) const MAX_HISTORY_LEN: usize = 64;

/// FNV-1a 64-bit offset basis (`AGENTS.md` §1).
pub(crate) const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;

/// FNV-1a 64-bit prime (`AGENTS.md` §1).
pub(crate) const FNV_PRIME: u64 = 0x100000001b3;

/// Dispatch a `form.*` call in the VibeScript engine.
pub fn call_form_state(
    path: &str,
    args: &[Value],
    span: Span,
) -> Result<Option<Value>, Diagnostic> {
    if let Some(res) = lifecycle::call_lifecycle(path, args, span)? {
        return Ok(Some(res));
    }
    if let Some(res) = history_crdt::call_history_crdt(path, args, span)? {
        return Ok(Some(res));
    }
    if let Some(res) = serialize_privacy::call_serialize_privacy(path, args, span)? {
        return Ok(Some(res));
    }
    Ok(None)
}

pub(crate) fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut h = FNV_OFFSET_BASIS;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

pub(crate) fn push_bounded(list: &mut Vec<Value>, item: Value) {
    if list.len() >= MAX_HISTORY_LEN {
        list.remove(0);
    }
    list.push(item);
}

pub(crate) fn get_sub_record(rec: &BTreeMap<String, Value>, key: &str) -> BTreeMap<String, Value> {
    match rec.get(key) {
        Some(Value::Record(sub)) => sub.clone(),
        _ => BTreeMap::new(),
    }
}

pub(crate) fn get_sub_list(rec: &BTreeMap<String, Value>, key: &str) -> Vec<Value> {
    match rec.get(key) {
        Some(Value::List(items)) => items.clone(),
        _ => Vec::new(),
    }
}

pub(crate) fn extract_schema_base(form: &BTreeMap<String, Value>) -> String {
    let base = match form.get("schema_iri") {
        Some(Value::Iri(s)) | Some(Value::String(s)) => s.as_str(),
        _ => "https://qualia.world/ontology/form#",
    };
    if base.ends_with('#') || base.ends_with('/') {
        base.to_string()
    } else {
        format!("{base}#")
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

pub(crate) fn extract_record(
    args: &[Value],
    idx: usize,
    ctx: &str,
    span: Span,
) -> Result<BTreeMap<String, Value>, Diagnostic> {
    match args.get(idx) {
        Some(Value::Record(map)) => Ok(map.clone()),
        _ => Err(Diagnostic::new(
            DiagCode::E100,
            span,
            format!("{ctx}: argument #{idx} must be a record"),
        )),
    }
}
