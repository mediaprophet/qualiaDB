//! `q42` category (reorg).

pub mod design_encode;
/// CBOR-LD model-helper sidecar (`.q42.cbor-ld`) for converted `.p64` packages.
pub mod model_helper;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
pub mod p64_weight;
pub mod q42_kvp;
#[cfg(not(target_arch = "wasm32"))]
pub mod q42_lexicon;
#[cfg(not(target_arch = "wasm32"))]
pub mod q42_reader;
#[cfg(not(target_arch = "wasm32"))]
pub mod q42_volume;
pub mod yaml_ld_q42;
