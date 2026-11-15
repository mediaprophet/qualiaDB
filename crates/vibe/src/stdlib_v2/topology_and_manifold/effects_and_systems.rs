//! Algebraic Effects, Content-Addressed Code, Structured Concurrency, Design-by-Contract,
//! CRDTs, Provenance, ZK Predicates & Hardware-Sympathetic Systems (§§14, 16, 17, 18, 19).

use super::{extract_i64, extract_list, extract_record, extract_str, fnv1a_64, fnv1a_value};
use crate::cbor_ast::{encode, TAG_VIBE_AST};
use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;
use crate::value::Value;
use crate::{parse_program, SENTINEL_BYTES};
use std::collections::BTreeMap;

pub fn call_effects_and_systems(
    path: &str,
    args: &[Value],
    span: Span,
) -> Result<Option<Value>, Diagnostic> {
    match path {
        "effect.perform" => Ok(Some(effect_perform(args, span)?)),
        "effect.handle" => Ok(Some(effect_handle(args, span)?)),
        "effect.resume" => Ok(Some(effect_resume(args, span)?)),
        "code.hash_ast" => Ok(Some(code_hash_ast(args, span)?)),
        "code.resolve" => Ok(Some(code_resolve(args, span)?)),
        "code.spawn_at" => Ok(Some(code_spawn_at(args, span)?)),
        "task.spawn" => Ok(Some(task_spawn(args, span)?)),
        "task.await" => Ok(Some(task_await(args, span)?)),
        "task.status" => Ok(Some(task_status(args, span)?)),
        "task.nursery" => Ok(Some(task_nursery(args, span)?)),
        "task.cancel" => Ok(Some(task_cancel(args, span)?)),
        "task.with_timeout" => Ok(Some(task_with_timeout(args, span)?)),
        "supervisor.start_link" => Ok(Some(supervisor_start_link(args, span)?)),
        "contract.requires" => Ok(Some(contract_check("requires", args, span)?)),
        "contract.ensures" => Ok(Some(contract_check("ensures", args, span)?)),
        "contract.invariant" => Ok(Some(contract_check("invariant", args, span)?)),
        "contract.decreases" => Ok(Some(contract_decreases(args, span)?)),
        "state.crdt_merge" => Ok(Some(state_crdt_merge(args, span)?)),
        "value.attach_provenance" => Ok(Some(value_attach_provenance(args, span)?)),
        "value.verify_provenance" => Ok(Some(value_verify_provenance(args, span)?)),
        "crypto.prove_zk_predicate" => {
            Err(unsupported_proof_route("crypto.prove_zk_predicate", span))
        }
        "crypto.verify_zk_predicate" => {
            Err(unsupported_proof_route("crypto.verify_zk_predicate", span))
        }
        "crypto.commit_predicate" => Ok(Some(crypto_commit_predicate(args, span)?)),
        "crypto.verify_commitment" => Ok(Some(crypto_verify_commitment(args, span)?)),
        "runtime.with_budget" => Ok(Some(runtime_with_budget(args, span)?)),
        "continuation.snapshot" => Ok(Some(continuation_snapshot(args, span)?)),
        "continuation.rollback_to" => Ok(Some(continuation_rollback_to(args, span)?)),
        "memory.with_arena" => Ok(Some(memory_with_arena(args, span)?)),
        "memory.perceus_plan" => Ok(Some(memory_perceus_plan(args, span)?)),
        "memory.layout_soa" => Ok(Some(memory_layout_soa(args, span)?)),
        "simd.validate_boundary" => Ok(Some(simd_validate_boundary(args, span)?)),
        "channel.lock_free_spsc" => Ok(Some(channel_lock_free_spsc(args, span)?)),
        _ => Ok(None),
    }
}

fn stamp_descriptor_only(out: &mut BTreeMap<String, Value>, kind: &str) {
    out.insert("descriptor_only".into(), Value::Bool(true));
    out.insert(
        "execution_qualification".into(),
        Value::String(kind.to_string()),
    );
}

fn effect_perform(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let op = extract_str(args, 0, "effect.perform", span)?;
    let payload = args.get(1).cloned().unwrap_or(Value::Null);
    let mut out = BTreeMap::new();
    out.insert("__effect".into(), Value::Bool(true));
    out.insert("op".into(), Value::String(op.to_string()));
    out.insert("payload".into(), payload);
    out.insert("resumed".into(), Value::Bool(false));
    stamp_descriptor_only(&mut out, "checked_effect_descriptor_only");
    Ok(Value::Record(out))
}

fn effect_handle(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let eff = extract_record(args, 0, "effect.handle", span)?;
    let handlers = extract_record(args, 1, "effect.handle", span)?;
    let op = eff.get("op").and_then(|v| v.as_str()).unwrap_or("");
    let handled_val = handlers
        .get(op)
        .or_else(|| handlers.get("_"))
        .cloned()
        .unwrap_or_else(|| eff.get("payload").cloned().unwrap_or(Value::Null));
    let mut out = BTreeMap::new();
    out.insert(
        "handled".into(),
        Value::Bool(handlers.contains_key(op) || handlers.contains_key("_")),
    );
    out.insert("op".into(), Value::String(op.to_string()));
    out.insert("result".into(), handled_val);
    stamp_descriptor_only(&mut out, "checked_effect_descriptor_only");
    Ok(Value::Record(out))
}

fn effect_resume(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let cont = extract_record(args, 0, "effect.resume", span)?;
    let val = args.get(1).cloned().unwrap_or(Value::Null);
    let mut out = cont.clone();
    out.insert("resumed".into(), Value::Bool(true));
    out.insert("value".into(), val);
    stamp_descriptor_only(&mut out, "checked_effect_descriptor_only");
    Ok(Value::Record(out))
}

#[derive(Debug, Clone)]
pub(crate) struct StoredAstArtifact {
    pub fast_index_u64: u64,
    pub normalized_src: String,
    /// Original checked source supplied by the caller. A normalized AST cell
    /// wrapper is suitable for hashing but is not the source to execute.
    pub execution_src: String,
    pub cbor_bytes: Vec<u8>,
}

static AST_CONTENT_STORE: std::sync::OnceLock<
    std::sync::Mutex<BTreeMap<String, StoredAstArtifact>>,
> = std::sync::OnceLock::new();

fn ast_content_store() -> &'static std::sync::Mutex<BTreeMap<String, StoredAstArtifact>> {
    AST_CONTENT_STORE.get_or_init(|| std::sync::Mutex::new(BTreeMap::new()))
}

fn parse_sha256_address(raw: &str) -> Option<String> {
    let hex_part = raw
        .strip_prefix("sha256:vibe:")
        .or_else(|| raw.strip_prefix("sha256:vibe-ast-0.1:"))
        .or_else(|| raw.strip_prefix("sha256:"))
        .unwrap_or(raw)
        .trim();
    if hex_part.len() != 64 || !hex_part.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    Some(hex_part.to_ascii_lowercase())
}

fn normalize_ast_source(src: &str) -> String {
    if let Some(rest) = src.trim_start().strip_prefix('=') {
        format!("const __cell = {};", rest.trim().trim_end_matches(';'))
    } else {
        src.to_string()
    }
}

/// Look up a previously content-addressed AST source by its `sha256:...` digest.
pub(crate) fn lookup_verified_ast_source(raw_address: &str) -> Option<String> {
    let expected_hex = parse_sha256_address(raw_address)?;
    let guard = ast_content_store().lock().ok()?;
    let art = guard.get(&expected_hex)?;
    let normalized = normalize_ast_source(&art.execution_src);
    let source_matches_stored_bytes = parse_program(&normalized)
        .ok()
        .map(|program| encode(&program) == art.cbor_bytes)
        .unwrap_or(false);
    if source_matches_stored_bytes && crate::crypto::sha256_hex(&art.cbor_bytes) == expected_hex {
        Some(art.execution_src.clone())
    } else {
        None
    }
}

fn code_hash_ast(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let src = extract_str(args, 0, "code.hash_ast", span)?;
    let normalized_src = normalize_ast_source(src);
    let prog = parse_program(&normalized_src)?;
    let bytes = encode(&prog);
    let fast_index_u64 = fnv1a_64(&bytes);
    let digest_hex = crate::crypto::sha256_hex(&bytes);
    let canonical_addr = format!("sha256:vibe:{digest_hex}");

    if let Ok(mut store) = ast_content_store().lock() {
        store.insert(
            digest_hex.clone(),
            StoredAstArtifact {
                fast_index_u64,
                normalized_src: normalized_src.clone(),
                execution_src: src.to_string(),
                cbor_bytes: bytes.clone(),
            },
        );
    }

    let mut out = BTreeMap::new();
    out.insert("algorithm".into(), Value::String("SHA-256".into()));
    out.insert(
        "canonical_format".into(),
        Value::String("cbor-ld:vibe-ast-0.1".into()),
    );
    out.insert("sha256_hex".into(), Value::String(digest_hex));
    out.insert("hash_hex".into(), Value::String(canonical_addr));
    out.insert(
        "fast_index_algorithm".into(),
        Value::String("FNV-1a-64".into()),
    );
    out.insert("fast_index_u64".into(), Value::U64(fast_index_u64));
    out.insert("hash_u64".into(), Value::U64(fast_index_u64));
    out.insert("ast_cbor_tag".into(), Value::I64(TAG_VIBE_AST as i64));
    out.insert("byte_len".into(), Value::I64(bytes.len() as i64));
    Ok(Value::Record(out))
}

fn code_resolve(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let hash_ref = extract_str(args, 0, "code.resolve", span)?;
    let mut out = BTreeMap::new();
    out.insert("code_hash".into(), Value::String(hash_ref.to_string()));
    out.insert("algorithm".into(), Value::String("SHA-256".into()));

    let Some(expected_hex) = parse_sha256_address(hash_ref) else {
        out.insert("resolved".into(), Value::Bool(false));
        out.insert("verified".into(), Value::Bool(false));
        out.insert("immutable".into(), Value::Bool(false));
        out.insert(
            "reason".into(),
            Value::String("invalid_sha256_digest_format".into()),
        );
        return Ok(Value::Record(out));
    };

    // Optional second argument: candidate source or CBOR bytes to verify against expected_hex.
    if let Some(Value::String(candidate_src)) = args.get(1) {
        let normalized_cand = normalize_ast_source(candidate_src);
        if let Ok(cand_prog) = parse_program(&normalized_cand) {
            let cand_bytes = encode(&cand_prog);
            let actual_hex = crate::crypto::sha256_hex(&cand_bytes);
            if actual_hex != expected_hex {
                out.insert("resolved".into(), Value::Bool(false));
                out.insert("verified".into(), Value::Bool(false));
                out.insert("immutable".into(), Value::Bool(false));
                out.insert("reason".into(), Value::String("digest_mismatch".into()));
                return Ok(Value::Record(out));
            }
        } else {
            out.insert("resolved".into(), Value::Bool(false));
            out.insert("verified".into(), Value::Bool(false));
            out.insert("immutable".into(), Value::Bool(false));
            out.insert(
                "reason".into(),
                Value::String("invalid_candidate_ast".into()),
            );
            return Ok(Value::Record(out));
        }
    }

    let stored_opt = ast_content_store()
        .lock()
        .ok()
        .and_then(|guard| guard.get(&expected_hex).cloned());

    let Some(artifact) = stored_opt else {
        out.insert("resolved".into(), Value::Bool(false));
        out.insert("verified".into(), Value::Bool(false));
        out.insert("immutable".into(), Value::Bool(false));
        out.insert("reason".into(), Value::String("content_not_found".into()));
        return Ok(Value::Record(out));
    };

    let recomputed_hex = crate::crypto::sha256_hex(&artifact.cbor_bytes);
    if recomputed_hex != expected_hex {
        out.insert("resolved".into(), Value::Bool(false));
        out.insert("verified".into(), Value::Bool(false));
        out.insert("immutable".into(), Value::Bool(false));
        out.insert("reason".into(), Value::String("digest_mismatch".into()));
        return Ok(Value::Record(out));
    }

    out.insert("resolved".into(), Value::Bool(true));
    out.insert("verified".into(), Value::Bool(true));
    out.insert("immutable".into(), Value::Bool(true));
    out.insert("sha256_hex".into(), Value::String(expected_hex));
    out.insert("fast_index_u64".into(), Value::U64(artifact.fast_index_u64));
    out.insert(
        "byte_len".into(),
        Value::I64(artifact.cbor_bytes.len() as i64),
    );
    out.insert("source".into(), Value::String(artifact.normalized_src));
    Ok(Value::Record(out))
}

fn code_spawn_at(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let node_did = extract_str(args, 0, "code.spawn_at", span)?;
    if node_did != "did:q42:local_scheduler" {
        return Err(Diagnostic::new(
            DiagCode::E600,
            span,
            format!("code.spawn_at: remote or unknown scheduler '{node_did}' is unsupported; only did:q42:local_scheduler is available"),
        ));
    }
    let code_hash = extract_str(args, 1, "code.spawn_at", span)?;
    if code_hash.starts_with("sha256:") && lookup_verified_ast_source(code_hash).is_none() {
        return Err(Diagnostic::new(
            DiagCode::E400,
            span,
            format!("code.spawn_at: content-addressed code '{code_hash}' is unavailable or failed SHA-256 verification"),
        ));
    }
    let call_args = args.get(2).cloned().unwrap_or_else(|| Value::List(vec![]));
    crate::task_scheduler::spawn_scheduled_task(node_did, code_hash, call_args, 0, span)
}

fn task_spawn(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let code_or_spec = extract_str(args, 0, "task.spawn", span)?;
    let call_args = args.get(1).cloned().unwrap_or(Value::Null);
    let elapsed_ms = match &call_args {
        Value::Record(r) => r.get("elapsed_ms").and_then(|v| v.as_i64()).unwrap_or(0),
        _ => 0,
    };
    crate::task_scheduler::spawn_scheduled_task(
        "did:q42:local_scheduler",
        code_or_spec,
        call_args,
        elapsed_ms,
        span,
    )
}

fn task_await(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let val = args.first().cloned().unwrap_or(Value::Null);
    crate::task_scheduler::await_value_standalone(val, span)
}

fn task_status(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let target = args.first().ok_or_else(|| {
        Diagnostic::new(
            DiagCode::E100,
            span,
            "task.status: missing task record or task_id argument",
        )
    })?;
    crate::task_scheduler::query_task_status(target, span)
}

fn task_nursery(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let tasks = extract_list(args, 0, "task.nursery", span)?;
    let any_failed = tasks.iter().any(|t| match t {
        Value::Record(r) => {
            r.get("status").and_then(|v| v.as_str()) == Some("failed")
                || r.get("ok") == Some(&Value::Bool(false))
        }
        _ => false,
    });
    let mut results = Vec::with_capacity(tasks.len());
    let mut live_siblings_cancelled = 0u64;
    for t in tasks {
        if let Value::Record(r) = t {
            if any_failed && r.get("status").and_then(|v| v.as_str()) != Some("failed") {
                let cancelled_val = crate::task_scheduler::cancel_scheduled_task(r, span)?;
                if let Value::Record(ref cr) = cancelled_val {
                    if cr.get("live_job_cancelled") == Some(&Value::Bool(true)) {
                        live_siblings_cancelled += 1;
                    }
                }
                results.push(cancelled_val);
            } else if !r.contains_key("status")
                || r.get("status").and_then(|v| v.as_str()) == Some("running")
            {
                if crate::task_scheduler::extract_task_id(r).is_some() {
                    let awaited = crate::task_scheduler::await_value_standalone(
                        Value::Record(r.clone()),
                        span,
                    )?;
                    results.push(awaited);
                } else {
                    let mut tr = r.clone();
                    tr.insert("status".into(), Value::String("completed".into()));
                    results.push(Value::Record(tr));
                }
            } else {
                results.push(Value::Record(r.clone()));
            }
        } else {
            results.push(t.clone());
        }
    }
    let mut out = BTreeMap::new();
    out.insert("all_succeeded".into(), Value::Bool(!any_failed));
    out.insert("cancelled_siblings".into(), Value::Bool(any_failed));
    out.insert(
        "live_siblings_cancelled".into(),
        Value::U64(live_siblings_cancelled),
    );
    out.insert("tasks".into(), Value::List(results));
    Ok(Value::Record(out))
}

fn task_cancel(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let rec = extract_record(args, 0, "task.cancel", span)?;
    crate::task_scheduler::cancel_scheduled_task(rec, span)
}

fn task_with_timeout(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let rec = extract_record(args, 0, "task.with_timeout", span)?;
    let timeout_ms = extract_i64(args, 1, "task.with_timeout", span)?;
    crate::task_scheduler::apply_task_timeout(rec, timeout_ms, span).map(Value::Record)
}

fn supervisor_start_link(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let children = extract_list(args, 0, "supervisor.start_link", span)?;
    let strategy = extract_str(args, 1, "supervisor.start_link", span)?;
    let max_restarts = args.get(2).and_then(|v| v.as_i64()).unwrap_or(3);
    let mut restarted = Vec::new();
    let mut restart_rest = false;
    for (idx, child) in children.iter().enumerate() {
        if let Value::Record(r) = child {
            let crashed = r.get("status").and_then(|v| v.as_str()) == Some("crashed");
            if crashed || (strategy == "RestForOne" && restart_rest) {
                restart_rest = true;
                restarted.push(Value::I64(idx as i64));
            }
        }
    }
    let mut out = BTreeMap::new();
    out.insert("strategy".into(), Value::String(strategy.to_string()));
    out.insert("max_restarts".into(), Value::I64(max_restarts));
    out.insert("restarted_indices".into(), Value::List(restarted.clone()));
    out.insert(
        "healthy".into(),
        Value::Bool((restarted.len() as i64) <= max_restarts),
    );
    stamp_descriptor_only(&mut out, "checked_supervisor_plan_descriptor_only");
    Ok(Value::Record(out))
}

fn contract_check(kind: &str, args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let ok = match args.first() {
        Some(Value::Bool(b)) => *b,
        _ => false,
    };
    let msg = args
        .get(1)
        .and_then(|v| v.as_str())
        .unwrap_or("contract failed");
    if !ok {
        return Err(Diagnostic::new(
            DiagCode::E500,
            span,
            format!("contract.{kind} violated: {msg}"),
        ));
    }
    let mut out = BTreeMap::new();
    out.insert("contract".into(), Value::String(kind.to_string()));
    out.insert("verified".into(), Value::Bool(true));
    Ok(Value::Record(out))
}

fn contract_decreases(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let prev = extract_i64(args, 0, "contract.decreases", span)?;
    let curr = extract_i64(args, 1, "contract.decreases", span)?;
    if curr >= prev || curr < 0 {
        return Err(Diagnostic::new(
            DiagCode::E500,
            span,
            format!("contract.decreases violated: measure {curr} did not strictly decrease from {prev} >= 0"),
        ));
    }
    let mut out = BTreeMap::new();
    out.insert("decreasing".into(), Value::Bool(true));
    out.insert("delta".into(), Value::I64(prev - curr));
    Ok(Value::Record(out))
}

fn state_crdt_merge(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let local = extract_record(args, 0, "state.crdt_merge", span)?;
    let remote = extract_record(args, 1, "state.crdt_merge", span)?;
    let mut merged = local.clone();
    for (k, rv) in remote {
        match merged.get(k) {
            Some(Value::I64(lv)) if rv.as_i64().is_some() => {
                merged.insert(k.clone(), Value::I64((*lv).max(rv.as_i64().unwrap())));
            }
            Some(lv) => {
                if fnv1a_value(rv) >= fnv1a_value(lv) {
                    merged.insert(k.clone(), rv.clone());
                }
            }
            None => {
                merged.insert(k.clone(), rv.clone());
            }
        }
    }
    Ok(Value::Record(merged))
}

static PROVENANCE_SEAL_STORE: std::sync::OnceLock<std::sync::Mutex<BTreeMap<String, String>>> =
    std::sync::OnceLock::new();

fn provenance_seal_store() -> &'static std::sync::Mutex<BTreeMap<String, String>> {
    PROVENANCE_SEAL_STORE.get_or_init(|| std::sync::Mutex::new(BTreeMap::new()))
}

fn canonical_provenance_digest(data: &Value, prov: &Value) -> String {
    let payload = format!("vibe-provenance-v1|data={data:?}|prov={prov:?}");
    crate::crypto::sha256_hex(payload.as_bytes())
}

fn value_attach_provenance(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let data = args.first().cloned().unwrap_or(Value::Null);
    let prov = extract_record(args, 1, "value.attach_provenance", span)?;
    let prov_val = Value::Record(prov.clone());
    let fast_checksum = fnv1a_value(&data) ^ fnv1a_value(&prov_val);
    let digest_sha256 = canonical_provenance_digest(&data, &prov_val);
    let seal = crate::crypto::sha256_hex(
        format!("vibe-prov-seal-v1|{digest_sha256}|{fast_checksum}").as_bytes(),
    );
    if let Ok(mut guard) = provenance_seal_store().lock() {
        guard.insert(digest_sha256.clone(), seal.clone());
    }
    let mut out = BTreeMap::new();
    out.insert("value".into(), data);
    out.insert("provenance".into(), prov_val);
    out.insert("digest_algorithm".into(), Value::String("SHA-256".into()));
    out.insert("digest_sha256".into(), Value::String(digest_sha256));
    out.insert("__provenance_seal".into(), Value::String(seal));
    out.insert("signature_u64".into(), Value::U64(fast_checksum));
    out.insert(
        "fast_checksum_algorithm".into(),
        Value::String("FNV-1a-64-non-cryptographic".into()),
    );
    out.insert(
        "verification_kind".into(),
        Value::String("content_sha256_integrity_not_external_pki".into()),
    );
    Ok(Value::Record(out))
}

fn value_verify_provenance(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let wrapped = extract_record(args, 0, "value.verify_provenance", span)?;
    let data = wrapped.get("value").cloned().unwrap_or(Value::Null);
    let prov = wrapped.get("provenance").cloned().unwrap_or(Value::Null);
    let expected_u64 = fnv1a_value(&data) ^ fnv1a_value(&prov);
    let actual_u64 = match wrapped.get("signature_u64") {
        Some(Value::U64(u)) => *u,
        _ => 0,
    };
    let expected_sha256 = canonical_provenance_digest(&data, &prov);
    let claimed_sha256 = wrapped
        .get("digest_sha256")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let claimed_seal = wrapped
        .get("__provenance_seal")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let seal_registered = provenance_seal_store()
        .lock()
        .ok()
        .and_then(|g| g.get(&expected_sha256).cloned())
        .map(|s| s == claimed_seal && !claimed_seal.is_empty())
        .unwrap_or(false);
    let integrity_verified = !claimed_sha256.is_empty()
        && claimed_sha256 == expected_sha256
        && seal_registered
        && expected_u64 == actual_u64
        && actual_u64 != 0;
    let mut out = BTreeMap::new();
    out.insert("authentic".into(), Value::Bool(integrity_verified));
    out.insert("integrity_verified".into(), Value::Bool(integrity_verified));
    out.insert("issuer_signature_verified".into(), Value::Bool(false));
    out.insert(
        "verification_kind".into(),
        Value::String("content_sha256_integrity_not_external_pki".into()),
    );
    out.insert(
        "expected_digest_sha256".into(),
        Value::String(expected_sha256),
    );
    out.insert("expected_signature".into(), Value::U64(expected_u64));
    Ok(Value::Record(out))
}

fn unsupported_proof_route(name: &str, span: Span) -> Diagnostic {
    Diagnostic::new(
        DiagCode::E600,
        span,
        format!("{name}: unsupported; no native zero-knowledge proof provider is configured"),
    )
}

fn local_predicate_commitment(predicate: &str, public_input: &Value) -> String {
    let payload =
        format!("vibe-local-predicate-commitment-v1|predicate={predicate}|public={public_input:?}");
    crate::crypto::sha256_hex(payload.as_bytes())
}

fn crypto_commit_predicate(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let predicate = extract_str(args, 0, "crypto.commit_predicate", span)?;
    if predicate.trim().is_empty() {
        return Err(Diagnostic::new(
            DiagCode::E400,
            span,
            "crypto.commit_predicate: predicate must not be empty",
        ));
    }
    let public_input = args.get(1).cloned().unwrap_or(Value::Null);
    let mut out = BTreeMap::new();
    out.insert("predicate".into(), Value::String(predicate.to_string()));
    out.insert(
        "commitment_sha256".into(),
        Value::String(local_predicate_commitment(predicate, &public_input)),
    );
    out.insert(
        "commitment_kind".into(),
        Value::String(
            "local_deterministic_binding_integrity_not_zkp_or_predicate_evaluation".into(),
        ),
    );
    out.insert("predicate_truth_verified".into(), Value::Bool(false));
    out.insert("issuer_authenticity_verified".into(), Value::Bool(false));
    Ok(Value::Record(out))
}

fn crypto_verify_commitment(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let commitment = extract_record(args, 0, "crypto.verify_commitment", span)?;
    let pred_arg = [commitment.get("predicate").cloned().unwrap_or(Value::Null)];
    let predicate = extract_str(
        &pred_arg,
        0,
        "crypto.verify_commitment.predicate",
        span,
    )?;
    let public_input = args.get(1).cloned().unwrap_or(Value::Null);
    let claimed = commitment
        .get("commitment_sha256")
        .and_then(Value::as_str)
        .unwrap_or("");
    let expected = local_predicate_commitment(predicate, &public_input);
    let mut out = BTreeMap::new();
    out.insert(
        "integrity_verified".into(),
        Value::Bool(!claimed.is_empty() && claimed == expected),
    );
    out.insert("predicate_truth_verified".into(), Value::Bool(false));
    out.insert("issuer_authenticity_verified".into(), Value::Bool(false));
    out.insert(
        "verification_kind".into(),
        Value::String(
            "local_deterministic_binding_integrity_not_zkp_or_predicate_evaluation".into(),
        ),
    );
    Ok(Value::Record(out))
}

fn runtime_with_budget(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let budget = extract_record(args, 0, "runtime.with_budget", span)?;
    let val = args.get(1).cloned().unwrap_or(Value::Null);
    let max_bytes = budget
        .get("max_bytes")
        .and_then(|v| v.as_i64())
        .unwrap_or(SENTINEL_BYTES as i64);
    if max_bytes < 0 || (max_bytes as u64) > SENTINEL_BYTES {
        return Err(Diagnostic::new(
            DiagCode::E400,
            span,
            format!("runtime.with_budget: max_bytes {max_bytes} exceeds 42 MiB Sentinel"),
        ));
    }
    let mut out = BTreeMap::new();
    out.insert("within_budget".into(), Value::Bool(true));
    out.insert("max_bytes".into(), Value::I64(max_bytes));
    out.insert("result".into(), val);
    stamp_descriptor_only(&mut out, "checked_budget_descriptor_only");
    Ok(Value::Record(out))
}

fn continuation_snapshot(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let state = extract_record(args, 0, "continuation.snapshot", span)?;
    let step = state.get("step").and_then(|v| v.as_i64()).unwrap_or(0);
    let digest = fnv1a_value(&Value::Record(state.clone()));
    let mut out = BTreeMap::new();
    out.insert("snapshot_digest".into(), Value::U64(digest));
    out.insert("step".into(), Value::I64(step));
    out.insert("state".into(), Value::Record(state.clone()));
    stamp_descriptor_only(&mut out, "checked_state_snapshot_descriptor_only");
    Ok(Value::Record(out))
}

fn continuation_rollback_to(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let snap = extract_record(args, 0, "continuation.rollback_to", span)?;
    let target_step = extract_i64(args, 1, "continuation.rollback_to", span)?;
    let mut restored = match snap.get("state") {
        Some(Value::Record(r)) => r.clone(),
        _ => snap.clone(),
    };
    restored.insert("step".into(), Value::I64(target_step));
    restored.insert("rolled_back".into(), Value::Bool(true));
    stamp_descriptor_only(&mut restored, "checked_state_rollback_descriptor_only");
    Ok(Value::Record(restored))
}

fn memory_with_arena(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let byte_cap = extract_i64(args, 0, "memory.with_arena", span)?;
    if byte_cap < 0 || (byte_cap as u64) > SENTINEL_BYTES {
        return Err(Diagnostic::new(
            DiagCode::E400,
            span,
            format!("memory.with_arena: requested {byte_cap} B exceeds 42 MiB Sentinel limit"),
        ));
    }
    let payload = args.get(1).cloned().unwrap_or(Value::Null);
    let mut out = BTreeMap::new();
    out.insert("arena_capacity".into(), Value::I64(byte_cap));
    out.insert("zero_heap_escape".into(), Value::Bool(true));
    out.insert("result".into(), payload);
    stamp_descriptor_only(&mut out, "checked_arena_descriptor_only");
    Ok(Value::Record(out))
}

fn memory_perceus_plan(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let val = args.first().cloned().unwrap_or(Value::Null);
    let ref_count = extract_i64(args, 1, "memory.perceus_plan", span)?;
    let mut out = BTreeMap::new();
    out.insert("in_place_reuse".into(), Value::Bool(ref_count == 1));
    out.insert("ref_count".into(), Value::I64(ref_count));
    out.insert("value".into(), val);
    stamp_descriptor_only(&mut out, "checked_perceus_plan_descriptor_only");
    Ok(Value::Record(out))
}

fn memory_layout_soa(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let schema = extract_record(args, 0, "memory.layout_soa", span)?;
    let count = args
        .get(1)
        .and_then(|v| match v {
            Value::Record(r) => r.get("count").and_then(|c| c.as_i64()),
            Value::I64(n) => Some(*n),
            _ => None,
        })
        .unwrap_or(64)
        .max(1);
    let mut offsets = BTreeMap::new();
    let mut cursor = 0i64;
    for k in schema.keys() {
        offsets.insert(k.clone(), Value::I64(cursor));
        let raw_bytes = count * 8;
        cursor += (raw_bytes + 63) & !63;
    }
    let mut out = BTreeMap::new();
    out.insert("layout".into(), Value::String("SoA".into()));
    out.insert("alignment_bytes".into(), Value::I64(64));
    out.insert("column_offsets".into(), Value::Record(offsets));
    out.insert("total_bytes".into(), Value::I64(cursor));
    stamp_descriptor_only(&mut out, "checked_soa_layout_descriptor_only");
    Ok(Value::Record(out))
}

fn simd_validate_boundary(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let batch = extract_list(args, 0, "simd.validate_boundary", span)?;
    let all_intact = batch.iter().all(|item| match item {
        Value::Record(r) => r.get("intact") != Some(&Value::Bool(false)),
        _ => true,
    });
    let mut out = BTreeMap::new();
    out.insert("lanes_processed".into(), Value::I64(batch.len() as i64));
    out.insert("all_intact".into(), Value::Bool(all_intact));
    Ok(Value::Record(out))
}

fn channel_lock_free_spsc(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let cap = extract_i64(args, 0, "channel.lock_free_spsc", span)?.max(1) as usize;
    let items = extract_list(args, 1, "channel.lock_free_spsc", span)?;
    let accepted: Vec<Value> = items.iter().take(cap).cloned().collect();
    let dropped = items.len().saturating_sub(cap) as i64;
    let mut out = BTreeMap::new();
    out.insert("capacity".into(), Value::I64(cap as i64));
    out.insert("queued".into(), Value::List(accepted));
    out.insert("dropped".into(), Value::I64(dropped));
    out.insert("lock_free".into(), Value::Bool(true));
    stamp_descriptor_only(&mut out, "checked_spsc_queue_descriptor_only");
    Ok(Value::Record(out))
}
