//! Compatibility exports for the production paged-KV runtime.
//!
//! New code should import [`crate::inference::runtime::kv::paged`]. This thin module preserves
//! the original public path while the implementation lives in a decomposed runtime library.

pub use crate::inference::runtime::kv::paged::*;
