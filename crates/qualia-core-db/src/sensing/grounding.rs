//! Bounded fail-closed grounding boundary for sensing-derived claims.
//!
//! This module compares explicit physical semantics and exact NQuin assertions,
//! preserves source/sample/citation identity and declared uncertainty, then
//! requires an injected evidence authority before any claim can be accepted.
//! It does not authenticate signatures itself, resolve arbitrary citation
//! hashes, establish scientific truth, or persist/audit a decision.

use crate::NQuin;

use super::record::CalibrationDeclaration;

pub const MAX_SENSING_GROUNDING_EVIDENCE: usize = 64;
pub const UNCERTAINTY_SCALE: u32 = 1_000_000;

/// Explicit proposition and physical quantity/unit binding to evaluate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SensingClaim {
    assertion: NQuin,
    physical_quantity_id: u64,
    unit_id: u64,
}

impl SensingClaim {
    pub const fn new(
        assertion: NQuin,
        physical_quantity_id: u64,
        unit_id: u64,
    ) -> Result<Self, GroundingError> {
        if physical_quantity_id == 0 || unit_id == 0 {
            return Err(GroundingError::MissingPhysicalSemantics);
        }
        Ok(Self {
            assertion,
            physical_quantity_id,
            unit_id,
        })
    }

    pub const fn assertion(self) -> NQuin {
        self.assertion
    }

    pub const fn physical_quantity_id(self) -> u64 {
        self.physical_quantity_id
    }

    pub const fn unit_id(self) -> u64 {
        self.unit_id
    }
}

/// An explicit, unresolved sensing-evidence declaration.
///
/// These values are assertions until `SensingEvidenceAuthenticator` confirms
/// the exact declaration against trusted signed evidence. Unknown uncertainty
/// remains `Unknown`; it is never replaced by zero. A calibration enum value
/// is likewise a declaration, not cryptographic proof by itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SensingEvidence {
    citation_id: u64,
    source_id: u64,
    sample_id: u64,
    assertion: NQuin,
    physical_quantity_id: u64,
    unit_id: u64,
    uncertainty: EvidenceUncertainty,
    calibration: CalibrationDeclaration,
}

impl SensingEvidence {
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        citation_id: u64,
        source_id: u64,
        sample_id: u64,
        assertion: NQuin,
        physical_quantity_id: u64,
        unit_id: u64,
        uncertainty: EvidenceUncertainty,
        calibration: CalibrationDeclaration,
    ) -> Result<Self, GroundingError> {
        if citation_id == 0 || source_id == 0 || sample_id == 0 {
            return Err(GroundingError::InvalidIdentifier);
        }
        if physical_quantity_id == 0 || unit_id == 0 {
            return Err(GroundingError::MissingPhysicalSemantics);
        }
        Ok(Self {
            citation_id,
            source_id,
            sample_id,
            assertion,
            physical_quantity_id,
            unit_id,
            uncertainty,
            calibration,
        })
    }

    pub const fn citation_id(self) -> u64 {
        self.citation_id
    }

    pub const fn source_id(self) -> u64 {
        self.source_id
    }

    pub const fn sample_id(self) -> u64 {
        self.sample_id
    }

    pub const fn assertion(self) -> NQuin {
        self.assertion
    }

    pub const fn physical_quantity_id(self) -> u64 {
        self.physical_quantity_id
    }

    pub const fn unit_id(self) -> u64 {
        self.unit_id
    }

    pub const fn uncertainty(self) -> EvidenceUncertainty {
        self.uncertainty
    }

    pub const fn calibration(self) -> CalibrationDeclaration {
        self.calibration
    }
}

/// Relative uncertainty declaration in millionths, or explicitly unknown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceUncertainty {
    Unknown,
    RelativeMillionths(u32),
}

impl EvidenceUncertainty {
    pub const fn relative_millionths(value: u32) -> Result<Self, GroundingError> {
        if value > UNCERTAINTY_SCALE {
            return Err(GroundingError::InvalidUncertainty);
        }
        Ok(Self::RelativeMillionths(value))
    }
}

/// Implemented only at the trust boundary that can verify the signed evidence
/// envelope and its source/grant/revocation/retention state. `true` must bind
/// every field of `evidence`, including its fact bytes, unit, uncertainty,
/// calibration declaration and IDs; a bare hash or unsigned record is never
/// sufficient. This trait is an authority boundary, not a signature algorithm.
pub trait SensingEvidenceAuthenticator {
    fn authenticates(&self, evidence: &SensingEvidence) -> bool;

    /// Confirm that the referenced calibration artifact itself is valid for
    /// this source/sample/quantity/time. A signed declaration alone is not
    /// calibration verification. Implementations may conservatively return
    /// false when no calibration authority or resolver is available.
    fn calibration_verified(&self, _evidence: &SensingEvidence) -> bool {
        false
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GroundingPolicy {
    maximum_auto_uncertainty_millionths: u32,
    require_calibration: bool,
}

impl GroundingPolicy {
    pub const fn new(
        maximum_auto_uncertainty_millionths: u32,
        require_calibration: bool,
    ) -> Result<Self, GroundingError> {
        if maximum_auto_uncertainty_millionths > UNCERTAINTY_SCALE {
            return Err(GroundingError::InvalidPolicy);
        }
        Ok(Self {
            maximum_auto_uncertainty_millionths,
            require_calibration,
        })
    }
}

/// Deterministic disposition. `Accepted` means authenticated support met the
/// declared uncertainty/calibration policy; it does not mean physical truth.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroundingDecision {
    Accepted {
        citation_id: u64,
        source_id: u64,
        sample_id: u64,
        uncertainty_millionths: u32,
    },
    Review {
        citation_id: u64,
        source_id: u64,
        sample_id: u64,
        reason: ReviewReason,
    },
    Denied(DenialReason),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReviewReason {
    UnknownUncertainty,
    UncertaintyAbovePolicy,
    CalibrationNotVerified,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DenialReason {
    NoEvidence,
    TooManyEvidenceItems,
    DuplicateCitation,
    NoExactSupport,
    UnsupportedEvidenceOnly,
    UnauthenticatedExactSupport,
    InvalidUncertainty,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroundingError {
    InvalidIdentifier,
    MissingPhysicalSemantics,
    InvalidUncertainty,
    InvalidPolicy,
}

/// Ground a claim against at most 64 explicitly resolved evidence records.
///
/// Only exact `(subject,predicate,object,context,metadata)` matches with equal
/// physical quantity and unit count as support. The evaluator never accepts an
/// unauthenticated exact match. Among authenticated exact matches, it chooses
/// the lowest known uncertainty, then citation ID, for deterministic output.
/// Unknown/high uncertainty or absent required calibration returns `Review`;
/// no exact authenticated support returns `Denied`.
pub fn evaluate_sensing_grounding(
    claim: SensingClaim,
    evidence: &[SensingEvidence],
    policy: GroundingPolicy,
    authenticator: &dyn SensingEvidenceAuthenticator,
) -> GroundingDecision {
    if evidence.is_empty() {
        return GroundingDecision::Denied(DenialReason::NoEvidence);
    }
    if evidence.len() > MAX_SENSING_GROUNDING_EVIDENCE {
        return GroundingDecision::Denied(DenialReason::TooManyEvidenceItems);
    }
    for i in 0..evidence.len() {
        for j in (i + 1)..evidence.len() {
            if evidence[i].citation_id == evidence[j].citation_id {
                return GroundingDecision::Denied(DenialReason::DuplicateCitation);
            }
        }
    }

    let mut exact_seen = false;
    let mut best: Option<(usize, Option<u32>)> = None;
    for (index, item) in evidence.iter().enumerate() {
        if !same_assertion(&claim.assertion, &item.assertion)
            || claim.physical_quantity_id != item.physical_quantity_id
            || claim.unit_id != item.unit_id
        {
            continue;
        }
        exact_seen = true;
        if !authenticator.authenticates(item) {
            continue;
        }
        let uncertainty = match item.uncertainty {
            EvidenceUncertainty::Unknown => None,
            EvidenceUncertainty::RelativeMillionths(value) if value <= UNCERTAINTY_SCALE => {
                Some(value)
            }
            EvidenceUncertainty::RelativeMillionths(_) => {
                return GroundingDecision::Denied(DenialReason::InvalidUncertainty)
            }
        };
        let replace_best = match best {
            None => true,
            Some((best_index, best_uncertainty)) => {
                uncertainty_order(uncertainty, item.citation_id)
                    < uncertainty_order(best_uncertainty, evidence[best_index].citation_id)
            }
        };
        if replace_best {
            best = Some((index, uncertainty));
        }
    }

    let Some((best_index, uncertainty)) = best else {
        return GroundingDecision::Denied(if exact_seen {
            DenialReason::UnauthenticatedExactSupport
        } else if evidence
            .iter()
            .any(|item| same_assertion(&claim.assertion, &item.assertion))
        {
            DenialReason::UnsupportedEvidenceOnly
        } else {
            DenialReason::NoExactSupport
        });
    };
    let selected = evidence[best_index];
    let identity = (selected.citation_id, selected.source_id, selected.sample_id);
    let Some(uncertainty_millionths) = uncertainty else {
        return GroundingDecision::Review {
            citation_id: identity.0,
            source_id: identity.1,
            sample_id: identity.2,
            reason: ReviewReason::UnknownUncertainty,
        };
    };
    if uncertainty_millionths > policy.maximum_auto_uncertainty_millionths {
        return GroundingDecision::Review {
            citation_id: identity.0,
            source_id: identity.1,
            sample_id: identity.2,
            reason: ReviewReason::UncertaintyAbovePolicy,
        };
    }
    if policy.require_calibration
        && (!matches!(selected.calibration, CalibrationDeclaration::Calibrated(_))
            || !authenticator.calibration_verified(&selected))
    {
        return GroundingDecision::Review {
            citation_id: identity.0,
            source_id: identity.1,
            sample_id: identity.2,
            reason: ReviewReason::CalibrationNotVerified,
        };
    }
    GroundingDecision::Accepted {
        citation_id: identity.0,
        source_id: identity.1,
        sample_id: identity.2,
        uncertainty_millionths,
    }
}

fn uncertainty_order(uncertainty: Option<u32>, citation_id: u64) -> (u8, u32, u64) {
    match uncertainty {
        Some(value) => (0, value, citation_id),
        None => (1, u32::MAX, citation_id),
    }
}

fn same_assertion(left: &NQuin, right: &NQuin) -> bool {
    left.subject == right.subject
        && left.predicate == right.predicate
        && left.object == right.object
        && left.context == right.context
        && left.metadata == right.metadata
}
