use crate::{
    allocation::measured,
    parent_runtime::*,
    qdnf_types::{Generation, OperationId},
    stream_runtime::*,
};
#[test]
fn admission_cancel_expiry_completion_are_allocation_free_and_conserve_budget() {
    let cap = ResourceBudget {
        bytes: 64 * 1024 * 1024,
        work: 10000,
        io: 10000,
    };
    let mut ledger = ReservationLedger::new(cap, cap, cap, cap, cap);
    let mut leases = LeaseTable::new();
    let mut operations = OperationTable::new();
    let mut scheduler = FairScheduler::new();
    let governor = ResourceGovernor::new(4096);
    let control = ResourceBudget {
        bytes: 4096,
        work: 64,
        io: 64,
    };
    let part = ResourceBudget {
        bytes: 16384,
        work: 1,
        io: 1,
    };
    let requirements = TransferRequirements {
        network: part,
        proof: part,
        storage: part,
        consumer: part,
        scratch: part,
    };
    let rows = [TransferSpec {
        operation: OperationId([0x81; 16]),
        principal: 1,
        policy_generation: Generation(1),
        deadline: 100,
        class: WorkClass::Background,
        cost: 1,
        credit_bytes: 16384,
        requirements,
        verified_peer: true,
    }];
    let mut rt = TransferRuntime::new(
        &mut ledger,
        &mut leases,
        &mut operations,
        &mut scheduler,
        &governor,
        RuntimeProfile::Constrained,
        control,
    )
    .unwrap();
    assert_eq!(
        measured(|| {
            let plan = ColdPlan::compile(&rows).unwrap();
            assert!(rt.admit(&plan, 0, 0, Generation(2)).is_err());
            assert!(rt.admit(&plan, 0, 100, Generation(1)).is_err());
            for mode in 0..3 {
                let mut ticket = rt.admit(&plan, 0, 0, Generation(1)).unwrap();
                assert!(matches!(rt.poll(0).unwrap(), RuntimePoll::Ready(_)));
                let credit = rt.issue_credit(&ticket, 0, Generation(1)).unwrap();
                let key = ticket.completion_key();
                let payload = [7u8; 16];
                let proof = [0u8; 1];
                let mut scratch = [0u8; 32];
                {
                    let mut stage = rt
                        .storage_stage(&ticket, &payload, &proof, &mut scratch, 0, Generation(1))
                        .unwrap();
                    let (p, q, s) = stage.buffers(0, Generation(1)).unwrap();
                    assert_eq!(p.as_ptr(), payload.as_ptr());
                    assert_eq!(q.as_ptr(), proof.as_ptr());
                    s[0] = 9;
                }
                assert_eq!(scratch[0], 9);
                if mode == 1 {
                    rt.cancel(&ticket).unwrap();
                }
                if mode == 2 {
                    assert_eq!(rt.expire(100).unwrap(), 1);
                }
                let result = rt
                    .complete_io(&mut ticket, key, if mode == 2 { 100 } else { 0 })
                    .unwrap();
                assert_eq!(
                    result,
                    match mode {
                        0 => FinishOutcome::Released,
                        1 => FinishOutcome::Cancelled,
                        _ => FinishOutcome::Expired,
                    }
                );
                assert!(rt.complete_io(&mut ticket, key, 0).is_err());
                let _ = std::hint::black_box(credit);
            }
            rt.close().unwrap();
        }),
        0
    );
    drop(rt);
    assert_eq!(ledger.used().host, ResourceBudget::ZERO);
    assert_eq!(leases.occupied_count(), 0);
}
