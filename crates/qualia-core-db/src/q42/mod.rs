//! `q42` category — consolidated from crate-root modules (reorg).

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
pub mod q42_weight;
#[cfg(not(target_arch = "wasm32"))]
pub mod q42_volume;
#[cfg(not(target_arch = "wasm32"))]
pub mod q42_reader;
#[cfg(not(target_arch = "wasm32"))]
pub mod q42_lexicon;
pub mod yaml_ld_q42;
pub mod design_encode;
