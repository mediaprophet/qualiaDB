//! Naive inference and defeasible revision over likeliness — the "specific update rule"
//! half of the calculus. Built on the [`super::algebra`] lattice so the laws are
//! inherited (reuse, not a parallel set of rules).

use super::algebra::{and, not, or};
use super::Likeliness;

/// Lower a likeliness by `steps` ordinal levels — chain attenuation. Saturating at
/// `Impossible`.
pub fn attenuate(l: Likeliness, steps: u8) -> Likeliness {
    Likeliness::from_level(l.level().saturating_sub(steps as i8))
}

/// **Naive modus ponens.** From a premise `p` (how expected it is to hold) and a rule's
/// reliability `r` (how expected it is that `p ⇒ q`), the conclusion `q` is `and(p, r)`
/// — inference never produces a conclusion stronger than its weakest input.
pub fn modus_ponens(premise: Likeliness, rule_reliability: Likeliness) -> Likeliness {
    and(premise, rule_reliability)
}

/// Inference along a chain of default rules from a premise: the weakest link, then
/// **attenuated by the number of inferential steps beyond the first**. Longer defeasible
/// chains weaken — the qualitative analogue of multiplying probabilities along a path.
/// An empty rule list returns the premise unchanged.
pub fn infer_chain(premise: Likeliness, rule_reliabilities: &[Likeliness]) -> Likeliness {
    if rule_reliabilities.is_empty() {
        return premise;
    }
    let weakest = rule_reliabilities.iter().copied().fold(premise, and);
    attenuate(weakest, (rule_reliabilities.len() as u8).saturating_sub(1))
}

/// **Rebuttal.** A counter-argument of strength `counter` (how expected it is that the
/// conclusion is *false*) caps the conclusion at `not(counter)`. A strong rebuttal
/// defeats: a `Certain` conclusion rebutted by a `Likely` counter falls to `Unlikely`.
pub fn rebut(conclusion: Likeliness, counter: Likeliness) -> Likeliness {
    and(conclusion, not(counter))
}

/// **Defeasible revision.** Fold a new supporting route and a rebuttal into a prior:
/// `and(or(prior, support), not(against))`. The support is an alternative route
/// (best-of), and the rebuttal caps the result. Symmetric in the sense that a strong
/// `against` overrides any amount of `support`.
pub fn revise(prior: Likeliness, support: Likeliness, against: Likeliness) -> Likeliness {
    and(or(prior, support), not(against))
}

#[cfg(test)]
mod tests {
    use super::*;
    use Likeliness::*;

    #[test]
    fn modus_ponens_is_weakest_link() {
        // "p is Likely" + "p⇒q is VeryLikely" → q is Likely.
        assert_eq!(modus_ponens(Likely, VeryLikely), Likely);
        // A certain rule cannot rescue an unlikely premise.
        assert_eq!(modus_ponens(Unlikely, Certain), Unlikely);
    }

    #[test]
    fn chains_attenuate_with_length() {
        // One step: just the weakest link.
        assert_eq!(infer_chain(Certain, &[Likely]), Likely);
        // Three Certain rules from a Certain premise → Certain weakest, attenuated by 2.
        assert_eq!(infer_chain(Certain, &[Certain, Certain, Certain]), Likely);
        // Empty chain is identity.
        assert_eq!(infer_chain(VeryLikely, &[]), VeryLikely);
    }

    #[test]
    fn rebuttal_defeats_in_proportion_to_strength() {
        // A Certain conclusion, rebutted by a Likely counter → Unlikely.
        assert_eq!(rebut(Certain, Likely), Unlikely);
        // A weak counter barely dents a strong conclusion.
        assert_eq!(rebut(Certain, Unlikely), Likely);
        // No counter (Impossible that it's false) leaves it untouched.
        assert_eq!(rebut(Likely, Impossible), Likely);
    }

    #[test]
    fn revision_combines_support_and_rebuttal() {
        // Supporting route lifts a weak prior; no rebuttal.
        assert_eq!(revise(Unlikely, Likely, Impossible), Likely);
        // A strong rebuttal overrides strong support.
        assert_eq!(revise(Likely, VeryLikely, Likely), Unlikely);
        // Neither support nor rebuttal → prior unchanged.
        assert_eq!(revise(Even, Impossible, Impossible), Even);
    }

    #[test]
    fn attenuate_saturates() {
        assert_eq!(attenuate(Unlikely, 10), Impossible);
        assert_eq!(attenuate(Likely, 1), Even);
    }
}
