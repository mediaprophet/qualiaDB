//! Dense Qwen3.5 GGUF graph adapter. Numerical primitives remain Etalon-owned.
//! Text-only llama.cpp GGUF packing (folded norms, tiled V heads, negative A).
mod bridge;
mod linear;
mod metadata;
mod plan;
mod session;
#[cfg(test)]
mod tests;
pub(crate) use metadata::Profile;
pub(crate) use session::Session;
