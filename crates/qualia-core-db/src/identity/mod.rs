//! `identity` category (reorg).

pub mod agency;
pub mod identifier;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
pub mod profiles;
#[cfg(not(target_arch = "wasm32"))]
pub mod key_vault;
pub mod webizen_identifiers;
pub mod vault_manifest;
pub mod credentials;
pub mod access_modality;
