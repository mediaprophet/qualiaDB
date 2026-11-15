//! Bounded structural dependency closure over exact original artifact bytes.
//! This module does not authenticate issuers, evaluate policy or mint rights.
//! Declared-edge completeness needs a separately authenticated manifest.
mod types;
mod walk;

pub use types::{
    ClosureCandidate, ClosureEdge, ClosureError, ClosureNode, MAX_CLOSURE_EDGES, MAX_CLOSURE_NODES,
    MAX_CLOSURE_ORIGINAL_BYTES,
};
pub use walk::verify_closure;

#[cfg(test)]
mod tests;
