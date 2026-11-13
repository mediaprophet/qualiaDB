use super::write::gates;
use super::{
    NativeSparseStore, StoreError as E, StoreLayout, PROOF_SLOT_BYTES, READ_SCRATCH_BYTES,
};
use crate::bundle::stream::{
    policy::{LocalEffectControl, LocalWholeSourceGrant},
    proof::decode_proof,
    runtime::{SparsePageWorkspace, StorageStage, StorageStageControl},
    storage::{verify_leaf_view, StorageIdentity, VerifiedLeafView, CHECKPOINT_BYTES},
};
use crate::crypto::network::authority::PinAuthority;
use crate::net::qdnf::types::{Generation, StrongDigest};
use crate::storage_driver::ranges::{RangeFile, RangeSession};
pub(super) struct StoredMaterial<'a> {
    pub view: VerifiedLeafView<'a>,
    pub proof_slot: &'a [u8],
    pub header: &'a mut [u8],
}
#[allow(clippy::too_many_arguments)]
pub(super) fn read_material<'a>(
    session: &mut RangeSession<'_, '_>,
    identity: &StorageIdentity,
    layout: StoreLayout,
    leaf: u64,
    scratch: &'a mut [u8],
    nodes: &mut [StrongDigest],
    control: &mut StorageStageControl<'_, '_>,
    authority: &mut LocalEffectControl<'_, '_>,
    clock: &impl Fn() -> u64,
    policy: &impl Fn() -> Generation,
) -> Result<StoredMaterial<'a>, E> {
    if scratch.len() < READ_SCRATCH_BYTES {
        return Err(E::ScratchCapacity);
    }
    let length = identity.tree_spec().encoded_leaf_length(leaf)?;
    let (proof_slot, rest) = scratch.split_at_mut(PROOF_SLOT_BYTES);
    let (header, rest) = rest.split_at_mut(CHECKPOINT_BYTES);
    let payload = &mut rest[..length];
    gates(control, authority, identity, clock, policy)?;
    session.read_exact_at(RangeFile::Payload, layout.leaf_offset(leaf)?, payload)?;
    gates(control, authority, identity, clock, policy)?;
    session.read_exact_at(RangeFile::Proof, layout.proof_offset(leaf)?, proof_slot)?;
    gates(control, authority, identity, clock, policy)?;
    let count = usize::from(proof_slot[0]);
    if count > crate::bundle::stream::proof::MAX_SIBLINGS {
        return Err(E::Bounds);
    }
    let encoded = 1 + count * 48;
    if proof_slot[encoded..].iter().any(|b| *b != 0) {
        return Err(E::Bounds);
    }
    let siblings = decode_proof(&proof_slot[..encoded], nodes)?;
    let view = verify_leaf_view(identity, leaf, payload, siblings)?;
    gates(control, authority, identity, clock, policy)?;
    Ok(StoredMaterial {
        view,
        proof_slot,
        header,
    })
}
impl NativeSparseStore<'_, '_> {
    /// Borrow verified actual readback only inside the bounded Stage callback.
    /// This grants no consumer/serving commit or lease; no borrowed view escapes.
    /// The inspect callback is trusted host code: it can copy bytes or perform
    /// effects and must separately govern/authorize consumer work. HRTB bounds
    /// borrow lifetime; it does not confine a hostile host or revoke copies.
    #[allow(clippy::too_many_arguments)]
    pub fn read_verified_leaf<T>(
        &mut self,
        stage: &mut StorageStage<'_, '_>,
        grant: &mut LocalWholeSourceGrant<'_, '_>,
        pins: &PinAuthority,
        workspace: &mut SparsePageWorkspace,
        leaf: u64,
        nodes: &mut [StrongDigest],
        clock: &impl Fn() -> u64,
        policy: &impl Fn() -> Generation,
        inspect: impl for<'v> FnOnce(&VerifiedLeafView<'v>) -> T,
    ) -> Result<T, E> {
        self.check_workspace(workspace)?;
        self.check_leaf(leaf)?;
        let identity = *self.identity();
        let candidate = self.binding.candidate;
        let layout = self.layout;
        let mut attempt = workspace.begin_attempt();
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
                    |control, buffers, _page| {
                        let material = read_material(
                            &mut self.session,
                            &identity,
                            layout,
                            leaf,
                            buffers.bytes.scratch,
                            buffers.siblings,
                            control,
                            authority,
                            clock,
                            policy,
                        )?;
                        gates(control, authority, &identity, clock, policy)?;
                        let output = inspect(&material.view);
                        gates(control, authority, &identity, clock, policy)?;
                        Ok::<_, E>(output)
                    },
                );
                Ok(match result {
                    Ok(value) => value,
                    Err(e) => Err(E::Runtime(e)),
                })
            },
        );
        let output = accepted.map_err(E::Policy)??;
        stage.recheck(clock(), policy())?;
        attempt.commit();
        Ok(output)
    }
}
