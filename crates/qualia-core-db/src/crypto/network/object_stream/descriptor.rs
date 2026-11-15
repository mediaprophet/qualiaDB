//! Structural page commitments only. Publisher/source/empty-root authority is a later owner check.
use super::codec::{Reader, Writer, check_ref, read_ref, write_ref};
use super::{Context, Error, GeometryPage, MAX_DESCRIPTOR, PageRef, RootPage};
use crate::crypto::network::digest::sha384;
use crate::net::qdnf::types::StrongDigest;
#[derive(Clone, Copy)]
pub struct Descriptor<'a> {
    bytes: &'a [u8],
    context: Context<'a>,
    metadata_root: StrongDigest,
    refs: &'a [u8],
}
impl<'a> Descriptor<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, Error> {
        if bytes.len() > MAX_DESCRIPTOR {
            return Err(Error::Capacity);
        }
        let mut r = Reader::new(bytes, b"QBDL-PCD-V1\0")?;
        r.suite()?;
        r.zero()?;
        let len = r.u16()? as usize;
        let context = Context::parse(r.take(len)?)?;
        let metadata_root = r.digest()?;
        super::types::nonzero(&metadata_root.0)?;
        let count = r.u16()? as usize;
        if count != context.page_count() {
            return Err(Error::Malformed);
        }
        let refs = r.take(count * 56)?;
        r.end()?;
        for (i, b) in refs.chunks_exact(56).enumerate() {
            check_ref(read_ref(b)?, context.fields().objects as usize, i, 72, 52)?;
        }
        Ok(Self {
            bytes,
            context,
            metadata_root,
            refs,
        })
    }
    pub fn encoded(&self) -> &'a [u8] {
        self.bytes
    }
    pub fn id(&self) -> StrongDigest {
        sha384(self.bytes)
    }
    pub fn context(&self) -> Context<'a> {
        self.context
    }
    pub fn metadata_root(&self) -> StrongDigest {
        self.metadata_root
    }
    pub fn page_ref(&self, p: usize) -> Result<PageRef, Error> {
        if p >= self.context.page_count() {
            return Err(Error::Range);
        }
        read_ref(&self.refs[p * 56..(p + 1) * 56])
    }
    pub fn check_page_pair(
        &self,
        p: usize,
        g: GeometryPage<'_>,
        r: RootPage<'_>,
    ) -> Result<(), Error> {
        let gr = self.context.page_ref(p)?;
        let rr = self.page_ref(p)?;
        if g.page as usize != p
            || r.page as usize != p
            || g.count() != gr.rows as usize
            || r.count() != rr.rows as usize
            || r.context != self.context.id()
            || sha384(g.encoded()) != gr.digest
            || sha384(r.encoded()) != rr.digest
        {
            return Err(Error::Unauthorized);
        }
        for i in 0..g.count() {
            if g.row(i)?.id != r.row(i)?.id {
                return Err(Error::Malformed);
            }
        }
        Ok(())
    }
}
pub fn encode_descriptor(
    context: Context<'_>,
    root: StrongDigest,
    refs: &[PageRef],
    out: &mut [u8],
) -> Result<usize, Error> {
    super::types::nonzero(&root.0)?;
    if refs.len() != context.page_count() {
        return Err(Error::Malformed);
    }
    for (i, r) in refs.iter().enumerate() {
        check_ref(*r, context.fields().objects as usize, i, 72, 52)?;
    }
    let mut w = Writer::new(
        out,
        18 + context.encoded().len() + 50 + 56 * refs.len(),
        b"QBDL-PCD-V1\0",
    )?;
    w.u16(1)?;
    w.u16(0)?;
    w.u16(context.encoded().len() as u16)?;
    w.put(context.encoded())?;
    w.put(&root.0)?;
    w.u16(refs.len() as u16)?;
    for r in refs {
        write_ref(&mut w, *r)?;
    }
    w.finish()
}
/// Bounded monotonic structural inventory walk, not an authority receipt.
pub struct Inventory<'a> {
    descriptor: Descriptor<'a>,
    next: usize,
    last: u32,
    poisoned: bool,
}
impl<'a> Inventory<'a> {
    pub fn new(descriptor: Descriptor<'a>) -> Self {
        Self {
            descriptor,
            next: 0,
            last: 0,
            poisoned: false,
        }
    }
    pub fn accept(&mut self, g: GeometryPage<'_>, r: RootPage<'_>) -> Result<(), Error> {
        if self.poisoned {
            return Err(Error::Closed);
        }
        self.poisoned = true;
        self.descriptor.check_page_pair(self.next, g, r)?;
        if g.row(0)?.id <= self.last {
            return Err(Error::Malformed);
        }
        self.last = g.row(g.count() - 1)?.id;
        self.next += 1;
        self.poisoned = false;
        Ok(())
    }
    pub fn complete(&self) -> Result<(), Error> {
        if self.poisoned || self.next != self.descriptor.context.page_count() {
            Err(Error::Incomplete)
        } else {
            Ok(())
        }
    }
}
