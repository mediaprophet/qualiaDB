use dashmap::DashMap;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use std::sync::Arc;

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

    /// Hydrates trusted groups from the `.q42` database slice using the zero-allocation VM.
    pub fn hydrate_from_db(&self, db: &[crate::NQuin]) {
        let mut program = [0u8; 1024];
        // Compile a query for all Quins declaring a TrustGroup
        if crate::mini_parser::compile_ntriples_to_bytecode(
            b"?group <q42:isTrustedGroup> ?key .",
            &mut program,
        )
        .is_ok()
        {
            let mut out = vec![crate::NQuin::default(); 128]; // Stack allocation alternative for demo, bounded
            if let Ok((match_count, _)) =
                crate::webizen_bytecode::execute_program(&program, db, &mut out, None)
            {
                for quin in &out[..match_count] {
                    let group_hash = quin.subject.to_le_bytes();
                    // Derive a dummy 32-byte VerifyingKey from the object hash since NQuin is 64-bit bounded
                    // In full production, this would resolve via `did:q42` hardware pointer to the 32-byte blob.
                    let mut key_bytes = [0u8; 32];
                    key_bytes[0..8].copy_from_slice(&quin.object.to_le_bytes());
                    if let Ok(public_key) = VerifyingKey::from_bytes(&key_bytes) {
                        self.add_trusted_group(group_hash, public_key);
                    }
                }
            }
        }
    }

    /// Add a trusted Group DID to the memory cache
    pub fn add_trusted_group(&self, group_hash: [u8; 8], public_key: VerifyingKey) {
        self.trusted_groups.insert(group_hash, public_key);
    }

    /// Verifies if a semantic route is authorized by a Trusted Group Verifiable Credential.
    /// Operates in O(1) memory lookup time. Instantly drops if group is unknown.
    pub fn is_authorized(
        &self,
        group_hash: &[u8; 8],
        quin_bytes: &[u8],
        signature_bytes: &[u8; 64],
    ) -> bool {
        if let Some(public_key) = self.trusted_groups.get(group_hash) {
            // Fast-path cryptographic verification
            if let Ok(sig) = Signature::from_slice(signature_bytes) {
                return public_key.verify(quin_bytes, &sig).is_ok();
            }
        }
        false
    }
}
