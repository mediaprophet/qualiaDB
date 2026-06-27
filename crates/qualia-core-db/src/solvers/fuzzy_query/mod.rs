//! **f-SPARQL** — degree-aware querying over the semantic graph.
//!
//! Classical SPARQL answers are crisp: a solution either matches or it does not.
//! f-SPARQL carries a **truth degree** in `[0, 1]` on every solution, so a query can
//! ask "guardians who are *roughly* the right age" or "relations that *strongly*
//! match", rank by confidence, and threshold (an α-cut) instead of demanding an exact
//! match. This is the query-layer face of the engine's fuzzy RDF
//! ([`crate::modalities::fuzzy_rdf_schema`]) and the identity discipline: a graded
//! answer is a *proposal with a confidence*, never an asserted fact.
//!
//! ## Not a fork of the SPARQL engine
//!
//! This is a thin, composable algebra **over the engine's own solution type**
//! ([`crate::sparql_ast::BindingRow`]). The crisp executor still does the graph
//! matching and produces rows one at a time; [`evaluate::annotate`] /
//! [`evaluate::collect_from`] attach a degree to each row and the combinators here
//! ([`solution`]) compose them. Nothing here re-implements the parser or the join
//! engine.
//!
//! ## Degree algebra
//!
//! Conjunction (a basic graph pattern / `AND`) combines degrees with a **t-norm**;
//! disjunction (`UNION`) with a **t-conorm**; `NOT` / negation with the fuzzy
//! complement. These are exactly the operators in [`crate::modalities::fuzzy`] — the
//! single source of truth — selected by [`DegreeNorm`]. Default semantics are Gödel
//! (min/max), the usual f-SPARQL choice.
//!
//! Kernel-class `Reduction` over the solution sequence; CPU path always present (§13).

pub mod evaluate;
pub mod membership;
pub mod solution;

pub use evaluate::{annotate, collect_from, conjunctive_query};
pub use solution::{FuzzyResultSet, FuzzySolution};

use crate::modalities::fuzzy::{
    fuzzy_not, t_conorm_godel, t_conorm_lukasiewicz, t_conorm_product, t_norm_godel,
    t_norm_lukasiewicz, t_norm_product,
};

/// Which fuzzy-logic operator family combines solution degrees. Each variant dispatches
/// to the corresponding operator in [`crate::modalities::fuzzy`] (reuse, not a
/// re-implementation).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DegreeNorm {
    /// Gödel: `and = min`, `or = max`. The standard f-SPARQL default.
    #[default]
    Godel,
    /// Product: `and = a·b`, `or = a + b − a·b`.
    Product,
    /// Łukasiewicz: `and = max(0, a+b−1)`, `or = min(1, a+b)`.
    Lukasiewicz,
}

impl DegreeNorm {
    /// Conjunction of two degrees (t-norm).
    pub fn and(self, a: f64, b: f64) -> f64 {
        let (a, b) = (a as f32, b as f32);
        let r = match self {
            DegreeNorm::Godel => t_norm_godel(a, b),
            DegreeNorm::Product => t_norm_product(a, b),
            DegreeNorm::Lukasiewicz => t_norm_lukasiewicz(a, b),
        };
        r as f64
    }

    /// Disjunction of two degrees (t-conorm).
    pub fn or(self, a: f64, b: f64) -> f64 {
        let (a, b) = (a as f32, b as f32);
        let r = match self {
            DegreeNorm::Godel => t_conorm_godel(a, b),
            DegreeNorm::Product => t_conorm_product(a, b),
            DegreeNorm::Lukasiewicz => t_conorm_lukasiewicz(a, b),
        };
        r as f64
    }

    /// Fuzzy complement (`1 − a`), shared with the modality layer.
    pub fn not(self, a: f64) -> f64 {
        fuzzy_not(a as f32) as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn norm_dispatch_matches_modality_operators() {
        assert!((DegreeNorm::Godel.and(0.3, 0.7) - 0.3).abs() < 1e-6);
        assert!((DegreeNorm::Godel.or(0.3, 0.7) - 0.7).abs() < 1e-6);
        assert!((DegreeNorm::Product.and(0.5, 0.5) - 0.25).abs() < 1e-6);
        assert!((DegreeNorm::Lukasiewicz.and(0.4, 0.4) - 0.0).abs() < 1e-6);
        assert!((DegreeNorm::Lukasiewicz.or(0.6, 0.6) - 1.0).abs() < 1e-6);
        assert!((DegreeNorm::Godel.not(0.2) - 0.8).abs() < 1e-6);
    }
}
