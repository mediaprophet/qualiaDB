//! QSync operation identity and local replay (SVC-01 partial).
//!
//! SHA-384 [`StrongDigest`](crate::net::qdnf::types::StrongDigest) operation
//! IDs. Transport ACK is not durability. A source signature is not reusable
//! after redaction. This library does not claim exactly-once external work,
//! content swarms, or RAM-sized datasets.
//!
//! Remaining SVC-01 packages (admission, content transfer)
//! stay open. Checkpoints are partial (SVC-01.07). Tombstones are partial
//! (SVC-01.11). Membership proofs are partial (SVC-01.08). Merge is partial
//! (SVC-01.10).

pub mod checkpoint;
pub mod merge;
pub mod operation;
pub mod proof;
pub mod tombstone;

pub use checkpoint::{
    build_checkpoint, forged_count_rejected, membership_implies_completeness, verify_checkpoint,
    Checkpoint, MAX_CHECKPOINT_OPS,
};
pub use merge::{
    decide, lww_overrides_revocation, wall_clock_is_membership_authority, Alternate, MergeProfile,
    MergeSet, MAX_ALTERNATIVES,
};
pub use operation::{
    operation_id, source_signature_reusable_after_redaction, transport_ack_is_durable,
    tx_from_operation_id, OpTable, OperationDesc, MAX_OPS, MAX_PARENTS,
};
pub use proof::{
    membership_implies_range_coverage, private_neighbor_metadata_in_proof, prove_membership,
    verify_membership, MembershipProof, MAX_PROOF_IDS,
};
pub use tombstone::{
    tombstone_implies_global_completeness, transport_ack_is_compaction_frontier, ReplicaGate,
    Tombstone, TombstoneTable, MAX_TOMBSTONES,
};
