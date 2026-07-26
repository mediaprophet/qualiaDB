//! Prepared native inference runtime.
//!
//! This library is the plan/run boundary for production inference. Model discovery,
//! compilation, immutable upload, receipts, and artifact management are cold control-plane
//! work. A prepared backend's decode step is the Tier-1 zero-heap data plane.

pub mod artifacts;
pub mod graph_assist;
pub mod kv;
pub mod prepared;
pub mod receipt;
pub mod scheduler;

pub use artifacts::{
    cleanup_stale_runs, ArtifactError, ArtifactFinish, ArtifactRetention, ArtifactStats,
    RunArtifactDir, StaleCleanupReport,
};
pub use prepared::{
    DecodeStepError, DecodeStepInput, DecodeStepOutput, PreparedBackend, PreparedDecodePlan,
    PreparedPlanState,
};
pub use receipt::{
    capture_source_provenance, sha256_file, sha256_token_ids, ArtifactCleanupCounters, BackendKind,
    BenchmarkManifest, ExecutionCounters, ExecutionReceipt, SourceProvenance,
    MANIFEST_SCHEMA_VERSION, RAW_GREEDY_DECODE_POLICY, RECEIPT_SCHEMA_VERSION,
};
pub use scheduler::{Admission, RequestScheduler, RequestState, RequestView, SchedulerError};
