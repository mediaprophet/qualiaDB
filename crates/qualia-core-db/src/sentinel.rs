//! Sentinel Validation Module for SPARQL-Star
//!
//! Implements Sentinel policy validation for embedded triples.
//! Sentinel rules are stored as dedicated Q42 blocks and validated during ingestion.

use crate::QualiaQuin;
use std::collections::HashSet;

/// Sentinel Rule ID type
pub type SentinelRuleId = u64;

/// Sentinel rule types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SentinelRuleType {
    /// Shape validation (SHACL-like)
    Shape,
    /// Confidence threshold
    Confidence,
    /// Domain restriction
    Domain,
    /// Structural gatekeeper
    Structural,
}

/// Sentinel rule definition
#[repr(C)]
#[derive(Debug, Clone)]
pub struct SentinelRule {
    /// Rule ID (hash of rule definition)
    pub rule_id: SentinelRuleId,
    /// Rule type
    pub rule_type: SentinelRuleType,
    /// Context ID this rule applies to (0 = all contexts)
    pub context_id: u64,
    /// Predicate hash this rule applies to (0 = all predicates)
    pub predicate_hash: u64,
    /// Minimum confidence threshold (0.0 - 1.0)
    pub min_confidence: f32,
    /// Required domain hash
    pub required_domain: u64,
}

impl SentinelRule {
    /// Create a new Sentinel rule
    pub fn new(rule_id: SentinelRuleId, rule_type: SentinelRuleType) -> Self {
        Self {
            rule_id,
            rule_type,
            context_id: 0,
            predicate_hash: 0,
            min_confidence: 0.0,
            required_domain: 0,
        }
    }
    
    /// Set the context ID for this rule
    pub fn with_context(mut self, context_id: u64) -> Self {
        self.context_id = context_id;
        self
    }
    
    /// Set the predicate hash for this rule
    pub fn with_predicate(mut self, predicate_hash: u64) -> Self {
        self.predicate_hash = predicate_hash;
        self
    }
    
    /// Set the minimum confidence threshold
    pub fn with_confidence(mut self, min_confidence: f32) -> Self {
        self.min_confidence = min_confidence;
        self
    }
    
    /// Set the required domain hash
    pub fn with_domain(mut self, required_domain: u64) -> Self {
        self.required_domain = required_domain;
        self
    }
}

/// Sentinel validation result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SentinelVerdict {
    /// Quin passes validation
    Pass,
    /// Quin fails validation
    Fail(String),
    /// Quin passes with reduced confidence
    PassWithWarning(String),
}

/// Sentinel validator
pub struct SentinelValidator {
    /// Active Sentinel rules
    rules: Vec<SentinelRule>,
    /// Fast-path cache for rule IDs (Bloom filter simulation)
    active_rule_ids: HashSet<SentinelRuleId>,
    /// Confidence weights from metadata field
    confidence_threshold: f32,
}

impl SentinelValidator {
    /// Create a new Sentinel validator
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            active_rule_ids: HashSet::new(),
            confidence_threshold: 0.5,
        }
    }
    
    /// Set the confidence threshold
    pub fn set_confidence_threshold(&mut self, threshold: f32) {
        self.confidence_threshold = threshold;
    }
    
    /// Add a Sentinel rule
    pub fn add_rule(&mut self, rule: SentinelRule) {
        self.active_rule_ids.insert(rule.rule_id);
        self.rules.push(rule);
    }
    
    /// Fast-path check: does this Quin need Sentinel validation?
    /// Returns false if no rules apply, allowing fast-path to Q42 append buffer
    pub fn needs_validation(&self, quin: &QualiaQuin) -> bool {
        // Check if any rules apply to this context or predicate
        for rule in &self.rules {
            if rule.context_id == 0 || rule.context_id == quin.context {
                if rule.predicate_hash == 0 || rule.predicate_hash == quin.predicate {
                    return true;
                }
            }
        }
        false
    }
    
    /// Validate a QualiaQuin against Sentinel rules
    pub fn validate(&self, quin: &QualiaQuin) -> SentinelVerdict {
        // Extract confidence from metadata field (bits [0..31])
        let confidence = (quin.metadata & 0xFFFFFFFF) as f32 / u32::MAX as f32;
        
        // Check confidence threshold
        if confidence < self.confidence_threshold {
            return SentinelVerdict::Fail(format!(
                "Confidence {} below threshold {}",
                confidence, self.confidence_threshold
            ));
        }
        
        // Check each applicable rule
        for rule in &self.rules {
            if rule.context_id != 0 && rule.context_id != quin.context {
                continue;
            }
            
            if rule.predicate_hash != 0 && rule.predicate_hash != quin.predicate {
                continue;
            }
            
            match rule.rule_type {
                SentinelRuleType::Confidence => {
                    if confidence < rule.min_confidence {
                        return SentinelVerdict::Fail(format!(
                            "Confidence {} below rule threshold {}",
                            confidence, rule.min_confidence
                        ));
                    }
                }
                SentinelRuleType::Domain => {
                    // Check if subject is in required domain
                    // This is a simplified check - real implementation would verify
                    // against a domain registry
                    if rule.required_domain != 0 && quin.subject != rule.required_domain {
                        return SentinelVerdict::Fail(format!(
                            "Subject not in required domain"
                        ));
                    }
                }
                SentinelRuleType::Shape => {
                    // Shape validation would check structural constraints
                    // For now, we just pass
                }
                SentinelRuleType::Structural => {
                    // Structural gatekeeper validation
                    // For now, we just pass
                }
            }
        }
        
        SentinelVerdict::Pass
    }
    
    /// Validate an embedded triple (Virtual ID)
    pub fn validate_embedded_triple(
        &self,
        virtual_id: u64,
        components: &[u64; 3],
        context_id: u64,
    ) -> SentinelVerdict {
        const TAG_EMBEDDED: u64 = 0x1;
        
        // Verify it's actually a Virtual ID
        if virtual_id & TAG_EMBEDDED != TAG_EMBEDDED {
            return SentinelVerdict::Fail("Not a Virtual ID".to_string());
        }
        
        // Check if any rules apply to embedded triples in this context
        for rule in &self.rules {
            if rule.context_id == 0 || rule.context_id == context_id {
                match rule.rule_type {
                    SentinelRuleType::Structural => {
                        // Check structural validity of embedded triple
                        // For now, just verify all components are non-zero
                        if components[0] == 0 || components[1] == 0 || components[2] == 0 {
                            return SentinelVerdict::Fail(
                                "Embedded triple has zero component".to_string()
                            );
                        }
                    }
                    _ => {}
                }
            }
        }
        
        SentinelVerdict::Pass
    }
}

impl Default for SentinelValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Sentinel rule storage in Q42 format
/// Sentinel rules are stored as Quins with special metadata flags
#[repr(C)]
#[derive(Debug, Clone)]
pub struct SentinelStorage {
    /// Sentinel block ID
    pub block_id: u64,
    /// Number of rules in this block
    pub rule_count: u32,
    /// Reserved for future use
    pub reserved: u32,
}

impl SentinelStorage {
    /// Serialize Sentinel rules to Quins
    pub fn rules_to_quins(rules: &[SentinelRule]) -> Vec<QualiaQuin> {
        const SENTINEL_METADATA_FLAG: u64 = 0x1 << 62; // Use bit 62 as Sentinel flag
        
        rules.iter()
            .map(|rule| {
                QualiaQuin {
                    subject: rule.rule_id,
                    predicate: (rule.rule_type as u64) << 8 | (rule.context_id & 0xFF),
                    object: rule.predicate_hash,
                    context: rule.context_id,
                    metadata: SENTINEL_METADATA_FLAG | 
                              ((rule.min_confidence as u64) & 0xFFFFFFFF) |
                              (rule.required_domain << 32),
                    parity: 0,
                }
            })
            .collect()
    }
    
    /// Deserialize Quins to Sentinel rules
    pub fn quins_to_rules(quins: &[QualiaQuin]) -> Vec<SentinelRule> {
        const SENTINEL_METADATA_FLAG: u64 = 0x1 << 62;
        
        quins.iter()
            .filter(|quin| quin.metadata & SENTINEL_METADATA_FLAG != 0)
            .map(|quin| {
                let rule_type = match (quin.predicate >> 8) % 4 {
                    0 => SentinelRuleType::Shape,
                    1 => SentinelRuleType::Confidence,
                    2 => SentinelRuleType::Domain,
                    _ => SentinelRuleType::Structural,
                };
                
                SentinelRule {
                    rule_id: quin.subject,
                    rule_type,
                    context_id: quin.context,
                    predicate_hash: quin.object,
                    min_confidence: (quin.metadata & 0xFFFFFFFF) as f32 / u32::MAX as f32,
                    required_domain: (quin.metadata >> 32) & 0x0FFF_FFFF,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod sentinel_tests {
    use super::*;

    #[test]
    fn test_sentinel_rule_creation() {
        let rule = SentinelRule::new(123, SentinelRuleType::Confidence)
            .with_confidence(0.8);
        
        assert_eq!(rule.rule_id, 123);
        assert_eq!(rule.min_confidence, 0.8);
    }

    #[test]
    fn test_validator_creation() {
        let validator = SentinelValidator::new();
        assert!(!validator.needs_validation(&QualiaQuin {
            subject: 1, predicate: 2, object: 3, context: 0, metadata: 0, parity: 0
        }));
    }

    #[test]
    fn test_add_rule() {
        let mut validator = SentinelValidator::new();
        let rule = SentinelRule::new(123, SentinelRuleType::Confidence)
            .with_predicate(456)
            .with_context(789);
        validator.add_rule(rule);
        
        let quin = QualiaQuin {
            subject: 1, predicate: 456, object: 3, context: 789, metadata: 0, parity: 0
        };
        assert!(validator.needs_validation(&quin));
    }

    #[test]
    fn test_validate_pass() {
        let mut validator = SentinelValidator::new();
        validator.set_confidence_threshold(0.5);
        
        let quin = QualiaQuin {
            subject: 1, predicate: 2, object: 3, context: 0, 
            metadata: u32::MAX as u64, // Maximum confidence
            parity: 0
        };
        
        assert_eq!(validator.validate(&quin), SentinelVerdict::Pass);
    }

    #[test]
    fn test_validate_fail_confidence() {
        let mut validator = SentinelValidator::new();
        validator.set_confidence_threshold(0.5);
        
        let quin = QualiaQuin {
            subject: 1, predicate: 2, object: 3, context: 0, 
            metadata: 0, // Zero confidence
            parity: 0
        };
        
        match validator.validate(&quin) {
            SentinelVerdict::Fail(_) => {}
            _ => panic!("Should have failed"),
        }
    }

    #[test]
    fn test_validate_embedded_triple() {
        let validator = SentinelValidator::new();
        const TAG_EMBEDDED: u64 = 0x1;
        
        let virtual_id = TAG_EMBEDDED | 0x123456789ABCDEF0;
        let components = [1u64, 2, 3];
        
        assert_eq!(
            validator.validate_embedded_triple(virtual_id, &components, 0),
            SentinelVerdict::Pass
        );
    }

    #[test]
    fn test_sentinel_storage_serialization() {
        let rules = vec![
            SentinelRule::new(123, SentinelRuleType::Confidence).with_confidence(0.8),
            SentinelRule::new(456, SentinelRuleType::Domain).with_domain(789),
        ];
        
        let quins = SentinelStorage::rules_to_quins(&rules);
        assert_eq!(quins.len(), 2);
        
        let rules_back = SentinelStorage::quins_to_rules(&quins);
        assert_eq!(rules_back.len(), 2);
    }
}