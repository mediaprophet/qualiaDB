pub use crate::crypto::network::errors::CryptoError as Error;
use crate::net::qdnf::types::StrongDigest;
pub const PROFILE: &[u8] = b"qbdl-stream-protected-v1";
pub const PUBLISHER_PURPOSE: &[u8] = b"qbdl-stream-protected-v1/publisher";
pub const RECIPIENT_PURPOSE: &[u8] = b"qbdl-stream-protected-v1/recipient-authority";
pub const MAX_OBJECTS: usize = 8193;
pub const MAX_PAGES: usize = 65;
pub const PAGE_ROWS: usize = 128;
pub const MAX_METADATA: u64 = 8 * 1024 * 1024;
pub const MAX_OBJECT_BYTES: u64 = 1 << 40;
pub const LEAF_BYTES: u64 = 16384;
pub const SEGMENT_LEAVES: u64 = 1 << 20;
pub const MAX_SEGMENTS: u64 = 64;
pub const MAX_CONTEXT: usize = 3804;
pub const MAX_DESCRIPTOR: usize = 7512;
pub const RECIPIENT_BYTES: usize = 1856;
pub const WRAP_BYTES: usize = 1328;
pub const WRAP_CONTEXT_BYTES: usize = 1438;
pub const MAX_DELIVERY: usize = 7874;
pub const MAX_PRIVATE_PACKAGE: usize = 8192;
pub const RECORD_HEADER_BYTES: usize = 88;
pub const SEGMENT_INFO_BYTES: usize = 116;
pub const LEAF_AAD_BYTES: usize = 126;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PageRef {
    pub page: u16,
    pub rows: u16,
    pub length: u32,
    pub digest: StrongDigest,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecipientKeyIds {
    pub ml_kem_768: StrongDigest,
    pub x25519: StrongDigest,
}
pub(crate) fn pages(n: usize) -> Result<usize, Error> {
    if n > MAX_OBJECTS {
        Err(Error::Capacity)
    } else {
        Ok(n.div_ceil(PAGE_ROWS))
    }
}
pub(crate) fn rows(n: usize, page: usize) -> Result<usize, Error> {
    if page >= pages(n)? {
        Err(Error::Range)
    } else {
        Ok((n - page * PAGE_ROWS).min(PAGE_ROWS))
    }
}
pub(crate) fn nonzero(bytes: &[u8]) -> Result<(), Error> {
    if bytes.iter().all(|b| *b == 0) {
        Err(Error::Malformed)
    } else {
        Ok(())
    }
}
