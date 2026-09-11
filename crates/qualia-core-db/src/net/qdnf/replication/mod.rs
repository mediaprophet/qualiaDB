//! Replica originals, QNF evaluation, and caller-paged streams (E06.4–E06.6).
//!
//! Q42 projections and existing artifact envelopes remain the selected
//! representation. [`qnf_eval::QnfExtensionAdopted`] is the evaluation
//! result, not a silent default. Large replica files are read through
//! [`stream`] in 4 KiB pages. Tempfile crash recovery lives in
//! [`disk_crash`]; OS process-kill WAL is not claimed.

pub mod disk_crash;
pub mod originals;
pub mod qnf_eval;
pub mod stream;

pub use disk_crash::{
    disk_backend_crash_injected, os_process_kill_qualified, FilePairStore, RecoveredPair,
    ReceiptClass, MAX_EFFECT_BYTES,
};
pub use originals::{
    projection_replaces_original, store_original, OriginalObject, STREAM_PAGE_BYTES,
};
pub use qnf_eval::{
    evaluate_qnf, FormatDecision, FormatRecord, QnfExtensionAdopted, SelectedRepresentation,
};
pub use stream::{hash_file_pages, verify_file_pages, PageWindow, PagedFile, PAGE_BYTES};
