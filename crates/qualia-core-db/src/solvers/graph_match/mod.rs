//! Fuzzy RDF graph matching (Ma, Li & Ma) — degree-aware similarity and the
//! approximate subgraph match that is the identity-reconciliation primitive.
//!
//! - [`fuzzy_similarity`] — fuzzy Jaccard / Dice over degree-bearing triples.
//! - [`approximate`] — ranked, tolerant subgraph matching → a `closeMatch` *proposal*
//!   with a degree (never an asserted identity).

pub mod approximate;
pub mod fuzzy_similarity;

pub use approximate::{approximate_match, MatchResult};
pub use fuzzy_similarity::{fuzzy_dice, fuzzy_jaccard, FuzzyTriple};
