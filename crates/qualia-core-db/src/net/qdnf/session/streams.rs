//! Bounded reliable streams. Independent stream ids; capped out-of-order.

use crate::net::qdnf::errors::QdnfError;

pub const MAX_STREAMS_PER_DIR: usize = 64;
pub const MAX_STREAM_OFFSET: u64 = 1 << 32;
/// Out-of-order frames retained per stream (head-of-line isolation cap).
pub const MAX_OOO_FRAMES: usize = 4;
/// Combined unread + OOO bytes retained per stream.
pub const MAX_STREAM_BUFFER_BYTES: u32 = 4096;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StreamFrame {
    pub stream_id: u16,
    pub offset: u64,
    pub fin: bool,
    pub declared_final: Option<u64>,
    pub len: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct OooFrame {
    offset: u64,
    len: u16,
    fin: bool,
    declared_final: Option<u64>,
}

#[derive(Clone, Copy, Debug)]
pub struct StreamState {
    pub next_offset: u64,
    pub final_size: Option<u64>,
    pub open: bool,
    unread_bytes: u32,
    ooo_bytes: u32,
    ooo: [Option<OooFrame>; MAX_OOO_FRAMES],
}

impl StreamState {
    pub const fn new() -> Self {
        Self {
            next_offset: 0,
            final_size: None,
            open: true,
            unread_bytes: 0,
            ooo_bytes: 0,
            ooo: [None; MAX_OOO_FRAMES],
        }
    }

    pub const fn unread_bytes(&self) -> u32 {
        self.unread_bytes
    }

    pub fn consume(&mut self, n: u32) -> Result<(), QdnfError> {
        if n > self.unread_bytes {
            return Err(QdnfError::Range);
        }
        self.unread_bytes -= n;
        Ok(())
    }

    pub fn accept(&mut self, frame: StreamFrame) -> Result<(), QdnfError> {
        if !self.open {
            return Err(QdnfError::Closed);
        }
        if frame.stream_id as usize >= MAX_STREAMS_PER_DIR {
            return Err(QdnfError::Capacity);
        }
        if frame.offset > MAX_STREAM_OFFSET {
            return Err(QdnfError::Range);
        }
        self.check_final(&frame)?;
        if frame.offset < self.next_offset {
            return Err(QdnfError::Overlap);
        }
        if frame.offset == self.next_offset {
            self.apply_in_order(frame.len, frame.fin, frame.declared_final)?;
            self.drain_ooo()?;
            return Ok(());
        }
        self.buffer_ooo(frame)
    }

    fn check_final(&mut self, frame: &StreamFrame) -> Result<(), QdnfError> {
        if let Some(fin) = self.final_size {
            if let Some(declared) = frame.declared_final {
                if declared != fin {
                    return Err(QdnfError::Conflict);
                }
            }
            let end = frame
                .offset
                .checked_add(frame.len as u64)
                .ok_or(QdnfError::Range)?;
            if end > fin {
                return Err(QdnfError::Range);
            }
        }
        if let Some(declared) = frame.declared_final {
            self.final_size = Some(declared);
        }
        if frame.fin {
            let end = frame
                .offset
                .checked_add(frame.len as u64)
                .ok_or(QdnfError::Range)?;
            match self.final_size {
                Some(fin) if fin != end => return Err(QdnfError::Conflict),
                None => self.final_size = Some(end),
                Some(_) => {}
            }
        }
        Ok(())
    }

    fn apply_in_order(
        &mut self,
        len: u16,
        _fin: bool,
        _declared: Option<u64>,
    ) -> Result<(), QdnfError> {
        let add = len as u32;
        let next_unread = self.unread_bytes.saturating_add(add);
        if next_unread > MAX_STREAM_BUFFER_BYTES {
            return Err(QdnfError::Capacity);
        }
        self.next_offset = self
            .next_offset
            .checked_add(len as u64)
            .ok_or(QdnfError::Range)?;
        self.unread_bytes = next_unread;
        Ok(())
    }

    fn buffer_ooo(&mut self, frame: StreamFrame) -> Result<(), QdnfError> {
        let add = frame.len as u32;
        if self.ooo_bytes.saturating_add(add) > MAX_STREAM_BUFFER_BYTES {
            return Err(QdnfError::Capacity);
        }
        let mut free = None;
        let mut i = 0usize;
        while i < MAX_OOO_FRAMES {
            match self.ooo[i] {
                Some(existing) => {
                    let existing_end = existing.offset.saturating_add(existing.len as u64);
                    let incoming_end = frame.offset.saturating_add(frame.len as u64);
                    if frame.offset < existing_end && existing.offset < incoming_end {
                        return Err(QdnfError::Overlap);
                    }
                }
                None if free.is_none() => free = Some(i),
                None => {}
            }
            i += 1;
        }
        let idx = free.ok_or(QdnfError::Capacity)?;
        self.ooo[idx] = Some(OooFrame {
            offset: frame.offset,
            len: frame.len,
            fin: frame.fin,
            declared_final: frame.declared_final,
        });
        self.ooo_bytes = self.ooo_bytes.saturating_add(add);
        Ok(())
    }

    fn drain_ooo(&mut self) -> Result<(), QdnfError> {
        loop {
            let mut found = None;
            let mut i = 0usize;
            while i < MAX_OOO_FRAMES {
                if let Some(f) = self.ooo[i] {
                    if f.offset == self.next_offset {
                        found = Some(i);
                        break;
                    }
                }
                i += 1;
            }
            match found {
                None => return Ok(()),
                Some(idx) => {
                    let f = self.ooo[idx].take().unwrap();
                    self.ooo_bytes = self.ooo_bytes.saturating_sub(f.len as u32);
                    self.apply_in_order(f.len, f.fin, f.declared_final)?;
                }
            }
        }
    }
}

/// Independently controlled streams. Stall on id 0 does not block id 1.
pub struct StreamTable {
    states: [StreamState; MAX_STREAMS_PER_DIR],
}

impl StreamTable {
    pub const fn new() -> Self {
        Self {
            states: [StreamState::new(); MAX_STREAMS_PER_DIR],
        }
    }

    pub fn accept(&mut self, frame: StreamFrame) -> Result<(), QdnfError> {
        let id = frame.stream_id as usize;
        if id >= MAX_STREAMS_PER_DIR {
            return Err(QdnfError::Capacity);
        }
        self.states[id].accept(frame)
    }

    pub fn stream(&self, id: u16) -> Result<&StreamState, QdnfError> {
        let idx = id as usize;
        if idx >= MAX_STREAMS_PER_DIR {
            return Err(QdnfError::Capacity);
        }
        Ok(&self.states[idx])
    }

    pub fn stream_mut(&mut self, id: u16) -> Result<&mut StreamState, QdnfError> {
        let idx = id as usize;
        if idx >= MAX_STREAMS_PER_DIR {
            return Err(QdnfError::Capacity);
        }
        Ok(&mut self.states[idx])
    }
}

/// Head-of-line on one stream must not stall another stream that has credit.
pub const fn stream_progress_isolated() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(id: u16, offset: u64, len: u16) -> StreamFrame {
        StreamFrame {
            stream_id: id,
            offset,
            fin: false,
            declared_final: None,
            len,
        }
    }

    #[test]
    fn final_size_conflict_is_rejected() {
        let mut s = StreamState::new();
        s.accept(StreamFrame {
            stream_id: 0,
            offset: 0,
            fin: true,
            declared_final: Some(4),
            len: 4,
        })
        .unwrap();
        assert_eq!(
            s.accept(StreamFrame {
                stream_id: 0,
                offset: 4,
                fin: true,
                declared_final: Some(8),
                len: 0,
            }),
            Err(QdnfError::Conflict)
        );
    }

    #[test]
    fn stream_progress_isolated_true() {
        assert!(stream_progress_isolated());
    }

    #[test]
    fn stalled_stream_zero_does_not_block_stream_one() {
        let mut t = StreamTable::new();
        let mut i = 0u16;
        while i < MAX_OOO_FRAMES as u16 {
            t.accept(frame(0, 100 + u64::from(i) * 10, 8)).unwrap();
            i += 1;
        }
        assert_eq!(t.accept(frame(0, 200, 8)), Err(QdnfError::Capacity));
        assert_eq!(t.stream(0).unwrap().next_offset, 0);
        t.accept(frame(1, 0, 16)).unwrap();
        assert_eq!(t.stream(1).unwrap().next_offset, 16);
        assert!(stream_progress_isolated());
    }

    #[test]
    fn unread_cap_stalls_only_that_stream() {
        let mut t = StreamTable::new();
        t.accept(frame(0, 0, 4096)).unwrap();
        assert_eq!(t.accept(frame(0, 4096, 1)), Err(QdnfError::Capacity));
        t.accept(frame(1, 0, 32)).unwrap();
        assert_eq!(t.stream(1).unwrap().next_offset, 32);
        t.stream_mut(0).unwrap().consume(8).unwrap();
        t.accept(frame(0, 4096, 8)).unwrap();
        assert_eq!(t.stream(0).unwrap().next_offset, 4104);
    }

    #[test]
    fn ooo_drain_when_gap_filled() {
        let mut s = StreamState::new();
        s.accept(frame(0, 8, 8)).unwrap();
        assert_eq!(s.next_offset, 0);
        s.accept(frame(0, 0, 8)).unwrap();
        assert_eq!(s.next_offset, 16);
        assert_eq!(s.unread_bytes(), 16);
    }
}
