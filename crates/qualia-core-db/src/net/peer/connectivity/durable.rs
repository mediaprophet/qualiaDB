//! Disconnection is a delivery mode. A relay write is never Delivered.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Delivery {
    QueuedLocally,
    AcceptedByCustodian,
    ReceivedByEndpoint,
    AppliedDurably,
    Expired,
    Unavailable,
}

pub const MAX_JOBS: usize = 8;
pub const MAX_BLOB: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DurableJob {
    pub op_id: u64,
    pub expiry_unix: u32,
    pub state: Delivery,
    pub len: u16,
    pub blob: [u8; MAX_BLOB],
}

impl DurableJob {
    pub const fn empty() -> Self {
        Self {
            op_id: 0,
            expiry_unix: 0,
            state: Delivery::Unavailable,
            len: 0,
            blob: [0u8; MAX_BLOB],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DurableQueue {
    jobs: [DurableJob; MAX_JOBS],
    len: usize,
}

impl DurableQueue {
    pub const fn new() -> Self {
        Self {
            jobs: [DurableJob::empty(); MAX_JOBS],
            len: 0,
        }
    }

    pub fn enqueue(&mut self, op_id: u64, expiry_unix: u32, payload: &[u8]) -> Result<usize, ()> {
        if payload.len() > MAX_BLOB || self.len >= MAX_JOBS {
            return Err(());
        }
        let mut i = 0;
        while i < self.len {
            if self.jobs[i].op_id == op_id {
                return Ok(i);
            }
            i += 1;
        }
        let mut job = DurableJob::empty();
        job.op_id = op_id;
        job.expiry_unix = expiry_unix;
        job.state = Delivery::QueuedLocally;
        job.len = payload.len() as u16;
        job.blob[..payload.len()].copy_from_slice(payload);
        self.jobs[self.len] = job;
        let idx = self.len;
        self.len += 1;
        Ok(idx)
    }

    /// Relay socket write is not this transition.
    pub fn mark_relay_write(&self) -> Delivery {
        Delivery::QueuedLocally
    }

    pub fn advance(&mut self, op_id: u64, to: Delivery, now_unix: u32) -> Result<(), ()> {
        let i = self.find(op_id).ok_or(())?;
        if now_unix >= self.jobs[i].expiry_unix {
            self.jobs[i].state = Delivery::Expired;
            return Ok(());
        }
        if !legal(self.jobs[i].state, to) {
            return Err(());
        }
        self.jobs[i].state = to;
        Ok(())
    }

    pub fn expire_due(&mut self, now_unix: u32) -> usize {
        let mut n = 0;
        let mut i = 0;
        while i < self.len {
            if self.jobs[i].state != Delivery::AppliedDurably
                && self.jobs[i].state != Delivery::Expired
                && now_unix >= self.jobs[i].expiry_unix
            {
                self.jobs[i].state = Delivery::Expired;
                n += 1;
            }
            i += 1;
        }
        n
    }

    pub fn state_of(&self, op_id: u64) -> Option<Delivery> {
        self.find(op_id).map(|i| self.jobs[i].state)
    }

    pub(crate) fn len(&self) -> usize {
        self.len
    }

    pub(crate) fn job_at(&self, i: usize) -> Option<DurableJob> {
        if i < self.len {
            Some(self.jobs[i])
        } else {
            None
        }
    }

    pub(crate) fn restore(&mut self, job: DurableJob) -> Result<(), ()> {
        if self.len >= MAX_JOBS {
            return Err(());
        }
        self.jobs[self.len] = job;
        self.len += 1;
        Ok(())
    }

    fn find(&self, op_id: u64) -> Option<usize> {
        let mut i = 0;
        while i < self.len {
            if self.jobs[i].op_id == op_id {
                return Some(i);
            }
            i += 1;
        }
        None
    }
}

fn legal(from: Delivery, to: Delivery) -> bool {
    use Delivery::*;
    matches!(
        (from, to),
        (QueuedLocally, AcceptedByCustodian)
            | (QueuedLocally, Expired)
            | (QueuedLocally, Unavailable)
            | (AcceptedByCustodian, ReceivedByEndpoint)
            | (AcceptedByCustodian, Expired)
            | (AcceptedByCustodian, Unavailable)
            | (ReceivedByEndpoint, AppliedDurably)
            | (ReceivedByEndpoint, Expired)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relay_write_is_not_delivered() {
        let q = DurableQueue::new();
        assert_eq!(q.mark_relay_write(), Delivery::QueuedLocally);
    }

    #[test]
    fn ram_dedup_and_expiry_is_not_persistence() {
        let mut q = DurableQueue::new();
        assert_eq!(q.enqueue(9, 100, b"op").unwrap(), 0);
        assert_eq!(q.enqueue(9, 100, b"op").unwrap(), 0);
        q.advance(9, Delivery::AcceptedByCustodian, 50).unwrap();
        q.advance(9, Delivery::ReceivedByEndpoint, 50).unwrap();
        q.advance(9, Delivery::AppliedDurably, 50).unwrap();
        assert_eq!(q.state_of(9), Some(Delivery::AppliedDurably));
        let mut q2 = DurableQueue::new();
        q2.enqueue(1, 10, b"x").unwrap();
        assert_eq!(q2.expire_due(10), 1);
        assert_eq!(q2.state_of(1), Some(Delivery::Expired));
    }
}
