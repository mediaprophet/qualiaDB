use super::codec::{Reader, Writer};
use super::{Error, MAX_OBJECT_BYTES, MAX_PAGES, PAGE_ROWS};
use crate::net::qdnf::types::StrongDigest;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GeometryRow {
    pub id: u32,
    pub length: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RootRow {
    pub id: u32,
    pub root: StrongDigest,
}
fn header(page: u16, first: u32, count: usize) -> Result<(), Error> {
    if page as usize >= MAX_PAGES
        || first != u32::from(page) * PAGE_ROWS as u32
        || count == 0
        || count > PAGE_ROWS
        || first as usize + count > super::MAX_OBJECTS
    {
        return Err(Error::Range);
    }
    Ok(())
}
#[derive(Clone, Copy)]
pub struct GeometryPage<'a> {
    bytes: &'a [u8],
    pub page: u16,
    pub first: u32,
    rows: &'a [u8],
}
impl<'a> GeometryPage<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, Error> {
        let mut r = Reader::new(bytes, b"QBDL-PGE-V1\0")?;
        r.suite()?;
        let page = r.u16()?;
        let first = r.u32()?;
        let count = r.u16()? as usize;
        r.zero()?;
        header(page, first, count)?;
        let rows = r.take(count * 12)?;
        r.end()?;
        let view = Self {
            bytes,
            page,
            first,
            rows,
        };
        let mut last = 0;
        for i in 0..count {
            let row = view.row(i)?;
            if row.id <= last || row.length > MAX_OBJECT_BYTES {
                return Err(Error::Malformed);
            }
            last = row.id;
        }
        Ok(view)
    }
    pub fn count(&self) -> usize {
        self.rows.len() / 12
    }
    pub fn encoded(&self) -> &'a [u8] {
        self.bytes
    }
    pub fn row(&self, i: usize) -> Result<GeometryRow, Error> {
        if i >= self.count() {
            return Err(Error::Range);
        }
        let b = &self.rows[i * 12..(i + 1) * 12];
        Ok(GeometryRow {
            id: u32::from_be_bytes(b[..4].try_into().unwrap()),
            length: u64::from_be_bytes(b[4..].try_into().unwrap()),
        })
    }
}
pub fn encode_geometry(page: u16, rows: &[GeometryRow], out: &mut [u8]) -> Result<usize, Error> {
    header(page, u32::from(page) * 128, rows.len())?;
    let mut last = 0;
    for r in rows {
        if r.id <= last || r.length > MAX_OBJECT_BYTES {
            return Err(Error::Malformed);
        }
        last = r.id;
    }
    let mut w = Writer::new(out, 24 + 12 * rows.len(), b"QBDL-PGE-V1\0")?;
    w.u16(1)?;
    w.u16(page)?;
    w.u32(u32::from(page) * 128)?;
    w.u16(rows.len() as u16)?;
    w.u16(0)?;
    for r in rows {
        w.u32(r.id)?;
        w.u64(r.length)?;
    }
    w.finish()
}
#[derive(Clone, Copy)]
pub struct RootPage<'a> {
    bytes: &'a [u8],
    pub context: StrongDigest,
    pub page: u16,
    pub first: u32,
    rows: &'a [u8],
}
impl<'a> RootPage<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, Error> {
        let mut r = Reader::new(bytes, b"QBDL-PRT-V1\0")?;
        r.suite()?;
        let context = r.digest()?;
        super::types::nonzero(&context.0)?;
        let page = r.u16()?;
        let first = r.u32()?;
        let count = r.u16()? as usize;
        r.zero()?;
        header(page, first, count)?;
        let rows = r.take(count * 52)?;
        r.end()?;
        let view = Self {
            bytes,
            context,
            page,
            first,
            rows,
        };
        let mut last = 0;
        for i in 0..count {
            let row = view.row(i)?;
            if row.id <= last {
                return Err(Error::Malformed);
            }
            super::types::nonzero(&row.root.0)?;
            last = row.id;
        }
        Ok(view)
    }
    pub fn encoded(&self) -> &'a [u8] {
        self.bytes
    }
    pub fn count(&self) -> usize {
        self.rows.len() / 52
    }
    pub fn row(&self, i: usize) -> Result<RootRow, Error> {
        if i >= self.count() {
            return Err(Error::Range);
        }
        let b = &self.rows[i * 52..(i + 1) * 52];
        Ok(RootRow {
            id: u32::from_be_bytes(b[..4].try_into().unwrap()),
            root: StrongDigest(b[4..].try_into().unwrap()),
        })
    }
}
pub fn encode_roots(
    context: StrongDigest,
    page: u16,
    rows: &[RootRow],
    out: &mut [u8],
) -> Result<usize, Error> {
    super::types::nonzero(&context.0)?;
    header(page, u32::from(page) * 128, rows.len())?;
    let mut last = 0;
    for r in rows {
        if r.id <= last {
            return Err(Error::Malformed);
        }
        super::types::nonzero(&r.root.0)?;
        last = r.id;
    }
    let mut w = Writer::new(out, 72 + 52 * rows.len(), b"QBDL-PRT-V1\0")?;
    w.u16(1)?;
    w.put(&context.0)?;
    w.u16(page)?;
    w.u32(u32::from(page) * 128)?;
    w.u16(rows.len() as u16)?;
    w.u16(0)?;
    for r in rows {
        w.u32(r.id)?;
        w.put(&r.root.0)?;
    }
    w.finish()
}
