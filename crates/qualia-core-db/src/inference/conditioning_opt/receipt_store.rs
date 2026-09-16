//! Signed, checkpoint-bound persistence for model-precision receipts.
//!
//! This is deliberately a cold-path artifact store. A receipt cannot restore a
//! runtime contract until its signature and the activated checkpoint's SHA-256
//! both verify. It therefore records a real calibration without treating the
//! existing simulated optimizer as model evidence.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tempfile::NamedTempFile;

use super::{ModelOptimizationReceipt, ModelPrecisionContract, ModelPrecisionRegistry};
use crate::inference::conditioning::ConditioningRegistry;

pub const MODEL_OPTIMIZATION_RECEIPT_SCHEMA_VERSION: u16 = 1;
pub const MAX_MODEL_OPTIMIZATION_RECEIPT_BYTES: usize = 64 * 1024;

#[derive(Debug)]
pub enum ReceiptStoreError {
    InvalidReceipt(&'static str),
    Serialization(String),
    Io(std::io::Error),
    Signature,
    ModelDigestMismatch,
    ModelNotRegistered,
    RegistryConflict,
    ExistingArtifactConflict,
}

impl From<std::io::Error> for ReceiptStoreError {
    fn from(source: std::io::Error) -> Self {
        Self::Io(source)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignedModelOptimizationReceipt {
    pub schema_version: u16,
    /// SHA-256 of the exact GGUF/P64 checkpoint evaluated by the campaign.
    pub model_sha256: String,
    pub receipt: ModelOptimizationReceipt,
    pub contract: ModelPrecisionContract,
    /// Ed25519 verifying key, encoded as 64 lowercase hexadecimal characters.
    pub signed_by: String,
    /// Ed25519 signature over the canonical receipt payload, as lowercase hex.
    pub signature: String,
}

#[derive(Serialize)]
struct SigningPayload<'a> {
    schema_version: u16,
    model_sha256: &'a str,
    receipt: &'a ModelOptimizationReceipt,
    contract: &'a ModelPrecisionContract,
}

impl SignedModelOptimizationReceipt {
    pub fn new_signed(
        model_sha256: impl Into<String>,
        receipt: ModelOptimizationReceipt,
        contract: ModelPrecisionContract,
        signing_key: &SigningKey,
    ) -> Result<Self, ReceiptStoreError> {
        let mut signed = Self {
            schema_version: MODEL_OPTIMIZATION_RECEIPT_SCHEMA_VERSION,
            model_sha256: model_sha256.into(),
            receipt,
            contract,
            signed_by: hex::encode(signing_key.verifying_key().to_bytes()),
            signature: String::new(),
        };
        signed.validate_binding()?;
        signed.signature = hex::encode(signing_key.sign(&signed.payload_digest()?).to_bytes());
        Ok(signed)
    }

    /// Verify both the artifact signature and its binding to the model/profile
    /// lifecycle fields. This method performs no registry mutation.
    pub fn verify(&self) -> Result<(), ReceiptStoreError> {
        self.validate_binding()?;
        let public_key = decode_fixed_hex::<32>(&self.signed_by)?;
        let signature = decode_fixed_hex::<64>(&self.signature)?;
        let verifying_key =
            VerifyingKey::from_bytes(&public_key).map_err(|_| ReceiptStoreError::Signature)?;
        verifying_key
            .verify(&self.payload_digest()?, &Signature::from_bytes(&signature))
            .map_err(|_| ReceiptStoreError::Signature)
    }

    /// Stable content identity used as the artifact filename. The signature is
    /// intentionally excluded so a re-sign of the same calibration resolves to
    /// the same immutable record.
    pub fn content_identity(&self) -> Result<String, ReceiptStoreError> {
        Ok(hex::encode(self.payload_digest()?))
    }

    /// Activate a verified contract only for the exact checkpoint digest that
    /// was evaluated. The caller obtains that digest from the model file during
    /// activation; a model ID alone is never sufficient evidence.
    pub fn apply_to_registries(
        &self,
        activated_model_sha256: &str,
        models: &mut ModelPrecisionRegistry,
        conditioning: &mut ConditioningRegistry,
    ) -> Result<(), ReceiptStoreError> {
        self.verify()?;
        if activated_model_sha256 != self.model_sha256 {
            return Err(ReceiptStoreError::ModelDigestMismatch);
        }
        if models.get(&self.receipt.model_id).is_none() {
            return Err(ReceiptStoreError::ModelNotRegistered);
        }

        match conditioning.get_version(&self.contract.profile_id, self.contract.profile_version) {
            Some(existing) if existing.spec_identity == self.contract.spec_identity => {}
            Some(_) => return Err(ReceiptStoreError::RegistryConflict),
            None => conditioning
                .register(
                    &self.contract.profile_id,
                    self.contract.profile_version,
                    self.contract.spec_identity,
                    Some(format!(
                        "model-optimization-receipt:{}",
                        self.content_identity()?
                    )),
                )
                .map_err(|_| ReceiptStoreError::RegistryConflict)?,
        }
        conditioning
            .activate(&self.contract.profile_id, self.contract.profile_version)
            .map_err(|_| ReceiptStoreError::RegistryConflict)?;
        models
            .set_active_profile(&self.receipt.model_id, self.contract.profile_id.clone())
            .map_err(|_| ReceiptStoreError::ModelNotRegistered)?;
        models
            .bind_active_contract(&self.receipt.model_id, self.contract.clone())
            .map_err(|_| ReceiptStoreError::RegistryConflict)
    }

    fn validate_binding(&self) -> Result<(), ReceiptStoreError> {
        if self.schema_version != MODEL_OPTIMIZATION_RECEIPT_SCHEMA_VERSION {
            return Err(ReceiptStoreError::InvalidReceipt(
                "unsupported receipt schema",
            ));
        }
        validate_sha256(&self.model_sha256)?;
        if self.receipt.model_id.is_empty() || self.receipt.run_id.is_empty() {
            return Err(ReceiptStoreError::InvalidReceipt(
                "model and run IDs are required",
            ));
        }
        let expected_profile = format!("urn:qualia:profile:model:{}", self.receipt.model_id);
        if self.contract.profile_id != expected_profile
            || self.contract.profile_version != self.receipt.promoted_version
            || self.contract.spec_identity != self.receipt.winning_spec_identity
        {
            return Err(ReceiptStoreError::InvalidReceipt(
                "contract does not match promoted receipt lifecycle",
            ));
        }
        for score in [
            self.receipt.baseline_score,
            self.receipt.optimized_score,
            self.receipt.held_out_test_score,
        ] {
            if !score.accuracy.is_finite()
                || !score.citation_fidelity.is_finite()
                || !score.constraint_satisfaction.is_finite()
            {
                return Err(ReceiptStoreError::InvalidReceipt("scores must be finite"));
            }
        }
        if !self.receipt.delta_improvement_pct.is_finite() {
            return Err(ReceiptStoreError::InvalidReceipt(
                "improvement must be finite",
            ));
        }
        Ok(())
    }

    fn payload_digest(&self) -> Result<[u8; 32], ReceiptStoreError> {
        let payload = SigningPayload {
            schema_version: self.schema_version,
            model_sha256: &self.model_sha256,
            receipt: &self.receipt,
            contract: &self.contract,
        };
        let bytes = serde_json::to_vec(&payload)
            .map_err(|error| ReceiptStoreError::Serialization(error.to_string()))?;
        let mut hasher = Sha256::new();
        hasher.update(b"qualia:model-optimization-receipt:v1\0");
        hasher.update(bytes);
        Ok(hasher.finalize().into())
    }
}

/// Explicit-root, immutable receipt artifact store. It does not choose a
/// persistence directory on the caller's behalf.
#[derive(Debug, Clone)]
pub struct ModelOptimizationReceiptStore {
    root: PathBuf,
}

impl ModelOptimizationReceiptStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn path_for(
        &self,
        receipt: &SignedModelOptimizationReceipt,
    ) -> Result<PathBuf, ReceiptStoreError> {
        Ok(self
            .root
            .join(format!("{}.json", receipt.content_identity()?)))
    }

    /// Write once through a same-directory temporary file, sync it, then use a
    /// no-clobber promotion. Repeated writes of identical receipts are safe;
    /// mismatching content at the same identity fails closed.
    pub fn persist(
        &self,
        receipt: &SignedModelOptimizationReceipt,
    ) -> Result<PathBuf, ReceiptStoreError> {
        receipt.verify()?;
        let destination = self.path_for(receipt)?;
        if destination.exists() {
            let existing = self.load_path(&destination)?;
            return if existing == *receipt {
                Ok(destination)
            } else {
                Err(ReceiptStoreError::ExistingArtifactConflict)
            };
        }
        fs::create_dir_all(&self.root)?;
        let bytes = serde_json::to_vec_pretty(receipt)
            .map_err(|error| ReceiptStoreError::Serialization(error.to_string()))?;
        if bytes.len() > MAX_MODEL_OPTIMIZATION_RECEIPT_BYTES {
            return Err(ReceiptStoreError::InvalidReceipt(
                "receipt exceeds byte budget",
            ));
        }
        let mut staging = NamedTempFile::new_in(&self.root)?;
        staging.write_all(&bytes)?;
        staging.as_file().sync_all()?;
        match staging.persist_noclobber(&destination) {
            Ok(_) => Ok(destination),
            Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => {
                let existing = self.load_path(&destination)?;
                if existing == *receipt {
                    Ok(destination)
                } else {
                    Err(ReceiptStoreError::ExistingArtifactConflict)
                }
            }
            Err(error) => Err(ReceiptStoreError::Io(error.error)),
        }
    }

    pub fn load_path(
        &self,
        path: &Path,
    ) -> Result<SignedModelOptimizationReceipt, ReceiptStoreError> {
        let len = fs::metadata(path)?.len();
        if len > MAX_MODEL_OPTIMIZATION_RECEIPT_BYTES as u64 {
            return Err(ReceiptStoreError::InvalidReceipt(
                "receipt exceeds byte budget",
            ));
        }
        let bytes = fs::read(path)?;
        let receipt = serde_json::from_slice::<SignedModelOptimizationReceipt>(&bytes)
            .map_err(|error| ReceiptStoreError::Serialization(error.to_string()))?;
        receipt.verify()?;
        Ok(receipt)
    }
}

fn validate_sha256(value: &str) -> Result<(), ReceiptStoreError> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(ReceiptStoreError::InvalidReceipt(
            "model SHA-256 must be lowercase hex",
        ))
    }
}

fn decode_fixed_hex<const N: usize>(value: &str) -> Result<[u8; N], ReceiptStoreError> {
    let bytes = hex::decode(value).map_err(|_| ReceiptStoreError::Signature)?;
    bytes.try_into().map_err(|_| ReceiptStoreError::Signature)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inference::conditioning::{BackendCapabilities, ConditioningBudget};
    use crate::inference::conditioning_eval::TaskScore;
    use crate::inference::conditioning_opt::{
        CompressionStrategy, ModelFamily, ModelPrecisionTarget, PrefixConfiguration,
    };

    fn fixture() -> (ModelOptimizationReceipt, ModelPrecisionContract) {
        let receipt = ModelOptimizationReceipt {
            model_id: "qwen-test".into(),
            run_id: "run-1".into(),
            target_domain: "code".into(),
            baseline_variant: "B0".into(),
            winning_variant: "B5".into(),
            baseline_score: TaskScore {
                accuracy: 0.4,
                citation_fidelity: 0.2,
                constraint_satisfaction: 0.2,
            },
            optimized_score: TaskScore {
                accuracy: 0.9,
                citation_fidelity: 1.0,
                constraint_satisfaction: 1.0,
            },
            held_out_test_score: TaskScore {
                accuracy: 0.8,
                citation_fidelity: 1.0,
                constraint_satisfaction: 1.0,
            },
            delta_improvement_pct: 50.0,
            winning_spec_identity: 0xBEEF,
            promoted_version: 3,
            previous_version: Some(2),
        };
        let contract = ModelPrecisionContract::new(
            "urn:qualia:profile:model:qwen-test",
            3,
            0xBEEF,
            ConditioningBudget {
                input_tokens: 1024,
                output_tokens: 256,
                tool_rounds: 2,
                max_bytes: 4096,
            },
            PrefixConfiguration::CanonicalCacheAligned,
            CompressionStrategy::PreservePrefix,
        )
        .unwrap();
        (receipt, contract)
    }

    #[test]
    fn signed_receipt_persists_verifies_and_restores_contract() {
        let (receipt, contract) = fixture();
        let signed = SignedModelOptimizationReceipt::new_signed(
            "a".repeat(64),
            receipt,
            contract,
            &SigningKey::from_bytes(&[7; 32]),
        )
        .unwrap();
        let dir = tempfile::tempdir().unwrap();
        let store = ModelOptimizationReceiptStore::new(dir.path());
        let path = store.persist(&signed).unwrap();
        assert_eq!(store.persist(&signed).unwrap(), path);
        let loaded = store.load_path(&path).unwrap();
        assert_eq!(loaded, signed);

        let mut models = ModelPrecisionRegistry::new();
        models.register(ModelPrecisionTarget::new(
            "qwen-test",
            ModelFamily::Qwen,
            8192,
            BackendCapabilities::native_gguf_baseline(),
            "code",
        ));
        let mut conditioning = ConditioningRegistry::new();
        loaded
            .apply_to_registries(&"a".repeat(64), &mut models, &mut conditioning)
            .unwrap();
        assert!(models
            .resolve_active_contract("qwen-test", &conditioning)
            .is_some());
    }

    #[test]
    fn tampering_or_wrong_checkpoint_fails_closed() {
        let (receipt, contract) = fixture();
        let mut signed = SignedModelOptimizationReceipt::new_signed(
            "b".repeat(64),
            receipt,
            contract,
            &SigningKey::from_bytes(&[8; 32]),
        )
        .unwrap();
        signed.receipt.winning_variant = "tampered".into();
        assert!(matches!(signed.verify(), Err(ReceiptStoreError::Signature)));

        let (receipt, contract) = fixture();
        let signed = SignedModelOptimizationReceipt::new_signed(
            "b".repeat(64),
            receipt,
            contract,
            &SigningKey::from_bytes(&[8; 32]),
        )
        .unwrap();
        let mut models = ModelPrecisionRegistry::new();
        models.register(ModelPrecisionTarget::native_gguf(
            "qwen-test",
            ModelFamily::Qwen,
            8192,
            "code",
        ));
        let mut conditioning = ConditioningRegistry::new();
        assert!(matches!(
            signed.apply_to_registries(&"c".repeat(64), &mut models, &mut conditioning),
            Err(ReceiptStoreError::ModelDigestMismatch)
        ));
        assert!(conditioning
            .get_active(&signed.contract.profile_id)
            .is_none());
    }
}
