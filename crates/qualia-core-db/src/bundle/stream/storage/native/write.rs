use super::checkpoint::session_io;
use super::job::ArmedNativeLeafWrite;
use super::{
    LeafBarrierObservation, NativeSparseStore, StoreError as E, StoreState, PROOF_SLOT_BYTES,
    WRITE_SCRATCH_BYTES,
};
use crate::bundle::stream::{
    policy::{LocalEffectControl, LocalWholeSourceGrant},
    proof::{decode_proof, LEAF_BYTES},
    runtime::{
        sparse_workspace::WorkspacePageAttempt, SparsePageWorkspace, StorageStage,
        StorageStageControl, TransferRuntime, TransferTicket,
    },
    storage::{encode_checkpoint, verify_leaf_view, StorageIdentity, CHECKPOINT_BYTES},
};
use crate::crypto::network::authority::PinAuthority;
use crate::net::qdnf::types::{Generation, StrongDigest};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WriteStep {
    PayloadWrite,
    PayloadWritten,
    ProofWrite,
    ProofWritten,
    PayloadSync,
    PayloadSynced,
    ProofSync,
    ProofSynced,
    BitmapWrite,
    BitmapWritten,
    HeaderWrite,
    HeaderWritten,
    CheckpointSync,
    CheckpointSynced,
    FinalCheck,
}
impl NativeSparseStore<'_, '_> {
    pub(super) fn next_sequence(&self) -> Result<u64, E> {
        self.sequence.checked_add(1).ok_or(E::Overflow)
    }

    /// Production convenience. Short StorageStage for prepare, drop, wait with
    /// no stage, then a new short stage for commit. Crash tests use the split
    /// `prepare_leaf_write` / `wait_leaf_write` / `commit_leaf_write` path.
    #[allow(clippy::too_many_arguments)]
    pub fn write_leaf(
        &mut self,
        runtime: &mut TransferRuntime<'_>,
        ticket: &TransferTicket,
        payload: &[u8],
        proof: &[u8],
        scratch: &mut [u8],
        grant: &mut LocalWholeSourceGrant<'_, '_>,
        pins: &PinAuthority,
        workspace: &mut SparsePageWorkspace,
        leaf: u64,
        nodes: &mut [StrongDigest],
        clock: &impl Fn() -> u64,
        policy: &impl Fn() -> Generation,
    ) -> Result<LeafBarrierObservation, E> {
        self.write_leaf_with_hook(
            runtime,
            ticket,
            payload,
            proof,
            scratch,
            grant,
            pins,
            workspace,
            leaf,
            nodes,
            clock,
            policy,
            &mut |_| Ok(()),
        )
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn write_leaf_with_hook(
        &mut self,
        runtime: &mut TransferRuntime<'_>,
        ticket: &TransferTicket,
        payload: &[u8],
        proof: &[u8],
        scratch: &mut [u8],
        grant: &mut LocalWholeSourceGrant<'_, '_>,
        pins: &PinAuthority,
        workspace: &mut SparsePageWorkspace,
        leaf: u64,
        nodes: &mut [StrongDigest],
        clock: &impl Fn() -> u64,
        policy: &impl Fn() -> Generation,
        hook: &mut impl FnMut(WriteStep) -> Result<(), E>,
    ) -> Result<LeafBarrierObservation, E> {
        self.check_workspace(workspace)?;
        self.check_leaf(leaf)?;
        let next = self.next_sequence()?;
        let mut attempt = workspace.begin_attempt();
        let armed = {
            let mut stage = runtime.storage_stage(
                ticket,
                payload,
                proof,
                scratch,
                clock(),
                policy(),
            )?;
            self.prepare_leaf_write(
                &mut stage,
                attempt.workspace(),
                leaf,
                nodes,
                next,
                clock,
                policy,
            )?
        };
        self.wait_leaf_write(armed, hook)?;
        let mut stage =
            runtime.storage_stage(ticket, payload, proof, scratch, clock(), policy())?;
        self.commit_leaf_write(
            &mut stage,
            grant,
            pins,
            attempt.workspace(),
            leaf,
            nodes,
            clock,
            policy,
        )?;
        stage.recheck(clock(), policy())?;
        self.finish_leaf_write(next, leaf, attempt)
    }

    /// Phase 2 wait. No StorageStage, grant, or RuntimeParents in the signature.
    pub(super) fn wait_leaf_write(
        &mut self,
        armed: ArmedNativeLeafWrite,
        hook: &mut impl FnMut(WriteStep) -> Result<(), E>,
    ) -> Result<(), E> {
        debug_assert!(armed.reserved_bytes() > 0);
        let session = &mut self.session;
        armed.wait_blocking(hook, &mut |op, bytes| session_io(session, op, bytes))
    }

    /// Publish sequence/Active after commit recheck. Commits the page attempt.
    pub(super) fn finish_leaf_write(
        &mut self,
        next: u64,
        leaf: u64,
        attempt: WorkspacePageAttempt<'_>,
    ) -> Result<LeafBarrierObservation, E> {
        attempt.commit();
        self.sequence = next;
        self.state = StoreState::Active;
        Ok(LeafBarrierObservation {
            identity: *self.identity(),
            leaf_index: leaf,
            sequence: next,
            backend_attempt: self.backend_attempt,
        })
    }

    /// Phase 1: short StorageStage recheck, copy into already-charged scratch
    /// plus owned I/O buffers, arm Poisoned, encode checkpoint bytes, drop
    /// the stage callback. Does not spend the one-shot grant.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn prepare_leaf_write(
        &mut self,
        stage: &mut StorageStage<'_, '_>,
        workspace: &mut SparsePageWorkspace,
        leaf: u64,
        nodes: &mut [StrongDigest],
        next: u64,
        clock: &impl Fn() -> u64,
        policy: &impl Fn() -> Generation,
    ) -> Result<ArmedNativeLeafWrite, E> {
        let identity = *self.identity();
        let layout = self.layout;
        let page_number = self.page;
        stage.with_sparse_page_buffers(
            clock(),
            policy(),
            workspace,
            nodes,
            |control, buffers, page| {
                if page.identity() != &identity || page.page_number() != page_number {
                    return Err(E::IdentityMismatch);
                }
                if buffers.bytes.scratch.len() < WRITE_SCRATCH_BYTES {
                    return Err(E::ScratchCapacity);
                }
                if buffers.bytes.payload.len() > LEAF_BYTES as usize
                    || buffers.bytes.proof.len() > PROOF_SLOT_BYTES
                {
                    return Err(E::Bounds);
                }
                let siblings = decode_proof(buffers.bytes.proof, buffers.siblings)?;
                let view = verify_leaf_view(&identity, leaf, buffers.bytes.payload, siblings)?;
                let current = policy();
                control.recheck(clock(), current)?;
                if current.0 != identity.fields().policy_generation {
                    return Err(E::IdentityMismatch);
                }
                control.recheck(clock(), policy())?;
                self.state = StoreState::Poisoned;
                page.record_integrity(&view)?;
                let payload_len = buffers.bytes.payload.len();
                if buffers.bytes.scratch.len() >= WRITE_SCRATCH_BYTES + payload_len {
                    buffers.bytes.scratch[WRITE_SCRATCH_BYTES..WRITE_SCRATCH_BYTES + payload_len]
                        .copy_from_slice(buffers.bytes.payload);
                }
                let (proof_slot, rest) = buffers.bytes.scratch.split_at_mut(PROOF_SLOT_BYTES);
                let header = &mut rest[..CHECKPOINT_BYTES];
                proof_slot.fill(0);
                proof_slot[..buffers.bytes.proof.len()].copy_from_slice(buffers.bytes.proof);
                encode_checkpoint(page, next, header)?;
                ArmedNativeLeafWrite::from_prepared(
                    layout.leaf_offset(leaf)?,
                    layout.proof_offset(leaf)?,
                    layout.checkpoint_offset(page_number, (next & 1) as u8)?,
                    view.bytes(),
                    proof_slot,
                    header,
                    page.payload(),
                )
            },
        )?
    }

    /// Phase 3: short grant + StorageStage recheck, then publish the in-memory
    /// integrity bit. Checkpoint *bytes* were already written in Phase 2.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn commit_leaf_write(
        &mut self,
        stage: &mut StorageStage<'_, '_>,
        grant: &mut LocalWholeSourceGrant<'_, '_>,
        pins: &PinAuthority,
        workspace: &mut SparsePageWorkspace,
        leaf: u64,
        nodes: &mut [StrongDigest],
        clock: &impl Fn() -> u64,
        policy: &impl Fn() -> Generation,
    ) -> Result<(), E> {
        let identity = *self.identity();
        let candidate = self.binding.candidate;
        let page_number = self.page;
        grant
            .accept_local_scoped(
                pins,
                || clock(),
                |authority| {
                    if authority.closure().identity() != candidate {
                        return Ok(Err(E::IdentityMismatch));
                    }
                    let result = stage.with_sparse_page_buffers(
                        clock(),
                        policy(),
                        workspace,
                        nodes,
                        |control, buffers, page| {
                            if page.identity() != &identity || page.page_number() != page_number {
                                return Err(E::IdentityMismatch);
                            }
                            gates(control, authority, &identity, clock, policy)?;
                            let siblings = decode_proof(buffers.bytes.proof, buffers.siblings)?;
                            let view =
                                verify_leaf_view(&identity, leaf, buffers.bytes.payload, siblings)?;
                            page.record_integrity(&view)?;
                            gates(control, authority, &identity, clock, policy)?;
                            Ok(())
                        },
                    );
                    Ok(match result {
                        Ok(value) => value,
                        Err(e) => Err(E::Runtime(e)),
                    })
                },
            )
            .map_err(E::Policy)?
    }
}
pub(super) fn gates(
    control: &mut StorageStageControl<'_, '_>,
    authority: &mut LocalEffectControl<'_, '_>,
    identity: &StorageIdentity,
    clock: &impl Fn() -> u64,
    policy: &impl Fn() -> Generation,
) -> Result<(), E> {
    let current = policy();
    control.recheck(clock(), current)?;
    if current.0 != identity.fields().policy_generation {
        return Err(E::IdentityMismatch);
    }
    authority.recheck()?;
    control.recheck(clock(), policy())?;
    Ok(())
}
