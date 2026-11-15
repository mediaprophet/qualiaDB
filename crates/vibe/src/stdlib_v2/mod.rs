//! VibeScript v0.02 (`vibe-0.2`) Standard Library — Directory-Backed Library (`AGENTS.md` §0-B).
//!
//! Organizes all Phase 2 capabilities from `docs/planning/Vibe vs (2).md` (§§1–22) into four
//! focused, single-purpose module tracks:
//! - [`reflect_and_types`]: Runtime type reflection (`reflect.*`), null-prototype record ops (`record.*`),
//!   immutable RFC 3986 URIs (`uri.*`), and machine-serializable CBOR-LD errors (`error.*`).
//! - [`num_and_autodiff`]: Strict non-coercing numeric parsers (`num.*`), exact base-10 financial
//!   decimals (`decimal.*`), Euclidean/checked/saturating math & SplitMix64 PRNG (`math.*`),
//!   and forward/multivariate automatic differentiation (`autodiff.*`).
//! - [`text_and_collections`]: Unicode UAX #29 extended grapheme cluster strings (`str.*`),
//!   stateless linear-time $O(n)$ Thompson NFA regex (`regex.*`), and immutable collections,
//!   natural sorting, deques, priority queues & structural sets (`list.*`, `deque.*`, `pqueue.*`, `set.*`).
//! - [`topology_and_manifold`]: Simplicial complexes, hypergraphs, cellular sheaves, $\mathbb{F}_2$
//!   persistent homology (`simplex.*`, `hypergraph.*`, `sheaf.*`, `topology.*`, `homotopy.*`),
//!   AI-native ontological context manifolds & zero-copy `did:q42` pointers (`context.*`, `manifold.*`,
//!   `inference.*`, `ast.*`, `diagnostic.*`), and algebraic effects, content-addressed code,
//!   structured concurrency, design-by-contract, CRDT/provenance/ZK, and hardware-sympathetic
//!   Perceus/SoA/SIMD systems (`effect.*`, `code.*`, `task.*`, `supervisor.*`, `contract.*`,
//!   `state.*`, `value.*`, `crypto.*`, `runtime.*`, `continuation.*`, `memory.*`, `simd.*`, `channel.*`).

pub mod num_and_autodiff;
pub mod reflect_and_types;
pub mod text_and_collections;
pub mod topology_and_manifold;

#[cfg(test)]
mod tests;

pub use num_and_autodiff::{call_num_and_autodiff, NUM_AND_AUTODIFF_BINDINGS};
pub use reflect_and_types::{call_reflect_and_types, REFLECT_AND_TYPES_BINDINGS};
pub use text_and_collections::{
    call_text_and_collections, split_graphemes, TEXT_AND_COLLECTIONS_BINDINGS,
};
pub use topology_and_manifold::{call_topology_and_manifold, TOPOLOGY_AND_MANIFOLD_BINDINGS};

use crate::error::Diagnostic;
use crate::span::Span;
use crate::value::Value;

/// Dispatches any VibeScript `v0.02` (`vibe-0.2`) standard library call.
pub fn call_stdlib_v2(
    path: &str,
    args: &[Value],
    span: Span,
) -> Result<Option<Value>, Diagnostic> {
    if let Some(v) = call_reflect_and_types(path, args, span)? {
        return Ok(Some(v));
    }
    if let Some(v) = call_num_and_autodiff(path, args, span)? {
        return Ok(Some(v));
    }
    if let Some(v) = call_text_and_collections(path, args, span)? {
        return Ok(Some(v));
    }
    if let Some(v) = call_topology_and_manifold(path, args, span)? {
        return Ok(Some(v));
    }
    Ok(None)
}
