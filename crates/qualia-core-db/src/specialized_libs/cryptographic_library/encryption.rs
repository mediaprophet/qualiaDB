// Part of the cryptographic_library module (split from the former mod.rs monolith
// per CLAUDE.md §11 — pure code motion, no behaviour change).
use super::*;

/// Encryption engine for data encryption
pub struct EncryptionEngine {
    encryption_algorithms: HashMap<EncryptionAlgorithm, EncryptionAlgorithmImpl>,
    decryption_algorithms: HashMap<EncryptionAlgorithm, DecryptionAlgorithmImpl>,
    key_derivation: KeyDerivation,
    performance_optimizer: EncryptionPerformanceOptimizer,
}

/// Encryption algorithm implementation
#[derive(Debug, Clone)]
pub struct EncryptionAlgorithmImpl {
    pub algorithm_id: String,
    pub algorithm: EncryptionAlgorithm,
    pub key_size: usize,
    pub iv_size: usize,
    pub tag_size: usize,
    pub parameters: EncryptionParameters,
}

/// Encryption parameters
#[derive(Debug, Clone)]
pub struct EncryptionParameters {
    pub mode: EncryptionMode,
    pub padding: Option<EncryptionPadding>,
    pub additional_data: Option<Vec<u8>>,
    pub custom_params: HashMap<String, Vec<u8>>,
}

/// Encryption modes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EncryptionMode {
    GCM,
    CCM,
    CTR,
    CBC,
    CFB,
    OFB,
    XTS,
}

/// Encryption padding
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EncryptionPadding {
    PKCS7,
    ISO10126,
    ANSIX923,
    ZeroPadding,
    NoPadding,
}

/// Decryption algorithm implementation
#[derive(Debug, Clone)]
pub struct DecryptionAlgorithmImpl {
    pub algorithm_id: String,
    pub algorithm: EncryptionAlgorithm,
    pub key_size: usize,
    pub iv_size: usize,
    pub tag_size: usize,
    pub parameters: DecryptionParameters,
}

/// Decryption parameters
#[derive(Debug, Clone)]
pub struct DecryptionParameters {
    pub mode: EncryptionMode,
    pub padding: Option<EncryptionPadding>,
    pub additional_data: Option<Vec<u8>>,
    pub custom_params: HashMap<String, Vec<u8>>,
}

/// Key derivation
pub struct KeyDerivation {
    derivation_functions: HashMap<String, DerivationFunction>,
    pub(super) derivation_parameters: DerivationParameters,
}

/// Derivation functions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DerivationFunction {
    HKDF,
    PBKDF2,
    Scrypt,
    Argon2,
    Custom(String),
}

/// Derivation parameters
#[derive(Debug, Clone)]
pub struct DerivationParameters {
    pub salt: Vec<u8>,
    pub iterations: u32,
    pub memory_cost: u32,
    pub parallelism: u32,
    pub output_length: usize,
}

/// Encryption performance optimizer
pub struct EncryptionPerformanceOptimizer {
    optimization_strategies: Vec<EncryptionOptimizationStrategy>,
    performance_metrics: EncryptionPerformanceMetrics,
}

/// Encryption optimization strategies
#[derive(Debug, Clone, PartialEq)]
pub enum EncryptionOptimizationStrategy {
    BatchEncryption,
    ParallelProcessing,
    HardwareAcceleration,
    MemoryOptimization,
    Caching,
}

/// Encryption performance metrics
#[derive(Debug, Clone)]
pub struct EncryptionPerformanceMetrics {
    pub average_encryption_time: f64,
    pub average_decryption_time: f64,
    pub throughput: f64,
    pub memory_usage: u64,
    pub cache_hit_rate: f64,
}
impl EncryptionEngine {
    pub fn new() -> Self {
        Self {
            encryption_algorithms: HashMap::new(),
            decryption_algorithms: HashMap::new(),
            key_derivation: KeyDerivation::new(),
            performance_optimizer: EncryptionPerformanceOptimizer::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        self.key_derivation.initialize()?;
        self.performance_optimizer.initialize()?;
        Ok(())
    }

    /// Register an encryption algorithm implementation.
    pub fn add_encryption_algorithm(
        &mut self,
        algorithm: EncryptionAlgorithm,
        implementation: EncryptionAlgorithmImpl,
    ) {
        self.encryption_algorithms.insert(algorithm, implementation);
    }

    /// Look up an encryption algorithm implementation.
    pub fn get_encryption_algorithm(
        &self,
        algorithm: &EncryptionAlgorithm,
    ) -> Option<&EncryptionAlgorithmImpl> {
        self.encryption_algorithms.get(algorithm)
    }

    /// Iterate over all registered encryption algorithm implementations.
    pub fn list_encryption_algorithms(&self) -> impl Iterator<Item = &EncryptionAlgorithmImpl> {
        self.encryption_algorithms.values()
    }

    /// Register a decryption algorithm implementation.
    pub fn add_decryption_algorithm(
        &mut self,
        algorithm: EncryptionAlgorithm,
        implementation: DecryptionAlgorithmImpl,
    ) {
        self.decryption_algorithms.insert(algorithm, implementation);
    }

    /// Look up a decryption algorithm implementation.
    pub fn get_decryption_algorithm(
        &self,
        algorithm: &EncryptionAlgorithm,
    ) -> Option<&DecryptionAlgorithmImpl> {
        self.decryption_algorithms.get(algorithm)
    }

    /// Iterate over all registered decryption algorithm implementations.
    pub fn list_decryption_algorithms(&self) -> impl Iterator<Item = &DecryptionAlgorithmImpl> {
        self.decryption_algorithms.values()
    }

    /// Derive key material using HKDF-SHA256 via the embedded [`KeyDerivation`] engine.
    pub fn derive_hkdf(&self, ikm: &[u8], info: &[u8]) -> Result<Vec<u8>, CryptographicError> {
        self.key_derivation.derive_hkdf(ikm, info)
    }

    pub fn encrypt_data(
        &mut self,
        key: &Key,
        data: &[u8],
        additional_data: Option<&[u8]>,
    ) -> Result<EncryptedData, CryptographicError> {
        self.encrypt_data_with(key, data, additional_data, EncryptionAlgorithm::AES256GCM)
    }

    /// Encrypt with an explicitly chosen AEAD algorithm
    /// (AES-256-GCM, ChaCha20-Poly1305, or XChaCha20-Poly1305).
    pub fn encrypt_data_with(
        &mut self,
        key: &Key,
        data: &[u8],
        additional_data: Option<&[u8]>,
        algorithm: EncryptionAlgorithm,
    ) -> Result<EncryptedData, CryptographicError> {
        let start_time = std::time::Instant::now();

        // Generate a nonce sized for the chosen algorithm
        let iv = self.generate_iv(&algorithm)?;

        // Encrypt data
        let (ciphertext, tag) =
            self.encrypt_with_key(&key, data, &iv, additional_data, &algorithm)?;

        let mode = match algorithm {
            EncryptionAlgorithm::AES256GCM => EncryptionMode::GCM,
            // ChaCha20 is a counter-mode stream cipher with a Poly1305 MAC; CTR is the
            // closest honest descriptor in the (cosmetic) EncryptionMode enum.
            _ => EncryptionMode::CTR,
        };

        let encrypted_data = EncryptedData {
            data_id: format!(
                "enc_{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            ),
            algorithm: algorithm.clone(),
            ciphertext,
            iv,
            tag,
            aad: additional_data.unwrap_or(b"").to_vec(),
            metadata: EncryptionMetadata {
                key_id: key.key_id.clone(),
                algorithm,
                mode,
                padding: Some(EncryptionPadding::NoPadding),
                created_at: start_time.elapsed().as_millis() as u64,
            },
        };

        // Record performance metrics
        self.performance_optimizer
            .record_encryption_time(start_time.elapsed().as_millis() as f64);

        Ok(encrypted_data)
    }

    pub fn decrypt_data(
        &mut self,
        key: &Key,
        encrypted_data: &EncryptedData,
    ) -> Result<Vec<u8>, CryptographicError> {
        let start_time = std::time::Instant::now();
        // Dispatch on the algorithm the ciphertext was produced with.
        let aad_ref = if encrypted_data.aad.is_empty() {
            None
        } else {
            Some(encrypted_data.aad.as_slice())
        };
        let plaintext = self.decrypt_with_key(
            &key,
            &encrypted_data.ciphertext,
            &encrypted_data.iv,
            &encrypted_data.tag,
            aad_ref,
            &encrypted_data.algorithm,
        )?;

        // Record performance metrics
        self.performance_optimizer
            .record_decryption_time(start_time.elapsed().as_millis() as f64);

        Ok(plaintext)
    }

    /// Expected nonce length in bytes for the given AEAD algorithm.
    fn nonce_len(algorithm: &EncryptionAlgorithm) -> usize {
        match algorithm {
            EncryptionAlgorithm::XChaCha20Poly1305 => 24,
            _ => 12, // AES-256-GCM and ChaCha20-Poly1305
        }
    }

    fn generate_iv(&self, algorithm: &EncryptionAlgorithm) -> Result<Vec<u8>, CryptographicError> {
        let len = Self::nonce_len(algorithm);
        let mut iv = vec![0u8; len];
        for b in iv.iter_mut() {
            *b = rand::random::<u8>();
        }
        Ok(iv)
    }

    fn encrypt_with_key(
        &self,
        key: &Key,
        data: &[u8],
        iv: &[u8],
        additional_data: Option<&[u8]>,
        algorithm: &EncryptionAlgorithm,
    ) -> Result<(Vec<u8>, Vec<u8>), CryptographicError> {
        use aead::{AeadInOut, KeyInit};
        if key.key_data.len() < 32 {
            return Err(CryptographicError::EncryptionError(
                "AEAD key must be 32 bytes".to_string(),
            ));
        }
        let expected_nonce = Self::nonce_len(algorithm);
        if iv.len() != expected_nonce {
            return Err(CryptographicError::EncryptionError(format!(
                "IV must be {expected_nonce} bytes for this algorithm"
            )));
        }
        let aad = additional_data.unwrap_or(b"");
        let mut buffer = data.to_vec();
        let tag = match algorithm {
            EncryptionAlgorithm::AES256GCM => {
                use aes_gcm::Aes256Gcm;
                let cipher = Aes256Gcm::new(
                    &aes_gcm::Key::<Aes256Gcm>::try_from(&key.key_data[..32]).unwrap(),
                );
                cipher
                    .encrypt_inout_detached(
                        &aes_gcm::Nonce::try_from(iv).unwrap(),
                        aad,
                        (&mut buffer[..]).into(),
                    )
                    .map_err(|e| CryptographicError::EncryptionError(e.to_string()))?
                    .to_vec()
            }
            EncryptionAlgorithm::ChaCha20Poly1305 => {
                use chacha20poly1305::ChaCha20Poly1305;
                let cipher = ChaCha20Poly1305::new(
                    &chacha20poly1305::Key::try_from(&key.key_data[..32]).unwrap(),
                );
                cipher
                    .encrypt_inout_detached(
                        &chacha20poly1305::Nonce::try_from(iv).unwrap(),
                        aad,
                        (&mut buffer[..]).into(),
                    )
                    .map_err(|e| CryptographicError::EncryptionError(e.to_string()))?
                    .to_vec()
            }
            EncryptionAlgorithm::XChaCha20Poly1305 => {
                use chacha20poly1305::XChaCha20Poly1305;
                let cipher = XChaCha20Poly1305::new(
                    &chacha20poly1305::Key::try_from(&key.key_data[..32]).unwrap(),
                );
                cipher
                    .encrypt_inout_detached(
                        &chacha20poly1305::XNonce::try_from(iv).unwrap(),
                        aad,
                        (&mut buffer[..]).into(),
                    )
                    .map_err(|e| CryptographicError::EncryptionError(e.to_string()))?
                    .to_vec()
            }
            EncryptionAlgorithm::Custom(name) => {
                return Err(CryptographicError::UnsupportedAlgorithm(format!(
                    "Custom cipher '{name}' not implemented"
                )));
            }
        };
        Ok((buffer, tag))
    }

    fn decrypt_with_key(
        &self,
        key: &Key,
        ciphertext: &[u8],
        iv: &[u8],
        tag: &[u8],
        additional_data: Option<&[u8]>,
        algorithm: &EncryptionAlgorithm,
    ) -> Result<Vec<u8>, CryptographicError> {
        use aead::{AeadInOut, KeyInit};
        if key.key_data.len() < 32 {
            return Err(CryptographicError::DecryptionError(
                "AEAD key must be 32 bytes".to_string(),
            ));
        }
        let expected_nonce = Self::nonce_len(algorithm);
        if iv.len() != expected_nonce {
            return Err(CryptographicError::DecryptionError(format!(
                "IV must be {expected_nonce} bytes for this algorithm"
            )));
        }
        if tag.len() != 16 {
            return Err(CryptographicError::DecryptionError(
                "AEAD tag must be 16 bytes".to_string(),
            ));
        }
        let aad = additional_data.unwrap_or(b"");
        let mut buffer = ciphertext.to_vec();
        match algorithm {
            EncryptionAlgorithm::AES256GCM => {
                use aes_gcm::Aes256Gcm;
                let cipher = Aes256Gcm::new(
                    &aes_gcm::Key::<Aes256Gcm>::try_from(&key.key_data[..32]).unwrap(),
                );
                cipher
                    .decrypt_inout_detached(
                        &aes_gcm::Nonce::try_from(iv).unwrap(),
                        aad,
                        (&mut buffer[..]).into(),
                        &aes_gcm::Tag::try_from(tag).unwrap(),
                    )
                    .map_err(|e| CryptographicError::DecryptionError(e.to_string()))?;
            }
            EncryptionAlgorithm::ChaCha20Poly1305 => {
                use chacha20poly1305::ChaCha20Poly1305;
                let cipher = ChaCha20Poly1305::new(
                    &chacha20poly1305::Key::try_from(&key.key_data[..32]).unwrap(),
                );
                cipher
                    .decrypt_inout_detached(
                        &chacha20poly1305::Nonce::try_from(iv).unwrap(),
                        aad,
                        (&mut buffer[..]).into(),
                        &chacha20poly1305::Tag::try_from(tag).unwrap(),
                    )
                    .map_err(|e| CryptographicError::DecryptionError(e.to_string()))?;
            }
            EncryptionAlgorithm::XChaCha20Poly1305 => {
                use chacha20poly1305::XChaCha20Poly1305;
                let cipher = XChaCha20Poly1305::new(
                    &chacha20poly1305::Key::try_from(&key.key_data[..32]).unwrap(),
                );
                cipher
                    .decrypt_inout_detached(
                        &chacha20poly1305::XNonce::try_from(iv).unwrap(),
                        aad,
                        (&mut buffer[..]).into(),
                        &chacha20poly1305::Tag::try_from(tag).unwrap(),
                    )
                    .map_err(|e| CryptographicError::DecryptionError(e.to_string()))?;
            }
            EncryptionAlgorithm::Custom(name) => {
                return Err(CryptographicError::UnsupportedAlgorithm(format!(
                    "Custom cipher '{name}' not implemented"
                )));
            }
        }
        Ok(buffer)
    }
}

impl KeyDerivation {
    pub fn new() -> Self {
        Self {
            derivation_functions: HashMap::new(),
            derivation_parameters: DerivationParameters {
                salt: vec![0u8; 16],
                iterations: 100000,
                memory_cost: 65536,
                parallelism: 4,
                output_length: 32,
            },
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        Ok(())
    }

    /// Register a derivation function under a named key.
    pub fn add_derivation_function(&mut self, name: String, function: DerivationFunction) {
        self.derivation_functions.insert(name, function);
    }

    /// Look up a derivation function by name.
    pub fn get_derivation_function(&self, name: &str) -> Option<&DerivationFunction> {
        self.derivation_functions.get(name)
    }

    /// Iterate over all registered derivation function names.
    pub fn list_derivation_functions(
        &self,
    ) -> impl Iterator<Item = (&String, &DerivationFunction)> {
        self.derivation_functions.iter()
    }

    /// Derive `output_length` bytes from input keying material using HKDF-SHA256.
    ///
    /// Uses the configured `derivation_parameters.salt` and `output_length`. `info`
    /// is the application-specific context/label that domain-separates derived keys.
    pub fn derive_hkdf(&self, ikm: &[u8], info: &[u8]) -> Result<Vec<u8>, CryptographicError> {
        use hkdf::Hkdf;
        use sha2::Sha256;
        let hk = Hkdf::<Sha256>::new(Some(&self.derivation_parameters.salt), ikm);
        let mut okm = vec![0u8; self.derivation_parameters.output_length];
        hk.expand(info, &mut okm)
            .map_err(|e| CryptographicError::EncryptionError(format!("HKDF expand failed: {e}")))?;
        Ok(okm)
    }
}

impl EncryptionPerformanceOptimizer {
    pub fn new() -> Self {
        Self {
            optimization_strategies: vec![
                EncryptionOptimizationStrategy::BatchEncryption,
                EncryptionOptimizationStrategy::ParallelProcessing,
            ],
            performance_metrics: EncryptionPerformanceMetrics {
                average_encryption_time: 0.0,
                average_decryption_time: 0.0,
                throughput: 0.0,
                memory_usage: 0,
                cache_hit_rate: 0.0,
            },
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        Ok(())
    }

    /// Get the configured optimization strategies.
    pub fn optimization_strategies(&self) -> &[EncryptionOptimizationStrategy] {
        &self.optimization_strategies
    }

    /// Add an optimization strategy if not already present.
    pub fn add_optimization_strategy(&mut self, strategy: EncryptionOptimizationStrategy) {
        if !self.optimization_strategies.contains(&strategy) {
            self.optimization_strategies.push(strategy);
        }
    }

    /// Record an encryption operation duration (milliseconds).
    pub fn record_encryption_time(&mut self, duration_ms: f64) {
        let m = &mut self.performance_metrics;
        if m.average_encryption_time == 0.0 {
            m.average_encryption_time = duration_ms;
        } else {
            m.average_encryption_time = 0.9 * m.average_encryption_time + 0.1 * duration_ms;
        }
        if m.average_encryption_time > 0.0 {
            m.throughput = 1000.0 / m.average_encryption_time;
        }
    }

    /// Record a decryption operation duration (milliseconds).
    pub fn record_decryption_time(&mut self, duration_ms: f64) {
        let m = &mut self.performance_metrics;
        if m.average_decryption_time == 0.0 {
            m.average_decryption_time = duration_ms;
        } else {
            m.average_decryption_time = 0.9 * m.average_decryption_time + 0.1 * duration_ms;
        }
    }

    /// Get a snapshot of the current performance metrics.
    pub fn metrics(&self) -> &EncryptionPerformanceMetrics {
        &self.performance_metrics
    }
}

impl EncryptionPerformanceMetrics {
    pub fn new() -> Self {
        Self {
            average_encryption_time: 0.0,
            average_decryption_time: 0.0,
            throughput: 0.0,
            memory_usage: 0,
            cache_hit_rate: 0.0,
        }
    }
}
