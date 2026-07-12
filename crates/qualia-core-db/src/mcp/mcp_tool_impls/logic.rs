use super::*;


pub fn symbolic_logic_infer(args: &[u8]) -> Result<String, McpSystemError> {
    use crate::solvers::symbolic_logic::{
        BoundedSatSolver, Clause, DefeasibleRule, Fact, ForwardChainingDefeasible, Literal,
        RuleType,
    };
    use crate::solvers::SolverConfig;

    let v = parse_tool_args(args)?;
    let solver = json_str(&v, "solver", "defeasible");
    let cfg = SolverConfig {
        max_iterations: v
            .get("max_iterations")
            .and_then(Value::as_u64)
            .unwrap_or(100) as u32,
        tolerance: json_f64(&v, "tolerance", 1e-6),
        step_size: json_f64(&v, "step_size", 0.01),
        verbose: json_bool(&v, "verbose", false),
    };

    if solver == "sat" {
        let mut s = BoundedSatSolver::new(cfg);
        if let Some(clauses) = v.get("clauses").and_then(Value::as_array) {
            for (idx, c) in clauses.iter().enumerate() {
                let lits = c
                    .get("literals")
                    .and_then(Value::as_array)
                    .ok_or(McpSystemError::InvalidParameters)?;
                let mut literals = [Literal {
                    variable: 0,
                    negated: false,
                }; 5];
                let mut n = 0u8;
                for lit in lits.iter().take(5) {
                    literals[n as usize] = Literal {
                        variable: lit.get("variable").and_then(Value::as_u64).unwrap_or(0) as u8,
                        negated: json_bool(lit, "negated", false),
                    };
                    n += 1;
                }
                let clause = Clause {
                    id: idx as u32 + 1,
                    num_literals: n,
                    learned: false,
                    activity: 1.0,
                    literals,
                };
                let _ = s.add_clause(clause);
            }
        }
        match s.solve() {
            Ok(st) => {
                return Ok(json!({
                    "solver": "sat",
                    "satisfiable": st.satisfiable,
                    "num_decisions": st.num_decisions
                })
                .to_string());
            }
            Err(_) => {
                return Ok(json!({"solver": "sat", "satisfiable": false}).to_string());
            }
        }
    }

    let mut s = ForwardChainingDefeasible::new(cfg);
    if let Some(facts) = v.get("facts").and_then(Value::as_array) {
        for (idx, f) in facts.iter().enumerate() {
            let lit = Literal {
                variable: f.get("variable").and_then(Value::as_u64).unwrap_or(0) as u8,
                negated: json_bool(f, "negated", false),
            };
            let fact = Fact {
                id: idx as u32 + 1,
                literal: lit,
                supporting_rules: [0; 3],
                defeated: false,
                confidence: json_f64(f, "confidence", 1.0),
            };
            let _ = s.add_fact(fact);
        }
    }
    if let Some(rules) = v.get("rules").and_then(Value::as_array) {
        for (idx, r) in rules.iter().enumerate() {
            let antecedents_arr = r
                .get("antecedents")
                .and_then(Value::as_array)
                .ok_or(McpSystemError::InvalidParameters)?;
            let mut antecedents = [Literal {
                variable: 0,
                negated: false,
            }; 5];
            for (i, a) in antecedents_arr.iter().take(5).enumerate() {
                antecedents[i] = Literal {
                    variable: a.get("variable").and_then(Value::as_u64).unwrap_or(0) as u8,
                    negated: json_bool(a, "negated", false),
                };
            }
            let cons = r
                .get("consequent")
                .ok_or(McpSystemError::InvalidParameters)?;
            let rule = DefeasibleRule {
                id: idx as u32 + 1,
                rule_type: match json_str(r, "rule_type", "defeasible") {
                    "strict" => RuleType::Strict,
                    "defeater" => RuleType::Defeater,
                    _ => RuleType::Defeasible,
                },
                priority: v.get("priority").and_then(Value::as_u64).unwrap_or(500) as u16,
                active: true,
                fire_count: 0,
                antecedents,
                consequent: Literal {
                    variable: cons.get("variable").and_then(Value::as_u64).unwrap_or(0) as u8,
                    negated: json_bool(cons, "negated", false),
                },
            };
            let _ = s.add_rule(rule);
        }
    }
    match s.infer() {
        Ok(st) => Ok(json!({
            "solver": "defeasible",
            "num_facts": st.num_facts,
            "rules_fired": st.rules_fired
        })
        .to_string()),
        Err(_) => Ok(json!({"solver": "defeasible", "num_facts": 0}).to_string()),
    }
}

pub fn evaluate_modality(args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(args)?;
    let modality = json_str(&v, "modality", "unknown");

    match modality {
        "ltl" => {
            use crate::modalities::temporal_ltl::evaluate_ltl_trace;
            let trace = if let Ok(quins) = parse_quin_slice(&v, "trace") {
                quins
            } else {
                vec![]
            };
            let formula =
                parse_ltl_formula(v.get("formula").ok_or(McpSystemError::InvalidParameters)?)?;
            let ok = evaluate_ltl_trace(&trace, &formula);
            Ok(json!({"modality": "ltl", "result": ok}).to_string())
        }
        "asp" => {
            // Real answer-set (stable-model) semantics via the Gelfond-Lifschitz
            // reduct, NOT the legacy context-bifurcation heuristic. Input is a
            // normal logic program: `atoms` (the Herbrand base as u64 atom ids)
            // and `rules`, each `{ "head": u64, "pos": [u64..], "neg": [u64..] }`
            // encoding `head :- pos.., not neg..`. `head == 0` encodes an
            // integrity constraint `:- pos.., not neg..`.
            use crate::modalities::asp::{compute_answer_sets, AspRule, ASP_MAX_ATOMS};
            let u64_array = |val: Option<&serde_json::Value>| -> Vec<u64> {
                val.and_then(|a| a.as_array())
                    .map(|arr| arr.iter().filter_map(|x| x.as_u64()).collect())
                    .unwrap_or_default()
            };
            let atoms = u64_array(v.get("atoms"));
            let mut rules: Vec<AspRule> = Vec::new();
            if let Some(arr) = v.get("rules").and_then(|r| r.as_array()) {
                for r in arr {
                    let head = r.get("head").and_then(|h| h.as_u64()).unwrap_or(0);
                    let pos = u64_array(r.get("pos"));
                    let neg = u64_array(r.get("neg"));
                    rules.push(AspRule::new(head, &pos, &neg));
                }
            }
            let mut out = [0u64; 64];
            let count = compute_answer_sets(&atoms, &rules, &mut out);
            // Decode each answer-set bitmask (bit i ⇔ atoms[i] present) into the
            // atom ids it contains. Bounded to ASP_MAX_ATOMS to avoid shift OOB.
            let answer_sets: Vec<Vec<u64>> = out[..count]
                .iter()
                .map(|&mask| {
                    atoms
                        .iter()
                        .take(ASP_MAX_ATOMS)
                        .enumerate()
                        .filter(|(i, _)| mask & (1u64 << i) != 0)
                        .map(|(_, &a)| a)
                        .collect()
                })
                .collect();
            Ok(json!({
                "modality": "asp",
                "answer_set_count": count,
                "answer_sets": answer_sets
            })
            .to_string())
        }
        "probabilistic" => {
            use crate::modalities::probabilistic::evaluate_threshold;
            let value = json_f64(&v, "value", 0.5);
            let threshold = json_f64(&v, "threshold", 0.4);
            Ok(json!({
                "modality": "probabilistic",
                "result": evaluate_threshold(value as f32, threshold as f32)
            })
            .to_string())
        }
        "argumentation" => {
            use crate::modalities::argumentation::ArgumentationFramework;
            let fw = ArgumentationFramework::new();
            Ok(json!({
                "modality": "argumentation",
                "grounded_extension_size": fw.grounded_extension().len()
            })
            .to_string())
        }
        "deontic" => {
            use crate::modalities::logic::deontic::{evaluate_deontic_contract, DeonticVerdict};
            let quins = parse_quin_slice(&v, "quins")?;
            let mut quins = quins;
            for q in &mut quins {
                ensure_parity(q);
            }
            let now = v.get("now_unix").and_then(Value::as_u64).unwrap_or(0) as u32;
            let mut out = vec![DeonticVerdict::default(); quins.len().max(1)];
            let n = evaluate_deontic_contract(&quins, now, &mut out)
                .map_err(|_| McpSystemError::InvalidParameters)?;
            let verdicts: Vec<Value> = out[..n]
                .iter()
                .map(|ver| {
                    json!({
                        "status": format!("{:?}", ver.status),
                        "opcode": ver.opcode
                    })
                })
                .collect();
            Ok(
                json!({"modality": "deontic", "verdict_count": n, "verdicts": verdicts})
                    .to_string(),
            )
        }
        "epistemic" => {
            use crate::modalities::epistemic::{evaluate_epistemic_frame, EpistemicVerdict};
            let quins = parse_quin_slice(&v, "quins")?;
            let agent = json_u64(&v, "agent_did_hash", 0);
            let world = json_u64(&v, "world_hash", 0);
            let mut out = vec![
                EpistemicVerdict {
                    claim: NQuin::default(),
                    status: crate::modalities::epistemic::EpistemicStatus::Skipped,
                    certainty: 0,
                };
                quins.len().max(1)
            ];
            let n = evaluate_epistemic_frame(&quins, agent, world, &mut out)
                .map_err(|_| McpSystemError::InvalidParameters)?;
            let verdicts: Vec<Value> = out[..n]
                .iter()
                .map(|ver| {
                    json!({
                        "status": format!("{:?}", ver.status),
                        "certainty": ver.certainty
                    })
                })
                .collect();
            Ok(
                json!({"modality": "epistemic", "verdict_count": n, "verdicts": verdicts})
                    .to_string(),
            )
        }
        "dl" => {
            use crate::modalities::dl::check_subsumption_quin;
            let sub = json_u64(&v, "sub_class_hash", 0);
            let sup = json_u64(&v, "super_class_hash", 0);
            let tbox = parse_quin_slice(&v, "tbox").unwrap_or_default();
            Ok(json!({
                "modality": "dl",
                "subsumed": check_subsumption_quin(sub, sup, &tbox)
            })
            .to_string())
        }
        "paraconsistent" => {
            use crate::modalities::paraconsistent::route_paraconsistent;
            let quins = parse_quin_slice(&v, "quins")?;
            let mut consistent = vec![NQuin::default(); quins.len().max(8)];
            let mut isolated = vec![NQuin::default(); quins.len().max(8)];
            let (c, i) = route_paraconsistent(&quins, &mut consistent, &mut isolated)
                .map_err(|_| McpSystemError::InvalidParameters)?;
            Ok(json!({
                "modality": "paraconsistent",
                "consistent_count": c,
                "isolated_count": i
            })
            .to_string())
        }
        _ => Err(McpSystemError::InvalidParameters),
    }
}

fn parse_ltl_formula(
    v: &Value,
) -> Result<crate::modalities::temporal_ltl::LtlFormula, McpSystemError> {
    use crate::modalities::temporal_ltl::LtlFormula;
    let ty = v
        .get("type")
        .and_then(Value::as_str)
        .ok_or(McpSystemError::InvalidParameters)?;
    let pred = |key: &str| json_u64(v, key, 0);
    Ok(match ty {
        "globally" | "G" => LtlFormula::Globally(pred("predicate")),
        "finally" | "F" => LtlFormula::Finally(pred("predicate")),
        "next" | "X" => LtlFormula::Next(pred("predicate")),
        "until" | "U" => LtlFormula::Until {
            ante: pred("ante"),
            consequent: pred("consequent"),
        },
        "release" | "R" => LtlFormula::Release {
            trigger: pred("trigger"),
            invariant: pred("invariant"),
        },
        _ => return Err(McpSystemError::InvalidParameters),
    })
}

/// Load N3 rules into a `RuleEngine`, fire them in the Webizen VM, and evaluate
/// a Quin against the fired conclusions. Emits WAL audit events for each result.
///
/// Args:
/// ```json
/// {
///   "n3_source": "{ ex:A q42:p ex:B } => { ex:A q42:q ex:C } .",
///   "quin": { "subject": 123, "predicate": 456, "object": 789, "context": 0 },
///   "ruleset_name": "default",
///   "contract_hash": 0
/// }
/// ```
///
/// Returns per-rule pass/fail verdicts as JSON.
pub fn evaluate_logic_rules(args: &[u8]) -> Result<String, McpSystemError> {
    use crate::modalities::logic::rules::RuleEngine;

    let v = parse_tool_args(args)?;
    let n3_source = v
        .get("n3_source")
        .and_then(Value::as_str)
        .ok_or(McpSystemError::InvalidParameters)?;
    let quin_obj = v.get("quin").ok_or(McpSystemError::InvalidParameters)?;
    let subject = json_u64(quin_obj, "subject", 0);
    let predicate = json_u64(quin_obj, "predicate", 0);
    let object = json_u64(quin_obj, "object", 0);
    let context = json_u64(quin_obj, "context", 0);
    let ruleset_name = json_str(&v, "ruleset_name", "default");
    let contract_hash = json_u64(&v, "contract_hash", 0);

    let mut engine = RuleEngine::with_contract(contract_hash);
    let rules_loaded = engine.load_n3(ruleset_name, n3_source);

    let quin = NQuin {
        subject,
        predicate,
        object,
        context,
        metadata: 0,
        parity: subject ^ predicate ^ object ^ context,
    };

    let results = engine.evaluate(&quin);
    let passed_count = results.iter().filter(|r| r.passed).count();
    let results_json: Vec<Value> = results
        .iter()
        .map(|r| {
            json!({
                "ruleset_name": r.ruleset_name,
                "rule_name": r.rule_name,
                "passed": r.passed,
                "message": r.message,
            })
        })
        .collect();

    Ok(json!({
        "rules_loaded": rules_loaded,
        "ruleset_name": ruleset_name,
        "contract_hash": contract_hash,
        "input_quin": {
            "subject": subject,
            "predicate": predicate,
            "object": object,
            "context": context,
        },
        "total_results": results_json.len(),
        "passed_count": passed_count,
        "failed_count": results_json.len() - passed_count,
        "results": results_json,
    })
    .to_string())
}
