//! Loss, reorder, duplicate, corruption, delay and partition injection.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::harness::partition::PartitionGate;

pub const PIPE_SLOTS: usize = 8;
pub const FRAME_CAP: usize = 512;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fault {
    None = 0,
    Drop = 1,
    Duplicate = 2,
    Reorder = 3,
    Corrupt = 4,
    Truncate = 5,
    Delay = 6,
    Partition = 7,
}

#[derive(Clone, Copy, Debug)]
pub struct FaultSchedule {
    pub pattern: [Fault; PIPE_SLOTS],
    pub index: usize,
}

impl FaultSchedule {
    pub const fn never() -> Self {
        Self {
            pattern: [Fault::None; PIPE_SLOTS],
            index: 0,
        }
    }

    pub const fn with_pattern(pattern: [Fault; PIPE_SLOTS]) -> Self {
        Self { pattern, index: 0 }
    }

    pub fn next(&mut self) -> Fault {
        let f = self.pattern[self.index % self.pattern.len()];
        self.index = self.index.wrapping_add(1);
        f
    }

    /// In-place mutate for Drop/Corrupt/Truncate. Duplicate/Reorder/Delay/Partition
    /// need [`FaultPipe`]; this helper does not queue.
    pub fn apply<'a>(&self, fault: Fault, frame: &'a mut [u8]) -> Option<&'a [u8]> {
        match fault {
            Fault::None => Some(frame),
            Fault::Drop | Fault::Partition => None,
            Fault::Duplicate | Fault::Reorder | Fault::Delay => Some(frame),
            Fault::Corrupt => {
                if !frame.is_empty() {
                    frame[0] ^= 0xff;
                }
                Some(frame)
            }
            Fault::Truncate => {
                if frame.len() > 4 {
                    Some(&frame[..frame.len() / 2])
                } else {
                    None
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
struct Slot {
    len: u16,
    delay: u8,
    bytes: [u8; FRAME_CAP],
}

impl Slot {
    const EMPTY: Self = Self {
        len: 0,
        delay: 0,
        bytes: [0u8; FRAME_CAP],
    };
}

/// Bounded eight-slot pipe. Duplicate, reorder and delay actually queue.
pub struct FaultPipe {
    schedule: FaultSchedule,
    slots: [Slot; PIPE_SLOTS],
    len: usize,
    gate: PartitionGate,
}

impl FaultPipe {
    pub const fn new() -> Self {
        Self::with_schedule(FaultSchedule::never())
    }

    pub const fn with_schedule(schedule: FaultSchedule) -> Self {
        Self {
            schedule,
            slots: [Slot::EMPTY; PIPE_SLOTS],
            len: 0,
            gate: PartitionGate::open(),
        }
    }

    pub const fn queued(&self) -> usize {
        self.len
    }

    pub const fn is_partitioned(&self) -> bool {
        self.gate.is_partitioned()
    }

    pub fn partition(&mut self) {
        self.gate.partition();
    }

    pub fn heal(&mut self) {
        self.gate.heal();
    }

    /// Drop in-flight slots. Does not model crash-consistent storage (QA-01.06).
    pub fn restart(&mut self) {
        self.slots = [Slot::EMPTY; PIPE_SLOTS];
        self.len = 0;
        self.gate.heal();
    }

    /// Decrement delay on every queued slot so delayed/reordered frames can complete.
    pub fn tick(&mut self) {
        let mut i = 0;
        while i < self.len {
            if self.slots[i].delay > 0 {
                self.slots[i].delay -= 1;
            }
            i += 1;
        }
    }

    pub fn push(&mut self, frame: &[u8]) -> Result<(), QdnfError> {
        self.gate.admit()?;
        if frame.len() > FRAME_CAP {
            return Err(QdnfError::Capacity);
        }
        let fault = self.schedule.next();
        match fault {
            Fault::None => self.enqueue(frame, 0),
            Fault::Drop => Ok(()),
            Fault::Duplicate => {
                if self.len + 2 > PIPE_SLOTS {
                    return Err(QdnfError::Capacity);
                }
                self.enqueue(frame, 0)?;
                self.enqueue(frame, 0)
            }
            Fault::Reorder => self.enqueue(frame, 1),
            Fault::Corrupt => {
                let mut tmp = [0u8; FRAME_CAP];
                tmp[..frame.len()].copy_from_slice(frame);
                if !frame.is_empty() {
                    tmp[0] ^= 0xff;
                }
                self.enqueue(&tmp[..frame.len()], 0)
            }
            Fault::Truncate => {
                if frame.len() > 4 {
                    self.enqueue(&frame[..frame.len() / 2], 0)
                } else {
                    Ok(())
                }
            }
            Fault::Delay => self.enqueue(frame, 1),
            Fault::Partition => {
                self.gate.partition();
                Err(QdnfError::WouldBlock)
            }
        }
    }

    pub fn pop(&mut self, out: &mut [u8]) -> Result<usize, QdnfError> {
        self.gate.admit()?;
        let mut i = 0;
        while i < self.len {
            if self.slots[i].delay == 0 {
                let n = self.slots[i].len as usize;
                if out.len() < n {
                    return Err(QdnfError::Capacity);
                }
                if n > 0 {
                    out[..n].copy_from_slice(&self.slots[i].bytes[..n]);
                }
                self.remove(i);
                return Ok(n);
            }
            i += 1;
        }
        Err(QdnfError::WouldBlock)
    }

    fn enqueue(&mut self, frame: &[u8], delay: u8) -> Result<(), QdnfError> {
        if self.len >= PIPE_SLOTS {
            return Err(QdnfError::Capacity);
        }
        let slot = &mut self.slots[self.len];
        slot.len = frame.len() as u16;
        slot.delay = delay;
        if !frame.is_empty() {
            slot.bytes[..frame.len()].copy_from_slice(frame);
        }
        self.len += 1;
        Ok(())
    }

    fn remove(&mut self, i: usize) {
        let mut j = i;
        while j + 1 < self.len {
            self.slots[j] = self.slots[j + 1];
            j += 1;
        }
        self.len -= 1;
        self.slots[self.len] = Slot::EMPTY;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pattern(first: Fault) -> FaultSchedule {
        let mut pattern = [Fault::None; PIPE_SLOTS];
        pattern[0] = first;
        FaultSchedule::with_pattern(pattern)
    }

    #[test]
    fn success() {
        let mut pipe = FaultPipe::new();
        let frame = [1u8, 2, 3, 4];
        pipe.push(&frame).unwrap();
        let mut out = [0u8; 8];
        let n = pipe.pop(&mut out).unwrap();
        assert_eq!(&out[..n], &frame);
        assert_eq!(pipe.pop(&mut out), Err(QdnfError::WouldBlock));
    }

    #[test]
    fn drop_discards() {
        let mut pipe = FaultPipe::with_schedule(pattern(Fault::Drop));
        pipe.push(&[9, 8, 7, 6]).unwrap();
        assert_eq!(pipe.queued(), 0);
        let mut out = [0u8; 8];
        assert_eq!(pipe.pop(&mut out), Err(QdnfError::WouldBlock));
    }

    #[test]
    fn partition_heal() {
        let mut pipe = FaultPipe::with_schedule(pattern(Fault::Partition));
        assert_eq!(pipe.push(&[1, 2, 3, 4]), Err(QdnfError::WouldBlock));
        assert!(pipe.is_partitioned());
        let mut out = [0u8; 8];
        assert_eq!(pipe.pop(&mut out), Err(QdnfError::WouldBlock));
        pipe.heal();
        pipe.push(&[1, 2, 3, 4]).unwrap();
        let n = pipe.pop(&mut out).unwrap();
        assert_eq!(&out[..n], &[1, 2, 3, 4]);
    }

    #[test]
    fn corrupt() {
        let mut pipe = FaultPipe::with_schedule(pattern(Fault::Corrupt));
        pipe.push(&[0x0f, 0x00]).unwrap();
        let mut out = [0u8; 8];
        let n = pipe.pop(&mut out).unwrap();
        assert_eq!(&out[..n], &[0xf0, 0x00]);
    }

    #[test]
    fn truncate() {
        let mut pipe = FaultPipe::with_schedule(pattern(Fault::Truncate));
        pipe.push(&[1, 2, 3, 4, 5, 6, 7, 8]).unwrap();
        let mut out = [0u8; 8];
        let n = pipe.pop(&mut out).unwrap();
        assert_eq!(&out[..n], &[1, 2, 3, 4]);
    }

    #[test]
    fn duplicate_queues_two() {
        let mut pipe = FaultPipe::with_schedule(pattern(Fault::Duplicate));
        pipe.push(&[7, 7]).unwrap();
        assert_eq!(pipe.queued(), 2);
        let mut out = [0u8; 8];
        let n1 = pipe.pop(&mut out).unwrap();
        assert_eq!(&out[..n1], &[7, 7]);
        let n2 = pipe.pop(&mut out).unwrap();
        assert_eq!(&out[..n2], &[7, 7]);
    }

    #[test]
    fn reorder_lets_later_frame_complete_first() {
        let mut pattern = [Fault::None; PIPE_SLOTS];
        pattern[0] = Fault::Reorder;
        let mut pipe = FaultPipe::with_schedule(FaultSchedule::with_pattern(pattern));
        pipe.push(&[1]).unwrap();
        pipe.push(&[2]).unwrap();
        let mut out = [0u8; 8];
        let n1 = pipe.pop(&mut out).unwrap();
        assert_eq!(&out[..n1], &[2]);
        pipe.tick();
        let n2 = pipe.pop(&mut out).unwrap();
        assert_eq!(&out[..n2], &[1]);
    }

    #[test]
    fn delay_completes_after_tick() {
        let mut pipe = FaultPipe::with_schedule(pattern(Fault::Delay));
        pipe.push(&[3, 4]).unwrap();
        let mut out = [0u8; 8];
        assert_eq!(pipe.pop(&mut out), Err(QdnfError::WouldBlock));
        pipe.tick();
        let n = pipe.pop(&mut out).unwrap();
        assert_eq!(&out[..n], &[3, 4]);
    }

    #[test]
    fn restart_drops_inflight() {
        let mut pipe = FaultPipe::new();
        pipe.push(&[1, 2]).unwrap();
        pipe.restart();
        assert_eq!(pipe.queued(), 0);
        let mut out = [0u8; 8];
        assert_eq!(pipe.pop(&mut out), Err(QdnfError::WouldBlock));
        pipe.push(&[9]).unwrap();
        let n = pipe.pop(&mut out).unwrap();
        assert_eq!(&out[..n], &[9]);
    }
}
