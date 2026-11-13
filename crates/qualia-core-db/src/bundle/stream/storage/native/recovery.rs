//! Reverify untrusted old material into a distinct actually charged destination.
use super::{
    checkpoint::store_verified,
    write::{gates, WriteStep},
};
use super::{
    LeafBarrierObservation, NativeSparseStore, StoreError as E, StoreState, PROOF_SLOT_BYTES,
    READ_SCRATCH_BYTES,
};
use crate::bundle::stream::{
    policy::LocalWholeSourceGrant,
    proof::{decode_proof, MAX_SIBLINGS},
    runtime::{recovery_source::RetainedRecoverySource, SparsePageWorkspace, StorageStage},
    storage::{verify_leaf_view, CHECKPOINT_BYTES},
};
use crate::crypto::network::authority::PinAuthority;
use crate::net::qdnf::types::{Generation, StrongDigest};
use crate::storage_driver::ranges::RangeFile;
impl NativeSparseStore<'_, '_> {
    /// No old bitmap, marker, ticket, quota or policy is promoted. The source is
    /// only bytes: proof verification and actual destination writes/barriers
    /// precede any current mark, including a legitimate all-zero leaf.
    #[allow(clippy::too_many_arguments)]
    pub fn recover_leaf_from(
        &mut self,
        stage: &mut StorageStage<'_, '_>,
        grant: &mut LocalWholeSourceGrant<'_, '_>,
        pins: &PinAuthority,
        workspace: &mut SparsePageWorkspace,
        source: &mut RetainedRecoverySource<'_>,
        leaf: u64,
        nodes: &mut [StrongDigest],
        clock: &impl Fn() -> u64,
        policy: &impl Fn() -> Generation,
    ) -> Result<LeafBarrierObservation, E> {
        self.check_workspace(workspace)?;
        self.check_leaf(leaf)?;
        if source.limits() != self.layout.range_limits()
            || source.attempt_id() == self.backend_attempt
            || !source.matches_canonical_root(
                self.session
                    .staging_path()
                    .parent()
                    .ok_or(E::IdentityMismatch)?,
            )
        {
            return Err(E::IdentityMismatch);
        }
        let next = self.sequence.checked_add(1).ok_or(E::Overflow)?;
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
                let result = stage.with_recovery_buffers(
                    clock(),
                    policy(),
                    attempt.workspace(),
                    source,
                    nodes,
                    |control, buffers, page, mut old| {
                        if buffers.bytes.scratch.len() < READ_SCRATCH_BYTES {
                            return Err(E::ScratchCapacity);
                        }
                        let (proof, rest) = buffers.bytes.scratch.split_at_mut(PROOF_SLOT_BYTES);
                        let (header, rest) = rest.split_at_mut(CHECKPOINT_BYTES);
                        let length = identity.tree_spec().encoded_leaf_length(leaf)?;
                        let payload = &mut rest[..length];
                        // This guard clears unaccepted source bytes/proof on every
                        // return/unwind without manufacturing zero availability.
                        let mut material = UnacceptedMaterial { proof, payload };
                        gates(control, authority, &identity, clock, policy)?;
                        old.read_exact_at(
                            RangeFile::Payload,
                            layout.leaf_offset(leaf)?,
                            material.payload,
                        )
                        .map_err(E::Recovery)?;
                        gates(control, authority, &identity, clock, policy)?;
                        old.read_exact_at(
                            RangeFile::Proof,
                            layout.proof_offset(leaf)?,
                            material.proof,
                        )
                        .map_err(E::Recovery)?;
                        gates(control, authority, &identity, clock, policy)?;
                        let count = usize::from(material.proof[0]);
                        if count > MAX_SIBLINGS {
                            return Err(E::Bounds);
                        }
                        let encoded = 1 + 48 * count;
                        if material.proof[encoded..].iter().any(|b| *b != 0) {
                            return Err(E::Bounds);
                        }
                        let siblings = decode_proof(&material.proof[..encoded], buffers.siblings)?;
                        let view = verify_leaf_view(&identity, leaf, material.payload, siblings)?;
                        gates(control, authority, &identity, clock, policy)?;
                        self.state = StoreState::Poisoned;
                        let observed = store_verified(
                            &mut self.session,
                            &identity,
                            &view,
                            material.proof,
                            header,
                            page,
                            control,
                            authority,
                            layout,
                            next,
                            self.backend_attempt,
                            clock,
                            policy,
                            &mut |_: WriteStep| Ok(()),
                        )?;
                        Ok::<_, E>(observed)
                    },
                );
                Ok(match result {
                    Ok(value) => value,
                    Err(e) => Err(E::Runtime(e)),
                })
            },
        );
        let observed = accepted.map_err(E::Policy)??;
        stage.recheck(clock(), policy())?;
        attempt.commit();
        self.sequence = next;
        self.state = StoreState::Active;
        Ok(observed)
    }
}
struct UnacceptedMaterial<'a> {
    proof: &'a mut [u8],
    payload: &'a mut [u8],
}
impl Drop for UnacceptedMaterial<'_> {
    fn drop(&mut self) {
        self.proof.fill(0);
        self.payload.fill(0)
    }
}
