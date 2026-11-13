//! Private opaque-ID mapping; original ordinals never become public IDs.
use super::{PreparedSource, publisher::PublisherError as E};
use crate::crypto::network::{digest::sha384, entropy::fill_os};
use crate::net::qdnf::types::StrongDigest;
#[derive(Clone, Copy)]
pub(crate) struct Mapping {
    pub(super) id: u32,
    pub(super) source_slot: usize,
    pub(super) digest: StrongDigest,
}
impl Mapping {
    pub(crate) const EMPTY: Self = Self {
        id: 0,
        source_slot: 0,
        digest: StrongDigest::ZERO,
    };
}
pub(super) fn assign(source: &PreparedSource<'_, '_>, out: &mut [Mapping]) -> Result<(), E> {
    assign_with(source, out, |b| fill_os(b).map_err(|_| E::Crypto))
}
fn assign_with(
    source: &PreparedSource<'_, '_>,
    out: &mut [Mapping],
    mut random: impl FnMut(&mut [u8]) -> Result<(), E>,
) -> Result<(), E> {
    if out.len() != source.objects().len() {
        return Err(E::Capacity);
    }
    for slot in 0..out.len() {
        let mut chosen = None;
        for _ in 0..16 {
            let mut bytes = [0; 4];
            random(&mut bytes)?;
            let id = u32::from_be_bytes(bytes);
            if id != 0 && !out[..slot].iter().any(|m| m.id == id) {
                chosen = Some(id);
                break;
            }
        }
        out[slot] = Mapping {
            id: chosen.ok_or(E::Entropy)?,
            source_slot: slot,
            digest: sha384(source.object_bytes(slot).map_err(|_| E::Source)?),
        };
    }
    out.sort_unstable_by_key(|m| m.id);
    Ok(())
}
#[cfg(test)]
pub(super) fn assign_fixture(
    source: &PreparedSource<'_, '_>,
    out: &mut [Mapping],
    random: impl FnMut(&mut [u8]) -> Result<(), E>,
) -> Result<(), E> {
    assign_with(source, out, random)
}
