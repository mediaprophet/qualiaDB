//! Bounded deficit round-robin across admitted scopes and work classes.
//!
//! Essential control, interactive service work, and background replication sit
//! in separate capped queues. Strict control priority would starve background
//! reconciliation; an uncapped control queue would let a flood monopolize the
//! host. Unknown senders are charged to the ledger transient pool before enqueue
//! (`ReservationLedger::reserve(..., verified_peer = false)`).

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::OperationId;

pub const MAX_SCOPES: usize = 8;
pub const CONTROL_CAP: usize = 4;
pub const INTERACTIVE_CAP: usize = 8;
pub const BACKGROUND_CAP: usize = 8;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkClass {
    Control = 0,
    Interactive = 1,
    Background = 2,
}

impl WorkClass {
    pub const COUNT: usize = 3;

    #[inline]
    pub const fn index(self) -> usize {
        self as usize
    }

    #[inline]
    pub const fn from_index(i: usize) -> Option<Self> {
        match i {
            0 => Some(Self::Control),
            1 => Some(Self::Interactive),
            2 => Some(Self::Background),
            _ => None,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScheduledWork {
    pub operation: OperationId,
    pub scope: u64,
    pub class: WorkClass,
    pub cost: u16,
    pub deadline: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PollOutcome {
    Ready(ScheduledWork),
    Expired(ScheduledWork),
}

#[derive(Clone, Copy)]
struct ScopeSlot {
    occupied: bool,
    id: u64,
    class_cursor: u8,
    deficit: [u32; WorkClass::COUNT],
    quantum: [u32; WorkClass::COUNT],
    control: [Option<ScheduledWork>; CONTROL_CAP],
    interactive: [Option<ScheduledWork>; INTERACTIVE_CAP],
    background: [Option<ScheduledWork>; BACKGROUND_CAP],
    control_len: u8,
    interactive_len: u8,
    background_len: u8,
}

pub struct FairScheduler {
    scopes: [ScopeSlot; MAX_SCOPES],
    cursor: u8,
}

impl FairScheduler {
    pub const fn new() -> Self {
        Self::with_quanta(1, 1, 1)
    }

    pub const fn with_quanta(control: u32, interactive: u32, background: u32) -> Self {
        let blank = ScopeSlot {
            occupied: false,
            id: 0,
            class_cursor: 0,
            deficit: [0; WorkClass::COUNT],
            quantum: [control, interactive, background],
            control: [None; CONTROL_CAP],
            interactive: [None; INTERACTIVE_CAP],
            background: [None; BACKGROUND_CAP],
            control_len: 0,
            interactive_len: 0,
            background_len: 0,
        };
        Self {
            scopes: [blank; MAX_SCOPES],
            cursor: 0,
        }
    }

    pub fn enqueue(&mut self, work: ScheduledWork) -> Result<(), QdnfError> {
        if work.cost == 0 {
            return Err(QdnfError::Malformed);
        }
        let idx = self.find_or_alloc_scope(work.scope)?;
        let scope = &mut self.scopes[idx];
        match work.class {
            WorkClass::Control => push_front_cap(&mut scope.control, &mut scope.control_len, work),
            WorkClass::Interactive => {
                push_front_cap(&mut scope.interactive, &mut scope.interactive_len, work)
            }
            WorkClass::Background => {
                push_front_cap(&mut scope.background, &mut scope.background_len, work)
            }
        }
    }

    /// One DRR step: next ready item, or `WouldBlock` if every capped queue is idle.
    pub fn poll(&mut self, now: u64) -> Result<PollOutcome, QdnfError> {
        for s in 0..MAX_SCOPES {
            let idx = (self.cursor as usize + s) % MAX_SCOPES;
            if !self.scopes[idx].occupied {
                continue;
            }
            let start = self.scopes[idx].class_cursor as usize;
            for c in 0..WorkClass::COUNT {
                let class_i = (start + c) % WorkClass::COUNT;
                if let Some(work) = self.try_take(idx, class_i) {
                    self.scopes[idx].class_cursor = ((class_i + 1) % WorkClass::COUNT) as u8;
                    self.cursor = ((idx + 1) % MAX_SCOPES) as u8;
                    self.release_scope_if_idle(idx);
                    if work.deadline < now {
                        return Ok(PollOutcome::Expired(work));
                    }
                    return Ok(PollOutcome::Ready(work));
                }
            }
        }
        Err(QdnfError::WouldBlock)
    }

    pub fn class_len(&self, scope: u64, class: WorkClass) -> usize {
        match self.scope_index(scope) {
            None => 0,
            Some(idx) => match class {
                WorkClass::Control => self.scopes[idx].control_len as usize,
                WorkClass::Interactive => self.scopes[idx].interactive_len as usize,
                WorkClass::Background => self.scopes[idx].background_len as usize,
            },
        }
    }

    fn find_or_alloc_scope(&mut self, id: u64) -> Result<usize, QdnfError> {
        if let Some(idx) = self.scope_index(id) {
            return Ok(idx);
        }
        for (i, slot) in self.scopes.iter_mut().enumerate() {
            if !slot.occupied {
                slot.occupied = true;
                slot.id = id;
                slot.class_cursor = 0;
                slot.deficit = [0; WorkClass::COUNT];
                slot.control_len = 0;
                slot.interactive_len = 0;
                slot.background_len = 0;
                slot.control = [None; CONTROL_CAP];
                slot.interactive = [None; INTERACTIVE_CAP];
                slot.background = [None; BACKGROUND_CAP];
                return Ok(i);
            }
        }
        Err(QdnfError::Capacity)
    }

    fn scope_index(&self, id: u64) -> Option<usize> {
        self.scopes
            .iter()
            .position(|s| s.occupied && s.id == id)
    }

    fn try_take(&mut self, idx: usize, class_i: usize) -> Option<ScheduledWork> {
        let scope = &mut self.scopes[idx];
        let (len, quantum) = match class_i {
            0 => (scope.control_len, scope.quantum[0]),
            1 => (scope.interactive_len, scope.quantum[1]),
            _ => (scope.background_len, scope.quantum[2]),
        };
        if len == 0 {
            scope.deficit[class_i] = 0;
            return None;
        }
        scope.deficit[class_i] = scope.deficit[class_i].saturating_add(quantum);
        let front = match class_i {
            0 => scope.control[0],
            1 => scope.interactive[0],
            _ => scope.background[0],
        }?;
        let cost = front.cost as u32;
        if cost > scope.deficit[class_i] {
            return None;
        }
        scope.deficit[class_i] -= cost;
        match class_i {
            0 => pop_front(&mut scope.control, &mut scope.control_len),
            1 => pop_front(&mut scope.interactive, &mut scope.interactive_len),
            _ => pop_front(&mut scope.background, &mut scope.background_len),
        }
    }

    fn release_scope_if_idle(&mut self, idx: usize) {
        let slot = &mut self.scopes[idx];
        if slot.control_len == 0 && slot.interactive_len == 0 && slot.background_len == 0 {
            slot.occupied = false;
            slot.id = 0;
            slot.class_cursor = 0;
            slot.deficit = [0; WorkClass::COUNT];
        }
    }
}

impl Default for FairScheduler {
    fn default() -> Self {
        Self::new()
    }
}

fn push_front_cap<const N: usize>(
    slots: &mut [Option<ScheduledWork>; N],
    len: &mut u8,
    work: ScheduledWork,
) -> Result<(), QdnfError> {
    let n = *len as usize;
    if n >= N {
        return Err(QdnfError::Capacity);
    }
    slots[n] = Some(work);
    *len = (n + 1) as u8;
    Ok(())
}

fn pop_front<const N: usize>(
    slots: &mut [Option<ScheduledWork>; N],
    len: &mut u8,
) -> Option<ScheduledWork> {
    if *len == 0 {
        return None;
    }
    let item = slots[0].take();
    let n = *len as usize;
    for i in 0..n - 1 {
        slots[i] = slots[i + 1];
    }
    *len -= 1;
    slots[*len as usize] = None;
    item
}

#[cfg(test)]
mod tests {
    use super::*;

    fn op(n: u8) -> OperationId {
        let mut id = [0u8; 16];
        id[0] = n;
        OperationId(id)
    }

    fn work(n: u8, class: WorkClass) -> ScheduledWork {
        ScheduledWork {
            operation: op(n),
            scope: 1,
            class,
            cost: 1,
            deadline: u64::MAX,
        }
    }

    #[test]
    fn control_queue_is_capped() {
        let mut sched = FairScheduler::new();
        for i in 0..CONTROL_CAP as u8 {
            sched.enqueue(work(i, WorkClass::Control)).unwrap();
        }
        assert_eq!(
            sched.enqueue(work(99, WorkClass::Control)),
            Err(QdnfError::Capacity)
        );
    }

    #[test]
    fn background_not_starved_when_control_is_capped() {
        let mut sched = FairScheduler::new();
        for i in 0..CONTROL_CAP as u8 {
            sched.enqueue(work(i, WorkClass::Control)).unwrap();
        }
        assert_eq!(
            sched.enqueue(work(80, WorkClass::Control)),
            Err(QdnfError::Capacity)
        );
        sched
            .enqueue(work(40, WorkClass::Background))
            .unwrap();
        let mut saw_background = false;
        let mut remaining_control = CONTROL_CAP;
        for _ in 0..(CONTROL_CAP + 2) {
            match sched.poll(0) {
                Ok(PollOutcome::Ready(w)) if w.class == WorkClass::Background => {
                    saw_background = true;
                    break;
                }
                Ok(PollOutcome::Ready(w)) if w.class == WorkClass::Control => {
                    remaining_control -= 1;
                }
                other => panic!("unexpected poll {other:?}"),
            }
        }
        assert!(
            saw_background,
            "background must run while control still has {remaining_control} queued"
        );
        assert!(
            remaining_control > 0,
            "fairness requires background before control drains"
        );
    }

    #[test]
    fn ninth_scope_is_capacity() {
        let mut sched = FairScheduler::new();
        for s in 0..MAX_SCOPES as u64 {
            let mut w = work(s as u8, WorkClass::Background);
            w.scope = s;
            sched.enqueue(w).unwrap();
        }
        let mut extra = work(8, WorkClass::Background);
        extra.scope = 99;
        assert_eq!(sched.enqueue(extra), Err(QdnfError::Capacity));
    }

    #[test]
    fn expired_deadline_does_not_run() {
        let mut sched = FairScheduler::new();
        let mut w = work(1, WorkClass::Interactive);
        w.deadline = 10;
        sched.enqueue(w).unwrap();
        assert_eq!(sched.poll(11), Ok(PollOutcome::Expired(w)));
    }
}
