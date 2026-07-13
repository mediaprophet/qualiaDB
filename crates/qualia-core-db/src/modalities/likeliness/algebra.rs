//! The Kleene / De Morgan lattice on the likeliness scale. These are the logical
//! connectives of the calculus; the non-probabilistic character lives here (no excluded
//! middle, no contradiction collapse).

use super::Likeliness;

/// Negation — the order-reversing involution `Impossible ↔ Certain`, `Even` fixed.
/// Note `not(not(l)) == l`, but `or(l, not l)` need not be `Certain`.
pub fn not(l: Likeliness) -> Likeliness {
    Likeliness::from_level(-l.level())
}

/// Conjunction — the **meet** (weakest link): a claim resting on both `a` and `b` is no
/// more expected than its least-expected part.
pub fn and(a: Likeliness, b: Likeliness) -> Likeliness {
    a.min(b)
}

/// Disjunction — the **join** (best alternative): a claim reachable via `a` or `b` is at
/// least as expected as its most-expected route.
pub fn or(a: Likeliness, b: Likeliness) -> Likeliness {
    a.max(b)
}

/// Likeliness of a conjunction of premises = the weakest (meet over all). The vacuous
/// conjunction (no premises) is `Certain` (the top).
pub fn combine_premises(premises: &[Likeliness]) -> Likeliness {
    premises.iter().copied().fold(Likeliness::Certain, and)
}

/// Likeliness of a conclusion reachable by alternative routes = the strongest (join over
/// all). The vacuous disjunction (no routes) is `Impossible` (the bottom).
pub fn combine_routes(routes: &[Likeliness]) -> Likeliness {
    routes.iter().copied().fold(Likeliness::Impossible, or)
}

#[cfg(test)]
mod tests {
    use super::*;
    use Likeliness::*;

    #[test]
    fn negation_is_an_involution_reflecting_the_scale() {
        assert_eq!(not(Impossible), Certain);
        assert_eq!(not(Unlikely), Likely);
        assert_eq!(not(Even), Even);
        for l in [Impossible, Unlikely, Even, VeryLikely, Certain] {
            assert_eq!(not(not(l)), l);
        }
    }

    #[test]
    fn meet_and_join_are_min_and_max() {
        assert_eq!(and(Likely, Unlikely), Unlikely);
        assert_eq!(or(Likely, Unlikely), Likely);
        assert_eq!(and(Certain, Even), Even);
        assert_eq!(or(Impossible, Even), Even);
    }

    #[test]
    fn de_morgan_laws_hold() {
        for a in [Impossible, Unlikely, Even, Likely, Certain] {
            for b in [Impossible, Unlikely, Even, Likely, Certain] {
                assert_eq!(not(and(a, b)), or(not(a), not(b)));
                assert_eq!(not(or(a, b)), and(not(a), not(b)));
            }
        }
    }

    #[test]
    fn kleene_no_excluded_middle_no_contradiction() {
        // The defining non-probabilistic property: a merely-Likely proposition and its
        // negation neither exhaust certainty nor collapse to impossibility.
        assert_eq!(or(Likely, not(Likely)), Likely); // ≠ Certain
        assert_eq!(and(Likely, not(Likely)), Unlikely); // ≠ Impossible
                                                        // Only at the extremes do they behave classically.
        assert_eq!(or(Certain, not(Certain)), Certain);
        assert_eq!(and(Certain, not(Certain)), Impossible);
    }

    #[test]
    fn vacuous_folds_are_top_and_bottom() {
        assert_eq!(combine_premises(&[]), Certain);
        assert_eq!(combine_routes(&[]), Impossible);
        assert_eq!(combine_premises(&[Likely, Even, Certain]), Even);
        assert_eq!(combine_routes(&[Unlikely, Even, VeryUnlikely]), Even);
    }
}
