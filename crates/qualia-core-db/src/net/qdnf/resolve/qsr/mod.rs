//! Qualia Scoped Rendezvous exact lookup (QSR-A through QSR-H, E07.4–E07.8).
//!
//! Digit indexing, cover validation, closed-world snapshot traversal, tagged
//! outcomes, membership-versus-completeness evidence, scoped private tokens,
//! hot-key coalescing, writer handover, and an honest closed-world XOR
//! comparison (not networked Kademlia). Authenticated lookup never returns
//! bool. Found/EmptyInSnapshot is not completeness.

mod compare;
mod completeness;
mod cover;
mod exact;
mod handover;
mod hot;
mod key;
mod outcome;
mod roots;
mod tokens;
mod traversal;

pub use compare::{
    kademlia_comparison_executed, local_xor_nearest, run_closed_world_comparison,
    unmeasured_better_than_kademlia_claimed, xor_distance, ClosedWorldComparison, LOCAL_XOR_CAP,
};
pub use completeness::{
    absence_is_authoritative, lookup_with_completeness, membership_implies_completeness,
    CompletenessEvidence, CompletenessKind, QsrAnswer,
};
pub use cover::{covers_digit, lookup_exact, validate_cover, CoverInterval};
pub use exact::{HandoverStage, Qsr, TokenStage};
pub use handover::{
    accept_mutation, activate_new, admit_write, drain_accepted, fence_old, lookup_via_handover,
    persist_boundary, Handover, WriterEpoch, ACCEPTED_CAP,
};
pub use hot::{
    coalesce_hot, insert_parent, partition_record, ParentTable, Partition, MAX_HOT_REPLICAS,
    PARENT_CAP,
};
pub use key::{digit, keys_equal, DIGIT_RADIX, KEY_DEPTH};
pub use outcome::QsrOutcome;
pub use roots::{bind_roots, snapshot_root, IndexRoots, SNAPSHOT_ROOT_DOMAIN};
pub use tokens::{
    admit_token, empty_in_snapshot_does_not_enumerate_neighbors, issue_token, lookup_private,
    token_reveals_degree, PrivateToken, TOKEN_DOMAIN,
};
pub use traversal::{lookup as lookup_snapshot, lookup_into, QsrSnapshot, SNAPSHOT_CAP};

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::{Generation, StrongDigest};

/// Public QSR lookup: exact cover/snapshot, then tokens, then handover.
///
/// Cover membership ([`lookup_exact`]) is not existence. [`QsrOutcome::Found`]
/// is membership, not completeness. `NativePeer::lookup_qsr` remains cover-only;
/// `NativePeer::lookup_qsr_full` calls this production path.
pub fn lookup_qsr_full(
    key: &StrongDigest,
    covers: &[CoverInterval],
    snapshot: &QsrSnapshot,
    required_generation: Generation,
    token: Option<TokenStage<'_>>,
    handover: Option<HandoverStage<'_>>,
    out: &mut [StrongDigest],
) -> Result<QsrOutcome, QdnfError> {
    Qsr::lookup(
        key,
        covers,
        snapshot,
        required_generation,
        token,
        handover,
        out,
    )
}

/// Same staged lookup as [`lookup_qsr_full`]. Snapshot-only closed-world
/// membership remains [`lookup_snapshot`] / [`lookup_into`].
pub fn lookup(
    key: &StrongDigest,
    covers: &[CoverInterval],
    snapshot: &QsrSnapshot,
    required_generation: Generation,
    token: Option<TokenStage<'_>>,
    handover: Option<HandoverStage<'_>>,
    out: &mut [StrongDigest],
) -> Result<QsrOutcome, QdnfError> {
    lookup_qsr_full(
        key,
        covers,
        snapshot,
        required_generation,
        token,
        handover,
        out,
    )
}
