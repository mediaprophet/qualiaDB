//! Lifecycle/resource fixtures; allocator measurement belongs to the root harness.
use super::*;
use crate::net::peer::runtime::{
    FairScheduler, LeaseTable, OperationTable, ReservationLedger, ResourceBudget, ResourceGovernor,
    ScheduledWork, WorkClass,
};
use crate::net::qdnf::{
    errors::QdnfError,
    types::{Generation, OperationId},
};
fn budget(bytes: u64) -> ResourceBudget {
    ResourceBudget {
        bytes,
        work: 10000,
        io: 10000,
    }
}
fn control() -> ResourceBudget {
    ResourceBudget {
        bytes: 4096,
        work: 1024,
        io: 64,
    }
}
fn spec(n: u8) -> TransferSpec {
    let part = ResourceBudget {
        bytes: 16,
        work: 1,
        io: 1,
    };
    TransferSpec {
        operation: OperationId([n; 16]),
        principal: 1,
        policy_generation: Generation(1),
        deadline: 100,
        class: WorkClass::Background,
        cost: 1,
        credit_bytes: 16,
        requirements: TransferRequirements {
            network: part,
            proof: part,
            storage: part,
            consumer: part,
            scratch: part,
        },
        verified_peer: true,
    }
}
fn with_runtime(profile: RuntimeProfile, run: impl FnOnce(&mut TransferRuntime<'_>)) {
    let cap = budget(64 * 1024 * 1024);
    let mut ledger = ReservationLedger::new(cap, cap, cap, cap, cap);
    let mut leases = LeaseTable::new();
    let mut ops = OperationTable::new();
    let mut scheduler = FairScheduler::new();
    let governor = ResourceGovernor::new(4096);
    let mut rt = TransferRuntime::new(
        &mut ledger,
        &mut leases,
        &mut ops,
        &mut scheduler,
        &governor,
        profile,
        control(),
    )
    .unwrap();
    run(&mut rt);
    assert_eq!(rt.live_transfers(), 0);
    for _ in 0..160 {
        if rt.pending_schedule_records() == 0 {
            break;
        }
        let _ = rt.poll(1000);
    }
    rt.close().unwrap();
    drop(rt);
    assert_eq!(ledger.used().host, ResourceBudget::ZERO);
    assert_eq!(leases.occupied_count(), 0);
}
fn ready(rt: &mut TransferRuntime<'_>, ticket: &TransferTicket) -> RequestCredit {
    let RuntimePoll::Ready(key) = rt.poll(0).unwrap() else {
        panic!("expected ready")
    };
    assert_eq!(key, ticket.completion_key());
    rt.issue_credit(ticket, 0, Generation(1)).unwrap()
}
#[path = "tests/capacity.rs"]
mod capacity;
#[path = "tests/lifecycle.rs"]
mod lifecycle;

#[path = "tests/storage_stage.rs"]
mod storage_stage;

#[path = "tests/storage_controlled.rs"]
mod storage_controlled;
