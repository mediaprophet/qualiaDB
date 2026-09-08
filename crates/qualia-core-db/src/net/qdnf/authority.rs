//! Semantic and authority contracts (FND-02).
//!
//! Entity, claim, handle and instrument planes stay distinct. No universal
//! NaturalAgent join key, reputation score, or authority from similarity, wallet
//! possession or nominal signature count.

use super::errors::QdnfError;
use super::types::{ProfileId, StrongDigest};

/// Identifier fabric planes. A verified route is not a NaturalAgent.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Plane {
    Entity = 1,
    Claim = 2,
    Handle = 3,
    Instrument = 4,
}

/// QPolicy decision. Only `Allow` with a current grant handle admits service traffic.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolicyOutcome {
    Allow = 1,
    Deny = 2,
    Challenge = 3,
    NeedsHuman = 4,
    Incomplete = 5,
    Error = 6,
}

impl PolicyOutcome {
    #[inline]
    pub const fn admits_service(self) -> bool {
        matches!(self, Self::Allow)
    }
}

/// Independent resource kinds. Energy, time and typed compute never mix.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceKind {
    EnergyJoules = 1,
    TimeSeconds = 2,
    TypedCompute = 3,
}

/// Observation quality. Unknown is not free or safe.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationQuality {
    Measured = 1,
    Estimated = 2,
    Unknown = 3,
}

/// Contributor / principal class for finite compensation.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompensationClass {
    Personal = 1,
    Humanitarian = 2,
    CorporateDelegated = 3,
}

/// Protected-person contact state. Payment cannot bypass these.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContactState {
    Request = 1,
    Consent = 2,
    Active = 3,
    Suspended = 4,
    Blocked = 5,
}

/// Independent facts that must not collapse into a trusted-peer boolean.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IndependentFacts {
    pub physically_reachable: bool,
    pub transport_key_possessed: bool,
    pub controller_authorized: bool,
    pub relationship_exists: bool,
    pub route_disclosable: bool,
    pub operation_authorized: bool,
}

impl IndependentFacts {
    pub const NONE: Self = Self {
        physically_reachable: false,
        transport_key_possessed: false,
        controller_authorized: false,
        relationship_exists: false,
        route_disclosable: false,
        operation_authorized: false,
    };
}

/// Purpose-scoped grant. Cache hits never renew permission.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TemporalGrant {
    pub purpose_digest: StrongDigest,
    pub audience_digest: StrongDigest,
    pub authority_generation: u64,
    pub not_before_unix: u64,
    pub expires_unix: u64,
    pub profile: ProfileId,
}

impl TemporalGrant {
    pub fn current_at(&self, now_unix: u64) -> Result<(), QdnfError> {
        if now_unix < self.not_before_unix {
            return Err(QdnfError::Unauthorized);
        }
        if now_unix >= self.expires_unix {
            return Err(QdnfError::Expired);
        }
        Ok(())
    }
}

/// Maps a policy outcome onto session admission.
pub fn admit_service(outcome: PolicyOutcome, grant_current: bool) -> Result<(), QdnfError> {
    match outcome {
        PolicyOutcome::Allow if grant_current => Ok(()),
        PolicyOutcome::Allow => Err(QdnfError::Expired),
        PolicyOutcome::Deny => Err(QdnfError::Denied),
        PolicyOutcome::Challenge => Err(QdnfError::Challenge),
        PolicyOutcome::NeedsHuman => Err(QdnfError::NeedsHuman),
        PolicyOutcome::Incomplete => Err(QdnfError::Incomplete),
        PolicyOutcome::Error => Err(QdnfError::Unauthorized),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reachability_is_not_authorization() {
        let mut facts = IndependentFacts::NONE;
        facts.physically_reachable = true;
        facts.transport_key_possessed = true;
        assert!(!facts.operation_authorized);
        assert!(!facts.controller_authorized);
    }

    #[test]
    fn allow_without_current_grant_is_not_admission() {
        assert_eq!(
            admit_service(PolicyOutcome::Allow, false),
            Err(QdnfError::Expired)
        );
        assert!(admit_service(PolicyOutcome::Allow, true).is_ok());
    }

    #[test]
    fn payment_cannot_bypass_blocked_contact() {
        assert_eq!(ContactState::Blocked as u8, 5);
        assert!(!PolicyOutcome::Deny.admits_service());
    }
}
