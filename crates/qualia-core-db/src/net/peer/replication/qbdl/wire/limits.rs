use super::{u16_at, u32_at, u64_at, zero, Error as E, MAX_RECORD_BYTES};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub record: u32,
    pub page: u32,
    pub cose: u32,
    pub index: u32,
    pub metadata: u32,
    pub closure: u32,
    pub entries: u32,
    pub inflight: u16,
    pub window: u64,
    pub cold_rows: u32,
    pub extent: u64,
    pub lifetime_ms: u32,
}
impl Limits {
    pub const BYTES: usize = 64;
    pub fn validate(&self) -> Result<(), E> {
        for (n, max) in [
            (self.record, MAX_RECORD_BYTES as u32),
            (self.page, 65536),
            (
                self.cose,
                crate::crypto::network::cose::MAX_ENVELOPE_BYTES as u32,
            ),
            (self.index, 8388608),
            (self.metadata, 8388608),
            (self.closure, 1048576),
            (self.entries, 4096),
            (self.cold_rows, 64),
            (self.lifetime_ms, 30000),
        ] {
            if n == 0 || n > max {
                return Err(E::Limit);
            }
        }
        if self.record < 304
            || self.page.checked_add(240).ok_or(E::Overflow)? > self.record
            || !(1..=30).contains(&self.inflight)
            || self.window == 0
            || self.window > 32 * 1024 * 1024
            || self.extent == 0
            || self.extent > 1 << 40
        {
            return Err(E::Limit);
        }
        Ok(())
    }
    pub fn parse(b: &[u8]) -> Result<Self, E> {
        if b.len() != 64 {
            return Err(E::Malformed);
        }
        if u16_at(b, 30) != 2 || u16_at(b, 44) != 1 {
            return Err(E::Unsupported);
        }
        zero(&b[46..48])?;
        zero(&b[60..64])?;
        let s = Self {
            record: u32_at(b, 0),
            page: u32_at(b, 4),
            cose: u32_at(b, 8),
            index: u32_at(b, 12),
            metadata: u32_at(b, 16),
            closure: u32_at(b, 20),
            entries: u32_at(b, 24),
            inflight: u16_at(b, 28),
            window: u64_at(b, 32),
            cold_rows: u32_at(b, 40),
            extent: u64_at(b, 48),
            lifetime_ms: u32_at(b, 56),
        };
        s.validate()?;
        Ok(s)
    }
    pub fn encode(&self, out: &mut [u8]) -> Result<usize, E> {
        self.validate()?;
        let b = out.get_mut(..64).ok_or(E::Capacity)?;
        b.fill(0);
        for (p, v) in [
            (0, self.record),
            (4, self.page),
            (8, self.cose),
            (12, self.index),
            (16, self.metadata),
            (20, self.closure),
            (24, self.entries),
            (40, self.cold_rows),
            (56, self.lifetime_ms),
        ] {
            b[p..p + 4].copy_from_slice(&v.to_be_bytes());
        }
        b[28..30].copy_from_slice(&self.inflight.to_be_bytes());
        b[30..32].copy_from_slice(&2u16.to_be_bytes());
        b[32..40].copy_from_slice(&self.window.to_be_bytes());
        b[44..46].copy_from_slice(&1u16.to_be_bytes());
        b[48..56].copy_from_slice(&self.extent.to_be_bytes());
        Ok(64)
    }
}
