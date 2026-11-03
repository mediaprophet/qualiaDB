//! Fail-closed bridge from signed QualiaDB consent to source-bound sensing authority.
//!
//! The existing `ConsentScope` covers health record families, not sensor sources or
//! acquisition/processing acts. This module therefore requires host-provided identity,
//! sensor-scope, revocation, and trusted-clock authorities. It never treats a Quin or
//! caller-supplied identity projection as authorization.
//!
//! This is one optional adapter for the existing signed grant, not the sole
//! policy framework for sensing. The host must separately resolve data-subject
//! ownership and any applicable guardianship through its selected policy system;
//! this module alone does not bind or decide those relationships.

use crate::governance::consent_contract::{ConsentError, ConsentGrant};

/// Sensor family selected by the caller. NFC is distinct from Bluetooth even
/// when both observations contain radio protocol identifiers. Unknown or
/// future families need an explicit policy implementation before authorization.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SensingModality {
    Wifi,
    Bluetooth,
    Cellular,
    Sdr,
    Radar,
    VisibleCamera,
    NearInfraredCamera,
    ThermalCamera,
    Microphone,
    FieldInstrument,
    Nfc,
}

/// Distinct actions so capture consent cannot implicitly authorize later use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SensingAction {
    Acquire,
    Process,
    Store,
    Export,
    Share,
}

/// Exact operation requested under a consent grant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SensingRequest {
    actor_id: [u8; 32],
    source_id: u64,
    modality: SensingModality,
    action: SensingAction,
    purpose_hash: u64,
}

impl SensingRequest {
    pub fn new(
        actor_id: [u8; 32],
        source_id: u64,
        modality: SensingModality,
        action: SensingAction,
        purpose_hash: u64,
    ) -> Result<Self, AuthorizationError> {
        if source_id == 0 || purpose_hash == 0 || actor_id == [0; 32] {
            return Err(AuthorizationError::InvalidRequest);
        }
        Ok(Self {
            actor_id,
            source_id,
            modality,
            action,
            purpose_hash,
        })
    }

    pub const fn actor_id(self) -> [u8; 32] {
        self.actor_id
    }
    pub const fn source_id(self) -> u64 {
        self.source_id
    }
    pub const fn modality(self) -> SensingModality {
        self.modality
    }
    pub const fn action(self) -> SensingAction {
        self.action
    }
    pub const fn purpose_hash(self) -> u64 {
        self.purpose_hash
    }
}

/// Required fail-closed integration authorities. Implementations are trust
/// boundaries and must be backed by authenticated identity, a sensor-specific
/// signed scope assertion, durable revocation state, and a trusted clock.
pub trait IdentityAuthority {
    /// Bind the grant's signing key to an accepted human/authority identity and
    /// authenticate the actor as the grant recipient. Do not infer identity from
    /// public-key possession or nonzero hashes alone.
    fn verify_parties(
        &self,
        grant: &ConsentGrant,
        actor_id: &[u8; 32],
    ) -> Result<(), AuthorizationError>;
}

pub trait SensingScopeAuthority {
    /// Verify that a signed scope bound to this grant's digest permits this exact
    /// source, modality, action, and purpose. Mapping a health-category bit alone
    /// to sensor permission is insufficient.
    fn verify_scope(
        &self,
        grant: &ConsentGrant,
        grant_digest: &[u8; 32],
        request: &SensingRequest,
    ) -> Result<(), AuthorizationError>;
}

pub trait RevocationAuthority {
    /// Revalidate complete, restart-safe state for this exact grant digest at the
    /// broker's trusted current time. Implementations must recheck checkpoint
    /// freshness/anti-rollback on every call; unknown state is an error, never false.
    fn is_revoked(
        &self,
        grant_id: &[u8; 32],
        grant_digest: &[u8; 32],
        current_time_unix: u64,
    ) -> Result<bool, AuthorizationError>;
}

pub trait TrustedUnixClock {
    /// Trusted anti-rollback Unix seconds. This timestamp is not supplied by the
    /// request or inferred from the sensor's untrusted metadata.
    fn now_unix(&mut self) -> Result<u64, AuthorizationError>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthorizationError {
    InvalidRequest,
    InvalidGrant,
    GrantNotYetValid,
    GrantExpired,
    PurposeMismatch,
    ActorMismatch,
    IdentityUnverified,
    UnsupportedSensorScope,
    RevocationUnknown,
    Revoked,
    ClockUnavailable,
}

/// One-shot policy decision, not durable authorization. Before use, consume it
/// through `revalidate`, which rechecks trusted time, identity, scope and live
/// revocation. It is not a device receipt or proof that capture occurred.
#[derive(Debug, PartialEq, Eq)]
pub struct SensingPermit {
    grant_id: [u8; 32],
    grant_digest: [u8; 32],
    request: SensingRequest,
    issued_at_unix: u64,
    expires_at_unix: u64,
}

impl SensingPermit {
    pub const fn grant_id(&self) -> [u8; 32] {
        self.grant_id
    }
    pub const fn grant_digest(&self) -> [u8; 32] {
        self.grant_digest
    }
    pub const fn request(&self) -> SensingRequest {
        self.request
    }
    pub const fn issued_at_unix(&self) -> u64 {
        self.issued_at_unix
    }
    pub const fn expires_at_unix(&self) -> u64 {
        self.expires_at_unix
    }

    /// Consume this decision and re-run every authorization gate immediately
    /// before one operation. The returned non-copy handle is intended to be
    /// moved directly into that operation and cannot be reused to authorize a
    /// second request.
    pub fn revalidate(
        self,
        grant: &ConsentGrant,
        identity: &impl IdentityAuthority,
        sensor_scope: &impl SensingScopeAuthority,
        revocations: &impl RevocationAuthority,
        clock: &mut impl TrustedUnixClock,
    ) -> Result<RevalidatedSensingUse, AuthorizationError> {
        if self.grant_id != grant.grant_id
            || self.grant_digest != grant.compute_digest()
            || self.request.actor_id != grant.recipient_did
        {
            return Err(AuthorizationError::InvalidGrant);
        }
        let fresh = authorize_sensing(
            grant,
            self.request,
            identity,
            sensor_scope,
            revocations,
            clock,
        )?;
        Ok(RevalidatedSensingUse {
            grant_id: fresh.grant_id,
            grant_digest: fresh.grant_digest,
            request: fresh.request,
            checked_at_unix: fresh.issued_at_unix,
        })
    }
}

/// Fresh, non-copy, exact-request handle for one immediate operation. It carries
/// no persistent authority; callers must consume it at the operation boundary.
#[derive(Debug, PartialEq, Eq)]
pub struct RevalidatedSensingUse {
    grant_id: [u8; 32],
    grant_digest: [u8; 32],
    request: SensingRequest,
    checked_at_unix: u64,
}

impl RevalidatedSensingUse {
    pub const fn grant_id(&self) -> [u8; 32] {
        self.grant_id
    }
    pub const fn grant_digest(&self) -> [u8; 32] {
        self.grant_digest
    }
    pub const fn request(&self) -> SensingRequest {
        self.request
    }
    pub const fn checked_at_unix(&self) -> u64 {
        self.checked_at_unix
    }
}

/// Verify grant signature/expiry, principal-recipient binding, exact purpose,
/// sensor scope, and complete revocation state before minting a permit.
pub fn authorize_sensing(
    grant: &ConsentGrant,
    request: SensingRequest,
    identity: &impl IdentityAuthority,
    sensor_scope: &impl SensingScopeAuthority,
    revocations: &impl RevocationAuthority,
    clock: &mut impl TrustedUnixClock,
) -> Result<SensingPermit, AuthorizationError> {
    if request.actor_id != grant.recipient_did {
        return Err(AuthorizationError::ActorMismatch);
    }
    if request.purpose_hash != grant.purpose_hash {
        return Err(AuthorizationError::PurposeMismatch);
    }
    if grant.grant_id == [0; 32] || grant.nonce == 0 || grant.created_at >= grant.expires_at {
        return Err(AuthorizationError::InvalidGrant);
    }

    let now = clock.now_unix()?;
    if now < grant.created_at {
        return Err(AuthorizationError::GrantNotYetValid);
    }
    grant.verify(now).map_err(map_consent_error)?;
    identity.verify_parties(grant, &request.actor_id)?;
    let grant_digest = grant.compute_digest();
    let revoked = revocations
        .is_revoked(&grant.grant_id, &grant_digest, now)
        .map_err(|_| AuthorizationError::RevocationUnknown)?;
    if revoked {
        return Err(AuthorizationError::Revoked);
    }
    sensor_scope.verify_scope(grant, &grant_digest, &request)?;

    // External authority callbacks may take time or observe revocation. Take
    // a fresh trusted time and revocation snapshot after they return.
    let final_now = clock.now_unix()?;
    if final_now < now {
        return Err(AuthorizationError::ClockUnavailable);
    }
    grant.verify(final_now).map_err(map_consent_error)?;
    if revocations
        .is_revoked(&grant.grant_id, &grant_digest, final_now)
        .map_err(|_| AuthorizationError::RevocationUnknown)?
    {
        return Err(AuthorizationError::Revoked);
    }
    let issued_at = clock.now_unix()?;
    if issued_at < final_now {
        return Err(AuthorizationError::ClockUnavailable);
    }
    grant.verify(issued_at).map_err(map_consent_error)?;

    Ok(SensingPermit {
        grant_id: grant.grant_id,
        grant_digest,
        request,
        issued_at_unix: issued_at,
        expires_at_unix: grant.expires_at,
    })
}

fn map_consent_error(error: ConsentError) -> AuthorizationError {
    match error {
        ConsentError::Expired { .. } => AuthorizationError::GrantExpired,
        ConsentError::RequesterMismatch => AuthorizationError::ActorMismatch,
        ConsentError::Revoked { .. } => AuthorizationError::Revoked,
        _ => AuthorizationError::InvalidGrant,
    }
}

#[cfg(test)]
#[path = "authorization_tests.rs"]
mod tests;
