// Part of the cryptographic_library module (split from the former mod.rs monolith
// per CLAUDE.md §11 — pure code motion, no behaviour change).
use super::*;

/// Cryptographic Library Manager
pub struct CryptographicLibrary {
    pub(super) key_manager: KeyManager,
    pub(super) signature_engine: SignatureEngine,
    encryption_engine: EncryptionEngine,
    pub(super) hash_engine: HashEngine,
    proof_engine: ProofEngine,
    security_monitor: SecurityMonitor,
}
impl CryptographicLibrary {
    /// Create new cryptographic library
    pub fn new() -> Self {
        Self {
            key_manager: KeyManager::new(),
            signature_engine: SignatureEngine::new(),
            encryption_engine: EncryptionEngine::new(),
            hash_engine: HashEngine::new(),
            proof_engine: ProofEngine::new(),
            security_monitor: SecurityMonitor::new(),
        }
    }

    /// Initialize the library
    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        // Initialize key manager
        self.key_manager.initialize()?;

        // Initialize signature engine
        self.signature_engine.initialize()?;

        // Initialize encryption engine
        self.encryption_engine.initialize()?;

        // Initialize hash engine
        self.hash_engine.initialize()?;

        // Initialize proof engine
        self.proof_engine.initialize()?;

        // Initialize security monitor
        self.security_monitor.initialize()?;

        Ok(())
    }

    /// Generate ML-DSA key pair
    pub fn generate_mldsa_key_pair(
        &mut self,
        key_id: String,
        security_level: SecurityLevel,
    ) -> Result<CryptographicResult<(Key, Key)>, CryptographicError> {
        let start_time = std::time::Instant::now();

        // Generate a real FIPS-204 ML-DSA-65 key pair (public key is produced alongside
        // the secret key ΓÇö it is NOT derivable from a 32-byte seed like Ed25519).
        let (priv_k, pub_k) = MlDsaSigner::generate_keypair().map_err(|e| {
            CryptographicError::SignatureError(format!("ML-DSA keygen failed: {e}"))
        })?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let private_id = format!("{key_id}_private");
        let public_id = format!("{key_id}_public");

        let private_key = Key {
            key_id: private_id.clone(),
            key_type: KeyType::Private,
            key_algorithm: KeyAlgorithm::MLDSA,
            key_data: priv_k.sk_bytes.clone(),
            metadata: KeyMetadata {
                key_id: private_id,
                key_type: KeyType::Private,
                key_algorithm: KeyAlgorithm::MLDSA,
                key_size: priv_k.sk_bytes.len(),
                created_at: now,
                expires_at: 0,
                last_used: 0,
                usage_count: 0,
                security_level: security_level.clone(),
                access_level: AccessLevel::Secret,
            },
        };
        let public_key = Key {
            key_id: public_id.clone(),
            key_type: KeyType::Public,
            key_algorithm: KeyAlgorithm::MLDSA,
            key_data: pub_k.pk_bytes.clone(),
            metadata: KeyMetadata {
                key_id: public_id,
                key_type: KeyType::Public,
                key_algorithm: KeyAlgorithm::MLDSA,
                key_size: pub_k.pk_bytes.len(),
                created_at: now,
                expires_at: 0,
                last_used: 0,
                usage_count: 0,
                security_level: security_level.clone(),
                access_level: AccessLevel::Public,
            },
        };

        // Store keys
        self.key_manager.store_key(private_key.clone())?;
        self.key_manager.store_key(public_key.clone())?;

        // Track the KeyPair relationship in the catalog
        self.key_manager.key_storage.key_catalog.add_relationship(
            &private_key.key_id,
            &public_key.key_id,
            KeyRelationshipType::KeyPair,
        );
        self.key_manager
            .key_storage
            .key_catalog
            .register_key(private_key.metadata.clone());
        self.key_manager
            .key_storage
            .key_catalog
            .register_key(public_key.metadata.clone());

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(CryptographicResult {
            result: (private_key, public_key),
            execution_time,
            memory_usage: 0,
            security_level,
            compliance_status: ComplianceStatus::Compliant,
        })
    }

    /// Sign data with ML-DSA
    pub fn sign_data(
        &mut self,
        key_id: &str,
        data: &[u8],
    ) -> Result<CryptographicResult<Signature>, CryptographicError> {
        let start_time = std::time::Instant::now();

        // Get private key
        let private_key = self.key_manager.get_key(key_id)?;

        // Validate key type
        if private_key.key_type != KeyType::Private {
            return Err(CryptographicError::InvalidKey(
                "Key must be private for signing".to_string(),
            ));
        }

        // Sign data
        let signature = self.signature_engine.sign_data(&private_key, data)?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(CryptographicResult {
            result: signature,
            execution_time,
            memory_usage: 0,
            security_level: private_key.metadata.security_level,
            compliance_status: ComplianceStatus::Compliant,
        })
    }

    /// Verify signature with ML-DSA
    pub fn verify_signature(
        &mut self,
        key_id: &str,
        signature: &Signature,
        data: &[u8],
    ) -> Result<CryptographicResult<bool>, CryptographicError> {
        let start_time = std::time::Instant::now();

        // Get public key
        let public_key = self.key_manager.get_key(key_id)?;

        // Validate key type
        if public_key.key_type != KeyType::Public {
            return Err(CryptographicError::InvalidKey(
                "Key must be public for verification".to_string(),
            ));
        }

        // Verify signature
        let is_valid = self
            .signature_engine
            .verify_signature(&public_key, signature, data)?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(CryptographicResult {
            result: is_valid,
            execution_time,
            memory_usage: 0,
            security_level: public_key.metadata.security_level,
            compliance_status: ComplianceStatus::Compliant,
        })
    }

    /// Encrypt data with AES-256-GCM
    pub fn encrypt_data(
        &mut self,
        key_id: &str,
        data: &[u8],
        additional_data: Option<&[u8]>,
    ) -> Result<CryptographicResult<EncryptedData>, CryptographicError> {
        let start_time = std::time::Instant::now();

        // Get symmetric key
        let key = self.key_manager.get_key(key_id)?;

        // Validate key type
        if key.key_type != KeyType::Symmetric {
            return Err(CryptographicError::InvalidKey(
                "Key must be symmetric for encryption".to_string(),
            ));
        }

        // Encrypt data
        let encrypted_data = self
            .encryption_engine
            .encrypt_data(&key, data, additional_data)?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(CryptographicResult {
            result: encrypted_data,
            execution_time,
            memory_usage: 0,
            security_level: key.metadata.security_level,
            compliance_status: ComplianceStatus::Compliant,
        })
    }

    /// Encrypt data with an explicitly chosen AEAD algorithm
    /// (AES-256-GCM, ChaCha20-Poly1305, or XChaCha20-Poly1305).
    pub fn encrypt_data_with_algorithm(
        &mut self,
        key_id: &str,
        data: &[u8],
        additional_data: Option<&[u8]>,
        algorithm: EncryptionAlgorithm,
    ) -> Result<CryptographicResult<EncryptedData>, CryptographicError> {
        let start_time = std::time::Instant::now();

        let key = self.key_manager.get_key(key_id)?;
        if key.key_type != KeyType::Symmetric {
            return Err(CryptographicError::InvalidKey(
                "Key must be symmetric for encryption".to_string(),
            ));
        }

        let encrypted_data =
            self.encryption_engine
                .encrypt_data_with(&key, data, additional_data, algorithm)?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(CryptographicResult {
            result: encrypted_data,
            execution_time,
            memory_usage: 0,
            security_level: key.metadata.security_level,
            compliance_status: ComplianceStatus::Compliant,
        })
    }

    /// Decrypt data with AES-256-GCM
    pub fn decrypt_data(
        &mut self,
        key_id: &str,
        encrypted_data: &EncryptedData,
    ) -> Result<CryptographicResult<Vec<u8>>, CryptographicError> {
        let start_time = std::time::Instant::now();

        // Get symmetric key
        let key = self.key_manager.get_key(key_id)?;

        // Validate key type
        if key.key_type != KeyType::Symmetric {
            return Err(CryptographicError::InvalidKey(
                "Key must be symmetric for decryption".to_string(),
            ));
        }

        // Decrypt data
        let decrypted_data = self.encryption_engine.decrypt_data(&key, encrypted_data)?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(CryptographicResult {
            result: decrypted_data,
            execution_time,
            memory_usage: 0,
            security_level: key.metadata.security_level,
            compliance_status: ComplianceStatus::Compliant,
        })
    }

    /// Compute hash with SHA-256
    pub fn compute_hash(
        &mut self,
        data: &[u8],
    ) -> Result<CryptographicResult<HashResult>, CryptographicError> {
        let start_time = std::time::Instant::now();

        // Compute hash
        let hash_result = self.hash_engine.compute_hash("SHA256", data)?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(CryptographicResult {
            result: hash_result,
            execution_time,
            memory_usage: 0,
            security_level: SecurityLevel::High,
            compliance_status: ComplianceStatus::Compliant,
        })
    }

    /// Compute hash with BLAKE3 (32-byte digest)
    pub fn compute_hash_blake3(
        &mut self,
        data: &[u8],
    ) -> Result<CryptographicResult<HashResult>, CryptographicError> {
        let start_time = std::time::Instant::now();

        let hash_result = self.hash_engine.compute_hash("BLAKE3", data)?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(CryptographicResult {
            result: hash_result,
            execution_time,
            memory_usage: 0,
            security_level: SecurityLevel::High,
            compliance_status: ComplianceStatus::Compliant,
        })
    }

    /// Derive key material using HKDF-SHA256 (RFC 5869).
    pub fn derive_hkdf(&self, ikm: &[u8], info: &[u8]) -> Result<Vec<u8>, CryptographicError> {
        self.encryption_engine.derive_hkdf(ikm, info)
    }

    /// Issue an ML-DSA-signed Verifiable Credential via the fiduciary VC fragment layout.
    pub fn issue_vc_mldsa(
        &self,
        claim_quins: &[crate::NQuin],
        issuer_sk_key_id: &str,
        issuer_did_hash: u64,
        context: &CryptoContext,
    ) -> Result<CryptographicResult<MlDsaVcProof>, CryptographicError> {
        let start_time = std::time::Instant::now();
        let sk_key = self.key_manager.get_key(issuer_sk_key_id)?;
        if sk_key.key_type != KeyType::Private {
            return Err(CryptographicError::InvalidKey(
                "Issuer key must be private for VC issuance".to_string(),
            ));
        }
        let proof = MlDsaVcProof::issue_vc_mldsa(
            claim_quins,
            &sk_key.key_data,
            issuer_did_hash,
            context,
        )
        .map_err(|e| CryptographicError::SignatureError(format!("VC issuance failed: {e}")))?;
        let execution_time = start_time.elapsed().as_millis() as u64;
        Ok(CryptographicResult {
            result: proof,
            execution_time,
            memory_usage: 0,
            security_level: sk_key.metadata.security_level,
            compliance_status: ComplianceStatus::Compliant,
        })
    }

    /// Verify an ML-DSA-signed Verifiable Credential issued via [`Self::issue_vc_mldsa`].
    pub fn verify_vc_mldsa(
        &self,
        proof: &MlDsaVcProof,
        claim_quins: &[crate::NQuin],
        issuer_pk_key_id: &str,
        context: &CryptoContext,
    ) -> Result<CryptographicResult<bool>, CryptographicError> {
        let start_time = std::time::Instant::now();
        let pk_key = self.key_manager.get_key(issuer_pk_key_id)?;
        if pk_key.key_type != KeyType::Public {
            return Err(CryptographicError::InvalidKey(
                "Issuer key must be public for VC verification".to_string(),
            ));
        }
        let is_valid = proof
            .verify_vc_mldsa(claim_quins, &pk_key.key_data, context)
            .map_err(|e| {
                CryptographicError::SignatureError(format!("VC verification failed: {e}"))
            })?;
        let execution_time = start_time.elapsed().as_millis() as u64;
        Ok(CryptographicResult {
            result: is_valid,
            execution_time,
            memory_usage: 0,
            security_level: pk_key.metadata.security_level,
            compliance_status: ComplianceStatus::Compliant,
        })
    }

    /// Generate zero-knowledge proof
    pub fn generate_zk_proof(
        &mut self,
        circuit_id: &str,
        witness: &[Vec<u8>],
        public_inputs: &[Vec<u8>],
    ) -> Result<CryptographicResult<Proof>, CryptographicError> {
        let start_time = std::time::Instant::now();

        // Generate proof
        let proof = self
            .proof_engine
            .generate_proof(circuit_id, witness, public_inputs)?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(CryptographicResult {
            result: proof,
            execution_time,
            memory_usage: 0,
            security_level: SecurityLevel::Critical,
            compliance_status: ComplianceStatus::Compliant,
        })
    }

    /// Verify zero-knowledge proof
    pub fn verify_zk_proof(
        &mut self,
        proof: &Proof,
        public_inputs: &[Vec<u8>],
    ) -> Result<CryptographicResult<bool>, CryptographicError> {
        let start_time = std::time::Instant::now();

        // Verify proof
        let is_valid = self.proof_engine.verify_proof(proof, public_inputs)?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(CryptographicResult {
            result: is_valid,
            execution_time,
            memory_usage: 0,
            security_level: SecurityLevel::Critical,
            compliance_status: ComplianceStatus::Compliant,
        })
    }

    /// Get security metrics
    pub fn get_security_metrics(&self) -> SecurityMetrics {
        self.security_monitor.get_metrics()
    }

    /// List all keys
    pub fn list_keys(&self) -> Vec<String> {
        self.key_manager.list_keys()
    }

    /// Get key information
    pub fn get_key_info(&self, key_id: &str) -> Option<KeyMetadata> {
        self.key_manager.get_key_metadata(key_id)
    }

    /// Rotate key
    pub fn rotate_key(
        &mut self,
        key_id: &str,
    ) -> Result<CryptographicResult<Key>, CryptographicError> {
        let start_time = std::time::Instant::now();

        // Get old key
        let old_key = self.key_manager.get_key(key_id)?;

        // Generate new key
        let new_key = self.key_manager.rotate_key(&old_key)?;

        // Track the RotatedFrom relationship in the catalog
        self.key_manager.key_storage.key_catalog.add_relationship(
            &new_key.key_id,
            &old_key.key_id,
            KeyRelationshipType::RotatedFrom,
        );
        self.key_manager
            .key_storage
            .key_catalog
            .register_key(new_key.metadata.clone());

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(CryptographicResult {
            result: new_key,
            execution_time,
            memory_usage: 0,
            security_level: old_key.metadata.security_level,
            compliance_status: ComplianceStatus::Compliant,
        })
    }
}
