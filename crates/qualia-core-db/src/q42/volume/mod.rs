//! Bounded reader-side helpers for unified Q42 volumes.
//!
//! These modules deliberately live beside the legacy builder while the writer is
//! decomposed in later phases.  They keep the read/query path checked and
//! caller-buffered without changing the 48-byte `NQuin` ABI.

mod cursor;
mod index;
mod manifest;
mod range;
mod range_volume;
mod stream_writer;
mod validate;

pub use cursor::{Q42BlockCursor, Q42BlockMeta};
pub use index::{BidxBlockRange, BidxMatchPage};
pub use manifest::{
    root_relative_path, Q42RangeVolumeSet, Q42SegmentMatchPage, Q42SegmentMatchRange,
    Q42SegmentRangeFactory, Q42VolumeManifest, Q42VolumeSegment, Q42VolumeSet,
    Q42VolumeSetQueryCursor, Q42VolumeSetQueryPage, MAX_VOLUME_MANIFEST_BYTES,
};
#[cfg(not(target_arch = "wasm32"))]
pub use range::{
    ipfs_gateway_range_source, ipns_gateway_range_source, HttpRangeSource,
    IpfsGatewaySegmentFactory,
};
pub use range::{
    validate_exact_range_response, verify_source_sha256, LocalFileRangeSource, Q42ByteRange,
    Q42RangeSource,
};
pub use range_volume::{
    Q42ObjectMatchPage, Q42ObjectSearchCursor, Q42RangeQueryCursor, Q42RangeQueryPage,
    Q42RangeQueryPattern, Q42RangeQueryPlan, Q42RangeQueryStrategy, Q42RangeVolume,
};
pub use stream_writer::StreamingQ42VolumeWriter;

pub(crate) use index::{bidx_block_range_for_hash, bidx_blocks_for_hash_into};
pub(crate) use validate::validate_volume_structure;
