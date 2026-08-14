//! Browser Anatomy render backends.
//!
//! Pack decoding and mixer semantics remain owned by the shared renderer. This
//! directory owns only target-specific presentation of the decoded mesh.

#[cfg(target_arch = "wasm32")]
pub mod webgl2;
