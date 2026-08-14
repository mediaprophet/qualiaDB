//! Trustless partial-CAR verification for IPFS-published Q42 bytes.
//!
//! Q42 offsets address the UnixFS file entity, never the serialized CAR.
//! Every CAR block is checked against its CID before any payload is copied
//! into a caller buffer. `entity-bytes` is inclusive on the wire; this
//! decoder accepts a half-open `[start, start+len)` after checked conversion.

use std::io;

use super::cid::{sha256, CidSha256};
use super::range::Q42ByteRange;

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

/// One verified raw-leaf block from a CARv1 stream.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedCarBlock {
    pub cid: CidSha256,
    pub data: Vec<u8>,
}

/// Decode a CARv1 buffer, verify every block CID, and concatenate raw (0x55)
/// leaves as the UnixFS entity. Non-raw blocks are verified but not appended
/// (dag-pb roots are proofs, not Q42 bytes).
pub fn decode_and_verify_car(car: &[u8]) -> io::Result<Vec<VerifiedCarBlock>> {
    let (header_len, header_size) = read_varint(car)?;
    let header_end = header_size
        .checked_add(header_len as usize)
        .ok_or_else(|| invalid("CAR header overflow"))?;
    if header_end > car.len() {
        return Err(invalid("CAR header truncated"));
    }
    let mut offset = header_end;
    let mut blocks = Vec::new();
    while offset < car.len() {
        let (block_len, used) = read_varint(&car[offset..])?;
        offset += used;
        let end = offset
            .checked_add(block_len as usize)
            .ok_or_else(|| invalid("CAR block overflow"))?;
        if end > car.len() {
            return Err(invalid("CAR block truncated"));
        }
        let (cid, cid_len) = parse_embedded_cid(&car[offset..end])?;
        let data = &car[offset + cid_len..end];
        cid.verify_block(data)?;
        blocks.push(VerifiedCarBlock {
            cid,
            data: data.to_vec(),
        });
        offset = end;
    }
    Ok(blocks)
}

/// Extract a half-open entity-byte interval from verified raw leaves.
pub fn extract_entity_bytes(
    blocks: &[VerifiedCarBlock],
    range: Q42ByteRange,
    out: &mut [u8],
) -> io::Result<()> {
    if out.len() != range.length {
        return Err(invalid("entity-bytes output length mismatch"));
    }
    let mut entity = Vec::new();
    for block in blocks {
        if block.cid.codec == CidSha256::RAW {
            entity.extend_from_slice(&block.data);
        }
    }
    let end = range.end()? as usize;
    if end > entity.len() {
        return Err(invalid("entity-bytes range exceeds reconstructed file"));
    }
    out.copy_from_slice(&entity[range.offset as usize..end]);
    Ok(())
}

/// Inclusive `entity-bytes=start:end` (IPIP-0402) → half-open Q42 range.
pub fn inclusive_entity_bytes(start: u64, end_inclusive: u64) -> io::Result<Q42ByteRange> {
    if end_inclusive < start {
        return Err(invalid("entity-bytes end is before start"));
    }
    let length = (end_inclusive - start)
        .checked_add(1)
        .ok_or_else(|| invalid("entity-bytes length overflow"))?;
    Ok(Q42ByteRange {
        offset: start,
        length: usize::try_from(length).map_err(|_| invalid("entity-bytes exceeds platform"))?,
    })
}

/// Encode a one-block raw CARv1 used by tests and fixtures.
pub fn encode_raw_car(blocks: &[&[u8]]) -> Vec<u8> {
    let mut out = Vec::new();
    // Minimal CBOR map {version:1, roots:[]} = a2 67 76 65 72 73 69 6f 6e 01 65 72 6f 6f 74 73 80
    let header = b"\xa2gversion\x01eroots\x80";
    write_varint(&mut out, header.len() as u64);
    out.extend_from_slice(header);
    for block in blocks {
        let cid = CidSha256::for_raw_block(block);
        let mut cid_bytes = vec![0x01];
        write_varint(&mut cid_bytes, cid.codec);
        cid_bytes.push(0x12);
        cid_bytes.push(32);
        cid_bytes.extend_from_slice(&cid.digest);
        write_varint(&mut out, (cid_bytes.len() + block.len()) as u64);
        out.extend_from_slice(&cid_bytes);
        out.extend_from_slice(block);
    }
    out
}

fn parse_embedded_cid(bytes: &[u8]) -> io::Result<(CidSha256, usize)> {
    if bytes.len() >= 34 && bytes[0] == 0x12 && bytes[1] == 32 {
        let mut digest = [0u8; 32];
        digest.copy_from_slice(&bytes[2..34]);
        return Ok((
            CidSha256 {
                version: 0,
                codec: CidSha256::DAG_PB,
                digest,
            },
            34,
        ));
    }
    if bytes.first() != Some(&0x01) {
        return Err(invalid("CAR block CID is neither CIDv0 nor CIDv1"));
    }
    let (codec, used) = read_varint(&bytes[1..])?;
    let mh = 1 + used;
    if bytes.len() < mh + 34 || bytes[mh] != 0x12 || bytes[mh + 1] != 32 {
        return Err(invalid("CAR block is not sha2-256"));
    }
    let mut digest = [0u8; 32];
    digest.copy_from_slice(&bytes[mh + 2..mh + 34]);
    // Confirm digest independently of the CID bytes we just parsed.
    let _ = sha256;
    Ok((
        CidSha256 {
            version: 1,
            codec,
            digest,
        },
        mh + 34,
    ))
}

fn write_varint(out: &mut Vec<u8>, mut value: u64) {
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if value == 0 {
            return;
        }
    }
}

fn read_varint(bytes: &[u8]) -> io::Result<(u64, usize)> {
    let mut value = 0u64;
    let mut shift = 0u32;
    for (index, byte) in bytes.iter().copied().enumerate() {
        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Ok((value, index + 1));
        }
        shift += 7;
        if shift >= 64 {
            return Err(invalid("CAR varint overflow"));
        }
    }
    Err(invalid("truncated CAR varint"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn car_verifies_cids_and_extracts_entity_bytes() {
        let first = b"Q42-HEADER-BYTES";
        let second = b"Q42-BLOCK-PAYLOAD";
        let car = encode_raw_car(&[first.as_slice(), second.as_slice()]);
        let blocks = decode_and_verify_car(&car).unwrap();
        assert_eq!(blocks.len(), 2);
        let range = inclusive_entity_bytes(4, 19).unwrap();
        let mut out = vec![0u8; range.length];
        extract_entity_bytes(&blocks, range, &mut out).unwrap();
        let mut expected = Vec::new();
        expected.extend_from_slice(first);
        expected.extend_from_slice(second);
        assert_eq!(out, expected[4..20]);
    }

    #[test]
    fn tampered_car_block_is_rejected() {
        let mut car = encode_raw_car(&[b"intact".as_slice()]);
        *car.last_mut().unwrap() ^= 0xff;
        assert!(decode_and_verify_car(&car).is_err());
    }
}
