//! Borrowed fixed-profile page references in deterministic encoded order.
use super::cbor::{R, W};
use super::{DescriptorError as E, PageRef, check_ref, pages_for};
#[derive(Clone, Copy)]
pub struct PageRefsView<'a> {
    encoded: &'a [u8],
    count: usize,
}
impl<'a> PageRefsView<'a> {
    pub fn len(self) -> usize {
        self.count
    }
    pub fn is_empty(self) -> bool {
        self.count == 0
    }
    pub fn get(self, index: usize) -> Result<PageRef, E> {
        if index >= self.count {
            return Err(E::Page);
        }
        let mut r = R::new(self.encoded)?;
        r.array()?;
        for i in 0..self.count {
            let row = read_ref(&mut r)?;
            if i == index {
                return Ok(row);
            }
        }
        Err(E::Page)
    }
    pub(crate) fn matches(self, refs: &[PageRef]) -> Result<(), E> {
        if refs.len() != self.count {
            return Err(E::Coverage);
        }
        for (i, want) in refs.iter().enumerate() {
            if self.get(i)? != *want {
                return Err(E::Commitment);
            }
        }
        Ok(())
    }
}
fn read_ref(r: &mut R<'_>) -> Result<PageRef, E> {
    r.fixed_array(5)?;
    Ok(PageRef {
        number: r.u32()?,
        first: r.u32()?,
        rows: r.u32()?,
        encoded_length: r.u32()?,
        digest: r.digest()?,
    })
}
pub(super) fn read_refs<'a>(r: &mut R<'a>, entries: usize) -> Result<PageRefsView<'a>, E> {
    let start = r.pos;
    let count = r.array()?;
    if count != pages_for(entries)? {
        return Err(E::Coverage);
    }
    for i in 0..count {
        check_ref(&read_ref(r)?, entries, i)?;
    }
    Ok(PageRefsView {
        encoded: &r.bytes[start..r.pos],
        count,
    })
}
pub(super) fn write_refs(w: &mut W<'_>, refs: &[PageRef], entries: usize) -> Result<(), E> {
    if refs.len() != pages_for(entries)? {
        return Err(E::Coverage);
    }
    w.array(refs.len())?;
    for (i, row) in refs.iter().enumerate() {
        check_ref(row, entries, i)?;
        w.array(5)?;
        w.uint(row.number as u64)?;
        w.uint(row.first as u64)?;
        w.uint(row.rows as u64)?;
        w.uint(row.encoded_length as u64)?;
        w.bytes(row.digest.as_bytes())?;
    }
    Ok(())
}

impl core::fmt::Debug for PageRefsView<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PageRefsView")
            .field("count", &self.count)
            .finish_non_exhaustive()
    }
}
