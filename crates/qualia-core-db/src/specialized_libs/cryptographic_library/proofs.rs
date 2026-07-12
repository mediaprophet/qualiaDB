// Part of the cryptographic_library module (split from the former mod.rs monolith
// per CLAUDE.md §11 — pure code motion, no behaviour change).
use super::*;

/// Proof engine for zero-knowledge proofs
pub struct ProofEngine {
    proof_systems: HashMap<String, ProofSystem>,
    proof_storage: ProofStorage,
    verification_engine: ProofVerificationEngine,
    performance_optimizer: ProofPerformanceOptimizer,
}

/// Proof system
#[derive(Debug, Clone)]
pub struct ProofSystem {
    pub system_id: String,
    pub system_type: ProofSystemType,
    pub circuit_builder: CircuitBuilder,
    pub prover: Prover,
    pub verifier: Verifier,
}

/// Proof system types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProofSystemType {
    ZkSnarks,
    ZkStarks,
    Bulletproofs,
    SigmaProtocols,
    Custom(String),
}

/// Circuit builder
#[derive(Debug, Clone)]
pub struct CircuitBuilder {
    pub builder_id: String,
    pub circuit_type: CircuitType,
    pub constraints: Vec<CircuitConstraint>,
    pub variables: Vec<CircuitVariable>,
}

/// Circuit types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CircuitType {
    Arithmetic,
    Boolean,
    Hash,
    Signature,
    Custom(String),
}

/// Circuit constraint
#[derive(Debug, Clone)]
pub struct CircuitConstraint {
    pub constraint_id: String,
    pub constraint_type: ConstraintType,
    pub left_hand: CircuitExpression,
    pub right_hand: CircuitExpression,
}

/// Constraint types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConstraintType {
    Equality,
    Inequality,
    Boolean,
    Custom(String),
}

/// Circuit expression
#[derive(Debug, Clone)]
pub enum CircuitExpression {
    Variable(String),
    Constant(Vec<u8>),
    Add(Box<CircuitExpression>, Box<CircuitExpression>),
    Mul(Box<CircuitExpression>, Box<CircuitExpression>),
    Sub(Box<CircuitExpression>, Box<CircuitExpression>),
    Div(Box<CircuitExpression>, Box<CircuitExpression>),
}

/// Circuit variable
#[derive(Debug, Clone)]
pub struct CircuitVariable {
    pub variable_id: String,
    pub variable_type: VariableType,
    pub value: Option<Vec<u8>>,
}

/// Variable types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VariableType {
    Public,
    Private,
    Constant,
    Witness,
}

/// Prover
#[derive(Debug, Clone)]
pub struct Prover {
    pub prover_id: String,
    pub proving_key: Vec<u8>,
    pub proving_algorithm: ProvingAlgorithm,
}

/// Proving algorithms
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProvingAlgorithm {
    Groth16,
    PLONK,
    Marlin,
    Halo2,
    Custom(String),
}

/// Verifier
#[derive(Debug, Clone)]
pub struct Verifier {
    pub verifier_id: String,
    pub verification_key: Vec<u8>,
    pub verification_algorithm: VerificationAlgorithm,
}

/// Verification algorithms
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VerificationAlgorithm {
    Groth16,
    PLONK,
    Marlin,
    Halo2,
    Custom(String),
}

/// Proof storage
pub struct ProofStorage {
    proofs: HashMap<String, Proof>,
    verification_records: HashMap<String, ProofVerificationRecord>,
    audit_log: ProofAuditLog,
}

/// Proof record
#[derive(Debug, Clone)]
pub struct ProofRecord {
    pub proof_id: String,
    pub system_id: String,
    pub circuit_id: String,
    pub public_inputs: Vec<Vec<u8>>,
    pub proof_data: Vec<u8>,
    pub timestamp: u64,
    pub metadata: ProofMetadata,
}

/// Proof metadata
#[derive(Debug, Clone)]
pub struct ProofMetadata {
    pub prover_id: String,
    pub purpose: String,
    pub context: Vec<String>,
    pub validity_period: Option<(u64, u64)>,
    pub security_level: SecurityLevel,
}

/// Proof verification record
#[derive(Debug, Clone)]
pub struct ProofVerificationRecord {
    pub verification_id: String,
    pub proof_id: String,
    pub verifier_id: String,
    pub result: ProofVerificationResult,
    pub timestamp: u64,
}

/// Proof verification result
#[derive(Debug, Clone)]
pub struct ProofVerificationResult {
    pub valid: bool,
    pub error_message: Option<String>,
    pub verification_time: u64,
    pub confidence: f64,
}

/// Proof audit log
pub struct ProofAuditLog {
    entries: Vec<ProofAuditEntry>,
    retention_policy: RetentionPolicy,
}

/// Proof audit entry
#[derive(Debug, Clone)]
pub struct ProofAuditEntry {
    pub entry_id: String,
    pub timestamp: u64,
    pub proof_id: String,
    pub operation: ProofOperation,
    pub user_id: String,
    pub ip_address: String,
    pub success: bool,
}

/// Proof operations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProofOperation {
    Generate,
    Verify,
    Revoke,
    Update,
}

/// Proof verification engine
pub struct ProofVerificationEngine {
    verification_algorithms: HashMap<String, VerificationAlgorithm>,
    batch_verifier: BatchVerifier,
    performance_optimizer: VerificationPerformanceOptimizer,
}

/// Batch verifier
pub struct BatchVerifier {
    batch_size: usize,
    parallel_verification: bool,
    verification_queue: Vec<QueuedVerification>,
}

/// Queued verification
#[derive(Debug, Clone)]
pub struct QueuedVerification {
    pub verification_id: String,
    pub proof_id: String,
    pub priority: VerificationPriority,
    pub queued_at: u64,
}

/// Verification priorities
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VerificationPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Verification performance optimizer
pub struct VerificationPerformanceOptimizer {
    optimization_strategies: Vec<VerificationOptimizationStrategy>,
    performance_metrics: VerificationPerformanceMetrics,
}

/// Verification optimization strategies
#[derive(Debug, Clone, PartialEq)]
pub enum VerificationOptimizationStrategy {
    BatchVerification,
    ParallelProcessing,
    Caching,
    HardwareAcceleration,
}

/// Verification performance metrics
#[derive(Debug, Clone)]
pub struct VerificationPerformanceMetrics {
    pub average_verification_time: f64,
    pub throughput: f64,
    pub cache_hit_rate: f64,
    pub batch_efficiency: f64,
}

/// Proof performance optimizer
pub struct ProofPerformanceOptimizer {
    optimization_strategies: Vec<ProofOptimizationStrategy>,
    performance_metrics: ProofPerformanceMetrics,
}

/// Proof optimization strategies
#[derive(Debug, Clone, PartialEq)]
pub enum ProofOptimizationStrategy {
    ParallelProving,
    CircuitOptimization,
    Precomputation,
    HardwareAcceleration,
}

/// Proof performance metrics
#[derive(Debug, Clone)]
pub struct ProofPerformanceMetrics {
    pub average_proving_time: f64,
    pub average_verification_time: f64,
    pub proof_size: u64,
    pub circuit_size: u64,
    pub cache_hit_rate: f64,
}
impl ProofEngine {
    pub fn new() -> Self {
        Self {
            proof_systems: HashMap::new(),
            proof_storage: ProofStorage::new(),
            verification_engine: ProofVerificationEngine::new(),
            performance_optimizer: ProofPerformanceOptimizer::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        self.proof_storage.initialize()?;
        self.verification_engine.initialize()?;
        self.performance_optimizer.initialize()?;
        Ok(())
    }

    /// Register a proof system.
    pub fn add_proof_system(&mut self, system: ProofSystem) {
        self.proof_systems.insert(system.system_id.clone(), system);
    }

    /// Look up a proof system by id.
    pub fn get_proof_system(&self, system_id: &str) -> Option<&ProofSystem> {
        self.proof_systems.get(system_id)
    }

    /// Iterate over all registered proof systems.
    pub fn list_proof_systems(&self) -> impl Iterator<Item = &ProofSystem> {
        self.proof_systems.values()
    }

    pub fn generate_proof(
        &mut self,
        circuit_id: &str,
        witness: &[Vec<u8>],
        public_inputs: &[Vec<u8>],
    ) -> Result<Proof, CryptographicError> {
        let start_time = std::time::Instant::now();

        // Generate proof
        let proof_data = self.generate_proof_data(circuit_id, witness, public_inputs)?;

        let proof = Proof {
            proof_id: format!(
                "proof_{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            ),
            system_id: "zk_snarks".to_string(),
            circuit_id: circuit_id.to_string(),
            public_inputs: public_inputs.to_vec(),
            proof_data,
            timestamp: start_time.elapsed().as_millis() as u64,
        };

        // Store proof
        self.proof_storage.store_proof(proof.clone())?;

        // Audit log the proof generation
        self.proof_storage.audit_log.log_entry(
            &proof.proof_id,
            ProofOperation::Generate,
            "system",
            true,
        );

        // Record performance metrics
        self.performance_optimizer.record_proving_time(
            start_time.elapsed().as_millis() as f64,
            proof.proof_data.len(),
        );

        Ok(proof)
    }

    pub fn verify_proof(
        &mut self,
        proof: &Proof,
        public_inputs: &[Vec<u8>],
    ) -> Result<bool, CryptographicError> {
        let start_time = std::time::Instant::now();

        // Verify proof
        let is_valid = self.verify_proof_data(&proof.proof_data, public_inputs)?;

        // Store verification record
        let verification_record = ProofVerificationRecord {
            verification_id: format!(
                "proof_verif_{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            ),
            proof_id: proof.proof_id.clone(),
            verifier_id: "system".to_string(),
            result: ProofVerificationResult {
                valid: is_valid,
                error_message: None,
                verification_time: start_time.elapsed().as_millis() as u64,
                confidence: 1.0,
            },
            timestamp: start_time.elapsed().as_millis() as u64,
        };

        self.proof_storage
            .store_verification_record(verification_record)?;

        // Audit log the proof verification
        self.proof_storage.audit_log.log_entry(
            &proof.proof_id,
            ProofOperation::Verify,
            "system",
            is_valid,
        );

        // Record performance metrics
        self.performance_optimizer
            .record_verification_time(start_time.elapsed().as_millis() as f64);

        Ok(is_valid)
    }

    fn generate_proof_data(
        &self,
        circuit_id: &str,
        witness: &[Vec<u8>],
        public_inputs: &[Vec<u8>],
    ) -> Result<Vec<u8>, CryptographicError> {
        #[cfg(feature = "zk-culling")]
        if circuit_id == "deontic_access" {
            return Self::generate_deontic_groth16_proof(witness, public_inputs);
        }

        Self::generate_commitment_proof_data(circuit_id, witness, public_inputs)
    }

    fn verify_proof_data(
        &self,
        proof_data: &[u8],
        public_inputs: &[Vec<u8>],
    ) -> Result<bool, CryptographicError> {
        if proof_data.len() < 65 {
            return Ok(false);
        }
        if proof_data[64] == 0x02 {
            #[cfg(feature = "zk-culling")]
            {
                return Self::verify_deontic_groth16_proof(proof_data, public_inputs);
            }
            #[cfg(not(feature = "zk-culling"))]
            {
                return Ok(false);
            }
        }
        Self::verify_commitment_proof_data(proof_data, public_inputs)
    }

    fn generate_commitment_proof_data(
        circuit_id: &str,
        witness: &[Vec<u8>],
        public_inputs: &[Vec<u8>],
    ) -> Result<Vec<u8>, CryptographicError> {
        use sha2::{Digest, Sha256};
        // Commitment: H(circuit_id || witness_bytes) stored in proof_data[0..32]
        // Public input binding: H(public_inputs) stored in proof_data[32..64]
        // Proof version tag in proof_data[64..128]
        let mut witness_hasher = Sha256::new();
        witness_hasher.update(circuit_id.as_bytes());
        for w in witness {
            witness_hasher.update(w);
        }
        let witness_commit = witness_hasher.finalize();

        let mut pub_hasher = Sha256::new();
        for p in public_inputs {
            pub_hasher.update(p);
        }
        let pub_commit = pub_hasher.finalize();

        let mut proof_data = vec![0u8; 128];
        proof_data[..32].copy_from_slice(&witness_commit);
        proof_data[32..64].copy_from_slice(&pub_commit);
        // Version tag 0x01 = SHA-256 commitment stub
        proof_data[64] = 0x01;
        proof_data[65] = 0x00;
        Ok(proof_data)
    }

    fn verify_commitment_proof_data(
        proof_data: &[u8],
        public_inputs: &[Vec<u8>],
    ) -> Result<bool, CryptographicError> {
        use sha2::{Digest, Sha256};
        if proof_data.len() < 128 {
            return Ok(false);
        }
        if proof_data[64] != 0x01 {
            return Ok(false);
        }
        let mut pub_hasher = Sha256::new();
        for p in public_inputs {
            pub_hasher.update(p);
        }
        let expected_pub_commit = pub_hasher.finalize();
        Ok(&proof_data[32..64] == expected_pub_commit.as_slice())
    }

    /// Reduce a secret WITNESS value to a field element via SHA-256 (the secret
    /// never leaves the prover; only its field image enters the circuit).
    #[cfg(feature = "zk-culling")]
    fn bytes_to_fr(data: &[u8]) -> ark_bls12_381::Fr {
        use ark_ff::PrimeField;
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(data);
        ark_bls12_381::Fr::from_be_bytes_mod_order(hash.as_slice())
    }

    /// Interpret a PUBLIC input as a canonical little-endian field element — NOT
    /// hashed. Prover and verifier must agree on the exact public value (e.g. a
    /// `policy_root` the witness genuinely satisfies), so hashing it (as for a
    /// witness) would make a valid proof unconstructible. Callers serialise the
    /// field element little-endian (`Fr::into_bigint().to_bytes_le()`).
    #[cfg(feature = "zk-culling")]
    fn public_input_to_fr(data: &[u8]) -> ark_bls12_381::Fr {
        use ark_ff::PrimeField;
        ark_bls12_381::Fr::from_le_bytes_mod_order(data)
    }

    #[cfg(feature = "zk-culling")]
    fn deontic_crs() -> Result<
        &'static (
            ark_groth16::ProvingKey<ark_bls12_381::Bls12_381>,
            ark_groth16::VerifyingKey<ark_bls12_381::Bls12_381>,
        ),
        CryptographicError,
    > {
        use std::sync::OnceLock;
        static CRS: OnceLock<(
            ark_groth16::ProvingKey<ark_bls12_381::Bls12_381>,
            ark_groth16::VerifyingKey<ark_bls12_381::Bls12_381>,
        )> = OnceLock::new();
        CRS.get_or_init(|| {
            crate::deontic_circuit::generate_deontic_crs()
                .map_err(|e| CryptographicError::ProofError(e))
                .expect("deontic CRS setup")
        });
        Ok(CRS.get().expect("deontic CRS initialized"))
    }

    #[cfg(feature = "zk-culling")]
    fn generate_deontic_groth16_proof(
        witness: &[Vec<u8>],
        public_inputs: &[Vec<u8>],
    ) -> Result<Vec<u8>, CryptographicError> {
        use ark_bls12_381::Bls12_381;
        use ark_groth16::Groth16;

        use ark_serialize::CanonicalSerialize;
        use ark_snark::SNARK;
        use sha2::{Digest, Sha256};

        let user_did = Self::bytes_to_fr(witness.first().map(|v| v.as_slice()).unwrap_or(b""));
        let role_id = Self::bytes_to_fr(witness.get(1).map(|v| v.as_slice()).unwrap_or(b""));
        let action_permission =
            Self::bytes_to_fr(witness.get(2).map(|v| v.as_slice()).unwrap_or(b""));
        let policy_root =
            Self::public_input_to_fr(public_inputs.first().map(|v| v.as_slice()).unwrap_or(b""));
        let temporal_constraint =
            Self::public_input_to_fr(public_inputs.get(1).map(|v| v.as_slice()).unwrap_or(b""));

        let circuit = crate::deontic_circuit::DeonticAccessCircuit {
            user_did_commitment: Some(user_did),
            role_id: Some(role_id),
            action_permission: Some(action_permission),
            policy_root: Some(policy_root),
            temporal_constraint: Some(temporal_constraint),
        };

        let (pk, _vk) = Self::deontic_crs()?;
        let mut rng = ark_std::rand::rngs::OsRng;
        let proof = Groth16::<Bls12_381>::prove(pk, circuit, &mut rng)
            .map_err(|e| CryptographicError::ProofError(e.to_string()))?;

        let mut serialized = Vec::new();
        proof
            .serialize_uncompressed(&mut serialized)
            .map_err(|e| CryptographicError::ProofError(e.to_string()))?;

        let mut witness_hasher = Sha256::new();
        witness_hasher.update(b"deontic_access");
        for w in witness {
            witness_hasher.update(w);
        }
        let witness_commit = witness_hasher.finalize();
        let mut pub_hasher = Sha256::new();
        for p in public_inputs {
            pub_hasher.update(p);
        }
        let pub_commit = pub_hasher.finalize();

        let mut proof_data = vec![0u8; 65 + serialized.len()];
        proof_data[..32].copy_from_slice(&witness_commit);
        proof_data[32..64].copy_from_slice(&pub_commit);
        proof_data[64] = 0x02;
        proof_data[65..].copy_from_slice(&serialized);
        Ok(proof_data)
    }

    #[cfg(feature = "zk-culling")]
    fn verify_deontic_groth16_proof(
        proof_data: &[u8],
        public_inputs: &[Vec<u8>],
    ) -> Result<bool, CryptographicError> {
        use ark_bls12_381::{Bls12_381, Fr};
        use ark_groth16::{Groth16, Proof};
        use ark_serialize::CanonicalDeserialize;
        use ark_snark::SNARK;

        if proof_data.len() <= 65 {
            return Ok(false);
        }
        let proof = Proof::<Bls12_381>::deserialize_uncompressed(&proof_data[65..])
            .map_err(|e| CryptographicError::ProofError(e.to_string()))?;
        let (_pk, vk) = Self::deontic_crs()?;
        let public_fr: Vec<Fr> = public_inputs
            .iter()
            .map(|p| Self::public_input_to_fr(p))
            .collect();
        Ok(Groth16::<Bls12_381>::verify(vk, &public_fr, &proof)
            .map_err(|e| CryptographicError::ProofError(e.to_string()))?)
    }
}

impl ProofStorage {
    pub fn new() -> Self {
        Self {
            proofs: HashMap::new(),
            verification_records: HashMap::new(),
            audit_log: ProofAuditLog::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        Ok(())
    }

    pub fn store_proof(&mut self, proof: Proof) -> Result<(), CryptographicError> {
        self.proofs.insert(proof.proof_id.clone(), proof);
        Ok(())
    }

    pub fn store_verification_record(
        &mut self,
        record: ProofVerificationRecord,
    ) -> Result<(), CryptographicError> {
        self.verification_records
            .insert(record.verification_id.clone(), record);
        Ok(())
    }
}

impl ProofAuditLog {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            retention_policy: RetentionPolicy {
                retention_days: 365,
                auto_delete: true,
                archive_before_delete: true,
            },
        }
    }

    /// Record a proof operation (generate, verify, revoke, update).
    pub fn log_entry(
        &mut self,
        proof_id: &str,
        operation: ProofOperation,
        user_id: &str,
        success: bool,
    ) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let entry = ProofAuditEntry {
            entry_id: format!("proof_{}_{}", timestamp, self.entries.len()),
            timestamp,
            proof_id: proof_id.to_string(),
            operation,
            user_id: user_id.to_string(),
            ip_address: String::new(),
            success,
        };
        self.entries.push(entry);
        let cutoff =
            timestamp.saturating_sub((self.retention_policy.retention_days as u64) * 86400);
        self.entries.retain(|e| e.timestamp >= cutoff);
    }

    /// Number of logged entries.
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Iterate over entries.
    pub fn entries(&self) -> &[ProofAuditEntry] {
        &self.entries
    }
}

impl ProofVerificationEngine {
    pub fn new() -> Self {
        Self {
            verification_algorithms: HashMap::new(),
            batch_verifier: BatchVerifier::new(),
            performance_optimizer: VerificationPerformanceOptimizer::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        self.performance_optimizer.initialize()?;
        Ok(())
    }

    /// Register a verification algorithm under a named key.
    pub fn add_verification_algorithm(&mut self, name: String, algorithm: VerificationAlgorithm) {
        self.verification_algorithms.insert(name, algorithm);
    }

    /// Look up a verification algorithm by name.
    pub fn get_verification_algorithm(&self, name: &str) -> Option<&VerificationAlgorithm> {
        self.verification_algorithms.get(name)
    }

    /// Iterate over all registered verification algorithms.
    pub fn list_verification_algorithms(&self) -> impl Iterator<Item = &VerificationAlgorithm> {
        self.verification_algorithms.values()
    }

    /// Get a reference to the batch verifier.
    pub fn batch_verifier(&self) -> &BatchVerifier {
        &self.batch_verifier
    }

    /// Get a mutable reference to the batch verifier.
    pub fn batch_verifier_mut(&mut self) -> &mut BatchVerifier {
        &mut self.batch_verifier
    }
}

impl BatchVerifier {
    pub fn new() -> Self {
        Self {
            batch_size: 100,
            parallel_verification: true,
            verification_queue: Vec::new(),
        }
    }

    /// Get the configured batch size.
    pub fn batch_size(&self) -> usize {
        self.batch_size
    }

    /// Set the batch size.
    pub fn set_batch_size(&mut self, size: usize) {
        self.batch_size = size;
    }

    /// Whether parallel verification is enabled.
    pub fn parallel_verification(&self) -> bool {
        self.parallel_verification
    }

    /// Enable or disable parallel verification.
    pub fn set_parallel_verification(&mut self, enabled: bool) {
        self.parallel_verification = enabled;
    }

    /// Enqueue a verification for batch processing.
    pub fn enqueue_verification(&mut self, verification: QueuedVerification) {
        self.verification_queue.push(verification);
    }

    /// Dequeue the next verification (FIFO order).
    pub fn dequeue_verification(&mut self) -> Option<QueuedVerification> {
        if self.verification_queue.is_empty() {
            None
        } else {
            Some(self.verification_queue.remove(0))
        }
    }

    /// Number of verifications currently queued.
    pub fn queue_len(&self) -> usize {
        self.verification_queue.len()
    }
}

impl VerificationPerformanceOptimizer {
    pub fn new() -> Self {
        Self {
            optimization_strategies: vec![
                VerificationOptimizationStrategy::BatchVerification,
                VerificationOptimizationStrategy::ParallelProcessing,
            ],
            performance_metrics: VerificationPerformanceMetrics {
                average_verification_time: 0.0,
                throughput: 0.0,
                cache_hit_rate: 0.0,
                batch_efficiency: 0.0,
            },
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        Ok(())
    }

    /// Get the configured optimization strategies.
    pub fn optimization_strategies(&self) -> &[VerificationOptimizationStrategy] {
        &self.optimization_strategies
    }

    /// Add an optimization strategy if not already present.
    pub fn add_optimization_strategy(&mut self, strategy: VerificationOptimizationStrategy) {
        if !self.optimization_strategies.contains(&strategy) {
            self.optimization_strategies.push(strategy);
        }
    }

    /// Record a verification duration (milliseconds) and update running averages.
    pub fn record_verification_time(&mut self, duration_ms: f64) {
        let m = &mut self.performance_metrics;
        if m.average_verification_time == 0.0 {
            m.average_verification_time = duration_ms;
        } else {
            m.average_verification_time = 0.9 * m.average_verification_time + 0.1 * duration_ms;
        }
        if m.average_verification_time > 0.0 {
            m.throughput = 1000.0 / m.average_verification_time;
        }
    }

    /// Get a snapshot of the current performance metrics.
    pub fn metrics(&self) -> &VerificationPerformanceMetrics {
        &self.performance_metrics
    }
}

impl VerificationPerformanceMetrics {
    pub fn new() -> Self {
        Self {
            average_verification_time: 0.0,
            throughput: 0.0,
            cache_hit_rate: 0.0,
            batch_efficiency: 0.0,
        }
    }
}

impl ProofPerformanceOptimizer {
    pub fn new() -> Self {
        Self {
            optimization_strategies: vec![
                ProofOptimizationStrategy::ParallelProving,
                ProofOptimizationStrategy::CircuitOptimization,
            ],
            performance_metrics: ProofPerformanceMetrics {
                average_proving_time: 0.0,
                average_verification_time: 0.0,
                proof_size: 0,
                circuit_size: 0,
                cache_hit_rate: 0.0,
            },
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        Ok(())
    }

    /// Get the configured optimization strategies.
    pub fn optimization_strategies(&self) -> &[ProofOptimizationStrategy] {
        &self.optimization_strategies
    }

    /// Add an optimization strategy if not already present.
    pub fn add_optimization_strategy(&mut self, strategy: ProofOptimizationStrategy) {
        if !self.optimization_strategies.contains(&strategy) {
            self.optimization_strategies.push(strategy);
        }
    }

    /// Record a proof generation duration (milliseconds).
    pub fn record_proving_time(&mut self, duration_ms: f64, proof_size: usize) {
        let m = &mut self.performance_metrics;
        if m.average_proving_time == 0.0 {
            m.average_proving_time = duration_ms;
        } else {
            m.average_proving_time = 0.9 * m.average_proving_time + 0.1 * duration_ms;
        }
        m.proof_size = proof_size as u64;
    }

    /// Record a proof verification duration (milliseconds).
    pub fn record_verification_time(&mut self, duration_ms: f64) {
        let m = &mut self.performance_metrics;
        if m.average_verification_time == 0.0 {
            m.average_verification_time = duration_ms;
        } else {
            m.average_verification_time = 0.9 * m.average_verification_time + 0.1 * duration_ms;
        }
    }

    /// Get a snapshot of the current performance metrics.
    pub fn metrics(&self) -> &ProofPerformanceMetrics {
        &self.performance_metrics
    }
}

impl ProofPerformanceMetrics {
    pub fn new() -> Self {
        Self {
            average_proving_time: 0.0,
            average_verification_time: 0.0,
            proof_size: 0,
            circuit_size: 0,
            cache_hit_rate: 0.0,
        }
    }
}
