//! Exactly-once completion. Late cancel reclaim cannot publish verified bits.
use super::super::{FinishOutcome, TransferRuntime, TransferState, TransferTicket};
use super::ticket::{JobState, StorageJobTicket};
use super::StorageJobs;
use crate::net::qdnf::errors::QdnfError;

impl StorageJobs {
    /// True only while the original transfer is still InFlight and the job was
    /// not cancelled or failed. Call after I/O, before [`Self::complete_io`].
    pub fn may_publish_verified(
        &self,
        runtime: &TransferRuntime<'_>,
        ticket: &TransferTicket,
        job: &StorageJobTicket,
    ) -> Result<bool, QdnfError> {
        let i = self.validate(runtime, ticket, job)?;
        if self.slot(i).state != JobState::Submitted {
            return Ok(false);
        }
        Ok(runtime.state(ticket)? == TransferState::InFlight)
    }

    /// Worker error/panic: keep the original charge and buffer occupancy.
    /// The slot is not vacant; [`Self::complete_io`] is still required.
    pub fn observe_worker_failure(
        &mut self,
        runtime: &mut TransferRuntime<'_>,
        ticket: &TransferTicket,
        job: &StorageJobTicket,
    ) -> Result<(), QdnfError> {
        let i = self.validate(runtime, ticket, job)?;
        runtime.cancel(ticket)?;
        self.slot_mut(i).state = JobState::Failed;
        Ok(())
    }

    /// Exactly-once parent complete plus job retirement. A second call refuses
    /// without touching the original ledger again.
    pub fn complete_io(
        &mut self,
        runtime: &mut TransferRuntime<'_>,
        ticket: &mut TransferTicket,
        job: &mut StorageJobTicket,
        now: u64,
    ) -> Result<FinishOutcome, QdnfError> {
        let i = self.validate(runtime, ticket, job)?;
        let key = job.key;
        let outcome = runtime.complete_io(ticket, key, now)?;
        self.retire_slot(i, job);
        Ok(outcome)
    }

    /// Job-table half after a parent `complete_io` performed through another
    /// access path (for example a short RuntimeParents scope). Does not refund.
    pub fn take_completed(
        &mut self,
        ticket: &TransferTicket,
        job: &mut StorageJobTicket,
    ) -> Result<(), QdnfError> {
        if !ticket.spent {
            return Err(QdnfError::WouldBlock);
        }
        job.matches_transfer(ticket)?;
        let i = self.validate_job_after_parent(job)?;
        self.retire_slot(i, job);
        Ok(())
    }

    fn validate_job_after_parent(&self, job: &StorageJobTicket) -> Result<usize, QdnfError> {
        if job.spent {
            return Err(QdnfError::DoubleRelease);
        }
        if job.key.owner != self.owner() {
            return Err(QdnfError::StaleGeneration);
        }
        let i = usize::from(job.job_slot);
        let slot = match self.slot_at(i) {
            Some(slot) => slot,
            None => return Err(QdnfError::StaleGeneration),
        };
        if slot.state == JobState::Vacant
            || slot.generation != job.job_generation
            || slot.key != job.key
        {
            return Err(QdnfError::StaleGeneration);
        }
        Ok(i)
    }
}
