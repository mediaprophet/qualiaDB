use super::{u32_at, u64_at, zero, Error as E};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObjectSelector {
    pub class: u8,
    pub number: u32,
    pub total: u64,
    pub offset: u64,
    pub chunk: u32,
}
impl ObjectSelector {
    pub const BYTES: usize = 32;
    pub fn parse(b: &[u8], request: bool) -> Result<Self, E> {
        if b.len() != 32 {
            return Err(E::Malformed);
        }
        zero(&b[1..4])?;
        zero(&b[28..32])?;
        let s = Self {
            class: b[0],
            number: u32_at(b, 4),
            total: u64_at(b, 8),
            offset: u64_at(b, 16),
            chunk: u32_at(b, 24),
        };
        s.validate(request)?;
        Ok(s)
    }
    pub fn validate(&self, request: bool) -> Result<(), E> {
        let max = match self.class {
            1 | 8 | 9 => 65536,
            2 | 5 => crate::crypto::network::cose::MAX_ENVELOPE_BYTES as u64,
            3 => 2048,
            4 => 1048576,
            6 => 64,
            7 => 8388608,
            _ => return Err(E::Unsupported),
        };
        if self.chunk == 0 || self.chunk > 65536 || self.total > max {
            return Err(E::Limit);
        }
        if self.total == 0 {
            if request && self.class == 1 && self.number == 0 && self.offset == 0 {
                return Ok(());
            } else {
                return Err(E::Malformed);
            }
        }
        match self.class {
            4 => {
                if self.number >= 16 {
                    return Err(E::Limit);
                }
            }
            8 | 9 => {
                if self.number >= 32 {
                    return Err(E::Limit);
                }
            }
            _ => {
                if self.number != 0 {
                    return Err(E::Malformed);
                }
            }
        }
        let end = self
            .offset
            .checked_add(u64::from(self.chunk))
            .ok_or(E::Overflow)?;
        if self.offset >= self.total || (!request && end > self.total) {
            return Err(E::Malformed);
        }
        if !matches!(self.class, 4 | 7)
            && (self.offset != 0
                || (!request && u64::from(self.chunk) != self.total)
                || (request && u64::from(self.chunk) < self.total))
        {
            return Err(E::Malformed);
        }
        if self.class == 6 && self.total != 64 {
            return Err(E::Malformed);
        }
        Ok(())
    }
    pub fn encode(&self, request: bool, out: &mut [u8]) -> Result<usize, E> {
        self.validate(request)?;
        let b = out.get_mut(..32).ok_or(E::Capacity)?;
        b.fill(0);
        b[0] = self.class;
        b[4..8].copy_from_slice(&self.number.to_be_bytes());
        b[8..16].copy_from_slice(&self.total.to_be_bytes());
        b[16..24].copy_from_slice(&self.offset.to_be_bytes());
        b[24..28].copy_from_slice(&self.chunk.to_be_bytes());
        Ok(32)
    }
}
