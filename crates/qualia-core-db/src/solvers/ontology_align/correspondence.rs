//! Ontology-alignment result types + the human-ratification guardrail.
//!
//! **Hard invariant (identity / out-of-band remainder):** this engine *proposes*
//! correspondences with a degree; it can emit `CloseMatch` routed to
//! `RequiresHumanReview`, and is **structurally forbidden** from asserting
//! `ExactMatch`. `exactMatch` requires signed human ratification — the machine
//! proposes, the human disposes. The type system enforces it: there is no
//! constructor here that yields an asserted exact match.

/// The status a *machine-proposed* correspondence may carry. Note the deliberate
/// absence of an "asserted ExactMatch" — that state is reachable only through the
/// human-ratification layer, never from an alignment solver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProposedStatus {
    /// A graded `skos:closeMatch` proposal awaiting human ratification.
    CloseMatch,
}

/// One proposed correspondence between a source entity and a target entity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Correspondence {
    pub source: usize,
    pub target: usize,
    /// Match degree in `[0,1]` (the fuzzy `closeMatch` strength).
    pub degree: f64,
    pub status: ProposedStatus,
    /// Always `true`: a proposed correspondence MUST be human-reviewed before it can
    /// become an `exactMatch`. There is no path here that sets this `false`.
    pub requires_human_review: bool,
}

impl Correspondence {
    /// The only constructor — a close-match *proposal*. By construction it is never
    /// an asserted exact match and always requires human review.
    pub fn propose(source: usize, target: usize, degree: f64) -> Self {
        Self {
            source,
            target,
            degree: degree.clamp(0.0, 1.0),
            status: ProposedStatus::CloseMatch,
            requires_human_review: true,
        }
    }
}

/// A full alignment: the proposed correspondences plus the total quality earned.
#[derive(Debug, Clone, PartialEq)]
pub struct Alignment {
    pub correspondences: Vec<Correspondence>,
    pub quality: f64,
}

impl Alignment {
    /// Every correspondence is a review-required close-match proposal (the invariant).
    pub fn all_require_review(&self) -> bool {
        self.correspondences.iter().all(|c| c.requires_human_review && c.status == ProposedStatus::CloseMatch)
    }
}
