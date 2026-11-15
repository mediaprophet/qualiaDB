//! Short StorageStage recheck, then drop that borrow before blocking I/O.
use super::super::{StorageStage, TransferRuntime, TransferTicket};
use super::ticket::{JobSlot, JobState, StorageJobTicket, MAX_STORAGE_JOBS};
use crate::net::qdnf::{errors::QdnfError, types::Generation};

/// Occupancy for submitted storage work. Charges stay on the original transfer.
pub struct StorageJobs {
    owner: usize,
    slots: [JobSlot; MAX_STORAGE_JOBS],
    live: usize,
}

impl StorageJobs {
    pub fn for_owner(owner: usize) -> Self {
        Self {
            owner,
            slots: [JobSlot::empty(); MAX_STORAGE_JOBS],
            live: 0,
        }
    }
    pub fn for_runtime(runtime: &TransferRuntime<'_>) -> Self {
        Self::for_owner(runtime.owner)
    }
    pub const fn live_jobs(&self) -> usize {
        self.live
    }
    pub const fn owner(&self) -> usize {
        self.owner
    }
    pub fn reserved_bytes(&self) -> u64 {
        self.slots
            .iter()
            .filter(|s| s.state != JobState::Vacant)
            .map(|s| s.reserved_bytes)
            .fold(0, |a, b| a.saturating_add(b))
    }
    pub fn buffer_held(&self, job: &StorageJobTicket) -> Result<bool, QdnfError> {
        if job.spent {
            return Ok(false);
        }
        let i = usize::from(job.job_slot);
        let slot = self.slots.get(i).ok_or(QdnfError::StaleGeneration)?;
        Ok(slot.state != JobState::Vacant && slot.generation == job.job_generation)
    }

    /// Recheck the live stage, retain already-charged buffer occupancy, drop
    /// the stage before returning. No second ReservationLedger is created.
    pub fn submit(
        &mut self,
        runtime: &mut TransferRuntime<'_>,
        ticket: &TransferTicket,
        payload: &[u8],
        proof: &[u8],
        scratch: &mut [u8],
        now: u64,
        policy: Generation,
    ) -> Result<StorageJobTicket, QdnfError> {
        let stage = runtime.storage_stage(ticket, payload, proof, scratch, now, policy)?;
        self.submit_stage(ticket, stage, now, policy)
    }

    /// Same occupancy rules when the caller already holds a short stage.
    /// The stage value is dropped before this returns.
    pub fn submit_stage(
        &mut self,
        ticket: &TransferTicket,
        mut stage: StorageStage<'_, '_>,
        now: u64,
        policy: Generation,
    ) -> Result<StorageJobTicket, QdnfError> {
        if self.owner != ticket.completion_key().owner {
            return Err(QdnfError::StaleGeneration);
        }
        if self.live >= MAX_STORAGE_JOBS {
            return Err(QdnfError::Capacity);
        }
        let key = ticket.completion_key();
        if self
            .slots
            .iter()
            .any(|s| s.state != JobState::Vacant && s.key == key)
        {
            return Err(QdnfError::Replay);
        }
        let reserved =
            stage.with_buffers(now, policy, |_, buffers| -> Result<u64, QdnfError> {
                if buffers.scratch.len() >= buffers.payload.len() {
                    buffers.scratch[..buffers.payload.len()].copy_from_slice(buffers.payload);
                }
                reserved_bytes(
                    buffers.payload.len(),
                    buffers.proof.len(),
                    buffers.scratch.len(),
                )
            })??;
        drop(stage);
        let slot = self
            .slots
            .iter()
            .position(|s| s.state == JobState::Vacant)
            .ok_or(QdnfError::Capacity)?;
        let generation = self.slots[slot].generation.next()?;
        self.slots[slot] = JobSlot {
            generation,
            state: JobState::Submitted,
            key,
            reserved_bytes: reserved,
        };
        self.live += 1;
        Ok(StorageJobTicket {
            key,
            job_generation: generation,
            job_slot: slot as u8,
            reserved_bytes: reserved,
            spent: false,
        })
    }

    /// Marks Cancelling on the job and the original transfer. The worker still
    /// owns the already-charged buffers until [`StorageJobs::complete_io`].
    pub fn cancel(
        &mut self,
        runtime: &mut TransferRuntime<'_>,
        ticket: &TransferTicket,
        job: &StorageJobTicket,
    ) -> Result<(), QdnfError> {
        let i = self.validate(runtime, ticket, job)?;
        runtime.cancel(ticket)?;
        self.slots[i].state = JobState::Cancelling;
        Ok(())
    }

    pub(super) fn validate(
        &self,
        runtime: &TransferRuntime<'_>,
        ticket: &TransferTicket,
        job: &StorageJobTicket,
    ) -> Result<usize, QdnfError> {
        runtime.validate(ticket)?;
        self.validate_job(ticket, job)
    }

    pub(super) fn validate_job(
        &self,
        ticket: &TransferTicket,
        job: &StorageJobTicket,
    ) -> Result<usize, QdnfError> {
        job.matches_transfer(ticket)?;
        if job.key.owner != self.owner {
            return Err(QdnfError::StaleGeneration);
        }
        let i = usize::from(job.job_slot);
        let slot = self.slots.get(i).ok_or(QdnfError::StaleGeneration)?;
        if slot.state == JobState::Vacant
            || slot.generation != job.job_generation
            || slot.key != job.key
        {
            return Err(QdnfError::StaleGeneration);
        }
        Ok(i)
    }

    pub(super) fn slot_mut(&mut self, i: usize) -> &mut JobSlot {
        &mut self.slots[i]
    }
    pub(super) fn slot(&self, i: usize) -> &JobSlot {
        &self.slots[i]
    }
    pub(super) fn slot_at(&self, i: usize) -> Option<&JobSlot> {
        self.slots.get(i)
    }
    pub(super) fn retire_slot(&mut self, i: usize, job: &mut StorageJobTicket) {
        self.slots[i].clear();
        self.live -= 1;
        job.spent = true;
    }
}

fn reserved_bytes(payload: usize, proof: usize, scratch: usize) -> Result<u64, QdnfError> {
    let payload = u64::try_from(payload).map_err(|_| QdnfError::Range)?;
    let proof = u64::try_from(proof).map_err(|_| QdnfError::Range)?;
    let scratch = u64::try_from(scratch).map_err(|_| QdnfError::Range)?;
    payload
        .checked_add(proof)
        .and_then(|n| n.checked_add(scratch))
        .ok_or(QdnfError::Capacity)
}
