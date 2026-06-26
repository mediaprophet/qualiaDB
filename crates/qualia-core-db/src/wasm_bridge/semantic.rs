//! WASM-bindgen API — semantic domain (split from wasm_bridge.rs; verbatim, no behaviour change).
//! WASM-bindgen API surface — exposes Qualia engine functions to JavaScript.
//!
//! All functions are `#[cfg(target_arch = "wasm32")]` and only compiled into
//! the browser/OPFS build.  Native desktop builds use direct Rust FFI.

#[cfg(target_arch = "wasm32")]
use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

// ─── Economics: Monte Carlo VaR ──────────────────────────────────────────────
#[cfg(target_arch = "wasm32")]
use super::*;


// ─── SHACL: inline constraint validation ─────────────────────────────────────

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct ShaclValidateParams {
    pub constraint_type: String,
    pub value: f64,
    pub target_value: f64,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn validate_shacl_constraint_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    let p: ShaclValidateParams = serde_wasm_bindgen::from_value(val)?;
    let compiler = crate::modalities::logic::shacl::ShaclCompiler::new();
    let shape = compiler.compile(
        crate::modalities::logic::shacl::ShaclTarget::TargetNode("wasm:target".to_string()),
        "wasm:property",
        match p.constraint_type.as_str() { "minInclusive" => crate::modalities::logic::shacl::ShaclConstraint::MinInclusive(p.value), "maxInclusive" => crate::modalities::logic::shacl::ShaclConstraint::MaxInclusive(p.value), "minExclusive" => crate::modalities::logic::shacl::ShaclConstraint::MinExclusive(p.value), "maxExclusive" => crate::modalities::logic::shacl::ShaclConstraint::MaxExclusive(p.value), "minCount" => crate::modalities::logic::shacl::ShaclConstraint::MinCount(p.value as u32), "maxCount" => crate::modalities::logic::shacl::ShaclConstraint::MaxCount(p.value as u32), "minLength" => crate::modalities::logic::shacl::ShaclConstraint::MinLength(p.value as u32), "maxLength" => crate::modalities::logic::shacl::ShaclConstraint::MaxLength(p.value as u32), _ => crate::modalities::logic::shacl::ShaclConstraint::MinInclusive(p.value), },
        crate::modalities::logic::shacl::ShaclSeverity::Violation,
    );
    let passes = shape.evaluate_numeric(p.target_value);
    #[derive(Serialize)]
    struct ValidationOut {
        passes: bool,
        constraint_type: String,
        value: f64,
        target_value: f64,
    }
    Ok(serde_wasm_bindgen::to_value(&ValidationOut {
        passes,
        constraint_type: p.constraint_type,
        value: p.value,
        target_value: p.target_value,
    })?)
}

// ─── Query Engine & Ingestion Formats ────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn execute_ntriples_query(query: &str, db_bytes: &[u8], max_results: usize) -> String {
    let mut program = [0u8; 1024];
    if crate::mini_parser::compile_ntriples_to_bytecode(query.as_bytes(), &mut program).is_err() {
        return r#"{"error": "Malformed query or program too large"}"#.to_string();
    }

    if db_bytes.len() % 48 != 0 {
        return r#"{"error": "db_bytes length must be a multiple of 48"}"#.to_string();
    }
    let quins = unsafe {
        std::slice::from_raw_parts(
            db_bytes.as_ptr() as *const crate::NQuin,
            db_bytes.len() / 48,
        )
    };

    let mut out = vec![crate::NQuin::default(); max_results];
    match crate::webizen_bytecode::execute_program_with_stats(&program, quins, &mut out, None) {
        Ok(stats) => {
            #[derive(Serialize)]
            struct MatchOut {
                s: String,
                p: String,
                o: String,
                c: String,
                m: String,
            }
            let mut matches = Vec::new();
            for i in 0..stats.match_count {
                matches.push(MatchOut {
                    s: out[i].subject.to_string(),
                    p: out[i].predicate.to_string(),
                    o: out[i].object.to_string(),
                    c: out[i].context.to_string(),
                    m: out[i].metadata.to_string(),
                });
            }
            #[derive(Serialize)]
            struct Res {
                matches: Vec<MatchOut>,
                vm_cycles: u64,
                direct_jump_ops: u64,
                lexicon_lookup_ops: u64,
            }

            serde_json::to_string(&Res {
                matches,
                vm_cycles: stats.vm_cycles,
                direct_jump_ops: stats.direct_jump_ops,
                lexicon_lookup_ops: stats.lexicon_lookup_ops,
            })
            .unwrap_or_else(|_| "{}".to_string())
        }
        Err(_) => r#"{"error": "VM execution error"}"#.to_string(),
    }
}

/// Compiles a query string (SPARQL WHERE-clause or N-Triples pattern) to a JSON
/// description of the Webizen VM bytecode program.  Useful for playground inspection
/// and benchmarking the compilation pipeline without supplying a database.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn compile_query_to_json(query: &str) -> String {
    use crate::query_compiler::QueryCompiler;

    #[derive(Serialize)]
    struct InstructionOut {
        op: String,
    }
    #[derive(Serialize)]
    struct ProgramOut {
        source: &'static str,
        compiled_len: usize,
        instructions: Vec<InstructionOut>,
    }

    // Try SPARQL / JSON-LD / N3 path first (has WHERE { } block)
    let bytecode = QueryCompiler::compile_to_bytecode(query);
    if !bytecode.is_empty() {
        let instructions: Vec<InstructionOut> = bytecode
            .iter()
            .map(|op| InstructionOut {
                op: format!("{:?}", op),
            })
            .collect();
        let compiled_len = instructions.len();
        return serde_json::to_string(&ProgramOut {
            source: "query_compiler",
            compiled_len,
            instructions,
        })
        .unwrap_or_else(|_| r#"{"error":"serialization failed"}"#.to_string());
    }

    // Fall back to N-Triples mini_parser pattern
    let mut program = [0u8; 1024];
    match crate::mini_parser::compile_ntriples_to_bytecode(query.as_bytes(), &mut program) {
        Ok(len) => {
            let instructions: Vec<InstructionOut> = program[..len]
                .iter()
                .enumerate()
                .map(|(i, &b)| InstructionOut {
                    op: format!("byte[{}]={:#04x}", i, b),
                })
                .collect();
            serde_json::to_string(&ProgramOut {
                source: "mini_parser",
                compiled_len: len,
                instructions,
            })
            .unwrap_or_else(|_| r#"{"error":"serialization failed"}"#.to_string())
        }
        Err(e) => format!(r#"{{"error":"compilation failed: {:?}"}}"#, e),
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn parse_turtle_wasm(payload: &str) -> JsValue {
    use rio_api::parser::TriplesParser;
    #[derive(Serialize)]
    struct QOut {
        subject: String,
        predicate: String,
        object: String,
    }

    let cursor = std::io::Cursor::new(payload.as_bytes());
    let mut parser = rio_turtle::TurtleParser::new(cursor, None);
    let mut triples = Vec::new();
    let mut on_triple = |t: rio_api::model::Triple| -> Result<(), std::io::Error> {
        triples.push(QOut {
            subject: t.subject.to_string(),
            predicate: t.predicate.to_string(),
            object: t.object.to_string(),
        });
        Ok(())
    };
    if parser.parse_all(&mut on_triple).is_err() {
        return JsValue::NULL; // Handle error appropriately
    }

    serde_wasm_bindgen::to_value(&triples).unwrap_or(JsValue::NULL)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn parse_n3logic_wasm(payload: &str) -> JsValue {
    #[derive(Serialize)]
    struct QOut {
        subject: String,
        predicate: String,
        object: String,
    }

    let cursor = std::io::Cursor::new(payload.as_bytes());
    let mut parser = crate::modalities::logic::n3_parser::N3Parser::new(cursor);
    let mut triples = Vec::new();

    let on_n3_event = |event: crate::modalities::logic::n3_parser::N3Event| -> Result<(), std::io::Error> {
        if let crate::modalities::logic::n3_parser::N3Event::StaticTriple(triple) = event {
            let s = match triple.subject {
                crate::modalities::logic::n3_parser::Term::Uri(s)
                | crate::modalities::logic::n3_parser::Term::Variable(s)
                | crate::modalities::logic::n3_parser::Term::Literal(s) => s,
            };
            let p = match triple.predicate {
                crate::modalities::logic::n3_parser::Term::Uri(s)
                | crate::modalities::logic::n3_parser::Term::Variable(s)
                | crate::modalities::logic::n3_parser::Term::Literal(s) => s,
            };
            let o = match triple.object {
                crate::modalities::logic::n3_parser::Term::Uri(s)
                | crate::modalities::logic::n3_parser::Term::Variable(s)
                | crate::modalities::logic::n3_parser::Term::Literal(s) => s,
            };
            triples.push(QOut {
                subject: s,
                predicate: p,
                object: o,
            });
        }
        Ok(())
    };

    if parser.parse_all(on_n3_event).is_err() {
        return JsValue::NULL;
    }

    serde_wasm_bindgen::to_value(&triples).unwrap_or(JsValue::NULL)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn parse_cbor_ld_wasm(payload: &[u8]) -> JsValue {
    match crate::cbor_compiler::parse_cbor_ld_to_quin(payload) {
        Ok(q) => {
            #[derive(Serialize)]
            struct QOut {
                subject: String,
                predicate: String,
                object: String,
                context: String,
            }
            let out = QOut {
                subject: q.subject.to_string(),
                predicate: q.predicate.to_string(),
                object: q.object.to_string(),
                context: q.context.to_string(),
            };
            serde_wasm_bindgen::to_value(&out).unwrap_or(JsValue::NULL)
        }
        Err(_) => JsValue::NULL,
    }
}

// ─── Forward Chaining ────────────────────────────────────────────────────────

/// Forward-chaining defeasible inference engine.
/// Input: `{ facts: ["bird", "penguin"], rules: [{ head: "flies", body: ["bird"], defeaters: ["penguin"] }, ...] }`
/// Output: `{ inferred: ["swims"] }`
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn forward_chain_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    use crate::solvers::symbolic_logic::{
        ForwardChainingDefeasible, DefeasibleRule, Fact, Literal, RuleType,
    };
    use crate::solvers::SolverConfig;
    use std::collections::HashMap;

    #[derive(Deserialize)]
    struct RuleInput { head: String, body: Vec<String>, defeaters: Vec<String> }
    #[derive(Deserialize)]
    struct FcInput { facts: Vec<String>, rules: Vec<RuleInput> }
    let input: FcInput = serde_wasm_bindgen::from_value(val)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    // Build atom → u8 index map
    // Variable index 0 is reserved as antecedent terminator in ForwardChainingDefeasible.
    let mut atom_map: HashMap<String, u8> = HashMap::new();
    let mut next_idx: u8 = 1;
    let mut get_idx = |s: &str, map: &mut HashMap<String, u8>, nxt: &mut u8| -> u8 {
        if let Some(&i) = map.get(s) { return i; }
        let i = *nxt;
        map.insert(s.to_string(), i);
        *nxt = nxt.saturating_add(1);
        i
    };
    for f in &input.facts { get_idx(f, &mut atom_map, &mut next_idx); }
    for r in &input.rules {
        get_idx(&r.head, &mut atom_map, &mut next_idx);
        for b in &r.body     { get_idx(b, &mut atom_map, &mut next_idx); }
        for d in &r.defeaters{ get_idx(d, &mut atom_map, &mut next_idx); }
    }

    let mut solver = ForwardChainingDefeasible::new(SolverConfig::default());

    // Add initial facts
    for (fact_id, atom) in input.facts.iter().enumerate() {
        let var = *atom_map.get(atom.as_str()).unwrap_or(&0);
        solver.add_fact(Fact {
            id: (fact_id as u32) + 1,
            literal: Literal { variable: var, negated: false },
            supporting_rules: [0; 3],
            defeated: false,
            confidence: 1.0,
        }).map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;
    }

    // Add rules and defeaters
    let base_id = input.facts.len() as u32 + 1;
    for (rule_id, r) in input.rules.iter().enumerate() {
        let head_var = *atom_map.get(r.head.as_str()).unwrap_or(&0);
        let mut antecedents = [Literal::default(); 5];
        for (i, b) in r.body.iter().take(5).enumerate() {
            antecedents[i] = Literal { variable: *atom_map.get(b.as_str()).unwrap_or(&0), negated: false };
        }

        // Main defeasible rule: head fires when all body atoms hold
        let main_rule = DefeasibleRule {
            id: base_id + (rule_id as u32) * 2,
            rule_type: if r.defeaters.is_empty() { RuleType::Strict } else { RuleType::Defeasible },
            antecedents,
            consequent: Literal { variable: head_var, negated: false },
            priority: 500,
            active: true,
            fire_count: 0,
        };
        solver.add_rule(main_rule).map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;

        // Defeater rules: for each defeater atom, add a Defeater rule that cancels the head
        for (d_i, d) in r.defeaters.iter().enumerate() {
            let d_var = *atom_map.get(d.as_str()).unwrap_or(&0);
            let mut d_antecedents = [Literal::default(); 5];
            d_antecedents[0] = Literal { variable: d_var, negated: false };
            let defeater_rule = DefeasibleRule {
                id: base_id + (rule_id as u32) * 2 + 1 + d_i as u32,
                rule_type: RuleType::Defeater,
                antecedents: d_antecedents,
                consequent: Literal { variable: head_var, negated: true },
                priority: 600,
                active: true,
                fire_count: 0,
            };
            solver.add_rule(defeater_rule).map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;
        }
    }

    solver.infer().map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;

    // Build reverse map to recover atom names from variable indices
    let rev_map: HashMap<u8, &str> = atom_map.iter().map(|(k, &v)| (v, k.as_str())).collect();
    let initial_fact_set: std::collections::HashSet<String> =
        input.facts.iter().cloned().collect();

    let mut inferred = Vec::new();
    for fact in &solver.facts {
        if fact.id == 0 || fact.defeated { continue; }
        if let Some(&name) = rev_map.get(&fact.literal.variable) {
            if !fact.literal.negated && !initial_fact_set.contains(name) {
                inferred.push(name.to_string());
            }
        }
    }

    #[derive(Serialize)]
    struct FcOut { inferred: Vec<String> }
    Ok(serde_wasm_bindgen::to_value(&FcOut { inferred })?)
}

// ─── Engine metadata ─────────────────────────────────────────────────────────

/// Capabilities compiled into the browser WASM build (native-only modules omitted).
#[cfg(target_arch = "wasm32")]
pub(crate) const WASM_CAPABILITY_REGISTRY: &[&str] = &[
    "SHACL",
    "QueryEngine",
    "N3Parser",
    "N3Compiler",
    "DeonticLogic",
    "EpistemicLogic",
    "ParaconsistentLogic",
    "DialecticalLogic",
    "TemporalLTL",
    "Bioinformatics",
    "OrganicChemistry",
    "Economics",
    "CogAI",
    "Profiles",
    "ResourceCatalog",
    "WasmIngest",
    "ControlTheory",
    "LwwCrdt",
    "GbmPath",
    "BlackScholes",
    "SatSolver",
    "ForwardChaining",
    "OdeDecay",
    "SciencePlayground",
    "LlmInference",
];

// --- Data Format: RDF Serializer ------------------------------------------------

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct RdfSerializeParams {
    pub quins: Vec<[u64; 6]>,
    pub format: String, // "nt", "turtle", "nquads", "trig", "n3", "jsonld"
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn serialize_rdf_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    use crate::sparql_library::serialisers::rdf_serializers::{serialize_to_ntriples, serialize_to_turtle, serialize_to_nquads, serialize_to_trig, serialize_to_n3, serialize_to_jsonld};
    use crate::NQuin;
    
    let p: RdfSerializeParams = serde_wasm_bindgen::from_value(val)?;
    
    let quins: Vec<NQuin> = p.quins.iter().map(|arr| NQuin {
        subject: arr[0],
        predicate: arr[1],
        object: arr[2],
        context: arr[3],
        metadata: arr[4],
        parity: arr[5],
    }).collect();
    
    let mut rdf_output = Vec::new();
    
    match p.format.as_str() {
        "nt" => serialize_to_ntriples(&mut rdf_output, &quins),
        "turtle" => serialize_to_turtle(&mut rdf_output, &quins),
        "nquads" => serialize_to_nquads(&mut rdf_output, &quins),
        "trig" => serialize_to_trig(&mut rdf_output, &quins),
        "n3" => serialize_to_n3(&mut rdf_output, &quins),
        "jsonld" => serialize_to_jsonld(&mut rdf_output, &quins),
        _ => return Err(JsValue::from_str("Invalid RDF format")),
    }.map_err(|e| JsValue::from_str(&e))?;
    
    #[derive(Serialize)]
    struct SerializeResult {
        rdf_data: String,
    }
    
    Ok(serde_wasm_bindgen::to_value(&SerializeResult {
        rdf_data: String::from_utf8(rdf_output).map_err(|e| JsValue::from_str(&e.to_string()))?,
    })?)
}





