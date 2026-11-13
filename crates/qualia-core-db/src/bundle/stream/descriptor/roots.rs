use super::cbor::{R, W, encode};
use super::refs::{PageRefsView, read_refs, write_refs};
use super::{
    ContextParameters, DescriptorError as E, LEAF_BYTES, MAX_ENTRIES, PageRef, SourceCommit,
};
use crate::net::qdnf::types::StrongDigest;
pub struct ContextInput<'a> {
    pub parameters: ContextParameters,
    pub source: SourceCommit,
    pub entries: usize,
    pub pages: &'a [PageRef],
}
#[derive(Clone, Copy)]
pub struct ContextView<'a> {
    pub(crate) encoded: &'a [u8],
    pub(crate) parameters: ContextParameters,
    pub(crate) source: SourceCommit,
    pub(crate) entries: usize,
    pub(crate) pages: PageRefsView<'a>,
}
impl<'a> ContextView<'a> {
    pub const fn encoded_bytes(self) -> &'a [u8] {
        self.encoded
    }
    pub const fn parameters(self) -> ContextParameters {
        self.parameters
    }
    pub const fn source(self) -> SourceCommit {
        self.source
    }
    pub const fn entry_count(self) -> usize {
        self.entries
    }
    pub const fn page_refs(self) -> PageRefsView<'a> {
        self.pages
    }
}
pub struct BodyInput<'a> {
    pub context_id: StrongDigest,
    pub context_root: &'a [u8],
    pub generation: u64,
    pub predecessor: Option<StrongDigest>,
    pub entries: usize,
    pub pages: &'a [PageRef],
}
#[derive(Clone, Copy)]
pub struct BodyView<'a> {
    pub(crate) encoded: &'a [u8],
    pub(crate) context_id: StrongDigest,
    pub(crate) context: ContextView<'a>,
    pub(crate) pages: PageRefsView<'a>,
}
impl<'a> BodyView<'a> {
    pub const fn encoded_bytes(self) -> &'a [u8] {
        self.encoded
    }
    pub const fn context_id(self) -> StrongDigest {
        self.context_id
    }
    pub const fn context(self) -> ContextView<'a> {
        self.context
    }
    pub const fn page_refs(self) -> PageRefsView<'a> {
        self.pages
    }
}
fn optional(w: &mut W<'_>, value: Option<StrongDigest>) -> Result<(), E> {
    match value {
        Some(d) => w.bytes(d.as_bytes()),
        None => w.null(),
    }
}
fn source_write(w: &mut W<'_>, s: SourceCommit) -> Result<(), E> {
    s.validate()?;
    w.map(8)?;
    w.uint(0)?;
    w.uint(s.length)?;
    w.uint(1)?;
    w.uint(s.version as u64)?;
    w.uint(2)?;
    w.uint(s.flags as u64)?;
    w.uint(3)?;
    w.bytes(s.header_digest.as_bytes())?;
    w.uint(4)?;
    w.uint(s.index_offset)?;
    w.uint(5)?;
    w.uint(s.index_length)?;
    w.uint(6)?;
    w.bytes(s.index_digest.as_bytes())?;
    w.uint(7)?;
    w.bytes(s.source_digest.as_bytes())
}
fn source_read(r: &mut R<'_>) -> Result<SourceCommit, E> {
    r.map(8)?;
    r.key(0)?;
    let length = r.uint()?;
    r.key(1)?;
    let version = u16::try_from(r.uint()?).map_err(|_| E::Unsupported)?;
    r.key(2)?;
    let flags = u16::try_from(r.uint()?).map_err(|_| E::Unsupported)?;
    r.key(3)?;
    let header_digest = r.digest()?;
    r.key(4)?;
    let index_offset = r.uint()?;
    r.key(5)?;
    let index_length = r.uint()?;
    r.key(6)?;
    let index_digest = r.digest()?;
    r.key(7)?;
    let source_digest = r.digest()?;
    let s = SourceCommit {
        length,
        version,
        flags,
        header_digest,
        index_offset,
        index_length,
        index_digest,
        source_digest,
    };
    s.validate()?;
    Ok(s)
}
fn context_write(w: &mut W<'_>, c: &ContextInput<'_>) -> Result<(), E> {
    c.parameters.validate()?;
    if c.entries > MAX_ENTRIES {
        return Err(E::Limit);
    }
    w.profile(13)?;
    w.uint(2)?;
    w.bytes(&c.parameters.nonce)?;
    w.uint(3)?;
    w.uint(c.parameters.generation)?;
    w.uint(4)?;
    optional(w, c.parameters.predecessor)?;
    w.uint(5)?;
    source_write(w, c.source)?;
    w.uint(6)?;
    w.uint(LEAF_BYTES)?;
    w.uint(7)?;
    w.uint(c.entries as u64)?;
    w.uint(8)?;
    write_refs(w, c.pages, c.entries)?;
    w.uint(9)?;
    w.uint(0)?;
    w.uint(10)?;
    w.bytes(c.parameters.semantic_contract.as_bytes())?;
    w.uint(11)?;
    w.bytes(c.parameters.policy.as_bytes())?;
    w.uint(12)?;
    w.bytes(c.parameters.consumer_profile.as_bytes())
}
pub fn encode_context_into(input: &ContextInput<'_>, out: &mut [u8]) -> Result<usize, E> {
    encode(out, |w| context_write(w, input))
}
pub fn decode_context(bytes: &[u8]) -> Result<ContextView<'_>, E> {
    let mut r = R::new(bytes)?;
    r.profile(13)?;
    r.key(2)?;
    let nonce = r.bytes()?.try_into().map_err(|_| E::Length)?;
    r.key(3)?;
    let generation = r.uint()?;
    r.key(4)?;
    let predecessor = r.optional_digest()?;
    r.key(5)?;
    let source = source_read(&mut r)?;
    r.key(6)?;
    if r.uint()? != LEAF_BYTES {
        return Err(E::Unsupported);
    }
    r.key(7)?;
    let entries = usize::try_from(r.uint()?).map_err(|_| E::Limit)?;
    if entries > MAX_ENTRIES {
        return Err(E::Limit);
    }
    r.key(8)?;
    let pages = read_refs(&mut r, entries)?;
    r.key(9)?;
    r.zero()?;
    r.key(10)?;
    let semantic_contract = r.digest()?;
    r.key(11)?;
    let policy = r.digest()?;
    r.key(12)?;
    let consumer_profile = r.digest()?;
    let parameters = ContextParameters {
        nonce,
        generation,
        predecessor,
        semantic_contract,
        policy,
        consumer_profile,
    };
    parameters.validate()?;
    r.finish()?;
    Ok(ContextView {
        encoded: bytes,
        parameters,
        source,
        entries,
        pages,
    })
}
fn body_write(w: &mut W<'_>, b: &BodyInput<'_>) -> Result<(), E> {
    let c = decode_context(b.context_root)?;
    if b.generation != c.parameters.generation
        || b.predecessor != c.parameters.predecessor
        || b.entries != c.entries
    {
        return Err(E::Context);
    }
    w.profile(8)?;
    w.uint(2)?;
    w.bytes(b.context_id.as_bytes())?;
    w.uint(3)?;
    w.bytes(b.context_root)?;
    w.uint(4)?;
    w.uint(b.generation)?;
    w.uint(5)?;
    optional(w, b.predecessor)?;
    w.uint(6)?;
    w.uint(b.entries as u64)?;
    w.uint(7)?;
    write_refs(w, b.pages, b.entries)
}
pub fn encode_body_into(input: &BodyInput<'_>, out: &mut [u8]) -> Result<usize, E> {
    encode(out, |w| body_write(w, input))
}
pub fn decode_body(bytes: &[u8]) -> Result<BodyView<'_>, E> {
    let mut r = R::new(bytes)?;
    r.profile(8)?;
    r.key(2)?;
    let context_id = r.digest()?;
    r.key(3)?;
    let context = decode_context(r.bytes()?)?;
    r.key(4)?;
    let generation = r.uint()?;
    r.key(5)?;
    let predecessor = r.optional_digest()?;
    r.key(6)?;
    let entries = usize::try_from(r.uint()?).map_err(|_| E::Limit)?;
    if entries != context.entries
        || generation != context.parameters.generation
        || predecessor != context.parameters.predecessor
    {
        return Err(E::Context);
    }
    r.key(7)?;
    let pages = read_refs(&mut r, entries)?;
    r.finish()?;
    Ok(BodyView {
        encoded: bytes,
        context_id,
        context,
        pages,
    })
}

pub fn context_encoded_len(input: &ContextInput<'_>) -> Result<usize, E> {
    let mut w = W::measure();
    context_write(&mut w, input)?;
    Ok(w.pos)
}
pub fn body_encoded_len(input: &BodyInput<'_>) -> Result<usize, E> {
    let mut w = W::measure();
    body_write(&mut w, input)?;
    Ok(w.pos)
}

impl core::fmt::Debug for ContextView<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ContextView")
            .field("entries", &self.entries)
            .field("generation", &self.parameters.generation)
            .finish_non_exhaustive()
    }
}
impl core::fmt::Debug for BodyView<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("BodyView")
            .field("context_id", &self.context_id)
            .field("entries", &self.context.entries)
            .finish_non_exhaustive()
    }
}
