//! Private control framing only. These views are NOT confidential carrier receipts.
use super::codec::{Reader, Writer};
use super::{
    Error, MAX_DELIVERY, MAX_PRIVATE_PACKAGE, RECIPIENT_BYTES, RECORD_HEADER_BYTES,
    RecipientRecord, WRAP_BYTES, Wrap,
};
use crate::net::qdnf::types::StrongDigest;
#[derive(Clone, Copy)]
pub struct Delivery<'a> {
    bytes: &'a [u8],
    pub descriptor: StrongDigest,
    pub request: [u8; 16],
    pub recipient: RecipientRecord<'a>,
    pub proof: &'a [u8],
    pub wrap: Wrap<'a>,
}
impl<'a> Delivery<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, Error> {
        if bytes.len() > MAX_DELIVERY {
            return Err(Error::Capacity);
        }
        let mut r = Reader::new(bytes, b"QBDL-PDL-V1\0")?;
        let descriptor = r.digest()?;
        let request = r.array()?;
        super::types::nonzero(&descriptor.0)?;
        super::types::nonzero(&request)?;
        if r.u16()? as usize != RECIPIENT_BYTES {
            return Err(Error::Malformed);
        }
        let recipient = RecipientRecord::parse(r.take(RECIPIENT_BYTES)?)?;
        let len = r.u16()? as usize;
        if len == 0 || len > crate::crypto::network::cose::MAX_ENVELOPE_BYTES {
            return Err(Error::Capacity);
        }
        let proof = r.take(len)?;
        if r.u16()? as usize != WRAP_BYTES {
            return Err(Error::Malformed);
        }
        let wrap = Wrap::parse(r.take(WRAP_BYTES)?)?;
        r.end()?;
        if descriptor != recipient.fields().descriptor
            || wrap.auth != recipient.id()
            || wrap.key_ids != recipient.key_ids()
        {
            return Err(Error::Unauthorized);
        }
        Ok(Self {
            bytes,
            descriptor,
            request,
            recipient,
            proof,
            wrap,
        })
    }
    pub fn encoded(&self) -> &'a [u8] {
        self.bytes
    }
}
pub fn encode_delivery(
    request: [u8; 16],
    recipient: RecipientRecord<'_>,
    proof: &[u8],
    wrap: Wrap<'_>,
    out: &mut [u8],
) -> Result<usize, Error> {
    super::types::nonzero(&request)?;
    if proof.is_empty() || proof.len() > crate::crypto::network::cose::MAX_ENVELOPE_BYTES {
        return Err(Error::Capacity);
    }
    if wrap.auth != recipient.id() || wrap.key_ids != recipient.key_ids() {
        return Err(Error::Unauthorized);
    }
    let mut w = Writer::new(
        out,
        82 + RECIPIENT_BYTES + WRAP_BYTES + proof.len(),
        b"QBDL-PDL-V1\0",
    )?;
    w.put(&recipient.fields().descriptor.0)?;
    w.put(&request)?;
    w.u16(RECIPIENT_BYTES as u16)?;
    w.put(recipient.encoded())?;
    w.u16(proof.len() as u16)?;
    w.put(proof)?;
    w.u16(WRAP_BYTES as u16)?;
    w.put(wrap.encoded())?;
    w.finish()
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecordHeader {
    pub transcript: StrongDigest,
    pub request: [u8; 16],
    pub total: u32,
    pub offset: u32,
    pub chunk: u16,
    pub final_chunk: bool,
}
impl RecordHeader {
    pub fn validate(&self) -> Result<(), Error> {
        super::types::nonzero(&self.transcript.0)?;
        super::types::nonzero(&self.request)?;
        let total = self.total as usize;
        let off = self.offset as usize;
        let len = self.chunk as usize;
        if total == 0
            || total > MAX_PRIVATE_PACKAGE
            || off >= total
            || off % 1024 != 0
            || len != (total - off).min(1024)
            || self.final_chunk != (off + len == total)
        {
            return Err(Error::Malformed);
        }
        Ok(())
    }
    pub fn parse(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() != RECORD_HEADER_BYTES {
            return Err(Error::Range);
        }
        let mut r = Reader::new(bytes, b"QBDL-PCT-V1\0")?;
        let transcript = r.digest()?;
        let request = r.array()?;
        let total = r.u32()?;
        let offset = r.u32()?;
        let chunk = r.u16()?;
        let flags = r.u16()?;
        if flags > 1 {
            return Err(Error::Malformed);
        }
        r.end()?;
        let h = Self {
            transcript,
            request,
            total,
            offset,
            chunk,
            final_chunk: flags == 1,
        };
        h.validate()?;
        Ok(h)
    }
    pub fn encode(&self, out: &mut [u8]) -> Result<usize, Error> {
        self.validate()?;
        let mut w = Writer::new(out, RECORD_HEADER_BYTES, b"QBDL-PCT-V1\0")?;
        w.put(&self.transcript.0)?;
        w.put(&self.request)?;
        w.u32(self.total)?;
        w.u32(self.offset)?;
        w.u16(self.chunk)?;
        w.u16(u16::from(self.final_chunk))?;
        w.finish()
    }
}
