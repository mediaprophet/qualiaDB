// crates/qualia-core-db/src/mcp_server.rs

// We still need access to standard library for I/O and String during init phase
extern crate std;

#[path = "mcp_tool_impls.rs"]
mod mcp_tool_impls;
#[path = "mcp_stub_impls.rs"]
mod mcp_stub_impls;
#[path = "mcp_format_impls.rs"]
mod mcp_format_impls;

use crate::wal::append_mutation;
use crate::NQuin;
use core::ptr::write_volatile;
use serde_json::{json, Value};
use std::string::String;

/// Explicit operational states defining the execution boundaries
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum McpRuntimeState {
    HandshakePhase,
    AllocationFirewallActive,
    SanctuaryGated,
}

#[derive(Debug)]
pub enum McpSystemError {
    SanctuaryGateTriggered,
    ToolNotFound,
    ToolNotReady,
    ParseError,
    IntentFrameViolation,
    FeatureNotEnabled,
    InvalidParameters,
}

#[derive(Debug, Clone)]
pub struct McpIntentFrame {
    pub purpose_hash: u64,
    pub active_deontic_constraints: Vec<u64>,
    pub active_profile_id: Option<u64>,
    pub session_nonce: u64,
    /// A genuine 32-byte cryptographic egress-override token, or `None` when the
    /// caller supplied no override — or supplied a malformed / placeholder /
    /// all-zero value, all of which fail closed to `None`. See
    /// [`parse_sanctuary_override`] for the validation contract.
    pub sanctuary_override: Option<[u8; 32]>,
    pub qpu_enabled: bool,
    pub llm_enabled: bool,
}

/// Zero-deserialization view over an incoming tools/call byte buffer
pub struct RawToolPayload<'a> {
    pub tool_name: &'a [u8],
    pub arguments_raw: &'a [u8],
}

#[derive(Clone, Copy)]
struct McpToolDescriptor {
    name: &'static str,
    description: &'static str,
    input_schema: &'static str,
}

// Simple raw-byte slice extractor. This is a very rudimentary byte matcher
// intended to satisfy the requirement of bypassing generic serde allocation.
fn extract_raw_json_string<'a>(payload: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    // Look for `"key":"value"`
    let mut i = 0;
    while i < payload.len() {
        if payload[i..].starts_with(key) {
            i += key.len();
            // find colon
            while i < payload.len() && (payload[i] == b' ' || payload[i] == b':') {
                i += 1;
            }
            if i < payload.len() && payload[i] == b'"' {
                i += 1;
                let start = i;
                while i < payload.len() && payload[i] != b'"' {
                    i += 1;
                }
                return Some(&payload[start..i]);
            }
        }
        i += 1;
    }
    None
}

fn stable_mcp_tools() -> &'static [McpToolDescriptor] {
    &[
        McpToolDescriptor {
            name: "query_graph",
            description: "Run guarded graph traversal against the in-memory daemon graph.",
            input_schema: r#"{"type":"object","properties":{"sanctuary_override":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "query_sparql",
            description: "Execute an N-Triples pattern query against the in-process daemon graph.",
            input_schema: r#"{"type":"object","required":["query"],"properties":{"query":{"type":"string"},"limit":{"type":"integer"}}}"#,
        },
        McpToolDescriptor {
            name: "get_graph_stats",
            description: "Return quin count and capacity for the resident daemon graph.",
            input_schema: r#"{"type":"object","properties":{}}"#,
        },
        McpToolDescriptor {
            name: "list_ontologies",
            description: "List startup ontology catalog entries and on-disk presence.",
            input_schema: r#"{"type":"object","properties":{}}"#,
        },
        McpToolDescriptor {
            name: "llm_infer",
            description: "Run local GGUF inference with caller prompt and optional model_path.",
            input_schema: r#"{"type":"object","required":["prompt"],"properties":{"prompt":{"type":"string"},"model_path":{"type":"string"},"graph_context":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "llm_chat",
            description: "Multi-turn chat completion via the local GGUF inference stack.",
            input_schema: r#"{"type":"object","required":["messages"],"properties":{"messages":{"type":"array"},"model_path":{"type":"string"},"graph_context":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "list_models",
            description: "Discover GGUF models under storage and any resident mounted model.",
            input_schema: r#"{"type":"object","properties":{}}"#,
        },
        McpToolDescriptor {
            name: "qpu_optimize",
            description: "Formulate a QUBO/circuit job from a problem description and classical solve.",
            input_schema: r#"{"type":"object","required":["problem"],"properties":{"problem":{"type":"object"}}}"#,
        },
        McpToolDescriptor {
            name: "qpu_dft",
            description: "Bounded Thomas-Fermi DFT ground-state energy from quins or grid resolution.",
            input_schema: r#"{"type":"object","properties":{"grid_resolution":{"type":"integer"},"quins":{"type":"array"}}}"#,
        },
        McpToolDescriptor {
            name: "qpu_status",
            description: "Return QPU bridge connection and job-queue status.",
            input_schema: r#"{"type":"object","properties":{}}"#,
        },
        McpToolDescriptor {
            name: "get_wallet_status",
            description: "Inspect queued ILP micropayments from pending_payments.ndjson.",
            input_schema: r#"{"type":"object","properties":{}}"#,
        },
        McpToolDescriptor {
            name: "get_did_info",
            description: "Parse a did:q42 identifier into its topological pointer hash.",
            input_schema: r#"{"type":"object","required":["did"],"properties":{"did":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "ingest_ontology",
            description: "Parse TTL/N3/q42 ontology file and extend the daemon graph.",
            input_schema: r#"{"type":"object","required":["path"],"properties":{"path":{"type":"string"},"context_hash":{"type":"integer"}}}"#,
        },
        McpToolDescriptor {
            name: "validate_shacl",
            description: "Validate quins for a target subject/property against SHACL constraints.",
            input_schema: r#"{"type":"object","required":["quins","target_subject","target_property","constraints"],"properties":{"quins":{"type":"array"},"target_subject":{"type":"integer"},"target_property":{"type":"integer"},"constraints":{"type":"array"}}}"#,
        },
        McpToolDescriptor {
            name: "list_qapps",
            description: "List installed qapps from storage Qapps directory.",
            input_schema: r#"{"type":"object","properties":{}}"#,
        },
        McpToolDescriptor {
            name: "get_qapp_manifest",
            description: "Load qapp.json manifest for an installed or bundled qapp.",
            input_schema: r#"{"type":"object","required":["qapp_name"],"properties":{"qapp_name":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "inspect_qapp_readiness",
            description: "Check manifest, entrypoints, and ontology readiness for a qapp.",
            input_schema: r#"{"type":"object","required":["qapp_name"],"properties":{"qapp_name":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "list_qapp_updates",
            description: "Compare installed qapp versions against bundled update offers.",
            input_schema: r#"{"type":"object","properties":{}}"#,
        },
        McpToolDescriptor {
            name: "get_system_status",
            description: "Return runtime status for the Qualia MCP surface.",
            input_schema: r#"{"type":"object","properties":{}}"#,
        },
        McpToolDescriptor {
            name: "describe_qapp_surface_schema",
            description: "Describe the current Qapp host surface schema exposed by Qualia.",
            input_schema: r#"{"type":"object","properties":{}}"#,
        },
        McpToolDescriptor {
            name: "inject_test_quin",
            description: "Inject a deterministic test Quin through the paraconsistent router.",
            input_schema: r#"{"type":"object","properties":{}}"#,
        },
        McpToolDescriptor {
            name: "evaluate_modality",
            description: "Evaluate logic modalities: ltl, asp, deontic, epistemic, dl, paraconsistent, probabilistic.",
            input_schema: r#"{"type":"object","required":["modality"],"properties":{"modality":{"type":"string"},"quins":{"type":"array"},"trace":{"type":"array"},"formula":{"type":"object"},"now_unix":{"type":"integer"},"agent_did_hash":{"type":"integer"},"world_hash":{"type":"integer"}}}"#,
        },
        McpToolDescriptor {
            name: "matrix_operation",
            description: "Linear algebra: multiply, transpose, solve, or inverse with caller-supplied matrices.",
            input_schema: r#"{"type":"object","required":["op"],"properties":{"op":{"type":"string","enum":["multiply","transpose","solve","inverse"]},"left":{"type":"object","properties":{"id":{"type":"string"},"rows":{"type":"integer"},"cols":{"type":"integer"},"data":{"type":"array","items":{"type":"number"}}}},"right":{"type":"object"},"matrices":{"type":"array"},"result_id":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "algebra_solve_polynomial",
            description: "Find all (real + complex) roots of a polynomial given descending coefficients.",
            input_schema: r#"{"type":"object","required":["coeffs"],"properties":{"coeffs":{"type":"array","items":{"type":"number"},"description":"descending coefficients [c_n, ..., c_1, c_0]"}}}"#,
        },
        McpToolDescriptor {
            name: "algebra_matrix_analyze",
            description: "Determinant, eigenvalues (general), symmetric eigensystem, or SVD of a row-major matrix.",
            input_schema: r#"{"type":"object","required":["op","rows","data"],"properties":{"op":{"type":"string","enum":["determinant","eigenvalues","eigen_symmetric","svd"]},"rows":{"type":"integer"},"cols":{"type":"integer"},"data":{"type":"array","items":{"type":"number"}}}}"#,
        },
        McpToolDescriptor {
            name: "cas",
            description: "Symbolic algebra: differentiate/simplify/expand/evaluate a text expression (e.g. 'x^3 - 2*x^2'), or solve/factor a quadratic symbolically.",
            input_schema: r#"{"type":"object","required":["op"],"properties":{"op":{"type":"string","enum":["differentiate","simplify","expand","evaluate","solve_quadratic","factor"]},"expr":{"type":"string"},"var":{"type":"string"},"env":{"type":"object"},"a":{"type":"number"},"b":{"type":"number"},"c":{"type":"number"}}}"#,
        },
        McpToolDescriptor {
            name: "ode_solve",
            description: "Run a configurable CFD or molecular-dynamics simulation step.",
            input_schema: r#"{"type":"object","properties":{"type":{"type":"string","enum":["cfd","distributed","molecular_dynamics"]},"nx":{"type":"integer"},"ny":{"type":"integer"},"dx":{"type":"number"},"time_step":{"type":"number"},"total_time":{"type":"number"},"num_threads":{"type":"integer"}}}"#,
        },
        McpToolDescriptor {
            name: "chemical_analysis",
            description: "Predict molecular properties from SMILES or formula via ChemistryModelingLibrary.",
            input_schema: r#"{"type":"object","properties":{"smiles":{"type":"string"},"formula":{"type":"string"},"molecular_weight":{"type":"number"},"prop":{"type":"string"},"properties":{"type":"array","items":{"type":"string"}}}}"#,
        },
        McpToolDescriptor {
            name: "statistical_analysis",
            description: "Descriptive statistics on caller-supplied tabular data.",
            input_schema: r#"{"type":"object","required":["rows"],"properties":{"stat":{"type":"string","enum":["mean","variance","correlation"]},"rows":{"type":"array"},"columns":{"type":"array","items":{"type":"string"}},"column":{"type":"string"},"column_y":{"type":"string"},"method":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "ml_inference",
            description: "Load a model by id and run inference on caller-supplied input bytes.",
            input_schema: r#"{"type":"object","properties":{"model_id":{"type":"string"},"model_path":{"type":"string"},"input_data":{"type":"array","items":{"type":"integer"}},"input_hex":{"type":"string"},"batch_size":{"type":"integer"},"temperature":{"type":"number"},"max_tokens":{"type":"integer"}}}"#,
        },
        McpToolDescriptor {
            name: "financial_model",
            description: "Black-Scholes option pricing or portfolio risk with caller parameters.",
            input_schema: r#"{"type":"object","properties":{"op":{"type":"string","enum":["option","risk"]},"underlying_price":{"type":"number"},"strike":{"type":"number"},"volatility":{"type":"number"},"assets":{"type":"array"},"cash_balance":{"type":"number"}}}"#,
        },
        McpToolDescriptor {
            name: "medical_score",
            description: "Clinical analysis for a caller-supplied patient record.",
            input_schema: r#"{"type":"object","properties":{"patient_id":{"type":"string"},"score":{"type":"string","enum":["diagnosis","treatment","prognosis","prevention"]},"patient":{"type":"object"}}}"#,
        },
        McpToolDescriptor {
            name: "engineering_analysis_op",
            description: "Structural, thermal, or dynamic FEA with caller model geometry and loads.",
            input_schema: r#"{"type":"object","properties":{"analysis":{"type":"string","enum":["structural","thermal","dynamic"]},"model":{"type":"object"},"dimensions":{"type":"array"},"youngs_modulus":{"type":"number"}}}"#,
        },
        McpToolDescriptor {
            name: "bioinformatics_align",
            description: "Pairwise nucleotide or protein alignment on caller query/target sequences.",
            input_schema: r#"{"type":"object","required":["query","target"],"properties":{"query":{"type":"string"},"target":{"type":"string"},"mode":{"type":"string","enum":["dna","protein"]}}}"#,
        },
        McpToolDescriptor {
            name: "chemical_descriptors",
            description: "Compute Lipinski/Veber descriptors from a SMILES string.",
            input_schema: r#"{"type":"object","required":["smiles"],"properties":{"smiles":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "clinical_risk",
            description: "Clinical risk scores: framingham, cha2ds2_vasc, sofa, egfr.",
            input_schema: r#"{"type":"object","properties":{"score":{"type":"string","enum":["framingham","cha2ds2","cha2ds2_vasc","sofa","egfr","renal"]},"age":{"type":"integer"},"input":{"type":"object"}}}"#,
        },
        McpToolDescriptor {
            name: "parse_csv",
            description: "Stream CSV into Quins via zero-heap parser. Requires field_mappings; accepts csv_data or file_path.",
            input_schema: r#"{"type":"object","required":["field_mappings"],"properties":{"csv_data":{"type":"string"},"file_path":{"type":"string"},"field_mappings":{"type":"array","items":{"type":"object","required":["source_key"],"properties":{"source_key":{"type":"string"},"predicate":{"type":"string"},"predicate_hash":{"type":"integer"},"datatype":{"type":"string","enum":["integer","float","datetime","string"]}}}},"base_class_hash":{"type":"integer"},"context_hash":{"type":"integer"},"ingest_to_graph":{"type":"boolean"}}}"#,
        },
        McpToolDescriptor {
            name: "parse_rdf",
            description: "Parse RDF/RDF-Star (nt, turtle, nquads, trig, n3, jsonld, cbor) into quins via zero-heap streaming parsers.",
            input_schema: r#"{"type":"object","required":["format"],"properties":{"format":{"type":"string"},"rdf_data":{"type":"string"},"file_path":{"type":"string"},"context_hash":{"type":"integer"},"ingest_to_graph":{"type":"boolean"}}}"#,
        },
        McpToolDescriptor {
            name: "parse_json",
            description: "Stream JSON objects into Quins via zero-heap parser. Requires field_mappings; accepts json_data or file_path.",
            input_schema: r#"{"type":"object","required":["field_mappings"],"properties":{"json_data":{"type":"string"},"file_path":{"type":"string"},"field_mappings":{"type":"array","items":{"type":"object","required":["source_key"],"properties":{"source_key":{"type":"string"},"predicate":{"type":"string"},"predicate_hash":{"type":"integer"},"datatype":{"type":"string","enum":["integer","float","datetime","string"]}}}},"base_class_hash":{"type":"integer"},"context_hash":{"type":"integer"},"ingest_to_graph":{"type":"boolean"}}}"#,
        },
        McpToolDescriptor {
            name: "serialize_csv",
            description: "Serialize quins or daemon graph slice to CSV. Returns inline csv_data or writes file_path when output=file.",
            input_schema: r#"{"type":"object","required":["headers","predicate_hashes"],"properties":{"quins":{"type":"array"},"use_graph":{"type":"boolean"},"context_hash":{"type":"integer"},"headers":{"type":"array","items":{"type":"string"}},"predicate_hashes":{"type":"array","items":{"type":"integer"}},"datatypes":{"type":"array","items":{"type":"string"}},"output":{"type":"string","enum":["inline","file"]},"file_path":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "serialize_json",
            description: "Serialize quins or daemon graph slice to JSON array. Returns inline json_data or writes file_path when output=file.",
            input_schema: r#"{"type":"object","required":["field_names","predicate_hashes"],"properties":{"quins":{"type":"array"},"use_graph":{"type":"boolean"},"context_hash":{"type":"integer"},"field_names":{"type":"array","items":{"type":"string"}},"predicate_hashes":{"type":"array","items":{"type":"integer"}},"datatypes":{"type":"array","items":{"type":"string"}},"output":{"type":"string","enum":["inline","file"]},"file_path":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "serialize_rdf",
            description: "Serialize quins or graph slice to RDF/RDF-Star via resolver-backed zero-heap dispatch.",
            input_schema: r#"{"type":"object","properties":{"quins":{"type":"array"},"use_graph":{"type":"boolean"},"context_hash":{"type":"integer"},"format":{"type":"string"},"rdf_star":{"type":"boolean"},"star":{"type":"boolean"},"output":{"type":"string","enum":["inline","file"]},"file_path":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "symbolic_logic_infer",
            description: "Defeasible forward-chaining or bounded SAT with caller facts, rules, and clauses.",
            input_schema: r#"{"type":"object","properties":{"solver":{"type":"string","enum":["defeasible","sat"]},"facts":{"type":"array"},"rules":{"type":"array"},"clauses":{"type":"array"},"max_iterations":{"type":"integer"}}}"#,
        },
        McpToolDescriptor {
            name: "geometric_algebra_op",
            description: "3D vector cross product, dot product, or angle between caller vectors.",
            input_schema: r#"{"type":"object","required":["a","b"],"properties":{"op":{"type":"string","enum":["cross","angle","dot"]},"a":{"type":"array","items":{"type":"number"},"minItems":3,"maxItems":3},"b":{"type":"array","items":{"type":"number"},"minItems":3,"maxItems":3}}}"#,
        },
        McpToolDescriptor {
            name: "run_docs_tests",
            description: "Run docs/tests headless suites (logic, wasm, native, or both). Native/both require daemon on localhost:4242.",
            input_schema: r#"{"type":"object","properties":{"mode":{"type":"string","enum":["logic","wasm","native","both"]}}}"#,
        },
        McpToolDescriptor {
            name: "get_pending_tasks",
            description: "Get tasks pending in the ambient orchestrator.",
            input_schema: r#"{"type":"object","properties":{}}"#,
        },
        McpToolDescriptor {
            name: "values_check",
            description: "Values abuse-check (human-rights guard): does an agent claiming a natural-person-only dignity right trip the inverse rights-guard? Runs the real agency.n3 G1/G1' lane. agentType is a webcivics values class (CorporatePerson, ArtificialAgent, NaturalPerson, ...).",
            input_schema: r#"{"type":"object","required":["agentType"],"properties":{"agentType":{"type":"string"},"claimsDignityRight":{"type":"boolean"}}}"#,
        },
        McpToolDescriptor {
            name: "values_evaluate",
            description: "Deontic-contract reasoner in values terms: is a norm (forbid/oblige/permit) bound to a party+action currently in force? Runs the native deontic VM and returns Active / Defeated (by an 'unless' exception) / Expired (past its window) / Malformed.",
            input_schema: r#"{"type":"object","required":["modality","party","action"],"properties":{"modality":{"type":"string","enum":["forbid","oblige","permit"]},"party":{"type":"string"},"action":{"type":"string"},"object":{"type":"string"},"now":{"type":"integer"},"expiry":{"type":"integer"},"unless":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "graph_resolve",
            description: "Resolve an identifier (IRI) against the live daemon graph: returns its modal identifier-KIND (open kind fabric — WebizenId / DidQ42 / ContentHash / ... or none for a plain dictionary reference) and its out-degree. Composes the hybrid-modality resolver (zero-alloc QuinIndex / slice scan + modal_kind) over one identity space.",
            input_schema: r#"{"type":"object","required":["iri"],"properties":{"iri":{"type":"string"}}}"#,
        },
    ]
}

/// Dispatches incoming tool actions without triggering dynamic heap allocations
pub unsafe fn enforce_fiduciary_tool_dispatch(
    payload: RawToolPayload,
    intent_frame: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    match payload.tool_name {
        // ── Graph Engine Tools ───────────────────────────────────────────────
        b"query_graph" => {
            // `is_none()` here means "no *valid* token": `build_intent_frame` already
            // rejected missing, malformed, placeholder ("MISSING"), and all-zero
            // values via `parse_sanctuary_override`. Presence alone never opens egress.
            if intent_frame.sanctuary_override.is_none() {
                let violation_quin = NQuin::new_conduct_violation(
                    b"EgressViolation: Invalid or Missing Cryptographic Sanctuary Override",
                );
                let _ = append_mutation(&violation_quin);
                return Err(McpSystemError::SanctuaryGateTriggered);
            }
            execute_bare_metal_graph_traversal(payload.arguments_raw, intent_frame)
        }

        b"query_sparql" => execute_sparql_query(payload.arguments_raw, intent_frame),

        b"get_graph_stats" => execute_graph_stats(payload.arguments_raw, intent_frame),

        b"list_ontologies" => execute_list_ontologies(payload.arguments_raw, intent_frame),

        // ── LLM Tools ─────────────────────────────────────────────────────────
        b"llm_infer" => {
            if !intent_frame.llm_enabled {
                return Err(McpSystemError::FeatureNotEnabled);
            }
            execute_llm_infer(payload.arguments_raw, intent_frame)
        }

        b"llm_chat" => {
            if !intent_frame.llm_enabled {
                return Err(McpSystemError::FeatureNotEnabled);
            }
            execute_llm_chat(payload.arguments_raw, intent_frame)
        }

        b"list_models" => execute_list_models(payload.arguments_raw, intent_frame),

        // ── QPU Tools ─────────────────────────────────────────────────────────
        b"qpu_optimize" => {
            if !intent_frame.qpu_enabled {
                return Err(McpSystemError::FeatureNotEnabled);
            }
            execute_qpu_optimize(payload.arguments_raw, intent_frame)
        }

        b"qpu_dft" => {
            if !intent_frame.qpu_enabled {
                return Err(McpSystemError::FeatureNotEnabled);
            }
            execute_qpu_dft(payload.arguments_raw, intent_frame)
        }

        b"qpu_status" => execute_qpu_status(payload.arguments_raw, intent_frame),

        // ── Scientific Computing Tools ───────────────────────────────────────
        b"matrix_operation" => execute_matrix_operation(payload.arguments_raw, intent_frame),
        b"algebra_solve_polynomial" => mcp_tool_impls::algebra_solve_polynomial(payload.arguments_raw),
        b"algebra_matrix_analyze" => mcp_tool_impls::algebra_matrix_analyze(payload.arguments_raw),
        b"cas" => mcp_tool_impls::cas(payload.arguments_raw),

        b"ode_solve" => execute_ode_solve(payload.arguments_raw, intent_frame),

        b"chemical_analysis" => execute_chemical_analysis(payload.arguments_raw, intent_frame),

        b"statistical_analysis" => {
            execute_statistical_analysis(payload.arguments_raw, intent_frame)
        }

        b"ml_inference" => execute_ml_inference(payload.arguments_raw, intent_frame),

        b"financial_model" => execute_financial_model(payload.arguments_raw, intent_frame),

        b"medical_score" => execute_medical_score(payload.arguments_raw, intent_frame),

        b"engineering_analysis_op" => {
            execute_engineering_analysis(payload.arguments_raw, intent_frame)
        }

        // ── Identifiers & Wallet Tools ─────────────────────────────────────────
        b"get_wallet_status" => execute_wallet_status(payload.arguments_raw, intent_frame),

        b"get_did_info" => execute_did_info(payload.arguments_raw, intent_frame),

        // ── Ontology Tools ────────────────────────────────────────────────────
        b"ingest_ontology" => execute_ingest_ontology(payload.arguments_raw, intent_frame),

        b"validate_shacl" => execute_shacl_validation(payload.arguments_raw, intent_frame),

        // ── Testing & Debugging Tools ───────────────────────────────────────
        b"inject_test_quin" => {
            execute_paraconsistent_injection(payload.arguments_raw, intent_frame)
        }

        b"list_qapps" => execute_list_qapps(payload.arguments_raw, intent_frame),

        b"get_qapp_manifest" => execute_get_qapp_manifest(payload.arguments_raw, intent_frame),

        b"inspect_qapp_readiness" => {
            execute_inspect_qapp_readiness(payload.arguments_raw, intent_frame)
        }

        b"list_qapp_updates" => execute_list_qapp_updates(payload.arguments_raw, intent_frame),

        b"describe_qapp_surface_schema" => {
            execute_describe_qapp_surface_schema(payload.arguments_raw, intent_frame)
        }

        b"get_system_status" => execute_system_status(payload.arguments_raw, intent_frame),

        // ── Extended Logic & Science Tools ───────────────────────────────────
        b"evaluate_modality" => execute_evaluate_modality(payload.arguments_raw, intent_frame),

        b"bioinformatics_align" => {
            execute_bioinformatics_align(payload.arguments_raw, intent_frame)
        }

        b"chemical_descriptors" => {
            execute_chemical_descriptors(payload.arguments_raw, intent_frame)
        }

        b"clinical_risk" => execute_clinical_risk(payload.arguments_raw, intent_frame),

        b"symbolic_logic_infer" => {
            execute_symbolic_logic_infer(payload.arguments_raw, intent_frame)
        }

        b"geometric_algebra_op" => {
            execute_geometric_algebra_op(payload.arguments_raw, intent_frame)
        }

        // ── Data Format Tools ──────────────────────────────────────────────────────
        b"parse_csv" => execute_parse_csv(payload.arguments_raw, intent_frame),
        b"parse_rdf" => execute_parse_rdf(payload.arguments_raw, intent_frame),
        b"parse_json" => execute_parse_json(payload.arguments_raw, intent_frame),
        b"serialize_csv" => execute_serialize_csv(payload.arguments_raw, intent_frame),
        b"serialize_json" => execute_serialize_json(payload.arguments_raw, intent_frame),
        b"serialize_rdf" => execute_serialize_rdf(payload.arguments_raw, intent_frame),

        b"run_docs_tests" => execute_run_docs_tests(payload.arguments_raw, intent_frame),
        b"get_pending_tasks" => execute_get_pending_tasks(payload.arguments_raw, intent_frame),

        // ── Values / Human-Rights Governance ────────────────────────────────────
        b"values_check" => mcp_tool_impls::values_check(payload.arguments_raw),
        b"values_evaluate" => mcp_tool_impls::values_evaluate(payload.arguments_raw),
        b"graph_resolve" => mcp_tool_impls::graph_resolve(payload.arguments_raw),

        _ => Err(McpSystemError::ToolNotFound),
    }
}

unsafe fn execute_get_pending_tasks(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    mcp_stub_impls::get_pending_tasks(args)
}

// ── Graph Engine Implementations ───────────────────────────────────────────

unsafe fn execute_sparql_query(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    mcp_stub_impls::query_sparql(args)
}

unsafe fn execute_bare_metal_graph_traversal(
    _args: &[u8],
    intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    let mut arena = crate::webizen::SlgArena::new();
    let contract = if intent
        .active_deontic_constraints
        .first()
        .copied()
        .unwrap_or(0)
        != 0
    {
        intent.active_deontic_constraints[0]
    } else {
        intent.purpose_hash
    };
    let fired = arena.fire_registered_rules(contract);
    Ok(fired.max(1).to_string())
}

unsafe fn execute_graph_stats(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    mcp_stub_impls::get_graph_stats(args)
}

unsafe fn execute_list_ontologies(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    mcp_stub_impls::list_ontologies(args)
}

// ── LLM Implementations ─────────────────────────────────────────────────────

unsafe fn execute_llm_infer(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    mcp_stub_impls::llm_infer(args)
}

unsafe fn execute_llm_chat(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    mcp_stub_impls::llm_chat(args)
}

unsafe fn execute_list_models(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    mcp_stub_impls::list_models(args)
}

// ── QPU Implementations ─────────────────────────────────────────────────────

unsafe fn execute_qpu_optimize(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    mcp_stub_impls::qpu_optimize(args)
}

unsafe fn execute_qpu_dft(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    mcp_stub_impls::qpu_dft(args)
}

unsafe fn execute_qpu_status(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    mcp_stub_impls::qpu_status(args)
}

// ── Scientific Computing Implementations ─────────────────────────────────

unsafe fn execute_matrix_operation(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    mcp_tool_impls::matrix_operation(args)
}

unsafe fn execute_ode_solve(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    mcp_tool_impls::ode_solve(args)
}

unsafe fn execute_chemical_analysis(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    mcp_tool_impls::chemical_analysis(args)
}

unsafe fn execute_statistical_analysis(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    mcp_tool_impls::statistical_analysis(args)
}

unsafe fn execute_ml_inference(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    mcp_tool_impls::ml_inference(args)
}

unsafe fn execute_financial_model(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    mcp_tool_impls::financial_model(args)
}

unsafe fn execute_medical_score(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    mcp_tool_impls::medical_score(args)
}

unsafe fn execute_engineering_analysis(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    mcp_tool_impls::engineering_analysis(args)
}

// ── Identity & Wallet Implementations ─────────────────────────────────────

unsafe fn execute_wallet_status(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    mcp_stub_impls::get_wallet_status(args)
}

unsafe fn execute_did_info(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    mcp_stub_impls::get_did_info(args)
}

// ── Ontology Implementations ────────────────────────────────────────────────

unsafe fn execute_ingest_ontology(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    mcp_stub_impls::ingest_ontology(args)
}

unsafe fn execute_shacl_validation(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    mcp_stub_impls::validate_shacl(args)
}

// ── Testing & Debugging Implementations ─────────────────────────────────────

unsafe fn execute_paraconsistent_injection(
    _args: &[u8],
    intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    let candidate = NQuin {
        subject: intent.purpose_hash,
        predicate: crate::q_hash("q42:testClaim"),
        object: intent.session_nonce,
        context: intent.purpose_hash,
        metadata: 0,
        parity: 0,
    };
    let mut q = candidate;
    q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;

    let mut consistent = [NQuin::default(); 8];
    let mut isolated = [NQuin::default(); 8];
    let (c, i) = crate::modalities::paraconsistent::route_paraconsistent(
        &[q],
        &mut consistent,
        &mut isolated,
    )
    .map_err(|_| McpSystemError::ParseError)?;

    for idx in 0..i {
        let _ = append_mutation(&isolated[idx]);
    }
    Ok((c + i).to_string())
}

unsafe fn execute_list_qapps(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    mcp_stub_impls::list_qapps(args)
}

unsafe fn execute_get_qapp_manifest(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    let qapp_name = extract_raw_json_string(args, b"\"qapp_name\"").unwrap_or(b"");
    if qapp_name.is_empty() {
        return Err(McpSystemError::InvalidParameters);
    }
    let name = std::str::from_utf8(qapp_name).map_err(|_| McpSystemError::InvalidParameters)?;
    mcp_stub_impls::get_qapp_manifest(name)
}

unsafe fn execute_inspect_qapp_readiness(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    let qapp_name = extract_raw_json_string(args, b"\"qapp_name\"").unwrap_or(b"");
    if qapp_name.is_empty() {
        return Err(McpSystemError::InvalidParameters);
    }
    let name = std::str::from_utf8(qapp_name).map_err(|_| McpSystemError::InvalidParameters)?;
    mcp_stub_impls::inspect_qapp_readiness(name)
}

unsafe fn execute_list_qapp_updates(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    mcp_stub_impls::list_qapp_updates(args)
}

unsafe fn execute_describe_qapp_surface_schema(
    _args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    let schema = r#"{
  "host_shell": "webizen-studio",
  "package_manifest": "qapp.json",
  "layout_strategies": ["PointGrid", "CssGrid", "FlexBox", "Masonry"],
  "presentation_modes": ["GridBound", "NodeRelational", "Spatial"],
  "coordinate_spaces": ["GlobalCartesian", "RelativeAnchored"],
  "layer_behaviors": ["Docked", "FloatingOverlay", "ModalOverlay", "FullCanvas"],
  "theme_scopes": ["environment", "app", "page", "module"],
  "manifest_surfaces": ["static-web", "wasm-local", "online-daemon-aware", "native-dioxus-pane"],
  "mcp_tools": ["list_qapps", "get_qapp_manifest", "inspect_qapp_readiness", "list_qapp_updates", "describe_qapp_surface_schema"]
}"#;
    Ok(schema.to_string())
}

unsafe fn execute_system_status(
    _args: &[u8],
    intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    Ok(json!({
        "server": "qualia-core-db-mcp",
        "version": env!("CARGO_PKG_VERSION"),
        "protocolVersion": "2025-03-26",
        "toolCount": stable_mcp_tools().len(),
        "qpuEnabled": intent.qpu_enabled,
        "llmEnabled": intent.llm_enabled,
        "activeProfileId": intent.active_profile_id,
    })
    .to_string())
}

unsafe fn execute_run_docs_tests(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    let mode = extract_raw_json_string(args, b"\"mode\"").unwrap_or(b"logic");
    let mode_str = core::str::from_utf8(mode).unwrap_or("logic");
    if !matches!(mode_str, "logic" | "wasm" | "native" | "both") {
        return Err(McpSystemError::InvalidParameters);
    }

    if matches!(mode_str, "native" | "both") && !daemon_health_ok(4242) {
        return Ok(json!({
            "ok": false,
            "mode": mode_str,
            "error": "daemon_unreachable",
            "hint": "Start the graph daemon with `qualia-cli service start` or `qualia-cli daemon start --dev`"
        })
        .to_string());
    }

    let root = resolve_repo_root();
    let script = root.join("docs/tests/run-headless.mjs");
    if !script.exists() {
        return Ok(json!({
            "ok": false,
            "mode": mode_str,
            "error": "runner_missing",
            "path": script.display().to_string()
        })
        .to_string());
    }

    let output = std::process::Command::new("node")
        .arg(script.as_os_str())
        .arg("--mode")
        .arg(mode_str)
        .current_dir(&root)
        .output()
        .map_err(|_| McpSystemError::ToolNotReady)?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    Ok(json!({
        "ok": output.status.success(),
        "mode": mode_str,
        "exitCode": output.status.code(),
        "stdout": stdout,
        "stderr": stderr
    })
    .to_string())
}

fn daemon_health_ok(port: u16) -> bool {
    use std::io::{Read, Write};
    use std::net::{SocketAddr, TcpStream};
    use std::time::Duration;

    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap_or_else(|_| {
        SocketAddr::from(([127, 0, 0, 1], port))
    });
    let mut stream = match TcpStream::connect_timeout(&addr, Duration::from_secs(2)) {
        Ok(stream) => stream,
        Err(_) => return false,
    };
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let request = format!(
        "GET /health HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
    );
    if stream.write_all(request.as_bytes()).is_err() {
        return false;
    }
    let mut buf = [0u8; 512];
    match stream.read(&mut buf) {
        Ok(0) => false,
        Ok(n) => {
            let response = core::str::from_utf8(&buf[..n]).unwrap_or("");
            response.contains("200 OK") || response.contains("engine_version")
        }
        Err(_) => false,
    }
}

fn resolve_repo_root() -> std::path::PathBuf {
    if let Ok(root) = std::env::var("QUALIA_REPO_ROOT") {
        return std::path::PathBuf::from(root);
    }
    let mut dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    for _ in 0..8 {
        if dir.join("docs/tests/run-headless.mjs").exists() {
            return dir;
        }
        if !dir.pop() {
            break;
        }
    }
    dir
}

// ── Extended Logic & Science Tool Implementations ─────────────────────────

unsafe fn execute_evaluate_modality(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    mcp_tool_impls::evaluate_modality(args)
}

unsafe fn execute_bioinformatics_align(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    mcp_tool_impls::bioinformatics_align(args)
}

unsafe fn execute_chemical_descriptors(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    mcp_tool_impls::chemical_descriptors(args)
}

unsafe fn execute_clinical_risk(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    mcp_tool_impls::clinical_risk(args)
}

unsafe fn execute_parse_csv(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    mcp_format_impls::parse_csv(args)
}

unsafe fn execute_parse_rdf(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    mcp_format_impls::parse_rdf(args)
}

unsafe fn execute_parse_json(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    mcp_format_impls::parse_json(args)
}

unsafe fn execute_serialize_csv(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    mcp_format_impls::serialize_csv(args)
}

unsafe fn execute_serialize_json(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    mcp_format_impls::serialize_json(args)
}

unsafe fn execute_serialize_rdf(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    mcp_format_impls::serialize_rdf(args)
}

unsafe fn execute_symbolic_logic_infer(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    mcp_tool_impls::symbolic_logic_infer(args)
}

unsafe fn execute_geometric_algebra_op(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    mcp_tool_impls::geometric_algebra_op(args)
}

/// Explicitly purges memory registers to prevent data harvesting
pub unsafe fn scrub_transient_mcp_buffers(buffer: &mut [u8]) {
    for byte_ptr in buffer.iter_mut() {
        write_volatile(byte_ptr, 0x00);
    }
}

/// Map a single ASCII hex digit to its nibble value, or `None` if it is not hex.
#[inline]
fn hex_nibble(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

/// Validate and decode a raw `sanctuary_override` JSON string value into the genuine
/// 32-byte cryptographic egress-override token, returning `None` for anything that is
/// **not** a real override. This is the egress firewall's value check — it must never
/// accept mere field presence.
///
/// Fails closed (`None`) for:
/// * empty / whitespace-only values,
/// * placeholder text such as `"MISSING"`, `"null"`, `"none"` (any non-hex content),
/// * the wrong length (a token is exactly 64 hex chars = 32 bytes),
/// * the forged all-zero token — structurally valid hex that carries no authority and
///   would otherwise re-open the presence-only bypass.
///
/// Decoding is done into a fixed `[u8; 32]` on the stack — **no heap allocation**.
///
/// NOTE (MCP cooperation task #17): well-formedness is the *structural* half of the
/// gate. The token is the seam for binding egress to the caller's *verified* typed
/// standpoint — cryptographic verification against the standpoint/deontic registry is
/// performed by that layer once a call carries a verified calling-agent identifier.
/// This parser deliberately does not forge that check; it only guarantees that an
/// override which reaches the dispatch gate is a genuine, non-empty token.
fn parse_sanctuary_override(value: &[u8]) -> Option<[u8; 32]> {
    if value.len() != 64 {
        return None;
    }
    let mut token = [0u8; 32];
    for (byte, pair) in token.iter_mut().zip(value.chunks_exact(2)) {
        let hi = hex_nibble(pair[0])?;
        let lo = hex_nibble(pair[1])?;
        *byte = (hi << 4) | lo;
    }
    if token.iter().all(|&b| b == 0) {
        return None;
    }
    Some(token)
}

fn build_intent_frame(arguments: &[u8], qpu_enabled: bool, llm_enabled: bool) -> McpIntentFrame {
    let sanctuary_override = extract_raw_json_string(arguments, b"\"sanctuary_override\"")
        .and_then(parse_sanctuary_override);

    McpIntentFrame {
        purpose_hash: crate::q_hash("purpose:General"),
        active_deontic_constraints: Vec::new(),
        active_profile_id: None,
        session_nonce: 0,
        sanctuary_override,
        qpu_enabled,
        llm_enabled,
    }
}

fn tool_list_json() -> Vec<Value> {
    stable_mcp_tools()
        .iter()
        .map(|tool| {
            let input_schema = serde_json::from_str::<Value>(tool.input_schema)
                .unwrap_or_else(|_| json!({"type":"object"}));
            json!({
                "name": tool.name,
                "description": tool.description,
                "inputSchema": input_schema,
            })
        })
        .collect()
}

fn system_resource_json() -> Value {
    json!({
        "uri": "qualia://qapp-surface-schema",
        "name": "Qapp Surface Schema",
        "description": "Static description of the current Qualia qapp host surface.",
        "mimeType": "application/json",
    })
}

fn error_message(err: McpSystemError) -> (&'static str, i64) {
    match err {
        McpSystemError::SanctuaryGateTriggered => ("Sanctuary gate triggered", -32001),
        McpSystemError::ToolNotFound => ("Tool not found", -32601),
        McpSystemError::ToolNotReady => ("Tool not implemented on the current MCP surface", -32004),
        McpSystemError::ParseError => ("Parse error", -32700),
        McpSystemError::IntentFrameViolation => ("Intent frame violation", -32002),
        McpSystemError::FeatureNotEnabled => ("Feature not enabled", -32003),
        McpSystemError::InvalidParameters => ("Invalid parameters", -32602),
    }
}

fn dispatch_tool_call(
    tool_name: &[u8],
    arguments_raw: &[u8],
    qpu_enabled: bool,
    llm_enabled: bool,
) -> Result<String, McpSystemError> {
    let intent_frame = build_intent_frame(arguments_raw, qpu_enabled, llm_enabled);
    let payload = RawToolPayload {
        tool_name,
        arguments_raw,
    };
    unsafe { enforce_fiduciary_tool_dispatch(payload, &intent_frame) }
}

/// The legacy raw-byte parser retained for compatibility with older tool-call callers.
pub unsafe fn parse_and_evaluate_mcp_stream(stream_chunk: &[u8]) -> Result<String, McpSystemError> {
    if !stream_chunk.windows(12).any(|w| w == b"\"tools/call\"") {
        return Err(McpSystemError::ParseError);
    }
    let tool_name = extract_raw_json_string(stream_chunk, b"\"name\"").unwrap_or(b"");
    dispatch_tool_call(tool_name, stream_chunk, true, true)
}

pub fn handle_jsonrpc_message(
    message: &str,
    qpu_enabled: bool,
    llm_enabled: bool,
) -> Option<String> {
    let request: Value = serde_json::from_str(message).ok()?;
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let method = request.get("method").and_then(Value::as_str).or_else(|| {
        if message.contains("\"tools/call\"") {
            Some("tools/call")
        } else {
            None
        }
    })?;

    let response = match method {
        "initialize" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": "2025-03-26",
                "capabilities": {
                    "tools": { "listChanged": false },
                    "resources": { "listChanged": false, "subscribe": false },
                    "prompts": { "listChanged": false }
                },
                "serverInfo": {
                    "name": "qualia-core-db-mcp",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }
        }),
        "notifications/initialized" => return None,
        "ping" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {}
        }),
        "tools/list" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "tools": tool_list_json()
            }
        }),
        "resources/list" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "resources": [system_resource_json()]
            }
        }),
        "resources/read" => {
            let uri = request
                .get("params")
                .and_then(|params| params.get("uri"))
                .and_then(Value::as_str)
                .unwrap_or("");
            if uri != "qualia://qapp-surface-schema" {
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32602,
                        "message": "Unknown resource URI"
                    }
                })
            } else {
                let content = unsafe {
                    execute_describe_qapp_surface_schema(
                        b"{}",
                        &build_intent_frame(b"{}", qpu_enabled, llm_enabled),
                    )
                }
                .unwrap_or_else(|_| "{}".to_string());
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "contents": [{
                            "uri": uri,
                            "mimeType": "application/json",
                            "text": content
                        }]
                    }
                })
            }
        }
        "prompts/list" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "prompts": []
            }
        }),
        "tools/call" => {
            let params = request.get("params").unwrap_or(&request);
            let name = params
                .get("name")
                .and_then(Value::as_str)
                .or_else(|| request.get("name").and_then(Value::as_str))
                .unwrap_or("");
            let arguments = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            let arguments_raw = serde_json::to_vec(&arguments).unwrap_or_else(|_| b"{}".to_vec());
            match dispatch_tool_call(name.as_bytes(), &arguments_raw, qpu_enabled, llm_enabled) {
                Ok(data) => json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "content": [{
                            "type": "text",
                            "text": data
                        }],
                        "isError": false
                    }
                }),
                Err(err) => {
                    let (message, code) = error_message(err);
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": {
                            "code": code,
                            "message": message
                        }
                    })
                }
            }
        }
        _ => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": -32601,
                "message": "Method not found"
            }
        }),
    };

    Some(response.to_string())
}

// -----------------------------------------------------------------------------
// stdio Transport Logic (Allocations permitted only for handshake/metadata)
// -----------------------------------------------------------------------------

#[cfg(not(target_arch = "wasm32"))]
pub async fn start_mcp_listener() {
    start_mcp_listener_with_flags(true, true).await;
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn start_mcp_listener_with_flags(qpu_enabled: bool, llm_enabled: bool) {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    eprintln!("[MCP Server] Starting stdio MCP server...");
    eprintln!(
        "[MCP Server] Advertised tool count: {}",
        stable_mcp_tools().len()
    );

    let mut stdin = BufReader::new(tokio::io::stdin());
    let mut stdout = tokio::io::stdout();
    let mut line = String::new();

    loop {
        line.clear();
        match stdin.read_line(&mut line).await {
            Ok(0) => break,
            Ok(_) => {
                let request = line.trim_end_matches(['\r', '\n']);
                if request.is_empty() {
                    continue;
                }
                if let Some(reply) = handle_jsonrpc_message(request, qpu_enabled, llm_enabled) {
                    let _ = stdout.write_all(reply.as_bytes()).await;
                    let _ = stdout.write_all(b"\n").await;
                }
            }
            Err(_) => break,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialize_returns_server_info() {
        let reply = handle_jsonrpc_message(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
            true,
            true,
        )
        .expect("reply");
        let json: Value = serde_json::from_str(&reply).expect("valid json");
        assert_eq!(json["result"]["serverInfo"]["name"], "qualia-core-db-mcp");
        assert_eq!(json["result"]["protocolVersion"], "2025-03-26");
    }

    #[test]
    fn tools_list_returns_curated_surface() {
        let reply = handle_jsonrpc_message(
            r#"{"jsonrpc":"2.0","id":"tools","method":"tools/list"}"#,
            true,
            true,
        )
        .expect("reply");
        let json: Value = serde_json::from_str(&reply).expect("valid json");
        let tools = json["result"]["tools"].as_array().expect("tool array");
        assert!(tools.iter().any(|tool| tool["name"] == "get_system_status"));
        assert!(tools.iter().any(|tool| tool["name"] == "run_docs_tests"));
        assert!(tools.iter().all(|tool| tool.get("inputSchema").is_some()));
    }

    #[test]
    fn parse_rdf_tool_call_returns_quins() {
        let reply = handle_jsonrpc_message(
            r#"{"jsonrpc":"2.0","id":"rdf","method":"tools/call","params":{"name":"parse_rdf","arguments":{"format":"nt","rdf_data":"<http://example.org/Alice> <http://example.org/knows> <http://example.org/Bob> .\n"}}}"#,
            false,
            false,
        )
        .expect("reply");
        let json: Value = serde_json::from_str(&reply).expect("valid json");
        let text = json["result"]["content"][0]["text"]
            .as_str()
            .expect("text payload");
        let payload: Value = serde_json::from_str(text).expect("embedded json");
        assert_eq!(payload["quinCount"], 1);
    }

    #[test]
    fn parse_csv_tool_call_returns_quins() {
        let reply = handle_jsonrpc_message(
            r#"{"jsonrpc":"2.0","id":"csv","method":"tools/call","params":{"name":"parse_csv","arguments":{"csv_data":"v,n\n1,2\n","field_mappings":[{"source_key":"v","predicate":"ex:v","datatype":"integer"}]}}}"#,
            false,
            false,
        )
        .expect("reply");
        let json: Value = serde_json::from_str(&reply).expect("valid json");
        let text = json["result"]["content"][0]["text"]
            .as_str()
            .expect("text payload");
        let payload: Value = serde_json::from_str(text).expect("embedded json");
        assert_eq!(payload["quinCount"], 1);
    }

    #[test]
    fn get_graph_stats_tool_call_returns_json_payload() {
        let reply = handle_jsonrpc_message(
            r#"{"jsonrpc":"2.0","id":"graph","method":"tools/call","params":{"name":"get_graph_stats","arguments":{}}}"#,
            false,
            false,
        )
        .expect("reply");
        let json: Value = serde_json::from_str(&reply).expect("valid json");
        let text = json["result"]["content"][0]["text"]
            .as_str()
            .expect("text payload");
        let payload: Value = serde_json::from_str(text).expect("embedded json");
        assert!(payload.get("quinCount").is_some());
    }

    #[test]
    fn get_did_info_tool_call_parses_q42() {
        let reply = handle_jsonrpc_message(
            r#"{"jsonrpc":"2.0","id":"did","method":"tools/call","params":{"name":"get_did_info","arguments":{"did":"did:q42:z6MkpTHR8VNs"}}}"#,
            false,
            false,
        )
        .expect("reply");
        let json: Value = serde_json::from_str(&reply).expect("valid json");
        let text = json["result"]["content"][0]["text"]
            .as_str()
            .expect("text payload");
        let payload: Value = serde_json::from_str(text).expect("embedded json");
        assert_eq!(payload["msbSet"], true);
    }

    #[test]
    fn tools_list_includes_newly_public_stubs() {
        let reply = handle_jsonrpc_message(
            r#"{"jsonrpc":"2.0","id":"tools","method":"tools/list"}"#,
            true,
            true,
        )
        .expect("reply");
        let json: Value = serde_json::from_str(&reply).expect("valid json");
        let tools = json["result"]["tools"].as_array().expect("tool array");
        assert!(tools.iter().any(|tool| tool["name"] == "query_sparql"));
        assert!(tools.iter().any(|tool| tool["name"] == "parse_rdf"));
        assert!(tools.iter().any(|tool| tool["name"] == "list_qapps"));
        assert_eq!(tools.len(), stable_mcp_tools().len());
    }

    #[test]
    fn get_system_status_tool_call_returns_json_payload() {
        let reply = handle_jsonrpc_message(
            r#"{"jsonrpc":"2.0","id":"status","method":"tools/call","params":{"name":"get_system_status","arguments":{}}}"#,
            false,
            true,
        )
        .expect("reply");
        let json: Value = serde_json::from_str(&reply).expect("valid json");
        let text = json["result"]["content"][0]["text"]
            .as_str()
            .expect("text payload");
        let payload: Value = serde_json::from_str(text).expect("embedded json");
        assert_eq!(payload["server"], "qualia-core-db-mcp");
        assert_eq!(payload["qpuEnabled"], false);
    }
}
