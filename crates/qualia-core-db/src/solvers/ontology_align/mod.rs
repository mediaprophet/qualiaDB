//! Ontology alignment (CI-SKM ch 4) — the engine behind "machine proposes
//! `closeMatch`, signed human ratifies `exactMatch`". Given two ontologies'
//! entity similarity, it proposes a graded correspondence set as **review-required
//! `closeMatch`** — never an asserted `exactMatch` (the human-ratification
//! invariant, enforced by [`correspondence`]).
//!
//! - [`correspondence`] — result types + the guardrail.
//! - [`align`] — alignment-as-optimization (greedy + hill-climbing).

pub mod align;
pub mod correspondence;

pub use align::align;
pub use correspondence::{Alignment, Correspondence, ProposedStatus};
