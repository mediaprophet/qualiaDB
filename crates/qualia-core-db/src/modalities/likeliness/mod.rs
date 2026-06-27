//! **Likeliness** — a qualitative, ordinal calculus of expectation (Vector Semantics
//! §4.2; Kornai's "naive" inference). The **third** uncertainty modality, built as its
//! own thing rather than folded into `defeasible`/`fuzzy`.
//!
//! > Timothy, 2026-06-27: "I'm undecided [whether it folds], which suggests it should
//! > be done as a new modality." Indecision about the fold is itself the signal that
//! > this is a distinct calculus — forcing it into an existing modality would distort
//! > both.
//!
//! ## Why it is genuinely distinct
//!
//! * Not [`crate::modalities::probabilistic`] (continuous `[0,1]`, Kolmogorov/Bayes):
//!   likeliness is **ordinal** and **non-additive** — `P(x)` and `P(¬x)` need not sum
//!   to a constant, and there is no normalisation. It expresses *how expected* a
//!   proposition is, qualitatively.
//! * Not [`crate::modalities::fuzzy`] (continuous `[0,1]` set-membership, t-norm):
//!   likeliness is graded **belief/expectation over propositions**, not degree of
//!   set-membership.
//! * Adjacent to [`crate::modalities::defeasible`] (defaults) but distinct: defeasible
//!   resolves a **crisp** conclusion by rule priority/defeaters; likeliness carries a
//!   **graded degree** on the conclusion and composes it.
//!
//! ## The calculus
//!
//! Likeliness lives on a symmetric 7-level ordinal scale centred on `Even`. The logical
//! operators ([`algebra`]) form a **Kleene / De Morgan algebra**: `not` is the
//! order-reversing involution, `and` is the meet (weakest link), `or` the join (best
//! alternative). Crucially `or(l, not l)` need *not* be `Certain` and `and(l, not l)`
//! need *not* be `Impossible` — **no excluded middle, no contradiction collapse** — which
//! is exactly the non-probabilistic, defeasible character. On top sit the naive
//! inference rules ([`inference`]): weakest-link modus ponens, chain attenuation, and
//! defeasible revision. Kernel-class `ElementwiseMap` (trivial CPU; no GPU path, §13).

pub mod algebra;
pub mod inference;

pub use algebra::{and, combine_premises, combine_routes, not, or};
pub use inference::{attenuate, infer_chain, modus_ponens, rebut, revise};

/// An ordinal degree of expectation. Stored so `self as i8` is the signed level in
/// `[-3, +3]`, with `Even = 0` the point of no information.
#[repr(i8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Likeliness {
    Impossible = -3,
    VeryUnlikely = -2,
    Unlikely = -1,
    Even = 0,
    Likely = 1,
    VeryLikely = 2,
    Certain = 3,
}

impl Likeliness {
    pub const MIN_LEVEL: i8 = -3;
    pub const MAX_LEVEL: i8 = 3;

    /// The signed ordinal level in `[-3, +3]`.
    pub const fn level(self) -> i8 {
        self as i8
    }

    /// The likeliness for a level, saturating to the scale bounds.
    pub fn from_level(level: i8) -> Self {
        match level.clamp(Self::MIN_LEVEL, Self::MAX_LEVEL) {
            -3 => Self::Impossible,
            -2 => Self::VeryUnlikely,
            -1 => Self::Unlikely,
            0 => Self::Even,
            1 => Self::Likely,
            2 => Self::VeryLikely,
            _ => Self::Certain,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_round_trips_and_orders() {
        for l in Likeliness::MIN_LEVEL..=Likeliness::MAX_LEVEL {
            assert_eq!(Likeliness::from_level(l).level(), l);
        }
        // Ordinal ordering ascends with expectation.
        assert!(Likeliness::Impossible < Likeliness::Even);
        assert!(Likeliness::Even < Likeliness::Certain);
        assert!(Likeliness::Likely < Likeliness::VeryLikely);
    }

    #[test]
    fn from_level_saturates() {
        assert_eq!(Likeliness::from_level(99), Likeliness::Certain);
        assert_eq!(Likeliness::from_level(-99), Likeliness::Impossible);
    }
}
