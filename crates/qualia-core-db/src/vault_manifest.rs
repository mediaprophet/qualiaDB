//! Vault Manifest CBOR-LD Projection
//! 
//! This module provides CBOR-LD serialization and deserialization for Qualia vault manifests,
//! enabling compact binary transfer while maintaining semantic interoperability through Q42 lexicon.

use std::collections::HashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};

#[cfg(not(target_arch = "wasm32"))]
use crate::q42_lexicon::{Q42Context, Q42CborLdParser, SemanticPayload, CborLdError};
#[cfg(not(target_arch = "wasm32"))]
use crate::q42_volume::Q42Volume;

/// Vault manifest structure with CBOR-LD support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultManifest {
    #[serde(rename = "@context")]
    pub context: String,
    #[serde(rename = "type")]
    pub manifest_type: String,
    pub id: String,
    pub created: String,
    pub modified: String,
    pub vocabulary: VocabularyLD,
    pub collections: Vec<CollectionLD>,
    pub capabilities: Vec<CapabilityLD>,
    #[serde(rename = "did_q42")]
    pub did_q42: Option<String>,
    #[serde(rename = "semantic_context")]
    pub semantic_context: Option<u64>,
}

/// Vocabulary namespace for vault manifests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VocabularyLD {
    #[serde(rename = "@context")]
    pub context: String,
    #[serde(rename = "base_uri")]
    pub base_uri: String,
    pub prefixes: HashMap<String, String>,
    pub terms: HashMap<String, TermDefinition>,
}

/// Term definition in vocabulary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TermDefinition {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@type")]
    pub term_type: Option<String>,
    pub description: Option<String>,
    pub range: Option<String>,
    pub domain: Option<String>,
}

/// Collection definition in vault manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionLD {
    #[serde(rename = "@context")]
    pub context: String,
    #[serde(rename = "@type")]
    pub collection_type: String,
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "target_shapes")]
    pub target_shapes: Vec<String>,
    #[serde(rename = "access_mode")]
    pub access_mode: String,
    #[serde(rename = "routing_constraints")]
    pub routing_constraints: Option<u8>,
}

/// Capability definition in vault manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityLD {
    #[serde(rename = "@context")]
    pub context: String,
    #[serde(rename = "@type")]
    pub capability_type: String,
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub actions: Vec<String>,
    pub target: String,
    #[serde(rename = "routing_constraints")]
    pub routing_constraints: Option<u8>,
    #[serde(rename = "expires")]
    pub expires: Option<String>,
}

/// CBOR-LD vault manifest processor
#[cfg(not(target_arch = "wasm32"))]
pub struct VaultManifestProcessor {
    q42_context: Arc<Q42Context>,
    cbor_ld_parser: Arc<Q42CborLdParser>,
}

#[cfg(not(target_arch = "wasm32"))]
impl VaultManifestProcessor {
    /// Create new processor from Q42 volume
    pub fn from_volume(volume: &Q42Volume) -> Result<Self, CborLdError> {
        let context = Arc::new(Q42Context::from_volume(volume).map_err(|_| CborLdError::InvalidOffset)?);
        let parser = Arc::new(Q42CborLdParser::from_volume(volume).map_err(|_| CborLdError::InvalidOffset)?);
        
        Ok(Self {
            q42_context: context,
            cbor_ld_parser: parser,
        })
    }
    
    /// Convert vault manifest to CBOR-LD binary format
    pub fn to_cbor_ld(&self, manifest: &VaultManifest) -> Result<Vec<u8>, CborLdError> {
        // Ensure manifest has proper Q42 context
        let mut enhanced_manifest = manifest.clone();
        enhanced_manifest.context = "https://qualia.org/ld/vault/v1".to_string();
        
        // Serialize to CBOR-LD
        ciborium::into_writer(&enhanced_manifest, Vec::new())
            .map_err(|_| CborLdError::InvalidValueType)
    }
    
    /// Convert CBOR-LD binary to vault manifest
    pub fn from_cbor_ld(&self, cbor_bytes: &[u8]) -> Result<VaultManifest, CborLdError> {
        // Deserialize from CBOR-LD
        let manifest: VaultManifest = ciborium::from_reader(cbor_bytes)
            .map_err(|_| CborLdError::InvalidValueType)?;
        
        // Validate context
        if manifest.context != "https://qualia.org/ld/vault/v1" {
            return Err(CborLdError::InvalidUtf8);
        }
        
        Ok(manifest)
    }
    
    /// Create compact CBOR-LD projection for transfer
    pub fn to_compact_cbor_ld(&self, manifest: &VaultManifest) -> Result<Vec<u8>, CborLdError> {
        // Create compact version with only essential fields
        let compact_manifest = CompactVaultManifest::from_full(manifest);
        
        ciborium::into_writer(&compact_manifest, Vec::new())
            .map_err(|_| CborLdError::InvalidValueType)
    }
    
    /// Convert from compact CBOR-LD projection
    pub fn from_compact_cbor_ld(&self, cbor_bytes: &[u8]) -> Result<VaultManifest, CborLdError> {
        let compact: CompactVaultManifest = ciborium::from_reader(cbor_bytes)
            .map_err(|_| CborLdError::InvalidValueType)?;
        
        Ok(compact.to_full())
    }
    
    /// Validate manifest against Q42 lexicon
    pub fn validate_manifest(&self, manifest: &VaultManifest) -> Result<(), CborLdError> {
        // Check if all terms exist in Q42 lexicon
        for collection in &manifest.collections {
            if let Some(hash) = self.q42_context.resolve_semantic_term(&collection.collection_type) {
                // Term exists in lexicon
            } else {
                return Err(CborLdError::InvalidUtf8);
            }
        }
        
        for capability in &manifest.capabilities {
            if let Some(hash) = self.q42_context.resolve_semantic_term(&capability.capability_type) {
                // Term exists in lexicon
            } else {
                return Err(CborLdError::InvalidUtf8);
            }
        }
        
        Ok(())
    }
    
    /// Get Q42 context reference
    pub fn q42_context(&self) -> &Arc<Q42Context> {
        &self.q42_context
    }
}

/// Compact vault manifest for efficient transfer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactVaultManifest {
    #[serde(rename = "@context")]
    pub context: String,
    #[serde(rename = "@type")]
    pub manifest_type: String,
    pub id: String,
    pub created: String,
    pub modified: String,
    #[serde(rename = "did_q42")]
    pub did_q42: Option<String>,
    #[serde(rename = "semantic_context")]
    pub semantic_context: Option<u64>,
    // Compact collections (only essential fields)
    pub collections: Vec<CompactCollectionLD>,
    // Compact capabilities (only essential fields)
    pub capabilities: Vec<CompactCapabilityLD>,
}

/// Compact collection definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactCollectionLD {
    pub id: String,
    pub name: String,
    #[serde(rename = "target_shapes")]
    pub target_shapes: Vec<String>,
    #[serde(rename = "routing_constraints")]
    pub routing_constraints: Option<u8>,
}

/// Compact capability definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactCapabilityLD {
    pub id: String,
    pub name: String,
    pub actions: Vec<String>,
    pub target: String,
    #[serde(rename = "routing_constraints")]
    pub routing_constraints: Option<u8>,
}

impl CompactVaultManifest {
    /// Convert from full vault manifest
    pub fn from_full(full: &VaultManifest) -> Self {
        Self {
            context: full.context.clone(),
            manifest_type: full.manifest_type.clone(),
            id: full.id.clone(),
            created: full.created.clone(),
            modified: full.modified.clone(),
            did_q42: full.did_q42.clone(),
            semantic_context: full.semantic_context,
            collections: full.collections.iter()
                .map(|c| CompactCollectionLD {
                    id: c.id.clone(),
                    name: c.name.clone(),
                    target_shapes: c.target_shapes.clone(),
                    routing_constraints: c.routing_constraints,
                })
                .collect(),
            capabilities: full.capabilities.iter()
                .map(|c| CompactCapabilityLD {
                    id: c.id.clone(),
                    name: c.name.clone(),
                    actions: c.actions.clone(),
                    target: c.target.clone(),
                    routing_constraints: c.routing_constraints,
                })
                .collect(),
        }
    }
    
    /// Convert to full vault manifest
    pub fn to_full(&self) -> VaultManifest {
        VaultManifest {
            context: self.context.clone(),
            manifest_type: self.manifest_type.clone(),
            id: self.id.clone(),
            created: self.created.clone(),
            modified: self.modified.clone(),
            vocabulary: VocabularyLD {
                context: "https://qualia.org/ld/vocab/".to_string(),
                base_uri: "https://qualia.org/ld/vocab/".to_string(),
                prefixes: HashMap::new(),
                terms: HashMap::new(),
            },
            collections: self.collections.iter()
                .map(|c| CollectionLD {
                    context: "https://qualia.org/ld/vault/v1".to_string(),
                    collection_type: "Collection".to_string(),
                    id: c.id.clone(),
                    name: c.name.clone(),
                    description: None,
                    target_shapes: c.target_shapes.clone(),
                    access_mode: "read".to_string(),
                    routing_constraints: c.routing_constraints,
                })
                .collect(),
            capabilities: self.capabilities.iter()
                .map(|c| CapabilityLD {
                    context: "https://qualia.org/ld/vault/v1".to_string(),
                    capability_type: "Capability".to_string(),
                    id: c.id.clone(),
                    name: c.name.clone(),
                    description: None,
                    actions: c.actions.clone(),
                    target: c.target.clone(),
                    routing_constraints: c.routing_constraints,
                    expires: None,
                })
                .collect(),
            did_q42: self.did_q42.clone(),
            semantic_context: self.semantic_context,
        }
    }
}

impl VaultManifest {
    /// Create new vault manifest
    pub fn new(id: String) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        
        Self {
            context: "https://qualia.org/ld/vault/v1".to_string(),
            manifest_type: "VaultManifest".to_string(),
            id,
            created: now.clone(),
            modified: now,
            vocabulary: VocabularyLD {
                context: "https://qualia.org/ld/vocab/".to_string(),
                base_uri: "https://qualia.org/ld/vocab/".to_string(),
                prefixes: {
                    let mut prefixes = HashMap::new();
                    prefixes.insert("qualia".to_string(), "https://qualia.org/ld/vocab/".to_string());
                    prefixes.insert("did".to_string(), "https://www.w3.org/TR/did-core/".to_string());
                    prefixes.insert("sec".to_string(), "https://w3id.org/security/".to_string());
                    prefixes.insert("xsd".to_string(), "http://www.w3.org/2001/XMLSchema#".to_string());
                    prefixes
                },
                terms: HashMap::new(),
            },
            collections: Vec::new(),
            capabilities: Vec::new(),
            did_q42: None,
            semantic_context: None,
        }
    }
    
    /// Add collection to manifest
    pub fn add_collection(&mut self, collection: CollectionLD) {
        self.collections.push(collection);
        self.modified = chrono::Utc::now().to_rfc3339();
    }
    
    /// Add capability to manifest
    pub fn add_capability(&mut self, capability: CapabilityLD) {
        self.capabilities.push(capability);
        self.modified = chrono::Utc::now().to_rfc3339();
    }
    
    /// Set DID Q42 identifier
    pub fn set_did_q42(&mut self, did_q42: String) {
        self.did_q42 = Some(did_q42);
        self.modified = chrono::Utc::now().to_rfc3339();
    }
    
    /// Set semantic context
    pub fn set_semantic_context(&mut self, semantic_context: u64) {
        self.semantic_context = Some(semantic_context);
        self.modified = chrono::Utc::now().to_rfc3339();
    }
    
    /// Validate manifest structure
    pub fn validate(&self) -> Result<(), String> {
        if self.id.is_empty() {
            return Err("Manifest ID cannot be empty".to_string());
        }
        
        if self.created.is_empty() {
            return Err("Created timestamp cannot be empty".to_string());
        }
        
        if self.modified.is_empty() {
            return Err("Modified timestamp cannot be empty".to_string());
        }
        
        // Validate collections
        for (i, collection) in self.collections.iter().enumerate() {
            if collection.id.is_empty() {
                return Err(format!("Collection {} has empty ID", i));
            }
            if collection.name.is_empty() {
                return Err(format!("Collection {} has empty name", i));
            }
        }
        
        // Validate capabilities
        for (i, capability) in self.capabilities.iter().enumerate() {
            if capability.id.is_empty() {
                return Err(format!("Capability {} has empty ID", i));
            }
            if capability.name.is_empty() {
                return Err(format!("Capability {} has empty name", i));
            }
            if capability.target.is_empty() {
                return Err(format!("Capability {} has empty target", i));
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mini_parser::hash_token;
    use std::collections::HashMap;

    #[test]
    fn test_vault_manifest_creation() {
        let manifest = VaultManifest::new("test-vault-123".to_string());
        
        assert_eq!(manifest.id, "test-vault-123");
        assert_eq!(manifest.manifest_type, "VaultManifest");
        assert_eq!(manifest.context, "https://qualia.org/ld/vault/v1");
        assert!(!manifest.created.is_empty());
        assert!(!manifest.modified.is_empty());
        assert!(manifest.collections.is_empty());
        assert!(manifest.capabilities.is_empty());
    }

    #[test]
    fn test_compact_manifest_conversion() {
        let mut full_manifest = VaultManifest::new("test-vault-123".to_string());
        
        // Add test collection
        full_manifest.add_collection(CollectionLD {
            context: "https://qualia.org/ld/vault/v1".to_string(),
            collection_type: "Collection".to_string(),
            id: "collection-1".to_string(),
            name: "Test Collection".to_string(),
            description: Some("A test collection".to_string()),
            target_shapes: vec!["foaf:Person".to_string()],
            access_mode: "read".to_string(),
            routing_constraints: Some(0b01),
        });
        
        // Add test capability
        full_manifest.add_capability(CapabilityLD {
            context: "https://qualia.org/ld/vault/v1".to_string(),
            capability_type: "Capability".to_string(),
            id: "capability-1".to_string(),
            name: "Test Capability".to_string(),
            description: Some("A test capability".to_string()),
            actions: vec!["read".to_string(), "write".to_string()],
            target: "collection-1".to_string(),
            routing_constraints: Some(0b01),
            expires: None,
        });
        
        // Convert to compact
        let compact = CompactVaultManifest::from_full(&full_manifest);
        
        assert_eq!(compact.id, full_manifest.id);
        assert_eq!(compact.collections.len(), 1);
        assert_eq!(compact.capabilities.len(), 1);
        assert_eq!(compact.collections[0].name, "Test Collection");
        assert_eq!(compact.capabilities[0].name, "Test Capability");
        
        // Convert back to full
        let restored = compact.to_full();
        
        assert_eq!(restored.id, full_manifest.id);
        assert_eq!(restored.collections.len(), 1);
        assert_eq!(restored.capabilities.len(), 1);
        assert_eq!(restored.collections[0].name, "Test Collection");
        assert_eq!(restored.capabilities[0].name, "Test Capability");
    }

    #[test]
    fn test_manifest_validation() {
        let mut manifest = VaultManifest::new("test-vault-123".to_string());
        
        // Valid manifest
        assert!(manifest.validate().is_ok());
        
        // Invalid manifest - empty ID
        manifest.id = "".to_string();
        assert!(manifest.validate().is_err());
        
        // Fix ID
        manifest.id = "test-vault-123".to_string();
        
        // Add invalid collection
        manifest.add_collection(CollectionLD {
            context: "https://qualia.org/ld/vault/v1".to_string(),
            collection_type: "Collection".to_string(),
            id: "".to_string(), // Empty ID
            name: "Test Collection".to_string(),
            description: None,
            target_shapes: vec![],
            access_mode: "read".to_string(),
            routing_constraints: None,
        });
        
        assert!(manifest.validate().is_err());
    }
}
