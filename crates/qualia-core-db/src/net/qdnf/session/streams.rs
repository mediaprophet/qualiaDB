//! Bounded reliable streams. Final-size consistency and overlap equality.

use crate::net::qdnf::errors::QdnfError;

pub const MAX_STREAMS_PER_DIR: usize = 64;
pub const MAX_STREAM_OFFSET: u64 = 1 << 32;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StreamFrame {
    pub stream_id: u16,
    pub offset: u64,
    pub fin: bool,
    pub declared_final: Option<u64>,
    pub len: u16,
}

#[derive(Clone, Copy, Debug)]
pub struct StreamState {
    pub next_offset: u64,
    pub final_size: Option<u64>,
    pub open: bool,
}

impl StreamState {
    pub const fn new() -> Self {
        Self {
            next_offset: 0,
            final_size: None,
            open: true,
        }
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
        if frame.offset != self.next_offset {
            if frame.offset < self.next_offset {
                return Err(QdnfError::Overlap);
            }
            return Err(QdnfError::Incomplete);
        }
        self.next_offset = self
            .next_offset
            .checked_add(frame.len as u64)
            .ok_or(QdnfError::Range)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
