//! `q42` category (reorg).

pub mod design_encode;
/// Attested run recipe for a native `.p64` package (layout + mode + measured knobs).
pub mod execution_profile;
/// Host GPU native capability profile (CUDA / DX12 / Vulkan / Metal tiers over WGSL floor).
pub mod machine_gpu_profile;
/// Canonical Q42 v3 model-metadata volume for converted `.p64` packages.
pub mod model_helper;
pub mod p64_weight;
pub mod q42_kvp;
#[cfg(not(target_arch = "wasm32"))]
pub mod q42_lexicon;
#[cfg(not(target_arch = "wasm32"))]
pub mod q42_reader;
#[cfg(not(target_arch = "wasm32"))]
pub mod q42_volume;
pub mod yaml_ld_q42;
