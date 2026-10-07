//! Zero-allocation RFC 1035 / RFC 3597 binary DNS wire parser & encoder.
//!
//! Follows QualiaDB 0-A zero-heap hot path rules: operates over caller-supplied
//! buffers without allocating `Vec` or `String`.

#![allow(dead_code)]

/// DNS header flags bitmasks (RFC 1035 §4.1.1).
pub const FLAG_QR_RESPONSE: u16 = 0x8000;
pub const OPCODE_MASK: u16 = 0x7800;
pub const FLAG_AA_AUTHORITATIVE: u16 = 0x0400;
pub const FLAG_TC_TRUNCATED: u16 = 0x0200;
pub const FLAG_RD_RECURSION_DESIRED: u16 = 0x0100;
pub const FLAG_RA_RECURSION_AVAILABLE: u16 = 0x0080;
pub const FLAG_AD_AUTHENTICATED_DATA: u16 = 0x0020;
pub const FLAG_CD_CHECKING_DISABLED: u16 = 0x0010;
pub const RCODE_MASK: u16 = 0x000F;

/// Standard RR Type identifiers (RFC 1035, RFC 3597, RFC 4034).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum DnsType {
    A = 1,
    NS = 2,
    CNAME = 5,
    SOA = 6,
    PTR = 12,
    MX = 15,
    TXT = 16,
    AAAA = 28,
    SRV = 33,
    CERT = 37,
    OPT = 41,
    DS = 43,
    RRSIG = 46,
    NSEC = 47,
    DNSKEY = 48,
    NSEC3 = 50,
    TLSA = 52,
    SVCB = 64,
    HTTPS = 65,
    CAA = 257,
    Unknown(u16),
}

impl From<u16> for DnsType {
    fn from(val: u16) -> Self {
        match val {
            1 => Self::A,
            2 => Self::NS,
            5 => Self::CNAME,
            6 => Self::SOA,
            12 => Self::PTR,
            15 => Self::MX,
            16 => Self::TXT,
            28 => Self::AAAA,
            33 => Self::SRV,
            37 => Self::CERT,
            41 => Self::OPT,
            43 => Self::DS,
            46 => Self::RRSIG,
            47 => Self::NSEC,
            48 => Self::DNSKEY,
            50 => Self::NSEC3,
            52 => Self::TLSA,
            64 => Self::SVCB,
            65 => Self::HTTPS,
            257 => Self::CAA,
            other => Self::Unknown(other),
        }
    }
}

impl From<DnsType> for u16 {
    fn from(t: DnsType) -> Self {
        match t {
            DnsType::A => 1,
            DnsType::NS => 2,
            DnsType::CNAME => 5,
            DnsType::SOA => 6,
            DnsType::PTR => 12,
            DnsType::MX => 15,
            DnsType::TXT => 16,
            DnsType::AAAA => 28,
            DnsType::SRV => 33,
            DnsType::CERT => 37,
            DnsType::OPT => 41,
            DnsType::DS => 43,
            DnsType::RRSIG => 46,
            DnsType::NSEC => 47,
            DnsType::DNSKEY => 48,
            DnsType::NSEC3 => 50,
            DnsType::TLSA => 52,
            DnsType::SVCB => 64,
            DnsType::HTTPS => 65,
            DnsType::CAA => 257,
            DnsType::Unknown(other) => other,
        }
    }
}

/// Standard DNS class (IN = Internet).
pub const CLASS_IN: u16 = 1;

/// Fixed 12-byte DNS message header (RFC 1035 §4.1.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct DnsHeader {
    pub id: u16,
    pub flags: u16,
    pub qdcount: u16,
    pub ancount: u16,
    pub nscount: u16,
    pub arcount: u16,
}

impl DnsHeader {
    pub const SIZE: usize = 12;

    pub fn new_query(id: u16, recursion_desired: bool) -> Self {
        let mut flags = 0u16;
        if recursion_desired {
            flags |= FLAG_RD_RECURSION_DESIRED;
        }
        Self {
            id,
            flags,
            qdcount: 1,
            ancount: 0,
            nscount: 0,
            arcount: 0,
        }
    }

    pub fn encode(&self, out: &mut [u8]) -> Result<usize, &'static str> {
        if out.len() < Self::SIZE {
            return Err("buffer too small for DNS header");
        }
        out[0..2].copy_from_slice(&self.id.to_be_bytes());
        out[2..4].copy_from_slice(&self.flags.to_be_bytes());
        out[4..6].copy_from_slice(&self.qdcount.to_be_bytes());
        out[6..8].copy_from_slice(&self.ancount.to_be_bytes());
        out[8..10].copy_from_slice(&self.nscount.to_be_bytes());
        out[10..12].copy_from_slice(&self.arcount.to_be_bytes());
        Ok(Self::SIZE)
    }

    pub fn decode(src: &[u8]) -> Result<Self, &'static str> {
        if src.len() < Self::SIZE {
            return Err("buffer too short for DNS header");
        }
        Ok(Self {
            id: u16::from_be_bytes([src[0], src[1]]),
            flags: u16::from_be_bytes([src[2], src[3]]),
            qdcount: u16::from_be_bytes([src[4], src[5]]),
            ancount: u16::from_be_bytes([src[6], src[7]]),
            nscount: u16::from_be_bytes([src[8], src[9]]),
            arcount: u16::from_be_bytes([src[10], src[11]]),
        })
    }

    pub fn is_response(&self) -> bool {
        (self.flags & FLAG_QR_RESPONSE) != 0
    }

    pub fn is_authenticated_data(&self) -> bool {
        (self.flags & FLAG_AD_AUTHENTICATED_DATA) != 0
    }

    pub fn rcode(&self) -> u8 {
        (self.flags & RCODE_MASK) as u8
    }
}

/// Zero-heap question view over a raw DNS packet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DnsQuestion<'a> {
    pub name: &'a [u8],
    pub qtype: DnsType,
    pub qclass: u16,
}

/// Encode a dotted ASCII domain name into DNS wire label format: `[len, label..., 0]`.
pub fn encode_domain_name(name: &str, out: &mut [u8]) -> Result<usize, &'static str> {
    let mut offset = 0;
    for label in name.split('.') {
        if label.is_empty() {
            continue;
        }
        let len = label.len();
        if len > 63 {
            return Err("DNS label exceeds 63 bytes");
        }
        if offset + 1 + len >= out.len() {
            return Err("DNS name buffer overflow");
        }
        out[offset] = len as u8;
        offset += 1;
        out[offset..offset + len].copy_from_slice(label.as_bytes());
        offset += len;
    }
    if offset >= out.len() {
        return Err("DNS name buffer overflow on null terminator");
    }
    out[offset] = 0;
    offset += 1;
    Ok(offset)
}

/// Decode a domain name from wire format into a plain dotted string inside `out_buf`.
pub fn decode_domain_name(
    packet: &[u8],
    mut offset: usize,
    out_buf: &mut [u8],
) -> Result<(usize, usize), &'static str> {
    let mut out_pos = 0;
    let mut jumps = 0usize;
    let mut next_offset = None;

    loop {
        if offset >= packet.len() {
            return Err("DNS name offset out of bounds");
        }
        let len = packet[offset];
        if len == 0 {
            if next_offset.is_none() {
                next_offset = Some(offset + 1);
            }
            break;
        }

        // Pointer compression: top 2 bits are 11 (0xC0)
        if (len & 0xC0) == 0xC0 {
            if offset + 1 >= packet.len() {
                return Err("DNS compression pointer truncated");
            }
            if next_offset.is_none() {
                next_offset = Some(offset + 2);
            }
            let pointer = (((len & 0x3F) as usize) << 8) | (packet[offset + 1] as usize);
            offset = pointer;
            jumps += 1;
            if jumps > 16 {
                return Err("DNS compression pointer loop detected");
            }
            continue;
        }

        offset += 1;
        let label_len = len as usize;
        if offset + label_len > packet.len() {
            return Err("DNS label length truncated");
        }
        if out_pos > 0 {
            if out_pos >= out_buf.len() {
                return Err("decoded name buffer overflow");
            }
            out_buf[out_pos] = b'.';
            out_pos += 1;
        }
        if out_pos + label_len > out_buf.len() {
            return Err("decoded name buffer overflow");
        }
        out_buf[out_pos..out_pos + label_len].copy_from_slice(&packet[offset..offset + label_len]);
        out_pos += label_len;
        offset += label_len;
    }

    let consumed = next_offset.unwrap_or(offset);
    Ok((consumed, out_pos))
}

/// A parsed Resource Record view pointing directly into packet bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DnsRecordView<'a> {
    pub rtype: DnsType,
    pub rclass: u16,
    pub ttl: u32,
    pub rdata: &'a [u8],
    pub rdata_offset: usize,
    pub packet: &'a [u8],
}

/// Parser for resource records out of a binary DNS packet.
pub fn parse_records<'a, F>(
    packet: &'a [u8],
    mut offset: usize,
    count: usize,
    mut callback: F,
) -> Result<usize, &'static str>
where
    F: FnMut(&str, DnsRecordView<'a>) -> Result<(), &'static str>,
{
    let mut name_buf = [0u8; 256];
    for _ in 0..count {
        if offset >= packet.len() {
            return Err("unexpected EOF in DNS records");
        }
        let (new_offset, name_len) = decode_domain_name(packet, offset, &mut name_buf)?;
        offset = new_offset;

        if offset + 10 > packet.len() {
            return Err("DNS record header truncated");
        }
        let rtype = DnsType::from(u16::from_be_bytes([packet[offset], packet[offset + 1]]));
        let rclass = u16::from_be_bytes([packet[offset + 2], packet[offset + 3]]);
        let ttl = u32::from_be_bytes([
            packet[offset + 4],
            packet[offset + 5],
            packet[offset + 6],
            packet[offset + 7],
        ]);
        let rdlength = u16::from_be_bytes([packet[offset + 8], packet[offset + 9]]) as usize;
        offset += 10;

        if offset + rdlength > packet.len() {
            return Err("DNS RDATA truncated");
        }
        let rdata_offset = offset;
        let rdata = &packet[offset..offset + rdlength];
        offset += rdlength;

        let name_str =
            std::str::from_utf8(&name_buf[..name_len]).map_err(|_| "invalid UTF-8 in DNS name")?;
        callback(
            name_str,
            DnsRecordView {
                rtype,
                rclass,
                ttl,
                rdata,
                rdata_offset,
                packet,
            },
        )?;
    }
    Ok(offset)
}

/// Write a single standard DNS query packet into `out`.
pub fn build_query_packet(
    id: u16,
    domain: &str,
    qtype: DnsType,
    recursion_desired: bool,
    out: &mut [u8],
) -> Result<usize, &'static str> {
    let header = DnsHeader::new_query(id, recursion_desired);
    let mut offset = header.encode(out)?;

    let name_len = encode_domain_name(domain, &mut out[offset..])?;
    offset += name_len;

    if offset + 4 > out.len() {
        return Err("buffer too small for DNS question type and class");
    }
    out[offset..offset + 2].copy_from_slice(&u16::from(qtype).to_be_bytes());
    out[offset + 2..offset + 4].copy_from_slice(&CLASS_IN.to_be_bytes());
    offset += 4;

    Ok(offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_roundtrip() {
        let h = DnsHeader::new_query(0x1234, true);
        let mut buf = [0u8; 12];
        let n = h.encode(&mut buf).unwrap();
        assert_eq!(n, 12);
        let decoded = DnsHeader::decode(&buf).unwrap();
        assert_eq!(decoded.id, 0x1234);
        assert_eq!(
            decoded.flags & FLAG_RD_RECURSION_DESIRED,
            FLAG_RD_RECURSION_DESIRED
        );
        assert_eq!(decoded.qdcount, 1);
    }

    #[test]
    fn domain_encode_decode() {
        let mut wire = [0u8; 64];
        let n = encode_domain_name("sub.domain.example.org", &mut wire).unwrap();
        assert!(n > 0);

        let mut decoded = [0u8; 64];
        let (consumed, len) = decode_domain_name(&wire, 0, &mut decoded).unwrap();
        assert_eq!(consumed, n);
        let s = std::str::from_utf8(&decoded[..len]).unwrap();
        assert_eq!(s, "sub.domain.example.org");
    }

    #[test]
    fn query_packet_builder() {
        let mut pkt = [0u8; 512];
        let len = build_query_packet(42, "example.com", DnsType::TXT, true, &mut pkt).unwrap();
        assert!(len > 12);

        let h = DnsHeader::decode(&pkt).unwrap();
        assert_eq!(h.id, 42);
        assert_eq!(h.qdcount, 1);
    }
}
