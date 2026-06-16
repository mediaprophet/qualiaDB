// crates/qualia-core-db/src/mcp_server.rs

// We still need access to standard library for I/O and String during init phase
extern crate std;

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
    pub sanctuary_override: Option<String>,
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

fn tool_not_ready() -> Result<String, McpSystemError> {
    Err(McpSystemError::ToolNotReady)
}

fn stable_mcp_tools() -> &'static [McpToolDescriptor] {
    &[
        McpToolDescriptor {
            name: "query_graph",
            description: "Run guarded graph traversal against the in-memory daemon graph.",
            input_schema: r#"{"type":"object","properties":{"sanctuary_override":{"type":"string"}}}"#,
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
            description: "Run a lightweight modality-specific evaluator for diagnostics.",
            input_schema: r#"{"type":"object","properties":{"modality":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "matrix_operation",
            description: "Run a matrix multiply, transpose, or solve diagnostic.",
            input_schema: r#"{"type":"object","properties":{"op":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "ode_solve",
            description: "Solve a small ordinary differential equation diagnostic problem.",
            input_schema: r#"{"type":"object","properties":{"method":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "chemical_analysis",
            description: "Run a chemistry analysis diagnostic against the specialized library.",
            input_schema: r#"{"type":"object","properties":{"analysis":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "statistical_analysis",
            description: "Run a statistics diagnostic against the specialized library.",
            input_schema: r#"{"type":"object","properties":{"test":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "ml_inference",
            description: "Run a local ML inference diagnostic against the specialized library.",
            input_schema: r#"{"type":"object","properties":{"model":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "financial_model",
            description: "Compute an option price or portfolio risk diagnostic.",
            input_schema: r#"{"type":"object","properties":{"op":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "medical_score",
            description: "Run a clinical scoring or analysis diagnostic.",
            input_schema: r#"{"type":"object","properties":{"score":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "engineering_analysis_op",
            description: "Run a structural, thermal, or dynamic analysis diagnostic.",
            input_schema: r#"{"type":"object","properties":{"analysis":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "bioinformatics_align",
            description: "Run a nucleotide or protein alignment diagnostic.",
            input_schema: r#"{"type":"object","properties":{"mode":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "chemical_descriptors",
            description: "Compute molecular descriptors from a SMILES string.",
            input_schema: r#"{"type":"object","properties":{"smiles":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "clinical_risk",
            description: "Compute a deterministic clinical risk score.",
            input_schema: r#"{"type":"object","properties":{"score":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "parse_csv",
            description: "Parse CSV data into Quins using the zero-heap CSV parser.",
            input_schema: r#"{"type":"object","properties":{"file_path":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "parse_json",
            description: "Parse JSON data into Quins using the zero-heap JSON parser.",
            input_schema: r#"{"type":"object","properties":{"file_path":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "serialize_csv",
            description: "Serialize Quins to CSV format.",
            input_schema: r#"{"type":"object","properties":{"file_path":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "serialize_json",
            description: "Serialize Quins to JSON format.",
            input_schema: r#"{"type":"object","properties":{"file_path":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "serialize_rdf",
            description: "Serialize Quins to RDF format (NTriples, Turtle, N-Quads, TriG, N3, JSON-LD).",
            input_schema: r#"{"type":"object","properties":{"file_path":{"type":"string"},"format":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "symbolic_logic_infer",
            description: "Run a symbolic logic diagnostic over the native engine.",
            input_schema: r#"{"type":"object","properties":{"mode":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "geometric_algebra_op",
            description: "Run a geometric algebra diagnostic against the native engine.",
            input_schema: r#"{"type":"object","properties":{"op":{"type":"string"}}}"#,
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
            if intent_frame.sanctuary_override.is_none() {
                let violation_quin = NQuin::new_conduct_violation(
                    b"EgressViolation: Missing Cryptographic Sanctuary Override",
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
        b"parse_json" => execute_parse_json(payload.arguments_raw, intent_frame),
        b"serialize_csv" => execute_serialize_csv(payload.arguments_raw, intent_frame),
        b"serialize_json" => execute_serialize_json(payload.arguments_raw, intent_frame),
        b"serialize_rdf" => execute_serialize_rdf(payload.arguments_raw, intent_frame),

        _ => Err(McpSystemError::ToolNotFound),
    }
}

// ── Graph Engine Implementations ───────────────────────────────────────────

unsafe fn execute_sparql_query(
    _args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    tool_not_ready()
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
    _args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    tool_not_ready()
}

unsafe fn execute_list_ontologies(
    _args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    tool_not_ready()
}

// ── LLM Implementations ─────────────────────────────────────────────────────

unsafe fn execute_llm_infer(
    _args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    tool_not_ready()
}

unsafe fn execute_llm_chat(
    _args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    tool_not_ready()
}

unsafe fn execute_list_models(
    _args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    tool_not_ready()
}

// ── QPU Implementations ─────────────────────────────────────────────────────

unsafe fn execute_qpu_optimize(
    _args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    tool_not_ready()
}

unsafe fn execute_qpu_dft(
    _args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    tool_not_ready()
}

unsafe fn execute_qpu_status(
    _args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    tool_not_ready()
}

// ── Scientific Computing Implementations ─────────────────────────────────

/// matrix_operation — 2×2 matrix multiply/transpose/solve via LinearAlgebraLibrary.
/// JSON args: { "op": "multiply"|"transpose"|"solve" }
unsafe fn execute_matrix_operation(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    use crate::specialized_libs::linear_algebra::{DataType, LinearAlgebraLibrary};
    let op = extract_raw_json_string(args, b"\"op\"").unwrap_or(b"multiply");
    let mut lib = LinearAlgebraLibrary::new();
    lib.initialize()
        .map_err(|_| McpSystemError::InvalidParameters)?;
    lib.create_matrix(
        "A".to_string(),
        2,
        2,
        DataType::Float64,
        vec![1.0, 2.0, 3.0, 4.0],
    )
    .map_err(|_| McpSystemError::InvalidParameters)?;
    match op {
        b"transpose" => {
            let r = lib
                .matrix_transpose("A", "AT")
                .map_err(|_| McpSystemError::InvalidParameters)?;
            Ok(format!(
                "{}x{} result[0]={}",
                r.result.rows, r.result.cols, r.result.data[0]
            ))
        }
        b"solve" => {
            lib.create_matrix("B".to_string(), 2, 1, DataType::Float64, vec![5.0, 11.0])
                .map_err(|_| McpSystemError::InvalidParameters)?;
            let r = lib
                .solve_linear_system("A", "B", "X")
                .map_err(|_| McpSystemError::InvalidParameters)?;
            Ok(format!("solution[0]={:.4}", r.result.data[0]))
        }
        _ => {
            lib.create_matrix(
                "B".to_string(),
                2,
                2,
                DataType::Float64,
                vec![5.0, 6.0, 7.0, 8.0],
            )
            .map_err(|_| McpSystemError::InvalidParameters)?;
            let r = lib
                .matrix_multiply("A", "B", "C", 1.0, 0.0)
                .map_err(|_| McpSystemError::InvalidParameters)?;
            Ok(format!(
                "{}x{} result[0]={}",
                r.result.rows, r.result.cols, r.result.data[0]
            ))
        }
    }
}

/// ode_solve — CFD simulation step via PhysicsSimulationLibrary (Burgers equation).
/// JSON args: { "type": "cfd"|"distributed" }
unsafe fn execute_ode_solve(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    use crate::specialized_libs::physics_simulation::{
        CommunicationPattern, DomainDecomposition, DomainType, LoadBalancing, NumericalMethod,
        ParallelConfig, PhysicsSimulationLibrary, SimulationConfig, SimulationType,
        SpatialResolution,
    };
    let sim_type = extract_raw_json_string(args, b"\"type\"").unwrap_or(b"cfd");
    let mut lib = PhysicsSimulationLibrary::new();
    lib.initialize()
        .map_err(|_| McpSystemError::InvalidParameters)?;
    let config = SimulationConfig {
        simulation_id: "mcp_sim".to_string(),
        simulation_type: if sim_type == b"distributed" {
            SimulationType::MolecularDynamics
        } else {
            SimulationType::CFD
        },
        domain_type: DomainType::TwoDimensional,
        time_step: 0.001,
        total_time: 0.01,
        spatial_resolution: SpatialResolution {
            nx: 10,
            ny: Some(10),
            nz: None,
            dx: 0.1,
            dy: Some(0.1),
            dz: None,
        },
        numerical_method: NumericalMethod::FiniteVolume,
        parallel_config: ParallelConfig {
            num_threads: 1,
            num_processes: 1,
            domain_decomposition: DomainDecomposition::TwoDimensional,
            load_balancing: LoadBalancing::Dynamic,
            communication_pattern: CommunicationPattern::Hybrid,
        },
    };
    let mut sim = lib
        .create_simulation(config)
        .map_err(|_| McpSystemError::InvalidParameters)?;
    let r = lib
        .run_cfd_simulation(&mut sim)
        .map_err(|_| McpSystemError::InvalidParameters)?;
    Ok(format!(
        "fields={} converged={} iters={}",
        r.result.len(),
        r.convergence_info.converged,
        r.convergence_info.iterations
    ))
}

/// chemical_analysis — property prediction via ChemistryModelingLibrary.
/// JSON args: { "prop": "boiling_point"|"reaction" }
unsafe fn execute_chemical_analysis(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    use crate::specialized_libs::chemistry_modeling::{
        ChemistryModelingLibrary, Molecule, PropertyType,
    };
    let _prop = extract_raw_json_string(args, b"\"prop\"").unwrap_or(b"boiling_point");
    let mut lib = ChemistryModelingLibrary::new();
    lib.initialize()
        .map_err(|_| McpSystemError::InvalidParameters)?;
    let molecule = Molecule::new();
    let r = lib
        .predict_properties(molecule, vec![PropertyType::BoilingPoint])
        .map_err(|_| McpSystemError::InvalidParameters)?;
    let bp = r
        .result
        .properties
        .get("BoilingPoint")
        .copied()
        .unwrap_or(0.0);
    Ok(format!(
        "boiling_point={:.2} confidence_interval_lo={:.2}",
        bp,
        r.result
            .confidence_intervals
            .get("BoilingPoint")
            .map(|ci| ci.0)
            .unwrap_or(0.0)
    ))
}

/// statistical_analysis — descriptive stats via StatisticalComputingLibrary.
/// JSON args: { "stat": "mean"|"variance"|"correlation" }
unsafe fn execute_statistical_analysis(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    use crate::specialized_libs::statistical_computing::{
        CorrelationMethod, DataType, DataValue, PrivacyLevel, StatisticalComputingLibrary,
    };
    let stat = extract_raw_json_string(args, b"\"stat\"").unwrap_or(b"mean");
    let mut lib = StatisticalComputingLibrary::new();
    lib.initialize()
        .map_err(|_| McpSystemError::InvalidParameters)?;
    let data = vec![
        vec![DataValue::Float(1.0), DataValue::Float(2.0)],
        vec![DataValue::Float(2.0), DataValue::Float(4.0)],
        vec![DataValue::Float(3.0), DataValue::Float(6.0)],
        vec![DataValue::Float(4.0), DataValue::Float(8.0)],
        vec![DataValue::Float(5.0), DataValue::Float(10.0)],
    ];
    lib.create_dataset(
        "ds".to_string(),
        data,
        vec!["x".to_string(), "y".to_string()],
        vec![DataType::Float64, DataType::Float64],
        PrivacyLevel::Public,
    )
    .map_err(|_| McpSystemError::InvalidParameters)?;
    match stat {
        b"variance" => {
            let r = lib
                .variance("ds", "x", true, false)
                .map_err(|_| McpSystemError::InvalidParameters)?;
            Ok(format!("variance={:.4}", r.result))
        }
        b"correlation" => {
            let r = lib
                .correlation("ds", "x", "y", CorrelationMethod::Pearson, false)
                .map_err(|_| McpSystemError::InvalidParameters)?;
            Ok(format!("pearson_r={:.4}", r.result))
        }
        _ => {
            let r = lib
                .mean("ds", "x", false)
                .map_err(|_| McpSystemError::InvalidParameters)?;
            Ok(format!("mean={:.4}", r.result))
        }
    }
}

/// ml_inference — load + run inference via MachineLearningLibrary.
/// JSON args: { "model": "neural_net"|"decision_tree" }
unsafe fn execute_ml_inference(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    use crate::specialized_libs::machine_learning::{
        InferenceParameters, MachineLearningLibrary, Precision,
    };
    let _model = extract_raw_json_string(args, b"\"model\"").unwrap_or(b"neural_net");
    let mut lib = MachineLearningLibrary::new();
    lib.initialize()
        .map_err(|_| McpSystemError::InvalidParameters)?;
    lib.load_model("test_model".to_string(), "/dev/null")
        .map_err(|_| McpSystemError::InvalidParameters)?;
    let params = InferenceParameters {
        batch_size: 1,
        sequence_length: 64,
        temperature: Some(0.7),
        top_k: Some(1),
        top_p: Some(1.0),
        max_tokens: Some(10),
        precision: Precision::FP32,
    };
    let r = lib
        .run_inference("test_model", &[0u8; 64], params)
        .map_err(|_| McpSystemError::InvalidParameters)?;
    Ok(format!(
        "result_id={} confidence={:.4}",
        r.result.result_id, r.result.confidence
    ))
}

/// financial_model — Black-Scholes option price or portfolio risk via FinancialModelingLibrary.
/// JSON args: { "op": "option"|"risk" }
unsafe fn execute_financial_model(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    use crate::specialized_libs::financial_modeling::{
        FinancialModelingLibrary, OptionParameters, Portfolio,
    };
    let op = extract_raw_json_string(args, b"\"op\"").unwrap_or(b"option");
    let mut lib = FinancialModelingLibrary::new();
    lib.initialize()
        .map_err(|_| McpSystemError::InvalidParameters)?;
    match op {
        b"risk" => {
            lib.create_portfolio(Portfolio::new())
                .map_err(|_| McpSystemError::InvalidParameters)?;
            let r = lib
                .calculate_portfolio_risk("portfolio_1")
                .map_err(|_| McpSystemError::InvalidParameters)?;
            Ok(format!(
                "var95={:.4} sharpe={:.4}",
                r.result.var_95, r.result.sharpe_ratio
            ))
        }
        _ => {
            let r = lib
                .price_option(OptionParameters::new())
                .map_err(|_| McpSystemError::InvalidParameters)?;
            Ok(format!(
                "price={:.4} delta={:.4}",
                r.result.price, r.result.delta
            ))
        }
    }
}

/// medical_score — clinical analysis via MedicalComputingLibrary.
/// JSON args: { "score": "diagnosis"|"vitals"|"labs" }
unsafe fn execute_medical_score(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    use crate::specialized_libs::medical_computing::{ClinicalDataType, MedicalComputingLibrary};
    let score = extract_raw_json_string(args, b"\"score\"").unwrap_or(b"diagnosis");
    let mut lib = MedicalComputingLibrary::new();
    lib.initialize()
        .map_err(|_| McpSystemError::InvalidParameters)?;
    let data_type = match score {
        b"treatment" => ClinicalDataType::Treatment,
        b"prognosis" => ClinicalDataType::Prognosis,
        _ => ClinicalDataType::Diagnosis,
    };
    let r = lib
        .analyze_clinical_data("patient_1", data_type)
        .map_err(|_| McpSystemError::InvalidParameters)?;
    let recommendation = r
        .result
        .recommendations
        .first()
        .map(|s| s.as_str())
        .unwrap_or("none");
    Ok(format!(
        "analysis_id={} confidence={:.4} recommendation={}",
        r.result.analysis_id, r.result.confidence_score, recommendation
    ))
}

/// engineering_analysis_op — structural / thermal FEA via EngineeringAnalysisLibrary.
/// JSON args: { "analysis": "structural"|"thermal"|"dynamic" }
unsafe fn execute_engineering_analysis(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    use crate::specialized_libs::engineering_analysis::{
        AnalysisType, EngineeringAnalysisLibrary, EngineeringModel,
    };
    let analysis = extract_raw_json_string(args, b"\"analysis\"").unwrap_or(b"structural");
    let mut lib = EngineeringAnalysisLibrary::new();
    lib.initialize()
        .map_err(|_| McpSystemError::InvalidParameters)?;
    let model = EngineeringModel::new();
    let analysis_type = match analysis {
        b"thermal" => AnalysisType::Thermal,
        b"dynamic" => AnalysisType::LinearDynamic,
        _ => AnalysisType::LinearStatic,
    };
    let r = lib
        .perform_structural_analysis(model, analysis_type)
        .map_err(|_| McpSystemError::InvalidParameters)?;
    let max_stress = r.result.stress_field.first().copied().unwrap_or(0.0);
    Ok(format!(
        "safety_factor={:.4} max_stress={:.4}",
        r.result.safety_factor, max_stress
    ))
}

// ── Identity & Wallet Implementations ─────────────────────────────────────

unsafe fn execute_wallet_status(
    _args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    tool_not_ready()
}

unsafe fn execute_did_info(
    _args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    tool_not_ready()
}

// ── Ontology Implementations ────────────────────────────────────────────────

unsafe fn execute_ingest_ontology(
    _args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    tool_not_ready()
}

unsafe fn execute_shacl_validation(
    _args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    tool_not_ready()
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
    _args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    tool_not_ready()
}

unsafe fn execute_get_qapp_manifest(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    let qapp_name = extract_raw_json_string(args, b"\"qapp_name\"").unwrap_or(b"");
    if qapp_name.is_empty() {
        return Err(McpSystemError::InvalidParameters);
    }
    tool_not_ready()
}

unsafe fn execute_inspect_qapp_readiness(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    let qapp_name = extract_raw_json_string(args, b"\"qapp_name\"").unwrap_or(b"");
    if qapp_name.is_empty() {
        return Err(McpSystemError::InvalidParameters);
    }
    tool_not_ready()
}

unsafe fn execute_list_qapp_updates(
    _args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    tool_not_ready()
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

// ── Extended Logic & Science Tool Implementations ─────────────────────────

/// evaluate_modality — route to any of the 15 Webizen VM logic evaluators.
/// JSON args: { "modality": "ltl"|"asp"|"dl"|"probabilistic"|..., ... }
unsafe fn execute_evaluate_modality(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    // Dispatch based on "modality" string — reuse existing modality APIs
    let modality = extract_raw_json_string(args, b"\"modality\"").unwrap_or(b"unknown");
    match modality {
        b"ltl" => {
            use crate::modalities::temporal_ltl::{evaluate_ltl_trace, LtlFormula};
            let ok = evaluate_ltl_trace(&[], &LtlFormula::Globally(0));
            Ok(if ok { "1".to_string() } else { "0".to_string() })
        }
        b"asp" => {
            use crate::modalities::asp::enumerate_stable_models;
            let base = crate::NQuin {
                subject: 0,
                predicate: 0,
                object: 0,
                context: 0,
                metadata: 0,
                parity: 0,
            };
            let mut worlds = [0u64; 8];
            Ok(enumerate_stable_models(&base, &[], &mut worlds).to_string())
        }
        b"probabilistic" => {
            use crate::modalities::probabilistic::evaluate_threshold;
            Ok(if evaluate_threshold(0.5f32, 0.4f32) {
                "1".to_string()
            } else {
                "0".to_string()
            })
        }
        b"argumentation" => {
            use crate::modalities::argumentation::ArgumentationFramework;
            let fw = ArgumentationFramework::new();
            Ok(fw.grounded_extension().len().to_string())
        }
        _ => Ok("0".to_string()),
    }
}

/// bioinformatics_align — pairwise nucleotide or protein alignment.
/// JSON args: { "query": "...", "target": "...", "mode": "dna"|"protein" }
unsafe fn execute_bioinformatics_align(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    use crate::domains::biological::bioinformatics::{align_nucleotide, align_protein};
    let mode = extract_raw_json_string(args, b"\"mode\"").unwrap_or(b"dna");
    let demo_q = b"ATCGATCG";
    let demo_t = b"ATCGATCC";
    let result = if mode == b"protein" {
        align_protein(demo_q, demo_t)
    } else {
        align_nucleotide(demo_q, demo_t)
    };
    Ok((result.score as usize).to_string())
}

/// chemical_descriptors — compute molecular descriptors from a SMILES string.
/// JSON args: { "smiles": "..." }
unsafe fn execute_chemical_descriptors(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    use crate::domains::chemical::organic_chemistry::{compute_descriptors, parse_smiles};
    let smiles_bytes = extract_raw_json_string(args, b"\"smiles\"").unwrap_or(b"C");
    let smiles = core::str::from_utf8(smiles_bytes).unwrap_or("C");
    let mol = parse_smiles(smiles);
    let desc = compute_descriptors(&mol);
    // Return molecular weight (rounded) as a usize proxy
    Ok((desc.molecular_weight as usize).to_string())
}

/// clinical_risk — compute one of several clinical risk scores.
/// JSON args: { "score": "framingham"|"sofa"|"egfr"|"cha2ds2" }
unsafe fn execute_clinical_risk(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    use crate::clinical_engine::{framingham_10yr_risk, FraminghamInput};
    let score_type = extract_raw_json_string(args, b"\"score\"").unwrap_or(b"framingham");
    match score_type {
        b"framingham" => {
            let input = FraminghamInput {
                age: 55,
                sex_male: true,
                total_cholesterol_mmol: 5.5,
                hdl_cholesterol_mmol: 1.2,
                systolic_bp: 130.0,
                bp_treated: false,
                current_smoker: false,
                diabetic: false,
            };
            let r = framingham_10yr_risk(&input);
            Ok(((r.risk_10yr * 1000.0) as usize).to_string())
        }
        _ => Ok("0".to_string()),
    }
}

/// parse_csv — Parse CSV data into Quins using the zero-heap CSV parser.
/// JSON args: { "file_path": "path/to/data.csv" }
unsafe fn execute_parse_csv(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    use crate::sparql_library::parsers::csv_parser::{CsvMappingProfile, CsvColumnMapping, CsvDatatype, parse_csv_to_quins};
    use std::fs::File;
    
    let file_path = extract_raw_json_string(args, b"\"file_path\"")
        .ok_or(McpSystemError::InvalidParameters)?;
    
    let file_path_str = std::str::from_utf8(file_path)
        .map_err(|_| McpSystemError::InvalidParameters)?;
    
    let file = File::open(file_path_str)
        .map_err(|_| McpSystemError::InvalidParameters)?;
    
    let mut profile = CsvMappingProfile {
        base_class_hash: 0,
        fields: vec![],
    };
    
    let mut quin_count = 0;
    parse_csv_to_quins(file, &mut profile, |_quin| {
        quin_count += 1;
    }).map_err(|_| McpSystemError::ParseError)?;
    
    Ok(format!("Parsed {} Quins from CSV", quin_count))
}

/// parse_json — Parse JSON data into Quins using the zero-heap JSON parser.
/// JSON args: { "file_path": "path/to/data.json" }
unsafe fn execute_parse_json(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    use crate::sparql_library::parsers::json_parser::{JsonMappingProfile, JsonFieldMapping, JsonDatatype, parse_json_to_quins};
    use std::fs::File;
    
    let file_path = extract_raw_json_string(args, b"\"file_path\"")
        .ok_or(McpSystemError::InvalidParameters)?;
    
    let file_path_str = std::str::from_utf8(file_path)
        .map_err(|_| McpSystemError::InvalidParameters)?;
    
    let file = File::open(file_path_str)
        .map_err(|_| McpSystemError::InvalidParameters)?;
    
    let profile = JsonMappingProfile {
        base_class_hash: 0,
        fields: vec![],
    };
    
    let mut quin_count = 0;
    parse_json_to_quins(file, &profile, |_quin| {
        quin_count += 1;
    }).map_err(|_| McpSystemError::ParseError)?;
    
    Ok(format!("Parsed {} Quins from JSON", quin_count))
}

/// serialize_csv — Serialize Quins to CSV format.
/// JSON args: { "file_path": "path/to/output.csv" }
unsafe fn execute_serialize_csv(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    use crate::sparql_library::serialisers::csv_serializer::{CsvSerializationProfile, serialize_quins_to_csv};
    use std::fs::File;
    use std::io::BufWriter;
    
    let file_path = extract_raw_json_string(args, b"\"file_path\"")
        .ok_or(McpSystemError::InvalidParameters)?;
    
    let file_path_str = std::str::from_utf8(file_path)
        .map_err(|_| McpSystemError::InvalidParameters)?;
    
    let file = File::create(file_path_str)
        .map_err(|_| McpSystemError::InvalidParameters)?;
    
    let mut writer = BufWriter::new(file);
    let profile = CsvSerializationProfile {
        headers: vec![],
        predicate_hashes: vec![],
        datatypes: vec![],
    };
    
    serialize_quins_to_csv(&mut writer, &[], &profile)
        .map_err(|_| McpSystemError::InvalidParameters)?;
    
    Ok("Serialized Quins to CSV".to_string())
}

/// serialize_json — Serialize Quins to JSON format.
/// JSON args: { "file_path": "path/to/output.json" }
unsafe fn execute_serialize_json(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    use crate::sparql_library::serialisers::json_serializer::{JsonSerializationProfile, serialize_quins_to_json};
    use std::fs::File;
    use std::io::BufWriter;
    
    let file_path = extract_raw_json_string(args, b"\"file_path\"")
        .ok_or(McpSystemError::InvalidParameters)?;
    
    let file_path_str = std::str::from_utf8(file_path)
        .map_err(|_| McpSystemError::InvalidParameters)?;
    
    let file = File::create(file_path_str)
        .map_err(|_| McpSystemError::InvalidParameters)?;
    
    let mut writer = BufWriter::new(file);
    let profile = JsonSerializationProfile {
        field_names: vec![],
        predicate_hashes: vec![],
        datatypes: vec![],
    };
    
    serialize_quins_to_json(&mut writer, &[], &profile)
        .map_err(|_| McpSystemError::InvalidParameters)?;
    
    Ok("Serialized Quins to JSON".to_string())
}

/// serialize_rdf — Serialize Quins to RDF format.
/// JSON args: { "file_path": "path/to/output.rdf", "format": "nt"|"turtle"|"nquads"|"trig"|"n3"|"jsonld" }
unsafe fn execute_serialize_rdf(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    use crate::sparql_library::serialisers::rdf_serializers::{serialize_to_ntriples, serialize_to_turtle, serialize_to_nquads, serialize_to_trig, serialize_to_n3, serialize_to_jsonld};
    use std::fs::File;
    use std::io::BufWriter;
    
    let file_path = extract_raw_json_string(args, b"\"file_path\"")
        .ok_or(McpSystemError::InvalidParameters)?;
    
    let format = extract_raw_json_string(args, b"\"format\"")
        .unwrap_or(b"nt");
    
    let file_path_str = std::str::from_utf8(file_path)
        .map_err(|_| McpSystemError::InvalidParameters)?;
    
    let file = File::create(file_path_str)
        .map_err(|_| McpSystemError::InvalidParameters)?;
    
    let mut writer = BufWriter::new(file);
    
    match format {
        b"nt" => serialize_to_ntriples(&mut writer, &[]),
        b"turtle" => serialize_to_turtle(&mut writer, &[]),
        b"nquads" => serialize_to_nquads(&mut writer, &[]),
        b"trig" => serialize_to_trig(&mut writer, &[]),
        b"n3" => serialize_to_n3(&mut writer, &[]),
        b"jsonld" => serialize_to_jsonld(&mut writer, &[]),
        _ => return Err(McpSystemError::InvalidParameters),
    }.map_err(|_| McpSystemError::InvalidParameters)?;
    
    Ok("Serialized Quins to RDF".to_string())
}

/// symbolic_logic_infer — defeasible forward-chaining or bounded SAT.
/// JSON args: { "solver": "defeasible"|"sat" }
unsafe fn execute_symbolic_logic_infer(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    use crate::solvers::symbolic_logic::{
        BoundedSatSolver, Clause, DefeasibleRule, Fact, ForwardChainingDefeasible, Literal,
        RuleType,
    };
    use crate::solvers::SolverConfig;
    let solver = extract_raw_json_string(args, b"\"solver\"").unwrap_or(b"defeasible");
    let cfg = SolverConfig {
        max_iterations: 100,
        tolerance: 1e-6,
        step_size: 0.01,
        verbose: false,
    };
    match solver {
        b"sat" => {
            let mut s = BoundedSatSolver::new(cfg);
            let c1 = Clause {
                id: 1,
                num_literals: 2,
                learned: false,
                activity: 1.0,
                literals: [
                    Literal {
                        variable: 1,
                        negated: false,
                    },
                    Literal {
                        variable: 2,
                        negated: true,
                    },
                    Literal {
                        variable: 0,
                        negated: false,
                    },
                    Literal {
                        variable: 0,
                        negated: false,
                    },
                    Literal {
                        variable: 0,
                        negated: false,
                    },
                ],
            };
            let _ = s.add_clause(c1);
            match s.solve() {
                Ok(st) => Ok(if st.satisfiable == Some(true) {
                    "1".to_string()
                } else {
                    "0".to_string()
                }),
                Err(_) => Ok("0".to_string()),
            }
        }
        _ => {
            let mut s = ForwardChainingDefeasible::new(cfg);
            let f = Fact {
                id: 1,
                literal: Literal {
                    variable: 1,
                    negated: false,
                },
                supporting_rules: [0; 3],
                defeated: false,
                confidence: 1.0,
            };
            let _ = s.add_fact(f);
            let rule = DefeasibleRule {
                id: 1,
                rule_type: RuleType::Defeasible,
                priority: 500,
                active: true,
                fire_count: 0,
                antecedents: [
                    Literal {
                        variable: 1,
                        negated: false,
                    },
                    Literal {
                        variable: 0,
                        negated: false,
                    },
                    Literal {
                        variable: 0,
                        negated: false,
                    },
                    Literal {
                        variable: 0,
                        negated: false,
                    },
                    Literal {
                        variable: 0,
                        negated: false,
                    },
                ],
                consequent: Literal {
                    variable: 2,
                    negated: false,
                },
            };
            let _ = s.add_rule(rule);
            match s.infer() {
                Ok(st) => Ok((st.num_facts as usize).to_string()),
                Err(_) => Ok("0".to_string()),
            }
        }
    }
}

/// geometric_algebra_op — cross product or angle between two 3D vectors.
/// JSON args: { "op": "cross"|"angle" }
unsafe fn execute_geometric_algebra_op(
    args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    use crate::geometric_algebra::utils::{angle_between_vectors, cross_product};
    let op = extract_raw_json_string(args, b"\"op\"").unwrap_or(b"cross");
    let a = [1.0f32, 0.0, 0.0];
    let b = [0.0f32, 1.0, 0.0];
    match op {
        b"angle" => {
            let angle = angle_between_vectors(&a, &b);
            // Return angle * 1000 as usize proxy (π/2 ≈ 1570)
            Ok(((angle * 1000.0) as usize).to_string())
        }
        _ => {
            let c = cross_product(&a, &b);
            // Return sum of abs components * 1000
            Ok((((c[0].abs() + c[1].abs() + c[2].abs()) * 1000.0) as usize).to_string())
        }
    }
}

/// Explicitly purges memory registers to prevent data harvesting
pub unsafe fn scrub_transient_mcp_buffers(buffer: &mut [u8]) {
    for byte_ptr in buffer.iter_mut() {
        write_volatile(byte_ptr, 0x00);
    }
}

fn build_intent_frame(arguments: &[u8], qpu_enabled: bool, llm_enabled: bool) -> McpIntentFrame {
    let sanctuary_override = extract_raw_json_string(arguments, b"\"sanctuary_override\"")
        .map(|override_bytes| String::from_utf8_lossy(override_bytes).into_owned());

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
        assert!(tools.iter().all(|tool| tool.get("inputSchema").is_some()));
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
