//! Native write arming: drop stage/grant/parent locks before the OS wait.
use super::checkpoint::{write_verified_bytes, LeafIoOp};
use super::write::WriteStep;
use super::{NativeSparseStore, StoreError as E, StoreState, PROOF_SLOT_BYTES};
use crate::bundle::stream::{
    policy::LocalWholeSourceGrant,
    proof::LEAF_BYTES,
    runtime::{StorageStage, TransferRuntime, TransferTicket},
    storage::{BITMAP_BYTES, CHECKPOINT_BYTES},
};
use crate::crypto::network::authority::PinAuthority;
use crate::net::qdnf::types::Generation;

/// Armed write holding no StorageStage, grant, RuntimeParents, or
/// CurrentAuthorityGuard. Bytes remain the caller's already-charged buffers.
#[must_use = "drop parent guards, then call wait_blocking"]
#[derive(Debug)]
pub struct ArmedNativeWrite {
    reserved_bytes: u64,
}

impl ArmedNativeWrite {
    pub const fn reserved_bytes(&self) -> u64 {
        self.reserved_bytes
    }

    /// Blocking wait site. This function takes no StorageStage,
    /// LocalWholeSourceGrant, CurrentAuthorityGuard, or RuntimeParents.
    /// Mutex<AuthorityOwner> / RuntimeParents are not held across this wait.
    pub fn wait_blocking<R>(self, hook: impl FnOnce() -> R) -> R {
        // OS wait (analogous to WriteStep::PayloadWrite). Caller must have
        // dropped stage, grant, and parent mutexes before this call.
        hook()
    }
}

/// Production leaf I/O plan. Owns copied payload/proof/checkpoint bytes.
/// Holds no StorageStage, grant, RuntimeParents, or CurrentAuthorityGuard.
#[must_use = "drop parent guards, then call wait_blocking"]
pub(crate) struct ArmedNativeLeafWrite {
    reserved_bytes: u64,
    payload_offset: u64,
    proof_offset: u64,
    checkpoint_base: u64,
    payload_len: usize,
    payload: [u8; LEAF_BYTES as usize],
    proof: [u8; PROOF_SLOT_BYTES],
    header: [u8; CHECKPOINT_BYTES],
    bitmap: [u8; BITMAP_BYTES],
}

impl core::fmt::Debug for ArmedNativeLeafWrite {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ArmedNativeLeafWrite")
            .field("reserved_bytes", &self.reserved_bytes)
            .field("payload_len", &self.payload_len)
            .finish_non_exhaustive()
    }
}

impl ArmedNativeLeafWrite {
    pub const fn reserved_bytes(&self) -> u64 {
        self.reserved_bytes
    }

    pub(super) fn from_prepared(
        payload_offset: u64,
        proof_offset: u64,
        checkpoint_base: u64,
        payload: &[u8],
        proof: &[u8],
        header: &[u8],
        bitmap: &[u8],
    ) -> Result<Self, E> {
        if payload.len() > LEAF_BYTES as usize
            || proof.len() > PROOF_SLOT_BYTES
            || header.len() != CHECKPOINT_BYTES
            || bitmap.len() != BITMAP_BYTES
        {
            return Err(E::Bounds);
        }
        let payload_len = payload.len();
        let mut owned = Self {
            reserved_bytes: 0,
            payload_offset,
            proof_offset,
            checkpoint_base,
            payload_len,
            payload: [0; LEAF_BYTES as usize],
            proof: [0; PROOF_SLOT_BYTES],
            header: [0; CHECKPOINT_BYTES],
            bitmap: [0; BITMAP_BYTES],
        };
        owned.payload[..payload_len].copy_from_slice(payload);
        owned.proof[..proof.len()].copy_from_slice(proof);
        owned.header.copy_from_slice(header);
        owned.bitmap.copy_from_slice(bitmap);
        let payload_n = u64::try_from(payload_len).map_err(|_| E::Overflow)?;
        let proof_n = u64::try_from(proof.len()).map_err(|_| E::Overflow)?;
        let header_n = u64::try_from(header.len()).map_err(|_| E::Overflow)?;
        let bitmap_n = u64::try_from(bitmap.len()).map_err(|_| E::Overflow)?;
        owned.reserved_bytes = payload_n
            .checked_add(proof_n)
            .and_then(|n| n.checked_add(header_n))
            .and_then(|n| n.checked_add(bitmap_n))
            .ok_or(E::Overflow)?;
        Ok(owned)
    }

    #[cfg(test)]
    fn for_wait_site_test(reserved_bytes: u64) -> Self {
        Self {
            reserved_bytes,
            payload_offset: 0,
            proof_offset: 0,
            checkpoint_base: 0,
            payload_len: 0,
            payload: [0; LEAF_BYTES as usize],
            proof: [0; PROOF_SLOT_BYTES],
            header: [0; CHECKPOINT_BYTES],
            bitmap: [0; BITMAP_BYTES],
        }
    }

    /// Production wait site. Takes no StorageStage, LocalWholeSourceGrant,
    /// CurrentAuthorityGuard, or RuntimeParents. Mutex<AuthorityOwner> /
    /// RuntimeParents are not held across this wait. Cancellation is observed
    /// at Phase 3 recheck; this wait does not call gates().
    pub(crate) fn wait_blocking(
        self,
        hook: &mut impl FnMut(WriteStep) -> Result<(), E>,
        io: &mut impl FnMut(LeafIoOp, &[u8]) -> Result<(), E>,
    ) -> Result<(), E> {
        write_verified_bytes(
            self.payload_offset,
            self.proof_offset,
            self.checkpoint_base,
            &self.payload[..self.payload_len],
            &self.proof,
            &self.header,
            &self.bitmap,
            hook,
            io,
        )
    }
}

/// Recheck a short StorageStage, retain already-charged occupancy, drop the
/// stage before returning. No NativeSparseStore borrow is required.
pub fn arm_write_from_stage(
    runtime: &mut TransferRuntime<'_>,
    ticket: &TransferTicket,
    payload: &[u8],
    proof: &[u8],
    scratch: &mut [u8],
    now: u64,
    policy: Generation,
) -> Result<ArmedNativeWrite, E> {
    let mut stage = runtime.storage_stage(ticket, payload, proof, scratch, now, policy)?;
    arm_from_live_stage(&mut stage, now, policy)
}

fn arm_from_live_stage(
    stage: &mut StorageStage<'_, '_>,
    now: u64,
    policy: Generation,
) -> Result<ArmedNativeWrite, E> {
    let reserved = stage.with_buffers(now, policy, |_, buffers| -> Result<u64, E> {
        if buffers.scratch.len() >= buffers.payload.len() {
            buffers.scratch[..buffers.payload.len()].copy_from_slice(buffers.payload);
        }
        let payload = u64::try_from(buffers.payload.len()).map_err(|_| E::Overflow)?;
        let proof = u64::try_from(buffers.proof.len()).map_err(|_| E::Overflow)?;
        let scratch = u64::try_from(buffers.scratch.len()).map_err(|_| E::Overflow)?;
        payload
            .checked_add(proof)
            .and_then(|n| n.checked_add(scratch))
            .ok_or(E::Overflow)
    })??;
    Ok(ArmedNativeWrite {
        reserved_bytes: reserved,
    })
}

impl NativeSparseStore<'_, '_> {
    /// Recheck grant and stage, copy/retain already-charged buffers, then
    /// return. Caller must drop `grant` and any RuntimeParents /
    /// CurrentAuthorityGuard before [`ArmedNativeWrite::wait_blocking`].
    #[allow(clippy::too_many_arguments)]
    pub fn arm_leaf_write(
        &mut self,
        runtime: &mut TransferRuntime<'_>,
        ticket: &TransferTicket,
        grant: &mut LocalWholeSourceGrant<'_, '_>,
        pins: &PinAuthority,
        payload: &[u8],
        proof: &[u8],
        scratch: &mut [u8],
        clock: &impl Fn() -> u64,
        policy: &impl Fn() -> Generation,
    ) -> Result<ArmedNativeWrite, E> {
        if self.state != StoreState::Active {
            return Err(E::Poisoned);
        }
        let candidate = self.binding.candidate;
        grant
            .accept_local_scoped(
                pins,
                || clock(),
                |authority| {
                    if authority.closure().identity() != candidate {
                        return Ok(Err(E::IdentityMismatch));
                    }
                    let mut stage = match runtime.storage_stage(
                        ticket,
                        payload,
                        proof,
                        scratch,
                        clock(),
                        policy(),
                    ) {
                        Ok(stage) => stage,
                        Err(e) => return Ok(Err(E::Runtime(e))),
                    };
                    Ok(arm_from_live_stage(&mut stage, clock(), policy()))
                },
            )
            .map_err(E::Policy)?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::stream::runtime::{
        ColdPlan, FinishOutcome, RuntimeParents, RuntimePoll, RuntimeProfile, TransferRequirements,
        TransferRuntime, TransferSpec, TransferTicket,
    };
    use crate::net::peer::runtime::{
        FairScheduler, LeaseTable, OperationTable, ReservationLedger, ResourceBudget,
        ResourceGovernor, WorkClass,
    };
    use crate::net::qdnf::{
        authority::{AuthorityOwner, CurrentAuthorityGuard},
        types::{Generation, OperationId},
    };
    use std::sync::Mutex;

    fn with_runtime(run: impl FnOnce(&mut TransferRuntime<'_>)) {
        let cap = ResourceBudget {
            bytes: 64 * 1024 * 1024,
            work: 10000,
            io: 10000,
        };
        let mut ledger = ReservationLedger::new(cap, cap, cap, cap, cap);
        let mut leases = LeaseTable::new();
        let mut ops = OperationTable::new();
        let mut scheduler = FairScheduler::new();
        let governor = ResourceGovernor::new(4096);
        let mut rt = TransferRuntime::new(
            &mut ledger,
            &mut leases,
            &mut ops,
            &mut scheduler,
            &governor,
            RuntimeProfile::Constrained,
            ResourceBudget {
                bytes: 4096,
                work: 1024,
                io: 64,
            },
        )
        .unwrap();
        run(&mut rt);
        assert_eq!(rt.live_transfers(), 0);
        for _ in 0..160 {
            if rt.pending_schedule_records() == 0 {
                break;
            }
            let _ = rt.poll(1000);
        }
        rt.close().unwrap();
    }

    fn ready(rt: &mut TransferRuntime<'_>, ticket: &TransferTicket) {
        let RuntimePoll::Ready(key) = rt.poll(0).unwrap() else {
            panic!("expected ready")
        };
        assert_eq!(key, ticket.completion_key());
        let _credit = rt.issue_credit(ticket, 0, Generation(1)).unwrap();
    }

    fn spec() -> TransferSpec {
        let part = ResourceBudget {
            bytes: 16,
            work: 1,
            io: 1,
        };
        TransferSpec {
            operation: OperationId([9; 16]),
            principal: 1,
            policy_generation: Generation(1),
            deadline: 100,
            class: WorkClass::Background,
            cost: 1,
            credit_bytes: 16,
            requirements: TransferRequirements {
                network: part,
                proof: part,
                storage: part,
                consumer: part,
                scratch: part,
            },
            verified_peer: true,
        }
    }

    #[test]
    fn write_drops_stage_before_simulated_blocking_hook() {
        with_runtime(|rt| {
            let rows = [spec()];
            let plan = ColdPlan::compile(&rows).unwrap();
            let mut ticket = rt.admit(&plan, 0, 0, Generation(1)).unwrap();
            ready(rt, &ticket);
            let payload = [7; 16];
            let proof = [8; 16];
            let mut scratch = [0; 16];
            let armed = arm_write_from_stage(
                rt,
                &ticket,
                &payload,
                &proof,
                &mut scratch,
                0,
                Generation(1),
            )
            .unwrap();
            assert_eq!(scratch[0], 7);
            let mut saw_wait = false;
            armed.wait_blocking(|| {
                saw_wait = true;
                // No live StorageStage borrow: exclusive runtime use is legal.
                assert_eq!(rt.live_transfers(), 1);
                rt.cancel(&ticket).unwrap();
            });
            assert!(saw_wait);
            let key = ticket.completion_key();
            assert_eq!(
                rt.complete_io(&mut ticket, key, 1),
                Ok(FinishOutcome::Cancelled)
            );
        });
    }

    #[test]
    fn production_write_wait_site_takes_no_stage_grant_or_parent_mutex() {
        let cap = ResourceBudget {
            bytes: 64 * 1024 * 1024,
            work: 10000,
            io: 10000,
        };
        let ledger = Mutex::new(ReservationLedger::new(cap, cap, cap, cap, cap));
        let mut leases = Mutex::new(LeaseTable::new());
        let mut operations = Mutex::new(OperationTable::new());
        let mut scheduler = Mutex::new(FairScheduler::new());
        let governor = ResourceGovernor::new(4096);
        let mut parents = RuntimeParents::adopt(
            &ledger,
            &mut leases,
            &mut operations,
            &mut scheduler,
            &governor,
        )
        .unwrap();
        let authority = Mutex::new(AuthorityOwner::new());
        let mut state = parents
            .admit(
                RuntimeProfile::Constrained,
                ResourceBudget {
                    bytes: 4096,
                    work: 1024,
                    io: 64,
                },
            )
            .unwrap();
        let rows = [spec()];
        let plan = ColdPlan::compile(&rows).unwrap();
        let mut ticket = parents
            .try_with_runtime(&mut state, |mut rt| rt.admit(&plan, 0, 0, Generation(1)))
            .unwrap();
        parents
            .try_with_runtime(&mut state, |mut rt| {
                let RuntimePoll::Ready(key) = rt.poll(0)? else {
                    panic!("expected ready")
                };
                assert_eq!(key, ticket.completion_key());
                let _credit = rt.issue_credit(&ticket, 0, Generation(1))?;
                Ok(())
            })
            .unwrap();
        let payload = [7; 16];
        let proof = [8; 16];
        let mut scratch = [0; 16];
        {
            let _guard = CurrentAuthorityGuard::try_lock(&authority).unwrap();
            parents
                .try_with_runtime(&mut state, |mut rt| {
                    let _stage = rt.storage_stage(
                        &ticket,
                        &payload,
                        &proof,
                        &mut scratch,
                        0,
                        Generation(1),
                    )?;
                    Ok(())
                })
                .unwrap();
        }
        assert!(ledger.try_lock().is_ok());
        assert!(CurrentAuthorityGuard::try_lock(&authority).is_ok());
        let mut saw_payload_write = false;
        ArmedNativeLeafWrite::for_wait_site_test(16)
            .wait_blocking(
                &mut |step| {
                    if step == WriteStep::PayloadWrite {
                        saw_payload_write = true;
                        assert!(ledger.try_lock().is_ok());
                        assert!(CurrentAuthorityGuard::try_lock(&authority).is_ok());
                    }
                    Ok(())
                },
                &mut |_, _| Ok(()),
            )
            .unwrap();
        assert!(saw_payload_write);
        parents
            .try_with_runtime(&mut state, |mut rt| {
                let key = ticket.completion_key();
                rt.complete_io(&mut ticket, key, 1)?;
                Ok(())
            })
            .unwrap();
        state.try_close().unwrap();
        parents.try_close().unwrap();
        assert_eq!(ledger.lock().unwrap().used().host, ResourceBudget::ZERO);
    }

    #[test]
    fn write_leaf_drops_stage_before_payload_write_hook() {
        with_runtime(|rt| {
            let rows = [spec()];
            let plan = ColdPlan::compile(&rows).unwrap();
            let mut ticket = rt.admit(&plan, 0, 0, Generation(1)).unwrap();
            ready(rt, &ticket);
            let payload = [7; 16];
            let proof = [8; 16];
            let mut scratch = [0; 16];
            let occupancy = arm_write_from_stage(
                rt,
                &ticket,
                &payload,
                &proof,
                &mut scratch,
                0,
                Generation(1),
            )
            .unwrap();
            let reserved = occupancy.reserved_bytes();
            drop(occupancy);
            let mut first = None;
            ArmedNativeLeafWrite::for_wait_site_test(reserved)
                .wait_blocking(
                    &mut |step| {
                        if first.is_none() {
                            first = Some(step);
                        }
                        if step == WriteStep::PayloadWrite {
                            assert_eq!(rt.live_transfers(), 1);
                            rt.cancel(&ticket).unwrap();
                        }
                        Ok(())
                    },
                    &mut |_, _| Ok(()),
                )
                .unwrap();
            assert_eq!(first, Some(WriteStep::PayloadWrite));
            let key = ticket.completion_key();
            assert_eq!(
                rt.complete_io(&mut ticket, key, 1),
                Ok(FinishOutcome::Cancelled)
            );
        });
    }
}
