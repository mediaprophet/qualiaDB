//! Clinical mailbox, pairing, envelope and help fixtures.

use super::common::{digest, expect_err, verified};
use crate::net::qdnf::authority::{ContactState, PolicyOutcome};
use crate::net::qdnf::clinical::{
    admit_confidential_help, notify_guardian, seal_envelope, CapacityMandatePolicy, CareGrant,
    CareGrantKind, GrantTable, Mailbox,
};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::policy_labels::Confidentiality;
use crate::net::qdnf::types::StrongDigest;

fn clinician() -> StrongDigest {
    digest(2)
}

fn grant_until(audience: StrongDigest, exp: u64) -> crate::net::qdnf::authority::TemporalGrant {
    crate::net::qdnf::authority::TemporalGrant {
        purpose_digest: digest(0x11),
        audience_digest: audience,
        authority_generation: 1,
        not_before_unix: 0,
        expires_unix: exp,
        profile: crate::net::qdnf::types::ProfileId::QDNF_CRYPTO_1,
    }
}

pub fn s07_revoke_before_deliver() -> Result<(), QdnfError> {
    let mut mb = Mailbox::new();
    let slot = mb.store(b"pending-ct", clinician(), 1)?;
    mb.revoke_clinician(clinician())?;
    expect_err(mb.deliver(slot, clinician(), 1), QdnfError::Revoked)
}

pub fn s08_blocked_is_not_grant() -> Result<(), QdnfError> {
    let mut grants = GrantTable::new();
    grants.add(CareGrant {
        kind: CareGrantKind::Clinician,
        recipient: clinician(),
        grant: grant_until(clinician(), 100),
    })?;
    match grants.authorize_kind(
        CareGrantKind::Clinician,
        clinician(),
        ContactState::Active,
        ContactState::Blocked,
        10,
    ) {
        Err(QdnfError::Denied) => Ok(()),
        Err(e) => Err(e),
        Ok(()) => Err(QdnfError::Denied),
    }
}

pub fn s09_ordinary_bytes() -> Result<(), QdnfError> {
    let label = verified(Confidentiality::C2Sensitive, digest(1))?;
    let mut grants = GrantTable::new();
    grants.add(CareGrant {
        kind: CareGrantKind::Clinician,
        recipient: clinician(),
        grant: grant_until(clinician(), 100),
    })?;
    match seal_envelope(
        &grants,
        &label,
        digest(0x21),
        digest(0x11),
        100,
        b"",
        PolicyOutcome::Allow,
        ContactState::Active,
        ContactState::Active,
        10,
    ) {
        Err(QdnfError::Malformed) => Ok(()),
        Err(e) => Err(e),
        Ok(_) => Err(QdnfError::Malformed),
    }
}

pub fn s10_key_generation() -> Result<(), QdnfError> {
    let mut mb = Mailbox::new();
    let slot = mb.store(b"sealed-ct", clinician(), 1)?;
    expect_err(mb.deliver(slot, clinician(), 2), QdnfError::StaleGeneration)
}

pub fn s11_confidential_help() -> Result<(), QdnfError> {
    expect_err(
        notify_guardian(
            CapacityMandatePolicy::IndependentConfidentialHelp,
            digest(3),
            ContactState::Active,
        ),
        QdnfError::Denied,
    )?;
    admit_confidential_help(
        digest(8),
        CapacityMandatePolicy::IndependentConfidentialHelp,
        ContactState::Blocked,
    )
}

pub fn s28_stale_generation() -> Result<(), QdnfError> {
    s10_key_generation()
}

pub fn s38_revoke_before_consume() -> Result<(), QdnfError> {
    s07_revoke_before_deliver()
}

pub fn s39_revoke_after_chunk() -> Result<(), QdnfError> {
    let mut mb = Mailbox::new();
    let first = mb.store(b"chunk-one", clinician(), 1)?;
    mb.deliver(first, clinician(), 1)?;
    let second = mb.store(b"chunk-two", clinician(), 1)?;
    mb.revoke_clinician(clinician())?;
    expect_err(mb.deliver(second, clinician(), 1), QdnfError::Revoked)
}
