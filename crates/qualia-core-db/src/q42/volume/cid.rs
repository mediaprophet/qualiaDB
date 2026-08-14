//! CID parse and block verification for IPFS-published Q42 segments.
//!
//! A gateway is only a transport. The CID's multihash is the authority: a
//! decoded block is accepted only when `sha2-256(block) == CID digest`.
//! CIDv0 (`Qm…`) and CIDv1 sha2-256 (`bafy…` / `bafk…` / raw hex) are supported.

use std::io;

use sha2::{Digest, Sha256};

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

/// A verified content identifier. Only sha2-256 (32-byte) digests are accepted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CidSha256 {
    pub version: u8,
    pub codec: u64,
    pub digest: [u8; 32],
}

impl CidSha256 {
    pub const RAW: u64 = 0x55;
    pub const DAG_PB: u64 = 0x70;

    pub fn parse(text: &str) -> io::Result<Self> {
        let text = text.trim();
        if text.is_empty() {
            return Err(invalid("empty CID"));
        }
        if text.starts_with("Qm") {
            return parse_cidv0(text);
        }
        if let Some(hex) = text.strip_prefix("f") {
            return parse_cidv1(&decode_hex(hex)?);
        }
        if let Some(b32) = text.strip_prefix("b") {
            return parse_cidv1(&decode_base32(b32)?);
        }
        Err(invalid(
            "CID must be CIDv0 (Qm…) or CIDv1 base32 (b…) / hex (f…)",
        ))
    }

    pub fn for_raw_block(block: &[u8]) -> Self {
        Self {
            version: 1,
            codec: Self::RAW,
            digest: sha256(block),
        }
    }

    pub fn verify_block(&self, block: &[u8]) -> io::Result<()> {
        let actual = sha256(block);
        if actual != self.digest {
            return Err(invalid("block bytes do not match CID sha2-256 digest"));
        }
        Ok(())
    }

    pub fn encode_base32(&self) -> String {
        let mut raw = Vec::new();
        raw.push(self.version);
        encode_varint(&mut raw, self.codec);
        raw.push(0x12);
        raw.push(32);
        raw.extend_from_slice(&self.digest);
        let mut out = String::from("b");
        out.push_str(&encode_base32(&raw));
        out
    }
}

fn parse_cidv0(text: &str) -> io::Result<CidSha256> {
    let bytes = decode_base58btc(text)?;
    if bytes.len() != 34 || bytes[0] != 0x12 || bytes[1] != 32 {
        return Err(invalid("CIDv0 must be sha2-256 (34 bytes)"));
    }
    let mut digest = [0u8; 32];
    digest.copy_from_slice(&bytes[2..]);
    Ok(CidSha256 {
        version: 0,
        codec: CidSha256::DAG_PB,
        digest,
    })
}

fn parse_cidv1(bytes: &[u8]) -> io::Result<CidSha256> {
    if bytes.first() != Some(&0x01) {
        return Err(invalid("CIDv1 version byte must be 0x01"));
    }
    let (codec, used) = decode_varint(&bytes[1..])?;
    let rest = &bytes[1 + used..];
    if rest.len() != 34 || rest[0] != 0x12 || rest[1] != 32 {
        return Err(invalid("only sha2-256 CIDv1 is accepted"));
    }
    let mut digest = [0u8; 32];
    digest.copy_from_slice(&rest[2..]);
    Ok(CidSha256 {
        version: 1,
        codec,
        digest,
    })
}

pub fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn encode_varint(out: &mut Vec<u8>, mut value: u64) {
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

fn decode_varint(bytes: &[u8]) -> io::Result<(u64, usize)> {
    let mut value = 0u64;
    let mut shift = 0u32;
    for (index, byte) in bytes.iter().copied().enumerate() {
        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Ok((value, index + 1));
        }
        shift += 7;
        if shift >= 64 {
            return Err(invalid("CID varint overflow"));
        }
    }
    Err(invalid("truncated CID varint"))
}

fn decode_hex(text: &str) -> io::Result<Vec<u8>> {
    if text.len() % 2 != 0 {
        return Err(invalid("odd-length hex CID"));
    }
    let mut out = Vec::with_capacity(text.len() / 2);
    let bytes = text.as_bytes();
    for pair in bytes.chunks(2) {
        let hi = hex_nibble(pair[0])?;
        let lo = hex_nibble(pair[1])?;
        out.push((hi << 4) | lo);
    }
    Ok(out)
}

fn hex_nibble(byte: u8) -> io::Result<u8> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(invalid("invalid hex digit in CID")),
    }
}

fn decode_base32(text: &str) -> io::Result<Vec<u8>> {
    let mut bits: u32 = 0;
    let mut nbits = 0u32;
    let mut out = Vec::new();
    for byte in text.bytes() {
        if byte == b'=' {
            continue;
        }
        let value = match byte {
            b'a'..=b'z' => byte - b'a',
            b'A'..=b'Z' => byte - b'A',
            b'2'..=b'7' => byte - b'2' + 26,
            _ => return Err(invalid("invalid base32 digit in CID")),
        };
        bits = (bits << 5) | u32::from(value);
        nbits += 5;
        if nbits >= 8 {
            nbits -= 8;
            out.push((bits >> nbits) as u8);
            bits &= (1 << nbits) - 1;
        }
    }
    Ok(out)
}

fn encode_base32(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz234567";
    let mut bits: u32 = 0;
    let mut nbits = 0u32;
    let mut out = String::new();
    for byte in bytes {
        bits = (bits << 8) | u32::from(*byte);
        nbits += 8;
        while nbits >= 5 {
            nbits -= 5;
            out.push(ALPHABET[((bits >> nbits) & 0x1f) as usize] as char);
            bits &= (1 << nbits) - 1;
        }
    }
    if nbits > 0 {
        out.push(ALPHABET[((bits << (5 - nbits)) & 0x1f) as usize] as char);
    }
    out
}

fn decode_base58btc(text: &str) -> io::Result<Vec<u8>> {
    const ALPHABET: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    let mut acc = vec![0u8; 1];
    for byte in text.bytes() {
        let digit = ALPHABET
            .iter()
            .position(|candidate| *candidate == byte)
            .ok_or_else(|| invalid("invalid base58 digit in CID"))?;
        let mut carry = digit;
        for slot in acc.iter_mut().rev() {
            let value = *slot as usize * 58 + carry;
            *slot = (value & 0xff) as u8;
            carry = value >> 8;
        }
        while carry > 0 {
            acc.insert(0, (carry & 0xff) as u8);
            carry >>= 8;
        }
    }
    let leading = text.bytes().take_while(|b| *b == b'1').count();
    let mut out = vec![0u8; leading];
    let skip = acc.iter().position(|b| *b != 0).unwrap_or(acc.len());
    out.extend_from_slice(&acc[skip..]);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_block_round_trip_and_tamper_is_detected() {
        let block = b"q42-superblock-bytes";
        let cid = CidSha256::for_raw_block(block);
        let encoded = cid.encode_base32();
        assert!(encoded.starts_with('b'));
        let parsed = CidSha256::parse(&encoded).unwrap();
        parsed.verify_block(block).unwrap();
        assert!(parsed.verify_block(b"tampered").is_err());
    }

    #[test]
    fn hex_cidv1_parses() {
        let block = b"hello-q42";
        let digest = sha256(block);
        let mut raw = vec![0x01, 0x55, 0x12, 32];
        raw.extend_from_slice(&digest);
        let mut hex = String::from("f");
        for byte in &raw {
            hex.push_str(&format!("{byte:02x}"));
        }
        let cid = CidSha256::parse(&hex).unwrap();
        assert_eq!(cid.codec, CidSha256::RAW);
        cid.verify_block(block).unwrap();
    }
}
