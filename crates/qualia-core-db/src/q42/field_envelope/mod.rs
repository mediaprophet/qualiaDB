//! Common current-format EMF/QAC storage envelope, with no scientific admission.
//!
//! Successful parsing checks bounded byte structure and index integrity only.
//! Unknown modes, schemas, revisions, critical fields and references are exposed
//! uninterpreted. Family adapters MUST admit these, resolve reference graphs,
//! check shapes/units and verify requested whole chunks before scientific use.
//! This module does not authenticate publishers or implement range proofs.

mod metadata;
mod metadata_writer;
mod reader;
mod writer;
pub use metadata::{Block, Blocks, Field, Fields};
pub use metadata_writer::{BlockInput, FieldInput, FieldValue, MetadataInput};
pub use reader::{Entry, StructuralView};
pub use writer::{EntryInput, EnvelopeInput};

pub const HEADER_BYTES: usize = 128;
pub const DIRECTORY_BYTES: usize = 128;
pub const INDEX_DOMAIN: &[u8] = b"QUALIA-FIELD-INDEX-v1\0";

/// Current structural family selectors. Variant names retain their development
/// spellings until a coordinated API rename; they do not define dispatch defaults.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    QacV1Candidate,
    EmfV4Candidate,
}
impl Family {
    pub fn magic(self) -> &'static [u8; 8] {
        match self {
            Self::QacV1Candidate => b"qac\0\0\0\0\0",
            Self::EmfV4Candidate => b"Q42EMF64",
        }
    }
    pub fn version(self) -> u16 {
        match self {
            Self::QacV1Candidate => 1,
            Self::EmfV4Candidate => 4,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub file_bytes: u64,
    pub metadata_bytes: u64,
    pub entries: u32,
    pub blocks: u32,
    pub fields_per_block: u32,
    pub string_bytes: u32,
    pub chunk_bytes: u64,
    pub aggregate_decoded_bytes: u64,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            file_bytes: 256 << 20,
            metadata_bytes: 4 << 20,
            entries: 4096,
            blocks: 4096,
            fields_per_block: 128,
            string_bytes: 16 << 10,
            chunk_bytes: 16 << 20,
            aggregate_decoded_bytes: 256 << 20,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Extent,
    Overflow,
    Limit,
    Family,
    Version,
    Header,
    Crc,
    IndexDigest,
    Directory,
    Order,
    Padding,
    Codec,
    Metadata,
    Numeric,
    PayloadDigest,
    EntryIndex,
}
pub(crate) fn u16le(b: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([b[o], b[o + 1]])
}
pub(crate) fn u32le(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}
pub(crate) fn u64le(b: &[u8], o: usize) -> u64 {
    u64::from_le_bytes(b[o..o + 8].try_into().unwrap())
}
pub(crate) fn align(n: u64, a: u64) -> Result<u64, Error> {
    Ok(n.checked_add(a - 1).ok_or(Error::Overflow)? & !(a - 1))
}
pub(crate) fn extent(b: &[u8], start: u64, len: u64) -> Result<&[u8], Error> {
    let end = start.checked_add(len).ok_or(Error::Overflow)?;
    let s = usize::try_from(start).map_err(|_| Error::Extent)?;
    let e = usize::try_from(end).map_err(|_| Error::Extent)?;
    b.get(s..e).ok_or(Error::Extent)
}
pub(crate) fn zero(b: &[u8]) -> Result<(), Error> {
    if b.iter().all(|&v| v == 0) {
        Ok(())
    } else {
        Err(Error::Padding)
    }
}
pub(crate) fn crc(b: &[u8]) -> Result<(), Error> {
    let mut bytes = [0; 128];
    bytes.copy_from_slice(b);
    bytes[124..].fill(0);
    if crate::container_10d::crc32c::crc32c(&bytes) == u32le(b, 124) {
        Ok(())
    } else {
        Err(Error::Crc)
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod writer_tests;

#[cfg(test)]
mod metadata_writer_tests;

#[cfg(test)]
mod adversarial_tests;
