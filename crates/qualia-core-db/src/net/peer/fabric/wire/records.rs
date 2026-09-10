//! Remaining CSCP message codecs. Decoded PathEvidence is never local validation.

use super::{
    finish, locator_kind_byte, locator_kind_from, parse_header, path_class_byte, path_class_from,
    u16_be, u32_be, u64_be, walk_tlvs, write_tlv, CscpError, MSG_CONNECT_ACCEPT, MSG_CONNECT_REJECT,
    MSG_CONTACT, MSG_CUSTODY_LEASE, MSG_PATH_EVIDENCE, MSG_RECEIPT, MSG_RELAY_LEASE, TAG_CRITICAL,
};
use crate::net::peer::fabric::carrier::PathClass;
use crate::net::peer::fabric::contact::ContactDescriptor;
use crate::net::peer::fabric::evidence::{ObserverKind, PathEvidence};
use crate::net::peer::fabric::lease::{CustodyLease, RelayLease};
use crate::net::peer::fabric::receipt::{OpReceipt, ReceiptStatus};

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectReason {
    Policy = 1,
    Stale = 2,
    Expired = 3,
    Revoked = 4,
    Capacity = 5,
}

fn known_contact(t: u8) -> bool {
    matches!(t, 1 | 2 | 3 | 4 | 5 | 6)
}
fn known_lease(t: u8) -> bool {
    matches!(t, 1 | 2 | 3 | 4 | 5 | 6)
}
fn known_evidence(t: u8) -> bool {
    matches!(t, 1 | 2 | 3 | 4 | 5)
}
fn known_receipt(t: u8) -> bool {
    matches!(t, 1 | 2 | 3 | 4 | 5 | 6)
}
fn known_accept(t: u8) -> bool {
    matches!(t, 1 | 2)
}
fn known_reject(t: u8) -> bool {
    matches!(t, 1)
}

pub fn encode_contact(d: &ContactDescriptor, out: &mut [u8]) -> Result<usize, CscpError> {
    let mut body = [0u8; 128];
    let mut n = 0usize;
    n += write_tlv(&mut body[n..], 1 | TAG_CRITICAL, &d.contact_key)?;
    n += write_tlv(&mut body[n..], 2 | TAG_CRITICAL, &d.generation.to_be_bytes())?;
    n += write_tlv(&mut body[n..], 3 | TAG_CRITICAL, &d.expiry_unix.to_be_bytes())?;
    n += write_tlv(
        &mut body[n..],
        4 | TAG_CRITICAL,
        &[locator_kind_byte(d.kind)],
    )?;
    n += write_tlv(&mut body[n..], 5, &d.locator)?;
    n += write_tlv(&mut body[n..], 6, &d.network_generation.to_be_bytes())?;
    finish(out, MSG_CONTACT, &body[..n])
}

pub fn decode_contact(bytes: &[u8]) -> Result<ContactDescriptor, CscpError> {
    let body = parse_header(bytes, MSG_CONTACT)?;
    let mut d = ContactDescriptor::mailbox([0u8; 32], 0, 0);
    walk_tlvs(body, known_contact, |tag, val| {
        match tag {
            1 if val.len() == 32 => d.contact_key.copy_from_slice(val),
            2 => d.generation = u32_be(val)?,
            3 => d.expiry_unix = u32_be(val)?,
            4 if val.len() == 1 => d.kind = locator_kind_from(val[0])?,
            5 if val.len() == 16 => d.locator.copy_from_slice(val),
            6 => d.network_generation = u32_be(val)?,
            _ => {}
        }
        Ok(())
    })?;
    Ok(d)
}

pub fn encode_relay_lease(l: &RelayLease, out: &mut [u8]) -> Result<usize, CscpError> {
    let mut body = [0u8; 192];
    let mut n = 0usize;
    n += write_tlv(&mut body[n..], 1 | TAG_CRITICAL, &l.lease_id.to_be_bytes())?;
    n += write_tlv(&mut body[n..], 2 | TAG_CRITICAL, &l.operator.to_be_bytes())?;
    let mut parts = [0u8; 64];
    parts[..32].copy_from_slice(&l.participants[0]);
    parts[32..].copy_from_slice(&l.participants[1]);
    n += write_tlv(&mut body[n..], 3 | TAG_CRITICAL, &parts)?;
    let mut bytes = [0u8; 16];
    bytes[..8].copy_from_slice(&l.max_bytes.to_be_bytes());
    bytes[8..].copy_from_slice(&l.remaining_bytes.to_be_bytes());
    n += write_tlv(&mut body[n..], 4 | TAG_CRITICAL, &bytes)?;
    let mut exp = [0u8; 12];
    exp[..8].copy_from_slice(&l.expiry_ms.to_be_bytes());
    exp[8..].copy_from_slice(&l.generation.to_be_bytes());
    n += write_tlv(&mut body[n..], 5 | TAG_CRITICAL, &exp)?;
    n += write_tlv(
        &mut body[n..],
        6 | TAG_CRITICAL,
        &[u8::from(l.export_observations)],
    )?;
    finish(out, MSG_RELAY_LEASE, &body[..n])
}

pub fn decode_relay_lease(bytes: &[u8]) -> Result<RelayLease, CscpError> {
    let body = parse_header(bytes, MSG_RELAY_LEASE)?;
    let mut l = RelayLease::grant(0, 0, [0u8; 32], [0u8; 32], 0, 0, 0, false);
    walk_tlvs(body, known_lease, |tag, val| {
        match tag {
            1 => l.lease_id = u64_be(val)?,
            2 => l.operator = u64_be(val)?,
            3 if val.len() == 64 => {
                l.participants[0].copy_from_slice(&val[..32]);
                l.participants[1].copy_from_slice(&val[32..]);
            }
            4 if val.len() == 16 => {
                l.max_bytes = u64_be(&val[..8])?;
                l.remaining_bytes = u64_be(&val[8..])?;
            }
            5 if val.len() == 12 => {
                l.expiry_ms = u64_be(&val[..8])?;
                l.generation = u32_be(&val[8..])?;
            }
            6 if val.len() == 1 => l.export_observations = val[0] != 0,
            _ => {}
        }
        Ok(())
    })?;
    Ok(l)
}

pub fn encode_custody(c: &CustodyLease, out: &mut [u8]) -> Result<usize, CscpError> {
    let mut body = [0u8; 64];
    let mut n = 0usize;
    n += write_tlv(&mut body[n..], 1 | TAG_CRITICAL, &c.lease_id.to_be_bytes())?;
    n += write_tlv(&mut body[n..], 2 | TAG_CRITICAL, &c.operator.to_be_bytes())?;
    n += write_tlv(&mut body[n..], 3 | TAG_CRITICAL, &c.expiry_unix.to_be_bytes())?;
    n += write_tlv(&mut body[n..], 4 | TAG_CRITICAL, &c.max_objects.to_be_bytes())?;
    finish(out, MSG_CUSTODY_LEASE, &body[..n])
}

pub fn decode_custody(bytes: &[u8]) -> Result<CustodyLease, CscpError> {
    let body = parse_header(bytes, MSG_CUSTODY_LEASE)?;
    let mut c = CustodyLease::grant(0, 0, 0, 0);
    walk_tlvs(body, known_lease, |tag, val| {
        match tag {
            1 => c.lease_id = u64_be(val)?,
            2 => c.operator = u64_be(val)?,
            3 => c.expiry_unix = u32_be(val)?,
            4 => c.max_objects = u16_be(val)?,
            _ => {}
        }
        Ok(())
    })?;
    Ok(c)
}

pub fn encode_evidence(e: &PathEvidence, out: &mut [u8]) -> Result<usize, CscpError> {
    let mut body = [0u8; 64];
    let mut n = 0usize;
    n += write_tlv(
        &mut body[n..],
        1 | TAG_CRITICAL,
        &[path_class_byte(e.class)],
    )?;
    n += write_tlv(&mut body[n..], 2 | TAG_CRITICAL, &e.generation.to_be_bytes())?;
    let obs = match e.observer {
        ObserverKind::LocalTransport => 1u8,
        ObserverKind::RemoteAssertion => 2u8,
    };
    n += write_tlv(&mut body[n..], 3 | TAG_CRITICAL, &[obs])?;
    n += write_tlv(&mut body[n..], 4 | TAG_CRITICAL, &[u8::from(e.validated())])?;
    let mut rest = [0u8; 14];
    rest[..2].copy_from_slice(&e.max_payload.to_be_bytes());
    rest[2..6].copy_from_slice(&e.rtt_ms.to_be_bytes());
    rest[6..].copy_from_slice(&e.observed_at_ms.to_be_bytes());
    n += write_tlv(&mut body[n..], 5, &rest)?;
    finish(out, MSG_PATH_EVIDENCE, &body[..n])
}

/// Decode a report. Never locally validated. `observer=2 && validated=1` is malformed.
pub fn decode_evidence(bytes: &[u8]) -> Result<PathEvidence, CscpError> {
    let body = parse_header(bytes, MSG_PATH_EVIDENCE)?;
    let mut class = PathClass::Offline;
    let mut generation = 0u32;
    let mut observer = 2u8;
    let mut validated = 0u8;
    let mut max_payload = 0u16;
    let mut rtt_ms = 0u32;
    let mut at = 0u64;
    walk_tlvs(body, known_evidence, |tag, val| {
        match tag {
            1 if val.len() == 1 => class = path_class_from(val[0])?,
            2 => generation = u32_be(val)?,
            3 if val.len() == 1 => observer = val[0],
            4 if val.len() == 1 => validated = val[0],
            5 if val.len() == 14 => {
                max_payload = u16_be(&val[..2])?;
                rtt_ms = u32_be(&val[2..6])?;
                at = u64_be(&val[6..])?;
            }
            _ => {}
        }
        Ok(())
    })?;
    if observer == 2 && validated != 0 {
        return Err(CscpError::Malformed);
    }
    let _ = (max_payload, rtt_ms, at);
    Ok(PathEvidence::remote_assertion(class, generation, at))
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
    finish(out, MSG_RECEIPT, &body[..n])
}

pub fn decode_receipt(bytes: &[u8]) -> Result<OpReceipt, CscpError> {
    let body = parse_header(bytes, MSG_RECEIPT)?;
    let mut r = OpReceipt::queued(0, [0u8; 32], [0u8; 32], 0, 0);
    walk_tlvs(body, known_receipt, |tag, val| {
        match tag {
            1 => r.op_id = u64_be(val)?,
            2 if val.len() == 32 => r.content_digest.copy_from_slice(val),
            3 if val.len() == 32 => r.peer.copy_from_slice(val),
            4 => r.grant_generation = u32_be(val)?,
            5 if val.len() == 1 => {
                r.status = match val[0] {
                    1 => ReceiptStatus::Queued,
                    2 => ReceiptStatus::Received,
                    3 => ReceiptStatus::Validated,
                    4 => ReceiptStatus::Committed,
                    5 => ReceiptStatus::Denied,
                    6 => ReceiptStatus::Expired,
                    _ => return Err(CscpError::Malformed),
                };
            }
            6 => r.expiry_unix = u32_be(val)?,
            _ => {}
        }
        Ok(())
    })?;
    Ok(r)
}

pub fn encode_accept(class: PathClass, generation: u32, out: &mut [u8]) -> Result<usize, CscpError> {
    let mut body = [0u8; 32];
    let mut n = 0usize;
    n += write_tlv(&mut body[n..], 1 | TAG_CRITICAL, &[path_class_byte(class)])?;
    n += write_tlv(&mut body[n..], 2 | TAG_CRITICAL, &generation.to_be_bytes())?;
    finish(out, MSG_CONNECT_ACCEPT, &body[..n])
}

pub fn decode_accept(bytes: &[u8]) -> Result<(PathClass, u32), CscpError> {
    let body = parse_header(bytes, MSG_CONNECT_ACCEPT)?;
    let mut class = PathClass::Offline;
    let mut generation = 0u32;
    walk_tlvs(body, known_accept, |tag, val| {
        match tag {
            1 if val.len() == 1 => class = path_class_from(val[0])?,
            2 => generation = u32_be(val)?,
            _ => {}
        }
        Ok(())
    })?;
    Ok((class, generation))
}

pub fn encode_reject(reason: RejectReason, out: &mut [u8]) -> Result<usize, CscpError> {
    let mut body = [0u8; 8];
    let n = write_tlv(&mut body, 1 | TAG_CRITICAL, &[reason as u8])?;
    finish(out, MSG_CONNECT_REJECT, &body[..n])
}

pub fn decode_reject(bytes: &[u8]) -> Result<RejectReason, CscpError> {
    let body = parse_header(bytes, MSG_CONNECT_REJECT)?;
    let mut reason = RejectReason::Policy;
    walk_tlvs(body, known_reject, |tag, val| {
        if tag == 1 && val.len() == 1 {
            reason = match val[0] {
                1 => RejectReason::Policy,
                2 => RejectReason::Stale,
                3 => RejectReason::Expired,
                4 => RejectReason::Revoked,
                5 => RejectReason::Capacity,
                _ => return Err(CscpError::Malformed),
            };
        }
        Ok(())
    })?;
    Ok(reason)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::peer::fabric::evidence::TransportWitness;

    #[test]
    fn contact_and_lease_round_trip() {
        let d = ContactDescriptor::mailbox([9u8; 32], 4, 2_000_000_000);
        let mut buf = [0u8; 256];
        let n = encode_contact(&d, &mut buf).unwrap();
        let back = decode_contact(&buf[..n]).unwrap();
        assert_eq!(back.generation, 4);
        let l = RelayLease::grant(7, 1, [1u8; 32], [2u8; 32], 32, 100, 1, false);
        let n = encode_relay_lease(&l, &mut buf).unwrap();
        let lb = decode_relay_lease(&buf[..n]).unwrap();
        assert_eq!(lb.max_bytes, 32);
        let c = CustodyLease::grant(3, 1, 50, 4);
        let n = encode_custody(&c, &mut buf).unwrap();
        assert_eq!(decode_custody(&buf[..n]).unwrap().max_objects, 4);
    }

    #[test]
    fn observer2_validated1_is_malformed() {
        let local = PathEvidence::from_witness(TransportWitness::from_local(
            PathClass::Relayed,
            1,
            64,
            5,
            10,
        ));
        let mut buf = [0u8; 128];
        let n = encode_evidence(&local, &mut buf).unwrap();
        let mut i = 10usize;
        while i + 3 <= n {
            let tag = buf[i] & 0x7f;
            let len = u16::from_be_bytes([buf[i + 1], buf[i + 2]]) as usize;
            if tag == 3 && len == 1 {
                buf[i + 3] = 2;
                break;
            }
            i += 3 + len;
        }
        assert_eq!(decode_evidence(&buf[..n]), Err(CscpError::Malformed));
    }

    #[test]
    fn accept_reject_receipt_round_trip() {
        let mut buf = [0u8; 256];
        let n = encode_accept(PathClass::Relayed, 3, &mut buf).unwrap();
        assert_eq!(decode_accept(&buf[..n]).unwrap(), (PathClass::Relayed, 3));
        let n = encode_reject(RejectReason::Stale, &mut buf).unwrap();
        assert_eq!(decode_reject(&buf[..n]).unwrap(), RejectReason::Stale);
        let r = OpReceipt::queued(9, [3u8; 32], [1u8; 32], 4, 100);
        let n = encode_receipt(&r, &mut buf).unwrap();
        let back = decode_receipt(&buf[..n]).unwrap();
        assert_eq!(back.op_id, 9);
        assert_eq!(back.status, ReceiptStatus::Queued);
    }

    fn append_unknown_critical(buf: &mut [u8], n: usize) -> usize {
        let extra = super::super::write_tlv(&mut buf[n..], 0x7f | TAG_CRITICAL, &[9]).unwrap();
        let blen = u16::from_be_bytes([buf[8], buf[9]]) + extra as u16;
        buf[8..10].copy_from_slice(&blen.to_be_bytes());
        n + extra
    }

    #[test]
    fn unknown_critical_rejected_on_every_message_type() {
        let mut buf = [0u8; 512];
        let d = ContactDescriptor::mailbox([9u8; 32], 4, 2_000_000_000);
        let encoded = encode_contact(&d, &mut buf).unwrap();
        let n = append_unknown_critical(&mut buf, encoded);
        assert_eq!(decode_contact(&buf[..n]), Err(CscpError::UnknownCritical));
        let l = RelayLease::grant(7, 1, [1u8; 32], [2u8; 32], 32, 100, 1, false);
        let encoded = encode_relay_lease(&l, &mut buf).unwrap();
        let n = append_unknown_critical(&mut buf, encoded);
        assert_eq!(decode_relay_lease(&buf[..n]), Err(CscpError::UnknownCritical));
        let c = CustodyLease::grant(3, 1, 50, 4);
        let encoded = encode_custody(&c, &mut buf).unwrap();
        let n = append_unknown_critical(&mut buf, encoded);
        assert_eq!(decode_custody(&buf[..n]), Err(CscpError::UnknownCritical));
        let local = PathEvidence::from_witness(TransportWitness::from_local(
            PathClass::Relayed,
            1,
            64,
            5,
            10,
        ));
        let encoded = encode_evidence(&local, &mut buf).unwrap();
        let n = append_unknown_critical(&mut buf, encoded);
        assert_eq!(decode_evidence(&buf[..n]), Err(CscpError::UnknownCritical));
        let r = OpReceipt::queued(9, [3u8; 32], [1u8; 32], 4, 100);
        let encoded = encode_receipt(&r, &mut buf).unwrap();
        let n = append_unknown_critical(&mut buf, encoded);
        assert_eq!(decode_receipt(&buf[..n]), Err(CscpError::UnknownCritical));
        let encoded = encode_accept(PathClass::Relayed, 3, &mut buf).unwrap();
        let n = append_unknown_critical(&mut buf, encoded);
        assert_eq!(decode_accept(&buf[..n]), Err(CscpError::UnknownCritical));
        let encoded = encode_reject(RejectReason::Policy, &mut buf).unwrap();
        let n = append_unknown_critical(&mut buf, encoded);
        assert_eq!(decode_reject(&buf[..n]), Err(CscpError::UnknownCritical));
    }

    #[test]
    fn decoded_evidence_is_never_locally_validated() {
        let local = PathEvidence::from_witness(TransportWitness::from_local(
            PathClass::Relayed,
            7,
            64,
            5,
            10,
        ));
        assert!(local.validated());
        let mut buf = [0u8; 128];
        let n = encode_evidence(&local, &mut buf).unwrap();
        let got = decode_evidence(&buf[..n]).unwrap();
        assert!(!got.validated());
        assert_eq!(got.observer, ObserverKind::RemoteAssertion);
        assert_eq!(got.class, PathClass::Relayed);
        assert_eq!(got.generation, 7);
    }
}
