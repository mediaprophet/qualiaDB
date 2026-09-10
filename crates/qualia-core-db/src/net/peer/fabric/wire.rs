//! CSCP wire codec for `draft-webcivics-cscp-00`.
//!
//! Not a QUIC implementation. Unknown critical TLVs fail closed.

use super::carrier::PathClass;
use super::connect::{connect, ConnectHandle, Fabric};
use super::contact::LocatorKind;
use super::intent::{ConnectionIntent, ProtectionPolicy, Purpose, PurposeClass};
use super::kernel::FabricError;
use super::receipt::{OpReceipt, ReceiptStatus};
use crate::net::peer::connectivity::policy::Disclosure;
use crate::net::peer::runtime::ResourceBudget;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CscpError {
    Truncated,
    BadMagic,
    UnknownCritical,
    Duplicate,
    Capacity,
    Policy,
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

pub fn disclosure_byte(d: Disclosure) -> u8 {
    match d {
        Disclosure::DirectPermitted => 1,
        Disclosure::ApprovedRelaysOnly => 2,
        Disclosure::QualifiedMultiHop => 3,
        Disclosure::Isolated => 4,
    }
}

pub fn disclosure_from(b: u8) -> Result<Disclosure, CscpError> {
    match b {
        1 => Ok(Disclosure::DirectPermitted),
        2 => Ok(Disclosure::ApprovedRelaysOnly),
        3 => Ok(Disclosure::QualifiedMultiHop),
        4 => Ok(Disclosure::Isolated),
        _ => Err(CscpError::Policy),
    }
}

fn header(out: &mut [u8], msg: u8, body_len: u16) -> Result<usize, CscpError> {
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

fn write_tlv(out: &mut [u8], tag: u8, val: &[u8]) -> Result<usize, CscpError> {
    let n = 3 + val.len();
    if out.len() < n {
        return Err(CscpError::Capacity);
    }
    out[0] = tag;
    out[1..3].copy_from_slice(&(val.len() as u16).to_be_bytes());
    out[3..n].copy_from_slice(val);
    Ok(n)
}

fn known_connect(tag: u8) -> bool {
    matches!(
        tag,
        TAG_PEER | TAG_PURPOSE | TAG_PROTECTION | TAG_BUDGET | TAG_DEADLINE | TAG_AUTHORITY
    )
}

/// Encode `draft-webcivics-cscp-00` ConnectRequest.
pub fn encode_connect_request(
    intent: &ConnectionIntent,
    out: &mut [u8],
) -> Result<usize, CscpError> {
    let mut body = [0u8; 256];
    let mut n = 0usize;
    n += write_tlv(&mut body[n..], TAG_PEER | TAG_CRITICAL, &intent.peer)?;
    let mut purpose = [0u8; 9];
    purpose[..8].copy_from_slice(&intent.purpose.iri_hash.to_be_bytes());
    purpose[8] = intent.purpose.class as u8;
    n += write_tlv(&mut body[n..], TAG_PURPOSE | TAG_CRITICAL, &purpose)?;
    let prot = [
        disclosure_byte(intent.protection.disclosure),
        u8::from(intent.protection.require_e2e_session),
        u8::from(intent.protection.public_dht),
    ];
    n += write_tlv(&mut body[n..], TAG_PROTECTION | TAG_CRITICAL, &prot)?;
    let mut budget = [0u8; 24];
    budget[..8].copy_from_slice(&intent.budget.bytes.to_be_bytes());
    budget[8..16].copy_from_slice(&intent.budget.work.to_be_bytes());
    budget[16..24].copy_from_slice(&intent.budget.io.to_be_bytes());
    n += write_tlv(&mut body[n..], TAG_BUDGET | TAG_CRITICAL, &budget)?;
    let mut dl = [0u8; 16];
    dl[..8].copy_from_slice(&intent.created_ms.to_be_bytes());
    dl[8..16].copy_from_slice(&intent.deadline_ms.to_be_bytes());
    n += write_tlv(&mut body[n..], TAG_DEADLINE | TAG_CRITICAL, &dl)?;
    n += write_tlv(
        &mut body[n..],
        TAG_AUTHORITY,
        &intent.authority.to_be_bytes(),
    )?;
    if 10 + n > out.len() {
        return Err(CscpError::Capacity);
    }
    header(out, MSG_CONNECT_REQUEST, n as u16)?;
    out[10..10 + n].copy_from_slice(&body[..n]);
    Ok(10 + n)
}

pub fn decode_connect_request(bytes: &[u8], now_ms: u64) -> Result<ConnectionIntent, CscpError> {
    if bytes.len() < 10 {
        return Err(CscpError::Truncated);
    }
    if &bytes[..4] != MAGIC || bytes[4] != VERSION {
        return Err(CscpError::BadMagic);
    }
    if bytes[5] != MSG_CONNECT_REQUEST {
        return Err(CscpError::Policy);
    }
    let blen = u16::from_be_bytes([bytes[8], bytes[9]]) as usize;
    if blen > MAX_BODY || 10 + blen > bytes.len() {
        return Err(CscpError::Capacity);
    }
    let body = &bytes[10..10 + blen];
    let mut seen = [false; 16];
    let mut peer = [0u8; 32];
    let mut purpose_hash = 0u64;
    let mut purpose_class = PurposeClass::Ordinary;
    let mut disclosure = Disclosure::Isolated;
    let mut require_e2e = true;
    let mut public_dht = false;
    let mut budget = ResourceBudget::ZERO;
    let mut created = 0u64;
    let mut deadline = 0u64;
    let mut authority = 0u64;
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
        if tag & TAG_CRITICAL != 0 && !known_connect(tag & 0x7f) {
            return Err(CscpError::UnknownCritical);
        }
        match tag & 0x7f {
            TAG_PEER if val.len() == 32 => peer.copy_from_slice(val),
            TAG_PURPOSE if val.len() == 9 => {
                let mut h = [0u8; 8];
                h.copy_from_slice(&val[..8]);
                purpose_hash = u64::from_be_bytes(h);
                purpose_class = match val[8] {
                    1 => PurposeClass::Ordinary,
                    2 => PurposeClass::Clinical,
                    3 => PurposeClass::Infrastructure,
                    _ => return Err(CscpError::Policy),
                };
            }
            TAG_PROTECTION if val.len() == 3 => {
                disclosure = disclosure_from(val[0])?;
                require_e2e = val[1] != 0;
                public_dht = val[2] != 0;
            }
            TAG_BUDGET if val.len() == 24 => {
                let mut b = [0u8; 8];
                b.copy_from_slice(&val[..8]);
                budget.bytes = u64::from_be_bytes(b);
                b.copy_from_slice(&val[8..16]);
                budget.work = u64::from_be_bytes(b);
                b.copy_from_slice(&val[16..24]);
                budget.io = u64::from_be_bytes(b);
            }
            TAG_DEADLINE if val.len() == 16 => {
                let mut t = [0u8; 8];
                t.copy_from_slice(&val[..8]);
                created = u64::from_be_bytes(t);
                t.copy_from_slice(&val[8..16]);
                deadline = u64::from_be_bytes(t);
            }
            TAG_AUTHORITY if val.len() == 8 => {
                let mut a = [0u8; 8];
                a.copy_from_slice(val);
                authority = u64::from_be_bytes(a);
            }
            _ => {}
        }
        fields += 1;
    }
    if now_ms >= deadline {
        return Err(CscpError::Policy);
    }
    let mut intent = ConnectionIntent::new(
        peer,
        Purpose {
            iri_hash: purpose_hash,
            class: purpose_class,
        },
        ProtectionPolicy {
            disclosure,
            require_e2e_session: require_e2e,
            public_dht,
        },
        budget,
        created,
        deadline,
    );
    intent.authority = authority;
    Ok(intent)
}

/// Admit a ConnectRequest from the wire. Isolated profiles still queue.
pub fn connect_from_wire(
    fabric: &mut Fabric,
    bytes: &[u8],
    now_ms: u64,
) -> Result<ConnectHandle, FabricError> {
    let intent = decode_connect_request(bytes, now_ms)?;
    connect(
        fabric,
        intent.peer,
        intent.purpose,
        intent.protection,
        intent.budget,
        now_ms,
        intent.deadline_ms,
    )
}

pub fn encode_reject(out: &mut [u8], reason: u8) -> Result<usize, CscpError> {
    if out.len() < 14 {
        return Err(CscpError::Capacity);
    }
    header(out, MSG_CONNECT_REJECT, 4)?;
    out[10] = 1 | TAG_CRITICAL;
    out[11..13].copy_from_slice(&1u16.to_be_bytes());
    out[13] = reason;
    Ok(14)
}

pub fn encode_receipt(r: &OpReceipt, out: &mut [u8]) -> Result<usize, CscpError> {
    let mut body = [0u8; 128];
    let mut n = 0usize;
    n += write_tlv(&mut body[n..], 1 | TAG_CRITICAL, &r.op_id.to_be_bytes())?;
    n += write_tlv(&mut body[n..], 2 | TAG_CRITICAL, &r.content_digest)?;
    n += write_tlv(&mut body[n..], 3 | TAG_CRITICAL, &r.peer)?;
    n += write_tlv(
        &mut body[n..],
        4 | TAG_CRITICAL,
        &r.grant_generation.to_be_bytes(),
    )?;
    n += write_tlv(&mut body[n..], 5 | TAG_CRITICAL, &[r.status as u8])?;
    n += write_tlv(
        &mut body[n..],
        6 | TAG_CRITICAL,
        &r.expiry_unix.to_be_bytes(),
    )?;
    if 10 + n > out.len() {
        return Err(CscpError::Capacity);
    }
    header(out, MSG_RECEIPT, n as u16)?;
    out[10..10 + n].copy_from_slice(&body[..n]);
    Ok(10 + n)
}

#[allow(dead_code)]
pub fn path_class_byte(c: PathClass) -> u8 {
    match c {
        PathClass::DirectV6 => 1,
        PathClass::DirectV4 => 2,
        PathClass::Relayed => 3,
        PathClass::BrowserGateway => 4,
        PathClass::Offline => 5,
    }
}

pub fn locator_kind_byte(k: LocatorKind) -> u8 {
    match k {
        LocatorKind::Mailbox => 1,
        LocatorKind::RelayHint => 2,
        LocatorKind::Direct => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::peer::fabric::kernel::FabricState;
    use crate::net::peer::fabric::intent::ProtectionPolicy;

    fn sample() -> ConnectionIntent {
        ConnectionIntent::new(
            [0x42u8; 32],
            Purpose::clinical(),
            ProtectionPolicy::RELAY_ONLY,
            ResourceBudget {
                bytes: 4096,
                work: 2,
                io: 2,
            },
            10,
            9_000,
        )
    }

    #[test]
    fn connect_request_round_trip() {
        let intent = sample();
        let mut buf = [0u8; 512];
        let n = encode_connect_request(&intent, &mut buf).unwrap();
        let back = decode_connect_request(&buf[..n], 20).unwrap();
        assert_eq!(back.peer, intent.peer);
        assert_eq!(back.purpose, intent.purpose);
        assert_eq!(back.protection.disclosure, Disclosure::ApprovedRelaysOnly);
        assert!(!back.protection.public_dht);
        assert_eq!(back.budget.bytes, 4096);
    }

    #[test]
    fn unknown_critical_tlv_is_rejected() {
        let intent = sample();
        let mut buf = [0u8; 512];
        let n = encode_connect_request(&intent, &mut buf).unwrap();
        // Append a critical unknown tag and bump body length.
        let extra = write_tlv(&mut buf[n..], 0x7f | TAG_CRITICAL, &[9]).unwrap();
        let blen = u16::from_be_bytes([buf[8], buf[9]]) + extra as u16;
        buf[8..10].copy_from_slice(&blen.to_be_bytes());
        assert_eq!(
            decode_connect_request(&buf[..n + extra], 20),
            Err(CscpError::UnknownCritical)
        );
    }

    #[test]
    fn wire_connect_isolated_queues() {
        let mut intent = sample();
        intent.protection = ProtectionPolicy::ISOLATED;
        let mut buf = [0u8; 512];
        let n = encode_connect_request(&intent, &mut buf).unwrap();
        let mut f = Fabric::new();
        let h = connect_from_wire(&mut f, &buf[..n], 20).unwrap();
        assert_eq!(h.state, FabricState::OfflineQueued);
        assert!(f.session().is_none());
    }

    #[test]
    fn receipt_committed_is_not_implied_by_encode() {
        let r = OpReceipt::queued(1, [2u8; 32], [3u8; 32], 1, 100);
        assert_eq!(r.status, ReceiptStatus::Queued);
        let mut buf = [0u8; 256];
        let n = encode_receipt(&r, &mut buf).unwrap();
        assert_eq!(&buf[..4], MAGIC);
        assert_eq!(buf[5], MSG_RECEIPT);
        assert!(n > 10);
        let _ = locator_kind_byte(LocatorKind::Mailbox);
    }
}
