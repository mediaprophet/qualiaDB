//! Zero-Heap Lossless DNS Record Chunker & Codec.
//!
//! Provides 100% byte-exact (lossless) encoding and decoding for arbitrary-length
//! DNS resource records (TXT, CERT, DNSKEY, RRSIG, HTTPS, SOA, etc.) across
//! deterministic 48-byte `NQuin` chunk chains.
//!
//! Each chunk stores up to 16 raw payload bytes in `object` and `context`,
//! indexed by sequence numbers in `metadata`, protected by ECC parity.
//!
//! Conforms strictly to QualiaDB Rule 0-A (zero-heap in hot paths) and Rule 0-B (< 500 lines).

#![allow(dead_code)]

use super::wire::DnsType;
use crate::{q_hash, NQuin, PermissiveRoutingLane};

/// Modality opcode for lossless payload chunk in predicate bits [0..7].
pub const OP_DNS_CHUNK: u8 = 0x62;

/// Maximum payload bytes stored per Super-Quin chunk.
pub const CHUNK_CAPACITY: usize = 16;

/// Encode an arbitrary byte slice of DNS RDATA into contiguous, lossless `NQuin` chunks.
///
/// Returns the number of quins written into `out_chunk_quins`. Zero heap allocation.
pub fn encode_lossless_rdata_chunks(
    domain: &str,
    rtype: DnsType,
    ttl: u32,
    rdata: &[u8],
    lane: PermissiveRoutingLane,
    out_chunk_quins: &mut [NQuin],
) -> Result<usize, &'static str> {
    if rdata.is_empty() {
        return Ok(0);
    }

    let total_chunks = (rdata.len() + CHUNK_CAPACITY - 1) / CHUNK_CAPACITY;
    if total_chunks > out_chunk_quins.len() {
        return Err("output quins buffer too small for lossless rdata chunks");
    }
    if total_chunks > 0xFFFF {
        return Err("rdata exceeds maximum chunkable size (1MB)");
    }

    let subject = q_hash(domain);
    let pred_base = q_hash("qualia:dns:lossless_chunk") ^ (u16::from(rtype) as u64);
    let predicate = (pred_base << 8) | (OP_DNS_CHUNK as u64);

    for (chunk_idx, chunk_bytes) in rdata.chunks(CHUNK_CAPACITY).enumerate() {
        let mut hi_buf = [0u8; 8];
        let mut lo_buf = [0u8; 8];

        let hi_len = core::cmp::min(chunk_bytes.len(), 8);
        hi_buf[..hi_len].copy_from_slice(&chunk_bytes[..hi_len]);

        if chunk_bytes.len() > 8 {
            let lo_len = chunk_bytes.len() - 8;
            lo_buf[..lo_len].copy_from_slice(&chunk_bytes[8..8 + lo_len]);
        }

        let object = u64::from_be_bytes(hi_buf);
        let context = u64::from_be_bytes(lo_buf);

        // Metadata layout:
        // [61..62]: PermissiveRoutingLane
        // [32..60]: chunk_idx (29 bits)
        // [16..31]: total_chunks (16 bits)
        // [0..15]:  valid bytes in this chunk (1..16) + ttl fold
        let valid_bytes = chunk_bytes.len() as u64;
        let metadata = ((lane as u64) << 61)
            | (((chunk_idx as u64) & 0x1FFF_FFFF) << 32)
            | (((total_chunks as u64) & 0xFFFF) << 16)
            | (valid_bytes & 0xFF)
            | (((ttl as u64) & 0xFF) << 8);

        let parity = NQuin::calculate_parity(subject, predicate, object, context, metadata);
        out_chunk_quins[chunk_idx] = NQuin {
            subject,
            predicate,
            object,
            context,
            metadata,
            parity,
        };
    }

    Ok(total_chunks)
}

/// Reassemble an arbitrary-length DNS RDATA payload from an `NQuin` chunk slice.
///
/// Returns the total bytes written into `out_rdata`. Zero heap allocation.
pub fn decode_lossless_rdata_chunks(
    chunk_quins: &[NQuin],
    expected_rtype: DnsType,
    out_rdata: &mut [u8],
) -> Result<usize, &'static str> {
    if chunk_quins.is_empty() {
        return Ok(0);
    }

    let expected_pred_base =
        q_hash("qualia:dns:lossless_chunk") ^ (u16::from(expected_rtype) as u64);
    let expected_predicate = (expected_pred_base << 8) | (OP_DNS_CHUNK as u64);

    let first = &chunk_quins[0];
    if !first.verify_ecc_parity() || first.predicate != expected_predicate {
        return Err("invalid chunk parity or rtype mismatch on first chunk");
    }

    let expected_total_chunks = ((first.metadata >> 16) & 0xFFFF) as usize;
    if chunk_quins.len() < expected_total_chunks {
        return Err("missing chunk quins in slice");
    }

    let mut written = 0;

    for (i, quin) in chunk_quins.iter().take(expected_total_chunks).enumerate() {
        if !quin.verify_ecc_parity() {
            return Err("chunk ECC parity verification failed");
        }
        if quin.predicate != expected_predicate {
            return Err("chunk predicate mismatch");
        }

        let chunk_idx = ((quin.metadata >> 32) & 0x1FFF_FFFF) as usize;
        if chunk_idx != i {
            return Err("out-of-order chunk detected");
        }

        let valid_bytes = (quin.metadata & 0xFF) as usize;
        if valid_bytes > CHUNK_CAPACITY || valid_bytes == 0 {
            return Err("invalid chunk payload byte length");
        }
        if written + valid_bytes > out_rdata.len() {
            return Err("output buffer overflow during rdata reassembly");
        }

        let hi_bytes = quin.object.to_be_bytes();
        let lo_bytes = quin.context.to_be_bytes();

        let hi_take = core::cmp::min(valid_bytes, 8);
        out_rdata[written..written + hi_take].copy_from_slice(&hi_bytes[..hi_take]);
        written += hi_take;

        if valid_bytes > 8 {
            let lo_take = valid_bytes - 8;
            out_rdata[written..written + lo_take].copy_from_slice(&lo_bytes[..lo_take]);
            written += lo_take;
        }
    }

    Ok(written)
}

/// Lossless structured record enum providing zero-copy access to decoded fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LosslessDnsRecord<'a> {
    A([u8; 4]),
    Aaaa([u8; 16]),
    Cname(&'a str),
    Ns(&'a str),
    Ptr(&'a str),
    Mx {
        preference: u16,
        exchange: &'a str,
    },
    Srv {
        priority: u16,
        weight: u16,
        port: u16,
        target: &'a str,
    },
    Txt(&'a [u8]),
    Caa {
        flags: u8,
        tag: &'a str,
        value: &'a [u8],
    },
    Tlsa {
        usage: u8,
        selector: u8,
        matching_type: u8,
        cert_data: &'a [u8],
    },
    Https {
        priority: u16,
        target: &'a str,
        params: &'a [u8],
    },
    Dnskey {
        flags: u16,
        protocol: u8,
        algorithm: u8,
        public_key: &'a [u8],
    },
    Ds {
        key_tag: u16,
        algorithm: u8,
        digest_type: u8,
        digest: &'a [u8],
    },
    Raw {
        rtype: DnsType,
        rdata: &'a [u8],
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lossless_chunking_roundtrip_multi_chunk() {
        // Test with a 38-byte payload (spans 3 chunks: 16 + 16 + 6)
        let sample_payload = b"qualialabs-lossless-dnskey-payload-12345678";
        assert_eq!(sample_payload.len(), 43);

        let mut quins = [NQuin::default(); 8];
        let num_chunks = encode_lossless_rdata_chunks(
            "node.qualia.net",
            DnsType::DNSKEY,
            3600,
            sample_payload,
            PermissiveRoutingLane::EnforceBilateralMicroCommons,
            &mut quins,
        )
        .unwrap();

        assert_eq!(num_chunks, 3);
        for q in &quins[..num_chunks] {
            assert!(q.verify_ecc_parity());
            assert_eq!(
                q.identify_routing_lane(),
                PermissiveRoutingLane::EnforceBilateralMicroCommons
            );
        }

        let mut reassembled = [0u8; 128];
        let bytes_out =
            decode_lossless_rdata_chunks(&quins[..num_chunks], DnsType::DNSKEY, &mut reassembled)
                .unwrap();

        assert_eq!(bytes_out, sample_payload.len());
        assert_eq!(&reassembled[..bytes_out], sample_payload);
    }

    #[test]
    fn lossless_chunking_exact_16_bytes() {
        // Test boundary condition: exact multiple of CHUNK_CAPACITY (16 bytes)
        let sample_payload = [0x42u8; 16];
        let mut quins = [NQuin::default(); 2];
        let num_chunks = encode_lossless_rdata_chunks(
            "exact16.qualia.net",
            DnsType::TXT,
            300,
            &sample_payload,
            PermissiveRoutingLane::PassthroughStandard,
            &mut quins,
        )
        .unwrap();

        assert_eq!(num_chunks, 1);
        let mut out = [0u8; 32];
        let n = decode_lossless_rdata_chunks(&quins[..num_chunks], DnsType::TXT, &mut out).unwrap();
        assert_eq!(n, 16);
        assert_eq!(&out[..16], &sample_payload);
    }

    #[test]
    fn lossless_chunking_parity_tamper_detected() {
        let sample = b"unaltered-data";
        let mut quins = [NQuin::default(); 2];
        let n = encode_lossless_rdata_chunks(
            "tamper.test",
            DnsType::TXT,
            300,
            sample,
            PermissiveRoutingLane::PassthroughStandard,
            &mut quins,
        )
        .unwrap();

        // Tamper with object byte
        quins[0].object ^= 0x01;
        let mut out = [0u8; 32];
        assert!(decode_lossless_rdata_chunks(&quins[..n], DnsType::TXT, &mut out).is_err());
    }
}
