use super::cbor::{R, W, encode};
use super::{
    ContextEntry, ContextPageHeader, DescriptorError as E, EntryPageHeader, EntryRecord,
    MAX_ENTRIES, MAX_PAGES, PAGE_ROWS,
};
use crate::net::qdnf::types::StrongDigest;
fn geometry(number: u32, first: u32, rows: usize) -> Result<(), E> {
    if number as usize >= MAX_PAGES
        || first as usize != number as usize * PAGE_ROWS
        || rows == 0
        || rows > PAGE_ROWS
        || first as usize + rows > MAX_ENTRIES
    {
        return Err(E::Page);
    }
    Ok(())
}
fn context_write(w: &mut W<'_>, number: u32, first: u32, rows: &[ContextEntry]) -> Result<(), E> {
    geometry(number, first, rows.len())?;
    w.profile(5)?;
    w.uint(2)?;
    w.uint(number as u64)?;
    w.uint(3)?;
    w.uint(first as u64)?;
    w.uint(4)?;
    w.array(rows.len())?;
    for row in rows {
        if row.length > (1 << 40) {
            return Err(E::Limit);
        }
        w.array(4)?;
        w.uint(row.id as u64)?;
        w.uint(row.length)?;
        w.uint(0)?;
        w.uint(0)?;
    }
    Ok(())
}
pub fn encode_context_page_into(
    number: u32,
    first: u32,
    rows: &[ContextEntry],
    out: &mut [u8],
) -> Result<usize, E> {
    encode(out, |w| context_write(w, number, first, rows))
}
pub fn decode_context_page(bytes: &[u8], out: &mut [ContextEntry]) -> Result<ContextPageHeader, E> {
    let mut r = R::new(bytes)?;
    r.profile(5)?;
    r.key(2)?;
    let number = r.u32()?;
    r.key(3)?;
    let first = r.u32()?;
    r.key(4)?;
    let rows = r.array()?;
    geometry(number, first, rows)?;
    if out.len() < rows {
        return Err(E::Capacity);
    }
    for row in &mut out[..rows] {
        r.fixed_array(4)?;
        let id = r.u32()?;
        let length = r.uint()?;
        if length > (1 << 40) {
            return Err(E::Limit);
        }
        r.zero()?;
        r.zero()?;
        *row = ContextEntry { id, length };
    }
    r.finish()?;
    Ok(ContextPageHeader {
        number,
        first,
        rows,
    })
}
fn entry_write(
    w: &mut W<'_>,
    context: StrongDigest,
    number: u32,
    first: u32,
    rows: &[EntryRecord<'_>],
) -> Result<(), E> {
    geometry(number, first, rows.len())?;
    w.profile(6)?;
    w.uint(2)?;
    w.bytes(context.as_bytes())?;
    w.uint(3)?;
    w.uint(number as u64)?;
    w.uint(4)?;
    w.uint(first as u64)?;
    w.uint(5)?;
    w.array(rows.len())?;
    for (i, row) in rows.iter().enumerate() {
        if row.ordinal as usize != first as usize + i
            || row.kind.is_empty()
            || row.kind.len() > 64
            || row.offset < 64
            || row.length > (1 << 40)
            || row.offset.checked_add(row.length).is_none()
        {
            return Err(E::Field);
        }
        w.array(9)?;
        w.uint(row.id as u64)?;
        w.uint(row.ordinal as u64)?;
        w.text(row.kind)?;
        w.uint(row.inner_version)?;
        w.uint(row.offset)?;
        w.uint(row.length)?;
        w.uint(0)?;
        w.uint(0)?;
        w.bytes(row.root.as_bytes())?;
    }
    Ok(())
}
pub fn encode_entry_page_into(
    context: StrongDigest,
    number: u32,
    first: u32,
    rows: &[EntryRecord<'_>],
    out: &mut [u8],
) -> Result<usize, E> {
    encode(out, |w| entry_write(w, context, number, first, rows))
}
pub fn decode_entry_page<'a>(
    bytes: &'a [u8],
    out: &mut [EntryRecord<'a>],
) -> Result<EntryPageHeader, E> {
    let mut r = R::new(bytes)?;
    r.profile(6)?;
    r.key(2)?;
    let context = r.digest()?;
    r.key(3)?;
    let number = r.u32()?;
    r.key(4)?;
    let first = r.u32()?;
    r.key(5)?;
    let rows = r.array()?;
    geometry(number, first, rows)?;
    if out.len() < rows {
        return Err(E::Capacity);
    }
    for (i, row) in out[..rows].iter_mut().enumerate() {
        r.fixed_array(9)?;
        let id = r.u32()?;
        let ordinal = r.u32()?;
        let kind = r.text()?;
        let inner_version = r.uint()?;
        let offset = r.uint()?;
        let length = r.uint()?;
        r.zero()?;
        r.zero()?;
        let root = r.digest()?;
        if ordinal as usize != first as usize + i
            || kind.is_empty()
            || kind.len() > 64
            || offset < 64
            || length > (1 << 40)
            || offset.checked_add(length).is_none()
        {
            return Err(E::Field);
        }
        *row = EntryRecord {
            id,
            ordinal,
            kind,
            inner_version,
            offset,
            length,
            root,
        };
    }
    r.finish()?;
    Ok(EntryPageHeader {
        context,
        number,
        first,
        rows,
    })
}
