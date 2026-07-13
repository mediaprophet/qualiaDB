//! **KG↔LLM grounding evaluation** — does a model's asserted claim actually trace to
//! the graph facts it cited?
//!
//! The output gate (`inference/orchestrator.rs::orchestrate_inference`) already refuses
//! any LLM output that carries *no* provenance citation. That is a presence check. This
//! library deepens it into a **support** check: given the structured claim the model
//! emitted (`AgentOutput::semantic_quin`) and the **resolved** cited facts, it measures
//! the degree to which the claim is supported by those facts and returns a graded
//! verdict.
//!
//! ## Why this is knowledge-level, not wisdom-level
//!
//! Grounding only asks "is this asserted *knowledge* traceable to attested facts?" — a
//! Data→Knowledge check the machine is allowed to make. It never authors the final
//! "ought"; weakly-grounded claims are routed to **human review**, not silently
//! accepted or rewritten. This mirrors the engine-wide identity discipline: a partial
//! match is a *proposal requiring ratification* (`closeMatch`), never an asserted fact.
//!
//! ## Fail-closed
//!
//! No citations, or a claim that traces to nothing in the evidence, yields
//! [`GroundingVerdict::Ungrounded`] — the gate blocks. A score is only ever produced
//! from real component arithmetic over the cited quins; nothing is fabricated.
//!
//! Kernel-class `Reduction` over the evidence set; CPU path always present (§13).

pub mod claim_support;
pub mod evaluate;

pub use claim_support::{component_support, entity_grounding, report, GroundingReport};
pub use evaluate::{
    evaluate_grounding, evaluate_output_grounding, grounding_verdict, resolve_citations,
};

use crate::NQuin;

/// Thresholds partitioning the grounding score into the three verdicts. `deny < permit`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GroundingThresholds {
    /// Below this the claim is treated as ungrounded → block.
    pub deny: f64,
    /// At/above this the claim is well-grounded → allow.
    pub permit: f64,
}

impl Default for GroundingThresholds {
    fn default() -> Self {
        // A 2-of-3 role match (≈0.67) clears `permit`; a single role or both endpoints
        // merely *appearing* in evidence (≈0.33–0.5) lands in the human-review band; a
        // lone endpoint (≤0.25) is ungrounded.
        Self {
            deny: 0.3,
            permit: 0.6,
        }
    }
}

/// The graded outcome of grounding a claim in cited evidence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GroundingVerdict {
    /// Claim is well-supported by the cited facts — safe to commit.
    Grounded { score: f64 },
    /// Partial support — must be routed to human review (do not auto-commit).
    Weak { score: f64 },
    /// Claim does not trace to the cited facts — block.
    Ungrounded { score: f64 },
}

impl GroundingVerdict {
    pub fn score(&self) -> f64 {
        match *self {
            GroundingVerdict::Grounded { score }
            | GroundingVerdict::Weak { score }
            | GroundingVerdict::Ungrounded { score } => score,
        }
    }

    /// True only for [`GroundingVerdict::Grounded`].
    pub fn is_grounded(&self) -> bool {
        matches!(self, GroundingVerdict::Grounded { .. })
    }
}

/// Resolves a provenance citation hash to its full fact quin. Implemented by whatever
/// holds the graph (the daemon's quin store, a temporal-graph snapshot, a test stub).
/// Grounding needs the full triple of each cited fact — a bare hash is not enough — so
/// the gate activates only where facts are resolvable, and never false-denies a
/// grounded claim for lack of a resolver.
pub trait GroundingResolver {
    /// Return the fact quin for a provenance citation hash, or `None` if unknown.
    fn resolve(&self, citation_hash: u64) -> Option<NQuin>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verdict_accessors() {
        assert!(GroundingVerdict::Grounded { score: 0.9 }.is_grounded());
        assert!(!GroundingVerdict::Weak { score: 0.4 }.is_grounded());
        assert!((GroundingVerdict::Ungrounded { score: 0.1 }.score() - 0.1).abs() < 1e-12);
    }

    #[test]
    fn default_thresholds_ordered() {
        let t = GroundingThresholds::default();
        assert!(t.deny < t.permit);
    }
}
