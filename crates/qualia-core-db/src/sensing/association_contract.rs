//! Host-model and evidence-policy boundaries for relational association.

use core::num::NonZeroU64;

use super::super::{
    radio_evidence::{RadioIdentifier, RadioObservationTime},
    record::{CalibrationDeclaration, MeasurementQuality, ProvenanceDeclaration},
    uncertainty::{EvidenceReference, MethodVersion, UncertaintyRecord},
    MeasurementModality, PhysicalFieldSemantics, SemanticId,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandidateKind {
    Unknown,
    Entity,
    Agent,
}

/// Opaque candidate reference, not an authenticated identity. Ontology types
/// can denote an animal, instrument, person, physical process, or another kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EntityCandidate {
    pub id: NonZeroU64,
    pub kind: CandidateKind,
    pub ontology_type: SemanticId,
}

/// Separate binary propositions may coexist; ownership, proximity, care and
/// device operation must use separate caller-selected relation identifiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AssociationHypothesis {
    pub id: NonZeroU64,
    pub subject: EntityCandidate,
    pub relation_id: NonZeroU64,
    pub object: EntityCandidate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssociationEvidenceKind {
    Measurement(MeasurementModality),
    RadioIdentifier,
    NfcIdentifier,
    EnvironmentalContext,
    SocialContext,
    IdentityAnchor,
    Other(SemanticId),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AssociationValue {
    Unknown,
    Missing,
    Scalar {
        value: f64,
        quantity: SemanticId,
        unit: SemanticId,
    },
    Radio(RadioIdentifier),
    /// Resolved by the host model/authority, including NFC records and identity
    /// anchor enumerations. A citation alone does not authenticate its contents.
    Reference(EvidenceReference),
}

/// Missing metadata remains explicit and is passed unchanged to both trust
/// boundaries. Dependence group IDs are declarations, not independence proof.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AssociationEvidence<'a> {
    pub observation_id: NonZeroU64,
    pub source_id: SemanticId,
    pub context_id: SemanticId,
    pub kind: AssociationEvidenceKind,
    pub value: AssociationValue,
    pub observed_at: RadioObservationTime,
    pub quality: MeasurementQuality,
    pub calibration: CalibrationDeclaration,
    pub provenance: ProvenanceDeclaration,
    pub field: Option<PhysicalFieldSemantics>,
    pub uncertainty: Option<&'a UncertaintyRecord>,
    pub dependence_group: Option<NonZeroU64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DependenceAssumption {
    Unknown,
    /// One model likelihood for the full joint observation, with arbitrary
    /// correlation. No multiplication of individual measurements occurs.
    Joint,
    /// Groups are conditionally independent under BOTH the hypothesis and its
    /// negation. Each group's internal correlation remains the model's job.
    IndependentGroups {
        justification: EvidenceReference,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AssociationModelReference {
    pub method: MethodVersion,
    pub calibration: EvidenceReference,
}

/// No person-specific policy is assumed. Even a scientific case without an
/// actor or configured policy needs an affirmative authority decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AssociationRequest {
    pub context_id: NonZeroU64,
    pub purpose_id: NonZeroU64,
    pub actor_id: Option<NonZeroU64>,
    pub policy_context: Option<EvidenceReference>,
}

/// Publication results are in exact hypothesis order. Combined with the other
/// authority arguments, they describe every proposed output field except the
/// publication decision that the authority is about to issue.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AssociationPhase<'a> {
    Evaluate,
    Publish {
        results: &'a [super::AssociationResult],
        evaluation_decision: EvidenceReference,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssociationPermission {
    Unknown,
    Denied,
    Approved { decision: EvidenceReference },
}

/// Must check current host/Qualia policy, evidence provenance and appropriate
/// use, candidate/relation scope, model calibration/applicability, and any
/// independence justification. Context citations alone are not permission.
/// Evaluate decisions bind the input scope. Publish decisions must bind that
/// scope, the evaluation decision, and every exact proposed result; inspecting
/// only the inputs is insufficient. Returned citations refer to the authority's
/// phase-specific records, not cryptographic receipts created by this module.
/// Calls must be bounded, allocation-free and deterministic for the snapshot.
pub trait AssociationAuthority {
    fn authorize(
        &mut self,
        phase: AssociationPhase<'_>,
        request: &AssociationRequest,
        hypotheses: &[AssociationHypothesis],
        evidence: &[AssociationEvidence<'_>],
        model: AssociationModelReference,
        dependence: DependenceAssumption,
    ) -> AssociationPermission;
}

/// A filtered borrow prevents accidental per-record multiplication in the
/// estimator. The host model must still account for all within-group dependence.
#[derive(Clone, Copy)]
pub struct AssociationEvidenceGroup<'a, 'data> {
    pub(super) evidence: &'a [AssociationEvidence<'data>],
    pub(super) group: Option<NonZeroU64>,
}

impl<'a, 'data> AssociationEvidenceGroup<'a, 'data> {
    pub const fn group_id(self) -> Option<NonZeroU64> {
        self.group
    }

    pub fn records(self) -> impl Iterator<Item = &'a AssociationEvidence<'data>> {
        self.evidence
            .iter()
            .filter(move |record| self.group.is_none() || record.dependence_group == self.group)
    }
}

/// Natural logs of joint likelihoods/densities for one group under H and !H.
/// Negative infinity denotes impossible evidence; NaN/+infinity are rejected.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BinaryLogLikelihoods {
    pub given_association: f64,
    pub given_no_association: f64,
}

/// Supplies a separately calibrated prior and joint group likelihoods. `None`
/// means outside model coverage, never neutral evidence or a default prior.
/// The prior must not already incorporate observations in the evidence slice.
/// Implementations must be bounded, allocation-free and deterministic; this
/// boundary does not train, calibrate, authenticate or sandbox a host model.
pub trait CalibratedAssociationModel {
    fn reference(&self) -> AssociationModelReference;
    fn prior_probability(
        &self,
        request: &AssociationRequest,
        hypothesis: &AssociationHypothesis,
    ) -> Option<f64>;
    fn joint_log_likelihoods(
        &self,
        request: &AssociationRequest,
        hypothesis: &AssociationHypothesis,
        evidence: AssociationEvidenceGroup<'_, '_>,
    ) -> Option<BinaryLogLikelihoods>;
}
