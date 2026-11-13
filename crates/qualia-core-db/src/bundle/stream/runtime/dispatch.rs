//! Fair readiness, one-shot capacity credit and retained-I/O cancellation.
use super::{
    CancelReason, RequestCredit, RuntimePoll, TransferRuntime, TransferState, TransferTicket,
};
use crate::net::peer::runtime::{CancelEpoch, PollOutcome};
use crate::net::qdnf::{errors::QdnfError, types::Generation};

impl TransferRuntime<'_> {
    pub fn poll(&mut self, now: u64) -> Result<RuntimePoll, QdnfError> {
        self.require_open()?;
        let result = self.scheduler.poll(now)?;
        let work = match result {
            PollOutcome::Ready(w) | PollOutcome::Expired(w) => w,
        };
        // Only an exact still-queued record is ours. Unrelated records always return intact.
        let Some(i) = self
            .slots
            .iter()
            .position(|s| s.queued && s.work == Some(work))
        else {
            return Ok(RuntimePoll::External(result));
        };
        self.slots[i].queued = false;
        self.pending_records -= 1;
        if now >= self.slots[i].deadline {
            self.cancel_slot(i, CancelReason::Expired)?;
        }
        if let TransferState::Cancelling(reason) = self.slots[i].state {
            return Ok(RuntimePoll::Cancelled(self.key(i)?, reason));
        }
        self.slots[i].state = TransferState::Ready;
        Ok(RuntimePoll::Ready(self.key(i)?))
    }

    pub fn issue_credit(
        &mut self,
        ticket: &TransferTicket,
        now: u64,
        current_policy: Generation,
    ) -> Result<RequestCredit, QdnfError> {
        let i = self.validate(ticket)?;
        if now >= self.slots[i].deadline {
            self.cancel_slot(i, CancelReason::Expired)?;
        }
        if current_policy != self.slots[i].policy {
            self.cancel_slot(i, CancelReason::Revoked)?;
        }
        if let TransferState::Cancelling(reason) = self.slots[i].state {
            return Err(reason.error());
        }
        if self.slots[i].state == TransferState::InFlight {
            return Err(QdnfError::Replay);
        }
        if self.slots[i].state != TransferState::Ready {
            return Err(QdnfError::WouldBlock);
        }
        self.slots[i].state = TransferState::InFlight;
        Ok(RequestCredit {
            key: ticket.key,
            bytes: self.slots[i].credit,
            cancel_epoch: self.slots[i].cancel_epoch,
        })
    }
    pub fn state(&self, ticket: &TransferTicket) -> Result<TransferState, QdnfError> {
        Ok(self.slots[self.validate(ticket)?].state)
    }
    pub fn cancel_epoch(&self, ticket: &TransferTicket) -> Result<CancelEpoch, QdnfError> {
        Ok(self.slots[self.validate(ticket)?].cancel_epoch)
    }
    pub fn cancel(&mut self, ticket: &TransferTicket) -> Result<(), QdnfError> {
        let i = self.validate(ticket)?;
        self.cancel_slot(i, CancelReason::Cancelled)
    }
    pub fn expire(&mut self, now: u64) -> Result<usize, QdnfError> {
        self.require_open()?;
        let mut expired = 0;
        for i in 0..self.slots.len() {
            if matches!(
                self.slots[i].state,
                TransferState::Queued | TransferState::Ready | TransferState::InFlight
            ) && now >= self.slots[i].deadline
            {
                self.cancel_slot(i, CancelReason::Expired)?;
                expired += 1;
            }
        }
        Ok(expired)
    }
    pub(super) fn cancel_slot(&mut self, i: usize, reason: CancelReason) -> Result<(), QdnfError> {
        if matches!(self.slots[i].state, TransferState::Cancelling(_)) {
            return Ok(());
        }
        let op = self.slots[i].operation.ok_or(QdnfError::Closed)?;
        let epoch = self.operations.cancel(op.id)?;
        self.slots[i].cancel_epoch = epoch;
        self.slots[i].state = TransferState::Cancelling(reason);
        Ok(())
    }
}
