use dashmap::DashMap;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use std::sync::Arc;

pub struct CivicsRoutingTable {
    // Maps the 8-byte hash of the Group DID to its 32-byte Ed25519 Public Key
    trusted_groups: Arc<DashMap<[u8; 8], VerifyingKey>>,
}

impl CivicsRoutingTable {
    /// Creates a default `CivicsRoutingTable`.
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
                    let mut key_bytes = [0u8; 32];
                    let mut found_chunks = false;

                    // Look for 4 sequential 8-byte key chunks in DB for this subject
                    let chunk0_pred = crate::q_hash("q42:keyChunk0");
                    let chunk1_pred = crate::q_hash("q42:keyChunk1");
                    let chunk2_pred = crate::q_hash("q42:keyChunk2");
                    let chunk3_pred = crate::q_hash("q42:keyChunk3");

                    let mut c0 = None;
                    let mut c1 = None;
                    let mut c2 = None;
                    let mut c3 = None;

                    for q in db {
                        if q.subject == quin.subject {
                            if q.predicate == chunk0_pred { c0 = Some(q.object.to_le_bytes()); }
                            else if q.predicate == chunk1_pred { c1 = Some(q.object.to_le_bytes()); }
                            else if q.predicate == chunk2_pred { c2 = Some(q.object.to_le_bytes()); }
                            else if q.predicate == chunk3_pred { c3 = Some(q.object.to_le_bytes()); }
                        }
                    }

                    if let (Some(b0), Some(b1), Some(b2), Some(b3)) = (c0, c1, c2, c3) {
                        key_bytes[0..8].copy_from_slice(&b0);
                        key_bytes[8..16].copy_from_slice(&b1);
                        key_bytes[16..24].copy_from_slice(&b2);
                        key_bytes[24..32].copy_from_slice(&b3);
                        found_chunks = true;
                    }

                    if found_chunks {
                        if let Ok(public_key) = VerifyingKey::from_bytes(&key_bytes) {
                            self.add_trusted_group(group_hash, public_key);
                        } else {
                            log::warn!("Invalid 32-byte Ed25519 key for trusted group {:x?}", group_hash);
                        }
                    } else {
                        // Fail closed: do not fabricate a 32-byte key with zero-padding
                        log::debug!("Trusted group {:x?} has no complete 32-byte public key chunks", group_hash);
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
