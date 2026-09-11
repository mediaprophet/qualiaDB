//! QSync operation identity and local replay (SVC-01 partial).
//!
//! SHA-384 [`StrongDigest`](crate::net::qdnf::types::StrongDigest) operation
//! IDs. Transport ACK is not durability. A source signature is not reusable
//! after redaction. This library does not claim exactly-once external work,
//! content swarms, or RAM-sized datasets.
//!
//! Remaining SVC-01 packages stay open. Checkpoints are partial (SVC-01.07).
//! Tombstones are partial (SVC-01.11). Membership proofs are partial (SVC-01.08).
//! Merge is partial (SVC-01.10). Manifests are partial (SVC-01.12). Transfer is
//! partial (SVC-01.13). Resume is partial (SVC-01.14). Receipts are partial
//! (SVC-01.04). Crash injection is in-memory (SVC-01.15). Scan is bounded-page
//! (SVC-01.16) and does not materialize RAM-sized datasets. Evidential pages
//! copy through [`source::ScanSource`]; the synthetic XOR `next_page` fill is
//! not storage evidence. Cache misses are Incomplete (E06.3). Originals are
//! distinct from projections (E06.4). Production scans require ScanSource
//! (E11.1).

pub mod cache;
pub mod checkpoint;
pub mod commit_adapter;
pub mod crash;
pub mod custody;
pub mod disk_crash;
pub mod dtn;
pub mod manifest;
pub mod merge;
pub mod merkle;
pub mod operation;
pub mod originals;
pub mod pair;
pub mod projection;
pub mod proof;
pub mod receipts;
pub mod resume;
pub mod scan;
pub mod source;
pub mod tombstone;
pub mod transfer;

pub use cache::{
    cache_key, eviction_proves_absence, insert as cache_insert, lookup as cache_lookup,
    CacheLookup, CacheTable,
};
pub use checkpoint::{
    build_checkpoint, forged_count_rejected, membership_implies_completeness, verify_checkpoint,
    Checkpoint, MAX_CHECKPOINT_OPS,
};
pub use commit_adapter::{commit_verified_block, resume_from_log};
pub use custody::{mark_stored, CustodyState};
pub use dtn::{retry_is_new_effect, DtnHold};
pub use manifest::{
    container_generation_is_qsync_root, disclose_raw_artifact,
    raw_artifact_requires_full_authorization, ByteRange, ContentManifest, MAX_DECODED_BYTES,
    MAX_RANGES,
};
pub use merge::{
    decide, lww_overrides_revocation, wall_clock_is_membership_authority, Alternate, MergeProfile,
    MergeSet, MAX_ALTERNATIVES,
};
pub use merkle::{bounded_membership, manifest_page, range_witness, ManifestPage, RangeWitness};
pub use operation::{
    operation_id, source_signature_reusable_after_redaction, transport_ack_is_durable,
    tx_from_operation_id, OpTable, OperationDesc, MAX_OPS, MAX_PARENTS,
};
pub use originals::{store_original, OriginalObject, QnfExtensionAdopted};
pub use pair::{commit_pair, recover_pair, stage_paired, DurablePairing};
pub use projection::{derive_projection, membership_oracle_from_dedup, plan_delta, Projection};
pub use proof::{
    membership_implies_range_coverage, private_neighbor_metadata_in_proof, prove_membership,
    verify_membership, MembershipProof, MAX_PROOF_IDS,
};
pub use tombstone::{
    tombstone_implies_global_completeness, transport_ack_is_compaction_frontier, ReplicaGate,
    Tombstone, TombstoneTable, MAX_TOMBSTONES,
};
