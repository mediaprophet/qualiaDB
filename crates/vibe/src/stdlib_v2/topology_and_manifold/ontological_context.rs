//! AI-Native Ontological Context Manifold, Latent Pointers & Structural AST Deltas (§21.4).

use super::{extract_list, extract_record, extract_str, fnv1a_64, fnv1a_value};
use crate::cbor_ast::{encode, encode_value_cbor_ld, TAG_CBOR_LD_COMPRESSED, TAG_VIBE_AST};
use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;
use crate::value::Value;
use crate::{parse_program, project_program, ProjectOptions};
use std::collections::{BTreeMap, BTreeSet};

pub fn call_ontological_context(
    path: &str,
    args: &[Value],
    span: Span,
) -> Result<Option<Value>, Diagnostic> {
    match path {
        "context.fork_by_ontology" => Ok(Some(context_fork_by_ontology(args, span)?)),
        "context.spawn_swarm" => Ok(Some(context_spawn_swarm(args, span)?)),
        "context.collapse_manifold" => Ok(Some(context_collapse_manifold(args, span)?)),
        "manifold.mount" => Ok(Some(manifold_mount(args, span)?)),
        "context.to_latent_pointer" => Ok(Some(context_to_latent_pointer(args, span)?)),
        "context.query_distance" => Ok(Some(context_query_distance(args, span)?)),
        "inference.next_valid_logit_mask" => Ok(Some(inference_next_valid_logit_mask(args, span)?)),
        "ast.apply_structural_delta" => Ok(Some(ast_apply_structural_delta(args, span)?)),
        "diagnostic.to_compact_token_error" => {
            Ok(Some(diagnostic_to_compact_token_error(args, span)?))
        }
        _ => Ok(None),
    }
}

fn context_fork_by_ontology(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let state_rec = extract_record(args, 0, "context.fork_by_ontology", span)?;
    let ontology_iri = extract_str(args, 1, "context.fork_by_ontology", span)?;
    let objective = args
        .get(2)
        .and_then(|v| v.as_str())
        .unwrap_or("ontological-subtask");

    let domain_tag = ontology_iri
        .trim_end_matches('#')
        .trim_end_matches('/')
        .rsplit(['/', ':', '#'])
        .next()
        .unwrap_or("");

    let mut projected = BTreeMap::new();
    for (k, v) in state_rec {
        if domain_tag.is_empty()
            || k.starts_with(domain_tag)
            || k.contains(domain_tag)
            || !k.contains(':')
        {
            projected.insert(k.clone(), v.clone());
        }
    }
    if projected.is_empty() {
        projected = state_rec.clone();
    }

    let proj_val = Value::Record(projected.clone());
    let cbor_bytes = encode_value_cbor_ld(&proj_val, ontology_iri);
    let fork_hash = fnv1a_64(&cbor_bytes) ^ fnv1a_64(objective.as_bytes());

    let mut out = BTreeMap::new();
    out.insert("fork_id".into(), Value::String(format!("fork:{fork_hash:016x}")));
    out.insert("ontology".into(), Value::Iri(ontology_iri.to_string()));
    out.insert("objective".into(), Value::String(objective.to_string()));
    out.insert("projected_state".into(), Value::Record(projected));
    out.insert("cbor_ld_byte_len".into(), Value::I64(cbor_bytes.len() as i64));
    Ok(Value::Record(out))
}

fn context_spawn_swarm(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let branches = extract_list(args, 0, "context.spawn_swarm", span)?;
    let swarm_hash = fnv1a_value(&Value::List(branches.to_vec()));
    let mut out = BTreeMap::new();
    out.insert("swarm_id".into(), Value::String(format!("swarm:{swarm_hash:016x}")));
    out.insert("branch_count".into(), Value::I64(branches.len() as i64));
    out.insert("branches".into(), Value::List(branches.to_vec()));
    out.insert("deterministic_order".into(), Value::Bool(true));
    Ok(Value::Record(out))
}

fn context_collapse_manifold(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let branch_outputs = extract_list(args, 0, "context.collapse_manifold", span)?;
    let shared_ontology = args
        .get(1)
        .and_then(|v| v.as_str())
        .unwrap_or("https://qualiadb.org/contexts/vibe-v0.02.jsonld");

    let mut merged = BTreeMap::new();
    let mut conflict_keys = BTreeSet::new();

    for b in branch_outputs {
        if let Value::Record(rec) = b {
            let source_map = match rec.get("projected_state") {
                Some(Value::Record(ps)) => ps,
                _ => rec,
            };
            for (k, v) in source_map {
                if k == "fork_id" || k == "cbor_ld_byte_len" {
                    continue;
                }
                if let Some(prev) = merged.get(k) {
                    if prev != v {
                        conflict_keys.insert(k.clone());
                    }
                } else {
                    merged.insert(k.clone(), v.clone());
                }
            }
        }
    }

    let conflicts: Vec<Value> = conflict_keys.into_iter().map(Value::String).collect();
    let collapsed = conflicts.is_empty();
    let merged_val = Value::Record(merged.clone());
    let _cbor = encode_value_cbor_ld(&merged_val, shared_ontology);

    let mut out = BTreeMap::new();
    out.insert("collapsed".into(), Value::Bool(collapsed));
    out.insert("ontology".into(), Value::Iri(shared_ontology.to_string()));
    out.insert("merged_state".into(), merged_val);
    out.insert("conflicts".into(), Value::List(conflicts));
    out.insert("cbor_ld_tag".into(), Value::I64(TAG_CBOR_LD_COMPRESSED as i64));
    Ok(Value::Record(out))
}

fn manifold_mount(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let resource_iri = extract_str(args, 0, "manifold.mount", span)?;
    let schema = args.get(1).cloned().unwrap_or(Value::Null);
    let h = fnv1a_64(resource_iri.as_bytes()) | (1u64 << 63);
    let mut out = BTreeMap::new();
    out.insert("mounted".into(), Value::Bool(true));
    out.insert("pointer_did".into(), Value::Iri(format!("did:q42:{h:016x}")));
    out.insert("resource_iri".into(), Value::Iri(resource_iri.to_string()));
    out.insert("token_cost".into(), Value::I64(1));
    out.insert("schema".into(), schema);
    Ok(Value::Record(out))
}

fn context_to_latent_pointer(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let target = args.first().ok_or_else(|| {
        Diagnostic::new(
            DiagCode::E100,
            span,
            "context.to_latent_pointer requires 1 argument",
        )
    })?;
    let fp = fnv1a_value(target) | (1u64 << 63);
    let mut features = Vec::with_capacity(8);
    let mut sum_sq = 0.0f64;
    let mut raw = [0.0f64; 8];
    for (i, slot) in raw.iter_mut().enumerate() {
        let byte = ((fp >> (i * 8)) & 0xFF) as f64;
        *slot = (byte / 127.5) - 1.0;
        sum_sq += (*slot) * (*slot);
    }
    let norm = sum_sq.sqrt().max(1e-12);
    for r in raw {
        features.push(Value::F64(r / norm));
    }

    let mut out = BTreeMap::new();
    out.insert("pointer_did".into(), Value::Iri(format!("did:q42:latent:{fp:016x}")));
    out.insert("fingerprint".into(), Value::U64(fp));
    out.insert("features".into(), Value::List(features));
    Ok(Value::Record(out))
}

fn extract_features(v: &Value) -> Option<Vec<f64>> {
    if let Value::Record(r) = v {
        if let Some(Value::List(l)) = r.get("features") {
            let vec: Vec<f64> = l.iter().filter_map(|x| x.as_f64()).collect();
            if !vec.is_empty() {
                return Some(vec);
            }
        }
    }
    None
}

fn context_query_distance(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    if args.len() < 2 {
        return Err(Diagnostic::new(
            DiagCode::E100,
            span,
            "context.query_distance requires 2 arguments",
        ));
    }
    if args[0] == args[1] {
        return Ok(Value::F64(0.0));
    }
    if let (Some(fa), Some(fb)) = (extract_features(&args[0]), extract_features(&args[1])) {
        let n = fa.len().min(fb.len());
        let sum_sq: f64 = (0..n).map(|i| (fa[i] - fb[i]).powi(2)).sum();
        return Ok(Value::F64(sum_sq.sqrt()));
    }
    let ha = fnv1a_value(&args[0]);
    let hb = fnv1a_value(&args[1]);
    let hamming = (ha ^ hb).count_ones() as f64 / 64.0;
    Ok(Value::F64(hamming))
}

fn inference_next_valid_logit_mask(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let _state = extract_record(args, 0, "inference.next_valid_logit_mask", span)?;
    let phase = extract_str(args, 1, "inference.next_valid_logit_mask", span)?;
    let tokens: &[&str] = match phase {
        "top_level" | "module" => &[
            "@context",
            "@ontology",
            "@Clean",
            "@Provenance",
            "let",
            "fn",
            "intent",
            "need",
            "provide",
            "promise",
            "consensus",
        ],
        "statement" | "block" => &[
            "let", "if", "match", "observe", "receive", "spawn", "emit", "yield", "Must", "May",
            "MustNot",
        ],
        "expression" => &["(", "{", "[", "true", "false", "null", "|>"],
        _ => &["let", "fn", "|>", "{", "}"],
    };
    let allowed: Vec<Value> = tokens.iter().map(|t| Value::String((*t).to_string())).collect();
    let mut out = BTreeMap::new();
    out.insert("phase".into(), Value::String(phase.to_string()));
    out.insert("allowed_tokens".into(), Value::List(allowed.clone()));
    out.insert("masked_count".into(), Value::I64(32000 - allowed.len() as i64));
    out.insert("syntax_safe".into(), Value::Bool(true));
    Ok(Value::Record(out))
}

fn ast_apply_structural_delta(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let src = extract_str(args, 0, "ast.apply_structural_delta", span)?;
    let delta = extract_record(args, 1, "ast.apply_structural_delta", span)?;

    let mut mutated_src = src.to_string();
    if let (Some(from), Some(to)) = (
        delta.get("replace_from").and_then(|v| v.as_str()),
        delta.get("replace_to").and_then(|v| v.as_str()),
    ) {
        mutated_src = mutated_src.replace(from, to);
    }
    if let Some(append_stmt) = delta.get("append_statement").and_then(|v| v.as_str()) {
        mutated_src.push('\n');
        mutated_src.push_str(append_stmt);
    }

    let prog = parse_program(&mutated_src)?;
    let projected = project_program(&prog, &ProjectOptions::default());
    let cbor_bytes = encode(&prog);

    let mut out = BTreeMap::new();
    out.insert("projected_source".into(), Value::String(projected));
    out.insert("ast_cbor_tag".into(), Value::I64(TAG_VIBE_AST as i64));
    out.insert("byte_len".into(), Value::I64(cbor_bytes.len() as i64));
    Ok(Value::Record(out))
}

fn diagnostic_to_compact_token_error(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let diag_rec = extract_record(args, 0, "diagnostic.to_compact_token_error", span)?;
    let code = diag_rec
        .get("code")
        .and_then(|v| v.as_str())
        .unwrap_or("E300");
    let start = diag_rec
        .get("start")
        .and_then(|v| v.as_i64())
        .unwrap_or(span.start as i64);
    let end = diag_rec
        .get("end")
        .and_then(|v| v.as_i64())
        .unwrap_or(span.end as i64);
    let fix = diag_rec
        .get("fix")
        .and_then(|v| v.as_str())
        .unwrap_or("align-type-or-guard");
    let directive = format!("{code}@{start}..{end}:{fix}");
    let token_est = ((directive.len() / 4) + 1) as i64;

    let mut out = BTreeMap::new();
    out.insert("code".into(), Value::String(code.to_string()));
    out.insert("span".into(), Value::List(vec![Value::I64(start), Value::I64(end)]));
    out.insert("compact_directive".into(), Value::String(directive));
    out.insert("token_estimate".into(), Value::I64(token_est));
    Ok(Value::Record(out))
}
