//! Bounded submitted jobs; original transfer charges stay until complete_io.
use super::super::{
    ColdPlan, FinishOutcome, RuntimeParents, RuntimePoll, RuntimeProfile, TransferRequirements,
    TransferRuntime, TransferSpec, TransferTicket,
};
use super::StorageJobs;
use crate::net::peer::runtime::{
    FairScheduler, LeaseTable, OperationTable, ReservationLedger, ResourceBudget, ResourceGovernor,
    WorkClass,
};
use crate::net::qdnf::{
    authority::{AuthorityOwner, CurrentAuthorityGuard},
    errors::QdnfError,
    types::{Generation, OperationId},
};
use std::sync::Mutex;

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
fn with_runtime(run: impl FnOnce(&mut TransferRuntime<'_>)) {
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
        RuntimeProfile::Constrained,
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
fn ready(rt: &mut TransferRuntime<'_>, ticket: &TransferTicket) {
    let RuntimePoll::Ready(key) = rt.poll(0).unwrap() else {
        panic!("expected ready")
    };
    assert_eq!(key, ticket.completion_key());
    let _credit = rt.issue_credit(ticket, 0, Generation(1)).unwrap();
}

#[test]
fn submit_slow_complete_keeps_live_transfers_charged_until_complete() {
    with_runtime(|rt| {
        let rows = [spec(1)];
        let plan = ColdPlan::compile(&rows).unwrap();
        let mut ticket = rt.admit(&plan, 0, 0, Generation(1)).unwrap();
        ready(rt, &ticket);
        let mut jobs = StorageJobs::for_runtime(rt);
        let payload = [7; 16];
        let proof = [8; 16];
        let mut scratch = [0; 16];
        let held = rt.parent_used();
        let window = rt.charged_window_bytes();
        let mut job = jobs
            .submit(
                rt,
                &ticket,
                &payload,
                &proof,
                &mut scratch,
                0,
                Generation(1),
            )
            .unwrap();
        assert_eq!(scratch[0], 7);
        assert_eq!(rt.live_transfers(), 1);
        assert_eq!(jobs.live_jobs(), 1);
        assert!(jobs.buffer_held(&job).unwrap());
        assert_eq!(rt.parent_used(), held);
        assert_eq!(rt.charged_window_bytes(), window);
        assert_eq!(
            jobs.submit(
                rt,
                &ticket,
                &payload,
                &proof,
                &mut scratch,
                0,
                Generation(1)
            )
            .err(),
            Some(QdnfError::Replay)
        );
        assert!(jobs.may_publish_verified(rt, &ticket, &job).unwrap());
        assert_eq!(rt.live_transfers(), 1);
        assert_eq!(rt.parent_used(), held);
        assert_eq!(
            jobs.complete_io(rt, &mut ticket, &mut job, 1),
            Ok(FinishOutcome::Released)
        );
        assert_eq!(rt.live_transfers(), 0);
        assert!(!jobs.buffer_held(&job).unwrap());
        assert_eq!(
            jobs.complete_io(rt, &mut ticket, &mut job, 1),
            Err(QdnfError::DoubleRelease)
        );
    });
}

#[test]
fn cancel_then_complete_is_cancelled_without_double_refund_or_early_reuse() {
    with_runtime(|rt| {
        let rows = [spec(2)];
        let plan = ColdPlan::compile(&rows).unwrap();
        let mut ticket = rt.admit(&plan, 0, 0, Generation(1)).unwrap();
        ready(rt, &ticket);
        let mut jobs = StorageJobs::for_runtime(rt);
        let payload = [7; 16];
        let proof = [8; 16];
        let mut scratch = [0; 16];
        let mut job = jobs
            .submit(
                rt,
                &ticket,
                &payload,
                &proof,
                &mut scratch,
                0,
                Generation(1),
            )
            .unwrap();
        let charged = rt.parent_used();
        let window = rt.charged_window_bytes();
        jobs.cancel(rt, &ticket, &job).unwrap();
        assert_eq!(rt.live_transfers(), 1);
        assert_eq!(rt.parent_used(), charged);
        assert_eq!(rt.charged_window_bytes(), window);
        assert!(jobs.buffer_held(&job).unwrap());
        assert!(!jobs.may_publish_verified(rt, &ticket, &job).unwrap());
        assert_eq!(
            jobs.submit(
                rt,
                &ticket,
                &payload,
                &proof,
                &mut scratch,
                0,
                Generation(1)
            )
            .err(),
            Some(QdnfError::Cancelled)
        );
        assert_eq!(
            rt.admit(&plan, 0, 0, Generation(1)).err(),
            Some(QdnfError::Conflict)
        );
        assert_eq!(
            jobs.complete_io(rt, &mut ticket, &mut job, 1),
            Ok(FinishOutcome::Cancelled)
        );
        assert_eq!(rt.live_transfers(), 0);
        assert!(!jobs.buffer_held(&job).unwrap());
        assert_eq!(
            jobs.complete_io(rt, &mut ticket, &mut job, 1),
            Err(QdnfError::DoubleRelease)
        );
        let mut next = rt.admit(&plan, 0, 0, Generation(1)).unwrap();
        ready(rt, &next);
        assert_ne!(next.completion_key(), ticket.completion_key());
        let key = next.completion_key();
        rt.complete_io(&mut next, key, 1).unwrap();
    });
}

#[test]
fn worker_panic_or_error_conserves_charges_and_does_not_free_slot() {
    with_runtime(|rt| {
        let rows = [spec(3)];
        let plan = ColdPlan::compile(&rows).unwrap();
        let mut ticket = rt.admit(&plan, 0, 0, Generation(1)).unwrap();
        ready(rt, &ticket);
        let mut jobs = StorageJobs::for_runtime(rt);
        let payload = [7; 16];
        let proof = [8; 16];
        let mut scratch = [0; 16];
        let mut job = jobs
            .submit(
                rt,
                &ticket,
                &payload,
                &proof,
                &mut scratch,
                0,
                Generation(1),
            )
            .unwrap();
        let charged = rt.parent_used();
        let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            panic!("storage worker");
        }));
        assert!(panicked.is_err());
        assert_eq!(rt.live_transfers(), 1);
        assert_eq!(rt.parent_used(), charged);
        jobs.observe_worker_failure(rt, &ticket, &job).unwrap();
        assert_eq!(rt.live_transfers(), 1);
        assert_eq!(rt.parent_used(), charged);
        assert!(jobs.buffer_held(&job).unwrap());
        assert!(!jobs.may_publish_verified(rt, &ticket, &job).unwrap());
        assert_eq!(
            rt.admit(&plan, 0, 0, Generation(1)).err(),
            Some(QdnfError::Conflict)
        );
        assert_eq!(
            jobs.complete_io(rt, &mut ticket, &mut job, 1),
            Ok(FinishOutcome::Cancelled)
        );
    });
}

#[test]
fn job_ticket_from_owner_a_cannot_complete_on_owner_b() {
    with_runtime(|a| {
        with_runtime(|b| {
            let rows = [spec(4)];
            let plan = ColdPlan::compile(&rows).unwrap();
            let mut ticket_a = a.admit(&plan, 0, 0, Generation(1)).unwrap();
            ready(a, &ticket_a);
            let mut jobs_a = StorageJobs::for_runtime(a);
            let payload = [7; 16];
            let proof = [8; 16];
            let mut scratch_a = [0; 16];
            let mut job_a = jobs_a
                .submit(
                    a,
                    &ticket_a,
                    &payload,
                    &proof,
                    &mut scratch_a,
                    0,
                    Generation(1),
                )
                .unwrap();
            let mut ticket_b = b.admit(&plan, 0, 0, Generation(1)).unwrap();
            ready(b, &ticket_b);
            let mut jobs_b = StorageJobs::for_runtime(b);
            let mut scratch_b = [0; 16];
            assert_eq!(
                jobs_b.complete_io(b, &mut ticket_b, &mut job_a, 0).err(),
                Some(QdnfError::StaleGeneration)
            );
            assert_eq!(
                jobs_a.complete_io(b, &mut ticket_a, &mut job_a, 0).err(),
                Some(QdnfError::StaleGeneration)
            );
            assert_eq!(a.live_transfers(), 1);
            assert_eq!(b.live_transfers(), 1);
            jobs_a.complete_io(a, &mut ticket_a, &mut job_a, 0).unwrap();
            let mut job_b = jobs_b
                .submit(
                    b,
                    &ticket_b,
                    &payload,
                    &proof,
                    &mut scratch_b,
                    0,
                    Generation(1),
                )
                .unwrap();
            jobs_b.complete_io(b, &mut ticket_b, &mut job_b, 0).unwrap();
        });
    });
}

#[test]
fn wait_site_does_not_hold_runtime_parents_or_authority_mutex() {
    let cap = budget(64 * 1024 * 1024);
    let ledger = Mutex::new(ReservationLedger::new(cap, cap, cap, cap, cap));
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
    let authority = Mutex::new(AuthorityOwner::new());
    let mut state = parents
        .admit(RuntimeProfile::Constrained, control())
        .unwrap();
    let rows = [spec(5)];
    let plan = ColdPlan::compile(&rows).unwrap();
    let mut ticket = parents
        .try_with_runtime(&mut state, |mut rt| rt.admit(&plan, 0, 0, Generation(1)))
        .unwrap();
    parents
        .try_with_runtime(&mut state, |mut rt| {
            let RuntimePoll::Ready(key) = rt.poll(0)? else {
                panic!("expected ready")
            };
            assert_eq!(key, ticket.completion_key());
            let _credit = rt.issue_credit(&ticket, 0, Generation(1))?;
            Ok(())
        })
        .unwrap();
    let mut jobs = StorageJobs::for_owner(ticket.completion_key().owner);
    let payload = [7; 16];
    let proof = [8; 16];
    let mut scratch = [0; 16];
    let mut job = {
        let _guard = CurrentAuthorityGuard::try_lock(&authority).unwrap();
        parents
            .try_with_runtime(&mut state, |mut rt| {
                let stage =
                    rt.storage_stage(&ticket, &payload, &proof, &mut scratch, 0, Generation(1))?;
                jobs.submit_stage(&ticket, stage, 0, Generation(1))
            })
            .unwrap()
    };
    // Mutex<AuthorityOwner> and the ledger mutex are not held across this wait.
    // leases/operations/scheduler stay `&mut` borrowed by RuntimeParents::adopt
    // for `'host`; inner exclusion is proven by try_with_runtime succeeding
    // (WouldBlock if those mutexes were still locked).
    assert!(ledger.try_lock().is_ok());
    assert!(CurrentAuthorityGuard::try_lock(&authority).is_ok());
    assert!(jobs.buffer_held(&job).unwrap());
    parents
        .try_with_runtime(&mut state, |mut rt| {
            let key = ticket.completion_key();
            rt.complete_io(&mut ticket, key, 1)?;
            Ok(())
        })
        .unwrap();
    jobs.take_completed(&ticket, &mut job).unwrap();
    state.try_close().unwrap();
    parents.try_close().unwrap();
    assert_eq!(ledger.lock().unwrap().used().host, ResourceBudget::ZERO);
}
