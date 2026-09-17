//! Privacy-preserving linear algebra.
//!
//! Differential-privacy releases use caller-owned output buffers. BFV keys and
//! ciphertexts deliberately live outside the 48-byte semantic datum ABI; a
//! [`HeCiphertextRef`] is the fixed-size reference that crosses that boundary.

mod differential_privacy;

#[cfg(feature = "privacy-he")]
mod bfv;

pub use differential_privacy::{
    gaussian_sigma, CompositionMethod, DifferentialPrivacy, NoiseMechanism, NoiseSource, OsNoise,
    PrivacyAccountant, PrivacyBudget, PrivacyError,
};

#[cfg(feature = "privacy-he")]
pub use bfv::{
    decode_fixed_point_into, encode_fixed_point_into, BfvCiphertext, BfvEngine, HeCiphertextRef,
    HeError, HeScheme, MAX_BFV_PACKED_SLOTS, MAX_SERIALIZED_HE_CONTEXT_BYTES,
};

use std::sync::{Arc, Mutex};

use super::core_types::LinearAlgebraError;

/// Operations implemented by the configured HE backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum HomomorphicOperation {
    Add = 1,
    Multiply = 2,
    Rotate = 3,
}

/// Homomorphic scheme selected for exact linear algebra.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum HomomorphicKeyType {
    Bfv = 1,
}

/// Heap-free key metadata. Secret/public/evaluation key bytes stay in the HE backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HomomorphicKey {
    pub key_id_hash: u64,
    pub key_type: HomomorphicKeyType,
    pub created_at: u64,
    pub expires_at: u64,
}

/// Rotation policy for externally managed HE keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyRotationPolicy {
    pub rotation_interval: u64,
    pub max_key_age: u64,
    pub automatic_rotation: bool,
}

/// Fixed-capacity metadata registry. Key material is never copied into this table.
pub struct HomomorphicKeyManager {
    keys: [Option<HomomorphicKey>; 8],
    pub key_rotation_policy: KeyRotationPolicy,
}

impl HomomorphicKeyManager {
    pub const fn new() -> Self {
        Self {
            keys: [None; 8],
            key_rotation_policy: KeyRotationPolicy {
                rotation_interval: 86_400 * 30,
                max_key_age: 86_400 * 90,
                automatic_rotation: true,
            },
        }
    }

    pub fn register(&mut self, key: HomomorphicKey) -> Result<(), PrivacyError> {
        if let Some(existing) = self
            .keys
            .iter_mut()
            .find(|slot| slot.is_some_and(|item| item.key_id_hash == key.key_id_hash))
        {
            *existing = Some(key);
            return Ok(());
        }
        let slot = self
            .keys
            .iter_mut()
            .find(|slot| slot.is_none())
            .ok_or(PrivacyError::CapacityExceeded)?;
        *slot = Some(key);
        Ok(())
    }

    pub fn get(&self, key_id_hash: u64) -> Option<&HomomorphicKey> {
        self.keys
            .iter()
            .flatten()
            .find(|key| key.key_id_hash == key_id_hash)
    }

    pub fn remove_expired(&mut self, now_unix: u64) -> usize {
        let mut removed = 0;
        for slot in &mut self.keys {
            if slot.is_some_and(|key| key.expires_at <= now_unix) {
                *slot = None;
                removed += 1;
            }
        }
        removed
    }

    pub fn len(&self) -> usize {
        self.keys.iter().flatten().count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for HomomorphicKeyManager {
    fn default() -> Self {
        Self::new()
    }
}

/// HE capability facade. Construct [`BfvEngine`] explicitly when the feature is enabled;
/// key generation is intentionally not hidden inside `LinearAlgebraLibrary::new()`.
pub struct HomomorphicOperations {
    pub supported_operations: [HomomorphicOperation; 3],
    pub key_manager: HomomorphicKeyManager,
    pub backend_available: bool,
}

impl HomomorphicOperations {
    pub const fn new() -> Self {
        Self {
            supported_operations: [
                HomomorphicOperation::Add,
                HomomorphicOperation::Multiply,
                HomomorphicOperation::Rotate,
            ],
            key_manager: HomomorphicKeyManager::new(),
            backend_available: cfg!(feature = "privacy-he"),
        }
    }

    pub fn initialize(&mut self) -> Result<(), LinearAlgebraError> {
        self.backend_available = cfg!(feature = "privacy-he");
        Ok(())
    }
}

impl Default for HomomorphicOperations {
    fn default() -> Self {
        Self::new()
    }
}

/// Secure aggregation protocols supported by BFV packed arithmetic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AggregationProtocol {
    SecureSum = 1,
    SecureMean = 2,
}

pub struct SecureAggregation {
    pub aggregation_protocols: [AggregationProtocol; 2],
    pub privacy_budget: PrivacyBudget,
}

impl SecureAggregation {
    pub fn new() -> Self {
        Self {
            aggregation_protocols: [
                AggregationProtocol::SecureSum,
                AggregationProtocol::SecureMean,
            ],
            privacy_budget: PrivacyBudget::new_unchecked(1.0, 1e-6),
        }
    }

    pub fn initialize(&mut self) -> Result<(), LinearAlgebraError> {
        self.privacy_budget
            .validate()
            .map_err(|error| LinearAlgebraError::PrivacyError(error.to_string()))
    }
}

impl Default for SecureAggregation {
    fn default() -> Self {
        Self::new()
    }
}

/// Privacy engine for secure linear algebra.
pub struct PrivacyEngine {
    pub zk_proofs: Arc<Mutex<crate::zk_proofs::ZkProofSystem>>,
    pub homomorphic_operations: HomomorphicOperations,
    pub secure_aggregation: SecureAggregation,
    pub differential_privacy: DifferentialPrivacy,
}

impl PrivacyEngine {
    pub fn new() -> Self {
        Self {
            zk_proofs: Arc::new(Mutex::new(crate::zk_proofs::ZkProofSystem::new())),
            homomorphic_operations: HomomorphicOperations::new(),
            secure_aggregation: SecureAggregation::new(),
            differential_privacy: DifferentialPrivacy::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), LinearAlgebraError> {
        self.homomorphic_operations.initialize()?;
        self.secure_aggregation.initialize()?;
        self.differential_privacy
            .validate()
            .map_err(|error| LinearAlgebraError::PrivacyError(error.to_string()))
    }
}

impl Default for PrivacyEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_metadata_registry_is_fixed_capacity_and_expires() {
        let mut manager = HomomorphicKeyManager::new();
        for id in 0..8 {
            manager
                .register(HomomorphicKey {
                    key_id_hash: id,
                    key_type: HomomorphicKeyType::Bfv,
                    created_at: 10,
                    expires_at: 20 + id,
                })
                .unwrap();
        }
        assert_eq!(manager.len(), 8);
        assert_eq!(
            manager
                .register(HomomorphicKey {
                    key_id_hash: 99,
                    key_type: HomomorphicKeyType::Bfv,
                    created_at: 10,
                    expires_at: 100,
                })
                .unwrap_err(),
            PrivacyError::CapacityExceeded
        );
        assert_eq!(manager.remove_expired(22), 3);
        assert_eq!(manager.len(), 5);
    }

    #[test]
    fn privacy_engine_initializes_without_eager_he_key_generation() {
        let mut engine = PrivacyEngine::new();
        engine.initialize().unwrap();
        assert_eq!(
            engine.homomorphic_operations.backend_available,
            cfg!(feature = "privacy-he")
        );
    }
}
