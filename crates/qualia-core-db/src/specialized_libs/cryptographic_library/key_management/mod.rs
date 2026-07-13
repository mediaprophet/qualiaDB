// Part of the cryptographic_library module (split from the former mod.rs monolith
// per CLAUDE.md §11 — pure code motion, no behaviour change).
//
// `key_management` was itself becoming a >2400-line monolith, so it has been
// library-ized into single-concern submodules (CLAUDE.md §11). This module
// re-exports the full public surface so every path
// `crate::specialized_libs::cryptographic_library::key_management::<Item>`
// (and the re-export up to `cryptographic_library::<Item>`) resolves exactly as
// before. Pure code motion — no logic, signature, or behaviour change.

// Bring the grandparent (`cryptographic_library`) namespace into scope so the
// submodules' `use super::*;` transitively resolves shared items such as
// `Key`, `CryptographicError`, `MlDsaSigner`, `HashMap`, and the serde derives.
#[allow(unused_imports)]
use super::*;

mod types;
mod manager;
mod catalog;
mod search;
mod encryption_at_rest;
mod access;
mod generator;
mod rotation;
mod recovery;

pub use types::*;
pub use manager::*;
pub use catalog::*;
pub use search::*;
pub use encryption_at_rest::*;
pub use access::*;
pub use generator::*;
pub use rotation::*;
pub use recovery::*;
