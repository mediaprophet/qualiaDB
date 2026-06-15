//! Semantic Culler - Pre-GPU pipeline for agency-driven data filtering
//!
//! This module implements semantic and deontic culling before GPU rendering,
//! ensuring that unauthorized Quins are mathematically dropped before they
//! reach the WebGPU staging buffers. It integrates with zk_proofs and
//! fiduciary_crypto for cryptographic verification of agency permissions.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, OnceLock};
use serde::{Deserialize, Serialize};
use crate::zk_proofs::{ZkProofSystem, SemanticProof, MathematicalStatement, StatementType, FieldElement};
use crate::fiduciary_crypto::{FiduciaryCrypto, MlDsaSignature, CryptoContext};

#[cfg(feature = "zk-culling")]
use ark_bls12_381::{Bls12_381, Fr};
#[cfg(feature = "zk-culling")]
use ark_groth16::{Groth16, PreparedVerifyingKey, Proof};
#[cfg(feature = "zk-culling")]
use ark_snark::SNARK;

/// Cache the Verifying Key at boot to prevent GC allocation spikes
#[cfg(feature = "zk-culling")]
static DEONTIC_VK: OnceLock<PreparedVerifyingKey<Bls12_381>> = OnceLock::new();

#[cfg(feature = "zk-culling")]
/// Initialize the Deontic gateway with a verifying key
pub fn initialize_deontic_gateway(vk: PreparedVerifyingKey<Bls12_381>) -> Result<(), String> {
    DEONTIC_VK.set(vk).map_err(|_| "Deontic VK already initialized".to_string())
}

#[cfg(feature = "zk-culling")]
/// Executes before NQuins cross the PCIe bus
/// Constant-time ~3ms verification with fast-fail drop condition
pub fn verify_nquin_access(
    proof: &Proof<Bls12_381>,
    public_inputs: &[Fr],
    nquin_buffer: &mut [u8],
) -> bool {
    let vk = DEONTIC_VK.get().expect("Deontic VK not initialized");
    
    // Constant-time ~3ms verification
    match Groth16::<Bls12_381>::verify_with_processed_vk(vk, public_inputs, proof) {
        Ok(true) => true, // Access granted
        _ => {
            // FAST-FAIL: Drop the memory immediately
            // Zero out the semantic payload to prevent hardware cache leaks
            nquin_buffer.fill(0);
            false
        }
    }
}

/// Semantic culler for agency-driven data filtering
pub struct SemanticCuller {
    zk_system: Arc<Mutex<ZkProofSystem>>,
    fiduciary_crypto: Arc<Mutex<FiduciaryCrypto>>,
    agency_policies: HashMap<String, AgencyPolicy>,
    culling_stats: CullingStats,
}

/// Agency policy for data access control
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgencyPolicy {
    pub agency_id: String,
    pub access_level: AccessLevel,
    pub semantic_filters: Vec<SemanticFilter>,
    pub temporal_constraints: TemporalConstraints,
    pub deontic_rules: Vec<DeonticRule>,
}

/// Access levels for agency permissions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AccessLevel {
    Read,
    Write,
    Admin,
    System,
}

/// Semantic filter for content-based filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticFilter {
    pub filter_id: String,
    pub filter_type: FilterType,
    pub criteria: String,
    pub required: bool,
}

/// Filter types for semantic culling
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FilterType {
    Category,
    IntensityThreshold,
    EpistemicLevel,
    TemporalRange,
    Custom,
}

/// Temporal constraints for data access
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalConstraints {
    pub valid_from: u64,
    pub valid_until: u64,
    pub max_age_seconds: Option<u64>,
}

/// Deontic rules for permission logic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeonticRule {
    pub rule_id: String,
    pub rule_type: DeonticType,
    pub condition: String,
    pub action: DeonticAction,
}

/// Deontic logic types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeonticType {
    Obligation,
    Permission,
    Prohibition,
}

/// Actions for deontic rules
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeonticAction {
    Allow,
    Deny,
    RequireProof,
    RequireSignature,
}

/// Quin data structure for semantic filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quin {
    pub quin_id: String,
    pub semantic_id: u64,
    pub intensity: f64,
    pub epistemic_level: f64,
    pub timestamp: u64,
    pub category: String,
    pub agency_id: Option<String>,
    pub proof: Option<SemanticProof>,
    pub signature: Option<MlDsaSignature>,
}

/// Culling result for a Quin
#[derive(Debug, Clone)]
pub struct CullingResult {
    pub quin_id: String,
    pub allowed: bool,
    pub reason: CullingReason,
    pub verification_data: Option<VerificationData>,
}

/// Reasons for culling decisions
#[derive(Debug, Clone, PartialEq)]
pub enum CullingReason {
    SemanticFilterMatch,
    TemporalConstraintViolation,
    DeonticRuleViolation,
    ProofVerificationFailed,
    SignatureVerificationFailed,
    MissingPermission,
    Allowed,
}

/// Verification data for cryptographic checks
#[derive(Debug, Clone)]
pub struct VerificationData {
    pub proof_valid: Option<bool>,
    pub signature_valid: Option<bool>,
    pub verification_time_ms: u64,
}

/// Statistics for culling operations
#[derive(Debug, Clone)]
pub struct CullingStats {
    pub total_processed: u64,
    pub total_allowed: u64,
    pub total_denied: u64,
    pub semantic_filtered: u64,
    pub temporal_filtered: u64,
    pub deontic_filtered: u64,
    pub crypto_filtered: u64,
}

impl SemanticCuller {
    /// Create new semantic culler
    pub fn new() -> Self {
        Self {
            zk_system: Arc::new(Mutex::new(ZkProofSystem::new())),
            fiduciary_crypto: Arc::new(Mutex::new(FiduciaryCrypto::new())),
            agency_policies: HashMap::new(),
            culling_stats: CullingStats::default(),
        }
    }

    /// Add agency policy
    pub fn add_policy(&mut self, policy: AgencyPolicy) {
        self.agency_policies.insert(policy.agency_id.clone(), policy);
    }

    /// Cull a batch of Quins based on agency policies
    pub fn cull_quins(&mut self, agency_id: &str, quins: Vec<Quin>) -> Vec<CullingResult> {
        let policy = self.agency_policies.get(agency_id).cloned();
        
        let results: Vec<CullingResult> = quins.into_iter().map(|quin| {
            self.cull_single_quin(agency_id, &quin, policy.as_ref())
        }).collect();

        // Update statistics
        self.update_stats(&results);

        results
    }

    /// Cull a single Quin
    fn cull_single_quin(&mut self, agency_id: &str, quin: &Quin, policy: Option<&AgencyPolicy>) -> CullingResult {
        // If no policy exists, deny by default
        let policy = match policy {
            Some(p) => p,
            None => return CullingResult {
                quin_id: quin.quin_id.clone(),
                allowed: false,
                reason: CullingReason::MissingPermission,
                verification_data: None,
            }
        };

        // Check semantic filters
        if let Some(reason) = self.check_semantic_filters(quin, policy) {
            return CullingResult {
                quin_id: quin.quin_id.clone(),
                allowed: false,
                reason,
                verification_data: None,
            };
        }

        // Check temporal constraints
        if let Some(reason) = self.check_temporal_constraints(quin, policy) {
            return CullingResult {
                quin_id: quin.quin_id.clone(),
                allowed: false,
                reason,
                verification_data: None,
            };
        }

        // Check deontic rules
        if let Some(reason) = self.check_deontic_rules(quin, policy) {
            return CullingResult {
                quin_id: quin.quin_id.clone(),
                allowed: false,
                reason,
                verification_data: None,
            };
        }

        // Check cryptographic verification if required
        if let Some(verification_data) = self.check_cryptographic_verification(quin, policy) {
            if !verification_data.is_allowed() {
                let reason = if verification_data.proof_valid == Some(false) {
                    CullingReason::ProofVerificationFailed
                } else if verification_data.signature_valid == Some(false) {
                    CullingReason::SignatureVerificationFailed
                } else {
                    CullingReason::DeonticRuleViolation
                };

                return CullingResult {
                    quin_id: quin.quin_id.clone(),
                    allowed: false,
                    reason,
                    verification_data: Some(verification_data),
                };
            }
        }

        // All checks passed
        CullingResult {
            quin_id: quin.quin_id.clone(),
            allowed: true,
            reason: CullingReason::Allowed,
            verification_data: None,
        }
    }

    /// Check semantic filters
    fn check_semantic_filters(&self, quin: &Quin, policy: &AgencyPolicy) -> Option<CullingReason> {
        for filter in &policy.semantic_filters {
            if !filter.required {
                continue;
            }

            let matches = match filter.filter_type {
                FilterType::Category => quin.category == filter.criteria,
                FilterType::IntensityThreshold => {
                    let threshold: f64 = filter.criteria.parse().unwrap_or(0.0);
                    quin.intensity < threshold
                }
                FilterType::EpistemicLevel => {
                    let threshold: f64 = filter.criteria.parse().unwrap_or(0.0);
                    quin.epistemic_level < threshold
                }
                FilterType::TemporalRange => {
                    // Simplified temporal check
                    true
                }
                FilterType::Custom => {
                    // Custom filter logic would go here
                    false
                }
            };

            if matches {
                return Some(CullingReason::SemanticFilterMatch);
            }
        }

        None
    }

    /// Check temporal constraints
    fn check_temporal_constraints(&self, quin: &Quin, policy: &AgencyPolicy) -> Option<CullingReason> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Check valid_from
        if quin.timestamp < policy.temporal_constraints.valid_from {
            return Some(CullingReason::TemporalConstraintViolation);
        }

        // Check valid_until
        if quin.timestamp > policy.temporal_constraints.valid_until {
            return Some(CullingReason::TemporalConstraintViolation);
        }

        // Check max_age
        if let Some(max_age) = policy.temporal_constraints.max_age_seconds {
            if now - quin.timestamp > max_age {
                return Some(CullingReason::TemporalConstraintViolation);
            }
        }

        None
    }

    /// Check deontic rules
    fn check_deontic_rules(&self, quin: &Quin, policy: &AgencyPolicy) -> Option<CullingReason> {
        for rule in &policy.deontic_rules {
            match rule.rule_type {
                DeonticType::Prohibition => {
                    if rule.action == DeonticAction::Deny && self.evaluate_condition(&rule.condition, quin) {
                        return Some(CullingReason::DeonticRuleViolation);
                    }
                }
                DeonticType::Obligation => {
                    if rule.action == DeonticAction::RequireProof && quin.proof.is_none() {
                        return Some(CullingReason::DeonticRuleViolation);
                    }
                    if rule.action == DeonticAction::RequireSignature && quin.signature.is_none() {
                        return Some(CullingReason::DeonticRuleViolation);
                    }
                }
                DeonticType::Permission => {
                    // Permission rules are checked in cryptographic verification
                }
            }
        }

        None
    }

    /// Evaluate deontic condition (simplified)
    fn evaluate_condition(&self, condition: &str, quin: &Quin) -> bool {
        // Simplified condition evaluation
        // Real implementation would parse and evaluate logical expressions
        condition.contains(&quin.category) || condition.contains(&quin.semantic_id.to_string())
    }

    /// Check cryptographic verification
    fn check_cryptographic_verification(&self, quin: &Quin, policy: &AgencyPolicy) -> Option<VerificationData> {
        let mut proof_valid = None;
        let mut signature_valid = None;
        let start_time = std::time::Instant::now();

        // Check if any deontic rules require cryptographic verification
        let requires_proof = policy.deontic_rules.iter().any(|r| 
            r.rule_type == DeonticType::Obligation && r.action == DeonticAction::RequireProof
        );
        let requires_signature = policy.deontic_rules.iter().any(|r| 
            r.rule_type == DeonticType::Obligation && r.action == DeonticAction::RequireSignature
        );

        if requires_proof {
            if let Some(ref proof) = quin.proof {
                let mut zk_system = self.zk_system.lock().unwrap();
                match zk_system.verify_semantic_proof(&mut proof.clone()) {
                    Ok(_) => proof_valid = Some(true),
                    Err(_) => proof_valid = Some(false),
                }
            } else {
                proof_valid = Some(false);
            }
        }

        if requires_signature {
            if let Some(ref signature) = quin.signature {
                let fiduciary_crypto = self.fiduciary_crypto.lock().unwrap();
                let message = format!("{}:{}:{}", quin.quin_id, quin.semantic_id, quin.timestamp);
                let context = CryptoContext {
                    domain: "webizen_culling".to_string(),
                    purpose: "agency_verification".to_string(),
                    timestamp: 0,
                    nonce: [0u8; 32],
                };
                
                // Use default key for verification
                match fiduciary_crypto.verify(message.as_bytes(), signature, None, context.domain, context.purpose) {
                    Ok(valid) => signature_valid = Some(valid),
                    Err(_) => signature_valid = Some(false),
                }
            } else {
                signature_valid = Some(false);
            }
        }

        let verification_time = start_time.elapsed().as_millis() as u64;

        if proof_valid.is_some() || signature_valid.is_some() {
            Some(VerificationData {
                proof_valid,
                signature_valid,
                verification_time_ms: verification_time,
            })
        } else {
            None
        }
    }

    /// Update culling statistics
    fn update_stats(&mut self, results: &[CullingResult]) {
        self.culling_stats.total_processed += results.len() as u64;

        for result in results {
            if result.allowed {
                self.culling_stats.total_allowed += 1;
            } else {
                self.culling_stats.total_denied += 1;
                
                match result.reason {
                    CullingReason::SemanticFilterMatch => {
                        self.culling_stats.semantic_filtered += 1;
                    }
                    CullingReason::TemporalConstraintViolation => {
                        self.culling_stats.temporal_filtered += 1;
                    }
                    CullingReason::DeonticRuleViolation => {
                        self.culling_stats.deontic_filtered += 1;
                    }
                    CullingReason::ProofVerificationFailed | 
                    CullingReason::SignatureVerificationFailed => {
                        self.culling_stats.crypto_filtered += 1;
                    }
                    _ => {}
                }
            }
        }
    }

    /// Get culling statistics
    pub fn get_stats(&self) -> &CullingStats {
        &self.culling_stats
    }

    /// Reset culling statistics
    pub fn reset_stats(&mut self) {
        self.culling_stats = CullingStats::default();
    }

    /// Generate a semantic proof for a Quin
    pub fn generate_proof_for_quin(&mut self, quin: &Quin) -> Result<SemanticProof, String> {
        let mut zk_system = self.zk_system.lock().unwrap();
        
        let statement = MathematicalStatement {
            statement_id: quin.quin_id.clone(),
            statement_type: StatementType::Equality,
            expression: format!("intensity == {}", quin.intensity),
            variables: vec!["intensity".to_string()],
            constraints: vec![],
        };

        let mut witness = HashMap::new();
        witness.insert("intensity".to_string(), FieldElement { value: [0u8; 32] });

        zk_system.generate_semantic_proof(statement, witness)
            .map_err(|e| format!("Proof generation failed: {:?}", e))
    }

    /// Sign a Quin with fiduciary crypto
    pub fn sign_quin(&mut self, quin: &Quin, key_id: Option<&str>) -> Result<MlDsaSignature, String> {
        let fiduciary_crypto = self.fiduciary_crypto.lock().unwrap();
        
        let message = format!("{}:{}:{}", quin.quin_id, quin.semantic_id, quin.timestamp);
        
        fiduciary_crypto.sign(
            message.as_bytes(),
            key_id,
            "webizen_quin".to_string(),
            "agency_signature".to_string()
        ).map_err(|e| format!("Signing failed: {:?}", e))
    }
}

impl VerificationData {
    /// Check if verification allows the Quin
    fn is_allowed(&self) -> bool {
        match (self.proof_valid, self.signature_valid) {
            (Some(true), Some(true)) => true,
            (Some(true), None) => true,
            (None, Some(true)) => true,
            (Some(false), _) => false,
            (_, Some(false)) => false,
            (None, None) => true,
        }
    }
}

impl Default for CullingStats {
    fn default() -> Self {
        Self {
            total_processed: 0,
            total_allowed: 0,
            total_denied: 0,
            semantic_filtered: 0,
            temporal_filtered: 0,
            deontic_filtered: 0,
            crypto_filtered: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semantic_culler_creation() {
        let culler = SemanticCuller::new();
        assert_eq!(culler.get_stats().total_processed, 0);
    }

    #[test]
    fn test_agency_policy() {
        let policy = AgencyPolicy {
            agency_id: "test_agency".to_string(),
            access_level: AccessLevel::Read,
            semantic_filters: vec![],
            temporal_constraints: TemporalConstraints {
                valid_from: 0,
                valid_until: u64::MAX,
                max_age_seconds: None,
            },
            deontic_rules: vec![],
        };

        let mut culler = SemanticCuller::new();
        culler.add_policy(policy);
        
        assert_eq!(culler.agency_policies.len(), 1);
    }

    #[test]
    fn test_quin_culling() {
        let mut culler = SemanticCuller::new();
        
        let policy = AgencyPolicy {
            agency_id: "test_agency".to_string(),
            access_level: AccessLevel::Read,
            semantic_filters: vec![],
            temporal_constraints: TemporalConstraints {
                valid_from: 0,
                valid_until: u64::MAX,
                max_age_seconds: None,
            },
            deontic_rules: vec![],
        };
        culler.add_policy(policy);

        let quin = Quin {
            quin_id: "test_quin".to_string(),
            semantic_id: 123,
            intensity: 0.5,
            epistemic_level: 0.9,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            category: "test".to_string(),
            agency_id: None,
            proof: None,
            signature: None,
        };

        let results = culler.cull_quins("test_agency", vec![quin]);
        assert_eq!(results.len(), 1);
        assert!(results[0].allowed);
    }
}
