//! Exact, bounded lookup of externally supplied association-model data.
//!
//! Validation checks table shape and finite numbers, not training, calibration,
//! evidence authenticity or physical reality. The association authority must
//! verify the model/calibration reference against these exact table contents.
//! Citation equality binds reference IDs; the authority must separately bind
//! them to immutable resolved content before approving their use.
//! Priors and joint likelihoods are supplied explicitly; none are inferred from
//! identifiers, anchor counts, confidence bytes or calibration declarations.

use core::num::NonZeroU64;

use super::{
    association::{
        AssociationEvidence, AssociationEvidenceGroup, AssociationHypothesis,
        AssociationModelReference, AssociationRequest, AssociationValue, BinaryLogLikelihoods,
        CalibratedAssociationModel, DependenceAssumption, MAX_ASSOCIATION_EVIDENCE,
        MAX_ASSOCIATION_HYPOTHESES,
    },
    uncertainty::UncertaintyEstimate,
};

pub const MAX_ASSOCIATION_TABLE_LIKELIHOODS: usize =
    MAX_ASSOCIATION_HYPOTHESES * MAX_ASSOCIATION_EVIDENCE;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AssociationPriorRow {
    pub hypothesis: AssociationHypothesis,
    pub probability: f64,
}

/// A row applies only to the table's exact request, full hypothesis, and exact
/// ordered group evidence. `None` denotes the entire joint evidence slice.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AssociationLikelihoodRow {
    pub hypothesis_id: NonZeroU64,
    pub group_id: Option<NonZeroU64>,
    pub likelihoods: BinaryLogLikelihoods,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssociationTableError {
    CapacityExceeded,
    MissingInputs,
    InvalidDependence,
    InvalidEvidence,
    DuplicateEvidence,
    InvalidPrior,
    DuplicatePrior,
    InvalidLikelihood,
    DuplicateLikelihood,
    MissingLikelihood,
    UnknownHypothesis,
    UnknownGroup,
}

/// Zero-allocation borrowed model with complete declared table coverage.
/// This concrete implementation accepts only finite log likelihoods/densities;
/// exact impossible events require a different explicitly qualified model.
pub struct TableAssociationModel<'model, 'data> {
    reference: AssociationModelReference,
    request: AssociationRequest,
    dependence: DependenceAssumption,
    evidence: &'model [AssociationEvidence<'data>],
    priors: &'model [AssociationPriorRow],
    likelihoods: &'model [AssociationLikelihoodRow],
}

impl<'model, 'data> TableAssociationModel<'model, 'data> {
    pub fn new(
        reference: AssociationModelReference,
        request: AssociationRequest,
        dependence: DependenceAssumption,
        evidence: &'model [AssociationEvidence<'data>],
        priors: &'model [AssociationPriorRow],
        likelihoods: &'model [AssociationLikelihoodRow],
    ) -> Result<Self, AssociationTableError> {
        if evidence.len() > MAX_ASSOCIATION_EVIDENCE
            || priors.len() > MAX_ASSOCIATION_HYPOTHESES
            || likelihoods.len() > MAX_ASSOCIATION_TABLE_LIKELIHOODS
        {
            return Err(AssociationTableError::CapacityExceeded);
        }
        if evidence.is_empty() || priors.is_empty() {
            return Err(AssociationTableError::MissingInputs);
        }
        if dependence == DependenceAssumption::Unknown {
            return Err(AssociationTableError::InvalidDependence);
        }
        for (index, record) in evidence.iter().enumerate() {
            if evidence[..index]
                .iter()
                .any(|other| other.observation_id == record.observation_id)
            {
                return Err(AssociationTableError::DuplicateEvidence);
            }
            if matches!(record.value, AssociationValue::Scalar { value, .. } if !value.is_finite())
                || record
                    .uncertainty
                    .is_some_and(|uncertainty| record.field != Some(uncertainty.field()))
            {
                return Err(AssociationTableError::InvalidEvidence);
            }
            if matches!(dependence, DependenceAssumption::IndependentGroups { .. })
                && record.dependence_group.is_none()
            {
                return Err(AssociationTableError::InvalidDependence);
            }
        }
        for (index, row) in priors.iter().enumerate() {
            if !row.probability.is_finite() || !(0.0..=1.0).contains(&row.probability) {
                return Err(AssociationTableError::InvalidPrior);
            }
            if priors[..index]
                .iter()
                .any(|other| other.hypothesis.id == row.hypothesis.id)
            {
                return Err(AssociationTableError::DuplicatePrior);
            }
        }
        for (index, row) in likelihoods.iter().enumerate() {
            if !row.likelihoods.given_association.is_finite()
                || !row.likelihoods.given_no_association.is_finite()
            {
                return Err(AssociationTableError::InvalidLikelihood);
            }
            if likelihoods[..index].iter().any(|other| {
                other.hypothesis_id == row.hypothesis_id && other.group_id == row.group_id
            }) {
                return Err(AssociationTableError::DuplicateLikelihood);
            }
            if !priors
                .iter()
                .any(|prior| prior.hypothesis.id == row.hypothesis_id)
            {
                return Err(AssociationTableError::UnknownHypothesis);
            }
            if !group_exists(dependence, evidence, row.group_id) {
                return Err(AssociationTableError::UnknownGroup);
            }
        }
        for prior in priors {
            if dependence == DependenceAssumption::Joint {
                require_row(likelihoods, prior.hypothesis.id, None)?;
            } else {
                for (index, record) in evidence.iter().enumerate() {
                    if !evidence[..index]
                        .iter()
                        .any(|other| other.dependence_group == record.dependence_group)
                    {
                        require_row(likelihoods, prior.hypothesis.id, record.dependence_group)?;
                    }
                }
            }
        }
        Ok(Self {
            reference,
            request,
            dependence,
            evidence,
            priors,
            likelihoods,
        })
    }

    pub const fn request(&self) -> AssociationRequest {
        self.request
    }
    pub const fn dependence(&self) -> DependenceAssumption {
        self.dependence
    }
    pub const fn evidence(&self) -> &'model [AssociationEvidence<'data>] {
        self.evidence
    }
    pub const fn priors(&self) -> &'model [AssociationPriorRow] {
        self.priors
    }
    pub const fn likelihoods(&self) -> &'model [AssociationLikelihoodRow] {
        self.likelihoods
    }
}

impl CalibratedAssociationModel for TableAssociationModel<'_, '_> {
    fn reference(&self) -> AssociationModelReference {
        self.reference
    }

    fn prior_probability(
        &self,
        request: &AssociationRequest,
        hypothesis: &AssociationHypothesis,
    ) -> Option<f64> {
        if *request != self.request {
            return None;
        }
        self.priors
            .iter()
            .find(|row| row.hypothesis == *hypothesis)
            .map(|row| row.probability)
    }

    fn joint_log_likelihoods(
        &self,
        request: &AssociationRequest,
        hypothesis: &AssociationHypothesis,
        group: AssociationEvidenceGroup<'_, '_>,
    ) -> Option<BinaryLogLikelihoods> {
        if self.prior_probability(request, hypothesis).is_none() {
            return None;
        }
        let group_id = group.group_id();
        if !group_exists(self.dependence, self.evidence, group_id) {
            return None;
        }
        let mut expected = self
            .evidence
            .iter()
            .filter(|record| group_id.is_none() || record.dependence_group == group_id);
        let mut actual = group.records();
        loop {
            match (expected.next(), actual.next()) {
                (None, None) => break,
                (Some(left), Some(right)) if exact_record(left, right) => {}
                _ => return None,
            }
        }
        self.likelihoods
            .iter()
            .find(|row| row.hypothesis_id == hypothesis.id && row.group_id == group_id)
            .map(|row| row.likelihoods)
    }
}

fn group_exists(
    dependence: DependenceAssumption,
    evidence: &[AssociationEvidence<'_>],
    group: Option<NonZeroU64>,
) -> bool {
    match dependence {
        DependenceAssumption::Joint => group.is_none(),
        DependenceAssumption::IndependentGroups { .. } => {
            group.is_some()
                && evidence
                    .iter()
                    .any(|record| record.dependence_group == group)
        }
        DependenceAssumption::Unknown => false,
    }
}

fn require_row(
    rows: &[AssociationLikelihoodRow],
    hypothesis: NonZeroU64,
    group: Option<NonZeroU64>,
) -> Result<(), AssociationTableError> {
    if rows
        .iter()
        .any(|row| row.hypothesis_id == hypothesis && row.group_id == group)
    {
        Ok(())
    } else {
        Err(AssociationTableError::MissingLikelihood)
    }
}

fn exact_record(left: &AssociationEvidence<'_>, right: &AssociationEvidence<'_>) -> bool {
    if left != right {
        return false;
    }
    if let (AssociationValue::Scalar { value: a, .. }, AssociationValue::Scalar { value: b, .. }) =
        (left.value, right.value)
    {
        if a.to_bits() != b.to_bits() {
            return false;
        }
    }
    match (left.uncertainty, right.uncertainty) {
        (Some(left), Some(right)) => match (left.estimate(), right.estimate()) {
            (
                UncertaintyEstimate::AbsoluteBound { value: a, .. },
                UncertaintyEstimate::AbsoluteBound { value: b, .. },
            ) => a.to_bits() == b.to_bits(),
            (
                UncertaintyEstimate::ConfidenceInterval {
                    lower: a, upper: b, ..
                },
                UncertaintyEstimate::ConfidenceInterval {
                    lower: c, upper: d, ..
                },
            ) => a.to_bits() == c.to_bits() && b.to_bits() == d.to_bits(),
            _ => true,
        },
        _ => true,
    }
}

#[cfg(test)]
#[path = "association_table_tests.rs"]
mod tests;
