//! WGSL shader sources embedded in the Qualia WASM binary.
//!
//! - **Root** (`*.wgsl`) — U0/U1 compute (LLM, tensor query)
//! - **`wasm/`** — copies of WASM LLM inference shaders (fused attention/FFN/transformer, GEMV, dequant, top-K, ternary, LoRA)
//! - **`viewport/`** — U2 display (ambient, projector, epistemic, screen)

pub mod viewport;
