//! Webizen Identity Module for SPARQL-Star
//!
//! Implements high-priority lexicon slots and signature verification for Webizen identities.
//! Webizen IDs are sovereign actor identifiers with reserved prefix range.

use crate::NQuin;
use std::collections::HashMap;

/// Webizen ID type (64-bit with TAG_WEBIZEN prefix)
pub type WebizenId = u64;

/// TAG_WEBIZEN prefix for Webizen IDs (0x8 prefix per architectural decision)
pub const TAG_WEBIZEN: u64 = 0x8;

/// Webizen ID range: 0x8000_0000_0000_0000 to 0x8FFF_FFFF_FFFF_FFFF
pub const WEBIZEN_ID_MIN: u64 = 0x8000_0000_0000_0000;
pub const WEBIZEN_ID_MAX: u64 = 0x8FFF_FFFF_FFFF_FFFF;

/// Webizen identity record
#[repr(C)]
#[derive(Debug, Clone)]
pub struct WebizenIdentity {
    /// Webizen ID (with TAG_WEBIZEN prefix)
    pub webizen_id: WebizenId,
    /// WebID URI string hash
    pub webid_hash: u64,
    /// Ed25519 public key (32 bytes, stored as 4 u64)
    pub public_key: [u64; 4],
    /// DID hash (if using did:q42)
    pub did_hash: u64,
    /// Creation timestamp
    pub created_at: u64,
    /// Last updated timestamp
    pub updated_at: u64,
}

impl WebizenIdentity {
    /// Create a new Webizen identity
    pub fn new(webid_hash: u64, public_key: [u64; 4]) -> Self {
        let webizen_id = TAG_WEBIZEN | (webid_hash & 0x0FFF_FFFF_FFFF_FFFF);
        Self {
            webizen_id,
            webid_hash,
            public_key,
            did_hash: 0,
            created_at: 0,
            updated_at: 0,
        }
    }
    
    /// Set the DID hash
    pub fn with_did(mut self, did_hash: u64) -> Self {
        self.did_hash = did_hash;
        self
    }
    
    /// Check if an ID is a Webizen ID
    pub fn is_webizen_id(id: u64) -> bool {
        (id & 0xF000_0000_0000_0000) == (TAG_WEBIZEN << 60)
    }
}

/// Webizen identity registry
pub struct WebizenRegistry {
    /// High-priority lexicon slots for Webizens
    webizen_slots: HashMap<WebizenId, WebizenIdentity>,
    /// WebID hash to Webizen ID mapping
    webid_to_webizen: HashMap<u64, WebizenId>,
    /// Signature verification cache
    signature_cache: HashMap<(WebizenId, Vec<u8>), bool>,
}

impl WebizenRegistry {
    /// Create a new Webizen registry
    pub fn new() -> Self {
        Self {
            webizen_slots: HashMap::new(),
            webid_to_webizen: HashMap::new(),
            signature_cache: HashMap::new(),
        }
    }
    
    /// Register a Webizen identity
    pub fn register_webizen(&mut self, identity: WebizenIdentity) -> Result<(), String> {
        let webizen_id = identity.webizen_id;
        
        // Verify it's in the Webizen ID range
        if !WebizenIdentity::is_webizen_id(webizen_id) {
            return Err("ID not in Webizen range".to_string());
        }
        
        // Register in both maps
        self.webizen_slots.insert(webizen_id, identity.clone());
        self.webid_to_webizen.insert(identity.webid_hash, webizen_id);
        
        Ok(())
    }
    
    /// Get Webizen identity by ID
    pub fn get_webizen(&self, webizen_id: WebizenId) -> Option<&WebizenIdentity> {
        self.webizen_slots.get(&webizen_id)
    }
    
    /// Get Webizen ID by WebID hash
    pub fn get_webizen_by_webid(&self, webid_hash: u64) -> Option<WebizenId> {
        self.webid_to_webizen.get(&webid_hash).copied()
    }
    
    /// Verify an Ed25519 signature
    /// 
    /// Note: This is a simplified stub. Real implementation would use ed25519-dalek crate.
    pub fn verify_signature(
        &mut self,
        webizen_id: WebizenId,
        message: &[u8],
        signature: &[u8],
    ) -> Result<bool, String> {
        let identity = self.webizen_slots.get(&webizen_id)
            .ok_or_else(|| "Webizen not found".to_string())?;
        
        // Check cache
        let cache_key = (webizen_id, message.to_vec());
        if let Some(&cached) = self.signature_cache.get(&cache_key) {
            return Ok(cached);
        }

        // Repack [u64; 4] public key as [u8; 32] (little-endian)
        let mut key_bytes = [0u8; 32];
        for (i, &chunk) in identity.public_key.iter().enumerate() {
            key_bytes[i * 8..(i + 1) * 8].copy_from_slice(&chunk.to_le_bytes());
        }

        // Parse the verifying key
        use ed25519_dalek::{Signature, Verifier, VerifyingKey};
        let vk = VerifyingKey::from_bytes(&key_bytes)
            .map_err(|e| format!("Invalid public key: {e}"))?;

        // Signature must be exactly 64 bytes
        let sig_arr: [u8; 64] = signature.try_into()
            .map_err(|_| format!("Signature must be 64 bytes, got {}", signature.len()))?;
        let sig = Signature::from_bytes(&sig_arr);

        let verified = vk.verify_strict(message, &sig).is_ok();

        // Cache the result
        self.signature_cache.insert(cache_key, verified);

        Ok(verified)
    }
    
    /// Verify a Webizen signature on a NQuin
    pub fn verify_quin_signature(
        &mut self,
        quin: &NQuin,
        signature: &[u8],
    ) -> Result<bool, String> {
        // Check if the subject is a Webizen
        if !WebizenIdentity::is_webizen_id(quin.subject) {
            return Err("Subject is not a Webizen".to_string());
        }
        
        // Serialize the Quin for signing (excluding parity and signature metadata)
        let message = self.serialize_quin_for_signature(quin);
        
        self.verify_signature(quin.subject, &message, signature)
    }
    
    /// Serialize a NQuin for signature verification
    fn serialize_quin_for_signature(&self, quin: &NQuin) -> Vec<u8> {
        // Serialize subject, predicate, object, context, and metadata
        // Exclude parity as it's a checksum
        let mut bytes = Vec::with_capacity(40);
        bytes.extend_from_slice(&quin.subject.to_le_bytes());
        bytes.extend_from_slice(&quin.predicate.to_le_bytes());
        bytes.extend_from_slice(&quin.object.to_le_bytes());
        bytes.extend_from_slice(&quin.context.to_le_bytes());
        bytes.extend_from_slice(&quin.metadata.to_le_bytes());
        bytes
    }
}

impl Default for WebizenRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Webizen signature storage in Q42 format
/// Ed25519 signatures are stored as Quins with special metadata flags
#[repr(C)]
#[derive(Debug, Clone)]
pub struct WebizenSignatureStorage {
    /// Signature block ID
    pub block_id: u64,
    /// Number of signatures in this block
    pub signature_count: u32,
    /// Reserved for future use
    pub reserved: u32,
}

impl WebizenSignatureStorage {
    /// Serialize a signature to Quins
    pub fn signature_to_quins(
        webizen_id: WebizenId,
        quin_hash: u64,
        signature: &[u8; 64],
    ) -> Vec<NQuin> {
        const WEBIZEN_METADATA_FLAG: u64 = 0x1 << 63; // Use bit 63 as Webizen signature flag
        
        // Split 64-byte signature into 8 u64 values
        let sig_parts: [u64; 8] = unsafe { std::mem::transmute(*signature) };
        
        // Create Quins for each part
        let mut quins = Vec::with_capacity(8);
        for (i, part) in sig_parts.iter().enumerate() {
            quins.push(NQuin {
                subject: webizen_id,
                predicate: quin_hash,
                object: *part,
                context: i as u64, // Index in signature
                metadata: WEBIZEN_METADATA_FLAG,
                parity: 0,
            });
        }
        
        quins
    }
    
    /// Deserialize Quins to signature
    pub fn quins_to_signature(quins: &[NQuin]) -> Option<[u8; 64]> {
        const WEBIZEN_METADATA_FLAG: u64 = 0x1 << 63;
        
        // Filter Webizen signature Quins
        let sig_quins: Vec<_> = quins.iter()
            .filter(|quin| quin.metadata & WEBIZEN_METADATA_FLAG != 0)
            .collect();
        
        if sig_quins.len() != 8 {
            return None;
        }
        
        // Sort by context (index)
        let mut sorted_quins = sig_quins.clone();
        sorted_quins.sort_by_key(|q| q.context);
        
        // Reassemble signature
        let mut sig_parts: [u64; 8] = [0; 8];
        for (i, quin) in sorted_quins.iter().enumerate() {
            sig_parts[i] = quin.object;
        }
        
        Some(unsafe { std::mem::transmute(sig_parts) })
    }
}

#[cfg(test)]
mod webizen_tests {
    use super::*;

    #[test]
    fn test_webizen_id_range() {
        assert!(WebizenIdentity::is_webizen_id(WEBIZEN_ID_MIN));
        assert!(WebizenIdentity::is_webizen_id(WEBIZEN_ID_MAX));
        assert!(!WebizenIdentity::is_webizen_id(0x123456789ABCDEF0));
    }

    #[test]
    fn test_webizen_identity_creation() {
        let public_key = [1u64, 2, 3, 4];
        let identity = WebizenIdentity::new(0x123456789ABCDEF0, public_key);
        
        assert!(WebizenIdentity::is_webizen_id(identity.webizen_id));
        assert_eq!(identity.webid_hash, 0x123456789ABCDEF0);
    }

    #[test]
    fn test_registry_registration() {
        let mut registry = WebizenRegistry::new();
        let public_key = [1u64, 2, 3, 4];
        let identity = WebizenIdentity::new(0x123456789ABCDEF0, public_key);
        
        assert!(registry.register_webizen(identity).is_ok());
        assert!(registry.get_webizen(identity.webizen_id).is_some());
    }

    #[test]
    fn test_registry_webid_lookup() {
        let mut registry = WebizenRegistry::new();
        let public_key = [1u64, 2, 3, 4];
        let identity = WebizenIdentity::new(0x123456789ABCDEF0, public_key);
        let webizen_id = identity.webizen_id;
        
        registry.register_webizen(identity).unwrap();
        assert_eq!(
            registry.get_webizen_by_webid(0x123456789ABCDEF0),
            Some(webizen_id)
        );
    }

    #[test]
    fn test_signature_storage() {
        let webizen_id = TAG_WEBIZEN | 0x123456789ABCDEF0;
        let quin_hash = 0x9876543210FEDCBA;
        let signature = [1u8; 64];
        
        let quins = WebizenSignatureStorage::signature_to_quins(webizen_id, quin_hash, &signature);
        assert_eq!(quins.len(), 8);
        
        let signature_back = WebizenSignatureStorage::quins_to_signature(&quins);
        assert!(signature_back.is_some());
        assert_eq!(signature_back.unwrap(), signature);
    }

    #[test]
    fn test_verify_signature_stub() {
        let mut registry = WebizenRegistry::new();
        let public_key = [1u64, 2, 3, 4];
        let identity = WebizenIdentity::new(0x123456789ABCDEF0, public_key);
        registry.register_webizen(identity).unwrap();
        
        let message = b"test message";
        let signature = b"test signature";
        
        // Stub always returns true
        assert!(registry.verify_signature(identity.webizen_id, message, signature).is_ok());
    }
}