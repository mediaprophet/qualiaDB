//! Pure-Rust native DNS resolver client with zero heap in hot paths.
//!
//! Replaces 3rd-party dependencies (`trust-dns-resolver`, `hickory-proto`)
//! and external system commands (`dig`). Directly produces QualiaDB `NQuin`
//! records and feeds them into the Q42 zero-heap ring-buffer cache.
//!
//! Conforms to QualiaDB Rule 0-A (zero-heap in hot paths) and Rule 0-B.

#![allow(dead_code)]

use crate::{q_hash, NQuin, PermissiveRoutingLane};
use super::cache_ring::DnsCacheRing;
use super::quin_records::{decode_a_record, decode_aaaa_record, decode_cname_record, wire_record_view_to_quin};
use super::sdn::{parse_front_door_txt, SdnFrontDoorView};
use super::wire::{build_query_packet, parse_records, DnsHeader, DnsType};

/// Standard well-known recursive resolvers (IPv4).
pub const RESOLVER_CLOUDFLARE: &str = "1.1.1.1:53";
pub const RESOLVER_QUAD9: &str = "9.9.9.9:53";
pub const RESOLVER_GOOGLE: &str = "8.8.8.8:53";

/// Standard well-known recursive resolvers (IPv6).
pub const RESOLVER_CLOUDFLARE_V6: &str = "[2606:4700:4700::1111]:53";
pub const RESOLVER_QUAD9_V6: &str = "[2620:fe::fe]:53";
pub const RESOLVER_GOOGLE_V6: &str = "[2001:4860:4860::8888]:53";

/// Query configuration parameters.
#[derive(Debug, Clone, Copy)]
pub struct ResolverConfig<'a> {
    pub nameserver: &'a str,
    pub timeout_ms: u64,
    pub recursion_desired: bool,
    pub routing_lane: PermissiveRoutingLane,
}

impl<'a> Default for ResolverConfig<'a> {
    fn default() -> Self {
        Self {
            nameserver: RESOLVER_CLOUDFLARE,
            timeout_ms: 3000,
            recursion_desired: true,
            routing_lane: PermissiveRoutingLane::PassthroughStandard,
        }
    }
}

/// Helper to bind a UDP socket appropriate for IPv4 or IPv6 nameservers.
#[cfg(not(target_arch = "wasm32"))]
fn bind_socket_for_nameserver(nameserver: &str, timeout_ms: u64) -> Result<std::net::UdpSocket, &'static str> {
    use std::net::UdpSocket;
    use std::time::Duration;

    let is_v6 = nameserver.starts_with('[') || nameserver.matches(':').count() > 1;
    let socket = if is_v6 {
        UdpSocket::bind("[::]:0").or_else(|_| UdpSocket::bind("0.0.0.0:0"))
    } else {
        UdpSocket::bind("0.0.0.0:0")
    }.map_err(|_| "failed to bind local UDP socket")?;

    socket
        .set_read_timeout(Some(Duration::from_millis(timeout_ms)))
        .map_err(|_| "failed to set UDP read timeout")?;

    Ok(socket)
}

/// Zero-heap DNS resolver executing over raw UDP sockets on native platforms.
pub struct QualiaDnsResolver<'a> {
    pub config: ResolverConfig<'a>,
}

impl<'a> QualiaDnsResolver<'a> {
    pub fn new(config: ResolverConfig<'a>) -> Self {
        Self { config }
    }

    #[cfg(not(target_arch = "wasm32"))]
    /// Perform a raw UDP DNS query and parse the responses directly into `out_quins`.
    ///
    /// Returns the number of quins written into `out_quins`. Zero heap allocation.
    pub fn query_into(
        &self,
        domain: &str,
        qtype: DnsType,
        out_quins: &mut [NQuin],
    ) -> Result<usize, &'static str> {
        let socket = bind_socket_for_nameserver(self.config.nameserver, self.config.timeout_ms)?;

        let mut tx_buf = [0u8; 512];
        let query_id = (q_hash(domain) & 0xFFFF) as u16;
        let tx_len = build_query_packet(
            query_id,
            domain,
            qtype,
            self.config.recursion_desired,
            &mut tx_buf,
        )?;

        socket
            .send_to(&tx_buf[..tx_len], self.config.nameserver)
            .map_err(|_| "failed to send DNS query packet")?;

        let mut rx_buf = [0u8; 4096];
        let (rx_len, _) = socket
            .recv_from(&mut rx_buf)
            .map_err(|_| "DNS query timed out or failed to receive response")?;

        self.parse_response_into(&rx_buf[..rx_len], query_id, out_quins)
    }

    /// Parse a raw DNS response packet into caller-supplied `out_quins`.
    /// Zero heap allocation.
    pub fn parse_response_into(
        &self,
        packet: &[u8],
        expected_id: u16,
        out_quins: &mut [NQuin],
    ) -> Result<usize, &'static str> {
        let header = DnsHeader::decode(packet)?;
        if header.id != expected_id {
            return Err("DNS transaction ID mismatch");
        }
        if !header.is_response() {
            return Err("received DNS packet is not a response");
        }
        if header.rcode() != 0 {
            return Err("DNS server returned error rcode");
        }

        let mut offset = DnsHeader::SIZE;

        // Skip question section
        let mut dummy_name = [0u8; 256];
        for _ in 0..header.qdcount {
            let (new_offset, _) = super::wire::decode_domain_name(packet, offset, &mut dummy_name)?;
            offset = new_offset + 4; // skip qtype (2) and qclass (2)
            if offset > packet.len() {
                return Err("truncated question section");
            }
        }

        let mut quin_count = 0;
        let lane = self.config.routing_lane;

        // Parse answers with compression-aware view mapper
        parse_records(
            packet,
            offset,
            header.ancount as usize,
            |name, rec| {
                if quin_count < out_quins.len() {
                    if let Some(q) = wire_record_view_to_quin(name, &rec, lane) {
                        out_quins[quin_count] = q;
                        quin_count += 1;
                    }
                }
                Ok(())
            },
        )?;

        Ok(quin_count)
    }

    #[cfg(not(target_arch = "wasm32"))]
    /// Resolve an IPv4 `A` record directly.
    pub fn resolve_ipv4(&self, domain: &str) -> Result<[u8; 4], &'static str> {
        let mut quins = [NQuin::default(); 4];
        let count = self.query_into(domain, DnsType::A, &mut quins)?;
        for q in &quins[..count] {
            if let Some((ip, _, _)) = decode_a_record(q) {
                return Ok(ip);
            }
        }
        Err("no A record found")
    }

    #[cfg(not(target_arch = "wasm32"))]
    /// Resolve an IPv6 `AAAA` record directly.
    pub fn resolve_ipv6(&self, domain: &str) -> Result<[u8; 16], &'static str> {
        let mut quins = [NQuin::default(); 4];
        let count = self.query_into(domain, DnsType::AAAA, &mut quins)?;
        for q in &quins[..count] {
            if let Some((ip, _, _)) = decode_aaaa_record(q) {
                return Ok(ip);
            }
        }
        Err("no AAAA record found")
    }

    #[cfg(not(target_arch = "wasm32"))]
    /// Dual-stack IP resolution: queries AAAA first, falls back to A.
    pub fn resolve_ip(&self, domain: &str) -> Result<std::net::IpAddr, &'static str> {
        if let Ok(v6) = self.resolve_ipv6(domain) {
            return Ok(std::net::IpAddr::V6(std::net::Ipv6Addr::from(v6)));
        }
        let v4 = self.resolve_ipv4(domain)?;
        Ok(std::net::IpAddr::V4(std::net::Ipv4Addr::from(v4)))
    }

    #[cfg(not(target_arch = "wasm32"))]
    /// Resolve a `CNAME` record returning target hash and TTL.
    pub fn resolve_cname(&self, domain: &str) -> Result<(u64, u32), &'static str> {
        let mut quins = [NQuin::default(); 4];
        let count = self.query_into(domain, DnsType::CNAME, &mut quins)?;
        for q in &quins[..count] {
            if let Some((target_hash, ttl, _)) = decode_cname_record(q) {
                return Ok((target_hash, ttl));
            }
        }
        Err("no CNAME record found")
    }

    #[cfg(not(target_arch = "wasm32"))]
    /// Resolve and populate into a zero-heap cache ring buffer.
    pub fn query_and_cache<const N: usize>(
        &self,
        domain: &str,
        qtype: DnsType,
        ring: &mut DnsCacheRing<N>,
        current_clock: u32,
    ) -> Result<usize, &'static str> {
        let mut quins = [NQuin::default(); 8];
        let n = self.query_into(domain, qtype, &mut quins)?;
        for q in quins.iter().take(n) {
            ring.insert(*q, current_clock);
        }
        Ok(n)
    }

    #[cfg(not(target_arch = "wasm32"))]
    /// Resolve a `_qdp.<domain>` Front-Door record into a borrowed view.
    pub fn resolve_sdn_front_door<'b>(
        &self,
        domain: &'b str,
        txt_storage: &'b mut [u8; 512],
    ) -> Result<SdnFrontDoorView<'b>, &'static str> {
        let mut qdp_domain = [0u8; 128];
        const PREFIX: &[u8] = b"_qdp.";
        if PREFIX.len() + domain.len() > qdp_domain.len() {
            return Err("domain name too long for front-door query");
        }
        qdp_domain[..PREFIX.len()].copy_from_slice(PREFIX);
        qdp_domain[PREFIX.len()..PREFIX.len() + domain.len()].copy_from_slice(domain.as_bytes());
        let qdp_name = std::str::from_utf8(&qdp_domain[..PREFIX.len() + domain.len()])
            .map_err(|_| "invalid UTF-8 domain")?;

        let socket = bind_socket_for_nameserver(self.config.nameserver, self.config.timeout_ms)?;

        let mut tx_buf = [0u8; 512];
        let qid = 0x4242;
        let tx_len = build_query_packet(qid, qdp_name, DnsType::TXT, true, &mut tx_buf)?;
        socket
            .send_to(&tx_buf[..tx_len], self.config.nameserver)
            .map_err(|_| "send failed")?;

        let mut rx_buf = [0u8; 4096];
        let (rx_len, _) = socket.recv_from(&mut rx_buf).map_err(|_| "recv failed")?;

        let header = DnsHeader::decode(&rx_buf[..rx_len])?;
        if header.rcode() != 0 || header.ancount == 0 {
            return Err("front door TXT not found");
        }

        let mut offset = DnsHeader::SIZE;
        let mut dummy = [0u8; 256];
        for _ in 0..header.qdcount {
            let (new_off, _) = super::wire::decode_domain_name(&rx_buf[..rx_len], offset, &mut dummy)?;
            offset = new_off + 4;
        }

        let mut txt_len = 0;
        parse_records(
            &rx_buf[..rx_len],
            offset,
            header.ancount as usize,
            |_name, rec| {
                if rec.rtype == DnsType::TXT && txt_len == 0 {
                    let mut cursor = 0;
                    while cursor < rec.rdata.len() {
                        let seg_len = rec.rdata[cursor] as usize;
                        cursor += 1;
                        if cursor + seg_len > rec.rdata.len() {
                            break;
                        }
                        let cp = core::cmp::min(seg_len, txt_storage.len() - txt_len);
                        txt_storage[txt_len..txt_len + cp].copy_from_slice(&rec.rdata[cursor..cursor + cp]);
                        txt_len += cp;
                        cursor += seg_len;
                    }
                }
                Ok(())
            },
        )?;

        if txt_len == 0 {
            return Err("empty front-door TXT rdata");
        }

        let txt_str = std::str::from_utf8(&txt_storage[..txt_len]).map_err(|_| "invalid UTF-8 in TXT")?;
        parse_front_door_txt(domain, txt_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_response_synthetic_a() {
        // Build a query, then craft a synthetic IPv4 response
        let mut packet = [0u8; 512];
        let qlen = build_query_packet(0x1337, "test.local", DnsType::A, true, &mut packet).unwrap();

        let mut resp_header = DnsHeader::new_query(0x1337, true);
        resp_header.flags |= super::super::wire::FLAG_QR_RESPONSE | super::super::wire::FLAG_AA_AUTHORITATIVE;
        resp_header.ancount = 1;
        resp_header.encode(&mut packet[0..12]).unwrap();

        let mut offset = qlen;
        // Answer name: pointer to 12
        packet[offset] = 0xC0;
        packet[offset + 1] = 0x0C;
        offset += 2;
        // Type A (1), Class IN (1), TTL 300, RDLen 4, IP 127.0.0.1
        packet[offset..offset + 2].copy_from_slice(&1u16.to_be_bytes());
        packet[offset + 2..offset + 4].copy_from_slice(&1u16.to_be_bytes());
        packet[offset + 4..offset + 8].copy_from_slice(&300u32.to_be_bytes());
        packet[offset + 8..offset + 10].copy_from_slice(&4u16.to_be_bytes());
        packet[offset + 10..offset + 14].copy_from_slice(&[127, 0, 0, 1]);
        offset += 14;

        let resolver = QualiaDnsResolver::new(ResolverConfig::default());
        let mut quins = [NQuin::default(); 4];
        let n = resolver
            .parse_response_into(&packet[..offset], 0x1337, &mut quins)
            .unwrap();
        assert_eq!(n, 1);

        let (ip, ttl, _) = decode_a_record(&quins[0]).unwrap();
        assert_eq!(ip, [127, 0, 0, 1]);
        assert_eq!(ttl, 300);
    }

    #[test]
    fn parse_response_synthetic_aaaa() {
        // Build a query, then craft a synthetic IPv6 response
        let mut packet = [0u8; 512];
        let qlen = build_query_packet(0x2442, "v6.test.local", DnsType::AAAA, true, &mut packet).unwrap();

        let mut resp_header = DnsHeader::new_query(0x2442, true);
        resp_header.flags |= super::super::wire::FLAG_QR_RESPONSE | super::super::wire::FLAG_AA_AUTHORITATIVE;
        resp_header.ancount = 1;
        resp_header.encode(&mut packet[0..12]).unwrap();

        let mut offset = qlen;
        packet[offset] = 0xC0;
        packet[offset + 1] = 0x0C;
        offset += 2;
        // Type AAAA (28), Class IN (1), TTL 7200, RDLen 16
        packet[offset..offset + 2].copy_from_slice(&28u16.to_be_bytes());
        packet[offset + 2..offset + 4].copy_from_slice(&1u16.to_be_bytes());
        packet[offset + 4..offset + 8].copy_from_slice(&7200u32.to_be_bytes());
        packet[offset + 8..offset + 10].copy_from_slice(&16u16.to_be_bytes());
        let expected_ip: [u8; 16] = [
            0x20, 0x01, 0x0d, 0xb8, 0x85, 0xa3, 0x00, 0x00,
            0x00, 0x00, 0x8a, 0x2e, 0x03, 0x70, 0x73, 0x34,
        ];
        packet[offset + 10..offset + 26].copy_from_slice(&expected_ip);
        offset += 26;

        let resolver = QualiaDnsResolver::new(ResolverConfig::default());
        let mut quins = [NQuin::default(); 4];
        let n = resolver
            .parse_response_into(&packet[..offset], 0x2442, &mut quins)
            .unwrap();
        assert_eq!(n, 1);

        let (ip, ttl, _) = decode_aaaa_record(&quins[0]).unwrap();
        assert_eq!(ip, expected_ip);
        assert_eq!(ttl, 7200);
    }
}
