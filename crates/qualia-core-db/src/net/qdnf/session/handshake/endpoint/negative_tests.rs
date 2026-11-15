use super::*;
#[test]
fn cancellation_and_unwind_clear_retained_ephemerals_and_release_reservation() {
    let f = Fixture::new();
    let owner = owner();
    let ledger = ledger();
    let authority = EndpointAuthority::adopt_local(
        &owner,
        f.intent,
        Role::Initiator,
        now(),
        ENDPOINT_RESERVATION_BYTES,
    )
    .unwrap();
    let (mut a, b) = ipc_pair(ScopeEpoch { scope: 1, epoch: 1 }, 1500).unwrap();
    let route = Route::observed(&a, b.locator()).unwrap();
    let mut workspace = EndpointWorkspace::new();
    let mut endpoint = Initiator::new(
        &authority,
        EndpointReservation::admit(&ledger).unwrap(),
        &mut workspace,
        route,
        f.peer_i,
        &now,
    )
    .unwrap();
    let commitment = endpoint.signing_commitment().unwrap();
    endpoint
        .sign_client(f.initiator.capability(&commitment, &now), &f.pins_i, &now)
        .unwrap();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        endpoint.send_step(&mut a, &f.pins_i, &|| panic!("clock failure"))
    }));
    assert!(result.is_err());
    assert_eq!(endpoint.phase(), Phase::Failed);
    assert_eq!(ledger.lock().unwrap().used().host.bytes, 0);
    assert!(endpoint.common.workspace.io.iter().all(|b| *b == 0));
}
#[test]
fn local_revoke_before_send_and_late_signature_fail_closed() {
    let f = Fixture::new();
    let owner = owner();
    let ledger = ledger();
    let authority = EndpointAuthority::adopt_local(
        &owner,
        f.intent,
        Role::Initiator,
        now(),
        ENDPOINT_RESERVATION_BYTES,
    )
    .unwrap();
    let (mut a, b) = ipc_pair(ScopeEpoch { scope: 2, epoch: 1 }, 1500).unwrap();
    let route = Route::observed(&a, b.locator()).unwrap();
    let mut workspace = EndpointWorkspace::new();
    let mut endpoint = Initiator::new(
        &authority,
        EndpointReservation::admit(&ledger).unwrap(),
        &mut workspace,
        route,
        f.peer_i,
        &now,
    )
    .unwrap();
    let c = endpoint.signing_commitment().unwrap();
    let cap = f.initiator.capability(&c, &now);
    f.initiator.control.cancel().unwrap();
    assert!(endpoint.sign_client(cap, &f.pins_i, &now).is_err());
    assert_eq!(endpoint.phase(), Phase::Failed);
    endpoint.cancel();
    assert_eq!(
        endpoint.send_step(&mut a, &f.pins_i, &now),
        Err(E::Unauthorized)
    );
    authority.revoke().unwrap();
}
#[test]
fn resource_budget_and_early_install_refuse() {
    let f = Fixture::new();
    let owner = owner();
    let ledger = ledger();
    let authority = EndpointAuthority::adopt_local(
        &owner,
        f.intent,
        Role::Responder,
        now(),
        ENDPOINT_RESERVATION_BYTES,
    )
    .unwrap();
    let (a, b) = ipc_pair(ScopeEpoch { scope: 3, epoch: 1 }, 1280).unwrap();
    let route = Route::observed(&b, a.locator()).unwrap();
    let mut workspace = EndpointWorkspace::new();
    let mut endpoint = Responder::new(
        &authority,
        EndpointReservation::admit(&ledger).unwrap(),
        &mut workspace,
        route,
        f.peer_r,
        &now,
    )
    .unwrap();
    assert!(endpoint.install(&f.pins_r, &now).is_err());
    assert_eq!(endpoint.phase(), Phase::Failed);
    let zero = ResourceBudget::ZERO;
    let empty = Mutex::new(ReservationLedger::new(zero, zero, zero, zero, zero));
    assert!(EndpointReservation::admit(&empty).is_err());
}
