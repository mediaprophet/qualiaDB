//! VibeScript v0.02 Standard Library — Track 4: Simplicial/Sheaf/Homology Topology,
//! AI-Native Ontological Manifold, Algebraic Effects & Hardware-Sympathetic Systems.

pub mod effects_and_systems;
pub mod ontological_context;
pub mod simplex_and_sheaf;

use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;
use crate::value::Value;
use std::collections::BTreeMap;

/// All Track 4 standard library bindings registered for VibeScript v0.02.
pub const TOPOLOGY_AND_MANIFOLD_BINDINGS: &[&str] = &[
    // Simplicial complexes, hypergraphs, cellular sheaves & persistent homology (§22)
    "simplex.define",
    "simplex.fold",
    "hypergraph.bind_hyperedge",
    "sheaf.project_stalk",
    "sheaf.verify_intersection",
    "sheaf.apply_glue",
    "topology.betti_numbers",
    "topology.detect_voids",
    "homotopy.verify_equivalence",
    // AI-native ontological context manifold, latent pointers & structural AST deltas (§21.4)
    "context.fork_by_ontology",
    "context.spawn_swarm",
    "context.collapse_manifold",
    "manifold.mount",
    "context.to_latent_pointer",
    "context.query_distance",
    "inference.next_valid_logit_mask",
    "ast.apply_structural_delta",
    "diagnostic.to_compact_token_error",
    // Algebraic effects, content-addressed code, concurrency, contracts, CRDT/ZK & systems (§§14–19)
    "effect.perform",
    "effect.handle",
    "effect.resume",
    "code.hash_ast",
    "code.resolve",
    "code.spawn_at",
    "task.spawn",
    "task.await",
    "task.status",
    "task.nursery",
    "task.cancel",
    "task.with_timeout",
    "supervisor.start_link",
    "contract.requires",
    "contract.ensures",
    "contract.invariant",
    "contract.decreases",
    "state.crdt_merge",
    "value.attach_provenance",
    "value.verify_provenance",
    "crypto.prove_zk_predicate",
    "crypto.verify_zk_predicate",
    "runtime.with_budget",
    "continuation.snapshot",
    "continuation.rollback_to",
    "memory.with_arena",
    "memory.perceus_plan",
    "memory.layout_soa",
    "simd.validate_boundary",
    "channel.lock_free_spsc",
];

/// Dispatch a Track 4 `topology_and_manifold` call. Returns `Ok(None)` if `path` is not owned here.
pub fn call_topology_and_manifold(
    path: &str,
    args: &[Value],
    span: Span,
) -> Result<Option<Value>, Diagnostic> {
    if let Some(v) = simplex_and_sheaf::call_simplex_and_sheaf(path, args, span)? {
        return Ok(Some(v));
    }
    if let Some(v) = ontological_context::call_ontological_context(path, args, span)? {
        return Ok(Some(v));
    }
    if let Some(v) = effects_and_systems::call_effects_and_systems(path, args, span)? {
        return Ok(Some(v));
    }
    Ok(None)
}

pub(crate) fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

pub(crate) fn fnv1a_value(val: &Value) -> u64 {
    fnv1a_64(format!("{val:?}").as_bytes())
}

pub(crate) fn extract_str<'a>(
    args: &'a [Value],
    idx: usize,
    fn_name: &str,
    span: Span,
) -> Result<&'a str, Diagnostic> {
    match args.get(idx) {
        Some(Value::String(s) | Value::Iri(s)) => Ok(s.as_str()),
        _ => Err(Diagnostic::new(
            DiagCode::E100,
            span,
            format!("{fn_name}: argument {idx} must be a String or Iri"),
        )),
    }
}

pub(crate) fn extract_i64(
    args: &[Value],
    idx: usize,
    fn_name: &str,
    span: Span,
) -> Result<i64, Diagnostic> {
    match args.get(idx) {
        Some(Value::I64(n)) => Ok(*n),
        Some(Value::U64(n)) => Ok(*n as i64),
        _ => Err(Diagnostic::new(
            DiagCode::E100,
            span,
            format!("{fn_name}: argument {idx} must be an integer"),
        )),
    }
}

pub(crate) fn extract_record<'a>(
    args: &'a [Value],
    idx: usize,
    fn_name: &str,
    span: Span,
) -> Result<&'a BTreeMap<String, Value>, Diagnostic> {
    match args.get(idx) {
        Some(Value::Record(r)) => Ok(r),
        _ => Err(Diagnostic::new(
            DiagCode::E100,
            span,
            format!("{fn_name}: argument {idx} must be a Record"),
        )),
    }
}

pub(crate) fn extract_list<'a>(
    args: &'a [Value],
    idx: usize,
    fn_name: &str,
    span: Span,
) -> Result<&'a [Value], Diagnostic> {
    match args.get(idx) {
        Some(Value::List(l)) => Ok(l.as_slice()),
        _ => Err(Diagnostic::new(
            DiagCode::E100,
            span,
            format!("{fn_name}: argument {idx} must be a List"),
        )),
    }
}
