use super::super::*;
fn binding() -> AuthorityBinding {
    binding_for_controllers(b"principal", b"recipient", b"purpose", b"op").unwrap()
}
#[test]
fn full_owner_requires_explicit_revoke_retire_and_monotonic_replacement() {
    let mut owner = AuthorityOwner::new();
    let binding = binding();
    let (_, _, first) = owner
        .install_grant(binding, 10, 100, ContactState::Active)
        .unwrap();
    let old_permit = owner.issue_permit(first, binding, 20, 1).unwrap();
    for _ in 1..MAX_RECORDS {
        owner
            .install_grant(binding, 10, 100, ContactState::Active)
            .unwrap();
    }
    assert_eq!(owner.retire_revoked(first), Err(QdnfError::Denied));
    assert_eq!(
        owner.install_grant(binding, 10, 100, ContactState::Active),
        Err(QdnfError::Capacity)
    );
    owner.revoke(first).unwrap();
    assert_eq!(
        owner.install_grant(binding, 10, 100, ContactState::Active),
        Err(QdnfError::Capacity)
    );
    owner.retire_revoked(first).unwrap();
    assert_eq!(owner.retire_revoked(first), Err(QdnfError::Incomplete));
    assert_eq!(
        owner.issue_permit(first, binding, 20, 1),
        Err(QdnfError::Incomplete)
    );
    let (_, _, replacement) = owner
        .install_grant(binding, 10, 100, ContactState::Active)
        .unwrap();
    assert_eq!(replacement.slot, first.slot);
    assert_eq!(replacement.principal, first.principal);
    assert_eq!(replacement.generation, Generation((MAX_RECORDS + 1) as u64));
    assert_eq!(
        owner.issue_permit(first, binding, 20, 1),
        Err(QdnfError::StaleGeneration)
    );
    assert_eq!(owner.revoke(first), Err(QdnfError::StaleGeneration));
    assert_eq!(owner.retire_revoked(first), Err(QdnfError::StaleGeneration));
    assert_eq!(
        replacement.matches_permit(&old_permit),
        Err(QdnfError::StaleGeneration)
    );
    assert!(owner.issue_permit(replacement, binding, 20, 1).is_ok());
}
#[test]
fn retirement_does_not_reset_exhausted_global_generation() {
    let mut owner = AuthorityOwner::new();
    let binding = binding();
    let (_, _, handle) = owner
        .install_grant(binding, 10, 100, ContactState::Active)
        .unwrap();
    owner.next_generation = Generation(u64::MAX);
    owner.revoke(handle).unwrap();
    owner.retire_revoked(handle).unwrap();
    assert_eq!(owner.next_generation, Generation(u64::MAX));
    assert_eq!(
        owner.install_grant(binding, 10, 100, ContactState::Active),
        Err(QdnfError::StaleGeneration)
    );
    assert!(owner.records.iter().all(|r| !r.occupied));
}
