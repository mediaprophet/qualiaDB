use super::*;
use crate::net::peer::runtime::ResourceBudget;
fn ledger() -> Mutex<ReservationLedger> {
    let cap = ResourceBudget {
        bytes: 2 * ENDPOINT_RESERVATION_BYTES,
        work: 64,
        io: 64,
    };
    Mutex::new(ReservationLedger::new(cap, cap, cap, cap, cap))
}
#[test]
fn exact_handoff_has_no_release_readmit_gap_and_scope_is_actual_ledger() {
    let ledger = ledger();
    let mut original = EndpointReservation::admit(&ledger).unwrap();
    let before = ledger.lock().unwrap().used();
    let mut retained = original.retain_for_channel().unwrap();
    assert_eq!(ledger.lock().unwrap().used(), before);
    retained
        .with_ledger(|actual| {
            assert_eq!(actual.used(), before);
            assert_eq!(actual.available_handles(), 31);
            assert!(matches!(ledger.try_lock(), Err(TryLockError::WouldBlock)));
            Ok(())
        })
        .unwrap();
    retained.release_after_shutdown().unwrap();
    assert_eq!(ledger.lock().unwrap().used().host.bytes, 0);
    assert_eq!(retained.release_after_shutdown(), Err(E::DoubleRelease));
}
#[test]
fn failed_release_and_contended_scope_retain_exact_charge_for_retry() {
    let ledger = ledger();
    let mut retained = EndpointReservation::admit(&ledger)
        .unwrap()
        .retain_for_channel()
        .unwrap();
    let held = ledger.lock().unwrap();
    assert_eq!(retained.with_ledger(|_| Ok(())), Err(E::WouldBlock));
    assert_eq!(retained.release_after_shutdown(), Err(E::WouldBlock));
    assert_eq!(held.used().host.bytes, ENDPOINT_RESERVATION_BYTES);
    drop(held);
    retained.release_after_shutdown().unwrap();
    assert_eq!(ledger.lock().unwrap().available_handles(), 32);
}
#[test]
fn dropping_or_unwinding_lost_owner_keeps_canonical_charge() {
    let ledger = ledger();
    let retained = EndpointReservation::admit(&ledger)
        .unwrap()
        .retain_for_channel()
        .unwrap();
    drop(retained);
    assert_eq!(
        ledger.lock().unwrap().used().host.bytes,
        ENDPOINT_RESERVATION_BYTES
    );
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _retained = EndpointReservation::admit(&ledger)
            .unwrap()
            .retain_for_channel()
            .unwrap();
        panic!("retained owner unwind");
    }));
    assert!(outcome.is_err());
    assert_eq!(
        ledger.lock().unwrap().used().host.bytes,
        2 * ENDPOINT_RESERVATION_BYTES
    );
}

#[test]
fn empty_handoff_refusal_preserves_original_owner_and_exact_retained_charge() {
    let ledger = ledger();
    let mut original = EndpointReservation::admit(&ledger).unwrap();
    let mut retained = original.retain_for_channel().unwrap();
    assert!(matches!(original.retain_for_channel(), Err(E::Closed)));
    // The refused original still exists and its Drop cannot release retained ownership.
    drop(original);
    assert_eq!(
        ledger.lock().unwrap().used().host.bytes,
        ENDPOINT_RESERVATION_BYTES
    );
    retained.release_after_shutdown().unwrap();
    assert_eq!(ledger.lock().unwrap().used().host.bytes, 0);
}
