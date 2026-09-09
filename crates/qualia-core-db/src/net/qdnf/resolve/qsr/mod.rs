//! Qualia Scoped Rendezvous exact lookup (QSR-A through QSR-D, E07.4).
//!
//! Digit indexing, cover validation, closed-world snapshot traversal, tagged
//! outcomes, and membership-versus-completeness evidence. Authenticated lookup
//! never returns bool. Found/EmptyInSnapshot is not completeness.

mod completeness;
mod cover;
mod key;
mod outcome;
mod roots;
mod traversal;

pub use completeness::{
    absence_is_authoritative, lookup_with_completeness, membership_implies_completeness,
    CompletenessEvidence, CompletenessKind, QsrAnswer,
};
pub use cover::{covers_digit, lookup_exact, validate_cover, CoverInterval};
pub use key::{digit, keys_equal, DIGIT_RADIX, KEY_DEPTH};
pub use outcome::QsrOutcome;
pub use roots::{bind_roots, snapshot_root, IndexRoots, SNAPSHOT_ROOT_DOMAIN};
pub use traversal::{lookup, lookup_into, QsrSnapshot, SNAPSHOT_CAP};
