//! Actual managed received-Finished fixture, not a new key/budget factory.
use super::*;
use crate::bundle::stream::runtime::{RuntimeParents, RuntimeProfile};
use crate::net::peer::runtime::{FairScheduler, LeaseTable, OperationTable, ResourceGovernor};
fn small_control() -> ResourceBudget {
    ResourceBudget {
        bytes: 4096,
        work: 1,
        io: 1,
    }
}
#[test]
fn channel_and_existing_runtime_share_exact_locked_ledger_without_relock_or_new_charge() {
    with_pair(|mut p| {
        let mut leases = Mutex::new(LeaseTable::new());
        let mut ops = Mutex::new(OperationTable::new());
        let mut scheduler = Mutex::new(FairScheduler::new());
        let governor = ResourceGovernor::new(4096);
        let mut parents =
            RuntimeParents::adopt(p.li, &mut leases, &mut ops, &mut scheduler, &governor).unwrap();
        let mut state = parents
            .admit(RuntimeProfile::Constrained, small_control())
            .unwrap();
        let before = p.li.lock().unwrap().used();
        let a = aad(p.i.transcript(), 22, b"bridge");
        parents
            .try_with_runtime(&mut state, |access| {
                p.i.with_runtime_scope(access, |mut io, rt| {
                    assert!(p.li.try_lock().is_err());
                    assert_eq!(rt.parent_used(), before);
                    io.with_current(&p.f.pins_i, &now, |mut frame, current| {
                        frame.queue_record(current, &p.f.pins_i, &now, &a, b"bridge")?;
                        frame.send_pending(current, &p.f.pins_i, &now)
                    })
                })
            })
            .unwrap();
        assert_eq!(p.li.lock().unwrap().used(), before);
        let mut out = [0; 1024];
        assert_eq!(receive(&mut p.r, &p.f.pins_r, &mut out), Ok(6));
        assert_eq!(&out[..6], b"bridge");
        state.try_close().unwrap();
        parents.try_close().unwrap();
        assert_eq!(
            p.li.lock().unwrap().used().host.bytes,
            ENDPOINT_RESERVATION_BYTES
        );
        p.i.try_close().unwrap();
        p.r.try_close().unwrap();
    });
}
#[test]
fn foreign_actual_ledger_scope_refuses_without_destroying_original_channel() {
    with_pair(|mut p| {
        let mut leases = Mutex::new(LeaseTable::new());
        let mut ops = Mutex::new(OperationTable::new());
        let mut scheduler = Mutex::new(FairScheduler::new());
        let governor = ResourceGovernor::new(4096);
        let mut other =
            RuntimeParents::adopt(p.lr, &mut leases, &mut ops, &mut scheduler, &governor).unwrap();
        let mut state = other
            .admit(RuntimeProfile::Constrained, small_control())
            .unwrap();
        assert_eq!(
            other.try_with_runtime(&mut state, |access| p
                .i
                .with_runtime_scope(access, |_, _| Ok(()))),
            Err(E::StaleGeneration)
        );
        state.try_close().unwrap();
        other.try_close().unwrap();
        queue(&mut p.i, &p.f.pins_i, 23, b"correct").unwrap();
        send(&mut p.i, &p.f.pins_i).unwrap();
        let mut out = [0; 1024];
        assert_eq!(receive(&mut p.r, &p.f.pins_r, &mut out), Ok(7));
        p.i.try_close().unwrap();
        p.r.try_close().unwrap();
    });
}
#[test]
fn original_channel_charge_cannot_attach_to_reconstructed_ledger_before_new_registration() {
    with_pair(|mut p| {
        let cap = ResourceBudget {
            bytes: 2 * ENDPOINT_RESERVATION_BYTES,
            work: 64,
            io: 64,
        };
        let replacement = ReservationLedger::new(cap, cap, cap, cap, cap);
        let original = std::mem::replace(&mut *p.li.lock().unwrap(), replacement);
        let fake = EndpointReservation::admit(p.li).unwrap(); // same slot/gen/full budget, DIFFERENT value ID
        let mut leases = Mutex::new(LeaseTable::new());
        let mut ops = Mutex::new(OperationTable::new());
        let mut scheduler = Mutex::new(FairScheduler::new());
        let governor = ResourceGovernor::new(4096);
        let mut newer =
            RuntimeParents::adopt(p.li, &mut leases, &mut ops, &mut scheduler, &governor).unwrap();
        let mut state = newer
            .admit(RuntimeProfile::Constrained, small_control())
            .unwrap();
        assert_eq!(
            newer.try_with_runtime(&mut state, |access| p
                .i
                .with_runtime_scope(access, |_, _| Ok(()))),
            Err(E::StaleGeneration)
        );
        state.try_close().unwrap();
        newer.try_close().unwrap();
        drop(fake);
        let _replaced = std::mem::replace(&mut *p.li.lock().unwrap(), original);
        p.i.try_close().unwrap();
        p.r.try_close().unwrap();
        assert_eq!(p.li.lock().unwrap().used().host.bytes, 0);
    });
}
