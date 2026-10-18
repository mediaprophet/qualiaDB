//! CSCP v1 TLV helpers. `draft-webcivics-cscp-00` §4.

use super::carrier::PathClass;
use super::contact::LocatorKind;
use super::kernel::FabricError;
use crate::net::peer::connectivity::policy::Disclosure;

pub const MAGIC: &[u8; 4] = b"CSCP";
pub const VERSION: u8 = 1;
pub const MAX_BODY: usize = 1024;
pub const MAX_FIELDS: usize = 16;
pub const TAG_CRITICAL: u8 = 0x80;

pub const MSG_CONNECT_REQUEST: u8 = 1;
pub const MSG_CONTACT: u8 = 2;
pub const MSG_RELAY_LEASE: u8 = 3;
pub const MSG_CUSTODY_LEASE: u8 = 4;
pub const MSG_PATH_EVIDENCE: u8 = 5;
pub const MSG_RECEIPT: u8 = 6;
pub const MSG_CONNECT_ACCEPT: u8 = 7;
pub const MSG_CONNECT_REJECT: u8 = 8;

pub const TAG_PEER: u8 = 1;
pub const TAG_PURPOSE: u8 = 2;
pub const TAG_PROTECTION: u8 = 3;
pub const TAG_BUDGET: u8 = 4;
pub const TAG_DEADLINE: u8 = 5;
pub const TAG_AUTHORITY: u8 = 6;

mod connect_msg;
mod records;

pub use connect_msg::{connect_from_wire, decode_connect_request, encode_connect_request};
pub use records::{
    decode_accept, decode_contact, decode_custody, decode_evidence, decode_receipt, decode_reject,
    decode_relay_lease, encode_accept, encode_contact, encode_custody, encode_evidence,
    encode_receipt, encode_reject, encode_relay_lease, RejectReason,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CscpError {
    Truncated,
    BadMagic,
    UnknownCritical,
    Duplicate,
    Capacity,
    Policy,
    Malformed,
}

impl From<CscpError> for FabricError {
    fn from(e: CscpError) -> Self {
        match e {
            CscpError::Policy => FabricError::PolicyDenied,
            CscpError::Capacity => FabricError::Capacity,
            _ => FabricError::Illegal,
        }
    }
}

/// Maps a `Disclosure` level to its wire byte.
pub fn disclosure_byte(d: Disclosure) -> u8 {
    match d {
        Disclosure::DirectPermitted => 1,
        Disclosure::ApprovedRelaysOnly => 2,
        Disclosure::QualifiedMultiHop => 3,
        Disclosure::Isolated => 4,
    }
}

/// Decodes a `Disclosure` from its wire byte; `CscpError` on an unknown value — fails closed.
pub fn disclosure_from(b: u8) -> Result<Disclosure, CscpError> {
    match b {
        1 => Ok(Disclosure::DirectPermitted),
        2 => Ok(Disclosure::ApprovedRelaysOnly),
        3 => Ok(Disclosure::QualifiedMultiHop),
        4 => Ok(Disclosure::Isolated),
        _ => Err(CscpError::Policy),
    }
}

/// Maps a `PathClass` to its wire byte.
pub fn path_class_byte(c: PathClass) -> u8 {
    match c {
        PathClass::DirectV6 => 1,
        PathClass::DirectV4 => 2,
        PathClass::Relayed => 3,
        PathClass::BrowserGateway => 4,
        PathClass::Offline => 5,
    }
}

/// Decodes a `PathClass` from its wire byte; fails closed on unknown.
pub fn path_class_from(b: u8) -> Result<PathClass, CscpError> {
    match b {
        1 => Ok(PathClass::DirectV6),
        2 => Ok(PathClass::DirectV4),
        3 => Ok(PathClass::Relayed),
        4 => Ok(PathClass::BrowserGateway),
        5 => Ok(PathClass::Offline),
        _ => Err(CscpError::Malformed),
    }
}

/// Maps a `LocatorKind` to its wire byte.
pub fn locator_kind_byte(k: LocatorKind) -> u8 {
    match k {
        LocatorKind::Mailbox => 1,
        LocatorKind::RelayHint => 2,
        LocatorKind::Direct => 3,
    }
}

/// Decodes a `LocatorKind` from its wire byte; fails closed on unknown.
pub fn locator_kind_from(b: u8) -> Result<LocatorKind, CscpError> {
    match b {
        1 => Ok(LocatorKind::Mailbox),
        2 => Ok(LocatorKind::RelayHint),
        3 => Ok(LocatorKind::Direct),
        _ => Err(CscpError::Malformed),
    }
}

/// Writes the fixed CSCP message header (type byte + big-endian body length) into `out`.
pub(crate) fn header(out: &mut [u8], msg: u8, body_len: u16) -> Result<usize, CscpError> {
    if out.len() < 10 {
        return Err(CscpError::Capacity);
    }
    out[..4].copy_from_slice(MAGIC);
    out[4] = VERSION;
    out[5] = msg;
    out[6] = 0;
    out[7] = 0;
    out[8..10].copy_from_slice(&body_len.to_be_bytes());
    Ok(10)
}

/// Appends one tag/length/value triple into `out`; errors when the buffer is too small.
pub(crate) fn write_tlv(out: &mut [u8], tag: u8, val: &[u8]) -> Result<usize, CscpError> {
    let n = 3 + val.len();
    if out.len() < n {
        return Err(CscpError::Capacity);
    }
    out[0] = tag;
    out[1..3].copy_from_slice(&(val.len() as u16).to_be_bytes());
    out[3..n].copy_from_slice(val);
    Ok(n)
}

/// Validates the header and returns the body slice; `Malformed`/`Unsupported` on wrong type or truncation.
pub(crate) fn parse_header(bytes: &[u8], expect: u8) -> Result<&[u8], CscpError> {
    if bytes.len() < 10 {
        return Err(CscpError::Truncated);
    }
    if &bytes[..4] != MAGIC || bytes[4] != VERSION {
        return Err(CscpError::BadMagic);
    }
    if bytes[5] != expect {
        return Err(CscpError::Policy);
    }
    if bytes[6] != 0 || bytes[7] != 0 {
        return Err(CscpError::Malformed);
    }
    let blen = u16::from_be_bytes([bytes[8], bytes[9]]) as usize;
    if blen > MAX_BODY || 10 + blen > bytes.len() {
        return Err(CscpError::Capacity);
    }
    Ok(&bytes[10..10 + blen])
}

/// Iterates body TLVs, invoking `f(tag, val)` per entry; fails closed on truncated length fields.
pub(crate) fn walk_tlvs(
    body: &[u8],
    known: fn(u8) -> bool,
    mut visit: impl FnMut(u8, &[u8]) -> Result<(), CscpError>,
) -> Result<(), CscpError> {
    let mut seen = [false; 16];
    let mut i = 0usize;
    let mut fields = 0usize;
    while i < body.len() {
        if i + 3 > body.len() || fields >= MAX_FIELDS {
            return Err(CscpError::Truncated);
        }
        let tag = body[i];
        let len = u16::from_be_bytes([body[i + 1], body[i + 2]]) as usize;
        i += 3;
        if i + len > body.len() {
            return Err(CscpError::Truncated);
        }
        let val = &body[i..i + len];
        i += len;
        let idx = (tag & 0x7f) as usize;
        if idx < seen.len() {
            if seen[idx] {
                return Err(CscpError::Duplicate);
            }
            seen[idx] = true;
        }
        if tag & TAG_CRITICAL != 0 && !known(tag & 0x7f) {
            return Err(CscpError::UnknownCritical);
        }
        visit(tag & 0x7f, val)?;
        fields += 1;
    }
    Ok(())
}

/// As `walk_tlvs` plus a seen-tag bitmask for required-field validation.
pub(crate) fn walk_tlvs_seen(
    body: &[u8],
    known: fn(u8) -> bool,
    mut visit: impl FnMut(u8, &[u8]) -> Result<(), CscpError>,
) -> Result<u16, CscpError> {
    let mut seen_mask = 0u16;
    walk_tlvs(body, known, |tag, val| {
        if tag < 16 {
            seen_mask |= 1u16 << tag;
        }
        visit(tag, val)
    })?;
    Ok(seen_mask)
}

/// `Ok` when every tag in `tags` appears in the `seen` bitmask, else `Incomplete`.
pub(crate) fn require_tags(seen: u16, tags: &[u8]) -> Result<(), CscpError> {
    for &t in tags {
        if t >= 16 || seen & (1u16 << t) == 0 {
            return Err(CscpError::Malformed);
        }
    }
    Ok(())
}

/// Writes header + body as a complete message; returns total length.
pub(crate) fn finish(out: &mut [u8], msg: u8, body: &[u8]) -> Result<usize, CscpError> {
    if body.len() > MAX_BODY || 10 + body.len() > out.len() {
        return Err(CscpError::Capacity);
    }
    header(out, msg, body.len() as u16)?;
    out[10..10 + body.len()].copy_from_slice(body);
    Ok(10 + body.len())
}

/// Decodes a big-endian u64; `Malformed` on short input.
pub(crate) fn u64_be(v: &[u8]) -> Result<u64, CscpError> {
    if v.len() != 8 {
        return Err(CscpError::Malformed);
    }
    let mut b = [0u8; 8];
    b.copy_from_slice(v);
    Ok(u64::from_be_bytes(b))
}

/// Decodes a big-endian u32; `Malformed` on short input.
pub(crate) fn u32_be(v: &[u8]) -> Result<u32, CscpError> {
    if v.len() != 4 {
        return Err(CscpError::Malformed);
    }
    let mut b = [0u8; 4];
    b.copy_from_slice(v);
    Ok(u32::from_be_bytes(b))
}

/// Decodes a big-endian u16; `Malformed` on short input.
pub(crate) fn u16_be(v: &[u8]) -> Result<u16, CscpError> {
    if v.len() != 2 {
        return Err(CscpError::Malformed);
    }
    Ok(u16::from_be_bytes([v[0], v[1]]))
}
