use crate::NQuin;

/// Max backward-chaining depth for abductive explanation (bounded, zero-heap).
pub const MAX_ABDUCTION_DEPTH: usize = 64;

/// Abductive inference — inference to the best explanation. Given an observed
/// effect, walk BACKWARD along explanatory edges (`hypothesis →explains→ effect`,
/// predicate == `explains`) to the root hypothesis that accounts for it. Returns
/// that root hypothesis, or `None` if the observation has no explanation in the
/// rule set. Zero-heap (bounded backward chain, no allocation).
///
/// This is the logical abduction the rights/forensic layer needs ("what hypothesis
/// would account for this observation?") — distinct from the Pearl-style
/// belief-update abduction inside `dialectical::counterfactual_query`.
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
        assert_eq!(abductive_explanation(&rules, 3, explains), Some(1), "root hypothesis explains the observation");
        assert_eq!(abductive_explanation(&rules, 2, explains), Some(1));
        // An unexplained observation.
        assert_eq!(abductive_explanation(&rules, 99, explains), None);
    }
}
