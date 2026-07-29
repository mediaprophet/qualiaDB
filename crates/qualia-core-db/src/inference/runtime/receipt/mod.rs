//! Serializable cold-path evidence and fixed-size hot-path counters.

mod execution;
mod manifest;
mod source;

pub use execution::{
    ArtifactCleanupCounters, BackendKind, ExecutionCounters, ExecutionReceipt,
    COUNTER_COMPILE_CALLS, COUNTER_COMPUTE_DISPATCHES, COUNTER_DECODE_STEPS, COUNTER_DEVICE_FENCES,
    COUNTER_DEVICE_TO_HOST_BYTES, COUNTER_FALLBACKS, COUNTER_GRAPH_LAUNCHES,
    COUNTER_HOST_TO_DEVICE_BYTES, COUNTER_HOT_ALLOCATIONS, COUNTER_IMMUTABLE_UPLOAD_BYTES,
    RECEIPT_SCHEMA_VERSION,
};
pub use manifest::{
    sha256_file, sha256_token_ids, BenchmarkManifest, MANIFEST_SCHEMA_VERSION,
    RAW_GREEDY_DECODE_POLICY,
};
pub use source::{capture_source_provenance, SourceProvenance};
