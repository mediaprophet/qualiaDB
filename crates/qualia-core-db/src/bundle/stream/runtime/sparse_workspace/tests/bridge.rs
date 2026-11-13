use super::*;
use crate::bundle::stream::{
    proof::decode_proof,
    runtime::{ColdPlan, FinishOutcome, RuntimePoll, TransferRequirements, TransferSpec},
    storage::encode_checkpoint,
};
use crate::net::qdnf::types::OperationId;

fn transfer(rt: &mut TransferRuntime<'_>, n: u8) -> crate::bundle::stream::runtime::TransferTicket {
    fn part(bytes: u64) -> ResourceBudget {
        ResourceBudget {
            bytes,
            work: 1,
            io: 1,
        }
    }
    let rows = [TransferSpec {
        operation: OperationId([n; 16]),
        principal: 1,
        policy_generation: Generation(1),
        deadline: 100,
        class: WorkClass::Background,
        cost: 1,
        credit_bytes: 1,
        verified_peer: true,
        requirements: TransferRequirements {
            network: part(1),
            proof: part(1537),
            storage: part(2048),
            consumer: part(1),
            scratch: part(64),
        },
    }];
    let plan = ColdPlan::compile(&rows).unwrap();
    let ticket = rt.admit(&plan, 0, 0, Generation(1)).unwrap();
    assert!(matches!(rt.poll(0).unwrap(), RuntimePoll::Ready(_)));
    let _credit = rt.issue_credit(&ticket, 0, Generation(1)).unwrap();
    ticket
}
#[test]
fn actual_page_and_proof_views_allow_recheck_and_cancellation_clears_attempt() {
    with_runtime(|rt| {
        let id = one_leaf();
        let mut workspace = rt.acquire_sparse_page(id, 0).unwrap();
        let original = workspace.page.payload().as_ptr();
        let mut ticket = transfer(rt, 11);
        let mut scratch = [0; 400];
        let mut nodes = [StrongDigest::ZERO; 32];
        let mut attempt = workspace.begin_attempt();
        {
            let mut stage = rt
                .storage_stage(&ticket, &[7], &[0], &mut scratch, 0, Generation(1))
                .unwrap();
            let result = stage
                .with_sparse_page_buffers(
                    0,
                    Generation(1),
                    attempt.workspace(),
                    &mut nodes,
                    |control, buffers, page| {
                        assert_eq!(page.payload().as_ptr(), original);
                        let siblings = decode_proof(buffers.bytes.proof, buffers.siblings).unwrap();
                        let view =
                            verify_leaf_view(page.identity(), 0, buffers.bytes.payload, siblings)
                                .unwrap();
                        control.recheck(1, Generation(1))?;
                        page.record_integrity(&view).unwrap();
                        encode_checkpoint(page, 1, buffers.bytes.scratch).unwrap();
                        control.recheck(2, Generation(1))
                    },
                )
                .unwrap();
            result.unwrap();
            stage.recheck(3, Generation(1)).unwrap();
        }
        attempt.commit();
        assert!(workspace.has_integrity_mark(0).unwrap());
        let key = ticket.completion_key();
        assert_eq!(
            rt.complete_io(&mut ticket, key, 3).unwrap(),
            FinishOutcome::Released
        );

        let mut ticket = transfer(rt, 12);
        {
            let mut attempt = workspace.begin_attempt();
            let mut stage = rt
                .storage_stage(&ticket, &[7], &[0], &mut scratch, 0, Generation(1))
                .unwrap();
            let result = stage
                .with_sparse_page_buffers(
                    0,
                    Generation(1),
                    attempt.workspace(),
                    &mut nodes,
                    |control, buffers, page| {
                        let siblings = decode_proof(buffers.bytes.proof, buffers.siblings).unwrap();
                        let view =
                            verify_leaf_view(page.identity(), 0, buffers.bytes.payload, siblings)
                                .unwrap();
                        page.record_integrity(&view).unwrap();
                        control.cancel()?;
                        control.recheck(0, Generation(1))
                    },
                )
                .unwrap();
            assert_eq!(result, Err(QdnfError::Cancelled));
            // Stage drops before attempt; neither releases actual I/O itself.
        }
        assert!(!workspace.has_integrity_mark(0).unwrap());
        assert_eq!(rt.retained_sparse_pages(), 1);
        let key = ticket.completion_key();
        assert_eq!(
            rt.complete_io(&mut ticket, key, 0).unwrap(),
            FinishOutcome::Cancelled
        );
        rt.release_sparse_page(workspace).unwrap();
    });
}
#[test]
fn wrong_runtime_workspace_and_oversized_typed_scratch_never_invoke_effect() {
    with_runtime(|rt| {
        let mut own = rt.acquire_sparse_page(one_leaf(), 0).unwrap();
        let mut ticket = transfer(rt, 21);
        let mut scratch = [0; 400];
        let mut nodes = [StrongDigest::ZERO; 33];
        let before = rt.parent_used();
        with_runtime(|foreign| {
            let mut wrong = foreign.acquire_sparse_page(one_leaf(), 0).unwrap();
            {
                let mut stage = rt
                    .storage_stage(&ticket, &[7], &[0], &mut scratch, 0, Generation(1))
                    .unwrap();
                let result: Result<(), QdnfError> = stage.with_sparse_page_buffers(
                    0,
                    Generation(1),
                    &mut wrong,
                    &mut nodes[..32],
                    |_, _, _| panic!("foreign workspace must not reach effect"),
                );
                assert_eq!(result, Err(QdnfError::StaleGeneration));
            }
            foreign.release_sparse_page(wrong).unwrap();
        });
        {
            let mut stage = rt
                .storage_stage(&ticket, &[7], &[0], &mut scratch, 0, Generation(1))
                .unwrap();
            let result: Result<(), QdnfError> = stage.with_sparse_page_buffers(
                0,
                Generation(1),
                &mut own,
                &mut nodes,
                |_, _, _| panic!("oversized nodes must not reach effect"),
            );
            assert_eq!(result, Err(QdnfError::Capacity));
        }
        assert_eq!(rt.parent_used(), before);
        assert!(!own.has_integrity_mark(0).unwrap());
        let key = ticket.completion_key();
        rt.complete_io(&mut ticket, key, 0).unwrap();
        rt.release_sparse_page(own).unwrap();
    });
}
