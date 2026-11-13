use super::checkpoint::store_verified;
use super::{
    LeafBarrierObservation, NativeSparseStore, StoreError as E, StoreState, PROOF_SLOT_BYTES,
    WRITE_SCRATCH_BYTES,
};
use crate::bundle::stream::{
    policy::{LocalEffectControl, LocalWholeSourceGrant},
    proof::decode_proof,
    runtime::{SparsePageWorkspace, StorageStage, StorageStageControl},
    storage::{verify_leaf_view, StorageIdentity, CHECKPOINT_BYTES},
};
use crate::crypto::network::authority::PinAuthority;
use crate::net::qdnf::types::{Generation, StrongDigest};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum WriteStep {
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
    #[allow(clippy::too_many_arguments)]
    pub fn write_leaf(
        &mut self,
        stage: &mut StorageStage<'_, '_>,
        grant: &mut LocalWholeSourceGrant<'_, '_>,
        pins: &PinAuthority,
        workspace: &mut SparsePageWorkspace,
        leaf: u64,
        nodes: &mut [StrongDigest],
        clock: &impl Fn() -> u64,
        policy: &impl Fn() -> Generation,
    ) -> Result<LeafBarrierObservation, E> {
        self.write_leaf_with_hook(
            stage,
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
        stage: &mut StorageStage<'_, '_>,
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
        let next = self.sequence.checked_add(1).ok_or(E::Overflow)?;
        let identity = *self.identity();
        let candidate = self.binding.candidate;
        let layout = self.layout;
        let page_number = self.page;
        let mut attempt = workspace.begin_attempt();
        // The page guard surrounds the OUTER policy wrapper, including its
        // final check. Returning from the inner stage callback cannot commit it.
        let accepted = grant.accept_local_scoped(
            pins,
            || clock(),
            |authority| {
                if authority.closure().identity() != candidate {
                    return Ok(Err(E::IdentityMismatch));
                }
                let result = stage.with_sparse_page_buffers(
                    clock(),
                    policy(),
                    attempt.workspace(),
                    nodes,
                    |control, buffers, page| {
                        if page.identity() != &identity || page.page_number() != page_number {
                            return Err(E::IdentityMismatch);
                        }
                        if buffers.bytes.scratch.len() < WRITE_SCRATCH_BYTES {
                            return Err(E::ScratchCapacity);
                        }
                        let siblings = decode_proof(buffers.bytes.proof, buffers.siblings)?;
                        let view =
                            verify_leaf_view(&identity, leaf, buffers.bytes.payload, siblings)?;
                        let (proof_slot, rest) =
                            buffers.bytes.scratch.split_at_mut(PROOF_SLOT_BYTES);
                        let header = &mut rest[..CHECKPOINT_BYTES];
                        proof_slot.fill(0);
                        proof_slot[..buffers.bytes.proof.len()]
                            .copy_from_slice(buffers.bytes.proof);
                        gates(control, authority, &identity, clock, policy)?;
                        self.state = StoreState::Poisoned; // armed before any possible effect/unwind.
                        store_verified(
                            &mut self.session,
                            &identity,
                            &view,
                            proof_slot,
                            header,
                            page,
                            control,
                            authority,
                            layout,
                            next,
                            self.backend_attempt,
                            clock,
                            policy,
                            hook,
                        )
                    },
                );
                Ok(match result {
                    Ok(value) => value,
                    Err(e) => Err(E::Runtime(e)),
                })
            },
        );
        let observation = accepted.map_err(E::Policy)??;
        stage.recheck(clock(), policy())?;
        // Only now did BOTH outer authority and final original-ticket check pass.
        attempt.commit();
        self.sequence = next;
        self.state = StoreState::Active;
        Ok(observation)
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
