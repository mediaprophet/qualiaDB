//! E21.2 — exhaustive small-state ownership/commit/epoch checker. Not TLA+.

use core::sync::atomic::{AtomicBool, Ordering};

use crate::net::qdnf::authority::{
    binding_for_controllers, AuthorityOwner, ContactState, ExecutionPermit,
};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::Generation;

/// TLA+ / TLC has not been executed.
pub fn tla_plus_executed() -> bool {
    false
}

static CHECKED: AtomicBool = AtomicBool::new(false);

/// True after [`check_ownership_state_machine`] completes in this process.
pub fn bounded_ownership_model_checked() -> bool {
    CHECKED.load(Ordering::SeqCst)
}

fn expect_err(result: Result<(), QdnfError>, want: QdnfError) -> Result<(), QdnfError> {
    match result {
        Err(e) if e == want => Ok(()),
        Err(e) => Err(e),
        Ok(()) => Err(QdnfError::Conflict),
    }
}

fn contact_at(i: usize) -> ContactState {
    match i {
        0 => ContactState::Request,
        1 => ContactState::Consent,
        2 => ContactState::Active,
        3 => ContactState::Suspended,
        _ => ContactState::Blocked,
    }
}

/// Enumerate a tiny ownership/commit/epoch space.
///
/// Illegal transitions must surface [`QdnfError::StaleGeneration`] or
/// [`QdnfError::Unauthorized`] (or the more specific Revoked/Expired codes
/// on those paths). No TLA+ model is invoked.
pub fn check_ownership_state_machine() -> Result<usize, QdnfError> {
    let mut checked = 0usize;

    match Generation(u64::MAX).next() {
        Err(QdnfError::StaleGeneration) => checked = checked.saturating_add(1),
        _ => return Err(QdnfError::Conflict),
    }

    let mut contact_i = 0usize;
    while contact_i < 5 {
        let mut owner = AuthorityOwner::new();
        let binding =
            binding_for_controllers(b"did:q42:a", b"did:q42:b", b"q42:QSync/1", b"op-a")?;
        let contact = contact_at(contact_i);
        let installed = owner.install_grant(binding, 10, 100, contact);
        match (contact, installed) {
            (ContactState::Active, Ok(_)) => checked = checked.saturating_add(1),
            (ContactState::Active, Err(_)) => return Err(QdnfError::Conflict),
            (_, Err(QdnfError::Unauthorized)) => checked = checked.saturating_add(1),
            (_, Err(e)) => return Err(e),
            (_, Ok(_)) => return Err(QdnfError::Unauthorized),
        }
        contact_i = contact_i.saturating_add(1);
    }

    let mut owner = AuthorityOwner::new();
    let bind_a = binding_for_controllers(b"did:q42:a", b"did:q42:b", b"q42:QSync/1", b"op-a")?;
    let bind_a2 = binding_for_controllers(b"did:q42:a", b"did:q42:b", b"q42:QSync/1", b"op-a2")?;
    let bind_b = binding_for_controllers(b"did:q42:c", b"did:q42:d", b"q42:QSync/1", b"op-b")?;

    let (cred, contact, handle) =
        owner.install_grant(bind_a, 10, 100, ContactState::Active)?;
    checked = checked.saturating_add(1);

    let permit = owner.issue_permit(handle, bind_a, 20, 256)?;
    permit.matches_credential(&cred)?;
    permit.matches_contact(&contact)?;
    handle.matches_permit(&permit)?;
    permit.matches_handle(&handle)?;
    permit.current_at(20)?;
    checked = checked.saturating_add(1);

    expect_err(
        owner.issue_permit(handle, bind_b, 20, 256).map(|_| ()),
        QdnfError::Unauthorized,
    )?;
    checked = checked.saturating_add(1);

    let (_c2, _k2, handle2) = owner.install_grant(bind_a2, 10, 100, ContactState::Active)?;
    let permit2 = owner.issue_permit(handle2, bind_a2, 20, 256)?;
    expect_err(handle.matches_permit(&permit2), QdnfError::StaleGeneration)?;
    expect_err(permit2.matches_handle(&handle), QdnfError::StaleGeneration)?;
    checked = checked.saturating_add(1);

    expect_err(permit.current_at(100), QdnfError::Expired)?;
    let grant = owner.current_grant(handle)?;
    expect_err(permit.matches_grant(&grant, 100), QdnfError::Expired)?;
    permit.matches_grant(&grant, 20)?;
    checked = checked.saturating_add(1);

    expect_err(
        owner.issue_permit(handle, bind_a, 100, 256).map(|_| ()),
        QdnfError::Expired,
    )?;
    checked = checked.saturating_add(1);

    owner.revoke(handle)?;
    expect_err(
        owner.issue_permit(handle, bind_a, 20, 256).map(|_| ()),
        QdnfError::Revoked,
    )?;
    checked = checked.saturating_add(1);

    let mut owner2 = AuthorityOwner::new();
    let (_c, _k, live) =
        owner2.install_grant(bind_a, 10, 100, ContactState::Active)?;
    owner2.validate_handle(live)?;
    let issued: Result<ExecutionPermit, QdnfError> = owner2.issue_permit(live, bind_a, 20, 0);
    match issued {
        Err(QdnfError::Capacity) => checked = checked.saturating_add(1),
        Err(e) => return Err(e),
        Ok(_) => return Err(QdnfError::Conflict),
    }

    CHECKED.store(true, Ordering::SeqCst);
    Ok(checked)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_ownership_model_rejects_stale_and_unauthorized() {
        assert!(!tla_plus_executed());
        let n = check_ownership_state_machine().unwrap();
        assert!(bounded_ownership_model_checked());
        assert!(n >= 8);
        assert!(!tla_plus_executed());
    }

    #[test]
    fn generation_exhaustion_is_stale_not_wrap() {
        assert_eq!(
            Generation(u64::MAX).next(),
            Err(QdnfError::StaleGeneration)
        );
    }
}
