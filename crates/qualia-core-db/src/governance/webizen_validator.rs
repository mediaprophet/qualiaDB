//! Webizen NQuin Validation Layer
//!
//! Provides shape/confidence/domain/structural rule enforcement for NQuins at
//! ingestion and query time.  Rules are stored as dedicated Q42 blocks and
//! validated during ingestion via [`WebValidator`].
//!
//! A lightweight pattern-matching VM ([`execute_query_program`]) is also
//! provided for the CLI SPARQL query path; it is distinct from the full
//! Webizen bytecode engine in [`crate::webizen_bytecode`].

use crate::NQuin;
use std::collections::HashSet;

const OP_MATCH_SUBJ: u8 = 0x01;
const OP_MATCH_PRED: u8 = 0x02;
const OP_MATCH_OBJ:  u8 = 0x03;

const OP_BIND_VAR: u8   = 0x10;
const SLOT_SUBJ: u8 = 0x00;
const SLOT_PRED: u8 = 0x01;
const SLOT_OBJ: u8  = 0x02;

const OP_HALT: u8       = 0xFF;

/// Stack-allocated register file for bound variable hashes (max 16 variables).
#[derive(Debug, Default)]
struct VmRegisters {
    values: [u64; 16],
    bound_flags: u16,
}

impl VmRegisters {
    fn bind(&mut self, id: u8, value: u64) {
        if (id as usize) < self.values.len() {
            self.values[id as usize] = value;
            self.bound_flags |= 1 << id;
        }
    }
}

/// Execute a query bytecode program against a NQuin dataset.
///
/// Opcodes: `MATCH_SUBJ` / `MATCH_PRED` / `MATCH_OBJ` filter the candidate
/// set; `BIND_VAR` extracts a slot value into a register; `HALT` stops early.
///
/// Returns the vector of bound register values on success.
pub fn execute_query_program(program: &[u8], dataset: &[NQuin]) -> Result<Vec<u64>, &'static str> {
    let mut pc = 0;
    let mut registers = VmRegisters::default();
    let mut candidate_set: Vec<&NQuin> = dataset.iter().collect();

    while pc < program.len() {
        let opcode = program[pc];
        pc += 1;

        match opcode {
            OP_MATCH_SUBJ => {
                if pc + 8 > program.len() { return Err("Unexpected EOF in OP_MATCH_SUBJ"); }
                let target_hash = u64::from_le_bytes(program[pc..pc+8].try_into().unwrap());
                pc += 8;
                candidate_set.retain(|quin| quin.subject == target_hash);
            }
            OP_MATCH_PRED => {
                if pc + 8 > program.len() { return Err("Unexpected EOF in OP_MATCH_PRED"); }
                let target_hash = u64::from_le_bytes(program[pc..pc+8].try_into().unwrap());
                pc += 8;
                candidate_set.retain(|quin| quin.predicate == target_hash);
            }
            OP_MATCH_OBJ => {
                if pc + 8 > program.len() { return Err("Unexpected EOF in OP_MATCH_OBJ"); }
                let target_hash = u64::from_le_bytes(program[pc..pc+8].try_into().unwrap());
                pc += 8;
                candidate_set.retain(|quin| quin.object == target_hash);
            }
            OP_BIND_VAR => {
                if pc + 2 > program.len() { return Err("Unexpected EOF in OP_BIND_VAR"); }
                let slot   = program[pc];
                let reg_id = program[pc + 1];
                pc += 2;
                if let Some(q) = candidate_set.first() {
                    let value = match slot {
                        SLOT_SUBJ => q.subject,
                        SLOT_PRED => q.predicate,
                        SLOT_OBJ  => q.object,
                        _ => return Err("Invalid slot identifier"),
                    };
                    registers.bind(reg_id, value);
                }
            }
            OP_HALT => break,
            _ => return Err("Unknown Opcode"),
        }
    }

    let mut results = Vec::new();
    for i in 0..16 {
        if registers.bound_flags & (1 << i) != 0 {
            results.push(registers.values[i]);
        }
    }
    Ok(results)
}

// ── Rule types ────────────────────────────────────────────────────────────────

/// Unique identifier for a [`WebRule`].
pub type WebRuleId = u64;

/// Classification of a [`WebRule`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebRuleType {
    Shape,
    Confidence,
    Domain,
    Structural,
}

/// A validation rule applied by the [`WebValidator`] to incoming NQuins.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct WebRule {
    pub rule_id: WebRuleId,
    pub rule_type: WebRuleType,
    /// Context this rule applies to; `0` means all contexts.
    pub context_id: u64,
    /// Predicate this rule applies to; `0` means all predicates.
    pub predicate_hash: u64,
    pub min_confidence: f32,
    pub required_domain: u64,
}

impl WebRule {
    pub fn new(rule_id: WebRuleId, rule_type: WebRuleType) -> Self {
        Self {
            rule_id,
            rule_type,
            context_id: 0,
            predicate_hash: 0,
            min_confidence: 0.0,
            required_domain: 0,
        }
    }

    pub fn with_context(mut self, context_id: u64) -> Self {
        self.context_id = context_id;
        self
    }

    pub fn with_predicate(mut self, predicate_hash: u64) -> Self {
        self.predicate_hash = predicate_hash;
        self
    }

    pub fn with_confidence(mut self, min_confidence: f32) -> Self {
        self.min_confidence = min_confidence;
        self
    }

    pub fn with_domain(mut self, required_domain: u64) -> Self {
        self.required_domain = required_domain;
        self
    }
}

/// Outcome of a [`WebValidator::validate`] call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WebVerdict {
    Pass,
    Fail(String),
    PassWithWarning(String),
}

// ── Validator ─────────────────────────────────────────────────────────────────

/// Validates NQuins against a set of [`WebRule`]s before Q42 append.
pub struct WebValidator {
    rules: Vec<WebRule>,
    active_rule_ids: HashSet<WebRuleId>,
    confidence_threshold: f32,
}

impl WebValidator {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            active_rule_ids: HashSet::new(),
            confidence_threshold: 0.5,
        }
    }

    pub fn set_confidence_threshold(&mut self, threshold: f32) {
        self.confidence_threshold = threshold;
    }

    pub fn add_rule(&mut self, rule: WebRule) {
        self.active_rule_ids.insert(rule.rule_id);
        self.rules.push(rule);
    }

    /// Returns `false` when no rules apply, allowing the fast-path to the Q42
    /// append buffer without invoking the full validator.
    pub fn needs_validation(&self, quin: &NQuin) -> bool {
        for rule in &self.rules {
            if rule.context_id == 0 || rule.context_id == quin.context {
                if rule.predicate_hash == 0 || rule.predicate_hash == quin.predicate {
                    return true;
                }
            }
        }
        false
    }

    /// Validate a NQuin against all active rules.
    pub fn validate(&self, quin: &NQuin) -> WebVerdict {
        let confidence = (quin.metadata & 0xFFFFFFFF) as f32 / u32::MAX as f32;

        if confidence < self.confidence_threshold {
            return WebVerdict::Fail(format!(
                "Confidence {} below threshold {}",
                confidence, self.confidence_threshold
            ));
        }

        for rule in &self.rules {
            if rule.context_id != 0 && rule.context_id != quin.context {
                continue;
            }
            if rule.predicate_hash != 0 && rule.predicate_hash != quin.predicate {
                continue;
            }

            match rule.rule_type {
                WebRuleType::Confidence => {
                    if confidence < rule.min_confidence {
                        return WebVerdict::Fail(format!(
                            "Confidence {} below rule threshold {}",
                            confidence, rule.min_confidence
                        ));
                    }
                }
                WebRuleType::Domain => {
                    if rule.required_domain != 0 && quin.subject != rule.required_domain {
                        return WebVerdict::Fail("Subject not in required domain".to_string());
                    }
                }
                WebRuleType::Shape | WebRuleType::Structural => {}
            }
        }

        WebVerdict::Pass
    }

    /// Validate an embedded triple (Virtual ID).
    pub fn validate_embedded_triple(
        &self,
        virtual_id: u64,
        components: &[u64; 3],
        context_id: u64,
    ) -> WebVerdict {
        const TAG_EMBEDDED: u64 = 0x1;

        if virtual_id & TAG_EMBEDDED != TAG_EMBEDDED {
            return WebVerdict::Fail("Not a Virtual ID".to_string());
        }

        for rule in &self.rules {
            if rule.context_id == 0 || rule.context_id == context_id {
                if let WebRuleType::Structural = rule.rule_type {
                    if components[0] == 0 || components[1] == 0 || components[2] == 0 {
                        return WebVerdict::Fail(
                            "Embedded triple has zero component".to_string(),
                        );
                    }
                }
            }
        }

        WebVerdict::Pass
    }
}

impl Default for WebValidator {
    fn default() -> Self {
        Self::new()
    }
}

// ── Q42 storage ───────────────────────────────────────────────────────────────

/// Serialises and deserialises [`WebRule`]s to/from NQuin blocks in Q42.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct WebRuleStorage {
    pub block_id: u64,
    pub rule_count: u32,
    pub reserved: u32,
}

impl WebRuleStorage {
    const RULE_FLAG: u64 = 0x1 << 62;

    pub fn rules_to_quins(rules: &[WebRule]) -> Vec<NQuin> {
        rules.iter()
            .map(|rule| NQuin {
                subject: rule.rule_id,
                predicate: (rule.rule_type as u64) << 8 | (rule.context_id & 0xFF),
                object: rule.predicate_hash,
                context: rule.context_id,
                metadata: Self::RULE_FLAG
                    | ((rule.min_confidence as u64) & 0xFFFFFFFF)
                    | (rule.required_domain << 32),
                parity: 0,
            })
            .collect()
    }

    pub fn quins_to_rules(quins: &[NQuin]) -> Vec<WebRule> {
        quins.iter()
            .filter(|q| q.metadata & Self::RULE_FLAG != 0)
            .map(|q| {
                let rule_type = match (q.predicate >> 8) % 4 {
                    0 => WebRuleType::Shape,
                    1 => WebRuleType::Confidence,
                    2 => WebRuleType::Domain,
                    _ => WebRuleType::Structural,
                };
                WebRule {
                    rule_id: q.subject,
                    rule_type,
                    context_id: q.context,
                    predicate_hash: q.object,
                    min_confidence: (q.metadata & 0xFFFFFFFF) as f32 / u32::MAX as f32,
                    required_domain: (q.metadata >> 32) & 0x0FFF_FFFF,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_creation() {
        let rule = WebRule::new(123, WebRuleType::Confidence).with_confidence(0.8);
        assert_eq!(rule.rule_id, 123);
        assert_eq!(rule.min_confidence, 0.8);
    }

    #[test]
    fn validator_no_rules_needs_no_validation() {
        let validator = WebValidator::new();
        assert!(!validator.needs_validation(&NQuin {
            subject: 1, predicate: 2, object: 3, context: 0, metadata: 0, parity: 0
        }));
    }

    #[test]
    fn validator_add_rule_triggers_needs_validation() {
        let mut validator = WebValidator::new();
        let rule = WebRule::new(123, WebRuleType::Confidence)
            .with_predicate(456)
            .with_context(789);
        validator.add_rule(rule);
        let quin = NQuin {
            subject: 1, predicate: 456, object: 3, context: 789, metadata: 0, parity: 0
        };
        assert!(validator.needs_validation(&quin));
    }

    #[test]
    fn validate_pass_high_confidence() {
        let mut validator = WebValidator::new();
        validator.set_confidence_threshold(0.5);
        let quin = NQuin {
            subject: 1, predicate: 2, object: 3, context: 0,
            metadata: u32::MAX as u64,
            parity: 0,
        };
        assert_eq!(validator.validate(&quin), WebVerdict::Pass);
    }

    #[test]
    fn validate_fail_zero_confidence() {
        let mut validator = WebValidator::new();
        validator.set_confidence_threshold(0.5);
        let quin = NQuin {
            subject: 1, predicate: 2, object: 3, context: 0, metadata: 0, parity: 0
        };
        assert!(matches!(validator.validate(&quin), WebVerdict::Fail(_)));
    }

    #[test]
    fn validate_embedded_triple_pass() {
        let validator = WebValidator::new();
        let virtual_id = 0x1 | 0x123456789ABCDEF0;
        let components = [1u64, 2, 3];
        assert_eq!(
            validator.validate_embedded_triple(virtual_id, &components, 0),
            WebVerdict::Pass
        );
    }

    #[test]
    fn storage_round_trip() {
        let rules = vec![
            WebRule::new(123, WebRuleType::Confidence).with_confidence(0.8),
            WebRule::new(456, WebRuleType::Domain).with_domain(789),
        ];
        let quins = WebRuleStorage::rules_to_quins(&rules);
        assert_eq!(quins.len(), 2);
        let back = WebRuleStorage::quins_to_rules(&quins);
        assert_eq!(back.len(), 2);
    }

    #[test]
    fn execute_query_program_bind_var() {
        let quin = NQuin {
            subject: 0xAABB, predicate: 0xCCDD, object: 0xEEFF,
            context: 0, metadata: 0, parity: 0,
        };
        // MATCH_SUBJ 0xAABB, then BIND_VAR slot=OBJ reg=0, HALT
        let mut program = vec![OP_MATCH_SUBJ];
        program.extend_from_slice(&0xAABBu64.to_le_bytes());
        program.extend_from_slice(&[OP_BIND_VAR, SLOT_OBJ, 0, OP_HALT]);
        let results = execute_query_program(&program, &[quin]).unwrap();
        assert_eq!(results, vec![0xEEFF]);
    }
}
