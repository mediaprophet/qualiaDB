//! Bounded association estimates for generic entities and ontology relations.
//!
//! Existing identity-fabric enumeration measures anchor coverage, and epistemic
//! certainty bytes express assertion strength; neither is a calibrated
//! probability. This path uses an explicit host model and preserves estimates
//! as inferred beliefs. No address, NFC identifier, or posterior authenticates
//! an entity or proves ownership, custody, proximity, or forensic integrity.

use core::num::NonZeroU64;

use super::uncertainty::EvidenceReference;
use crate::epistemic::EpistemicState;

#[path = "association_contract.rs"]
mod contract;
pub use contract::*;

pub const MAX_ASSOCIATION_HYPOTHESES: usize = 32;
pub const MAX_ASSOCIATION_EVIDENCE: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnassessedReason {
    NoEvidence,
    UnknownDependence,
    MissingDependenceGroup,
    ModelOutsideCoverage,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AssociationResult {
    Unassessed(UnassessedReason),
    /// Binary P(relation holds | supplied evidence, host model and context).
    /// Different hypotheses are not assumed exhaustive or mutually exclusive.
    Estimated {
        probability: f64,
    },
}

/// Borrows exact evidence for subsequent rule/audit use; storage and signed
/// receipts are separate operations. Policy is rechecked before publication.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AssociationEstimate<'a, 'data> {
    hypothesis: AssociationHypothesis,
    request: AssociationRequest,
    model: AssociationModelReference,
    dependence: DependenceAssumption,
    evidence: &'a [AssociationEvidence<'data>],
    evaluation_decision: EvidenceReference,
    policy_decision: EvidenceReference,
    result: AssociationResult,
}

impl<'a, 'data> AssociationEstimate<'a, 'data> {
    pub const fn hypothesis(self) -> AssociationHypothesis {
        self.hypothesis
    }
    pub const fn request(self) -> AssociationRequest {
        self.request
    }
    pub const fn model(self) -> AssociationModelReference {
        self.model
    }
    pub const fn dependence(self) -> DependenceAssumption {
        self.dependence
    }
    pub const fn evidence(self) -> &'a [AssociationEvidence<'data>] {
        self.evidence
    }
    pub const fn evaluation_decision(self) -> EvidenceReference {
        self.evaluation_decision
    }
    pub const fn policy_decision(self) -> EvidenceReference {
        self.policy_decision
    }
    pub const fn result(self) -> AssociationResult {
        self.result
    }
    pub const fn epistemic_state(self) -> EpistemicState {
        match self.result {
            AssociationResult::Unassessed(_) => EpistemicState::Unknown,
            AssociationResult::Estimated { .. } => EpistemicState::InferredBelief,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssociationError {
    CapacityExceeded,
    OutputTooSmall,
    DuplicateHypothesis,
    DuplicateEvidence,
    InvalidEvidence,
    PermissionUnknown,
    PermissionDenied,
    InvalidModelValue,
    ImpossibleEvidence,
    NumericalRange,
}

/// Fixed stack scratch and deterministic group-ID order. Every failure leaves
/// the entire caller output unchanged; success writes only the returned prefix.
/// The host authority must serialize revocation against Evaluate/Publish if
/// its policy requires stronger atomicity than these two checks can provide.
pub fn estimate_associations<'a, 'data>(
    request: AssociationRequest,
    hypotheses: &[AssociationHypothesis],
    evidence: &'a [AssociationEvidence<'data>],
    dependence: DependenceAssumption,
    model: &impl CalibratedAssociationModel,
    authority: &mut impl AssociationAuthority,
    output: &mut [Option<AssociationEstimate<'a, 'data>>],
) -> Result<usize, AssociationError> {
    if hypotheses.len() > MAX_ASSOCIATION_HYPOTHESES || evidence.len() > MAX_ASSOCIATION_EVIDENCE {
        return Err(AssociationError::CapacityExceeded);
    }
    if output.len() < hypotheses.len() {
        return Err(AssociationError::OutputTooSmall);
    }
    validate_inputs(hypotheses, evidence)?;
    let reference = model.reference();
    let decision = require_permission(authority.authorize(
        AssociationPhase::Evaluate,
        &request,
        hypotheses,
        evidence,
        reference,
        dependence,
    ))?;
    let mut planned = [None; MAX_ASSOCIATION_HYPOTHESES];
    let mut proposed_results =
        [AssociationResult::Unassessed(UnassessedReason::NoEvidence); MAX_ASSOCIATION_HYPOTHESES];
    for (index, hypothesis) in hypotheses.iter().enumerate() {
        let result = estimate_one(&request, hypothesis, evidence, dependence, model)?;
        proposed_results[index] = result;
        planned[index] = Some(AssociationEstimate {
            hypothesis: *hypothesis,
            request,
            model: reference,
            dependence,
            evidence,
            evaluation_decision: decision,
            policy_decision: decision,
            result,
        });
    }
    let final_decision = require_permission(authority.authorize(
        AssociationPhase::Publish {
            results: &proposed_results[..hypotheses.len()],
            evaluation_decision: decision,
        },
        &request,
        hypotheses,
        evidence,
        reference,
        dependence,
    ))?;
    // Publish exactly the inspected results; only attach the returned receipt.
    for estimate in planned[..hypotheses.len()].iter_mut().flatten() {
        estimate.policy_decision = final_decision;
    }
    output[..hypotheses.len()].copy_from_slice(&planned[..hypotheses.len()]);
    Ok(hypotheses.len())
}

fn require_permission(
    permission: AssociationPermission,
) -> Result<EvidenceReference, AssociationError> {
    match permission {
        AssociationPermission::Approved { decision } => Ok(decision),
        AssociationPermission::Unknown => Err(AssociationError::PermissionUnknown),
        AssociationPermission::Denied => Err(AssociationError::PermissionDenied),
    }
}

fn validate_inputs(
    hypotheses: &[AssociationHypothesis],
    evidence: &[AssociationEvidence<'_>],
) -> Result<(), AssociationError> {
    for (i, hypothesis) in hypotheses.iter().enumerate() {
        if hypotheses[..i]
            .iter()
            .any(|other| other.id == hypothesis.id)
        {
            return Err(AssociationError::DuplicateHypothesis);
        }
    }
    for (i, item) in evidence.iter().enumerate() {
        if evidence[..i]
            .iter()
            .any(|other| other.observation_id == item.observation_id)
        {
            return Err(AssociationError::DuplicateEvidence);
        }
        if matches!(item.value, AssociationValue::Scalar { value, .. } if !value.is_finite()) {
            return Err(AssociationError::InvalidEvidence);
        }
        if let Some(uncertainty) = item.uncertainty {
            if item.field != Some(uncertainty.field()) {
                return Err(AssociationError::InvalidEvidence);
            }
        }
    }
    Ok(())
}

fn estimate_one(
    request: &AssociationRequest,
    hypothesis: &AssociationHypothesis,
    evidence: &[AssociationEvidence<'_>],
    dependence: DependenceAssumption,
    model: &impl CalibratedAssociationModel,
) -> Result<AssociationResult, AssociationError> {
    use UnassessedReason::*;
    let unassessed = |reason| Ok(AssociationResult::Unassessed(reason));
    if evidence.is_empty() {
        return unassessed(NoEvidence);
    }
    if dependence == DependenceAssumption::Unknown {
        return unassessed(UnknownDependence);
    }
    if matches!(dependence, DependenceAssumption::IndependentGroups { .. })
        && evidence.iter().any(|item| item.dependence_group.is_none())
    {
        return unassessed(MissingDependenceGroup);
    }
    let Some(prior) = model.prior_probability(request, hypothesis) else {
        return unassessed(ModelOutsideCoverage);
    };
    if !prior.is_finite() || !(0.0..=1.0).contains(&prior) {
        return Err(AssociationError::InvalidModelValue);
    }
    let mut log_yes = prior.ln();
    let mut log_no = (-prior).ln_1p();
    if dependence == DependenceAssumption::Joint {
        let group = AssociationEvidenceGroup {
            evidence,
            group: None,
        };
        let Some(likelihoods) = model.joint_log_likelihoods(request, hypothesis, group) else {
            return unassessed(ModelOutsideCoverage);
        };
        add_likelihoods(&mut log_yes, &mut log_no, likelihoods)?;
    } else {
        let mut previous: Option<NonZeroU64> = None;
        for _ in 0..evidence.len() {
            let next = evidence
                .iter()
                .filter_map(|item| item.dependence_group)
                .filter(|id| previous.is_none_or(|previous| *id > previous))
                .min();
            let Some(id) = next else {
                break;
            };
            let group = AssociationEvidenceGroup {
                evidence,
                group: Some(id),
            };
            let Some(likelihoods) = model.joint_log_likelihoods(request, hypothesis, group) else {
                return unassessed(ModelOutsideCoverage);
            };
            add_likelihoods(&mut log_yes, &mut log_no, likelihoods)?;
            previous = Some(id);
        }
    }
    if log_yes == f64::NEG_INFINITY && log_no == f64::NEG_INFINITY {
        return Err(AssociationError::ImpossibleEvidence);
    }
    let probability = if log_yes >= log_no {
        1.0 / (1.0 + (log_no - log_yes).exp())
    } else {
        let odds = (log_yes - log_no).exp();
        odds / (1.0 + odds)
    };
    // Finite evidence cannot establish an exact endpoint. Do not silently
    // publish floating-point saturation as model certainty.
    if log_yes.is_finite() && log_no.is_finite() && (probability == 0.0 || probability == 1.0) {
        return Err(AssociationError::NumericalRange);
    }
    Ok(AssociationResult::Estimated { probability })
}

fn add_likelihoods(
    yes: &mut f64,
    no: &mut f64,
    likelihoods: BinaryLogLikelihoods,
) -> Result<(), AssociationError> {
    for value in [
        likelihoods.given_association,
        likelihoods.given_no_association,
    ] {
        if value.is_nan() || value == f64::INFINITY {
            return Err(AssociationError::InvalidModelValue);
        }
    }
    *yes = add_log(*yes, likelihoods.given_association)?;
    *no = add_log(*no, likelihoods.given_no_association)?;
    Ok(())
}

fn add_log(left: f64, right: f64) -> Result<f64, AssociationError> {
    if left == f64::NEG_INFINITY || right == f64::NEG_INFINITY {
        return Ok(f64::NEG_INFINITY);
    }
    let sum = left + right;
    if !sum.is_finite() {
        return Err(AssociationError::NumericalRange);
    }
    Ok(sum)
}

#[cfg(test)]
#[path = "association_tests.rs"]
mod tests;
