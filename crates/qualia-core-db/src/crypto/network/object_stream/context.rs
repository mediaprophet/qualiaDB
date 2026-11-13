use super::codec::{Reader, Writer, check_ref, read_ref, write_ref};
use super::{Error, LEAF_BYTES, MAX_CONTEXT, MAX_METADATA, PageRef, SEGMENT_LEAVES};
use crate::crypto::network::digest::sha384;
use crate::net::qdnf::types::StrongDigest;
const MAGIC: &[u8; 12] = b"QBDL-PCX-V1\0";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContextFields {
    pub nonce: [u8; 32],
    pub scope_token: [u8; 32],
    pub generation: u64,
    pub key_epoch: u32,
    pub predecessor: StrongDigest,
    pub objects: u32,
    pub metadata_length: u64,
}
impl ContextFields {
    pub fn validate(&self) -> Result<(), Error> {
        super::types::nonzero(&self.nonce)?;
        super::types::nonzero(&self.scope_token)?;
        super::types::pages(self.objects as usize)?;
        if self.key_epoch == 0
            || (self.generation == 0) != self.predecessor.is_zero()
            || self.metadata_length == 0
            || self.metadata_length > MAX_METADATA
        {
            return Err(Error::Malformed);
        }
        Ok(())
    }
}
#[derive(Clone, Copy)]
pub struct Context<'a> {
    bytes: &'a [u8],
    fields: ContextFields,
    refs: &'a [u8],
}
impl<'a> Context<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, Error> {
        if bytes.len() > MAX_CONTEXT {
            return Err(Error::Capacity);
        }
        let mut r = Reader::new(bytes, MAGIC)?;
        r.suite()?;
        r.zero()?;
        let fields = ContextFields {
            nonce: r.array()?,
            scope_token: r.array()?,
            generation: r.u64()?,
            key_epoch: r.u32()?,
            predecessor: r.digest()?,
            objects: r.u32()?,
            metadata_length: r.u64()?,
        };
        fields.validate()?;
        if r.u32()? != LEAF_BYTES as u32 || r.u32()? != SEGMENT_LEAVES as u32 {
            return Err(Error::Downgrade);
        }
        let count = r.u16()? as usize;
        r.zero()?;
        if count != super::types::pages(fields.objects as usize)? {
            return Err(Error::Malformed);
        }
        let refs = r.take(count * 56)?;
        r.end()?;
        for (i, b) in refs.chunks_exact(56).enumerate() {
            check_ref(read_ref(b)?, fields.objects as usize, i, 24, 12)?;
        }
        Ok(Self {
            bytes,
            fields,
            refs,
        })
    }
    pub fn fields(&self) -> ContextFields {
        self.fields
    }
    pub fn encoded(&self) -> &'a [u8] {
        self.bytes
    }
    pub fn id(&self) -> StrongDigest {
        sha384(self.bytes)
    }
    pub fn page_count(&self) -> usize {
        self.refs.len() / 56
    }
    pub fn page_ref(&self, page: usize) -> Result<PageRef, Error> {
        if page >= self.page_count() {
            return Err(Error::Range);
        }
        read_ref(&self.refs[page * 56..(page + 1) * 56])
    }
}
pub fn encode_context(
    fields: ContextFields,
    refs: &[PageRef],
    out: &mut [u8],
) -> Result<usize, Error> {
    fields.validate()?;
    if refs.len() != super::types::pages(fields.objects as usize)? {
        return Err(Error::Malformed);
    }
    for (i, r) in refs.iter().enumerate() {
        check_ref(*r, fields.objects as usize, i, 24, 12)?;
    }
    let mut w = Writer::new(out, 164 + 56 * refs.len(), MAGIC)?;
    w.u16(1)?;
    w.u16(0)?;
    w.put(&fields.nonce)?;
    w.put(&fields.scope_token)?;
    w.u64(fields.generation)?;
    w.u32(fields.key_epoch)?;
    w.put(&fields.predecessor.0)?;
    w.u32(fields.objects)?;
    w.u64(fields.metadata_length)?;
    w.u32(LEAF_BYTES as u32)?;
    w.u32(SEGMENT_LEAVES as u32)?;
    w.u16(refs.len() as u16)?;
    w.u16(0)?;
    for r in refs {
        write_ref(&mut w, *r)?;
    }
    w.finish()
}
