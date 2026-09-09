//! Qualia Scoped Rendezvous exact lookup (QSR-A through QSR-H, E07.4–E07.8).
//!
//! Digit indexing, cover validation, closed-world snapshot traversal, tagged
//! outcomes, membership-versus-completeness evidence, scoped private tokens,
//! hot-key coalescing, writer handover, and an honest Kademlia comparison stub.
//! Authenticated lookup never returns bool. Found/EmptyInSnapshot is not
//! completeness.

mod compare;
mod completeness;
mod cover;
mod handover;
mod hot;
mod key;
mod outcome;
mod roots;
mod tokens;
mod traversal;

pub use compare::{
    kademlia_comparison_executed, local_xor_nearest, unmeasured_better_than_kademlia_claimed,
    xor_distance, LOCAL_XOR_CAP,
};
pub use completeness::{
    absence_is_authoritative, lookup_with_completeness, membership_implies_completeness,
    CompletenessEvidence, CompletenessKind, QsrAnswer,
};
pub use cover::{covers_digit, lookup_exact, validate_cover, CoverInterval};
pub use handover::{
    accept_mutation, activate_new, admit_write, drain_accepted, fence_old, persist_boundary,
    Handover, WriterEpoch, ACCEPTED_CAP,
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
pub use traversal::{lookup, lookup_into, QsrSnapshot, SNAPSHOT_CAP};
