//! QBS1 untrusted service framing. Parsing never grants disclosure, credit or presence.
mod limits;
mod prefix;
mod record;
mod selector;
mod variants;
pub use limits::Limits;
pub use prefix::{Header, Kind};
pub use record::ServiceRecordHeader;
pub use selector::ObjectSelector;
pub const PREFIX_BYTES: usize = 208;
pub const MAX_RECORD_BYTES: usize = 65_776;
pub const MAX_LEAF_RECORD_BYTES: usize = 18_137;
pub const SERVICE_IRI: &[u8] = b"q42:QBDLStream/1";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Malformed,
    Unsupported,
    Limit,
    Capacity,
    Overflow,
}
/// Borrowed UNTRUSTED fields; callers must use canonical admission owners.
pub struct Message<'a> {
    pub header: Header,
    pub body: &'a [u8],
}
impl<'a> Message<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, Error> {
        if bytes.len() < PREFIX_BYTES {
            return Err(Error::Malformed);
        }
        if bytes.len() > MAX_RECORD_BYTES {
            return Err(Error::Limit);
        }
        let header = Header::parse_prefix(&bytes[..PREFIX_BYTES])?;
        let body = &bytes[PREFIX_BYTES..];
        if u32_at(bytes, 180) as usize != body.len() {
            return Err(Error::Malformed);
        }
        variants::validate(&header, body)?;
        Ok(Self { header, body })
    }
}
pub fn encode(header: &Header, body: &[u8], out: &mut [u8]) -> Result<usize, Error> {
    let n = PREFIX_BYTES
        .checked_add(body.len())
        .ok_or(Error::Overflow)?;
    if n > MAX_RECORD_BYTES {
        return Err(Error::Limit);
    }
    header.validate()?;
    variants::validate(header, body)?;
    if out.len() < n {
        return Err(Error::Capacity);
    }
    header.write_prefix(body.len() as u32, &mut out[..PREFIX_BYTES]);
    out[PREFIX_BYTES..n].copy_from_slice(body);
    Ok(n)
}
fn u16_at(b: &[u8], p: usize) -> u16 {
    u16::from_be_bytes([b[p], b[p + 1]])
}
fn u32_at(b: &[u8], p: usize) -> u32 {
    u32::from_be_bytes(b[p..p + 4].try_into().unwrap())
}
fn u64_at(b: &[u8], p: usize) -> u64 {
    u64::from_be_bytes(b[p..p + 8].try_into().unwrap())
}
fn zero(b: &[u8]) -> Result<(), Error> {
    if b.iter().any(|v| *v != 0) {
        Err(Error::Malformed)
    } else {
        Ok(())
    }
}
#[cfg(test)]
mod tests;
