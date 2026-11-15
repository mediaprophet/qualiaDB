//! VibeScript v0.02 Web Server, CBOR-LD & Zero-JavaScript UI/Form Runtime
//! (`web.*`, `cbor_ld.*`, `ui.*`, `form.*`, `input.*`, `selection.*`, `validate.*`, `layout.*`).
//!
//! Directory-backed library structure (`AGENTS.md` §0-B):
//! - `web_runtime/web.rs`: HTTP responses (`ZERO_JS_CSP_HEADER`), HTTP 206 range slices, mail routing, P2P permits
//! - `web_runtime/ui_controls/`: Standard HTML controls (`standard.rs`) + composite uplift widgets (`composite.rs`)
//! - `web_runtime/layout_virtual.rs`: Virtualized slices, 2D flex layout solver, splitters, anchor positioning, hit testing
//! - `web_runtime/form_state/`: Form lifecycle (`lifecycle.rs`), undo/redo & Lamport LWW CRDT (`history_crdt.rs`),
//!   provider-TLD JSON ingestion, 4-boundary disclosure, ZK field proofs, and QuinRef/Turtle serialization (`serialize_privacy.rs`)
//! - `web_runtime/input_format/`: IME-safe masking, numeric/Quantity/Instant stepping, selection ranges (`mask_and_step.rs`),
//!   and QUDT Quantity, fixed-point currency, USRI coordinate, and Instant/Duration parsers (`typed_parsers.rs`)
//! - `web_runtime/validation.rs`: 11-flag `ValidityState` checker, form-wide & cross-field invariants, and Belnap/$E_\tau$ paraconsistent validation

#[path = "web_runtime/form_state/mod.rs"]
pub mod form_state;
#[path = "web_runtime/input_format/mod.rs"]
pub mod input_format;
#[path = "web_runtime/layout_virtual.rs"]
pub mod layout_virtual;
#[path = "web_runtime/ui_controls/mod.rs"]
pub mod ui_controls;
#[path = "web_runtime/validation.rs"]
pub mod validation;
#[path = "web_runtime/web.rs"]
pub mod web;

use std::collections::BTreeMap;

use crate::cbor_ast::cbor_ld::{
    call_cbor_ld, decode_value_cbor_ld, encode_value_cbor_ld, DEFAULT_CONTEXT_IRI,
};
use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;
use crate::value::Value;

pub use web::ZERO_JS_CSP_HEADER;

/// Dispatch a `web.*`, `cbor_ld.*`, `ui.*`, `form.*`, `input.*`, `selection.*`, `validate.*`, or `layout.*` call.
pub fn call_web_or_ui(
    path: &str,
    args: &[Value],
    span: Span,
) -> Result<Option<Value>, Diagnostic> {
    if let Some(v) = call_cbor_ld(path, args, span)? {
        return Ok(Some(v));
    }
    if let Some(v) = web::call_web(path, args, span)? {
        return Ok(Some(v));
    }
    if let Some(v) = ui_controls::call_ui_controls(path, args, span)? {
        return Ok(Some(v));
    }
    if let Some(v) = layout_virtual::call_layout_virtual(path, args, span)? {
        return Ok(Some(v));
    }
    if let Some(v) = form_state::call_form_state(path, args, span)? {
        return Ok(Some(v));
    }
    if let Some(v) = input_format::call_input_format(path, args, span)? {
        return Ok(Some(v));
    }
    if let Some(v) = validation::call_validation(path, args, span)? {
        return Ok(Some(v));
    }

    // Cross-module CBOR-LD / AST / Form bridges
    match path {
        "ui.patch_to_cbor_ld" => {
            let patches = args.first().ok_or_else(|| {
                Diagnostic::new(DiagCode::E100, span, "ui.patch_to_cbor_ld requires patches")
            })?;
            let ctx = match args.get(1) {
                Some(Value::String(s)) | Some(Value::Iri(s)) => s.as_str(),
                _ => "https://qualia.world/context/ui/v1",
            };
            call_cbor_ld(
                "cbor_ld.encode",
                &[patches.clone(), Value::Iri(ctx.to_string())],
                span,
            )
        }
        "ui.projectional_editor" => {
            let id = match args.first() {
                Some(Value::String(s)) => s.clone(),
                _ => {
                    return Err(Diagnostic::new(
                        DiagCode::E100,
                        span,
                        "ui.projectional_editor requires control id",
                    ))
                }
            };
            let src = match args.get(1) {
                Some(Value::String(s)) => s.clone(),
                _ => String::new(),
            };
            let prog = crate::parse_program(&src)?;
            let projected = crate::project_program(&prog, &crate::ProjectOptions::default());
            let ast_cbor = crate::cbor_ast::encode(&prog);
            let mut rec = BTreeMap::new();
            rec.insert("id".to_string(), Value::String(id));
            rec.insert(
                "kind".to_string(),
                Value::String("projectional-editor".to_string()),
            );
            rec.insert("projected_source".to_string(), Value::String(projected));
            rec.insert(
                "ast_cbor_tag".to_string(),
                Value::I64(crate::cbor_ast::TAG_VIBE_AST as i64),
            );
            rec.insert(
                "ast_cbor_byte_len".to_string(),
                Value::I64(ast_cbor.len() as i64),
            );
            Ok(Some(Value::Record(rec)))
        }
        "ui.dag_canvas" => {
            let id = match args.first() {
                Some(Value::String(s)) => s.clone(),
                _ => {
                    return Err(Diagnostic::new(
                        DiagCode::E100,
                        span,
                        "ui.dag_canvas requires control id",
                    ))
                }
            };
            let nodes = match args.get(1) {
                Some(Value::List(xs)) => xs.clone(),
                _ => Vec::new(),
            };
            let edges = match args.get(2) {
                Some(Value::List(xs)) => xs.clone(),
                _ => Vec::new(),
            };
            let mut rec = BTreeMap::new();
            rec.insert("id".to_string(), Value::String(id));
            rec.insert("kind".to_string(), Value::String("dag-canvas".to_string()));
            rec.insert("node_count".to_string(), Value::I64(nodes.len() as i64));
            rec.insert("edge_count".to_string(), Value::I64(edges.len() as i64));
            rec.insert("nodes".to_string(), Value::List(nodes));
            rec.insert("edges".to_string(), Value::List(edges));
            Ok(Some(Value::Record(rec)))
        }
        "form.to_cbor_ld" => {
            let form = args.first().ok_or_else(|| {
                Diagnostic::new(DiagCode::E100, span, "form.to_cbor_ld requires a form record")
            })?;
            let ctx = match args.get(1) {
                Some(Value::String(s)) | Some(Value::Iri(s)) => s.as_str(),
                _ => {
                    if let Value::Record(r) = form {
                        match r.get("schema_iri") {
                            Some(Value::Iri(s)) | Some(Value::String(s)) => s.as_str(),
                            _ => DEFAULT_CONTEXT_IRI,
                        }
                    } else {
                        DEFAULT_CONTEXT_IRI
                    }
                }
            };
            call_cbor_ld(
                "cbor_ld.encode",
                &[form.clone(), Value::Iri(ctx.to_string())],
                span,
            )
        }
        "form.from_cbor_ld" => call_cbor_ld("cbor_ld.decode", args, span),
        "form.sign_submission" => {
            let form = args.first().ok_or_else(|| {
                Diagnostic::new(
                    DiagCode::E100,
                    span,
                    "form.sign_submission requires a form record",
                )
            })?;
            let ctx = match args.get(1) {
                Some(Value::String(s)) | Some(Value::Iri(s)) => s.as_str(),
                _ => DEFAULT_CONTEXT_IRI,
            };
            let signer_did = match args.get(2) {
                Some(Value::String(s)) | Some(Value::Iri(s)) => s.clone(),
                _ => "did:q42:local-signer".to_string(),
            };
            let cbor_bytes = encode_value_cbor_ld(form, ctx);
            let digest = form_state::fnv1a_64(&cbor_bytes);
            let (_, roundtrip) = decode_value_cbor_ld(&cbor_bytes).map_err(|e| {
                Diagnostic::new(DiagCode::E600, span, format!("CBOR-LD verify error: {e}"))
            })?;
            let deterministic = encode_value_cbor_ld(&roundtrip, ctx) == cbor_bytes;

            let mut rec = BTreeMap::new();
            rec.insert("signed".to_string(), Value::Bool(true));
            rec.insert("signer_did".to_string(), Value::Iri(signer_did));
            rec.insert("context".to_string(), Value::Iri(ctx.to_string()));
            rec.insert(
                "cbor_ld_byte_len".to_string(),
                Value::I64(cbor_bytes.len() as i64),
            );
            rec.insert(
                "canonical_digest_hex".to_string(),
                Value::String(format!("0x{digest:016x}")),
            );
            rec.insert("deterministic_cde".to_string(), Value::Bool(deterministic));
            Ok(Some(Value::Record(rec)))
        }
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests;
