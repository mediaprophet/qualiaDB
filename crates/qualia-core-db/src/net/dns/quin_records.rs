//! QualiaDB Super-Quin (NQuin) DNS and SDN Record Codec.
//!
//! Maps standard DNS records and Qualia SDN Front-Door records to 48-byte
//! `NQuin` statements in QualiaDB's Q42 universe.
//!
//! Adheres strictly to QualiaDB Rule 0-A (zero-heap in hot paths) and
//! Rule 0-B (clean modular decomposition under 500 lines).

#![allow(dead_code)]

use super::wire::{DnsRecordView, DnsType};
use crate::{q_hash, NQuin, PermissiveRoutingLane};

/// Modality opcodes for DNS in predicate bits [0..7] (Rule 0: all new modalities >= 0x10).
pub const OP_DNS_A: u8 = 0x50;
pub const OP_DNS_NS: u8 = 0x51;
pub const OP_DNS_CNAME: u8 = 0x52;
pub const OP_DNS_SOA: u8 = 0x53;
pub const OP_DNS_TXT: u8 = 0x54;
pub const OP_DNS_AAAA: u8 = 0x55;
pub const OP_DNS_SRV: u8 = 0x56;
pub const OP_DNS_CERT: u8 = 0x57;
pub const OP_DNS_QDP_FRONT_DOOR: u8 = 0x58;
pub const OP_DNS_SDN_PEER: u8 = 0x59;
pub const OP_DNS_SDN_ROUTING: u8 = 0x5A;
pub const OP_DNS_PTR: u8 = 0x5B;
pub const OP_DNS_MX: u8 = 0x5C;
pub const OP_DNS_CAA: u8 = 0x5D;
pub const OP_DNS_TLSA: u8 = 0x5E;
pub const OP_DNS_HTTPS: u8 = 0x5F;
pub const OP_DNS_DNSKEY: u8 = 0x60;
pub const OP_DNS_DS: u8 = 0x61;

/// Inline type tag for integers in object bits [60..62] (resolver.rs authoritative layout).
pub const INLINE_TAG_INTEGER: u64 = 0b001u64 << 60;
/// Bit 63 sentinel for did:q42 topological pointer.
pub const DID_Q42_FLAG: u64 = 1u64 << 63;

/// Default DNS context URI hash.
pub const DNS_CONTEXT_HASH: u64 = q_hash("qualia:dns:zone");

/// Encode an IPv4 `A` record as an `NQuin`.
pub fn encode_a_record(
    domain: &str,
    ipv4_octets: [u8; 4],
    ttl: u32,
    lane: PermissiveRoutingLane,
) -> NQuin {
    let subject = q_hash(domain);
    let pred_hash = q_hash("qualia:dns:a");
    let predicate = (pred_hash << 8) | (OP_DNS_A as u64);
    let ip_u32 = u32::from_be_bytes(ipv4_octets) as u64;
    let object = INLINE_TAG_INTEGER | (ip_u32 & 0x0FFF_FFFF_FFFF_FFFF);
    let context = DNS_CONTEXT_HASH;
    let metadata = ((lane as u64) << 61) | (ttl as u64 & 0xFFFF_FFFF);
    let parity = NQuin::calculate_parity(subject, predicate, object, context, metadata);
    NQuin {
        subject,
        predicate,
        object,
        context,
        metadata,
        parity,
    }
}

/// Decode an `NQuin` into an IPv4 address if it represents an `A` record.
pub fn decode_a_record(quin: &NQuin) -> Option<([u8; 4], u32, PermissiveRoutingLane)> {
    if !quin.verify_ecc_parity() || (quin.predicate & 0xFF) as u8 != OP_DNS_A {
        return None;
    }
    let tag = quin.object & (0b111u64 << 60);
    if tag != INLINE_TAG_INTEGER {
        return None;
    }
    let ip_val = (quin.object & 0xFFFF_FFFF) as u32;
    let octets = ip_val.to_be_bytes();
    let ttl = (quin.metadata & 0xFFFF_FFFF) as u32;
    let lane = quin.identify_routing_lane();
    Some((octets, ttl, lane))
}

/// Encode an IPv6 `AAAA` record as an `NQuin`.
///
/// Losslessly packs all 128 bits:
/// - `object`: upper 64 bits of IPv6
/// - `context`: lower 64 bits of IPv6
pub fn encode_aaaa_record(
    domain: &str,
    ipv6_octets: [u8; 16],
    ttl: u32,
    lane: PermissiveRoutingLane,
) -> NQuin {
    let subject = q_hash(domain);
    let pred_hash = q_hash("qualia:dns:aaaa");
    let predicate = (pred_hash << 8) | (OP_DNS_AAAA as u64);
    let mut hi = [0u8; 8];
    let mut lo = [0u8; 8];
    hi.copy_from_slice(&ipv6_octets[0..8]);
    lo.copy_from_slice(&ipv6_octets[8..16]);
    let object = u64::from_be_bytes(hi);
    let context = u64::from_be_bytes(lo);
    let metadata = ((lane as u64) << 61) | (ttl as u64 & 0xFFFF_FFFF);
    let parity = NQuin::calculate_parity(subject, predicate, object, context, metadata);
    NQuin {
        subject,
        predicate,
        object,
        context,
        metadata,
        parity,
    }
}

/// Decode an `NQuin` into an IPv6 address if it represents an `AAAA` record.
pub fn decode_aaaa_record(quin: &NQuin) -> Option<([u8; 16], u32, PermissiveRoutingLane)> {
    if !quin.verify_ecc_parity() || (quin.predicate & 0xFF) as u8 != OP_DNS_AAAA {
        return None;
    }
    let mut ip = [0u8; 16];
    ip[0..8].copy_from_slice(&quin.object.to_be_bytes());
    ip[8..16].copy_from_slice(&quin.context.to_be_bytes());
    let ttl = (quin.metadata & 0xFFFF_FFFF) as u32;
    let lane = quin.identify_routing_lane();
    Some((ip, ttl, lane))
}

/// Encode a `CNAME` record as an `NQuin`.
pub fn encode_cname_record(
    domain: &str,
    target: &str,
    ttl: u32,
    lane: PermissiveRoutingLane,
) -> NQuin {
    let subject = q_hash(domain);
    let pred_hash = q_hash("qualia:dns:cname");
    let predicate = (pred_hash << 8) | (OP_DNS_CNAME as u64);
    let object = q_hash(target);
    let context = DNS_CONTEXT_HASH;
    let metadata = ((lane as u64) << 61) | (ttl as u64 & 0xFFFF_FFFF);
    let parity = NQuin::calculate_parity(subject, predicate, object, context, metadata);
    NQuin {
        subject,
        predicate,
        object,
        context,
        metadata,
        parity,
    }
}

/// Decode a `CNAME` record `(target_hash, ttl, lane)` from an `NQuin`.
pub fn decode_cname_record(quin: &NQuin) -> Option<(u64, u32, PermissiveRoutingLane)> {
    if !quin.verify_ecc_parity() || (quin.predicate & 0xFF) as u8 != OP_DNS_CNAME {
        return None;
    }
    Some((
        quin.object,
        (quin.metadata & 0xFFFF_FFFF) as u32,
        quin.identify_routing_lane(),
    ))
}

/// Encode an `NS` record as an `NQuin`.
pub fn encode_ns_record(domain: &str, ns: &str, ttl: u32, lane: PermissiveRoutingLane) -> NQuin {
    let subject = q_hash(domain);
    let pred_hash = q_hash("qualia:dns:ns");
    let predicate = (pred_hash << 8) | (OP_DNS_NS as u64);
    let object = q_hash(ns);
    let context = DNS_CONTEXT_HASH;
    let metadata = ((lane as u64) << 61) | (ttl as u64 & 0xFFFF_FFFF);
    let parity = NQuin::calculate_parity(subject, predicate, object, context, metadata);
    NQuin {
        subject,
        predicate,
        object,
        context,
        metadata,
        parity,
    }
}

/// Decode an `NS` record `(ns_hash, ttl, lane)` from an `NQuin`.
pub fn decode_ns_record(quin: &NQuin) -> Option<(u64, u32, PermissiveRoutingLane)> {
    if !quin.verify_ecc_parity() || (quin.predicate & 0xFF) as u8 != OP_DNS_NS {
        return None;
    }
    Some((
        quin.object,
        (quin.metadata & 0xFFFF_FFFF) as u32,
        quin.identify_routing_lane(),
    ))
}

/// Encode a `PTR` record as an `NQuin`.
pub fn encode_ptr_record(
    rev_domain: &str,
    target: &str,
    ttl: u32,
    lane: PermissiveRoutingLane,
) -> NQuin {
    let subject = q_hash(rev_domain);
    let pred_hash = q_hash("qualia:dns:ptr");
    let predicate = (pred_hash << 8) | (OP_DNS_PTR as u64);
    let object = q_hash(target);
    let context = DNS_CONTEXT_HASH;
    let metadata = ((lane as u64) << 61) | (ttl as u64 & 0xFFFF_FFFF);
    let parity = NQuin::calculate_parity(subject, predicate, object, context, metadata);
    NQuin {
        subject,
        predicate,
        object,
        context,
        metadata,
        parity,
    }
}

/// Decode a `PTR` record `(target_hash, ttl, lane)` from an `NQuin`.
pub fn decode_ptr_record(quin: &NQuin) -> Option<(u64, u32, PermissiveRoutingLane)> {
    if !quin.verify_ecc_parity() || (quin.predicate & 0xFF) as u8 != OP_DNS_PTR {
        return None;
    }
    Some((
        quin.object,
        (quin.metadata & 0xFFFF_FFFF) as u32,
        quin.identify_routing_lane(),
    ))
}

/// Encode an `MX` record as an `NQuin`.
/// `object` packs `(preference << 48) | (exchange_hash & 0x0000_FFFF_FFFF_FFFF)`.
pub fn encode_mx_record(
    domain: &str,
    preference: u16,
    exchange: &str,
    ttl: u32,
    lane: PermissiveRoutingLane,
) -> NQuin {
    let subject = q_hash(domain);
    let pred_hash = q_hash("qualia:dns:mx");
    let predicate = (pred_hash << 8) | (OP_DNS_MX as u64);
    let object = ((preference as u64) << 48) | (q_hash(exchange) & 0x0000_FFFF_FFFF_FFFF);
    let context = DNS_CONTEXT_HASH;
    let metadata = ((lane as u64) << 61) | (ttl as u64 & 0xFFFF_FFFF);
    let parity = NQuin::calculate_parity(subject, predicate, object, context, metadata);
    NQuin {
        subject,
        predicate,
        object,
        context,
        metadata,
        parity,
    }
}

/// Decode an `MX` record `(preference, exchange_hash, ttl, lane)` from an `NQuin`.
pub fn decode_mx_record(quin: &NQuin) -> Option<(u16, u64, u32, PermissiveRoutingLane)> {
    if !quin.verify_ecc_parity() || (quin.predicate & 0xFF) as u8 != OP_DNS_MX {
        return None;
    }
    let preference = (quin.object >> 48) as u16;
    let exchange_hash = quin.object & 0x0000_FFFF_FFFF_FFFF;
    Some((
        preference,
        exchange_hash,
        (quin.metadata & 0xFFFF_FFFF) as u32,
        quin.identify_routing_lane(),
    ))
}

/// Encode a `SOA` record as an `NQuin`.
pub fn encode_soa_record(
    domain: &str,
    mname: &str,
    rname: &str,
    serial: u32,
    ttl: u32,
    lane: PermissiveRoutingLane,
) -> NQuin {
    let subject = q_hash(domain);
    let pred_hash = q_hash("qualia:dns:soa");
    let predicate = (pred_hash << 8) | (OP_DNS_SOA as u64);
    let object = ((serial as u64) << 32) | (q_hash(mname) & 0xFFFF_FFFF);
    let context = q_hash(rname);
    let metadata = ((lane as u64) << 61) | (ttl as u64 & 0xFFFF_FFFF);
    let parity = NQuin::calculate_parity(subject, predicate, object, context, metadata);
    NQuin {
        subject,
        predicate,
        object,
        context,
        metadata,
        parity,
    }
}

/// Decode a `SOA` record `(serial, mname_hash, rname_hash, ttl, lane)` from an `NQuin`.
pub fn decode_soa_record(quin: &NQuin) -> Option<(u32, u64, u64, u32, PermissiveRoutingLane)> {
    if !quin.verify_ecc_parity() || (quin.predicate & 0xFF) as u8 != OP_DNS_SOA {
        return None;
    }
    let serial = (quin.object >> 32) as u32;
    let mname_hash = quin.object & 0xFFFF_FFFF;
    let rname_hash = quin.context;
    Some((
        serial,
        mname_hash,
        rname_hash,
        (quin.metadata & 0xFFFF_FFFF) as u32,
        quin.identify_routing_lane(),
    ))
}

/// Encode an `SRV` record as an `NQuin`.
pub fn encode_srv_record(
    domain: &str,
    priority: u16,
    weight: u16,
    port: u16,
    target: &str,
    ttl: u32,
    lane: PermissiveRoutingLane,
) -> NQuin {
    let subject = q_hash(domain);
    let pred_hash = q_hash("qualia:dns:srv");
    let predicate = (pred_hash << 8) | (OP_DNS_SRV as u64);
    let object = ((priority as u64) << 32) | ((weight as u64) << 16) | (port as u64);
    let context = q_hash(target);
    let metadata = ((lane as u64) << 61) | (ttl as u64 & 0xFFFF_FFFF);
    let parity = NQuin::calculate_parity(subject, predicate, object, context, metadata);
    NQuin {
        subject,
        predicate,
        object,
        context,
        metadata,
        parity,
    }
}

/// Decode an `SRV` record `(priority, weight, port, target_hash, ttl, lane)` from an `NQuin`.
pub fn decode_srv_record(quin: &NQuin) -> Option<(u16, u16, u16, u64, u32, PermissiveRoutingLane)> {
    if !quin.verify_ecc_parity() || (quin.predicate & 0xFF) as u8 != OP_DNS_SRV {
        return None;
    }
    let priority = (quin.object >> 32) as u16;
    let weight = (quin.object >> 16) as u16;
    let port = (quin.object & 0xFFFF) as u16;
    let target_hash = quin.context;
    Some((
        priority,
        weight,
        port,
        target_hash,
        (quin.metadata & 0xFFFF_FFFF) as u32,
        quin.identify_routing_lane(),
    ))
}

/// Encode a `CAA` record as an `NQuin`.
pub fn encode_caa_record(
    domain: &str,
    flags: u8,
    tag: &str,
    value: &str,
    ttl: u32,
    lane: PermissiveRoutingLane,
) -> NQuin {
    let subject = q_hash(domain);
    let pred_hash = q_hash("qualia:dns:caa");
    let predicate = (pred_hash << 8) | (OP_DNS_CAA as u64);
    let object = ((flags as u64) << 56) | (q_hash(tag) & 0x00FF_FFFF_FFFF_FFFF);
    let context = q_hash(value);
    let metadata = ((lane as u64) << 61) | (ttl as u64 & 0xFFFF_FFFF);
    let parity = NQuin::calculate_parity(subject, predicate, object, context, metadata);
    NQuin {
        subject,
        predicate,
        object,
        context,
        metadata,
        parity,
    }
}

/// Encode a `TLSA` record as an `NQuin`.
pub fn encode_tlsa_record(
    domain: &str,
    usage: u8,
    selector: u8,
    matching_type: u8,
    cert_hash: u64,
    ttl: u32,
    lane: PermissiveRoutingLane,
) -> NQuin {
    let subject = q_hash(domain);
    let pred_hash = q_hash("qualia:dns:tlsa");
    let predicate = (pred_hash << 8) | (OP_DNS_TLSA as u64);
    let object = ((usage as u64) << 56)
        | ((selector as u64) << 48)
        | ((matching_type as u64) << 40)
        | (cert_hash & 0x0000_00FF_FFFF_FFFF);
    let context = cert_hash;
    let metadata = ((lane as u64) << 61) | (ttl as u64 & 0xFFFF_FFFF);
    let parity = NQuin::calculate_parity(subject, predicate, object, context, metadata);
    NQuin {
        subject,
        predicate,
        object,
        context,
        metadata,
        parity,
    }
}

/// Encode an `HTTPS` / `SVCB` record as an `NQuin`.
pub fn encode_https_record(
    domain: &str,
    priority: u16,
    target: &str,
    params_hash: u64,
    ttl: u32,
    lane: PermissiveRoutingLane,
) -> NQuin {
    let subject = q_hash(domain);
    let pred_hash = q_hash("qualia:dns:https");
    let predicate = (pred_hash << 8) | (OP_DNS_HTTPS as u64);
    let object = ((priority as u64) << 48) | (q_hash(target) & 0x0000_FFFF_FFFF_FFFF);
    let context = params_hash;
    let metadata = ((lane as u64) << 61) | (ttl as u64 & 0xFFFF_FFFF);
    let parity = NQuin::calculate_parity(subject, predicate, object, context, metadata);
    NQuin {
        subject,
        predicate,
        object,
        context,
        metadata,
        parity,
    }
}

/// Encode a `DNSKEY` record as an `NQuin`.
pub fn encode_dnskey_record(
    domain: &str,
    flags: u16,
    protocol: u8,
    algorithm: u8,
    key_hash: u64,
    ttl: u32,
    lane: PermissiveRoutingLane,
) -> NQuin {
    let subject = q_hash(domain);
    let pred_hash = q_hash("qualia:dns:dnskey");
    let predicate = (pred_hash << 8) | (OP_DNS_DNSKEY as u64);
    let object = ((flags as u64) << 48)
        | ((protocol as u64) << 40)
        | ((algorithm as u64) << 32)
        | (key_hash & 0xFFFF_FFFF);
    let context = key_hash;
    let metadata = ((lane as u64) << 61) | (ttl as u64 & 0xFFFF_FFFF);
    let parity = NQuin::calculate_parity(subject, predicate, object, context, metadata);
    NQuin {
        subject,
        predicate,
        object,
        context,
        metadata,
        parity,
    }
}

/// Encode a `DS` record as an `NQuin`.
pub fn encode_ds_record(
    domain: &str,
    key_tag: u16,
    algorithm: u8,
    digest_type: u8,
    digest_hash: u64,
    ttl: u32,
    lane: PermissiveRoutingLane,
) -> NQuin {
    let subject = q_hash(domain);
    let pred_hash = q_hash("qualia:dns:ds");
    let predicate = (pred_hash << 8) | (OP_DNS_DS as u64);
    let object = ((key_tag as u64) << 48)
        | ((algorithm as u64) << 40)
        | ((digest_type as u64) << 32)
        | (digest_hash & 0xFFFF_FFFF);
    let context = digest_hash;
    let metadata = ((lane as u64) << 61) | (ttl as u64 & 0xFFFF_FFFF);
    let parity = NQuin::calculate_parity(subject, predicate, object, context, metadata);
    NQuin {
        subject,
        predicate,
        object,
        context,
        metadata,
        parity,
    }
}

/// Encode a generic DNS TXT or Front-Door entry into an `NQuin`.
pub fn encode_txt_record(domain: &str, text: &str, ttl: u32, lane: PermissiveRoutingLane) -> NQuin {
    let subject = q_hash(domain);
    let pred_hash = q_hash("qualia:dns:txt");
    let predicate = (pred_hash << 8) | (OP_DNS_TXT as u64);
    let object = q_hash(text);
    let context = DNS_CONTEXT_HASH;
    let metadata = ((lane as u64) << 61) | (ttl as u64 & 0xFFFF_FFFF);
    let parity = NQuin::calculate_parity(subject, predicate, object, context, metadata);
    NQuin {
        subject,
        predicate,
        object,
        context,
        metadata,
        parity,
    }
}

/// Encode a Qualia Socially Defined Network (SDN) Front-Door entry into an `NQuin`.
pub fn encode_sdn_front_door(
    domain: &str,
    did_topological_pointer: u64,
    ttl: u32,
    lane: PermissiveRoutingLane,
) -> NQuin {
    let subject = q_hash(domain);
    let pred_hash = q_hash("qualia:sdn:frontDoor");
    let predicate = (pred_hash << 8) | (OP_DNS_QDP_FRONT_DOOR as u64);
    let object = did_topological_pointer;
    let context = q_hash("qualia:sdn:overlay");
    let metadata = ((lane as u64) << 61) | (ttl as u64 & 0xFFFF_FFFF);
    let parity = NQuin::calculate_parity(subject, predicate, object, context, metadata);
    NQuin {
        subject,
        predicate,
        object,
        context,
        metadata,
        parity,
    }
}

/// Map a wire record view into an `NQuin`, resolving compressed domain names when present.
pub fn wire_record_view_to_quin(
    name: &str,
    rec: &DnsRecordView<'_>,
    lane: PermissiveRoutingLane,
) -> Option<NQuin> {
    match rec.rtype {
        DnsType::A => {
            if rec.rdata.len() != 4 {
                return None;
            }
            let mut ip = [0u8; 4];
            ip.copy_from_slice(rec.rdata);
            Some(encode_a_record(name, ip, rec.ttl, lane))
        }
        DnsType::AAAA => {
            if rec.rdata.len() != 16 {
                return None;
            }
            let mut ip = [0u8; 16];
            ip.copy_from_slice(rec.rdata);
            Some(encode_aaaa_record(name, ip, rec.ttl, lane))
        }
        DnsType::CNAME => {
            let mut buf = [0u8; 256];
            let (_, len) =
                super::wire::decode_domain_name(rec.packet, rec.rdata_offset, &mut buf).ok()?;
            let target = std::str::from_utf8(&buf[..len]).ok()?;
            Some(encode_cname_record(name, target, rec.ttl, lane))
        }
        DnsType::NS => {
            let mut buf = [0u8; 256];
            let (_, len) =
                super::wire::decode_domain_name(rec.packet, rec.rdata_offset, &mut buf).ok()?;
            let target = std::str::from_utf8(&buf[..len]).ok()?;
            Some(encode_ns_record(name, target, rec.ttl, lane))
        }
        DnsType::PTR => {
            let mut buf = [0u8; 256];
            let (_, len) =
                super::wire::decode_domain_name(rec.packet, rec.rdata_offset, &mut buf).ok()?;
            let target = std::str::from_utf8(&buf[..len]).ok()?;
            Some(encode_ptr_record(name, target, rec.ttl, lane))
        }
        DnsType::MX => {
            if rec.rdata.len() < 2 {
                return None;
            }
            let preference = u16::from_be_bytes([rec.rdata[0], rec.rdata[1]]);
            let mut buf = [0u8; 256];
            let (_, len) =
                super::wire::decode_domain_name(rec.packet, rec.rdata_offset + 2, &mut buf).ok()?;
            let exchange = std::str::from_utf8(&buf[..len]).ok()?;
            Some(encode_mx_record(name, preference, exchange, rec.ttl, lane))
        }
        DnsType::SRV => {
            if rec.rdata.len() < 6 {
                return None;
            }
            let priority = u16::from_be_bytes([rec.rdata[0], rec.rdata[1]]);
            let weight = u16::from_be_bytes([rec.rdata[2], rec.rdata[3]]);
            let port = u16::from_be_bytes([rec.rdata[4], rec.rdata[5]]);
            let mut buf = [0u8; 256];
            let (_, len) =
                super::wire::decode_domain_name(rec.packet, rec.rdata_offset + 6, &mut buf).ok()?;
            let target = std::str::from_utf8(&buf[..len]).ok()?;
            Some(encode_srv_record(
                name, priority, weight, port, target, rec.ttl, lane,
            ))
        }
        DnsType::CAA => {
            if rec.rdata.len() < 2 {
                return None;
            }
            let flags = rec.rdata[0];
            let tag_len = rec.rdata[1] as usize;
            if rec.rdata.len() < 2 + tag_len {
                return None;
            }
            let tag = std::str::from_utf8(&rec.rdata[2..2 + tag_len]).ok()?;
            let value = std::str::from_utf8(&rec.rdata[2 + tag_len..]).ok()?;
            Some(encode_caa_record(name, flags, tag, value, rec.ttl, lane))
        }
        DnsType::TLSA => {
            if rec.rdata.len() < 3 {
                return None;
            }
            let usage = rec.rdata[0];
            let selector = rec.rdata[1];
            let mtype = rec.rdata[2];
            let cert_hash = if rec.rdata.len() >= 11 {
                let mut b = [0u8; 8];
                b.copy_from_slice(&rec.rdata[3..11]);
                u64::from_be_bytes(b)
            } else {
                0
            };
            Some(encode_tlsa_record(
                name, usage, selector, mtype, cert_hash, rec.ttl, lane,
            ))
        }
        DnsType::HTTPS | DnsType::SVCB => {
            if rec.rdata.len() < 2 {
                return None;
            }
            let priority = u16::from_be_bytes([rec.rdata[0], rec.rdata[1]]);
            let mut buf = [0u8; 256];
            let (next_off, len) =
                super::wire::decode_domain_name(rec.packet, rec.rdata_offset + 2, &mut buf).ok()?;
            let target = std::str::from_utf8(&buf[..len]).ok()?;
            let params_hash =
                if next_off < rec.packet.len() && next_off < rec.rdata_offset + rec.rdata.len() {
                    let params = &rec.packet[next_off..rec.rdata_offset + rec.rdata.len()];
                    let mut h = 0xcbf29ce484222325u64;
                    for b in params {
                        h = (h ^ (*b as u64)).wrapping_mul(0x100000001b3);
                    }
                    h
                } else {
                    0
                };
            Some(encode_https_record(
                name,
                priority,
                target,
                params_hash,
                rec.ttl,
                lane,
            ))
        }
        DnsType::DNSKEY => {
            if rec.rdata.len() < 4 {
                return None;
            }
            let flags = u16::from_be_bytes([rec.rdata[0], rec.rdata[1]]);
            let protocol = rec.rdata[2];
            let algorithm = rec.rdata[3];
            let mut h = 0xcbf29ce484222325u64;
            for b in &rec.rdata[4..] {
                h = (h ^ (*b as u64)).wrapping_mul(0x100000001b3);
            }
            Some(encode_dnskey_record(
                name, flags, protocol, algorithm, h, rec.ttl, lane,
            ))
        }
        DnsType::DS => {
            if rec.rdata.len() < 4 {
                return None;
            }
            let key_tag = u16::from_be_bytes([rec.rdata[0], rec.rdata[1]]);
            let algorithm = rec.rdata[2];
            let digest_type = rec.rdata[3];
            let mut h = 0xcbf29ce484222325u64;
            for b in &rec.rdata[4..] {
                h = (h ^ (*b as u64)).wrapping_mul(0x100000001b3);
            }
            Some(encode_ds_record(
                name,
                key_tag,
                algorithm,
                digest_type,
                h,
                rec.ttl,
                lane,
            ))
        }
        DnsType::TXT => {
            let mut text_buf = [0u8; 256];
            let mut cursor = 0;
            let mut out_len = 0;
            while cursor < rec.rdata.len() {
                let segment_len = rec.rdata[cursor] as usize;
                cursor += 1;
                if cursor + segment_len > rec.rdata.len() {
                    break;
                }
                let copy_len = core::cmp::min(segment_len, text_buf.len() - out_len);
                text_buf[out_len..out_len + copy_len]
                    .copy_from_slice(&rec.rdata[cursor..cursor + copy_len]);
                out_len += copy_len;
                cursor += segment_len;
            }
            if let Ok(text) = std::str::from_utf8(&text_buf[..out_len]) {
                Some(encode_txt_record(name, text, rec.ttl, lane))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Fallback wrapper for raw RDATA when full packet is not provided.
pub fn wire_record_to_quin(
    name: &str,
    rtype: DnsType,
    ttl: u32,
    rdata: &[u8],
    lane: PermissiveRoutingLane,
) -> Option<NQuin> {
    let dummy_packet = rdata;
    let rec = DnsRecordView {
        rtype,
        rclass: 1,
        ttl,
        rdata,
        rdata_offset: 0,
        packet: dummy_packet,
    };
    wire_record_view_to_quin(name, &rec, lane)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_record_roundtrip() {
        let quin = encode_a_record(
            "node1.qualia.net",
            [192, 168, 1, 42],
            300,
            PermissiveRoutingLane::EnforceBilateralMicroCommons,
        );
        assert!(quin.verify_ecc_parity());
        assert_eq!(
            quin.identify_routing_lane(),
            PermissiveRoutingLane::EnforceBilateralMicroCommons
        );

        let (ip, ttl, lane) = decode_a_record(&quin).unwrap();
        assert_eq!(ip, [192, 168, 1, 42]);
        assert_eq!(ttl, 300);
        assert_eq!(lane, PermissiveRoutingLane::EnforceBilateralMicroCommons);
    }

    #[test]
    fn aaaa_record_lossless_roundtrip() {
        let ip_v6: [u8; 16] = [
            0x26, 0x06, 0x47, 0x00, 0x47, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x11, 0x11,
        ];
        let quin = encode_aaaa_record(
            "cloudflare-dns.com",
            ip_v6,
            3600,
            PermissiveRoutingLane::PassthroughStandard,
        );
        assert!(quin.verify_ecc_parity());

        let (decoded_ip, ttl, lane) = decode_aaaa_record(&quin).unwrap();
        assert_eq!(decoded_ip, ip_v6);
        assert_eq!(ttl, 3600);
        assert_eq!(lane, PermissiveRoutingLane::PassthroughStandard);
    }

    #[test]
    fn mx_record_roundtrip() {
        let quin = encode_mx_record(
            "qualia.net",
            10,
            "mail.qualia.net",
            600,
            PermissiveRoutingLane::PassthroughStandard,
        );
        assert!(quin.verify_ecc_parity());
        let (pref, exch_hash, ttl, _) = decode_mx_record(&quin).unwrap();
        assert_eq!(pref, 10);
        assert_eq!(exch_hash, q_hash("mail.qualia.net") & 0x0000_FFFF_FFFF_FFFF);
        assert_eq!(ttl, 600);
    }

    #[test]
    fn srv_record_roundtrip() {
        let quin = encode_srv_record(
            "_imaps._tcp.qualia.net",
            5,
            0,
            993,
            "mail.qualia.net",
            1800,
            PermissiveRoutingLane::EnforceBilateralMicroCommons,
        );
        assert!(quin.verify_ecc_parity());
        let (prio, weight, port, target_hash, ttl, _) = decode_srv_record(&quin).unwrap();
        assert_eq!(prio, 5);
        assert_eq!(weight, 0);
        assert_eq!(port, 993);
        assert_eq!(target_hash, q_hash("mail.qualia.net"));
        assert_eq!(ttl, 1800);
    }

    #[test]
    fn sdn_front_door_encoding() {
        let did_ptr = DID_Q42_FLAG | 0x1234_5678_9ABC;
        let quin = encode_sdn_front_door(
            "alice.webizen.net",
            did_ptr,
            600,
            PermissiveRoutingLane::EnforcePermissiveCommons,
        );
        assert!(quin.verify_ecc_parity());
        assert_eq!(quin.object & DID_Q42_FLAG, DID_Q42_FLAG);
        assert_eq!(
            quin.identify_routing_lane(),
            PermissiveRoutingLane::EnforcePermissiveCommons
        );
    }
}
