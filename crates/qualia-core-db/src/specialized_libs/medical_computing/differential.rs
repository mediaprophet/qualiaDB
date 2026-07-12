//! Transparent Bayesian differential engine — a ranked *epistemic proposal*, never a diagnosis.
//!
//! HONESTY (CLAUDE.md §15 + repo stance "investigative proposals, not verdicts"):
//! this computes a normalized naive-Bayes posterior over conditions given observed
//! findings. The MATH is what this module implements and tests. The KNOWLEDGE BASE
//! (priors + per-finding likelihoods) is **caller-supplied and non-authoritative** —
//! its clinical validity is the caller's responsibility, stated plainly on every
//! result via [`DIFFERENTIAL_EPISTEMIC_STATUS`]. No clinical fact is baked in.

use super::MedicalError;
use std::collections::HashMap;

/// Honest epistemic label stamped on every [`DifferentialProposal`].
pub const DIFFERENTIAL_EPISTEMIC_STATUS: &str = "Epistemic proposal — a ranked \
differential computed by transparent naive-Bayes over a CALLER-SUPPLIED, \
non-authoritative knowledge base. NOT a diagnosis. The clinical validity of the \
knowledge base is the caller's responsibility.";

const METHOD: &str = "normalized naive-Bayes posterior  P(condition | findings) \u{221d} prior \u{00b7} \u{220f} P(finding | condition)";

/// One condition's probabilistic model within a caller-supplied knowledge base.
#[derive(Debug, Clone)]
pub struct ConditionModel {
    pub condition_id: String,
    /// Prior weight P(condition). Must be finite and > 0. Need not sum to 1 across
    /// conditions — the posterior is normalized regardless.
    pub prior: f64,
    /// finding_id → P(finding present | condition), each in [0,1].
    pub likelihoods: HashMap<String, f64>,
}

/// A caller-supplied, **non-authoritative** knowledge base for the Bayes engine.
///
/// # Illustrative example (NOT authoritative)
/// Any example knowledge base constructed for tests or demos is explicitly
/// illustrative. This engine coins no medical facts; callers own the clinical
/// content and its validity.
#[derive(Debug, Clone)]
pub struct DiagnosticKnowledgeBase {
    pub conditions: Vec<ConditionModel>,
    /// Likelihood applied for an observed finding that a condition does not list.
    /// The caller's modelling choice (0.5 is uninformative); documented, not authoritative.
    pub unlisted_finding_likelihood: f64,
}

/// Posterior for one condition.
#[derive(Debug, Clone)]
pub struct ConditionPosterior {
    pub condition_id: String,
    pub prior: f64,
    pub posterior: f64,
}

/// Ranked differential proposal. `ranked` is sorted descending by posterior; ties
/// are broken by `condition_id` ascending for a deterministic order.
#[derive(Debug, Clone)]
pub struct DifferentialProposal {
    /// Honest label — this is a proposal over a caller-supplied KB, never a diagnosis.
    pub epistemic_status: &'static str,
    pub method: &'static str,
    pub observed_findings: Vec<String>,
    pub ranked: Vec<ConditionPosterior>,
}

/// Compute the normalized posterior differential over `kb` given the present
/// `observed` findings.
///
/// `observed` is the list of finding ids observed to be **present**; the posterior
/// is proportional to `prior · Π P(finding | condition)` over those findings.
///
/// Fails closed ([`MedicalError`]) on an empty KB, a non-finite/non-positive prior,
/// a likelihood outside [0,1], or when every condition's unnormalized posterior is
/// zero (findings incompatible with the whole KB) — never returns a fabricated result.
pub fn analyze_differential(
    observed: &[String],
    kb: &DiagnosticKnowledgeBase,
) -> Result<DifferentialProposal, MedicalError> {
    if kb.conditions.is_empty() {
        return Err(MedicalError::InsufficientData(
            "differential: knowledge base has no conditions".to_string(),
        ));
    }
    if !(kb.unlisted_finding_likelihood.is_finite()
        && (0.0..=1.0).contains(&kb.unlisted_finding_likelihood))
    {
        return Err(MedicalError::ValidationError(
            "differential: unlisted_finding_likelihood must be in [0,1]".to_string(),
        ));
    }

    // Validate each condition and compute its unnormalized posterior.
    let mut unnorm: Vec<f64> = Vec::with_capacity(kb.conditions.len());
    for cond in &kb.conditions {
        if !(cond.prior.is_finite() && cond.prior > 0.0) {
            return Err(MedicalError::ValidationError(format!(
                "differential: condition '{}' has a non-finite or non-positive prior",
                cond.condition_id
            )));
        }
        let mut p = cond.prior;
        for f in observed {
            let l = match cond.likelihoods.get(f) {
                Some(&v) => v,
                None => kb.unlisted_finding_likelihood,
            };
            if !(l.is_finite() && (0.0..=1.0).contains(&l)) {
                return Err(MedicalError::ValidationError(format!(
                    "differential: condition '{}' likelihood for finding '{}' must be in [0,1]",
                    cond.condition_id, f
                )));
            }
            p *= l;
        }
        unnorm.push(p);
    }

    let sum: f64 = unnorm.iter().sum();
    if sum <= 0.0 {
        return Err(MedicalError::InsufficientData(
            "differential: observed findings are incompatible with every condition \
             (all posteriors zero); cannot rank"
                .to_string(),
        ));
    }

    let mut ranked: Vec<ConditionPosterior> = kb
        .conditions
        .iter()
        .zip(unnorm.iter())
        .map(|(cond, &u)| ConditionPosterior {
            condition_id: cond.condition_id.clone(),
            prior: cond.prior,
            posterior: u / sum,
        })
        .collect();

    ranked.sort_by(|a, b| {
        b.posterior
            .partial_cmp(&a.posterior)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.condition_id.cmp(&b.condition_id))
    });

    Ok(DifferentialProposal {
        epistemic_status: DIFFERENTIAL_EPISTEMIC_STATUS,
        method: METHOD,
        observed_findings: observed.to_vec(),
        ranked,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // ILLUSTRATIVE, NON-AUTHORITATIVE knowledge base used purely to exercise the math.
    fn illustrative_kb() -> DiagnosticKnowledgeBase {
        let mut flu = HashMap::new();
        flu.insert("fever".to_string(), 0.9);
        flu.insert("cough".to_string(), 0.8);
        let mut cold = HashMap::new();
        cold.insert("fever".to_string(), 0.2);
        cold.insert("cough".to_string(), 0.6);
        DiagnosticKnowledgeBase {
            conditions: vec![
                ConditionModel {
                    condition_id: "influenza_like".to_string(),
                    prior: 0.6,
                    likelihoods: flu,
                },
                ConditionModel {
                    condition_id: "common_cold".to_string(),
                    prior: 0.4,
                    likelihoods: cold,
                },
            ],
            unlisted_finding_likelihood: 0.5,
        }
    }

    #[test]
    fn empty_kb_fails_closed() {
        let kb = DiagnosticKnowledgeBase {
            conditions: vec![],
            unlisted_finding_likelihood: 0.5,
        };
        assert!(analyze_differential(&["fever".to_string()], &kb).is_err());
    }

    #[test]
    fn hand_computed_posteriors() {
        // unnorm(flu)=0.6*0.9*0.8=0.432 ; unnorm(cold)=0.4*0.2*0.6=0.048 ; sum=0.48
        // post(flu)=0.9 ; post(cold)=0.1
        let kb = illustrative_kb();
        let obs = vec!["fever".to_string(), "cough".to_string()];
        let p = analyze_differential(&obs, &kb).unwrap();
        assert_eq!(p.ranked[0].condition_id, "influenza_like");
        assert!((p.ranked[0].posterior - 0.9).abs() < 1e-9);
        assert!((p.ranked[1].posterior - 0.1).abs() < 1e-9);
    }
}
