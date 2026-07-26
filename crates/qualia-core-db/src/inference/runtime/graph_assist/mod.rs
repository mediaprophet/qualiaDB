//! Explicit graph-assisted inference ABI.
//!
//! Graph retrieval is a cold/request-bound workload reducer, never an implicit token-kernel side
//! effect. Results, token spans, and prefix-page identities are caller-buffered and separately
//! accounted so model throughput cannot be relabelled as retrieval throughput.

mod identity;
mod prefix_registry;
mod query;

pub use identity::{derive_prefix_identity, PrefixIdentity};
pub use prefix_registry::{PrefixPageRegistry, PrefixPageSet, RegistryError};
pub use query::{
    query_graph_into, GraphAssistPolicy, GraphQuery, GraphQueryError, GraphSelectionReceipt,
};

#[cfg(test)]
mod tests;
