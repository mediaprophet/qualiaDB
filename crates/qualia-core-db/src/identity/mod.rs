//! `identity` category (reorg).

pub mod access_modality;
pub mod agency;
pub mod credentials;
pub mod identifier;
#[cfg(not(target_arch = "wasm32"))]
pub mod key_vault;
pub mod profiles;
pub mod vault_manifest;
pub mod webizen_identifiers;
