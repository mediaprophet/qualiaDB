use super::test_support::*;
use super::*;
use crate::crypto::network::{cose, errors::CryptoError};

#[test]
fn full_pinned_pair_yields_scoped_receipt_and_recheck() {
    let f = Fixture::new(1, 1);
    let (owner, _, handle, receipt) = f.installed();
    assert_eq!(receipt.principal(), f.config().principal);
    assert_eq!(receipt.descriptor_digest(), &DIGEST);
    assert_eq!(receipt.pin_generation(), handle.generation());
    assert_eq!(owner.recheck(&receipt, &DIGEST, 10), Ok(()));
    assert_eq!(
        owner.recheck(&receipt, &[3; 48], 10),
        Err(CryptoError::Unauthorized)
    );
    assert_eq!(
        owner.recheck(&receipt, &DIGEST, 100),
        Err(CryptoError::Expired)
    );
}

#[test]
fn foreign_admin_foreign_pin_self_signed_and_classical_chain_refuse() {
    let f = Fixture::new(1, 1);
    let (owner, admin, handle, receipt) = f.installed();
    let (mut untrusted, other_admin) = PinAuthority::new_local_bootstrap().unwrap();
    assert_eq!(
        untrusted.install_pin(&admin, f.config(), 10).err(),
        Some(CryptoError::Unauthorized)
    );
    let (wire, n) = f.signed();
    let mut work = cose::Workspace::new();
    assert!(cose::verify_pair(&wire[..n], &f.binding(), &f.keys(), &mut work).is_ok());
    assert_eq!(
        untrusted
            .verify_source(
                AuthoritySource::LocalPin(handle),
                &wire[..n],
                &f.binding(),
                10,
                &mut work
            )
            .err(),
        Some(CryptoError::Unauthorized)
    );
    assert_eq!(
        untrusted.recheck(&receipt, &DIGEST, 10),
        Err(CryptoError::Unauthorized)
    );
    assert_eq!(
        owner
            .verify_source(
                AuthoritySource::ClassicalDid(b"did:qi:classical"),
                &wire[..n],
                &f.binding(),
                10,
                &mut work
            )
            .err(),
        Some(CryptoError::Unauthorized)
    );
    assert_eq!(
        owner
            .verify_source(
                AuthoritySource::SignedControllerChain(b"self-signed"),
                &wire[..n],
                &f.binding(),
                10,
                &mut work
            )
            .err(),
        Some(CryptoError::Unsupported)
    );
    // Explicit OTHER local admin may install its own trust; this cannot change owner.
    assert!(untrusted.install_pin(&other_admin, f.config(), 10).is_ok());
}

#[test]
fn current_scope_epoch_and_both_signatures_are_required() {
    let f = Fixture::new(1, 1);
    let (owner, _, handle, _) = f.installed();
    let (mut wire, n) = f.signed();
    let binding = f.binding();
    for wrong in [
        cose::Binding {
            purpose: b"route",
            ..binding
        },
        cose::Binding {
            audience: b"other-reader",
            ..binding
        },
        cose::Binding {
            controller_epoch: 2,
            ..binding
        },
    ] {
        assert_eq!(
            owner
                .verify_source(
                    AuthoritySource::LocalPin(handle),
                    &wire[..n],
                    &wrong,
                    10,
                    &mut cose::Workspace::new()
                )
                .err(),
            Some(CryptoError::Unauthorized)
        );
    }
    wire[n - 1] ^= 1;
    assert_eq!(
        owner
            .verify_source(
                AuthoritySource::LocalPin(handle),
                &wire[..n],
                &binding,
                10,
                &mut cose::Workspace::new()
            )
            .err(),
        Some(CryptoError::Unauthorized)
    );
}

#[test]
fn validity_is_half_open_and_bad_install_does_not_consume_capacity() {
    let f = Fixture::new(1, 1);
    let (mut owner, admin) = PinAuthority::new_local_bootstrap().unwrap();
    let mut bad = f.config();
    bad.expires_unix = bad.not_before_unix;
    assert_eq!(
        owner.install_pin(&admin, bad, 0).err(),
        Some(CryptoError::Malformed)
    );
    assert_eq!(
        owner.install_pin(&admin, f.config(), 100).err(),
        Some(CryptoError::Expired)
    );
    let handle = owner.install_pin(&admin, f.config(), 0).unwrap();
    let (wire, n) = f.signed();
    let mut work = cose::Workspace::new();
    assert_eq!(
        owner
            .verify_source(
                AuthoritySource::LocalPin(handle),
                &wire[..n],
                &f.binding(),
                9,
                &mut work
            )
            .err(),
        Some(CryptoError::Unauthorized)
    );
    assert!(owner
        .verify_source(
            AuthoritySource::LocalPin(handle),
            &wire[..n],
            &f.binding(),
            10,
            &mut work
        )
        .is_ok());
    assert_eq!(
        owner
            .verify_source(
                AuthoritySource::LocalPin(handle),
                &wire[..n],
                &f.binding(),
                100,
                &mut work
            )
            .err(),
        Some(CryptoError::Expired)
    );
}

#[test]
fn revocation_invalidates_receipts_pending_and_same_scope_reinstall() {
    let f = Fixture::new(1, 1);
    let (mut owner, admin, handle, receipt) = f.installed();
    let grant = f.grant();
    let op = owner
        .begin_key_operation(&receipt, &DIGEST, request(&grant), 10)
        .unwrap();
    owner.revoke_pin(&admin, handle).unwrap();
    assert_eq!(owner.pending_key_operations(), 0);
    assert_eq!(
        owner.recheck(&receipt, &DIGEST, 11),
        Err(CryptoError::Revoked)
    );
    assert!(owner.complete_key_operation(&op, &grant, 7, 5, 11).is_err());
    assert_eq!(
        owner.install_pin(&admin, f.config(), 11).err(),
        Some(CryptoError::Conflict)
    );
}

#[test]
fn full_rotation_advances_epoch_and_fences_old_receipts_and_callbacks() {
    let old = Fixture::new(1, 1);
    let new = Fixture::new(3, 2);
    let (mut owner, admin, old_handle, receipt) = old.installed();
    let grant = old.grant();
    let op = owner
        .begin_key_operation(&receipt, &DIGEST, request(&grant), 10)
        .unwrap();
    assert_eq!(
        owner.rotate_pin(&admin, old_handle, old.config(), 10).err(),
        Some(CryptoError::StaleGeneration)
    );
    let mut wrong = new.config();
    wrong.audience = b"scope-escalation";
    assert_eq!(
        owner.rotate_pin(&admin, old_handle, wrong, 10).err(),
        Some(CryptoError::Unauthorized)
    );
    assert!(owner.recheck(&receipt, &DIGEST, 10).is_ok());
    let new_handle = owner
        .rotate_pin(&admin, old_handle, new.config(), 11)
        .unwrap();
    assert_ne!(old_handle.generation(), new_handle.generation());
    assert_eq!(owner.pending_key_operations(), 0);
    assert_eq!(
        owner.recheck(&receipt, &DIGEST, 11),
        Err(CryptoError::StaleGeneration)
    );
    assert!(owner.complete_key_operation(&op, &grant, 7, 5, 11).is_err());
    let (wire, n) = new.signed();
    assert!(owner
        .verify_source(
            AuthoritySource::LocalPin(new_handle),
            &wire[..n],
            &new.binding(),
            11,
            &mut cose::Workspace::new()
        )
        .is_ok());
}

#[test]
fn rotation_requires_new_full_pair_and_revoked_epoch_can_only_advance() {
    let old = Fixture::new(1, 1);
    let same_keys_new_epoch = Fixture::new(1, 2);
    let new = Fixture::new(3, 2);
    let (mut owner, admin, handle, receipt) = old.installed();
    assert_eq!(
        owner
            .rotate_pin(&admin, handle, same_keys_new_epoch.config(), 10)
            .err(),
        Some(CryptoError::Conflict)
    );
    owner.revoke_pin(&admin, handle).unwrap();
    assert_eq!(
        owner.recheck(&receipt, &DIGEST, 10),
        Err(CryptoError::Revoked)
    );
    assert!(owner.rotate_pin(&admin, handle, new.config(), 10).is_ok());
}
