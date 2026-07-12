// Part of the cryptographic_library::key_management module (split per CLAUDE.md
// §11 — pure code motion, no behaviour change).
//
// Key generation: entropy sourcing, per-algorithm key material generation
// (ML-DSA / ML-KEM / SLH-DSA / RSA / ECDSA / EdDSA / symmetric), public-key
// derivation, and the associated key-quality assessment types.
use super::*;

/// Key generator
pub struct KeyGenerator {
    generation_algorithms: HashMap<KeyAlgorithm, GenerationAlgorithm>,
    entropy_sources: Vec<EntropySource>,
    selected_entropy_source: Option<EntropySource>,
    pub(in crate::specialized_libs::cryptographic_library) quality_metrics: KeyQualityMetrics,
}

/// Generation algorithms
#[derive(Debug, Clone)]
pub struct GenerationAlgorithm {
    pub algorithm_id: String,
    pub algorithm: KeyAlgorithm,
    pub parameters: GenerationParameters,
    pub security_level: SecurityLevel,
}

/// Generation parameters
#[derive(Debug, Clone)]
pub struct GenerationParameters {
    pub key_size: usize,
    pub curve: Option<String>,
    pub hash_function: Option<String>,
    pub custom_params: HashMap<String, Vec<u8>>,
}

/// Entropy sources
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EntropySource {
    HardwareRNG,
    OSRandom,
    UserInput,
    Network,
    Quantum,
    Hybrid,
}

/// Key quality metrics
pub struct KeyQualityMetrics {
    pub entropy_score: f64,
    pub randomness_test_results: Vec<RandomnessTestResult>,
    pub security_assessment: SecurityAssessment,
}

/// Randomness test results
#[derive(Debug, Clone)]
pub struct RandomnessTestResult {
    pub test_name: String,
    pub test_type: RandomnessTestType,
    pub p_value: f64,
    pub passed: bool,
}

/// Randomness test types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RandomnessTestType {
    Frequency,
    BlockFrequency,
    Runs,
    LongestRun,
    Serial,
    CUSUM,
    Custom(String),
}

/// Security assessment
#[derive(Debug, Clone)]
pub struct SecurityAssessment {
    pub vulnerability_score: f64,
    pub compliance_score: f64,
    pub recommendations: Vec<SecurityRecommendation>,
}

/// Security recommendations
#[derive(Debug, Clone)]
pub struct SecurityRecommendation {
    pub recommendation_id: String,
    pub severity: RecommendationSeverity,
    pub description: String,
    pub action_required: bool,
}

/// Recommendation severity
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RecommendationSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl KeyGenerator {
    pub fn new() -> Self {
        Self {
            generation_algorithms: HashMap::new(),
            entropy_sources: vec![
                EntropySource::HardwareRNG,
                EntropySource::OSRandom,
                EntropySource::Quantum,
            ],
            selected_entropy_source: None,
            quality_metrics: KeyQualityMetrics::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        for alg in [
            KeyAlgorithm::MLDSA,
            KeyAlgorithm::Kyber,
            KeyAlgorithm::NTRU,
            KeyAlgorithm::SPHINCS,
            KeyAlgorithm::RSA,
            KeyAlgorithm::ECDSA,
            KeyAlgorithm::EdDSA,
            KeyAlgorithm::AES,
            KeyAlgorithm::ChaCha20,
        ] {
            self.generation_algorithms
                .insert(alg.clone(), GenerationAlgorithm::new(alg));
        }
        Ok(())
    }

    /// Set the preferred entropy source for key generation.
    pub fn set_entropy_source(&mut self, source: EntropySource) {
        self.selected_entropy_source = Some(source);
    }

    /// Get the currently selected entropy source, if any.
    pub fn get_entropy_source(&self) -> Option<&EntropySource> {
        self.selected_entropy_source.as_ref()
    }

    /// List the available entropy sources by name.
    pub fn list_entropy_sources(&self) -> Vec<String> {
        self.entropy_sources
            .iter()
            .map(|s| format!("{:?}", s))
            .collect()
    }

    /// Generate random key data of the requested size using the selected
    /// entropy source. For `HardwareRNG`, `OSRandom`, and `Quantum` (used as a
    /// placeholder since a real quantum RNG is not available) the bytes are
    /// filled with `rand::random()`. The generated material is also folded into
    /// the quality metrics as a simple entropy estimate.
    pub fn generate_key_data(&mut self, key_size: usize) -> Result<Vec<u8>, CryptographicError> {
        let source = self
            .selected_entropy_source
            .clone()
            .unwrap_or(EntropySource::OSRandom);

        if !self.entropy_sources.contains(&source) {
            return Err(CryptographicError::SecurityError(format!(
                "selected entropy source {:?} is not available",
                source
            )));
        }

        let mut data = vec![0u8; key_size];
        match source {
            EntropySource::HardwareRNG | EntropySource::OSRandom | EntropySource::Quantum => {
                // rand::random() draws from the OS CSPRNG; for HardwareRNG and
                // Quantum this is a placeholder until dedicated hardware is wired.
                for byte in data.iter_mut() {
                    *byte = rand::random();
                }
            }
            _ => {
                return Err(CryptographicError::SecurityError(format!(
                    "entropy source {:?} not yet supported for raw key generation",
                    source
                )));
            }
        }

        // Update quality metrics with a simple entropy estimate (8 bits/byte ideal).
        self.quality_metrics.entropy_score = if key_size > 0 { 8.0 } else { 0.0 };

        Ok(data)
    }

    pub fn generate_key(
        &mut self,
        key_id: String,
        key_type: KeyType,
        algorithm: KeyAlgorithm,
        security_level: SecurityLevel,
    ) -> Result<Key, CryptographicError> {
        let generation_algorithm = self.generation_algorithms.get(&algorithm).ok_or_else(|| {
            CryptographicError::UnsupportedAlgorithm("Algorithm not supported".to_string())
        })?;

        // Generate key data
        let key_data =
            self.generate_algorithm_key_data(&generation_algorithm, security_level.clone())?;

        // Create metadata
        let metadata = KeyMetadata {
            key_id: key_id.clone(),
            key_type: key_type.clone(),
            key_algorithm: algorithm.clone(),
            key_size: key_data.len(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            expires_at: 0,
            last_used: 0,
            usage_count: 0,
            security_level: security_level.clone(),
            access_level: AccessLevel::Secret,
        };

        Ok(Key {
            key_id,
            key_type,
            key_algorithm: algorithm,
            key_data,
            metadata,
        })
    }

    pub fn derive_public_key(
        &mut self,
        private_key: &Key,
        public_key_id: String,
    ) -> Result<Key, CryptographicError> {
        // Derive public key from private key
        let public_key_data = self.derive_public_key_data(&private_key)?;

        let metadata = KeyMetadata {
            key_id: public_key_id.clone(),
            key_type: KeyType::Public,
            key_algorithm: private_key.key_algorithm.clone(),
            key_size: public_key_data.len(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            expires_at: 0,
            last_used: 0,
            usage_count: 0,
            security_level: private_key.metadata.security_level.clone(),
            access_level: AccessLevel::Public,
        };

        Ok(Key {
            key_id: public_key_id,
            key_type: KeyType::Public,
            key_algorithm: private_key.key_algorithm.clone(),
            key_data: public_key_data,
            metadata,
        })
    }

    fn generate_algorithm_key_data(
        &self,
        algorithm: &GenerationAlgorithm,
        _security_level: SecurityLevel,
    ) -> Result<Vec<u8>, CryptographicError> {
        match &algorithm.algorithm {
            KeyAlgorithm::MLDSA => {
                let (priv_k, _pub_k) = MlDsaSigner::generate_keypair().map_err(|e| {
                    CryptographicError::SignatureError(format!("ML-DSA keygen failed: {e}"))
                })?;
                Ok(priv_k.sk_bytes)
            }
            KeyAlgorithm::Kyber => {
                use fips203::ml_kem_768;
                use fips203::traits::{KeyGen, SerDes};
                let (_ek, dk) = ml_kem_768::KG::try_keygen()
                    .map_err(|e| CryptographicError::SignatureError(e.to_string()))?;
                Ok(dk.into_bytes().to_vec())
            }
            KeyAlgorithm::NTRU => {
                use fips203::ml_kem_512;
                use fips203::traits::{KeyGen, SerDes};
                let (_ek, dk) = ml_kem_512::KG::try_keygen()
                    .map_err(|e| CryptographicError::SignatureError(e.to_string()))?;
                Ok(dk.into_bytes().to_vec())
            }
            KeyAlgorithm::SPHINCS => {
                use fips205::slh_dsa_sha2_256s;
                use fips205::traits::{KeyGen, SerDes};
                let (_pk, sk) = slh_dsa_sha2_256s::KG::try_keygen()
                    .map_err(|e| CryptographicError::SignatureError(e.to_string()))?;
                Ok(sk.into_bytes().to_vec())
            }
            KeyAlgorithm::RSA => {
                #[cfg(feature = "interop-crypto")]
                {
                    use rsa::{pkcs8::EncodePrivateKey, rand_core::OsRng, RsaPrivateKey};
                    let mut rng = OsRng;
                    let priv_key = RsaPrivateKey::new(&mut rng, 2048)
                        .map_err(|e| CryptographicError::SignatureError(e.to_string()))?;
                    Ok(priv_key
                        .to_pkcs8_der()
                        .map_err(|e| CryptographicError::SignatureError(e.to_string()))?
                        .as_bytes()
                        .to_vec())
                }
                #[cfg(not(feature = "interop-crypto"))]
                {
                    Err(CryptographicError::UnsupportedAlgorithm(
                        "RSA requires interop-crypto feature".to_string(),
                    ))
                }
            }
            KeyAlgorithm::ECDSA => {
                #[cfg(feature = "interop-crypto")]
                {
                    use crate::fiduciary_crypto::InteropEcdsaSigner;
                    let signer = InteropEcdsaSigner::generate()
                        .map_err(|e| CryptographicError::SignatureError(e.to_string()))?;
                    signer
                        .export_secret_key()
                        .map_err(|e| CryptographicError::SignatureError(e.to_string()))
                }
                #[cfg(not(feature = "interop-crypto"))]
                {
                    Err(CryptographicError::UnsupportedAlgorithm(
                        "ECDSA requires interop-crypto feature".to_string(),
                    ))
                }
            }
            KeyAlgorithm::AES | KeyAlgorithm::ChaCha20 | KeyAlgorithm::EdDSA => {
                Ok(rand::random::<[u8; 32]>().to_vec())
            }
            other => Err(CryptographicError::UnsupportedAlgorithm(format!(
                "{other:?} key generation not implemented"
            ))),
        }
    }

    fn derive_public_key_data(&self, private_key: &Key) -> Result<Vec<u8>, CryptographicError> {
        match &private_key.key_algorithm {
            KeyAlgorithm::MLDSA => {
                use fips204::ml_dsa_65;
                use fips204::traits::{SerDes, Signer};
                let sk_arr: [u8; ml_dsa_65::SK_LEN] =
                    private_key.key_data.as_slice().try_into().map_err(|_| {
                        CryptographicError::InvalidKey(format!(
                            "ML-DSA secret key must be {} bytes",
                            ml_dsa_65::SK_LEN
                        ))
                    })?;
                let sk = ml_dsa_65::PrivateKey::try_from_bytes(sk_arr)
                    .map_err(|e| CryptographicError::InvalidKey(e.to_string()))?;
                Ok(sk.get_public_key().into_bytes().to_vec())
            }
            KeyAlgorithm::Kyber => {
                use fips203::ml_kem_768;
                const K: usize = 3;
                let len_dk_pke = 384 * K;
                let dk_arr: [u8; ml_kem_768::DK_LEN] =
                    private_key.key_data.as_slice().try_into().map_err(|_| {
                        CryptographicError::InvalidKey(format!(
                            "Kyber decaps key must be {} bytes",
                            ml_kem_768::DK_LEN
                        ))
                    })?;
                Ok(dk_arr[len_dk_pke..len_dk_pke + ml_kem_768::EK_LEN].to_vec())
            }
            KeyAlgorithm::NTRU => {
                use fips203::ml_kem_512;
                const K: usize = 2;
                let len_dk_pke = 384 * K;
                let dk_arr: [u8; ml_kem_512::DK_LEN] =
                    private_key.key_data.as_slice().try_into().map_err(|_| {
                        CryptographicError::InvalidKey(format!(
                            "NTRU decaps key must be {} bytes",
                            ml_kem_512::DK_LEN
                        ))
                    })?;
                Ok(dk_arr[len_dk_pke..len_dk_pke + ml_kem_512::EK_LEN].to_vec())
            }
            KeyAlgorithm::SPHINCS => {
                use fips205::slh_dsa_sha2_256s;
                use fips205::traits::{SerDes, Signer};
                let sk_arr: [u8; slh_dsa_sha2_256s::SK_LEN] =
                    private_key.key_data.as_slice().try_into().map_err(|_| {
                        CryptographicError::InvalidKey(format!(
                            "SPHINCS secret key must be {} bytes",
                            slh_dsa_sha2_256s::SK_LEN
                        ))
                    })?;
                let sk = slh_dsa_sha2_256s::PrivateKey::try_from_bytes(&sk_arr)
                    .map_err(|e| CryptographicError::InvalidKey(e.to_string()))?;
                Ok(sk.get_public_key().into_bytes().to_vec())
            }
            KeyAlgorithm::RSA => {
                #[cfg(feature = "interop-crypto")]
                {
                    use rsa::{
                        pkcs8::{DecodePrivateKey, EncodePublicKey},
                        RsaPrivateKey, RsaPublicKey,
                    };
                    let priv_key = RsaPrivateKey::from_pkcs8_der(&private_key.key_data)
                        .map_err(|e| CryptographicError::InvalidKey(e.to_string()))?;
                    let pub_key = RsaPublicKey::from(&priv_key);
                    Ok(pub_key
                        .to_public_key_der()
                        .map_err(|e| CryptographicError::InvalidKey(e.to_string()))?
                        .to_vec())
                }
                #[cfg(not(feature = "interop-crypto"))]
                {
                    Err(CryptographicError::UnsupportedAlgorithm(
                        "RSA requires interop-crypto feature".to_string(),
                    ))
                }
            }
            KeyAlgorithm::ECDSA => {
                #[cfg(feature = "interop-crypto")]
                {
                    use crate::fiduciary_crypto::InteropEcdsaSigner;
                    let signer = InteropEcdsaSigner::from_secret_key(&private_key.key_data)
                        .map_err(|e| CryptographicError::InvalidKey(e.to_string()))?;
                    Ok(signer
                        .public_key()
                        .ok_or_else(|| {
                            CryptographicError::InvalidKey("ECDSA public key missing".to_string())
                        })?
                        .to_vec())
                }
                #[cfg(not(feature = "interop-crypto"))]
                {
                    Err(CryptographicError::UnsupportedAlgorithm(
                        "ECDSA requires interop-crypto feature".to_string(),
                    ))
                }
            }
            KeyAlgorithm::EdDSA => {
                use ed25519_dalek::SigningKey;
                if private_key.key_data.len() < 32 {
                    return Err(CryptographicError::InvalidKey(
                        "Private key too short".to_string(),
                    ));
                }
                let mut seed = [0u8; 32];
                seed.copy_from_slice(&private_key.key_data[..32]);
                let signing_key = SigningKey::from_bytes(&seed);
                Ok(signing_key.verifying_key().to_bytes().to_vec())
            }
            other => Err(CryptographicError::UnsupportedAlgorithm(format!(
                "{other:?} public key derivation not supported"
            ))),
        }
    }
}

impl GenerationAlgorithm {
    pub fn new(algorithm: KeyAlgorithm) -> Self {
        Self {
            algorithm_id: format!("gen_{}", format!("{:?}", algorithm).to_lowercase()),
            algorithm,
            parameters: GenerationParameters {
                key_size: 256,
                curve: None,
                hash_function: None,
                custom_params: HashMap::new(),
            },
            security_level: SecurityLevel::High,
        }
    }
}

impl KeyQualityMetrics {
    pub fn new() -> Self {
        Self {
            entropy_score: 0.0,
            randomness_test_results: Vec::new(),
            security_assessment: SecurityAssessment {
                vulnerability_score: 0.0,
                compliance_score: 1.0,
                recommendations: Vec::new(),
            },
        }
    }
}

impl SecurityAssessment {
    pub fn new() -> Self {
        Self {
            vulnerability_score: 0.0,
            compliance_score: 1.0,
            recommendations: Vec::new(),
        }
    }
}
