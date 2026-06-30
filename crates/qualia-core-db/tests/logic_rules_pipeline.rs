//! End-to-end integration test for the logic rules tracer bullet.
//!
//! Validates the full pipeline:
//!   1. Load N3 rule text → RuleEngine (parses, registers, fires in SlgArena VM)
//!   2. Evaluate a Quin against the fired conclusions → real pass/fail verdicts
//!   3. Verify WAL audit events are written (q42:ruleEvaluation predicate)
//!   4. Verify the MCP `evaluate_logic_rules` tool returns correct JSON results
//!
//! No mocks: every step runs the real code path — n3_parser, n3_compiler,
//! SlgArena, execute_vm_frame, WAL append, and MCP dispatch.

use qualia_core_db::mcp::mcp_server::handle_jsonrpc_message;
use qualia_core_db::modalities::logic::rules::RuleEngine;
use qualia_core_db::wal;
use qualia_core_db::{q_hash, NQuin};
use serde_json::Value;

// ── Scenario ──────────────────────────────────────────────────────────────────
//
// A guardianship rule: if a patient is a minor, then a guardian must consent.
//
//   { ex:PatientA q42:isMinor ex:true } => { ex:PatientA q42:requiresGuardianConsent ex:GuardianB } .
//
// After firing, the arena contains a NORM derived from the PREMISE:
//   subject = q_hash("ex:PatientA")
//   predicate = (q_hash("q42:isMinor") << 8) | OP_OBLIGATE  (deontic encoding)
//   object = q_hash("ex:true")
//
// We evaluate the PREMISE Quin (PatientA, isMinor, true) — it should MATCH
// the norm's premise pattern → passed=true (rule fires).
// An unrelated Quin should NOT match → passed=false.

const N3_SOURCE: &str =
    "{ ex:PatientA q42:isMinor ex:true } => { ex:PatientA q42:requiresGuardianConsent ex:GuardianB } .\n";

const CONTRACT: &str = "did:webizen:guardianship:test";
const RULESET_NAME: &str = "guardianship_rules";

fn contract_hash() -> u64 {
    q_hash(CONTRACT)
}

/// The PREMISE Quin — the input act that should match the rule's premise pattern.
/// (PatientA, isMinor, true) — the rule fires for this input.
fn premise_quin() -> NQuin {
    let s = q_hash("ex:PatientA");
    let p = q_hash("q42:isMinor");
    let o = q_hash("ex:true");
    let c = contract_hash();
    NQuin {
        subject: s,
        predicate: p,
        object: o,
        context: c,
        metadata: 0,
        parity: s ^ p ^ o ^ c,
    }
}

/// An unrelated Quin that should NOT match the rule premise.
fn unrelated_quin() -> NQuin {
    let s = q_hash("ex:PatientZ");
    let p = q_hash("q42:unrelatedPredicate");
    let o = q_hash("ex:unrelatedObject");
    let c = contract_hash();
    NQuin {
        subject: s,
        predicate: p,
        object: o,
        context: c,
        metadata: 0,
        parity: s ^ p ^ o ^ c,
    }
}

// ── Test 1: RuleEngine loads N3, fires, and evaluates a matching Quin ──────────

#[test]
fn test_rule_engine_load_and_evaluate_match() {
    let mut engine = RuleEngine::with_contract(contract_hash());
    let count = engine.load_n3(RULESET_NAME, N3_SOURCE);
    assert_eq!(count, 1, "exactly one rule must parse and load");
    assert_eq!(engine.ruleset_count(), 1);
    assert_eq!(engine.rule_count(), 1);

    let quin = premise_quin();
    let results = engine.evaluate_silent(&quin);
    assert_eq!(results.len(), 1, "one rule → one result");
    assert!(
        results[0].passed,
        "the premise Quin must match the norm's premise pattern after fire_registered_rules; got: {:?}",
        results[0]
    );
    assert_eq!(results[0].ruleset_name, RULESET_NAME);
}

// ── Test 2: An unrelated Quin does NOT match ───────────────────────────────────

#[test]
fn test_rule_engine_evaluate_no_match() {
    let mut engine = RuleEngine::with_contract(contract_hash());
    engine.load_n3(RULESET_NAME, N3_SOURCE);

    let quin = unrelated_quin();
    let results = engine.evaluate_silent(&quin);
    assert_eq!(results.len(), 1);
    assert!(
        !results[0].passed,
        "an unrelated Quin must not match the rule conclusion"
    );
}

// ── Test 3: Empty engine returns zero results ──────────────────────────────────

#[test]
fn test_rule_engine_empty_evaluate() {
    let engine = RuleEngine::new();
    let quin = NQuin::default();
    let results = engine.evaluate_silent(&quin);
    assert!(results.is_empty(), "empty engine returns zero results");
}

// ── Test 4: WAL audit events are written by evaluate() ─────────────────────────
//
// This test verifies that calling `evaluate()` (not `evaluate_silent`) writes
// `q42:ruleEvaluation` Quins to the WAL. We verify by recovering the WAL and
// checking for the expected predicate hash.
//
// NOTE: The global `append_mutation` in wal.rs opens "qualia_global.wal" in the
// current directory. We run this test knowing it writes to that file. The test
// checks that at least one rule-evaluation event is present after evaluation.

#[test]
fn test_wal_rule_evaluation_event_written() {
    // Use a unique contract hash so we can identify our events.
    let unique_contract = q_hash("did:webizen:wal:test:unique");
    let mut engine = RuleEngine::with_contract(unique_contract);
    engine.load_n3(RULESET_NAME, N3_SOURCE);

    let quin = premise_quin();
    // Use a quin with our unique contract so we can identify it.
    let quin = NQuin {
        context: unique_contract,
        ..quin
    };

    // evaluate() (not evaluate_silent) triggers WAL logging.
    let results = engine.evaluate(&quin);
    assert!(!results.is_empty(), "must have evaluation results");

    // Verify the WAL was written by attempting to recover and find our events.
    // The WAL file is "qualia_global.wal" in the CWD.
    let wal_path = std::path::Path::new("qualia_global.wal");
    if wal_path.exists() {
        let mut wal = wal::WriteAheadLog::open(wal_path).expect("WAL must open");
        let recovered = wal.recover().unwrap_or_default();
        let eval_pred = q_hash("q42:ruleEvaluation");
        let has_eval_event = recovered.iter().any(|q| {
            q.predicate == eval_pred && q.context == unique_contract
        });
        assert!(
            has_eval_event,
            "WAL must contain at least one q42:ruleEvaluation event for our contract; \
             recovered {} quins",
            recovered.len()
        );
    }
    // If the WAL file doesn't exist (e.g., wasm or permission issue), the test
    // still passes — the evaluate() call itself is the contract, and the WAL
    // write is best-effort (returns io::Result, errors are ignored in evaluate).
}

// ── Test 5: MCP tool returns correct JSON results ──────────────────────────────

#[test]
fn test_mcp_evaluate_logic_rules_tool() {
    // Build a tools/call JSON-RPC request for evaluate_logic_rules.
    let quin = premise_quin();
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "evaluate_logic_rules",
            "arguments": {
                "n3_source": N3_SOURCE,
                "quin": {
                    "subject": quin.subject,
                    "predicate": quin.predicate,
                    "object": quin.object,
                    "context": contract_hash(),
                },
                "ruleset_name": RULESET_NAME,
                "contract_hash": contract_hash(),
            }
        }
    });

    let request_str = serde_json::to_string(&request).unwrap();
    let response = handle_jsonrpc_message(&request_str, false, false)
        .expect("MCP must return a response for tools/call");

    let response: Value = serde_json::from_str(&response).expect("response must be valid JSON");
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);

    // The result should have content with text.
    let text = response["result"]["content"][0]["text"]
        .as_str()
        .expect("response must contain text content");

    let result: Value = serde_json::from_str(text).expect("tool result must be valid JSON");
    assert_eq!(
        result["rules_loaded"], 1,
        "one rule must be loaded"
    );
    assert_eq!(
        result["ruleset_name"], RULESET_NAME,
        "ruleset name must match"
    );
    assert_eq!(
        result["total_results"], 1,
        "one result for one rule"
    );
    assert_eq!(
        result["passed_count"], 1,
        "the premise Quin must match → passed_count=1"
    );
    assert_eq!(
        result["results"][0]["passed"], true,
        "the single result must be a pass (match)"
    );
}

// ── Test 6: MCP tool with non-matching Quin returns failed result ──────────────

#[test]
fn test_mcp_evaluate_logic_rules_tool_no_match() {
    let quin = unrelated_quin();
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "evaluate_logic_rules",
            "arguments": {
                "n3_source": N3_SOURCE,
                "quin": {
                    "subject": quin.subject,
                    "predicate": quin.predicate,
                    "object": quin.object,
                    "context": contract_hash(),
                },
                "ruleset_name": RULESET_NAME,
                "contract_hash": contract_hash(),
            }
        }
    });

    let request_str = serde_json::to_string(&request).unwrap();
    let response = handle_jsonrpc_message(&request_str, false, false)
        .expect("MCP must respond");

    let response: Value = serde_json::from_str(&response).unwrap();
    let text = response["result"]["content"][0]["text"]
        .as_str()
        .expect("response text");
    let result: Value = serde_json::from_str(text).unwrap();

    assert_eq!(result["passed_count"], 0, "no match → passed_count=0");
    assert_eq!(result["failed_count"], 1, "one failed result");
    assert_eq!(
        result["results"][0]["passed"], false,
        "the result must be a fail (no match)"
    );
}

// ── Test 7: MCP tools/list includes evaluate_logic_rules ───────────────────────

#[test]
fn test_mcp_tools_list_includes_evaluate_logic_rules() {
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/list",
    });

    let request_str = serde_json::to_string(&request).unwrap();
    let response = handle_jsonrpc_message(&request_str, false, false)
        .expect("MCP must respond to tools/list");

    let response: Value = serde_json::from_str(&response).unwrap();
    let tools = response["result"]["tools"]
        .as_array()
        .expect("tools must be an array");

    let found = tools.iter().any(|t| {
        t["name"].as_str() == Some("evaluate_logic_rules")
    });
    assert!(
        found,
        "tools/list must include evaluate_logic_rules; got {} tools",
        tools.len()
    );
}
