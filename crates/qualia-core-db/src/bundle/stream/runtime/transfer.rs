//! Coupled admission and completion; only original parent tokens release budget.
use super::types::{Slot, schedule_id};
use super::{
    CancelReason, ColdPlan, CompletionKey, FinishOutcome, RequestCredit, TransferRuntime,
    TransferState, TransferTicket,
};
use crate::net::peer::runtime::{CancelEpoch, ResourceBudget, ScheduledWork};
use crate::net::qdnf::{errors::QdnfError, types::Generation};

impl TransferRuntime<'_> {
    pub fn admit(
        &mut self,
        plan: &ColdPlan<'_>,
        index: usize,
        now: u64,
        current_policy: Generation,
    ) -> Result<TransferTicket, QdnfError> {
        self.require_open()?;
        let spec = plan.row(index)?;
        let mut budget = spec.validate()?;
        if now >= spec.deadline {
            return Err(QdnfError::Expired);
        }
        if current_policy != spec.policy_generation {
            return Err(QdnfError::Revoked);
        }
        if self.live >= self.supported_inflight() {
            return Err(QdnfError::Capacity);
        }
        let slot = self
            .slots
            .iter()
            .position(|s| s.state == TransferState::Vacant)
            .ok_or(QdnfError::Capacity)?;
        let generation = self.slots[slot].generation.next()?;
        let metadata = core::mem::size_of::<TransferTicket>()
            .checked_add(core::mem::size_of::<RequestCredit>())
            .and_then(|n| n.checked_add(core::mem::size_of::<CompletionKey>()))
            .ok_or(QdnfError::Range)?;
        budget = budget.saturating_add(ResourceBudget {
            bytes: u64::try_from(metadata).map_err(|_| QdnfError::Range)?,
            work: 0,
            io: 0,
        })?;
        let window = self
            .used_bytes
            .checked_add(budget.bytes)
            .ok_or(QdnfError::Range)?;
        if window > self.profile.window_bytes() {
            return Err(QdnfError::BudgetExhausted);
        }
        let capacity = u32::try_from(budget.bytes).map_err(|_| QdnfError::Range)?;
        let serial = self
            .serial
            .checked_add(1)
            .ok_or(QdnfError::StaleGeneration)?;
        let schedule = schedule_id(self.owner, serial)?;
        if self.scheduler.contains_operation(schedule) {
            return Err(QdnfError::Conflict);
        }
        let reservation =
            self.governor
                .admit_and_reserve(self.ledger, budget, spec.class, spec.verified_peer)?;
        let lease = match self.leases.acquire(capacity, true) {
            Ok(l) => l,
            Err(e) => {
                self.ledger.release(reservation)?;
                return Err(e);
            }
        };
        let operation = match self.operations.admit(spec.operation, lease.handle) {
            Ok(h) => h,
            Err(e) => {
                let a = self.leases.release(lease.handle);
                let b = self.ledger.release(reservation);
                a?;
                b?;
                return Err(e);
            }
        };
        let work = ScheduledWork {
            operation: schedule,
            scope: spec.principal,
            class: spec.class,
            cost: spec.cost,
            deadline: spec.deadline,
        };
        if let Err(e) = self.scheduler.enqueue(work) {
            let a = self
                .operations
                .complete(operation.id, operation.generation, self.leases);
            let b = self.ledger.release(reservation);
            a?;
            b?;
            return Err(e);
        }
        self.serial = serial;
        self.used_bytes = window;
        self.live += 1;
        self.pending_records += 1;
        self.slots[slot] = Slot {
            generation,
            state: TransferState::Queued,
            operation: Some(operation),
            work: Some(work),
            queued: true,
            reservation: Some(reservation),
            budget,
            deadline: spec.deadline,
            policy: spec.policy_generation,
            credit: spec.credit_bytes,
            proof_bytes: spec.requirements.proof.bytes,
            storage_bytes: spec.requirements.storage.bytes,
            cancel_epoch: CancelEpoch::ZERO,
        };
        Ok(TransferTicket {
            key: self.key(slot)?,
            spent: false,
        })
    }

    /// Caller asserts all actual I/O/borrowers have ended. Does not certify bytes or consumption.
    pub fn complete_io(
        &mut self,
        ticket: &mut TransferTicket,
        key: CompletionKey,
        now: u64,
    ) -> Result<FinishOutcome, QdnfError> {
        let i = self.validate(ticket)?;
        if key != ticket.key {
            return Err(QdnfError::StaleGeneration);
        }
        if now >= self.slots[i].deadline {
            self.cancel_slot(i, CancelReason::Expired)?;
        }
        if self.slots[i].queued {
            return Err(QdnfError::WouldBlock);
        }
        let state = self.slots[i].state;
        if !matches!(
            state,
            TransferState::InFlight | TransferState::Cancelling(_)
        ) {
            return Err(QdnfError::WouldBlock);
        }
        let operation = self.slots[i].operation.ok_or(QdnfError::Incomplete)?;
        match self
            .operations
            .complete(operation.id, operation.generation, self.leases)
        {
            Ok(()) | Err(QdnfError::Cancelled) => {}
            Err(e) => return Err(e),
        }
        let reservation = self.slots[i]
            .reservation
            .take()
            .ok_or(QdnfError::Incomplete)?;
        self.ledger.release(reservation)?;
        self.used_bytes -= self.slots[i].budget.bytes;
        self.live -= 1;
        self.slots[i].clear();
        ticket.spent = true;
        Ok(match state {
            TransferState::Cancelling(CancelReason::Cancelled) => FinishOutcome::Cancelled,
            TransferState::Cancelling(CancelReason::Expired) => FinishOutcome::Expired,
            TransferState::Cancelling(CancelReason::Revoked) => FinishOutcome::Revoked,
            _ => FinishOutcome::Released,
        })
    }
}
