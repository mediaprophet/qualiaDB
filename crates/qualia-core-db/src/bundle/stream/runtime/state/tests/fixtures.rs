use super::*;
pub(super) fn budget(bytes: u64) -> ResourceBudget {
    ResourceBudget {
        bytes,
        work: 10000,
        io: 10000,
    }
}
pub(super) fn ledger() -> ReservationLedger {
    let c = budget(64 * 1024 * 1024);
    ReservationLedger::new(c, c, c, c, c)
}
pub(super) fn control() -> ResourceBudget {
    ResourceBudget {
        bytes: 4096,
        work: 1024,
        io: 64,
    }
}
pub(super) fn spec(id: u8) -> TransferSpec {
    let p = ResourceBudget {
        bytes: 16,
        work: 1,
        io: 1,
    };
    TransferSpec {
        operation: OperationId([id; 16]),
        principal: 1,
        policy_generation: Generation(1),
        deadline: 100,
        class: WorkClass::Background,
        cost: 1,
        credit_bytes: 16,
        requirements: TransferRequirements {
            network: p,
            proof: p,
            storage: p,
            consumer: p,
            scratch: p,
        },
        verified_peer: true,
    }
}
pub(super) fn with_parents<T>(
    orphan: bool,
    run: impl for<'h> FnOnce(&RuntimeParents<'h>) -> T,
) -> T {
    let ledger = Mutex::new(ledger());
    let mut leases = Mutex::new(LeaseTable::new());
    let mut operations = Mutex::new(OperationTable::new());
    let mut scheduler = Mutex::new(FairScheduler::new());
    let governor = ResourceGovernor::new(4096);
    let mut parents = RuntimeParents::adopt(
        &ledger,
        &mut leases,
        &mut operations,
        &mut scheduler,
        &governor,
    )
    .unwrap();
    let result = run(&parents);
    if orphan {
        assert_eq!(parents.try_close(), Err(E::WouldBlock));
        assert!(parents.ledger.lock().unwrap().used().host.bytes > 0);
    } else {
        parents.try_close().unwrap();
        assert_eq!(
            parents.ledger.lock().unwrap().used().host,
            ResourceBudget::ZERO
        );
    }
    result
}
pub(super) fn ready<'h>(
    parents: &RuntimeParents<'h>,
    state: &mut RetainedRuntime<'_, 'h>,
    ticket: &TransferTicket,
) {
    parents
        .try_with_runtime(state, |mut rt| {
            let RuntimePoll::Ready(key) = rt.poll(0)? else {
                panic!("actual scheduler readiness")
            };
            assert_eq!(key, ticket.completion_key());
            let _credit = rt.issue_credit(ticket, 0, Generation(1))?;
            Ok(())
        })
        .unwrap();
}
pub(super) fn finish<'h>(
    parents: &RuntimeParents<'h>,
    state: &mut RetainedRuntime<'_, 'h>,
    ticket: &mut TransferTicket,
) {
    parents
        .try_with_runtime(state, |mut rt| {
            let key = ticket.completion_key();
            rt.complete_io(ticket, key, 0)?;
            Ok(())
        })
        .unwrap();
}
