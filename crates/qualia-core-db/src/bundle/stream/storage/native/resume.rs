use super::recovery::CopiedLeafMaterial;
use super::{checkpoint::store_verified, job::ArmedNativeLeafWrite, write::gates};
use super::{
    LeafBarrierObservation, NativeSparseStore, StoreError as E, StoreState, CHECKPOINT_BANK_BYTES,
    READ_SCRATCH_BYTES,
};
use crate::bundle::stream::{
    policy::LocalWholeSourceGrant,
    proof::{decode_proof, MAX_SIBLINGS},
    runtime::{SparsePageWorkspace, StorageStage, TransferRuntime, TransferTicket},
    storage::{decode_checkpoint, verify_leaf_view, BITMAP_BYTES, CHECKPOINT_BYTES},
};
use crate::crypto::network::{authority::PinAuthority, digest::sha384};
use crate::net::qdnf::types::{Generation, StrongDigest};
use crate::storage_driver::ranges::{RangeError, RangeFile};
/// Untrusted scheduling observation. False/absence means Unknown, never Absent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResumeHintObservation {
    pub page: u64,
    pub sequence: Option<u64>,
    pub bank: Option<u8>,
    pub claims_leaf: bool,
}
impl NativeSparseStore<'_, '_> {
    /// Same-live-session hint reload only. Both banks are unkeyed hints; no
    /// bitmap initialization or process-crash reopen authority is created.
    #[allow(clippy::too_many_arguments)]
    pub fn resume_hint_for_leaf(
        &mut self,
        stage: &mut StorageStage<'_, '_>,
        grant: &mut LocalWholeSourceGrant<'_, '_>,
        pins: &PinAuthority,
        workspace: &mut SparsePageWorkspace,
        leaf: u64,
        nodes: &mut [StrongDigest],
        clock: &impl Fn() -> u64,
        policy: &impl Fn() -> Generation,
    ) -> Result<ResumeHintObservation, E> {
        self.check_workspace(workspace)?;
        self.check_leaf(leaf)?;
        let identity = *self.identity();
        let candidate = self.binding.candidate;
        let layout = self.layout;
        let page_no = self.page;
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
                    |control, buffers, _| {
                        let frame = buffers
                            .bytes
                            .scratch
                            .get_mut(..CHECKPOINT_BANK_BYTES as usize)
                            .ok_or(E::ScratchCapacity)?;
                        let mut chosen: Option<(u64, u8, bool, StrongDigest)> = None;
                        for bank in 0..2u8 {
                            gates(control, authority, &identity, clock, policy)?;
                            match self.session.read_exact_at(
                                RangeFile::Checkpoint,
                                layout.checkpoint_offset(page_no, bank)?,
                                frame,
                            ) {
                                Ok(()) => {}
                                Err(RangeError::Io(std::io::ErrorKind::UnexpectedEof)) => continue,
                                Err(e) => return Err(e.into()),
                            }
                            gates(control, authority, &identity, clock, policy)?;
                            let checkpoint = match decode_checkpoint(&frame[..CHECKPOINT_BYTES]) {
                                Ok(c) => c,
                                Err(_) => continue,
                            };
                            let payload =
                                <&[u8; BITMAP_BYTES]>::try_from(&frame[CHECKPOINT_BYTES..])
                                    .map_err(|_| E::Bounds)?;
                            let hint = match checkpoint.bind_payload(&identity, page_no, payload) {
                                Ok(h) => h,
                                Err(_) => continue,
                            };
                            let sequence = hint.sequence();
                            let claim = hint.claims_leaf(leaf)?;
                            let digest = sha384(&frame[..CHECKPOINT_BYTES]);
                            if let Some((old, _, _, old_digest)) = chosen {
                                if old == sequence && old_digest != digest {
                                    return Err(E::CheckpointConflict);
                                }
                                if old >= sequence {
                                    continue;
                                }
                            }
                            chosen = Some((sequence, bank, claim, digest));
                        }
                        gates(control, authority, &identity, clock, policy)?;
                        Ok::<_, E>(match chosen {
                            Some((sequence, bank, claims_leaf, _)) => ResumeHintObservation {
                                page: page_no,
                                sequence: Some(sequence),
                                bank: Some(bank),
                                claims_leaf,
                            },
                            None => ResumeHintObservation {
                                page: page_no,
                                sequence: None,
                                bank: None,
                                claims_leaf: false,
                            },
                        })
                    },
                );
                Ok(match result {
                    Ok(value) => value,
                    Err(e) => Err(E::Runtime(e)),
                })
            },
        );
        let hint = accepted.map_err(E::Policy)??;
        stage.recheck(clock(), policy())?;
        attempt.commit();
        Ok(hint)
    }

    /// Re-read actual bytes/proof, verify, then ALWAYS rewrite+sync before a
    /// current mark. Even legitimate all-zero sparse reads take this path.
    /// Session payload/proof reads use owned copies, not a stage-loan scratch.
    #[allow(clippy::too_many_arguments)]
    pub fn reverify_and_rewrite_leaf(
        &mut self,
        runtime: &mut TransferRuntime<'_>,
        ticket: &TransferTicket,
        scratch: &mut [u8],
        grant: &mut LocalWholeSourceGrant<'_, '_>,
        pins: &PinAuthority,
        workspace: &mut SparsePageWorkspace,
        leaf: u64,
        nodes: &mut [StrongDigest],
        clock: &impl Fn() -> u64,
        policy: &impl Fn() -> Generation,
    ) -> Result<LeafBarrierObservation, E> {
        self.check_workspace(workspace)?;
        self.check_leaf(leaf)?;
        let next = self.next_sequence()?;
        let identity = *self.identity();
        let candidate = self.binding.candidate;
        let layout = self.layout;
        let page_number = self.page;
        let mut attempt = workspace.begin_attempt();
        if scratch.len() < READ_SCRATCH_BYTES {
            return Err(E::ScratchCapacity);
        }
        {
            let mut stage =
                runtime.storage_stage(ticket, &[], &[], scratch, clock(), policy())?;
            let current = policy();
            stage.recheck(clock(), current)?;
            if current.0 != identity.fields().policy_generation {
                return Err(E::IdentityMismatch);
            }
            stage.recheck(clock(), policy())?;
        }
        let length = identity.tree_spec().encoded_leaf_length(leaf)?;
        if length > crate::bundle::stream::proof::LEAF_BYTES as usize {
            return Err(E::Bounds);
        }
        let mut copied = CopiedLeafMaterial::new();
        copied.payload_len = length;
        self.session.read_exact_at(
            RangeFile::Payload,
            layout.leaf_offset(leaf)?,
            &mut copied.payload[..length],
        )?;
        self.session.read_exact_at(
            RangeFile::Proof,
            layout.proof_offset(leaf)?,
            &mut copied.proof,
        )?;
        let armed = {
            let mut stage =
                runtime.storage_stage(ticket, &[], &[], scratch, clock(), policy())?;
            match stage.with_sparse_page_buffers(
                clock(),
                policy(),
                attempt.workspace(),
                nodes,
                |control, buffers, page| {
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
                    let encoded = 1 + count * 48;
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
