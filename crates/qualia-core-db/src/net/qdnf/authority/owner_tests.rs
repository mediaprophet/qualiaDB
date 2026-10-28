use super::*;
use crate::net::qdnf::authority::DecodedClaim;
use crate::net::qdnf::authority::PolicyOutcome;
use crate::net::qdnf::types::OperationId;

#[test]
fn decoded_allow_cannot_issue_permit() {
    let owner = AuthorityOwner::new();
    let mut claim = DecodedClaim::empty();
    claim.asserted_outcome = PolicyOutcome::Allow;
    claim.signed_flag = true;
    assert_eq!(claim.admit_service(), Err(QdnfError::Unauthorized));
    let _ = owner;
}

#[test]
fn stale_handle_is_rejected() {
    let mut owner = AuthorityOwner::new();
    let binding =
        binding_for_controllers(b"did:q42:a", b"did:q42:b", b"q42:QSync/1", b"op-a").unwrap();
    let (_c, _k, handle) = owner
        .install_grant(binding, 10, 100, ContactState::Active)
        .unwrap();
    let mut stale = handle;
    stale.generation = Generation(99);
    assert_eq!(
        owner.issue_permit(stale, binding, 20, 256),
        Err(QdnfError::StaleGeneration)
    );
}

#[test]
fn revoked_grant_cannot_issue_permit() {
    let mut owner = AuthorityOwner::new();
    let binding =
        binding_for_controllers(b"did:q42:a", b"did:q42:b", b"q42:QSync/1", b"op-a").unwrap();
    let (_c, _k, handle) = owner
        .install_grant(binding, 10, 100, ContactState::Active)
        .unwrap();
    owner.revoke(handle).unwrap();
    assert_eq!(
        owner.issue_permit(handle, binding, 20, 256),
        Err(QdnfError::Revoked)
    );
}

#[test]
fn current_grant_issues_permit() {
    let mut owner = AuthorityOwner::new();
    let binding =
        binding_for_controllers(b"did:q42:a", b"did:q42:b", b"q42:QSync/1", b"op-a").unwrap();
    let (cred, contact, handle) = owner
        .install_grant(binding, 10, 100, ContactState::Active)
        .unwrap();
    let permit = owner.issue_permit(handle, binding, 20, 256).unwrap();
    assert!(permit.matches_credential(&cred).is_ok());
    assert!(permit.matches_contact(&contact).is_ok());
    assert_ne!(permit.operation(), OperationId::ZERO);
}

#[test]
fn installed_scope_and_operation_cannot_be_rebound_at_consumption() {
    let binding = binding_for_controllers(b"principal", b"recipient", b"purpose", b"edit").unwrap();
    let mut owner = AuthorityOwner::new();
    let (_, _, handle) = owner
        .install_grant(binding, 10, 100, ContactState::Active)
        .unwrap();
    let mut other_scope = binding;
    other_scope.scope = sha384(b"other-document");
    assert_eq!(
        owner.issue_permit(handle, other_scope, 20, 1),
        Err(QdnfError::Unauthorized)
    );
    let mut other_operation = binding;
    other_operation.operation = OperationId([0x77; 16]);
    assert_eq!(
        owner.issue_permit(handle, other_operation, 20, 1),
        Err(QdnfError::Unauthorized)
    );
    assert!(owner.issue_permit(handle, binding, 20, 1).is_ok());
}

#[test]
fn bounded_admission_preserves_ceiling_and_exact_time_window() {
    let binding = binding_for_controllers(b"principal", b"recipient", b"purpose", b"edit").unwrap();
    let mut owner = AuthorityOwner::new();
    let (_, _, handle) = owner
        .install_grant_with_ceiling(binding, 10, 100, ContactState::Active, 16)
        .unwrap();
    assert_eq!(owner.current_grant(handle).unwrap().not_before_unix, 10);
    assert_eq!(
        owner.issue_permit(handle, binding, 9, 1),
        Err(QdnfError::Denied)
    );
    assert_eq!(
        owner.issue_permit(handle, binding, 100, 1),
        Err(QdnfError::Expired)
    );
    assert_eq!(
        owner.issue_permit(handle, binding, 20, 17),
        Err(QdnfError::BudgetExhausted)
    );
    assert_eq!(
        owner.issue_permit(handle, binding, 20, 0),
        Err(QdnfError::Capacity)
    );
    let permit = owner.issue_permit(handle, binding, 10, 16).unwrap();
    assert_eq!(permit.resource_ceiling(), 16);
    // A ceiling limits each permit. It is deliberately not a usage/debit ledger.
    assert!(owner.issue_permit(handle, binding, 99, 16).is_ok());
}

#[test]
fn rejected_admission_does_not_spend_capacity_or_generation() {
    let binding = binding_for_controllers(b"principal", b"recipient", b"purpose", b"edit").unwrap();
    let mut owner = AuthorityOwner::new();
    let mut invalid = binding;
    invalid.scope = StrongDigest::ZERO;
    for _ in 0..16 {
        assert_eq!(
            owner.install_grant(invalid, 10, 100, ContactState::Active),
            Err(QdnfError::Unauthorized)
        );
        assert_eq!(
            owner.install_grant_with_ceiling(binding, 10, 100, ContactState::Active, 0),
            Err(QdnfError::Capacity)
        );
    }
    for index in 0..MAX_RECORDS {
        let (_, _, handle) = owner
            .install_grant_with_ceiling(binding, 10, 100, ContactState::Active, 1)
            .unwrap();
        let permit = owner.issue_permit(handle, binding, 20, 1).unwrap();
        assert_eq!(permit.policy_generation(), Generation((index + 1) as u64));
    }
    assert_eq!(
        owner.install_grant(binding, 10, 100, ContactState::Active),
        Err(QdnfError::Capacity)
    );
}

#[test]
fn invalid_binding_and_self_contact_refuse_before_install() {
    let binding = binding_for_controllers(b"principal", b"recipient", b"purpose", b"edit").unwrap();
    let mut owner = AuthorityOwner::new();
    for index in 0..6 {
        let mut invalid = binding;
        match index {
            0 => invalid.principal = StrongDigest::ZERO,
            1 => invalid.recipient = StrongDigest::ZERO,
            2 => invalid.instrument = StrongDigest::ZERO,
            3 => invalid.scope = StrongDigest::ZERO,
            4 => invalid.purpose = StrongDigest::ZERO,
            _ => invalid.operation = OperationId::ZERO,
        }
        assert_eq!(
            owner.install_grant(invalid, 10, 100, ContactState::Active),
            Err(QdnfError::Unauthorized)
        );
    }
    let mut self_contact = binding;
    self_contact.recipient = self_contact.principal;
    assert_eq!(
        owner.install_grant(self_contact, 10, 100, ContactState::Active),
        Err(QdnfError::Conflict)
    );
}

#[test]
fn legacy_ceiling_compatibility_does_not_drop_scope_or_revocation() {
    let binding = binding_for_controllers(b"principal", b"recipient", b"purpose", b"edit").unwrap();
    let mut owner = AuthorityOwner::new();
    let (_, _, handle) = owner
        .install_grant(binding, 10, 100, ContactState::Active)
        .unwrap();
    assert_eq!(
        owner
            .issue_permit(handle, binding, 20, u64::MAX)
            .unwrap()
            .resource_ceiling(),
        u64::MAX
    );
    owner.revoke(handle).unwrap();
    assert_eq!(
        owner.issue_permit(handle, binding, 20, 1),
        Err(QdnfError::Revoked)
    );
}
