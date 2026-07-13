//! Abductive inference — Peirce's "inference to the best explanation".
//!
//! Given observed effects, find the hypotheses that would account for them. This library
//! (split per CLAUDE.md §10) covers the full abductive cycle:
//!   * **chain explanation** ([`abductive_explanation`]) — walk explanatory edges back to a root;
//!   * **minimal explanation** ([`minimal_explanation`]) — the parsimonious set of roots covering
//!     a set of observations (Peirce's economy of hypotheses);
//!   * **counter-abduction** ([`counter_abduction`]) — aggressively prune refuted hypotheses;
//!   * **probabilistic abduction** ([`probabilistic`]) — Bayesian scoring / ranking of hypotheses;
//!   * **ATMS** ([`atms`]) — assumption-based truth maintenance: minimal environments + nogoods.
//!
//! Zero-heap throughout (bounded chains, caller-supplied `out` buffers, bitset environments).

use crate::NQuin;

pub mod atms;
pub mod probabilistic;

pub use atms::{env_subset, holds_in, is_nogood, label_add, label_holds, Environment};
pub use probabilistic::{bayesian_posteriors, best_hypothesis, Hypothesis};

/// Max backward-chaining depth for abductive explanation (bounded, zero-heap).
pub const MAX_ABDUCTION_DEPTH: usize = 64;

/// Abductive inference — walk BACKWARD along explanatory edges (`hypothesis →explains→ effect`,
/// predicate == `explains`) from an observed effect to the root hypothesis that accounts for it.
/// Returns that root, or `None` if the observation has no explanation in the rule set. Zero-heap.
pub fn abductive_explanation(rules: &[NQuin], observation: u64, explains: u64) -> Option<u64> {
    let mut current = observation;
    for _ in 0..MAX_ABDUCTION_DEPTH {
        let mut next = None;
        for q in rules {
            if q.predicate == explains && q.object == current {
                next = Some(q.subject);
                break;
            }
        }
        match next {
            Some(h) => current = h,
            None => break,
        }
    }
    if current != observation {
        Some(current)
    } else {
        None // no explanatory hypothesis for the observation
    }
}

/// **Minimal explanation generation** (Peirce's parsimony): the DISTINCT root hypotheses that
/// together explain every observation in `observations`, written into `out`. A single hypothesis
/// accounting for several observations appears once (the smallest covering set under the chain
/// model). Observations with no explanation are skipped. Returns the count. Zero-heap.
pub fn minimal_explanation(
    rules: &[NQuin],
    observations: &[u64],
    explains: u64,
    out: &mut [u64],
) -> usize {
    let mut n = 0usize;
    for &obs in observations {
        if let Some(root) = abductive_explanation(rules, obs, explains) {
            if !out[..n].contains(&root) && n < out.len() {
                out[n] = root;
                n += 1;
            }
        }
    }
    n
}

/// **Counter-abduction:** from `candidates`, drop every hypothesis that has been `refuted` (ruled
/// out by an observation, or contradicted by an established fact), writing the survivors into
/// `out`. Returns the surviving count — aggressive pruning of contradictory hypotheses. Zero-heap.
pub fn counter_abduction(candidates: &[u64], refuted: &[u64], out: &mut [u64]) -> usize {
    let mut n = 0usize;
    for &c in candidates {
        if !refuted.contains(&c) && n < out.len() {
            out[n] = c;
            n += 1;
        }
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edge(hypothesis: u64, effect: u64) -> NQuin {
        let mut q = NQuin {
            subject: hypothesis,
            predicate: crate::q_hash("abduces:explains"),
            object: effect,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;
        q
    }

    #[test]
    fn finds_root_explanation() {
        let explains = crate::q_hash("abduces:explains");
        // disease(1) → symptom-fever(2) → observed-temp(3). Root hypothesis = 1.
        let rules = [edge(1, 2), edge(2, 3)];
        assert_eq!(
            abductive_explanation(&rules, 3, explains),
            Some(1),
            "root hypothesis explains the observation"
        );
        assert_eq!(abductive_explanation(&rules, 2, explains), Some(1));
        // An unexplained observation.
        assert_eq!(abductive_explanation(&rules, 99, explains), None);
    }

    #[test]
    fn minimal_explanation_collapses_shared_roots() {
        let explains = crate::q_hash("abduces:explains");
        // Root 1 → 2 → 3 and 1 → 4 (one disease explains two symptoms 3 and 4); root 5 → 6.
        let rules = [edge(1, 2), edge(2, 3), edge(1, 4), edge(5, 6)];
        let mut out = [0u64; 8];
        // Observations {3, 4, 6}: minimal explanation is {1, 5} (1 covers both 3 and 4).
        let n = minimal_explanation(&rules, &[3, 4, 6], explains, &mut out);
        assert_eq!(n, 2, "shared root collapses → parsimonious set");
        assert!(out[..n].contains(&1) && out[..n].contains(&5));
        // An unexplained observation contributes nothing.
        let m = minimal_explanation(&rules, &[3, 99], explains, &mut out);
        assert_eq!(m, 1);
        assert_eq!(out[0], 1);
    }

    #[test]
    fn counter_abduction_prunes_refuted() {
        let mut out = [0u64; 8];
        let n = counter_abduction(&[1, 2, 3, 4], &[2, 4], &mut out);
        assert_eq!(n, 2);
        assert_eq!(&out[..n], &[1, 3]);
        // Nothing refuted → all survive.
        assert_eq!(counter_abduction(&[1, 2], &[], &mut out), 2);
        // All refuted → none survive.
        assert_eq!(counter_abduction(&[1, 2], &[1, 2], &mut out), 0);
    }
}
