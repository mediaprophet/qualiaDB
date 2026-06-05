use std::sync::Arc;
use dashmap::DashMap;
use ed25519_dalek::{VerifyingKey, Signature, Verifier};

pub struct CivicsRoutingTable {
    // Maps the 8-byte hash of the Group DID to its 32-byte Ed25519 Public Key
    trusted_groups: Arc<DashMap<[u8; 8], VerifyingKey>>,
}

impl CivicsRoutingTable {
    pub fn new() -> Self {
        Self {
            trusted_groups: Arc::new(DashMap::new()),
        }
    }

    /// Add a trusted Group DID to the memory cache
    pub fn add_trusted_group(&self, group_hash: [u8; 8], public_key: VerifyingKey) {
        self.trusted_groups.insert(group_hash, public_key);
    }

    /// Verifies if a semantic route is authorized by a Trusted Group Verifiable Credential.
    /// Operates in O(1) memory lookup time. Instantly drops if group is unknown.
    pub fn is_authorized(&self, group_hash: &[u8; 8], quin_bytes: &[u8], signature_bytes: &[u8; 64]) -> bool {
        if let Some(public_key) = self.trusted_groups.get(group_hash) {
            // Fast-path cryptographic verification
            if let Ok(sig) = Signature::from_slice(signature_bytes) {
                return public_key.verify(quin_bytes, &sig).is_ok();
            }
        }
        false
    }
}
