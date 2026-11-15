//! Reverify untrusted old material into a distinct actually charged destination.
use super::{checkpoint::store_verified, job::ArmedNativeLeafWrite, write::gates};
use super::{
    LeafBarrierObservation, NativeSparseStore, StoreError as E, StoreState, PROOF_SLOT_BYTES,
    READ_SCRATCH_BYTES,
};
use crate::bundle::stream::{
    policy::LocalWholeSourceGrant,
    proof::{decode_proof, LEAF_BYTES, MAX_SIBLINGS},
    runtime::{
        recovery_source::RetainedRecoverySource, SparsePageWorkspace, TransferRuntime,
        TransferTicket,
    },
    storage::{verify_leaf_view, CHECKPOINT_BYTES},
};
use crate::crypto::network::authority::PinAuthority;
use crate::net::qdnf::types::{Generation, StrongDigest};
use crate::storage_driver::ranges::RangeFile;

/// Owned copy of one recovered/reverified leaf. Source reads that require a
/// `RangeRecoveryRead` loan copy into this, then drop the loan before verify.
pub(super) struct CopiedLeafMaterial {
    pub payload: [u8; LEAF_BYTES as usize],
    pub payload_len: usize,
    pub proof: [u8; PROOF_SLOT_BYTES],
    pub header: [u8; CHECKPOINT_BYTES],
}

impl CopiedLeafMaterial {
    pub(super) fn new() -> Self {
        Self {
            payload: [0; LEAF_BYTES as usize],
            payload_len: 0,
            proof: [0; PROOF_SLOT_BYTES],
            header: [0; CHECKPOINT_BYTES],
        }
    }
}

impl Drop for CopiedLeafMaterial {
    fn drop(&mut self) {
        self.payload.fill(0);
        self.proof.fill(0);
        self.header.fill(0);
        self.payload_len = 0;
    }
}
impl NativeSparseStore<'_, '_> {
    /// No old bitmap, marker, ticket, quota or policy is promoted. The source is
    /// only bytes: proof verification and actual destination writes/barriers
    /// precede any current mark, including a legitimate all-zero leaf.
    #[allow(clippy::too_many_arguments)]
    pub fn recover_leaf_from(
        &mut self,
        runtime: &mut TransferRuntime<'_>,
        ticket: &TransferTicket,
        scratch: &mut [u8],
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
        let next = self.next_sequence()?;
        let identity = *self.identity();
        let candidate = self.binding.candidate;
        let layout = self.layout;
        let page_number = self.page;
        let mut attempt = workspace.begin_attempt();
        let mut copied = {
            let mut stage =
                runtime.storage_stage(ticket, &[], &[], scratch, clock(), policy())?;
            match stage.with_recovery_buffers(
                clock(),
                policy(),
                attempt.workspace(),
                source,
                nodes,
                |control, buffers, _, mut old| {
                    if buffers.bytes.scratch.len() < READ_SCRATCH_BYTES {
                        return Err(E::ScratchCapacity);
                    }
                    let (proof, rest) = buffers.bytes.scratch.split_at_mut(PROOF_SLOT_BYTES);
                    let (_header, rest) = rest.split_at_mut(CHECKPOINT_BYTES);
                    let length = identity.tree_spec().encoded_leaf_length(leaf)?;
                    if length > LEAF_BYTES as usize {
                        return Err(E::Bounds);
                    }
                    let payload = &mut rest[..length];
                    let material = UnacceptedMaterial { proof, payload };
                    let current = policy();
                    control.recheck(clock(), current)?;
                    if current.0 != identity.fields().policy_generation {
                        return Err(E::IdentityMismatch);
                    }
                    control.recheck(clock(), policy())?;
                    old.read_exact_at(
                        RangeFile::Payload,
                        layout.leaf_offset(leaf)?,
                        material.payload,
                    )
                    .map_err(E::Recovery)?;
                    control.recheck(clock(), policy())?;
                    old.read_exact_at(RangeFile::Proof, layout.proof_offset(leaf)?, material.proof)
                        .map_err(E::Recovery)?;
                    control.recheck(clock(), policy())?;
                    let mut copied = CopiedLeafMaterial::new();
                    copied.payload_len = length;
                    copied.payload[..length].copy_from_slice(material.payload);
                    copied.proof.copy_from_slice(material.proof);
                    Ok(copied)
                },
            ) {
                Ok(value) => value?,
                Err(e) => return Err(E::Runtime(e)),
            }
        };
        let armed = {
            let mut stage =
                runtime.storage_stage(ticket, &[], &[], scratch, clock(), policy())?;
            match stage.with_sparse_page_buffers(
                clock(),
                policy(),
                attempt.workspace(),
                nodes,
                |control, buffers, page| {
                    if buffers.bytes.scratch.len() < READ_SCRATCH_BYTES {
                        return Err(E::ScratchCapacity);
                    }
                    let current = policy();
                    control.recheck(clock(), current)?;
                    if current.0 != identity.fields().policy_generation {
                        return Err(E::IdentityMismatch);
                    }
                    control.recheck(clock(), policy())?;
                    let count = usize::from(copied.proof[0]);
                    if count > MAX_SIBLINGS {
                        return Err(E::Bounds);
                    }
                    let encoded = 1 + 48 * count;
                    if copied.proof[encoded..].iter().any(|b| *b != 0) {
                        return Err(E::Bounds);
                    }
                    let siblings = decode_proof(&copied.proof[..encoded], buffers.siblings)?;
                    let view = verify_leaf_view(
                        &identity,
                        leaf,
                        &copied.payload[..copied.payload_len],
                        siblings,
                    )?;
                    control.recheck(clock(), policy())?;
                    self.state = StoreState::Poisoned;
                    store_verified(&view, &mut copied.header, page, next)?;
                    ArmedNativeLeafWrite::from_prepared(
                        layout.leaf_offset(leaf)?,
                        layout.proof_offset(leaf)?,
                        layout.checkpoint_offset(page_number, (next & 1) as u8)?,
                        view.bytes(),
                        &copied.proof,
                        &copied.header,
                        page.payload(),
                    )
                },
            ) {
                Ok(value) => value?,
                Err(e) => return Err(E::Runtime(e)),
            }
        };
        self.wait_leaf_write(armed, &mut |_| Ok(()))?;
        {
            let mut stage =
                runtime.storage_stage(ticket, &[], &[], scratch, clock(), policy())?;
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
                            attempt.workspace(),
                            nodes,
                            |control, _, _| {
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
                .map_err(E::Policy)??;
            stage.recheck(clock(), policy())?;
        }
        self.finish_leaf_write(next, leaf, attempt)
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
