//! Biometric and worker-crash fixtures.

use super::common::{digest, expect_err};
use crate::net::qdnf::biometrics::{
    derive_template, execution_permit_from_match, match_local, recover_with_pin,
    recovery_without_biometric, remote_matching_selectable, reuse_template, Modality, RawCapture,
    ReuseIntent,
};
use crate::net::qdnf::cells::{reclaim_pass, PassCharge, PassGuard, PassOutcome};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::Generation;

fn capture() -> Result<RawCapture, QdnfError> {
    RawCapture::acquire(Modality::Fingerprint, digest(1), digest(2), 80, 0)
}

pub fn s18_match_not_authority() -> Result<(), QdnfError> {
    let probe = capture()?;
    let reference = derive_template(&probe, digest(3), digest(4), digest(5), Generation(1))?;
    let result = match_local(&probe, &reference, true, 10, 100)?;
    match execution_permit_from_match(&result) {
        Err(QdnfError::Denied) => Ok(()),
        Err(e) => Err(e),
        Ok(_) => Err(QdnfError::Denied),
    }
}

pub fn s19_cross_domain() -> Result<(), QdnfError> {
    let probe = capture()?;
    let template = derive_template(&probe, digest(3), digest(4), digest(5), Generation(1))?;
    expect_err(
        reuse_template(&template, ReuseIntent::Identification, digest(9)),
        QdnfError::Denied,
    )?;
    expect_err(
        reuse_template(&template, ReuseIntent::AuthorisedLocalMatch, digest(9)),
        QdnfError::Denied,
    )
}

pub fn s20_non_biometric_recovery() -> Result<(), QdnfError> {
    if !recovery_without_biometric() {
        return Err(QdnfError::Denied);
    }
    recover_with_pin(digest(7), digest(7))
}

pub fn s34_worker_crash() -> Result<(), QdnfError> {
    let charge = PassCharge {
        arena: 1024,
        scratch: 256,
        crypto: 128,
        kernel: 64,
        pinned: 0,
    };
    {
        let _guard = PassGuard::enter(&charge)?;
    }
    if reclaim_pass(PassOutcome::Unwind, charge.arena) != 0 {
        return Err(QdnfError::Capacity);
    }
    Ok(())
}

pub fn s40_threshold_missing() -> Result<(), QdnfError> {
    if remote_matching_selectable() {
        return Err(QdnfError::Downgrade);
    }
    if !recovery_without_biometric() {
        return Err(QdnfError::Denied);
    }
    Ok(())
}
