//! Graph optimization & propagation (Graph corpus / Vector Semantics).
//!
//! - [`hierarchical_path`] — fractal/hierarchical shortest-path decomposition into
//!   independent intra-cluster subproblems (maps onto the fractal-swarm cells).
//! - [`spreading_activation`] — associative relevance propagation for the 10D→5D
//!   NQuin relevance router.

pub mod hierarchical_path;
pub mod spreading_activation;

pub use hierarchical_path::{dijkstra, hierarchical_shortest_path};
pub use spreading_activation::{spreading_activation, top_k, Edge};
