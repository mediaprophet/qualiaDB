//! Probabilistic abduction — Bayesian scoring and ranking of competing hypotheses.
//!
//! Among the hypotheses that *could* explain an observation, the best is the most probable a
//! posteriori: `P(h | obs) ∝ P(h) · P(obs | h)` (prior × likelihood), normalised over the
//! candidates. Zero-heap (caller-supplied `out`).

/// A candidate abductive hypothesis: its id, Bayesian `prior` `P(h)`, and the `likelihood` it
/// assigns to the observation `P(obs | h)`. Both in `[0, ∞)` (typically `[0,1]`).
#[derive(Debug, Clone, Copy)]
pub struct Hypothesis {
    pub id: u64,
    pub prior: f32,
    pub likelihood: f32,
}

/// Posteriors `P(h | obs) ∝ prior·likelihood`, normalised over `hyps`, written into `out`
/// (parallel to `hyps`). Returns the evidence `P(obs) = Σ prior·likelihood` (the normaliser);
/// if it is ~0 (no hypothesis explains the observation) `out` is filled with zeros and `0.0` is
/// returned. Refuses on a length mismatch by returning `0.0` without writing.
pub fn bayesian_posteriors(hyps: &[Hypothesis], out: &mut [f32]) -> f32 {
    if out.len() < hyps.len() {
        return 0.0;
    }
    let mut evidence = 0.0f32;
    for h in hyps {
        evidence += h.prior * h.likelihood;
    }
    if evidence.abs() < 1e-12 {
        for o in out.iter_mut().take(hyps.len()) {
            *o = 0.0;
        }
        return 0.0;
    }
    for (i, h) in hyps.iter().enumerate() {
        out[i] = (h.prior * h.likelihood) / evidence;
    }
    evidence
}

/// The **maximum-a-posteriori** hypothesis id — the one with the greatest `prior·likelihood`
/// (argmax of the posterior; normalisation is monotone so it needs no division). `None` if `hyps`
/// is empty or carries no probability mass.
pub fn best_hypothesis(hyps: &[Hypothesis]) -> Option<u64> {
    let mut best: Option<(u64, f32)> = None;
    for h in hyps {
        let score = h.prior * h.likelihood;
        if score > 0.0 && best.map(|(_, s)| score > s).unwrap_or(true) {
            best = Some((h.id, score));
        }
    }
    best.map(|(id, _)| id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    #[test]
    fn posteriors_normalise_and_rank() {
        // Two hypotheses: h1 prior .2 likelihood .9; h2 prior .8 likelihood .1.
        let hyps = [
            Hypothesis {
                id: 1,
                prior: 0.2,
                likelihood: 0.9,
            }, // joint .18
            Hypothesis {
                id: 2,
                prior: 0.8,
                likelihood: 0.1,
            }, // joint .08
        ];
        let mut out = [0.0f32; 2];
        let evidence = bayesian_posteriors(&hyps, &mut out);
        assert!(close(evidence, 0.26));
        assert!(close(out[0], 0.18 / 0.26));
        assert!(close(out[1], 0.08 / 0.26));
        assert!(close(out[0] + out[1], 1.0), "posteriors sum to 1");
        // The high-likelihood hypothesis wins despite a lower prior (explaining-away).
        assert_eq!(best_hypothesis(&hyps), Some(1));
    }

    #[test]
    fn no_mass_yields_none_and_zero_evidence() {
        let hyps = [Hypothesis {
            id: 1,
            prior: 0.0,
            likelihood: 0.9,
        }];
        let mut out = [9.0f32; 1];
        assert_eq!(bayesian_posteriors(&hyps, &mut out), 0.0);
        assert_eq!(out[0], 0.0, "zeroed when no mass");
        assert_eq!(best_hypothesis(&hyps), None);
        assert_eq!(best_hypothesis(&[]), None);
    }
}
