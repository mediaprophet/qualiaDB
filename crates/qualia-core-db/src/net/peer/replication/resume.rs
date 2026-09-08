//! Resume only verified blocks of a pinned content manifest (SVC-01.14).
//!
//! A pin holds one [`ContentManifest`]. Verified bits are a `u8` bitmap aligned
//! to [`MAX_RANGES`] (8). An unverified block is not transferred
//! ([`QdnfError::Incomplete`]). A different digest cannot resume into an
//! existing pin ([`QdnfError::Conflict`]). Container generation is not a QSync
//! root. Packages remain open.

use super::manifest::{
    container_generation_is_qsync_root as manifest_is_qsync_root, ContentManifest, MAX_RANGES,
};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

/// Maximum concurrently pinned manifests.
pub const MAX_PINNED: usize = 4;

const _: () = assert!(MAX_RANGES == 8);
const _: () = assert!(MAX_RANGES <= u8::BITS as usize);

#[derive(Clone, Copy)]
struct PinSlot {
    manifest: Option<ContentManifest>,
    /// Bit `i` set ⇒ range index `i` is verified. Width matches [`MAX_RANGES`].
    verified: u8,
}

/// Four pinned manifests, each with an 8-bit verified bitmap.
pub struct ResumeTable {
    slots: [PinSlot; MAX_PINNED],
}

impl ResumeTable {
    pub const fn new() -> Self {
        Self {
            slots: [PinSlot {
                manifest: None,
                verified: 0,
            }; MAX_PINNED],
        }
    }
}

impl Default for ResumeTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Pin `manifest`. Same digest is idempotent (same pin id). Fifth distinct digest
/// is [`QdnfError::Capacity`].
pub fn pin(table: &mut ResumeTable, manifest: ContentManifest) -> Result<u8, QdnfError> {
    let mut i = 0usize;
    while i < MAX_PINNED {
        if let Some(held) = table.slots[i].manifest {
            if held.digest == manifest.digest {
                return Ok(i as u8);
            }
        }
        i += 1;
    }
    i = 0;
    while i < MAX_PINNED {
        if table.slots[i].manifest.is_none() {
            table.slots[i].manifest = Some(manifest);
            table.slots[i].verified = 0;
            return Ok(i as u8);
        }
        i += 1;
    }
    Err(QdnfError::Capacity)
}

/// Mark range `block_index` verified on `pin`. Only indices in `0..range_count`
/// of the pinned manifest may be marked.
pub fn mark_verified(table: &mut ResumeTable, pin: u8, block_index: u8) -> Result<(), QdnfError> {
    let range_count = pin_range_count(table, pin)?;
    if (block_index as usize) >= range_count {
        return Err(QdnfError::Range);
    }
    table.slots[pin as usize].verified |= 1u8 << block_index;
    Ok(())
}

/// Resume range `block_index` of `pin`. Ok only when that range is verified.
/// Unverified is [`QdnfError::Incomplete`], not transferred.
pub fn resume_block(table: &ResumeTable, pin: u8, block_index: u8) -> Result<(), QdnfError> {
    let (range_count, verified) = pin_verified(table, pin)?;
    if (block_index as usize) >= range_count {
        return Err(QdnfError::Range);
    }
    if (verified & (1u8 << block_index)) == 0 {
        return Err(QdnfError::Incomplete);
    }
    Ok(())
}

/// A different digest cannot resume into this pin.
pub fn resume_digest(table: &ResumeTable, pin: u8, digest: StrongDigest) -> Result<(), QdnfError> {
    let held = pin_manifest(table, pin)?;
    if held.digest != digest {
        return Err(QdnfError::Conflict);
    }
    Ok(())
}

/// Unverified blocks are never treated as transferred.
#[inline]
pub fn resume_unverified_is_allowed() -> bool {
    false
}

/// Container generation is not an authorized QSync root (SVC-01.14).
#[inline]
pub fn container_generation_is_qsync_root() -> bool {
    manifest_is_qsync_root()
}

fn pin_slot(table: &ResumeTable, pin: u8) -> Result<&PinSlot, QdnfError> {
    let i = pin as usize;
    if i >= MAX_PINNED {
        return Err(QdnfError::Unauthorized);
    }
    let slot = &table.slots[i];
    if slot.manifest.is_none() {
        return Err(QdnfError::Unauthorized);
    }
    Ok(slot)
}

fn pin_manifest(table: &ResumeTable, pin: u8) -> Result<ContentManifest, QdnfError> {
    pin_slot(table, pin)?
        .manifest
        .ok_or(QdnfError::Unauthorized)
}

fn pin_range_count(table: &ResumeTable, pin: u8) -> Result<usize, QdnfError> {
    Ok(pin_manifest(table, pin)?.range_count())
}

fn pin_verified(table: &ResumeTable, pin: u8) -> Result<(usize, u8), QdnfError> {
    let slot = pin_slot(table, pin)?;
    let range_count = slot.manifest.ok_or(QdnfError::Unauthorized)?.range_count();
    Ok((range_count, slot.verified))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::peer::replication::manifest::ByteRange;

    fn digest(tag: u8) -> StrongDigest {
        let mut d = StrongDigest::ZERO;
        d.0[0] = tag;
        d
    }

    fn bind(tag: u8, n_ranges: usize) -> ContentManifest {
        let mut ranges = [ByteRange { offset: 0, len: 1 }; MAX_RANGES];
        let mut i = 0usize;
        while i < n_ranges {
            ranges[i] = ByteRange {
                offset: (i as u64) * 8,
                len: 4,
            };
            i += 1;
        }
        ContentManifest::bind(digest(tag), 0, 64, &ranges[..n_ranges]).unwrap()
    }

    #[test]
    fn pin_mark_verified_resume_ok() {
        let mut table = ResumeTable::new();
        let pin_id = pin(&mut table, bind(1, 2)).unwrap();
        mark_verified(&mut table, pin_id, 0).unwrap();
        resume_block(&table, pin_id, 0).unwrap();
    }

    #[test]
    fn resume_before_mark_verified_is_incomplete() {
        let mut table = ResumeTable::new();
        let pin_id = pin(&mut table, bind(1, 2)).unwrap();
        assert_eq!(resume_block(&table, pin_id, 0), Err(QdnfError::Incomplete));
    }

    #[test]
    fn block_index_past_range_count_is_range() {
        let mut table = ResumeTable::new();
        let pin_id = pin(&mut table, bind(1, 2)).unwrap();
        assert_eq!(mark_verified(&mut table, pin_id, 2), Err(QdnfError::Range));
        assert_eq!(resume_block(&table, pin_id, 2), Err(QdnfError::Range));
    }

    #[test]
    fn fifth_pin_is_capacity() {
        let mut table = ResumeTable::new();
        pin(&mut table, bind(1, 1)).unwrap();
        pin(&mut table, bind(2, 1)).unwrap();
        pin(&mut table, bind(3, 1)).unwrap();
        pin(&mut table, bind(4, 1)).unwrap();
        assert_eq!(pin(&mut table, bind(5, 1)), Err(QdnfError::Capacity));
    }

    #[test]
    fn pin_same_digest_is_idempotent() {
        let mut table = ResumeTable::new();
        let a = pin(&mut table, bind(7, 2)).unwrap();
        let b = pin(&mut table, bind(7, 2)).unwrap();
        assert_eq!(a, b);
        pin(&mut table, bind(8, 1)).unwrap();
        let c = pin(&mut table, bind(7, 1)).unwrap();
        assert_eq!(a, c);
    }

    #[test]
    fn mark_verified_unknown_pin_is_unauthorized() {
        let mut table = ResumeTable::new();
        assert_eq!(
            mark_verified(&mut table, 0, 0),
            Err(QdnfError::Unauthorized)
        );
        assert_eq!(
            mark_verified(&mut table, MAX_PINNED as u8, 0),
            Err(QdnfError::Unauthorized)
        );
    }

    #[test]
    fn resume_unverified_is_not_allowed() {
        assert!(!resume_unverified_is_allowed());
    }

    #[test]
    fn container_generation_is_not_qsync_root() {
        assert!(!container_generation_is_qsync_root());
        assert!(!manifest_is_qsync_root());
    }

    #[test]
    fn different_digest_cannot_resume_into_pin() {
        let mut table = ResumeTable::new();
        let pin_id = pin(&mut table, bind(1, 1)).unwrap();
        resume_digest(&table, pin_id, digest(1)).unwrap();
        assert_eq!(
            resume_digest(&table, pin_id, digest(2)),
            Err(QdnfError::Conflict)
        );
    }
}
