//! Independent oracles. Product APIs are never the sole check.

pub mod crypto;
pub mod graph;
pub mod wire;

pub use crypto::{independent_finished, independent_transcript_bound_ok, unkeyed_finished_defect};
pub use graph::{independent_sha384, zero_digest_is_not_identity};
pub use wire::{plaintext_on_wire, ProtectedView};
