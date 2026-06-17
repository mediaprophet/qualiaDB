//! WGSL shader sources embedded in the Qualia WASM binary.
//!
//! - **Root** (`*.wgsl`) — U0/U1 compute (LLM, tensor query)
//! - **`viewport/`** — U2 display (ambient, projector, epistemic, screen)

pub mod viewport;