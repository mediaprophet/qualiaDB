//! Bounded reader-side helpers for unified Q42 volumes.
//!
//! These modules deliberately live beside the legacy builder while the writer is
//! decomposed in later phases.  They keep the read/query path checked and
//! caller-buffered without changing the 48-byte `NQuin` ABI.

mod cursor;
mod index;
mod validate;

pub use cursor::{Q42BlockCursor, Q42BlockMeta};
pub use index::{BidxBlockRange, BidxMatchPage};

pub(crate) use index::{bidx_block_range_for_hash, bidx_blocks_for_hash_into};
pub(crate) use validate::validate_volume_structure;
