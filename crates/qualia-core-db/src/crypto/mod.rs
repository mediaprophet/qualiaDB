//! `crypto` category (reorg).

pub mod zk_proofs;
pub mod fiduciary_crypto;
#[cfg(not(target_arch = "wasm32"))]
#[cfg(feature = "sanctuary-crypto")]
pub mod sanctuary_crypto;
#[cfg(feature = "pq-kem")]
pub mod pq_kem_shim;
#[cfg(feature = "zk-culling")]
pub mod deontic_circuit;
pub mod verifiable_credential;
