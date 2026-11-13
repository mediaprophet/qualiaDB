use super::test_support::*;
use crate::crypto::network::errors::CryptoError;
use crate::crypto::network::types::KeyPurpose;

#[test]
fn cancel_late_complete_and_slot_reuse_cannot_consume_new_operation() {
    let f = Fixture::new(1, 1);
    let (mut owner, _, _, receipt) = f.installed();
    let grant = f.grant();
    let first = owner
        .begin_key_operation(&receipt, &DIGEST, request(&grant), 10)
        .unwrap();
    owner.cancel_key_operation(&first).unwrap();
    assert_eq!(
        owner.cancel_key_operation(&first),
        Err(CryptoError::StaleGeneration)
    );
    let second = owner
        .begin_key_operation(&receipt, &DIGEST, request(&grant), 10)
        .unwrap();
    assert_eq!(
        owner.complete_key_operation(&first, &grant, 7, 5, 11).err(),
        Some(CryptoError::StaleGeneration)
    );
    assert_eq!(owner.pending_key_operations(), 1);
    let complete = owner
        .complete_key_operation(&second, &grant, 7, 5, 11)
        .unwrap();
    assert_eq!(complete.descriptor_digest(), &DIGEST);
    assert_eq!(complete.principal(), receipt.principal());
    assert!(complete.operation_generation().0 > 1);
    assert_eq!(owner.pending_key_operations(), 0);
    assert_eq!(
        owner
            .complete_key_operation(&second, &grant, 7, 5, 11)
            .err(),
        Some(CryptoError::StaleGeneration)
    );
}

#[test]
fn foreign_owner_cannot_complete_or_cancel_and_cannot_use_foreign_receipt() {
    let f = Fixture::new(1, 1);
    let (mut owner, _, _, receipt) = f.installed();
    let (mut other, _, _, _) = f.installed();
    let grant = f.grant();
    let op = owner
        .begin_key_operation(&receipt, &DIGEST, request(&grant), 10)
        .unwrap();
    assert_eq!(
        other
            .begin_key_operation(&receipt, &DIGEST, request(&grant), 10)
            .err(),
        Some(CryptoError::Unauthorized)
    );
    assert_eq!(
        other.cancel_key_operation(&op),
        Err(CryptoError::Unauthorized)
    );
    assert_eq!(
        other.complete_key_operation(&op, &grant, 7, 5, 11).err(),
        Some(CryptoError::Unauthorized)
    );
    assert_eq!(owner.pending_key_operations(), 1);
    owner.cancel_key_operation(&op).unwrap();
}

#[test]
fn provider_scope_cell_and_generation_are_checked_before_acquiring() {
    let f = Fixture::new(1, 1);
    let (mut owner, _, _, receipt) = f.installed();
    let grant = f.grant();
    for wrong in [
        crate::crypto::network::key_provider::ProviderGrant {
            revoked: true,
            ..grant
        },
        crate::crypto::network::key_provider::ProviderGrant {
            authority_generation: 8,
            ..grant
        },
        crate::crypto::network::key_provider::ProviderGrant {
            cell_epoch: 6,
            ..grant
        },
        crate::crypto::network::key_provider::ProviderGrant {
            deadline_unix: 10,
            ..grant
        },
    ] {
        assert!(owner
            .begin_key_operation(&receipt, &DIGEST, request(&wrong), 10)
            .is_err());
    }
    let mut wrong = grant;
    wrong.epoch.purpose = KeyPurpose::RouteUpdate;
    assert!(owner
        .begin_key_operation(&receipt, &DIGEST, request(&wrong), 10)
        .is_err());
    wrong = grant;
    wrong.epoch.controller = 99;
    assert!(owner
        .begin_key_operation(&receipt, &DIGEST, request(&wrong), 10)
        .is_err());
    wrong = grant;
    wrong.epoch.epoch = 2;
    assert!(owner
        .begin_key_operation(&receipt, &DIGEST, request(&wrong), 10)
        .is_err());
    assert!(owner
        .begin_key_operation(&receipt, &[7; 48], request(&grant), 10)
        .is_err());
    assert_eq!(owner.pending_key_operations(), 0);
}

#[test]
fn failed_completion_consumes_operation_and_rechecks_provider_and_deadline() {
    let f = Fixture::new(1, 1);
    let (mut owner, _, _, receipt) = f.installed();
    let grant = f.grant();
    let op = owner
        .begin_key_operation(&receipt, &DIGEST, request(&grant), 10)
        .unwrap();
    let mut changed = grant;
    changed.authority_generation = 8;
    assert_eq!(
        owner.complete_key_operation(&op, &changed, 8, 5, 11).err(),
        Some(CryptoError::StaleGeneration)
    );
    assert_eq!(owner.pending_key_operations(), 0);
    assert!(owner.complete_key_operation(&op, &grant, 7, 5, 11).is_err());
    let op = owner
        .begin_key_operation(&receipt, &DIGEST, request(&grant), 10)
        .unwrap();
    changed = grant;
    changed.revoked = true;
    assert_eq!(
        owner.complete_key_operation(&op, &changed, 7, 5, 11).err(),
        Some(CryptoError::Revoked)
    );
    let mut short = request(&grant);
    short.deadline_unix = 11;
    let op = owner
        .begin_key_operation(&receipt, &DIGEST, short, 10)
        .unwrap();
    assert_eq!(
        owner.complete_key_operation(&op, &grant, 7, 5, 11).err(),
        Some(CryptoError::Expired)
    );
    assert_eq!(owner.pending_key_operations(), 0);
}

#[test]
fn pending_capacity_is_bounded_and_cancel_recovers_all_slots() {
    let f = Fixture::new(1, 1);
    let (mut owner, _, _, receipt) = f.installed();
    let grant = f.grant();
    let mut ops = Vec::new();
    for _ in 0..32 {
        ops.push(
            owner
                .begin_key_operation(&receipt, &DIGEST, request(&grant), 10)
                .unwrap(),
        );
    }
    assert_eq!(
        owner
            .begin_key_operation(&receipt, &DIGEST, request(&grant), 10)
            .err(),
        Some(CryptoError::Capacity)
    );
    for op in ops {
        owner.cancel_key_operation(&op).unwrap();
    }
    assert_eq!(owner.pending_key_operations(), 0);
}
