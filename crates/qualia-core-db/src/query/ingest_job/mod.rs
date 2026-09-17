//! Durable ingest jobs: inspect, compare, resume, append, and attested streams.
//!
//! A job directory survives process death (unlike `tempfile::TempDir`). Incomplete
//! OSM-style runs can be reviewed and, when the source is seekable and uncompressed,
//! continued. HTTP/gzip streams are ingested without a second copy of the source;
//! SHA-256 of the *decompressed* bytes plus 16 MiB window hashes remain so the
//! original can be verified later without keeping the file.

#[cfg(not(target_arch = "wasm32"))]
mod append;
#[cfg(not(target_arch = "wasm32"))]
mod compare;
#[cfg(not(target_arch = "wasm32"))]
mod inspect;
#[cfg(not(target_arch = "wasm32"))]
mod job;
#[cfg(not(target_arch = "wasm32"))]
mod resume;
#[cfg(not(target_arch = "wasm32"))]
mod source;
#[cfg(not(target_arch = "wasm32"))]
mod status;

#[cfg(not(target_arch = "wasm32"))]
pub use append::append_rdf_to_root;
#[cfg(not(target_arch = "wasm32"))]
pub use compare::{compare_attestation_file_to_path, compare_attestation_to_stream, CompareReport};
#[cfg(not(target_arch = "wasm32"))]
pub use inspect::{inspect_job, inspect_legacy_scratch, inspect_volume_root, IngestInspectReport};
#[cfg(not(target_arch = "wasm32"))]
pub use job::{
    hex_encode, read_window_hashes, unix_now, window_commitment, write_json_atomic, IngestJob,
    IngestJobPhase, IngestJobSpec, SourceAttestation, JOB_CHECKPOINT, JOB_PROGRESS, JOB_SPEC,
};
#[cfg(not(target_arch = "wasm32"))]
pub use resume::{adopt_legacy_scratch, continue_job, publish_job, resume_is_supported};
#[cfg(not(target_arch = "wasm32"))]
pub use source::{
    detect_encoding, infer_rdf_format, open_ingest_source, DigestOutcome, DigestingReader,
    IngestEncoding, IngestRdfFormat, IngestSourceKind, OpenedSource, WINDOW_BYTES,
};
#[cfg(not(target_arch = "wasm32"))]
pub use status::{job_status, read_progress_json, IngestJobStatus};
