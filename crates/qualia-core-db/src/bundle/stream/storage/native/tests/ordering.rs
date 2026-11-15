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
fn actual_writes_and_barriers_precede_checkpoint_and_duplicate_uses_same_charge() {
    fixture(
        |mut store, workspace, rt, pins, bound, f, payload, _driver, quota, _binding| {
            let before = quota.charged_usage();
            let mut op = ticket(rt, payload.len(), 1);
            let mut scratch = vec![0; READ_SCRATCH_BYTES];
            let mut nodes = [StrongDigest::ZERO; 32];
            let mut owner = AuthorityOwner::new();
            let mut events = Vec::new();
            {
                let mut grant =
                    adopt_local_whole_source(&mut owner, pins, bound, f.request()).unwrap();
                split_hook_write(
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
                        events.push(step);
                        Ok(())
                    },
                )
                .unwrap();
            }
            finish(rt, &mut op);
            assert_eq!(
                events,
                vec![
                    WriteStep::PayloadWrite,
                    WriteStep::PayloadWritten,
                    WriteStep::ProofWrite,
                    WriteStep::ProofWritten,
                    WriteStep::PayloadSync,
                    WriteStep::PayloadSynced,
                    WriteStep::ProofSync,
                    WriteStep::ProofSynced,
                    WriteStep::BitmapWrite,
                    WriteStep::BitmapWritten,
                    WriteStep::HeaderWrite,
                    WriteStep::HeaderWritten,
                    WriteStep::CheckpointSync,
                    WriteStep::CheckpointSynced,
                    WriteStep::FinalCheck
                ]
            );
            assert!(workspace.has_integrity_mark(0).unwrap());
            assert_eq!(store.state(), StoreState::Active);
            let second = write(&mut store, workspace, rt, pins, bound, f, payload, 2).unwrap();
            assert_eq!(second.sequence, 2);
            assert_eq!(quota.charged_usage(), before);
            store
        },
    );
}
#[test]
fn every_stopped_real_protocol_boundary_poison_clears_and_new_facade_cannot_reuse_marks() {
    for stopped in [
        WriteStep::PayloadWrite,
        WriteStep::PayloadWritten,
        WriteStep::ProofWrite,
        WriteStep::ProofWritten,
        WriteStep::PayloadSync,
        WriteStep::PayloadSynced,
        WriteStep::ProofSync,
        WriteStep::ProofSynced,
        WriteStep::BitmapWrite,
        WriteStep::BitmapWritten,
        WriteStep::HeaderWrite,
        WriteStep::HeaderWritten,
        WriteStep::CheckpointSync,
        WriteStep::CheckpointSynced,
        WriteStep::FinalCheck,
    ] {
        fixture(
            |mut store, workspace, rt, pins, bound, f, payload, _driver, quota, binding| {
                let charge = quota.charged_usage();
                let mut op = ticket(rt, payload.len(), 1);
                let mut scratch = vec![0; READ_SCRATCH_BYTES];
                let mut nodes = [StrongDigest::ZERO; 32];
                let mut owner = AuthorityOwner::new();
                {
                    let mut grant =
                        adopt_local_whole_source(&mut owner, pins, bound, f.request()).unwrap();
                    let error = split_hook_write(
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
                            if step == stopped {
                                Err(StoreError::Backend(RangeError::Io(
                                    std::io::ErrorKind::Other,
                                )))
                            } else {
                                Ok(())
                            }
                        },
                    );
                    assert!(error.is_err());
                }
                finish(rt, &mut op);
                assert_eq!(store.state(), StoreState::Poisoned);
                assert!(!workspace.has_integrity_mark(0).unwrap());
                assert!(store.check_workspace(workspace).is_err());
                let mut store =
                    NativeSparseStore::new(store.into_staging(), binding, workspace).unwrap();
                assert!(!workspace.has_integrity_mark(0).unwrap());
                write(&mut store, workspace, rt, pins, bound, f, payload, 2).unwrap();
                assert_eq!(quota.charged_usage(), charge);
                store
            },
        );
    }
}
#[test]
fn outer_authority_final_check_failure_cannot_commit_page_attempt() {
    fixture(
        |mut store, workspace, rt, pins, bound, f, payload, _driver, _quota, _binding| {
            let mut op = ticket(rt, payload.len(), 1);
            let mut scratch = vec![0; READ_SCRATCH_BYTES];
            let mut nodes = [StrongDigest::ZERO; 32];
            let mut owner = AuthorityOwner::new();
            let armed = core::cell::Cell::new(false);
            let samples = core::cell::Cell::new(0usize);
            let clock = || {
                if armed.get() {
                    let n = samples.get() + 1;
                    samples.set(n);
                    if n >= 5 {
                        90
                    } else {
                        20
                    }
                } else {
                    20
                }
            };
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
                    &clock,
                    &|| Generation(1),
                    &mut |step| {
                        if step == WriteStep::FinalCheck {
                            armed.set(true)
                        }
                        Ok(())
                    },
                );
                assert!(matches!(result, Err(StoreError::Policy(_))));
            }
            finish(rt, &mut op);
            assert!(samples.get() >= 5);
            assert!(!workspace.has_integrity_mark(0).unwrap());
            assert_eq!(store.state(), StoreState::Poisoned);
            store
        },
    );
}
