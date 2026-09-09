//! Qualia Scoped Rendezvous exact lookup (QSR-A through QSR-D).
//!
//! Digit indexing, cover validation, closed-world snapshot traversal, and
//! tagged outcomes. Authenticated lookup never returns bool.

mod cover;
mod key;
mod outcome;
mod traversal;

pub use cover::{covers_digit, lookup_exact, validate_cover, CoverInterval};
pub use key::{digit, keys_equal, DIGIT_RADIX, KEY_DEPTH};
pub use outcome::QsrOutcome;
pub use traversal::{lookup, lookup_into, QsrSnapshot, SNAPSHOT_CAP};
