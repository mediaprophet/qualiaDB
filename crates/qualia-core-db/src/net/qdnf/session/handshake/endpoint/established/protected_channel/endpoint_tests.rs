//! Routed by endpoint/tests.rs to reuse its original private managed fixture.
use super::*;
use crate::net::qdnf::session::handshake::endpoint::ProtectedChannel;
#[path = "endpoint_fixtures.rs"]
mod fixtures;
use fixtures::*;

#[test]
fn persistent_packet_numbers_received_bytes_retry_and_actual_owner_scope() {
    with_pair(|mut p| {
        let extra = EndpointReservation::admit(p.li).unwrap();
        p.i.with_resources(|mut io, actual| {
            assert_eq!(actual.available_handles(), 30);
            assert!(p.li.try_lock().is_err());
            io.with_current(&p.f.pins_i, &now, |_, _| Ok(()))?;
            // The actual authority guard is GONE between effects, despite ledger scope.
            assert!(p.oi.try_lock().is_ok());
            Ok(())
        })
        .unwrap();
        drop(extra);
        queue(&mut p.i, &p.f.pins_i, 1, b"first").unwrap();
        p.ti.lock().unwrap().blocks = 1;
        assert_eq!(send(&mut p.i, &p.f.pins_i), Err(E::WouldBlock));
        let before = {
            let s = p.ti.lock().unwrap();
            (s.capture_len, s.capture)
        };
        assert_eq!(p.i.pending_frame_bytes(), before.0);
        // An attempt to overwrite the pending record must not reseal/burn PN.
        assert_eq!(
            queue(&mut p.i, &p.f.pins_i, 2, b"different"),
            Err(E::WouldBlock)
        );
        send(&mut p.i, &p.f.pins_i).unwrap();
        {
            let s = p.ti.lock().unwrap();
            assert_eq!((s.capture_len, s.capture), before);
            assert_eq!(pn(&s), 0);
        }
        let mut out = [0; 1024];
        assert_eq!(receive(&mut p.r, &p.f.pins_r, &mut out), Ok(5));
        assert_eq!(&out[..5], b"first");
        queue(&mut p.i, &p.f.pins_i, 2, b"second").unwrap();
        send(&mut p.i, &p.f.pins_i).unwrap();
        assert_eq!(pn(&p.ti.lock().unwrap()), 1);
        assert_eq!(receive(&mut p.r, &p.f.pins_r, &mut out), Ok(6));
        assert_eq!(&out[..6], b"second");
        p.i.try_close().unwrap();
        p.r.try_close().unwrap();
        assert_eq!(p.li.lock().unwrap().used().host.bytes, 0);
        assert_eq!(p.lr.lock().unwrap().used().host.bytes, 0);
        assert_eq!(p.i.try_close(), Err(E::DoubleRelease));
    });
}
#[test]
fn actual_inter_frame_revoke_prevents_queued_frame_send_without_refund() {
    with_pair(|mut p| {
        queue(&mut p.i, &p.f.pins_i, 3, b"not sent").unwrap();
        p.ai.revoke().unwrap(); // actual same owner acquired after prior frame scope
        assert_eq!(send(&mut p.i, &p.f.pins_i), Err(E::Revoked));
        assert_eq!(p.ti.lock().unwrap().sent, 0);
        assert_eq!(p.i.pending_frame_bytes(), 0);
        assert_eq!(
            p.li.lock().unwrap().used().host.bytes,
            ENDPOINT_RESERVATION_BYTES
        );
        p.i.try_close().unwrap();
        p.r.try_close().unwrap();
    });
}
#[test]
fn shutdown_and_release_refusals_keep_same_owner_charge_for_retry() {
    with_pair(|mut p| {
        p.ti.lock().unwrap().shutdown_blocks = 1;
        assert_eq!(p.i.try_close(), Err(E::WouldBlock));
        assert_eq!(p.li.lock().unwrap().available_handles(), 31);
        let held = p.li.lock().unwrap();
        // Shutdown now succeeds but canonical ledger is temporarily unavailable.
        assert_eq!(p.i.try_close(), Err(E::WouldBlock));
        assert_eq!(held.available_handles(), 31);
        drop(held);
        p.i.try_close().unwrap();
        assert_eq!(p.ti.lock().unwrap().shutdown_calls, 2);
        assert_eq!(p.li.lock().unwrap().available_handles(), 32);
        p.r.try_close().unwrap();
    });
}
#[test]
fn dropped_channel_closes_actual_bearer_but_retains_uncertain_charge() {
    with_pair(|mut p| {
        drop(p.i);
        assert_eq!(p.ti.lock().unwrap().shutdown_calls, 1);
        assert_eq!(
            p.li.lock().unwrap().used().host.bytes,
            ENDPOINT_RESERVATION_BYTES
        );
        p.r.try_close().unwrap();
    });
}
#[test]
fn caught_frame_error_is_terminal_and_cannot_reveal_or_send() {
    with_pair(|mut p| {
        let mut a = aad(p.i.transcript(), 4, b"x");
        a[87] = 0;
        let result = p.i.with_resources(|mut io, _| {
            io.with_current(&p.f.pins_i, &now, |mut frame, current| {
                assert_eq!(
                    frame.queue_record(current, &p.f.pins_i, &now, &a, b"x"),
                    Err(E::Malformed)
                );
                Ok(()) // cannot hide terminal failure from enclosing current gate
            })
        });
        assert_eq!(result, Err(E::Closed));
        assert_eq!(p.i.pending_frame_bytes(), 0);
        assert_eq!(send(&mut p.i, &p.f.pins_i), Err(E::Closed));
        p.i.try_close().unwrap();
        p.r.try_close().unwrap();
    });
}
#[test]
fn callback_unwind_erases_channel_and_preserves_unresolved_parent_charge() {
    with_pair(|mut p| {
        queue(&mut p.i, &p.f.pins_i, 5, b"pending").unwrap();
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _: Result<(), E> = p.i.with_resources(|_, _| panic!("bounded callback unwind"));
        }));
        assert!(outcome.is_err());
        assert_eq!(p.i.pending_frame_bytes(), 0);
        let held = p.li.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(held.used().host.bytes, ENDPOINT_RESERVATION_BYTES);
        drop(held);
        assert_eq!(p.i.try_close(), Err(E::Unauthorized));
        p.r.try_close().unwrap();
    });
}
#[path = "endpoint_allocation_tests.rs"]
mod allocation_tests;
#[path = "endpoint_receive_tests.rs"]
mod receive_tests;

#[path = "endpoint_runtime_tests.rs"]
mod runtime_tests;
