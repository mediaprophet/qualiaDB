//! QSync operation identity and local replay (SVC-01 partial).
//!
//! SHA-384 [`StrongDigest`](crate::net::qdnf::types::StrongDigest) operation
//! IDs. Transport ACK is not durability. A source signature is not reusable
//! after redaction. This library does not claim exactly-once external work,
//! content swarms, or RAM-sized datasets.
//!
//! Remaining SVC-01 packages (admission, proofs, merge, tombstones,
//! content transfer) stay open. Checkpoints are partial (SVC-01.07).

pub mod checkpoint;
pub mod operation;

pub use checkpoint::{
    build_checkpoint, forged_count_rejected, membership_implies_completeness, verify_checkpoint,
    Checkpoint, MAX_CHECKPOINT_OPS,
};
pub use operation::{
    operation_id, source_signature_reusable_after_redaction, transport_ack_is_durable,
    tx_from_operation_id, OpTable, OperationDesc, MAX_OPS, MAX_PARENTS,
};
