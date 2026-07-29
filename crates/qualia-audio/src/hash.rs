//! FNV-1a 64 — same family as Qualia `q_hash` (vision semantic).

#[inline]
pub fn q_hash(s: &str) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x100_0000_01b3;
    let mut h = FNV_OFFSET;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

#[inline]
pub fn q_hash_bytes(bytes: &[u8]) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x100_0000_01b3;
    let mut h = FNV_OFFSET;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MediaDigest {
    pub hash: u64,
    pub byte_len: u64,
}

pub fn media_digest(bytes: &[u8]) -> MediaDigest {
    let take = bytes.len().min(65_536);
    let mut h = q_hash_bytes(&bytes[..take]);
    h ^= (bytes.len() as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
    MediaDigest {
        hash: h,
        byte_len: bytes.len() as u64,
    }
}
