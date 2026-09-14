//! `identity` category (reorg).

pub mod access_modality;
pub mod agency;
pub mod credentials;
pub mod identifier;
#[cfg(not(target_arch = "wasm32"))]
pub mod key_vault;
pub mod profiles;
pub mod vault_manifest;
pub mod webizen_did;
pub mod webizen_identifiers;

pub use webizen_did::{
    derive_concept_uuid, derive_concept_uuid_pq, format_concept_urn, format_uuid_hex,
    parse_did_webizen, quin_concept_token, verify_dual_signature, verify_pq_signature,
    WebizenDidError, WebizenDidRef, WebizenRealm,
};
