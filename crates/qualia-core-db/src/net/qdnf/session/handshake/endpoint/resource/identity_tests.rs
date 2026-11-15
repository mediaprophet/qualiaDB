use super::*;
use crate::net::peer::runtime::{ReservationLedger, ResourceBudget};
use std::sync::Mutex;
fn owner() -> Mutex<ReservationLedger> {
    let c = ResourceBudget {
        bytes: 2 * ENDPOINT_RESERVATION_BYTES,
        work: 64,
        io: 64,
    };
    Mutex::new(ReservationLedger::new(c, c, c, c, c))
}
#[test]
fn drop_never_refunds_reconstructed_same_address_equal_reservation() {
    let ledger = owner();
    let original = EndpointReservation::admit(&ledger).unwrap();
    let old = std::mem::replace(&mut *ledger.lock().unwrap(), owner().into_inner().unwrap());
    let fake = EndpointReservation::admit(&ledger).unwrap();
    drop(original);
    assert_eq!(
        ledger.lock().unwrap().used().host.bytes,
        ENDPOINT_RESERVATION_BYTES
    );
    assert_eq!(old.used().host.bytes, ENDPOINT_RESERVATION_BYTES);
    drop(fake);
    assert_eq!(ledger.lock().unwrap().used().host.bytes, 0);
}
#[test]
fn retained_refuses_replacement_then_releases_exact_restored_original() {
    let ledger = owner();
    let mut original = EndpointReservation::admit(&ledger).unwrap();
    let mut retained = original.retain_for_channel().unwrap();
    drop(original);
    let old = std::mem::replace(&mut *ledger.lock().unwrap(), owner().into_inner().unwrap());
    let fake = EndpointReservation::admit(&ledger).unwrap();
    assert_eq!(retained.with_ledger(|_| Ok(())), Err(E::StaleGeneration));
    assert_eq!(retained.release_after_shutdown(), Err(E::StaleGeneration));
    assert_eq!(
        ledger.lock().unwrap().used().host.bytes,
        ENDPOINT_RESERVATION_BYTES
    );
    drop(fake);
    let _replaced = std::mem::replace(&mut *ledger.lock().unwrap(), old);
    retained.with_ledger(|_| Ok(())).unwrap();
    retained.release_after_shutdown().unwrap();
    assert_eq!(ledger.lock().unwrap().used().host.bytes, 0);
}
