use super::{
    checkpoint::store_verified,
    read::read_material,
    write::{gates, WriteStep},
};
use super::{
    LeafBarrierObservation, NativeSparseStore, StoreError as E, StoreState, CHECKPOINT_BANK_BYTES,
};
use crate::bundle::stream::{
    policy::LocalWholeSourceGrant,
    runtime::{SparsePageWorkspace, StorageStage},
    storage::{decode_checkpoint, BITMAP_BYTES, CHECKPOINT_BYTES},
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
    #[allow(clippy::too_many_arguments)]
    pub fn reverify_and_rewrite_leaf(
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
        self.check_workspace(workspace)?;
        self.check_leaf(leaf)?;
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
                let result = stage.with_sparse_page_buffers(
                    clock(),
                    policy(),
                    attempt.workspace(),
                    nodes,
                    |control, buffers, page| {
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
                        self.state = StoreState::Poisoned;
                        store_verified(
                            &mut self.session,
                            &identity,
                            &material.view,
                            material.proof_slot,
                            material.header,
                            page,
                            control,
                            authority,
                            layout,
                            next,
                            self.backend_attempt,
                            clock,
                            policy,
                            &mut |_: WriteStep| Ok(()),
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
        attempt.commit();
        self.sequence = next;
        self.state = StoreState::Active;
        Ok(observation)
    }
}
