//! Probabilistic graphical models (PRML ch 8) — relational inference over discrete
//! variables. The governance topology is relational and lives in the edges, so a
//! factor graph with belief propagation is the native inference for it.
//!
//! - [`factor_graph`] — discrete factor graph + sum-product belief propagation.

pub mod factor_graph;

pub use factor_graph::{Factor, FactorGraph};
