//! Host bind for `qualia-cooperative-core` — ABAC delegation + work board.
//!
//! Family names are **not** `Agency.*` (`Agency.evaluate` remains Ed25519 verify).

mod codec;
mod permits;
mod work;

pub use permits::permits;
pub use work::board_project;
