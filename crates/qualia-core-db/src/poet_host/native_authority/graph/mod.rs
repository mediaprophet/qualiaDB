// Copyright (c) 2026 Timothy Charles Holborn.
// SPDX-License-Identifier: CC-BY-NC-ND-4.0
//! Authenticated, exact-context project graphs owned by the native authority.
//!
//! This store is separate from the process-global daemon graph: raw daemon writes
//! cannot mutate it or silently acquire native authority. Every accepted write is
//! recorded in the existing authority journal before its revision is published.
//! Grants are independently signed by the adopted project issuer and bound to the
//! exact admission digest. Source membership alone confers no graph permission.
mod grant;
mod host;
pub use host::NativeGraphHost;
mod store;
pub use grant::{
    graph_grant_digest, GraphGrant, GraphRights, GraphScope, SignedGraphGrant, GRAPH_GRANT_CONTEXT,
};
pub(super) use store::{validate_graph_transition, validate_graphs, StoredGraph};
pub use store::{GraphRead, GraphReceipt, GraphSubscription};

pub const MAX_GRAPH_CONTEXTS: usize = 64;
pub const MAX_GRAPH_QUINS: usize = 4096;
pub const MAX_TOTAL_GRAPH_QUINS: usize = 16384;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GraphError {
    InvalidGrant,
    InvalidQuin,
    Signature,
    Authentication,
    StaleScope,
    Permission,
    Capacity,
    RevisionConflict,
    Cancelled,
    Poisoned,
    Busy,
    Clock,
    /// Publication is withheld; caller must reconcile the retained journal.
    Journal(String),
}
impl std::fmt::Display for GraphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "native graph: {self:?}")
    }
}
impl std::error::Error for GraphError {}
