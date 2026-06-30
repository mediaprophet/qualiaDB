//! Rules Module — the RuleEngine bridge between N3 text and the live Webizen VM.
//!
//! This module is the public API surface for loading N3 rules, evaluating them
//! against live Quins through the real `SlgArena` → `execute_vm_frame` pipeline,
//! and emitting WAL audit events for every evaluation.
//!
//! ## Pipeline
//! ```text
//!   N3 text
//!     -> n3_parser::parse_all           (parse rules)
//!     -> RuleEngine::load_n3            (register into internal SlgArena)
//!     -> SlgArena::fire_registered_rules (compile to norms + bytecode, execute)
//!     -> RuleEngine::evaluate(quin)     (match quin against fired conclusions)
//!     -> wal::log_rule_evaluation       (durable audit event)
//!     -> Vec<RuleResult>                (public result)
//! ```

use crate::modalities::logic::n3_parser::{N3Event, N3Parser};
use crate::wal;
use crate::{q_hash, NQuin};

/// GuardianShip ruleset identifier
pub const GUARDIANSHIP_RULESET: &str = "guardianship_rules";

/// WAL predicate hash for rule-evaluation audit events.
const RULE_EVAL_PREDICATE: u64 = q_hash("q42:ruleEvaluation");

/// Rule engine for evaluating rule-based constraints against live Quins.
///
/// Internally owns a `SlgArena` (the 42MB Webizen VM) into which parsed N3 rules
/// are registered and fired. The `evaluate` method matches an input Quin against
/// the conclusions asserted by fired rules, returning real pass/fail verdicts.
pub struct RuleEngine {
    /// Named rulesets — each holds the raw N3 source text for traceability.
    rulesets: Vec<RuleSet>,
    /// The live Webizen VM arena. Rules are registered + fired here on load.
    arena: crate::governance::webizen::SlgArena,
    /// Contract graph hash used when compiling rules to norms.
    contract_hash: u64,
}

/// A set of rules that can be applied to Quin data.
///
/// Stores the raw N3 source text so the ruleset can be re-parsed and re-fired
/// if the engine is reset or the contract context changes.
pub struct RuleSet {
    pub name: String,
    pub rules: Vec<Rule>,
    /// The raw N3 source text this ruleset was loaded from (for audit/replay).
    pub n3_source: String,
}

/// Individual rule definition (metadata — the actual logic lives in the VM).
pub struct Rule {
    pub name: String,
    pub condition: String,
    pub action: String,
}

/// Result of evaluating a single rule against a Quin.
#[derive(Debug, Clone)]
pub struct RuleResult {
    /// The name of the ruleset this result comes from.
    pub ruleset_name: String,
    /// The rule name (or N3 rule id if annotated).
    pub rule_name: String,
    /// `true` when the input Quin matches the rule's conclusion pattern.
    pub passed: bool,
    /// Human-readable detail: match, no-match, or error.
    pub message: String,
}

impl RuleEngine {
    /// Create a new rule engine with an empty arena.
    pub fn new() -> Self {
        Self {
            rulesets: Vec::new(),
            arena: crate::governance::webizen::SlgArena::new(),
            contract_hash: 0,
        }
    }

    /// Create a new rule engine bound to a specific contract graph hash.
    ///
    /// The contract hash is used when compiling N3 rules to deontic norms —
    /// it becomes the `context` field of the compiled norm Quin.
    pub fn with_contract(contract_hash: u64) -> Self {
        Self {
            rulesets: Vec::new(),
            arena: crate::governance::webizen::SlgArena::new(),
            contract_hash,
        }
    }

    /// Add a pre-built ruleset to the engine and fire its rules in the VM.
    pub fn add_ruleset(&mut self, ruleset: RuleSet) {
        self.parse_register_and_fire(&ruleset.n3_source);
        self.rulesets.push(ruleset);
    }

    /// Load an N3 source string as a new ruleset, parse it, register and fire
    /// the rules in the internal VM arena.
    ///
    /// Returns the number of rules parsed and fired.
    pub fn load_n3(&mut self, name: &str, n3_source: &str) -> usize {
        // Parse and register in one step — we can't return borrowed N3Rule<'a>
        // and also mutate self.rulesets, so we extract metadata here.
        let rule_metas = self.parse_register_and_fire(n3_source);
        let ruleset = RuleSet {
            name: name.to_string(),
            rules: rule_metas
                .into_iter()
                .map(|(id, premise_debug, conclusion_debug)| Rule {
                    name: id,
                    condition: premise_debug,
                    action: conclusion_debug,
                })
                .collect(),
            n3_source: n3_source.to_string(),
        };
        let count = ruleset.rules.len();
        self.rulesets.push(ruleset);
        count
    }

    /// Parse N3 text, register all logic rules into the arena, and fire them.
    /// Returns metadata tuples (id, premise_debug, conclusion_debug) for each rule.
    fn parse_register_and_fire(&mut self, n3_source: &str) -> Vec<(String, String, String)> {
        let mut parser = N3Parser::new(n3_source);
        let mut rules = Vec::new();
        parser
            .parse_all(|event| {
                if let N3Event::LogicRule(rule) = event {
                    rules.push(rule);
                }
                Ok(())
            })
            .expect("N3 source must parse cleanly for RuleEngine::load_n3");

        // Extract metadata before registering (registration doesn't consume the rule).
        let metas: Vec<(String, String, String)> = rules
            .iter()
            .map(|r| {
                (
                    r.id.unwrap_or("unnamed").to_string(),
                    format!("{:?}", r.premise),
                    format!("{:?}", r.conclusion),
                )
            })
            .collect();

        for rule in &rules {
            self.arena.register_rule(rule);
        }
        self.arena.fire_registered_rules(self.contract_hash);
        metas
    }

    /// Get a ruleset by name.
    pub fn get_ruleset(&self, name: &str) -> Option<&RuleSet> {
        self.rulesets.iter().find(|r| r.name == name)
    }

    /// Evaluate all rulesets against a Quin by checking whether the Quin
    /// matches the premise pattern of any fired rule (norm) in the arena.
    ///
    /// After `fire_registered_rules`, the arena contains deontic norms derived
    /// from rule premises. Each norm's predicate encodes the property path
    /// (shifted left 8 bits) and a deontic opcode in the low byte. This method
    /// extracts the property path and checks whether the input Quin's
    /// (subject, predicate, object) matches any norm's premise pattern.
    ///
    /// Each evaluation emits a `q42:ruleEvaluation` WAL audit event.
    /// Returns one `RuleResult` per rule across all rulesets.
    pub fn evaluate(&self, quin: &NQuin) -> Vec<RuleResult> {
        let results = self.evaluate_silent(quin);
        // Emit a WAL audit event for this evaluation.
        let _ = wal::log_rule_evaluation(quin, &results, self.contract_hash);
        results
    }

    /// Evaluate without WAL logging (for hot paths or testing).
    ///
    /// Collects active norms from the arena, extracts each norm's premise
    /// pattern (subject, property_path, object), and checks whether the input
    /// Quin matches. A rule "passes" when its norm's premise pattern matches
    /// the input Quin — meaning the rule fires for this input.
    pub fn evaluate_silent(&self, quin: &NQuin) -> Vec<RuleResult> {
        // Collect active quins (norms) from the arena.
        let mut active = [NQuin::default(); 512];
        let live_count = self.arena.collect_active_quins(&mut active);
        let live_quins = &active[..live_count];

        let mut results = Vec::new();
        for ruleset in &self.rulesets {
            for rule in &ruleset.rules {
                // Check if the input Quin matches any norm's premise pattern.
                // A norm's predicate is (property_path_hash << 8) | opcode,
                // so we extract the property path by shifting right 8 bits.
                // The shift loses the top 8 bits of the original hash, so we
                // mask the input quin's predicate to the same 56-bit width.
                let matched = live_quins.iter().any(|norm| {
                    let opcode = (norm.predicate & 0xFF) as u8;
                    // Only deontic norms (opcode 0x10-0x12) are rule-derived.
                    if opcode < 0x10 || opcode > 0x12 {
                        return false;
                    }
                    let norm_property_path = norm.predicate >> 8;
                    // The deontic encoding stores the path in bits [8..62] (55 bits)
                    // and clears bit 63 (DEFEATER_BIT). After >> 8, the path occupies
                    // bits [0..54]. Mask the input to the same 55-bit width.
                    let quin_property_path = quin.predicate & 0x007F_FFFF_FFFF_FFFF;
                    norm.subject == quin.subject
                        && norm_property_path == quin_property_path
                        && norm.object == quin.object
                });

                results.push(RuleResult {
                    ruleset_name: ruleset.name.clone(),
                    rule_name: rule.name.clone(),
                    passed: matched,
                    message: if matched {
                        "quin matches rule premise (rule fires)".to_string()
                    } else {
                        "no match".to_string()
                    },
                });
            }
        }
        results
    }

    /// Number of rulesets loaded.
    pub fn ruleset_count(&self) -> usize {
        self.rulesets.len()
    }

    /// Total number of rules across all rulesets.
    pub fn rule_count(&self) -> usize {
        self.rulesets.iter().map(|rs| rs.rules.len()).sum()
    }

    /// Number of rules currently registered in the VM arena.
    pub fn arena_rule_count(&self) -> usize {
        self.arena.rule_count()
    }
}

impl Default for RuleEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guardianship_ruleset_constant() {
        assert_eq!(GUARDIANSHIP_RULESET, "guardianship_rules");
    }

    #[test]
    fn test_rule_engine_creation() {
        let engine = RuleEngine::new();
        assert_eq!(engine.ruleset_count(), 0);
    }

    #[test]
    fn test_add_ruleset() {
        let mut engine = RuleEngine::new();
        let ruleset = RuleSet {
            name: "test_ruleset".to_string(),
            rules: vec![],
            n3_source: String::new(),
        };
        engine.add_ruleset(ruleset);
        assert_eq!(engine.ruleset_count(), 1);
    }

    #[test]
    fn test_get_ruleset() {
        let mut engine = RuleEngine::new();
        let ruleset = RuleSet {
            name: "test_ruleset".to_string(),
            rules: vec![],
            n3_source: String::new(),
        };
        engine.add_ruleset(ruleset);

        let retrieved = engine.get_ruleset("test_ruleset");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "test_ruleset");
    }

    #[test]
    fn test_load_n3_and_evaluate_match() {
        // A ground Strict rule: if AcmeCorp forbids the dignity right,
        // then AcmeCorp triggers a personhood error.
        // After firing, the arena contains a NORM from the PREMISE:
        //   (AcmeCorp, forbids, DignityRight) with deontic opcode packed in.
        let n3 = "{ ex:AcmeCorp q42:forbids ex:DignityRight } => \
                  { ex:AcmeCorp q42:triggers ex:PersonhoodError } .\n";
        let mut engine = RuleEngine::with_contract(q_hash("did:webizen:test:contract"));
        let count = engine.load_n3("guardianship", n3);
        assert_eq!(count, 1, "one rule must parse and register");

        // The PREMISE quin: (AcmeCorp, forbids, DignityRight) — matches the norm.
        let premise_quin = NQuin {
            subject: q_hash("ex:AcmeCorp"),
            predicate: q_hash("q42:forbids"),
            object: q_hash("ex:DignityRight"),
            context: q_hash("did:webizen:test:contract"),
            metadata: 0,
            parity: 0,
        };

        let results = engine.evaluate_silent(&premise_quin);
        assert_eq!(results.len(), 1);
        // The premise quin must match the norm's premise pattern.
        assert!(
            results[0].passed,
            "the premise quin must match the norm after fire_registered_rules; got: {:?}",
            results[0]
        );
    }

    #[test]
    fn test_load_n3_and_evaluate_no_match() {
        let n3 = "{ ex:AcmeCorp q42:forbids ex:DignityRight } => \
                  { ex:AcmeCorp q42:triggers ex:PersonhoodError } .\n";
        let mut engine = RuleEngine::with_contract(q_hash("did:webizen:test:contract"));
        engine.load_n3("guardianship", n3);

        // An unrelated quin that does NOT match the premise.
        let unrelated = NQuin {
            subject: q_hash("ex:SomeOtherCorp"),
            predicate: q_hash("q42:unrelated"),
            object: q_hash("ex:SomethingElse"),
            context: q_hash("did:webizen:test:contract"),
            metadata: 0,
            parity: 0,
        };

        let results = engine.evaluate_silent(&unrelated);
        assert_eq!(results.len(), 1);
        assert!(
            !results[0].passed,
            "an unrelated quin must not match the rule premise"
        );
    }

    #[test]
    fn test_empty_engine_evaluate() {
        let engine = RuleEngine::new();
        let quin = NQuin::default();
        let results = engine.evaluate_silent(&quin);
        assert_eq!(results.len(), 0, "empty engine returns zero results");
    }
}
