use super::*;
use crate::net::qdnf::{
    authority::ContactState,
    types::{OperationId, StrongDigest},
};
fn binding() -> AuthorityBinding {
    AuthorityBinding {
        principal: StrongDigest([1; 48]),
        recipient: StrongDigest([2; 48]),
        purpose: StrongDigest([3; 48]),
        instrument: StrongDigest([4; 48]),
        scope: StrongDigest([5; 48]),
        operation: OperationId([6; 16]),
    }
}
fn owner() -> (Mutex<AuthorityOwner>, MutationHandle) {
    let mut owner = AuthorityOwner::new();
    let (_, _, handle) = owner
        .install_grant_with_ceiling(binding(), 10, 20, ContactState::Active, 128)
        .unwrap();
    (Mutex::new(owner), handle)
}
#[test]
fn exact_actual_owner_affinity_even_when_records_collide() {
    let (a, ha) = owner();
    let (b, hb) = owner();
    let current = CurrentAuthorityGuard::try_lock(&a).unwrap();
    assert_eq!(current.check_handle(&a, ha, binding(), 11, 128), Ok(()));
    assert_eq!(
        current.check_handle(&b, hb, binding(), 11, 128),
        Err(E::Unauthorized)
    );
    assert!(matches!(
        CurrentAuthorityGuard::try_lock(&a),
        Err(E::WouldBlock)
    ));
    drop(current);
    assert!(CurrentAuthorityGuard::try_lock(&a).is_ok());
}
#[test]
fn same_guard_enforces_time_ceiling_binding_and_revoke() {
    let (a, h) = owner();
    let mut current = CurrentAuthorityGuard::try_lock(&a).unwrap();
    assert!(current.check_handle(&a, h, binding(), 9, 128).is_err());
    assert_eq!(
        current.check_handle(&a, h, binding(), 20, 128),
        Err(E::Expired)
    );
    assert!(current.check_handle(&a, h, binding(), 11, 129).is_err());
    let mut wrong = binding();
    wrong.scope = StrongDigest([7; 48]);
    assert!(current.check_handle(&a, h, wrong, 11, 128).is_err());
    current.revoke_handle(&a, h).unwrap();
    assert_eq!(
        current.check_handle(&a, h, binding(), 11, 128),
        Err(E::Revoked)
    );
}
#[test]
fn pure_checks_have_zero_raw_allocations_with_positive_control() {
    use crate::specialized_libs::computational_geometry::allocation_counter::AllocGuard;
    let (a, h) = owner();
    let current = CurrentAuthorityGuard::try_lock(&a).unwrap();
    current.check_handle(&a, h, binding(), 11, 128).unwrap();
    let measure = AllocGuard::begin("actual current guard", true);
    for _ in 0..16 {
        current.check_handle(&a, h, binding(), 11, 128).unwrap();
    }
    measure.check().unwrap();
    let positive = AllocGuard::begin("current guard positive", true);
    std::hint::black_box(vec![1u8; 8]);
    assert!(positive.check().is_err());
}
