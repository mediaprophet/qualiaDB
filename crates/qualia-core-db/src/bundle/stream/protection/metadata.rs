//! Private fixed-width M serializer, never an exported key-bearing byte object.
use super::{ObjectKind, geometry::PreparedGeometry, publisher::PublisherError as E};
use crate::crypto::network::object_stream::Context;
use crate::net::qdnf::types::StrongDigest;
use crate::specialized_libs::cryptographic_library::ManagedObjectPublication;
use zeroize::Zeroize;
/// Exact references awaiting the separate recognized closure/policy composition.
/// Construction alone neither authenticates these digests nor grants disclosure.
#[derive(Clone, Copy)]
pub(crate) struct PrivateReferences {
    pub(crate) closure: StrongDigest,
    pub(crate) policy: StrongDigest,
    pub(crate) consumer: StrongDigest,
}
pub(super) struct Erase<'a> {
    pub(super) bytes: &'a mut [u8],
}
impl Drop for Erase<'_> {
    fn drop(&mut self) {
        self.bytes.zeroize();
    }
}
struct Writer<'a> {
    bytes: &'a mut [u8],
    pos: usize,
}
impl Writer<'_> {
    fn put(&mut self, b: &[u8]) -> Result<(), E> {
        let end = self.pos.checked_add(b.len()).ok_or(E::Capacity)?;
        self.bytes
            .get_mut(self.pos..end)
            .ok_or(E::Capacity)?
            .copy_from_slice(b);
        self.pos = end;
        Ok(())
    }
    fn u16(&mut self, n: u16) -> Result<(), E> {
        self.put(&n.to_be_bytes())
    }
    fn u32(&mut self, n: u32) -> Result<(), E> {
        self.put(&n.to_be_bytes())
    }
    fn u64(&mut self, n: u64) -> Result<(), E> {
        self.put(&n.to_be_bytes())
    }
}
pub(super) fn write(
    g: &PreparedGeometry<'_, '_>,
    p: &ManagedObjectPublication<'_>,
    c: Context<'_>,
    refs: PrivateReferences,
    out: &mut [u8],
) -> Result<(), E> {
    if out.len() != g.metadata_length()
        || refs.closure.is_zero()
        || refs.policy.is_zero()
        || refs.consumer.is_zero()
    {
        return Err(E::Capacity);
    }
    g.check_context(c)?;
    let h = g.source.checked_header();
    let mut w = Writer { bytes: out, pos: 0 };
    w.put(b"QBDL-PMD-V1\0")?;
    w.u16(1)?;
    w.u16(0)?;
    w.put(&c.id().0)?;
    w.u16(h.probe().version)?;
    w.u16(h.probe().flags)?;
    w.u64(h.total_length())?;
    w.put(&g.source_digest.0)?;
    w.u64(h.index_offset())?;
    w.u32(64)?;
    w.u32(u32::try_from(h.index_length()).map_err(|_| E::Capacity)?)?;
    w.put(&refs.closure.0)?;
    w.put(&refs.policy.0)?;
    w.put(&refs.consumer.0)?;
    w.u32(g.object_count() as u32)?;
    if w.pos != 288 {
        return Err(E::Codec);
    }
    for (sorted, m) in g.mappings.iter().enumerate() {
        let row = &g.source.objects()[m.source_slot];
        w.u32(m.id)?;
        w.u64(row.offset())?;
        w.u64(row.length())?;
        let (kind, ordinal) = match row.kind() {
            ObjectKind::Entry(n) => (1, n),
            ObjectKind::Gap => (2, u32::MAX),
        };
        w.put(&[kind])?;
        w.u32(ordinal)?;
        w.put(&[u8::from(row.length() > 0)])?;
        w.u16(0)?;
        w.put(&m.digest.0)?;
        let end = w.pos.checked_add(32).ok_or(E::Capacity)?;
        let master: &mut [u8; 32] = w
            .bytes
            .get_mut(w.pos..end)
            .ok_or(E::Capacity)?
            .try_into()
            .map_err(|_| E::Capacity)?;
        p.metadata_master_row(sorted, master)
            .map_err(|_| E::Crypto)?;
        if (*master == [0; 32]) != (row.length() == 0) {
            return Err(E::Crypto);
        }
        w.pos = end;
    }
    w.put(g.source.header_bytes())?;
    w.put(g.source.index_bytes())?;
    if w.pos != w.bytes.len() {
        return Err(E::Codec);
    }
    Ok(())
}
