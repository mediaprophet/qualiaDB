//! Bounded reader-side helpers for unified Q42 volumes.
//!
//! These modules deliberately live beside the legacy builder while the writer is
//! decomposed in later phases.  They keep the read/query path checked and
//! caller-buffered without changing the 48-byte `NQuin` ABI.

mod car;
mod cid;
pub(crate) mod codec_probe;
mod compact;
mod inspect;
mod magnet;
mod opfs_source;
mod publication;
mod publish;
mod query_mode;
mod verify;
#[cfg(test)]
mod compat;
mod cursor;
mod index;
mod manifest;
mod postings;
mod range;
mod range_volume;
mod stream_writer;
mod validate;
mod write_quins;

pub use cursor::{Q42BlockCursor, Q42BlockMeta};
pub use index::{BidxBlockRange, BidxMatchPage};
pub use manifest::{
    root_relative_path, Q42LexiconRangeFactory, Q42LexiconSegment, Q42RangeVolumeSet,
    Q42SegmentMatchPage, Q42SegmentMatchRange, Q42SegmentRangeFactory, Q42VolumeManifest,
    Q42VolumeSegment, Q42VolumeSet, Q42VolumeSetQueryCursor, Q42VolumeSetQueryPage,
    MAX_VOLUME_MANIFEST_BYTES,
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
pub use car::{
    decode_and_verify_car, encode_raw_car, extract_entity_bytes, inclusive_entity_bytes,
    VerifiedCarBlock,
};
pub use cid::CidSha256;
pub use compact::compact_volume_set;
pub use opfs_source::{
    verify_car_bytes_as_q42_source, verify_local_car_as_q42_source, OpfsCallbackRangeSource,
    OpfsSliceRangeSource, VerifiedCarRangeSource,
};
pub use inspect::{Q42InspectReport, Q42SectionReport};
pub use magnet::{compose_magnet, sha1_hex_file, Q42Magnet, Q42VolumeSetMagnets};
pub use publication::{
    classify_q42_path, classify_q42_volume, classify_q42_volume_set, deny_public_publication,
    quin_requires_sanctuary, ClassificationCounts, PublicationIntent, Q42PublicationClass,
    Q42PublicationVerdict, Q42Transport,
};
pub use publish::{
    append_segment_to_root, Q42RolloverPublisher, DEFAULT_SEGMENT_MAX_BYTES,
};
pub use query_mode::{Q42QueryMode, RESIDENT_QUERY_MAX_BYTES};
pub use verify::{
    verify_volume_set_from_root, CheckStatus, Q42VerifyReceipt, Q42VerifySetReport, VerifyCheck,
    VerifyLevel,
};
pub use postings::{
    encode_block_postings, encode_postings_section, measure_bloom_false_positives,
    BlockFieldPostings, FIELD_POSTINGS_MAGIC,
};
pub use range_volume::{
    Q42ObjectMatchPage, Q42ObjectSearchCursor, Q42RangeQueryCursor, Q42RangeQueryPage,
    Q42RangeQueryPattern, Q42RangeQueryPlan, Q42RangeQueryStrategy, Q42RangeVolume,
};
pub use stream_writer::StreamingQ42VolumeWriter;
#[cfg(not(target_arch = "wasm32"))]
pub use write_quins::{write_sorted_quins_volume, write_sorted_quins_volume_with_author};

pub(crate) use index::{bidx_block_range_for_hash, bidx_blocks_for_hash_into};
pub(crate) use validate::validate_volume_structure;
