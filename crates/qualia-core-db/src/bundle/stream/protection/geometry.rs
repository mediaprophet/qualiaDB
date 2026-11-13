//! Actual existing geometry/context codecs over a private source mapping.
use super::{
    PreparedSource,
    ids::{self, Mapping},
    publisher::PublisherError as E,
};
use crate::crypto::network::{digest::sha384, object_stream::*};
use crate::net::qdnf::types::StrongDigest;
pub(crate) const EMPTY_REF: PageRef = PageRef {
    page: 0,
    rows: 0,
    length: 0,
    digest: StrongDigest::ZERO,
};
pub(crate) const EMPTY_ROOT: RootRow = RootRow {
    id: 0,
    root: StrongDigest::ZERO,
};
pub(crate) struct PreparedGeometry<'a, 's> {
    pub(super) source: PreparedSource<'a, 's>,
    pub(super) mappings: &'s [Mapping],
    pub(super) refs: &'s [PageRef],
    pub(super) source_digest: StrongDigest,
}
impl<'a, 's> PreparedGeometry<'a, 's> {
    pub(crate) fn prepare(
        source: PreparedSource<'a, 's>,
        mappings: &'s mut [Mapping],
        refs: &'s mut [PageRef],
        scratch: &mut [u8],
    ) -> Result<Self, E> {
        let n = source.objects().len();
        let pages = n.div_ceil(PAGE_ROWS);
        let mappings = mappings.get_mut(..n).ok_or(E::Capacity)?;
        let refs = refs.get_mut(..pages).ok_or(E::Capacity)?;
        if scratch.len() < 24 + PAGE_ROWS * 12 {
            return Err(E::Capacity);
        }
        ids::assign(&source, mappings)?;
        for (page, reference) in refs.iter_mut().enumerate() {
            *reference = page_bytes(&source, mappings, page, scratch)?;
        }
        let source_digest = sha384(source.source_bytes());
        Ok(Self {
            source,
            mappings,
            refs,
            source_digest,
        })
    }
    pub(crate) fn object_count(&self) -> usize {
        self.mappings.len()
    }
    pub(crate) fn metadata_length(&self) -> usize {
        self.source.metadata_length()
    }
    pub(crate) fn encode_page(&self, page: usize, out: &mut [u8]) -> Result<PageRef, E> {
        page_bytes(&self.source, self.mappings, page, out)
    }
    pub(crate) fn encode_context(&self, fields: ContextFields, out: &mut [u8]) -> Result<usize, E> {
        if fields.objects as usize != self.object_count()
            || fields.metadata_length != self.metadata_length() as u64
        {
            return Err(E::Source);
        }
        encode_context(fields, self.refs, out).map_err(|_| E::Codec)
    }
    pub(super) fn check_context(&self, c: Context<'_>) -> Result<(), E> {
        if c.fields().objects as usize != self.object_count()
            || c.fields().metadata_length != self.metadata_length() as u64
            || c.page_count() != self.refs.len()
        {
            return Err(E::Source);
        }
        for (i, r) in self.refs.iter().enumerate() {
            if c.page_ref(i).map_err(|_| E::Codec)? != *r {
                return Err(E::Source);
            }
        }
        Ok(())
    }
}
fn page_bytes(
    source: &PreparedSource<'_, '_>,
    maps: &[Mapping],
    page: usize,
    out: &mut [u8],
) -> Result<PageRef, E> {
    if page >= maps.len().div_ceil(PAGE_ROWS) {
        return Err(E::State);
    }
    let start = page * PAGE_ROWS;
    let end = (start + PAGE_ROWS).min(maps.len());
    let mut rows = [GeometryRow { id: 0, length: 0 }; PAGE_ROWS];
    for (row, m) in rows.iter_mut().zip(&maps[start..end]) {
        let s = source.objects().get(m.source_slot).ok_or(E::Source)?;
        *row = GeometryRow {
            id: m.id,
            length: s.length(),
        };
    }
    let n = encode_geometry(page as u16, &rows[..end - start], out).map_err(|_| E::Codec)?;
    Ok(PageRef {
        page: page as u16,
        rows: (end - start) as u16,
        length: n as u32,
        digest: sha384(&out[..n]),
    })
}
