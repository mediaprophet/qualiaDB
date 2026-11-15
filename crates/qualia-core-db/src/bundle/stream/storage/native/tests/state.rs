use super::super::write::WriteStep;
use super::*;
use crate::bundle::stream::policy::LocalWholeSourceGrant;
use crate::bundle::stream::runtime::{SparsePageWorkspace, TransferRuntime, TransferTicket};
use crate::crypto::network::authority::PinAuthority;

fn split_hook_write(
    store: &mut NativeSparseStore<'_, '_>,
    workspace: &mut SparsePageWorkspace,
    rt: &mut TransferRuntime<'_>,
    op: &TransferTicket,
    grant: &mut LocalWholeSourceGrant<'_, '_>,
    pins: &PinAuthority,
    payload: &[u8],
    scratch: &mut [u8],
    nodes: &mut [StrongDigest],
    clock: &impl Fn() -> u64,
    policy: &impl Fn() -> Generation,
    hook: &mut impl FnMut(WriteStep) -> Result<(), StoreError>,
) -> Result<LeafBarrierObservation, StoreError> {
    let next = store.next_sequence()?;
    let mut attempt = workspace.begin_attempt();
    let armed = {
        let mut stage = rt
            .storage_stage(op, payload, &[0], scratch, 20, Generation(1))
            .unwrap();
        store.prepare_leaf_write(
            &mut stage,
            attempt.workspace(),
            0,
            nodes,
            next,
            clock,
            policy,
        )?
    };
    store.wait_leaf_write(armed, hook)?;
    let mut stage = rt
        .storage_stage(op, payload, &[0], scratch, 20, Generation(1))
        .unwrap();
    store.commit_leaf_write(
        &mut stage,
        grant,
        pins,
        attempt.workspace(),
        0,
        nodes,
        clock,
        policy,
    )?;
    stage.recheck(clock(), policy())?;
    store.finish_leaf_write(next, 0, attempt)
}

#[test]
fn newer_facade_over_other_backend_mechanically_rejects_old_facade_key() {
    fixture(
        |mut old, workspace, rt, pins, bound, f, payload, driver, quota, binding| {
            write(&mut old, workspace, rt, pins, bound, f, payload, 1).unwrap();
            let layout = StoreLayout::for_binding(&binding).unwrap();
            let session = driver
                .create_range_staging(
                    quota
                        .admit_staging([3; 32], layout.aggregate_allowance().unwrap())
                        .unwrap(),
                    layout.range_limits(),
                )
                .unwrap();
            let mut new = NativeSparseStore::new(session, binding, workspace).unwrap();
            assert!(!workspace.has_integrity_mark(0).unwrap());
            assert!(old.check_workspace(workspace).is_err());
            assert!(write(&mut old, workspace, rt, pins, bound, f, payload, 2).is_err());
            write(&mut new, workspace, rt, pins, bound, f, payload, 3).unwrap();
            drop(old);
            new
        },
    );
}
#[test]
fn current_policy_change_or_timeout_after_real_io_clears_marks_and_poison() {
    for timeout in [false, true] {
        fixture(
            |mut store, workspace, rt, pins, bound, f, payload, _driver, _quota, _binding| {
                let now = core::cell::Cell::new(20);
                let policy = core::cell::Cell::new(Generation(1));
                let mut op = ticket(rt, payload.len(), 1);
                let mut scratch = vec![0; READ_SCRATCH_BYTES];
                let mut nodes = [StrongDigest::ZERO; 32];
                let mut owner = AuthorityOwner::new();
                {
                    let mut grant =
                        adopt_local_whole_source(&mut owner, pins, bound, f.request()).unwrap();
                    let result = split_hook_write(
                        &mut store,
                        workspace,
                        rt,
                        &op,
                        &mut grant,
                        pins,
                        payload,
                        &mut scratch,
                        &mut nodes,
                        &|| now.get(),
                        &|| policy.get(),
                        &mut |step| {
                            if step == WriteStep::HeaderWrite {
                                if timeout {
                                    now.set(120)
                                } else {
                                    policy.set(Generation(2))
                                }
                            }
                            Ok(())
                        },
                    );
                    assert!(result.is_err());
                }
                finish(rt, &mut op);
                assert!(!workspace.has_integrity_mark(0).unwrap());
                assert_eq!(store.state(), StoreState::Poisoned);
                store
            },
        );
    }
}
#[test]
fn unwind_after_real_sync_clears_armed_page_and_keeps_store_poisoned() {
    fixture(
        |mut store, workspace, rt, pins, bound, f, payload, _driver, _quota, _binding| {
            let mut op = ticket(rt, payload.len(), 1);
            let mut scratch = vec![0; READ_SCRATCH_BYTES];
            let mut nodes = [StrongDigest::ZERO; 32];
            let mut owner = AuthorityOwner::new();
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let mut grant =
                    adopt_local_whole_source(&mut owner, pins, bound, f.request()).unwrap();
                let _ = split_hook_write(
                    &mut store,
                    workspace,
                    rt,
                    &op,
                    &mut grant,
                    pins,
                    payload,
                    &mut scratch,
                    &mut nodes,
                    &|| 20,
                    &|| Generation(1),
                    &mut |step| {
                        if step == WriteStep::CheckpointSynced {
                            panic!("deliberate native effect unwind")
                        }
                        Ok(())
                    },
                );
            }));
            assert!(result.is_err());
            finish(rt, &mut op);
            assert!(!workspace.has_integrity_mark(0).unwrap());
            assert_eq!(store.state(), StoreState::Poisoned);
            store
        },
    );
}
#[test]
fn malformed_input_refuses_before_file_growth_and_does_not_poison() {
    fixture(
        |mut store, workspace, rt, pins, bound, f, payload, _driver, quota, _binding| {
            let charge = quota.charged_usage();
            let mut op = ticket(rt, payload.len(), 1);
            let mut scratch = vec![0; READ_SCRATCH_BYTES];
            let mut nodes = [StrongDigest::ZERO; 32];
            let mut owner = AuthorityOwner::new();
            {
                let mut grant =
                    adopt_local_whole_source(&mut owner, pins, bound, f.request()).unwrap();
                assert!(store
                    .write_leaf(
                        rt,
                        &op,
                        payload,
                        &[1],
                        &mut scratch,
                        &mut grant,
                        pins,
                        workspace,
                        0,
                        &mut nodes,
                        &|| 20,
                        &|| Generation(1)
                    )
                    .is_err());
            }
            finish(rt, &mut op);
            assert_eq!(store.state(), StoreState::Active);
            assert_eq!(
                store
                    .session
                    .observed_file_length(RangeFile::Payload)
                    .unwrap(),
                0
            );
            assert_eq!(quota.charged_usage(), charge);
            store
        },
    );
}
