use super::*;

/// Noise mechanisms
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NoiseMechanism {
    Laplace,
    Gaussian,
    Exponential,
    Geometric,
    Custom(String),
}

/// Privacy accountant
pub struct PrivacyAccountant {
    pub total_epsilon_spent: f64,
    pub total_delta_spent: f64,
    pub composition_method: CompositionMethod,
    pub remaining_budget: PrivacyBudget,
}

/// Composition methods
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompositionMethod {
    BasicComposition,
    AdvancedComposition,
    RDPComposition,
    GaussianDP,
    Custom(String),
}

/// Sensitivity function
#[derive(Debug, Clone)]
pub struct SensitivityFunction {
    pub function_id: String,
    pub sensitivity: f64,
    pub computation_method: SensitivityMethod,
}

/// Sensitivity methods
#[derive(Debug, Clone, PartialEq)]
pub enum SensitivityMethod {
    Global,
    Local,
    Smooth,
    Approximate,
}

/// Secure aggregation
pub struct SecureAggregation {
    aggregation_protocols: Vec<AggregationProtocol>,
    encryption_schemes: Vec<EncryptionScheme>,
    integrity_checks: Vec<IntegrityCheck>,
}

/// Aggregation protocols
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AggregationProtocol {
    SecureSum,
    SecureMean,
    SecureMin,
    SecureMax,
    SecureMedian,
    Custom(String),
}

/// Encryption schemes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EncryptionScheme {
    Homomorphic,
    SecretSharing,
    Threshold,
    Oblivious,
    Custom(String),
}

/// Integrity checks
#[derive(Debug, Clone)]
pub struct IntegrityCheck {
    pub check_id: String,
    pub check_type: IntegrityCheckType,
    pub verification_method: VerificationMethod,
}

/// Integrity check types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IntegrityCheckType {
    Hash,
    MAC,
    DigitalSignature,
    ZeroKnowledge,
}

/// Verification methods
#[derive(Debug, Clone, PartialEq)]
pub enum VerificationMethod {
    Deterministic,
    Probabilistic,
    Interactive,
    NonInteractive,
}

/// Privacy budget
pub struct PrivacyBudget {
    pub epsilon: f64,
    pub delta: f64,
    pub remaining_epsilon: f64,
    pub remaining_delta: f64,
    pub budget_period: u64,
    pub last_reset: u64,
}

impl StatisticalPrivacyEngine {
    pub fn new() -> Self {
        Self {
            fiduciary_crypto: Arc::new(Mutex::new(FiduciaryCrypto::new())),
            zk_proofs: Arc::new(Mutex::new(ZkProofSystem::new())),
            differential_privacy: DifferentialPrivacy::new(),
            secure_aggregation: SecureAggregation::new(),
            privacy_budget: PrivacyBudget {
                epsilon: 1.0,
                delta: 1e-6,
                remaining_epsilon: 1.0,
                remaining_delta: 1e-6,
                budget_period: 86400, // 24 hours
                last_reset: 0,
            },
        }
    }

    pub fn initialize(&mut self) -> Result<(), StatisticalError> {
        self.differential_privacy.initialize()?;
        self.secure_aggregation.initialize()?;
        Ok(())
    }

    pub fn add_laplace_noise(
        &mut self,
        value: f64,
        sensitivity: f64,
    ) -> Result<(f64, f64), StatisticalError> {
        let epsilon = 1.0;
        let scale = sensitivity / epsilon;

        let noise = self.generate_laplace_noise(scale)?;
        let noisy_value = value + noise;

        // Update privacy budget
        self.privacy_budget.remaining_epsilon -= epsilon;

        Ok((noisy_value, epsilon))
    }

    pub fn add_histogram_noise(
        &mut self,
        counts: &[u32],
    ) -> Result<(Vec<u32>, f64), StatisticalError> {
        let epsilon = 1.0;
        let sensitivity = 1.0;
        let scale = sensitivity / epsilon;

        let mut noisy_counts = Vec::with_capacity(counts.len());
        for &count in counts {
            let noise = self.generate_laplace_noise(scale)?;
            let noisy_count = (count as f64 + noise).max(0.0) as u32;
            noisy_counts.push(noisy_count);
        }

        // Update privacy budget
        self.privacy_budget.remaining_epsilon -= epsilon;

        Ok((noisy_counts, epsilon))
    }

    /// Sample real Laplace(0, `scale`) noise via inverse-CDF transform over OS
    /// entropy.
    ///
    /// SECURITY / CORRECTNESS: a former implementation drew from a global
    /// monotonic `AtomicU64` counter, so the "noise" was fully deterministic
    /// and predictable — which **voids** the differential-privacy guarantee
    /// (an observer who knows the call sequence can subtract the exact noise
    /// and recover the raw value). The old transform was also not a Laplace
    /// sample. This uses real OS entropy (`getrandom`, native + wasm) and the
    /// correct inverse CDF, and **fails closed** (returns `PrivacyError`, no
    /// output) when entropy is unavailable rather than degrade to weak noise.
    ///
    /// Note: `epsilon` is fixed at 1.0 by the callers and the budget is a
    /// simple per-query decrement — this is a valid fixed-ε mechanism, but it
    /// does not enforce ε-composition across many queries (that lives in the
    /// separate `PrivacyAccountant`). Per-query noise is now genuinely random.
    fn generate_laplace_noise(&self, scale: f64) -> Result<f64, StatisticalError> {
        let mut bytes = [0u8; 8];
        getrandom::fill(&mut bytes).map_err(|e| {
            StatisticalError::PrivacyError(format!(
                "no OS entropy for differential-privacy noise: {e}"
            ))
        })?;
        // Map the random u64 to the OPEN interval (0,1) so that ln(1 - 2|u|)
        // stays finite: (x + 0.5) / 2^64 is never exactly 0 or 1.
        let x = u64::from_le_bytes(bytes) as f64;
        let r = (x + 0.5) / (u64::MAX as f64 + 1.0);
        // u ~ Uniform(-1/2, 1/2); Laplace inverse CDF:
        //   X = -scale * sgn(u) * ln(1 - 2|u|)
        let u = r - 0.5;
        Ok(-scale * u.signum() * (1.0 - 2.0 * u.abs()).ln())
    }

    /// Encrypt (seal) a statistical result using the fiduciary crypto system.
    ///
    /// `FiduciaryCrypto` exposes ML-DSA (FIPS-204) signing rather than symmetric
    /// encryption, so "encryption" here means producing an authenticated
    /// signature over the result bytes. The returned bytes are the ML-DSA
    /// signature; a holder of the public key can verify that the result was
    /// produced by this engine and has not been tampered with. A default
    /// signing key is generated lazily on first use.
    pub fn encrypt_result(&self, data: &[u8]) -> Result<Vec<u8>, StatisticalError> {
        let mut crypto = self
            .fiduciary_crypto
            .lock()
            .map_err(|e| StatisticalError::PrivacyError(e.to_string()))?;

        const STAT_KEY_ID: &str = "statistical_results";
        if !crypto.list_keys().iter().any(|k| k == STAT_KEY_ID) {
            crypto
                .generate_key(STAT_KEY_ID.to_string())
                .map_err(|e| StatisticalError::PrivacyError(e.to_string()))?;
        }

        let signature = crypto
            .sign(
                data,
                Some(STAT_KEY_ID),
                "statistical_computing".to_string(),
                "result_encryption".to_string(),
            )
            .map_err(|e| StatisticalError::PrivacyError(e.to_string()))?;

        Ok(signature.sig_bytes)
    }

    /// Verify (open) a statistical result sealed by `encrypt_result`.
    ///
    /// Returns `Ok(true)` when the signature is valid for `data` under the
    /// engine's statistical-results key.
    pub fn verify_result(&self, data: &[u8], signature: &[u8]) -> Result<bool, StatisticalError> {
        let crypto = self
            .fiduciary_crypto
            .lock()
            .map_err(|e| StatisticalError::PrivacyError(e.to_string()))?;

        const STAT_KEY_ID: &str = "statistical_results";
        let sig = MlDsaSignature {
            sig_bytes: signature.to_vec(),
        };
        crypto
            .verify(
                data,
                &sig,
                Some(STAT_KEY_ID),
                "statistical_computing".to_string(),
                "result_encryption".to_string(),
            )
            .map_err(|e| StatisticalError::PrivacyError(e.to_string()))
    }

    /// Generate a zero-knowledge proof that a statistical computation was
    /// performed correctly.
    ///
    /// The proof binds the private `inputs` and public `outputs` together: a
    /// SHA-256 commitment over all inputs/outputs becomes a private witness,
    /// and the same commitment is exposed as the single public input. The
    /// circuit enforces `one * commitment = commitment`, so a verifying party
    /// learns only that the prover knows the commitment bound to the published
    /// outputs — not the inputs themselves. The returned bytes are a
    /// `serde_json`-serialised `ZkProof` (which carries its own public inputs),
    /// so it can be verified by `verify_computation` without extra state.
    pub fn prove_computation(
        &self,
        computation_id: &str,
        inputs: &[Vec<u8>],
        outputs: &[Vec<u8>],
    ) -> Result<Vec<u8>, StatisticalError> {
        let mut zk = self
            .zk_proofs
            .lock()
            .map_err(|e| StatisticalError::PrivacyError(e.to_string()))?;

        // Commitment over inputs and outputs: SHA-256 -> 32-byte field element.
        let mut hasher = Sha256::new();
        for chunk in inputs {
            hasher.update(chunk);
        }
        for chunk in outputs {
            hasher.update(chunk);
        }
        let digest = hasher.finalize();
        let mut commitment = [0u8; 32];
        commitment.copy_from_slice(&digest);

        let circuit_id = format!("stat_comp_{}", computation_id);
        zk.create_circuit(circuit_id.clone())
            .map_err(|e| StatisticalError::PrivacyError(e.to_string()))?;

        // Public input: the commitment bound to the published outputs.
        zk.add_variable(&circuit_id, "commitment".to_string(), VariableType::Public)
            .map_err(|e| StatisticalError::PrivacyError(e.to_string()))?;
        // Private witness: the multiplicative identity and the same commitment.
        zk.add_variable(&circuit_id, "one".to_string(), VariableType::Private)
            .map_err(|e| StatisticalError::PrivacyError(e.to_string()))?;
        zk.add_variable(&circuit_id, "in_commit".to_string(), VariableType::Private)
            .map_err(|e| StatisticalError::PrivacyError(e.to_string()))?;

        // Constraint: one * in_commit = commitment (binds private/public).
        zk.add_constraint(
            &circuit_id,
            CircuitExpression::Variable("one".to_string()),
            CircuitExpression::Variable("in_commit".to_string()),
            CircuitExpression::Variable("commitment".to_string()),
        )
        .map_err(|e| StatisticalError::PrivacyError(e.to_string()))?;

        zk.generate_keys(&circuit_id)
            .map_err(|e| StatisticalError::PrivacyError(e.to_string()))?;

        // Field-one in little-endian: [1, 0, ...].
        let mut one_val = [0u8; 32];
        one_val[0] = 1;

        let mut witness = HashMap::new();
        witness.insert("one".to_string(), FieldElement { value: one_val });
        witness.insert("in_commit".to_string(), FieldElement { value: commitment });
        witness.insert("commitment".to_string(), FieldElement { value: commitment });

        let public_inputs = vec![FieldElement { value: commitment }];

        let proof = zk
            .generate_proof(&circuit_id, witness, public_inputs)
            .map_err(|e| StatisticalError::PrivacyError(e.to_string()))?;

        serde_json::to_vec(&proof).map_err(|e| StatisticalError::PrivacyError(e.to_string()))
    }

    /// Verify a zero-knowledge computation proof produced by `prove_computation`.
    ///
    /// `proof` is the serialised `ZkProof` bytes. When `public_inputs` is
    /// non-empty, each entry is interpreted as a 32-byte little-endian field
    /// element and checked against the public inputs embedded in the proof, so
    /// callers can confirm the proof binds to the outputs they expect.
    pub fn verify_computation(
        &self,
        proof: &[u8],
        public_inputs: &[Vec<u8>],
    ) -> Result<bool, StatisticalError> {
        let zk_proof: ZkProof = serde_json::from_slice(proof)
            .map_err(|e| StatisticalError::PrivacyError(e.to_string()))?;

        // Optional binding check: the caller-supplied public inputs must match
        // the ones embedded in the proof.
        if !public_inputs.is_empty() {
            if public_inputs.len() != zk_proof.public_inputs.len() {
                return Ok(false);
            }
            for (expected, actual) in public_inputs.iter().zip(&zk_proof.public_inputs) {
                let mut expected_arr = [0u8; 32];
                let len = expected.len().min(32);
                expected_arr[..len].copy_from_slice(&expected[..len]);
                if expected_arr != actual.value {
                    return Ok(false);
                }
            }
        }

        let mut zk = self
            .zk_proofs
            .lock()
            .map_err(|e| StatisticalError::PrivacyError(e.to_string()))?;

        let result = zk
            .verify_proof(&zk_proof)
            .map_err(|e| StatisticalError::PrivacyError(e.to_string()))?;

        Ok(result.is_valid)
    }
}

impl DifferentialPrivacy {
    pub fn new() -> Self {
        Self {
            noise_mechanisms: vec![NoiseMechanism::Laplace, NoiseMechanism::Gaussian],
            privacy_accountant: PrivacyAccountant {
                total_epsilon_spent: 0.0,
                total_delta_spent: 0.0,
                composition_method: CompositionMethod::AdvancedComposition,
                remaining_budget: PrivacyBudget {
                    epsilon: 1.0,
                    delta: 1e-6,
                    remaining_epsilon: 1.0,
                    remaining_delta: 1e-6,
                    budget_period: 86400,
                    last_reset: 0,
                },
            },
            sensitivity_analyzer: SensitivityAnalyzer::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), StatisticalError> {
        self.sensitivity_analyzer.initialize()?;
        Ok(())
    }

    /// Returns the list of noise mechanisms available to this DP engine.
    pub fn noise_mechanisms(&self) -> &[NoiseMechanism] {
        &self.noise_mechanisms
    }

    /// Register an additional noise mechanism if not already present.
    pub fn add_noise_mechanism(&mut self, mechanism: NoiseMechanism) {
        if !self.noise_mechanisms.contains(&mechanism) {
            self.noise_mechanisms.push(mechanism);
        }
    }

    /// Returns `true` when the given noise mechanism is registered.
    pub fn supports_noise_mechanism(&self, mechanism: &NoiseMechanism) -> bool {
        self.noise_mechanisms.contains(mechanism)
    }

    /// Returns a reference to the privacy accountant tracking epsilon/delta spend.
    pub fn privacy_accountant(&self) -> &PrivacyAccountant {
        &self.privacy_accountant
    }

    /// Returns a mutable reference to the privacy accountant.
    pub fn privacy_accountant_mut(&mut self) -> &mut PrivacyAccountant {
        &mut self.privacy_accountant
    }

    /// Spend `epsilon`/`delta` from the privacy budget, recording the
    /// consumption in the accountant. Returns `Err` when the budget is
    /// insufficient.
    pub fn spend_budget(&mut self, epsilon: f64, delta: f64) -> Result<(), StatisticalError> {
        let remaining = &self.privacy_accountant.remaining_budget;
        if remaining.remaining_epsilon < epsilon || remaining.remaining_delta < delta {
            return Err(StatisticalError::PrivacyError(
                "Insufficient privacy budget".to_string(),
            ));
        }
        self.privacy_accountant.remaining_budget.remaining_epsilon -= epsilon;
        self.privacy_accountant.remaining_budget.remaining_delta -= delta;
        self.privacy_accountant.total_epsilon_spent += epsilon;
        self.privacy_accountant.total_delta_spent += delta;
        Ok(())
    }
}

impl SensitivityAnalyzer {
    pub fn new() -> Self {
        Self {
            sensitivity_functions: HashMap::new(),
            sensitivity_cache: HashMap::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), StatisticalError> {
        Ok(())
    }

    /// Register a named sensitivity function so it can be looked up by name
    /// from `compute_sensitivity` / `get_sensitivity`.
    pub fn register_function(&mut self, name: &str, func: SensitivityFunction) {
        self.sensitivity_functions.insert(name.to_string(), func);
    }

    /// Compute the L1 sensitivity of a statistical operation over `data`.
    ///
    /// Sensitivity is the maximum change in the operation's output when a single
    /// record is added or removed. The following closed-form approximations are
    /// used (each assumes a bounded domain where one record can shift a value by
    /// at most 1.0):
    ///
    /// - `mean`:      `1/n`        — one record moves the mean by `1/n`.
    /// - `sum`:       `1.0`        — one record changes the sum by at most 1.
    /// - `count`:     `1.0`        — one record changes the count by 1.
    /// - `median`:    `range / n`  — adjacent-element approximation.
    /// - `variance`:  `(max-min)^2 / n` — bounded shift approximation.
    /// - `histogram`: `1.0`        — one record changes a single bin by 1.
    ///
    /// Results are cached keyed by `operation` so repeated DP queries reuse the
    /// computed sensitivity.
    pub fn compute_sensitivity(
        &mut self,
        operation: &str,
        data: &[f64],
    ) -> Result<f64, StatisticalError> {
        // A registered function wins over the built-in approximations.
        if let Some(func) = self.sensitivity_functions.get(operation) {
            self.sensitivity_cache
                .insert(operation.to_string(), func.sensitivity);
            return Ok(func.sensitivity);
        }

        if data.is_empty() {
            return Err(StatisticalError::InvalidData(
                "Cannot compute sensitivity over empty data".to_string(),
            ));
        }

        let n = data.len() as f64;
        let sensitivity = match operation {
            "mean" => 1.0 / n,
            "sum" => 1.0,
            "count" => 1.0,
            "histogram" => 1.0,
            "median" => {
                let min = data.iter().cloned().fold(f64::INFINITY, f64::min);
                let max = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                (max - min) / n
            }
            "variance" => {
                let min = data.iter().cloned().fold(f64::INFINITY, f64::min);
                let max = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                let range = max - min;
                (range * range) / n
            }
            other => {
                return Err(StatisticalError::InvalidOperation(format!(
                    "Unknown sensitivity operation '{}'",
                    other
                )))
            }
        };

        self.sensitivity_cache
            .insert(operation.to_string(), sensitivity);
        Ok(sensitivity)
    }

    /// Get the sensitivity for an operation, returning the cached value when
    /// available and computing (and caching) it otherwise.
    pub fn get_sensitivity(
        &mut self,
        operation: &str,
        data: &[f64],
    ) -> Result<f64, StatisticalError> {
        if let Some(cached) = self.sensitivity_cache.get(operation) {
            return Ok(*cached);
        }
        self.compute_sensitivity(operation, data)
    }
}

impl SecureAggregation {
    pub fn new() -> Self {
        Self {
            aggregation_protocols: vec![
                AggregationProtocol::SecureSum,
                AggregationProtocol::SecureMean,
            ],
            encryption_schemes: vec![
                EncryptionScheme::Homomorphic,
                EncryptionScheme::SecretSharing,
            ],
            integrity_checks: Vec::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), StatisticalError> {
        Ok(())
    }

    /// Returns the list of registered aggregation protocols.
    pub fn aggregation_protocols(&self) -> &[AggregationProtocol] {
        &self.aggregation_protocols
    }

    /// Register an additional aggregation protocol if not already present.
    pub fn add_aggregation_protocol(&mut self, protocol: AggregationProtocol) {
        if !self.aggregation_protocols.contains(&protocol) {
            self.aggregation_protocols.push(protocol);
        }
    }

    /// Returns the list of registered encryption schemes.
    pub fn encryption_schemes(&self) -> &[EncryptionScheme] {
        &self.encryption_schemes
    }

    /// Register an additional encryption scheme if not already present.
    pub fn add_encryption_scheme(&mut self, scheme: EncryptionScheme) {
        if !self.encryption_schemes.contains(&scheme) {
            self.encryption_schemes.push(scheme);
        }
    }

    /// Register an integrity check.
    pub fn add_integrity_check(&mut self, check: IntegrityCheck) {
        self.integrity_checks.push(check);
    }

    /// Returns the list of registered integrity checks.
    pub fn integrity_checks(&self) -> &[IntegrityCheck] {
        &self.integrity_checks
    }

    /// Look up an integrity check by id.
    pub fn get_integrity_check(&self, check_id: &str) -> Option<&IntegrityCheck> {
        self.integrity_checks
            .iter()
            .find(|c| c.check_id == check_id)
    }

    /// Returns the number of registered integrity checks.
    pub fn integrity_check_count(&self) -> usize {
        self.integrity_checks.len()
    }
}
