use super::*;

#[test]
fn canonical_external_scheduler_record_is_returned_not_discarded() {
    let cap = budget(1024 * 1024);
    let mut ledger = ReservationLedger::new(cap, cap, cap, cap, cap);
    let mut leases = LeaseTable::new();
    let mut ops = OperationTable::new();
    let mut scheduler = FairScheduler::new();
    let external = ScheduledWork {
        operation: OperationId([250; 16]),
        scope: 99,
        class: WorkClass::Interactive,
        cost: 1,
        deadline: 100,
    };
    scheduler.enqueue(external).unwrap();
    let gov = ResourceGovernor::new(4096);
    let mut rt = TransferRuntime::new(
        &mut ledger,
        &mut leases,
        &mut ops,
        &mut scheduler,
        &gov,
        RuntimeProfile::Constrained,
        control(),
    )
    .unwrap();
    match rt.poll(0).unwrap() {
        RuntimePoll::External(crate::net::peer::runtime::PollOutcome::Ready(w)) => {
            assert_eq!(w, external)
        }
        _ => panic!("external work lost"),
    };
    rt.close().unwrap();
}
#[test]
fn exhausted_canonical_lease_table_rolls_back_constructor_charges() {
    let cap = budget(1024 * 1024);
    let mut ledger = ReservationLedger::new(cap, cap, cap, cap, cap);
    let mut leases = LeaseTable::new();
    let mut ops = OperationTable::new();
    let mut scheduler = FairScheduler::new();
    let gov = ResourceGovernor::new(4096);
    let mut held = Vec::new();
    for _ in 0..32 {
        held.push(leases.acquire(1, true).unwrap());
    }
    assert_eq!(
        TransferRuntime::new(
            &mut ledger,
            &mut leases,
            &mut ops,
            &mut scheduler,
            &gov,
            RuntimeProfile::Constrained,
            control()
        )
        .err(),
        Some(QdnfError::Capacity)
    );
    assert_eq!(ledger.used().host, ResourceBudget::ZERO);
    assert_eq!(leases.occupied_count(), 32);
    for l in held {
        leases.release(l.handle).unwrap();
    }
}
#[test]
fn essential_control_floor_cannot_be_borrowed_by_bulk() {
    let cap = budget(128 * 1024);
    let mut ledger = ReservationLedger::new(cap, cap, cap, cap, cap);
    let mut leases = LeaseTable::new();
    let mut ops = OperationTable::new();
    let mut scheduler = FairScheduler::new();
    let gov = ResourceGovernor::new(4096);
    let mut rt = TransferRuntime::new(
        &mut ledger,
        &mut leases,
        &mut ops,
        &mut scheduler,
        &gov,
        RuntimeProfile::Constrained,
        control(),
    )
    .unwrap();
    let base = rt.parent_used();
    let mut row = spec(8);
    row.requirements.consumer.bytes = cap.bytes - base.host.bytes - 1000;
    let rows = [row];
    let plan = ColdPlan::compile(&rows).unwrap();
    assert_eq!(
        rt.admit(&plan, 0, 0, Generation(1)).err(),
        Some(QdnfError::BudgetExhausted)
    );
    assert_eq!(rt.parent_used(), base);
    assert_eq!(rt.leased_slots(), 1);
    rt.close().unwrap();
}

#[test]
fn deliberately_colliding_and_owner_prefix_foreign_records_remain_external() {
    with_runtime(RuntimeProfile::Constrained, |rt| {
        let collision = ScheduledWork {
            operation: super::types::schedule_id(rt.owner, 1).unwrap(),
            scope: 77,
            class: WorkClass::Interactive,
            cost: 1,
            deadline: 100,
        };
        rt.scheduler.enqueue(collision).unwrap();
        let rows = [spec(9)];
        let plan = ColdPlan::compile(&rows).unwrap();
        let held = rt.parent_used();
        assert_eq!(
            rt.admit(&plan, 0, 0, Generation(1)).err(),
            Some(QdnfError::Conflict)
        );
        assert_eq!(rt.parent_used(), held);
        assert_eq!(rt.pending_schedule_records(), 0);
        match rt.poll(0).unwrap() {
            RuntimePoll::External(crate::net::peer::runtime::PollOutcome::Ready(w)) => {
                assert_eq!(w, collision)
            }
            _ => panic!("colliding record consumed"),
        };
        let foreign = ScheduledWork {
            operation: super::types::schedule_id(rt.owner, 999).unwrap(),
            ..collision
        };
        rt.scheduler.enqueue(foreign).unwrap();
        match rt.poll(0).unwrap() {
            RuntimePoll::External(crate::net::peer::runtime::PollOutcome::Ready(w)) => {
                assert_eq!(w, foreign)
            }
            _ => panic!("prefix record consumed"),
        };
        let mut ticket = rt.admit(&plan, 0, 0, Generation(1)).unwrap();
        let credit = ready(rt, &ticket);
        rt.complete_io(&mut ticket, credit.completion_key(), 0)
            .unwrap();
    });
}

#[test]
fn desktop_admits_thirty_with_distinct_scopes_then_preserves_control_slots() {
    with_runtime(RuntimeProfile::Desktop, |rt| {
        let rows: [TransferSpec; 31] = core::array::from_fn(|i| {
            let mut r = spec((i + 1) as u8);
            r.principal = (i / 8 + 1) as u64;
            r
        });
        let plan = ColdPlan::compile(&rows).unwrap();
        let mut tickets: [Option<TransferTicket>; 30] = core::array::from_fn(|_| None);
        for (i, t) in tickets.iter_mut().enumerate() {
            *t = Some(rt.admit(&plan, i, 0, Generation(1)).unwrap());
        }
        assert_eq!(rt.leased_slots(), 31);
        let held = rt.parent_used();
        assert_eq!(
            rt.admit(&plan, 30, 0, Generation(1)).err(),
            Some(QdnfError::Capacity)
        );
        assert_eq!(rt.parent_used(), held);
        for ticket in tickets.iter().flatten() {
            rt.cancel(ticket).unwrap();
        }
        for _ in 0..30 {
            assert!(matches!(
                rt.poll(0),
                Ok(RuntimePoll::Cancelled(_, CancelReason::Cancelled))
            ));
        }
        for ticket in tickets.iter_mut().flatten() {
            let key = ticket.completion_key();
            rt.complete_io(ticket, key, 0).unwrap();
        }
    });
}

#[test]
fn exhausted_canonical_ledger_rolls_back_static_constructor_handle() {
    let cap = budget(1024 * 1024);
    let mut ledger = ReservationLedger::new(cap, cap, cap, cap, cap);
    let mut leases = LeaseTable::new();
    let mut ops = OperationTable::new();
    let mut scheduler = FairScheduler::new();
    let gov = ResourceGovernor::new(4096);
    let mut held = Vec::new();
    for _ in 0..31 {
        held.push(
            ledger
                .reserve(
                    ResourceBudget {
                        bytes: 1,
                        work: 0,
                        io: 0,
                    },
                    true,
                )
                .unwrap(),
        );
    }
    let before = ledger.used();
    assert_eq!(
        TransferRuntime::new(
            &mut ledger,
            &mut leases,
            &mut ops,
            &mut scheduler,
            &gov,
            RuntimeProfile::Constrained,
            control()
        )
        .err(),
        Some(QdnfError::Capacity)
    );
    assert_eq!(ledger.used(), before);
    assert_eq!(leases.occupied_count(), 0);
    for token in held {
        ledger.release(token).unwrap();
    }
}
#[test]
fn hot_metadata_overflow_and_window_rejection_are_charge_free() {
    with_runtime(RuntimeProfile::Constrained, |rt| {
        let before = rt.parent_used();
        let mut row = spec(11);
        row.requirements.consumer.bytes = u64::MAX - 80;
        let rows = [row];
        let plan = ColdPlan::compile(&rows).unwrap();
        assert_eq!(
            rt.admit(&plan, 0, 0, Generation(1)).err(),
            Some(QdnfError::Range)
        );
        assert_eq!(rt.parent_used(), before);
        let mut row = spec(12);
        row.requirements.consumer.bytes = 8 * 1024 * 1024;
        let rows = [row];
        let plan = ColdPlan::compile(&rows).unwrap();
        assert_eq!(
            rt.admit(&plan, 0, 0, Generation(1)).err(),
            Some(QdnfError::BudgetExhausted)
        );
        assert_eq!(rt.parent_used(), before);
        assert_eq!(rt.leased_slots(), 1);
    });
}
