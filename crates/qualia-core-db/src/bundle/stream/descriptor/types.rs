use crate::net::qdnf::types::StrongDigest;
pub const PROFILE: &str = "qbdl-stream-pq-1";
pub const MAX_ENCODED: usize = 65_536;
pub const MAX_METADATA: usize = 8 * 1024 * 1024;
pub const MAX_ENTRIES: usize = 4096;
pub const PAGE_ROWS: usize = 128;
pub const MAX_PAGES: usize = 32;
pub const LEAF_BYTES: u64 = crate::bundle::stream::proof::LEAF_BYTES;
const _: () = assert!(LEAF_BYTES == 16_384);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DescriptorError {
    Capacity,
    Limit,
    Encoding,
    Field,
    Length,
    Unsupported,
    Overflow,
    Page,
    Coverage,
    DuplicateId,
    Commitment,
    Context,
    Incomplete,
    Poisoned,
    Sink,
    Source,
    Proof,
}
impl core::fmt::Display for DescriptorError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "descriptor {:?}", self)
    }
}
impl core::error::Error for DescriptorError {}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCommit {
    pub length: u64,
    pub version: u16,
    pub flags: u16,
    pub header_digest: StrongDigest,
    pub index_offset: u64,
    pub index_length: u64,
    pub index_digest: StrongDigest,
    pub source_digest: StrongDigest,
}
impl SourceCommit {
    pub(crate) fn validate(&self) -> Result<(), DescriptorError> {
        if self.version != 1 || self.flags > 1 {
            return Err(DescriptorError::Unsupported);
        }
        if self.length < 64
            || self.length > (1 << 40)
            || self.index_offset < 64
            || self.index_offset.checked_add(self.index_length) != Some(self.length)
        {
            return Err(DescriptorError::Source);
        }
        if self.index_offset % if self.flags == 1 { 4096 } else { 64 } != 0 {
            return Err(DescriptorError::Source);
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContextParameters {
    pub nonce: [u8; 32],
    pub generation: u64,
    pub predecessor: Option<StrongDigest>,
    pub semantic_contract: StrongDigest,
    pub policy: StrongDigest,
    pub consumer_profile: StrongDigest,
}
impl ContextParameters {
    pub(crate) fn validate(&self) -> Result<(), DescriptorError> {
        if self.nonce == [0; 32]
            || (self.generation == 0) != self.predecessor.is_none()
            || self.semantic_contract.is_zero()
            || self.policy.is_zero()
            || self.consumer_profile.is_zero()
        {
            return Err(DescriptorError::Context);
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PageRef {
    pub number: u32,
    pub first: u32,
    pub rows: u32,
    pub encoded_length: u32,
    pub digest: StrongDigest,
}
impl PageRef {
    pub const EMPTY: Self = Self {
        number: 0,
        first: 0,
        rows: 0,
        encoded_length: 0,
        digest: StrongDigest::ZERO,
    };
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContextEntry {
    pub id: u32,
    pub length: u64,
}
impl ContextEntry {
    pub const EMPTY: Self = Self { id: 0, length: 0 };
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EntryRecord<'a> {
    pub id: u32,
    pub ordinal: u32,
    pub kind: &'a str,
    pub inner_version: u64,
    pub offset: u64,
    pub length: u64,
    pub root: StrongDigest,
}
impl EntryRecord<'_> {
    pub const EMPTY: Self = Self {
        id: 0,
        ordinal: 0,
        kind: "",
        inner_version: 0,
        offset: 0,
        length: 0,
        root: StrongDigest::ZERO,
    };
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContextPageHeader {
    pub number: u32,
    pub first: u32,
    pub rows: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EntryPageHeader {
    pub context: StrongDigest,
    pub number: u32,
    pub first: u32,
    pub rows: usize,
}
pub(crate) fn pages_for(entries: usize) -> Result<usize, DescriptorError> {
    if entries > MAX_ENTRIES {
        return Err(DescriptorError::Limit);
    }
    Ok(entries.div_ceil(PAGE_ROWS))
}
pub(crate) fn expected_rows(entries: usize, page: usize) -> Result<usize, DescriptorError> {
    if page >= pages_for(entries)? {
        return Err(DescriptorError::Page);
    }
    Ok((entries - page * PAGE_ROWS).min(PAGE_ROWS))
}
pub(crate) fn check_ref(r: &PageRef, entries: usize, index: usize) -> Result<(), DescriptorError> {
    if r.number as usize != index
        || r.first as usize != index * PAGE_ROWS
        || r.rows as usize != expected_rows(entries, index)?
        || r.encoded_length == 0
        || r.encoded_length as usize > MAX_ENCODED
    {
        return Err(DescriptorError::Coverage);
    }
    Ok(())
}
/// Complete local structural candidate; not trusted authority or verified whole-source download.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CandidateIdentity {
    pub(crate) descriptor: StrongDigest,
    pub(crate) context: StrongDigest,
    pub(crate) generation: u64,
    pub(crate) entries: u32,
}
impl CandidateIdentity {
    pub const fn descriptor_id(&self) -> StrongDigest {
        self.descriptor
    }
    pub const fn context_id(&self) -> StrongDigest {
        self.context
    }
    pub const fn generation(&self) -> u64 {
        self.generation
    }
    pub const fn entry_count(&self) -> u32 {
        self.entries
    }
    pub fn binding<'a>(
        &'a self,
        key_ids: crate::crypto::network::cose::KeyIds,
        controller_epoch: u64,
        purpose: &'a [u8],
        audience: &'a [u8],
    ) -> crate::crypto::network::cose::Binding<'a> {
        crate::crypto::network::cose::Binding {
            descriptor_digest: &self.descriptor.0,
            key_ids,
            controller_epoch,
            purpose,
            audience,
        }
    }
}
