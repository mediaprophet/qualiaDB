//! Bounded reader-side helpers for unified Q42 volumes.
//!
//! These modules deliberately live beside the legacy builder while the writer is
//! decomposed in later phases.  They keep the read/query path checked and
//! caller-buffered without changing the 48-byte `NQuin` ABI.
//!
//! wasm32: the byte-format read path (`index`, `postings`, `validate`, `range`,
//! `range_volume`, `manifest` descriptors, `car`/`cid`, `opfs_source` slice
//! adapters, `publication` policy types, `query_mode`) is transport-neutral and
//! compiles for the browser. Modules that need a filesystem, mmap, tempfile,
//! or blocking HTTP stay `cfg(not(target_arch = "wasm32"))`.

mod car;
mod cid;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod codec_probe;
#[cfg(not(target_arch = "wasm32"))]
mod compact;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod compat;
#[cfg(not(target_arch = "wasm32"))]
mod cursor;
mod index;
#[cfg(not(target_arch = "wasm32"))]
mod inspect;
#[cfg(not(target_arch = "wasm32"))]
mod magnet;
mod manifest;
#[cfg(not(target_arch = "wasm32"))]
mod network_quanta;
mod opfs_source;
mod payload_catalogue;
mod postings;
mod publication;
#[cfg(not(target_arch = "wasm32"))]
mod publish;
mod query_mode;
mod range;
mod range_volume;
#[cfg(not(target_arch = "wasm32"))]
mod stream_writer;
mod validate;
mod wasm_slice_reader;
#[cfg(not(target_arch = "wasm32"))]
mod verify;
#[cfg(not(target_arch = "wasm32"))]
mod write_quins;

pub use car::{
    decode_and_verify_car, encode_raw_car, extract_entity_bytes, inclusive_entity_bytes,
    VerifiedCarBlock,
};
pub use cid::CidSha256;
#[cfg(not(target_arch = "wasm32"))]
pub use compact::compact_volume_set;
#[cfg(not(target_arch = "wasm32"))]
pub use cursor::{Q42BlockCursor, Q42BlockMeta};
pub use index::{BidxBlockRange, BidxMatchPage};
#[cfg(not(target_arch = "wasm32"))]
pub use inspect::{Q42InspectReport, Q42SectionReport};
#[cfg(not(target_arch = "wasm32"))]
pub use magnet::{compose_magnet, sha1_hex_file, Q42Magnet, Q42VolumeSetMagnets};
#[cfg(not(target_arch = "wasm32"))]
pub use manifest::{root_relative_path, Q42VolumeSet};
pub use manifest::{
    Q42LexiconRangeFactory, Q42LexiconSegment, Q42RangeVolumeSet, Q42SegmentMatchPage,
    Q42SegmentMatchRange, Q42SegmentRangeFactory, Q42VolumeManifest, Q42VolumeSegment,
    Q42VolumeSetQueryCursor, Q42VolumeSetQueryPage, MAX_VOLUME_MANIFEST_BYTES,
};
#[cfg(not(target_arch = "wasm32"))]
#[allow(unused_imports)] // integrator re-exports through q42_volume.rs
pub use network_quanta::{admit_logical_size, Completeness, NetworkCursor, ScanBudget};
#[cfg(not(target_arch = "wasm32"))]
pub use opfs_source::verify_local_car_as_q42_source;
pub use opfs_source::{
    verify_car_bytes_as_q42_source, OpfsCallbackRangeSource, OpfsSliceRangeSource,
    VerifiedCarRangeSource,
};
#[allow(unused_imports)]
pub use payload_catalogue::{
    decode_payload_catalogue_section, encode_payload_catalogue_section,
    header_payload_catalogue_range, set_header_payload_catalogue_range,
    PAYLOAD_CATALOGUE_HEADER_BYTES, PAYLOAD_CATALOGUE_MAGIC, PAYLOAD_CATALOGUE_VERSION,
};
pub use postings::{
    encode_block_postings, encode_postings_section, measure_bloom_false_positives,
    BlockFieldPostings, FIELD_POSTINGS_MAGIC,
};
#[cfg(not(target_arch = "wasm32"))]
pub use publication::{classify_q42_path, classify_q42_volume, classify_q42_volume_set};
pub use publication::{
    deny_public_publication, provenance_path_for, quin_requires_sanctuary, ClassificationCounts,
    IngestAccessPolicy, IngestProvenanceRecord, PublicationIntent, Q42PublicationClass,
    Q42PublicationVerdict, Q42Transport,
};
#[cfg(not(target_arch = "wasm32"))]
pub use publish::{append_segment_to_root, Q42RolloverPublisher, DEFAULT_SEGMENT_MAX_BYTES};
pub use query_mode::{Q42QueryMode, RESIDENT_QUERY_MAX_BYTES};
#[cfg(not(target_arch = "wasm32"))]
pub use range::{
    ipfs_gateway_range_source, ipns_gateway_range_source, HttpRangeSource,
    IpfsGatewaySegmentFactory, LocalFileRangeSource,
};
pub use range::{
    validate_exact_range_response, verify_source_sha256, Q42ByteRange, Q42RangeSource,
};
pub use range_volume::{
    Q42ObjectMatchPage, Q42ObjectSearchCursor, Q42RangeQueryCursor, Q42RangeQueryPage,
    Q42RangeQueryPattern, Q42RangeQueryPlan, Q42RangeQueryStrategy, Q42RangeVolume,
};
#[cfg(not(target_arch = "wasm32"))]
pub use stream_writer::StreamingQ42VolumeWriter;
#[cfg(not(target_arch = "wasm32"))]
pub use verify::{
    verify_volume_set_from_root, CheckStatus, Q42VerifyReceipt, Q42VerifySetReport, VerifyCheck,
    VerifyLevel,
};
#[cfg(not(target_arch = "wasm32"))]
pub use write_quins::{write_sorted_quins_volume, write_sorted_quins_volume_with_author};

pub(crate) use index::{bidx_block_range_for_hash, bidx_blocks_for_hash_into};
pub(crate) use validate::validate_volume_structure;
pub use wasm_slice_reader::WasmSliceQ42Reader;
