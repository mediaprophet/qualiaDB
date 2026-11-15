//! Service-only AEAD additional data; distinct from the private-package codec.
use super::{u16_at, u32_at, Error as E, MAX_RECORD_BYTES, PREFIX_BYTES};
use crate::net::qdnf::types::StrongDigest;
pub const DOMAIN: &[u8; 12] = b"QBDL-SCT-V1\0";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ServiceRecordHeader {
    pub transcript: StrongDigest,
    pub message_id: [u8; 16],
    pub total: u32,
    pub offset: u32,
    pub chunk: u16,
    pub final_chunk: bool,
}
impl ServiceRecordHeader {
    pub const BYTES: usize = 88;
    pub fn validate(&self) -> Result<(), E> {
        if self.transcript == StrongDigest::ZERO || self.message_id == [0; 16] {
            return Err(E::Malformed);
        }
        if !(PREFIX_BYTES..=MAX_RECORD_BYTES).contains(&(self.total as usize)) {
            return Err(E::Limit);
        }
        if self.offset >= self.total || self.offset % 1024 != 0 {
            return Err(E::Malformed);
        }
        let n = (self.total - self.offset).min(1024);
        if self.chunk as u32 != n || self.final_chunk != (self.offset + n == self.total) {
            return Err(E::Malformed);
        }
        Ok(())
    }
    pub fn parse(b: &[u8]) -> Result<Self, E> {
        if b.len() != 88 {
            return Err(E::Malformed);
        }
        if &b[..12] != DOMAIN {
            return Err(E::Unsupported);
        }
        let final_flag = u16_at(b, 86);
        if final_flag > 1 {
            return Err(E::Unsupported);
        }
        let s = Self {
            transcript: StrongDigest(b[12..60].try_into().unwrap()),
            message_id: b[60..76].try_into().unwrap(),
            total: u32_at(b, 76),
            offset: u32_at(b, 80),
            chunk: u16_at(b, 84),
            final_chunk: final_flag == 1,
        };
        s.validate()?;
        Ok(s)
    }
    pub fn encode(&self, out: &mut [u8]) -> Result<usize, E> {
        self.validate()?;
        let b = out.get_mut(..88).ok_or(E::Capacity)?;
        b[..12].copy_from_slice(DOMAIN);
        b[12..60].copy_from_slice(&self.transcript.0);
        b[60..76].copy_from_slice(&self.message_id);
        b[76..80].copy_from_slice(&self.total.to_be_bytes());
        b[80..84].copy_from_slice(&self.offset.to_be_bytes());
        b[84..86].copy_from_slice(&self.chunk.to_be_bytes());
        b[86..88].copy_from_slice(&(self.final_chunk as u16).to_be_bytes());
        Ok(88)
    }
}
