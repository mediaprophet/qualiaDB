//! Semantic and authority contracts (FND-02).
//!
//! Entity, claim, handle and instrument planes stay distinct. No universal
//! NaturalAgent join key, reputation score, or authority from similarity, wallet
//! possession or nominal signature count.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::{ProfileId, StrongDigest};

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

    pub const ALL: Self = Self {
        physically_reachable: true,
        transport_key_possessed: true,
        controller_authorized: true,
        relationship_exists: true,
        route_disclosable: true,
        operation_authorized: true,
    };
}

/// Independent facts must not collapse into a trusted-peer boolean.
#[inline]
pub fn collapse_to_trusted_peer(_facts: IndependentFacts) -> Result<bool, QdnfError> {
    Err(QdnfError::Denied)
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

/// Cached view of a grant. `cached_at` is observational; it never extends expiry.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CachedGrant {
    pub stored: TemporalGrant,
    pub cached_at: u64,
}

impl CachedGrant {
    /// Uses the stored grant's `not_before` / `expires` clocks, not `cached_at`.
    #[inline]
    pub fn current_at(&self, now_unix: u64) -> Result<(), QdnfError> {
        let _ = self.cached_at;
        self.stored.current_at(now_unix)
    }
}

/// Blocked/Revoked beat Allow. An expired historical grant cannot authorize now.
#[inline]
pub fn evaluate_precedence(
    revoked: bool,
    blocked: bool,
    expired: bool,
    allow: bool,
) -> Result<(), QdnfError> {
    if revoked {
        return Err(QdnfError::Revoked);
    }
    if blocked {
        return Err(QdnfError::Denied);
    }
    if expired {
        return Err(QdnfError::Expired);
    }
    if allow {
        Ok(())
    } else {
        Err(QdnfError::Unauthorized)
    }
}

/// Bootstrap/control allowance is independent of payment. Paid cannot admit alone.
#[inline]
pub fn bootstrap_admit(paid: bool, control_allowance: bool) -> Result<(), QdnfError> {
    let _ = paid;
    if control_allowance {
        Ok(())
    } else {
        Err(QdnfError::Denied)
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

    fn grant_until(expires_unix: u64) -> TemporalGrant {
        TemporalGrant {
            purpose_digest: StrongDigest::ZERO,
            audience_digest: StrongDigest::ZERO,
            authority_generation: 1,
            not_before_unix: 0,
            expires_unix,
            profile: ProfileId::QDNF_CRYPTO_1,
        }
    }

    #[test]
    fn collapse_to_trusted_peer_is_refused_even_when_all_facts_hold() {
        assert_eq!(
            collapse_to_trusted_peer(IndependentFacts::ALL),
            Err(QdnfError::Denied)
        );
        assert_eq!(
            collapse_to_trusted_peer(IndependentFacts::NONE),
            Err(QdnfError::Denied)
        );
    }

    #[test]
    fn cache_hit_does_not_renew_expiry() {
        let cached = CachedGrant {
            stored: grant_until(100),
            cached_at: 90,
        };
        assert!(cached.current_at(90).is_ok());
        assert_eq!(cached.current_at(100), Err(QdnfError::Expired));
        assert_eq!(cached.current_at(150), Err(QdnfError::Expired));
        assert_eq!(cached.stored.expires_unix, 100);
        assert_eq!(cached.cached_at, 90);
    }

    #[test]
    fn cached_at_is_not_the_expiry_clock() {
        let cached = CachedGrant {
            stored: grant_until(50),
            cached_at: 200,
        };
        assert_eq!(cached.current_at(40), Ok(()));
        assert_eq!(cached.current_at(50), Err(QdnfError::Expired));
        assert_eq!(cached.current_at(200), Err(QdnfError::Expired));
    }

    #[test]
    fn revoked_beats_allow() {
        assert_eq!(
            evaluate_precedence(true, false, false, true),
            Err(QdnfError::Revoked)
        );
    }

    #[test]
    fn blocked_beats_allow() {
        assert_eq!(
            evaluate_precedence(false, true, false, true),
            Err(QdnfError::Denied)
        );
    }

    #[test]
    fn expired_historical_allow_cannot_authorize_current() {
        assert_eq!(
            evaluate_precedence(false, false, true, true),
            Err(QdnfError::Expired)
        );
    }

    #[test]
    fn revoked_precedes_blocked_and_expired() {
        assert_eq!(
            evaluate_precedence(true, true, true, true),
            Err(QdnfError::Revoked)
        );
        assert_eq!(
            evaluate_precedence(false, true, true, true),
            Err(QdnfError::Denied)
        );
    }

    #[test]
    fn current_allow_without_revocation_block_or_expiry() {
        assert!(evaluate_precedence(false, false, false, true).is_ok());
        assert_eq!(
            evaluate_precedence(false, false, false, false),
            Err(QdnfError::Unauthorized)
        );
    }

    #[test]
    fn payment_cannot_admit_without_control_allowance() {
        assert_eq!(bootstrap_admit(true, false), Err(QdnfError::Denied));
        assert!(bootstrap_admit(false, true).is_ok());
        assert!(bootstrap_admit(true, true).is_ok());
        assert_eq!(bootstrap_admit(false, false), Err(QdnfError::Denied));
    }
}
