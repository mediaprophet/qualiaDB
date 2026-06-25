use crate::NQuin;

// Marks a Quin as consumed by setting metadata bit 59 (CONSUMED_BIT).
// Canonical bit position lives in the FrameLayout ABI (single source of truth).
pub use crate::frame_layout::CONSUMED_BIT;

pub fn consume_quin(q: &mut NQuin) {
    q.metadata |= CONSUMED_BIT;
}

pub fn is_consumed(q: &NQuin) -> bool {
    (q.metadata & CONSUMED_BIT) != 0
}

// ─── Girard's linear-logic connectives ──────────────────────────────────────────────
//
// Linear logic treats propositions as RESOURCES: each is used exactly once unless explicitly
// marked reusable with the `!` exponential. The connectives split into multiplicatives
// (⊗ tensor, ⅋ par), additives (⊕ plus, & with), their units, and the exponentials (! ?).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Connective {
    /// A positive literal.
    Atom,
    /// A literal's linear negation `a⊥`.
    AtomDual,
    /// `A ⊗ B` — multiplicative conjunction ("both, together").
    Tensor,
    /// `A ⅋ B` — multiplicative disjunction.
    Par,
    /// `A ⊕ B` — additive disjunction (internal choice).
    Plus,
    /// `A & B` — additive conjunction (external choice).
    With,
    /// `1` — unit of `⊗`.
    One,
    /// `⊥` — unit of `⅋`.
    Bottom,
    /// `0` — unit of `⊕`.
    Zero,
    /// `⊤` — unit of `&`.
    Top,
    /// `!A` — exponential "of course" (reusable: weakening + contraction apply).
    OfCourse,
    /// `?A` — exponential "why not" (dual of `!`).
    WhyNot,
}

impl Connective {
    /// Linear negation `(·)⊥` — the involutive De Morgan dual (`A⊥⊥ = A`).
    pub fn dual(self) -> Connective {
        use Connective::*;
        match self {
            Atom => AtomDual,
            AtomDual => Atom,
            Tensor => Par,
            Par => Tensor,
            Plus => With,
            With => Plus,
            One => Bottom,
            Bottom => One,
            Zero => Top,
            Top => Zero,
            OfCourse => WhyNot,
            WhyNot => OfCourse,
        }
    }

    /// Multiplicative connectives/units (`⊗ ⅋ 1 ⊥`).
    pub fn is_multiplicative(self) -> bool {
        use Connective::*;
        matches!(self, Tensor | Par | One | Bottom)
    }

    /// Additive connectives/units (`⊕ & 0 ⊤`).
    pub fn is_additive(self) -> bool {
        use Connective::*;
        matches!(self, Plus | With | Zero | Top)
    }

    /// Exponential connectives (`! ?`).
    pub fn is_exponential(self) -> bool {
        matches!(self, Connective::OfCourse | Connective::WhyNot)
    }

    /// A resource under `!` ("of course") is **reusable** — structural weakening and contraction
    /// are licensed. Everything else is linear (consume-once).
    pub fn is_reusable(self) -> bool {
        matches!(self, Connective::OfCourse)
    }
}

/// Whether a resource quin `q` may be consumed to satisfy a demand: a reusable (`!`-marked)
/// resource always can; a linear resource only if not already consumed.
pub fn can_consume(q: &NQuin, reusable: bool) -> bool {
    reusable || !is_consumed(q)
}

/// `A ⊗ B` consumption: a tensor demand needs **both** operands available *together*. Consumes
/// each linear operand (leaves reusable ones); returns `false` without mutating if either is
/// already exhausted.
pub fn tensor_consume(a: &mut NQuin, a_reusable: bool, b: &mut NQuin, b_reusable: bool) -> bool {
    if !can_consume(a, a_reusable) || !can_consume(b, b_reusable) {
        return false;
    }
    if !a_reusable {
        consume_quin(a);
    }
    if !b_reusable {
        consume_quin(b);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consume_quin() {
        let mut q = NQuin {
            subject: 0,
            predicate: 0,
            object: 0,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        assert!(!is_consumed(&q));
        consume_quin(&mut q);
        assert!(is_consumed(&q));
    }

    #[test]
    fn linear_negation_is_involutive_and_dualises_connectives() {
        use Connective::*;
        for c in [Atom, AtomDual, Tensor, Par, Plus, With, One, Bottom, Zero, Top, OfCourse, WhyNot] {
            assert_eq!(c.dual().dual(), c, "(A⊥)⊥ = A for {:?}", c);
            assert_ne!(c.dual(), c, "no connective is its own dual: {:?}", c);
        }
        // The characteristic De Morgan dualities.
        assert_eq!(Tensor.dual(), Par);
        assert_eq!(Plus.dual(), With);
        assert_eq!(One.dual(), Bottom);
        assert_eq!(Zero.dual(), Top);
        assert_eq!(OfCourse.dual(), WhyNot);
    }

    #[test]
    fn connective_classification_and_reuse() {
        use Connective::*;
        assert!(Tensor.is_multiplicative() && !Tensor.is_additive());
        assert!(With.is_additive() && !With.is_multiplicative());
        assert!(OfCourse.is_exponential() && WhyNot.is_exponential());
        // Only `!A` is reusable; linear atoms are consume-once.
        assert!(OfCourse.is_reusable());
        assert!(!Atom.is_reusable());
    }

    #[test]
    fn tensor_consumes_both_and_respects_reuse() {
        let mk = || NQuin { subject: 1, predicate: 2, object: 3, context: 0, metadata: 0, parity: 0 };
        // Two linear resources: tensor consumes both; a second demand fails.
        let mut a = mk();
        let mut b = mk();
        assert!(tensor_consume(&mut a, false, &mut b, false));
        assert!(is_consumed(&a) && is_consumed(&b));
        assert!(!tensor_consume(&mut a, false, &mut b, false), "linear resources are exhausted");

        // A reusable (!-marked) resource is never exhausted.
        let mut r = mk();
        let mut s = mk();
        assert!(tensor_consume(&mut r, true, &mut s, false));
        assert!(!is_consumed(&r), "reusable resource is not consumed");
        assert!(is_consumed(&s));
        assert!(tensor_consume(&mut r, true, &mut s, true), "reusable can satisfy again");
    }
}
