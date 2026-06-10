//! SHACL Extensions for Core Modalities
//!
//! This module provides SHACL constraint extensions for the core logical reasoning modalities
//! including epistemic logic, paraconsistent logic, temporal LTL, spatio-temporal reasoning,
//! graph theory, calculus, argumentation, dialectical logic, ASP, and probabilistic reasoning.

use crate::webizen::SlgOpcode;

// ── Epistemic Logic Constraints ─────────────────────────────────────────────────

/// `q42:EpistemicConfiguration` — validates epistemic reasoning parameters
#[derive(Debug, Clone)]
pub struct EpistemicConfiguration {
    pub max_certainty: u8,           // Maximum certainty value (0-255)
    pub max_nesting_depth: u8,       // Maximum RDF-Star nesting depth
    pub max_agent_contexts: u32,     // Maximum number of agent contexts
    pub common_knowledge_buffer_size: u32, // Buffer size for common knowledge
}

/// `q42:EpistemicQuery` — validates epistemic query parameters
#[derive(Debug, Clone)]
pub struct EpistemicQuery {
    pub query_type: String,          // "knows", "believes", "common_knowledge"
    pub min_certainty_threshold: u8,
    pub require_agent_did: bool,     // Require specific agent DID
    pub require_world_context: bool, // Require specific world/context
}

// ── Paraconsistent Logic Constraints ─────────────────────────────────────────

/// `q42:ParaconsistentConfiguration` — validates paraconsistent reasoning parameters
#[derive(Debug, Clone)]
pub struct ParaconsistentConfiguration {
    pub max_isolation_severity: u8,  // Maximum contradiction severity
    pub isolation_buffer_size: u32,    // Buffer size for isolated contradictions
    pub merge_threshold: f64,          // Threshold for paraconsistent merge
    pub require_context_tracking: bool, // Require isolation context tracking
}

/// `q42:ContradictionHandling` — validates contradiction handling parameters
#[derive(Debug, Clone)]
pub struct ContradictionHandling {
    pub allowed_handling_modes: Vec<String>, // ["isolate", "merge", "reject"]
    pub max_contradictions_per_context: u32,
    pub require_audit_trail: bool,
}

// ── Temporal LTL Constraints ───────────────────────────────────────────────────

/// `q42:LTLConfiguration` — validates temporal LTL formula parameters
#[derive(Debug, Clone)]
pub struct LTLConfiguration {
    pub max_trace_length: u32,        // Maximum temporal trace length
    pub max_formula_depth: u8,        // Maximum nesting depth of LTL formulas
    pub allowed_operators: Vec<String>, // ["G", "F", "X", "U", "R"]
    pub require_well_formedness: bool, // Require well-formed temporal formulas
}

/// `q42:TemporalTrace` — validates temporal trace parameters
#[derive(Debug, Clone)]
pub struct TemporalTrace {
    pub min_trace_length: u32,
    pub max_trace_length: u32,
    pub require_monotonic_timestamps: bool,
    pub allowed_temporal_granularity: Vec<String>, // ["seconds", "milliseconds", "nanoseconds"]
}

// ── Spatio-Temporal Constraints ─────────────────────────────────────────────

/// `q42:SpatioTemporalConfiguration` — validates spatio-temporal reasoning parameters
#[derive(Debug, Clone)]
pub struct SpatioTemporalConfiguration {
    pub max_spatial_regions: u32,     // Maximum number of spatial regions
    pub max_temporal_intervals: u32,   // Maximum number of temporal intervals
    pub max_region_complexity: u8,     // Maximum boundary points per region
    pub allowed_spatial_relations: Vec<String>, // RCC8 relations
}

/// `q42:AllenIntervalConfiguration` — validates Allen interval algebra parameters
#[derive(Debug, Clone)]
pub struct AllenIntervalConfiguration {
    pub max_interval_overlap: f64,     // Maximum allowed overlap (0.0 to 1.0)
    pub require_consistency: bool,     // Require interval consistency
    pub allowed_temporal_relations: Vec<String>, // Allen's 7 relations
}

/// `q42:SpatialRegionConfiguration` — validates spatial region parameters
#[derive(Debug, Clone)]
pub struct SpatialRegionConfiguration {
    pub max_boundary_points: u32,     // Maximum boundary points per region
    pub min_region_area: f64,         // Minimum region area in square units
    pub max_region_area: f64,         // Maximum region area in square units
    pub require_simple_polygon: bool,   // Require non-self-intersecting polygons
}

// ── Graph Theory Constraints ─────────────────────────────────────────────────

/// `q42:GraphConfiguration` — validates graph structure parameters
#[derive(Debug, Clone)]
pub struct GraphConfiguration {
    pub max_nodes: u32,               // Maximum number of nodes in graph
    pub max_edges: u32,               // Maximum number of edges in graph
    pub max_node_degree: u32,          // Maximum degree per node
    pub allowed_graph_types: Vec<String>, // ["directed", "undirected", "weighted", "bipartite"]
}

/// `q42:GraphAnalysisConfiguration` — validates graph analysis parameters
#[derive(Debug, Clone)]
pub struct GraphAnalysisConfiguration {
    pub allowed_centrality_measures: Vec<String>, // ["degree", "betweenness", "closeness"]
    pub max_community_count: u32,     // Maximum number of communities
    pub min_community_size: u32,      // Minimum community size
    pub require_connectedness: bool,    // Require graph to be connected
}

/// `q42:GraphAlgorithmConfiguration` — validates graph algorithm parameters
#[derive(Debug, Clone)]
pub struct GraphAlgorithmConfiguration {
    pub max_iterations: u32,         // Maximum algorithm iterations
    pub convergence_threshold: f64,  // Convergence threshold
    pub allowed_algorithms: Vec<String>, // ["pagerank", "dijkstra", "floyd_warshall"]
    pub require_termination: bool,   // Require algorithm termination guarantee
}

// ── Calculus Constraints ─────────────────────────────────────────────────────

/// `q42:CalculusConfiguration` — validates numerical integration parameters
#[derive(Debug, Clone)]
pub struct CalculusConfiguration {
    pub max_grid_points: u64,         // Maximum number of grid points
    pub max_integration_steps: u64,   // Maximum integration steps
    pub allowed_integrators: Vec<String>, // ["simpsons", "trapezoidal", "rk4", "adaptive"]
    pub require_convergence_check: bool,
}

/// `q42:ODEConfiguration` — validates ODE solver parameters
#[derive(Debug, Clone)]
pub struct ODEConfiguration {
    pub max_ode_order: u8,            // Maximum ODE order (1st, 2nd, etc.)
    pub max_step_size: f64,           // Maximum step size
    pub min_step_size: f64,           // Minimum step size
    pub allowed_solvers: Vec<String>, // ["rk4", "adaptive", "implicit"]
    pub require_stability_check: bool,  // Require stability analysis
}

/// `q42:TensorProvenanceConfiguration` — validates tensor provenance tracking
#[derive(Debug, Clone)]
pub struct TensorProvenanceConfiguration {
    pub max_tensor_rank: u8,         // Maximum tensor rank
    pub max_tensor_dimensions: u8,   // Maximum number of tensor dimensions
    pub require_origin_tracking: bool, // Require origin/source tracking
    pub allowed_operations: Vec<String>, // ["matmul", "transpose", "contraction"]
}

// ── Argumentation Theory Constraints ───────────────────────────────────────

/// `q42:ArgumentationConfiguration` — validates argumentation framework parameters
#[derive(Debug, Clone)]
pub struct ArgumentationConfiguration {
    pub max_arguments: u32,          // Maximum number of arguments in framework
    pub max_attack_depth: u8,         // Maximum attack/defense depth
    pub max_defense_depth: u8,        // Maximum defense depth
    pub allowed_argument_types: Vec<String>, // ["deductive", "inductive", "abductive"]
}

/// `q42:ArgumentEvaluationConfiguration` — validates argument evaluation parameters
#[derive(Debug, Clone)]
pub struct ArgumentEvaluationConfiguration {
    pub max_evaluation_steps: u32,  // Maximum evaluation steps
    pub allowed_evaluators: Vec<String>, // ["grounded", "abstract", "probabilistic"]
    pub require_consistency_check: bool,
    pub max_confidence_interval: f64,
}

// ── Dialectical Logic Constraints ─────────────────────────────────────────────

/// `q42:DialecticalConfiguration` — validates dialectical synthesis parameters
#[derive(Debug, Clone)]
pub struct DialecticalConfiguration {
    pub max_thesis_count: u32,       // Maximum number of theseses
    pub max_antithesis_count: u32,    // Maximum number of antitheses
    pub max_synthesis_rounds: u32,    // Maximum synthesis rounds
    pub require_contradiction_detection: bool,
}

/// `q42:SynthesisConfiguration` — validates dialectical synthesis parameters
#[derive(Debug, Clone)]
pub struct SynthesisConfiguration {
    pub max_synthesis_depth: u8,     // Maximum synthesis depth
    pub allowed_synthesis_methods: Vec<String>, // ["hegelian", "marxist", "aristotelian"]
    pub require_coherence_check: bool, // Require logical coherence
    pub min_coherence_score: f64,     // Minimum coherence threshold
}

// ── ASP (Answer Set Programming) Constraints ─────────────────────────────

/// `q42:ASPConfiguration` — validates ASP solver parameters
#[derive(Debug, Clone)]
pub struct ASPConfiguration {
    pub max_variables: u32,          // Maximum number of ASP variables
    pub max_clauses: u32,             // Maximum number of ASP clauses
    pub max_answer_sets: u32,         // Maximum number of answer sets
    pub allowed_solvers: Vec<String>, // ["clingo", "dlv", "aspino"]
}

/// `q42:StableModelConfiguration` — validates stable model computation parameters
#[derive(Debug, Clone)]
pub struct StableModelConfiguration {
    pub max_model_complexity: u32,   // Maximum model complexity score
    pub max_grounding_time_ms: u64,   // Maximum grounding time
    pub require_optimization: bool,   // Require optimization hints
    pub allowed_search_strategies: Vec<String>, // ["branch_and_bound", "local_search", "genetic"]
}

// ── Probabilistic Reasoning Constraints ───────────────────────────────────────

/// `q42:ProbabilisticConfiguration` — validates probabilistic reasoning parameters
#[derive(Debug, Clone)]
pub struct ProbabilisticConfiguration {
    pub max_random_variables: u32,   // Maximum number of random variables
    pub max_probability_precision: u8, // Bits of probability precision
    pub allowed_distributions: Vec<String>, // ["normal", "uniform", "exponential", "beta"]
    pub require_normalization: bool,   // Require probability normalization
}

/// `q42:BayesianInferenceConfiguration` — validates Bayesian inference parameters
#[derive(Debug, Clone)]
pub struct BayesianInferenceConfiguration {
    pub max_hypothesis_count: u32,   // Maximum number of hypotheses
    pub max_evidence_count: u32,      // Maximum evidence count
    pub allowed_inference_methods: Vec<String>, // ["mcmc", "variational", "exact"]
    pub burn_in_samples: u32,          // Number of burn-in samples
    pub post_burn_in_samples: u32,    // Number of post-burn-in samples
}

// ── DL (Description Logic) Constraints ─────────────────────────────────────────

/// `q42:DLConfiguration` — validates description logic parameters
#[derive(Debug, Clone)]
pub struct DLConfiguration {
    pub max_predicates: u32,          // Maximum number of predicates
    pub max_clauses: u32,             // Maximum number of clauses
    pub max_herbrand_universe: u64,    // Maximum Herbrand universe size
    pub allowed_reasoners: Vec<String>, // ["forward", "backward", "SLD"]
}

/// `q42:DLQueryConfiguration` — validates DL query parameters
#[derive(Debug, Clone)]
pub struct DLQueryConfiguration {
    pub max_query_depth: u8,          // Maximum query nesting depth
    pub allowed_query_types: Vec<String>, // ["conjunctive", "disjunctive", "existential", "universal"]
    pub require_safety: bool,          // Require query safety guarantees
    pub max_result_limit: u32,         // Maximum number of results
}

// ── Diffusion Constraints ─────────────────────────────────────────────────────

/// `q42:DiffusionConfiguration` — validates diffusion process parameters
#[derive(Debug, Clone)]
pub struct DiffusionConfiguration {
    pub max_particles: u64,           // Maximum number of particles
    pub max_time_steps: u64,           // Maximum simulation time steps
    pub allowed_schemes: Vec<String>, // ["brownian", "geometric", "levy"]
    pub require_stability: bool,       // Require numerical stability
}

/// `q42:DiffusionGridConfiguration` — validates diffusion grid parameters
#[derive(Debug, Clone)]
pub struct DiffusionGridConfiguration {
    pub max_grid_dimensions: u32,     // Maximum grid dimensionality
    pub max_grid_size_per_dim: u64,   // Maximum grid size per dimension
    pub allowed_boundary_conditions: Vec<String>, // ["reflective", "absorbing", "periodic"]
    pub require_stability_check: bool,   // Require CFL condition check
}

// ── Linear Logic Constraints ───────────────────────────────────────────────────

/// `q42:LinearLogicConfiguration` — validates linear logic parameters
#[derive(Debug, Clone)]
pub struct LinearLogicConfiguration {
    pub max_literals: u32,           // Maximum number of literals
    pub max_clauses: u32,             // Maximum number of clauses
    pub max_resource_capacity: u32,   // Maximum resource capacity
    pub require_resource_tracking: bool,
}

// ── Control Feedback Constraints ─────────────────────────────────────────────

/// `q42:ControlFeedbackConfiguration` — validates control feedback loop parameters
#[derive(Debug, Clone)]
pub struct ControlFeedbackConfiguration {
    pub max_feedback_loops: u8,       // Maximum number of nested feedback loops
    pub max_loop_iterations: u64,     // Maximum iterations per loop
    pub allowed_controller_types: Vec<String>, // ["pid", "mpc", "robust"]
    pub require_stability_analysis: bool, // Require stability analysis
}

/// `q42:FeedbackGainConfiguration` — validates feedback gain parameters
#[derive(Debug, Clone)]
pub struct FeedbackGainConfiguration {
    pub max_proportional_gain: f64,   // Maximum proportional gain
    pub max_integral_gain: f64,       // Maximum integral gain
    pub max_derivative_gain: f64,      // Maximum derivative gain
    pub require_gain_scheduling: bool, // Require gain scheduling
}

// ── Interval Reasoning Constraints ─────────────────────────────────────────

/// `q42:IntervalArithmeticConfiguration` — validates interval arithmetic parameters
#[derive(Debug, Clone)]
pub struct IntervalArithmeticConfiguration {
    pub max_interval_width: f64,      // Maximum interval width
    pub allowed_operations: Vec<String>, // ["addition", "subtraction", "multiplication", "division"]
    pub require_enclosure: bool,       // Require interval enclosure property
    pub precision_mode: String,        // "affine" or "interval"
}

// ── Opcode Generation Functions ───────────────────────────────────────────────

impl EpistemicConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![
            SlgOpcode::CheckMaxInclusive(self.max_certainty as f64),
            SlgOpcode::CheckMaxInclusive(self.max_nesting_depth as f64),
            SlgOpcode::CheckMaxInclusive(self.max_agent_contexts as f64),
        ]
    }
}

impl ParaconsistentConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![
            SlgOpcode::CheckMaxInclusive(self.max_isolation_severity as f64),
            SlgOpcode::CheckMaxInclusive(self.isolation_buffer_size as f64),
            SlgOpcode::CheckMaxInclusive(self.merge_threshold),
        ]
    }
}

impl LTLConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![
            SlgOpcode::CheckMaxInclusive(self.max_trace_length as f64),
            SlgOpcode::CheckMaxInclusive(self.max_formula_depth as f64),
        ]
    }
}

impl SpatioTemporalConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![
            SlgOpcode::CheckMaxInclusive(self.max_spatial_regions as f64),
            SlgOpcode::CheckMaxInclusive(self.max_temporal_intervals as f64),
            SlgOpcode::CheckMaxInclusive(self.max_region_complexity as f64),
        ]
    }
}

impl GraphConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![
            SlgOpcode::CheckMaxInclusive(self.max_nodes as f64),
            SlgOpcode::CheckMaxInclusive(self.max_edges as f64),
            SlgOpcode::CheckMaxInclusive(self.max_node_degree as f64),
        ]
    }
}

impl CalculusConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![
            SlgOpcode::CheckMaxInclusive(self.max_grid_points as f64),
            SlgOpcode::CheckMaxInclusive(self.max_integration_steps as f64),
        ]
    }
}

impl ODEConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![
            SlgOpcode::CheckMaxInclusive(self.max_ode_order as f64),
            SlgOpcode::CheckMaxInclusive(self.max_step_size),
            SlgOpcode::CheckMinInclusive(self.min_step_size),
        ]
    }
}

impl ArgumentationConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![
            SlgOpcode::CheckMaxInclusive(self.max_arguments as f64),
            SlgOpcode::CheckMaxInclusive(self.max_attack_depth as f64),
            SlgOpcode::CheckMaxInclusive(self.max_defense_depth as f64),
        ]
    }
}

impl DialecticalConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![
            SlgOpcode::CheckMaxInclusive(self.max_thesis_count as f64),
            SlgOpcode::CheckMaxInclusive(self.max_antithesis_count as f64),
            SlgOpcode::CheckMaxInclusive(self.max_synthesis_rounds as f64),
        ]
    }
}

impl ASPConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![
            SlgOpcode::CheckMaxInclusive(self.max_variables as f64),
            // SlgOpcode::CheckInclusive(),
            SlgOpcode::CheckMaxInclusive(self.max_answer_sets as f64),
        ]
    }
}

impl ProbabilisticConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![
            SlgOpcode::CheckMaxInclusive(self.max_random_variables as f64),
            SlgOpcode::CheckMaxInclusive(self.max_probability_precision as f64),
        ]
    }
}

impl DLConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![
            SlgOpcode::CheckMaxInclusive(self.max_predicates as f64),
            SlgOpcode::CheckMaxInclusive(self.max_clauses as f64),
            SlgOpcode::CheckMaxInclusive(self.max_herbrand_universe as f64),
        ]
    }
}

impl DiffusionConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![
            SlgOpcode::CheckMaxInclusive(self.max_particles as f64),
            SlgOpcode::CheckMaxInclusive(self.max_time_steps as f64),
        ]
    }
}

// Generic opcode generation for other constraint types
macro_rules! generate_simple_validation_opcodes {
    ($struct_name:ident, $field:ident) => {
        impl $struct_name {
            pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
                vec![SlgOpcode::CheckMaxInclusive(self.$field as f64)]
            }
        }
    };
}

generate_simple_validation_opcodes!(EpistemicQuery, min_certainty_threshold);
generate_simple_validation_opcodes!(ContradictionHandling, max_contradictions_per_context);
// TemporalTrace has a manual impl with more comprehensive opcodes
generate_simple_validation_opcodes!(AllenIntervalConfiguration, max_interval_overlap);
// SpatialRegionConfiguration has a manual impl with both min and max checks
generate_simple_validation_opcodes!(GraphAnalysisConfiguration, max_community_count);
generate_simple_validation_opcodes!(GraphAlgorithmConfiguration, max_iterations);
generate_simple_validation_opcodes!(TensorProvenanceConfiguration, max_tensor_rank);
generate_simple_validation_opcodes!(ArgumentEvaluationConfiguration, max_evaluation_steps);
generate_simple_validation_opcodes!(SynthesisConfiguration, max_synthesis_depth);
generate_simple_validation_opcodes!(StableModelConfiguration, max_model_complexity);
// BayesianInferenceConfiguration has a manual impl with burn-in tracking
generate_simple_validation_opcodes!(DLQueryConfiguration, max_result_limit);
generate_simple_validation_opcodes!(DiffusionGridConfiguration, max_grid_size_per_dim);
generate_simple_validation_opcodes!(LinearLogicConfiguration, max_literals);
generate_simple_validation_opcodes!(ControlFeedbackConfiguration, max_feedback_loops);
// FeedbackGainConfiguration has a manual impl with comprehensive opcodes
generate_simple_validation_opcodes!(IntervalArithmeticConfiguration, max_interval_width);

// Complex opcode generation for range validations
impl TemporalTrace {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![
            SlgOpcode::CheckMinInclusive(self.min_trace_length as f64),
            SlgOpcode::CheckMaxInclusive(self.max_trace_length as f64),
        ]
    }
}

impl SpatialRegionConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![
            SlgOpcode::CheckMinInclusive(self.min_region_area),
            SlgOpcode::CheckMaxInclusive(self.max_region_area),
        ]
    }
}

impl BayesianInferenceConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![
            SlgOpcode::CheckMaxInclusive(self.burn_in_samples as f64),
            SlgOpcode::CheckMaxInclusive(self.post_burn_in_samples as f64),
        ]
    }
}

impl FeedbackGainConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![
            SlgOpcode::CheckMaxInclusive(self.max_proportional_gain),
            SlgOpcode::CheckMaxInclusive(self.max_integral_gain),
            // SlgOpcode::CheckInclusive(), // Minimum gain is 0
        ]
    }
}

// ── SHACL TTL Vocabulary for Core Modalities ─────────────────────────────────────

/// Returns comprehensive SHACL TTL vocabulary for all core modalities
pub fn get_core_modalities_shacl_ttl() -> &'static str {
    r#"
@prefix q42: <https://qualia.network/q42#> .
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

# ── Epistemic Logic Constraints ─────────────────────────────────────────────

q42:EpistemicConfigurationShape a sh:NodeShape ;
    sh:property [
        sh:path q42:maxCertainty ;
        sh:datatype xsd:unsignedByte ;
        sh:maxInclusive 255 ;
        sh:message "Certainty must be between 0 and 255" ;
    ] ;
    sh:property [
        sh:path q42:maxNestingDepth ;
        sh:datatype xsd:unsignedByte ;
        sh:maxInclusive 10 ;
        sh:message "Nesting depth must be ≤ 10" ;
    ] ;
    sh:property [
        sh:path q42:maxAgentContexts ;
        sh:datatype xsd:integer ;
        sh:minInclusive 1 ;
        sh:maxInclusive 1000 ;
        sh:message "Agent contexts must be between 1 and 1000" ;
    ] .

q42:EpistemicQueryShape a sh:NodeShape ;
    sh:property [
        sh:path q42:queryType ;
        sh:in ("knows" "believes" "common_knowledge") ;
        sh:message "Query type must be a valid epistemic operator" ;
    ] ;
    sh:property [
        sh:path q42:minCertaintyThreshold ;
        sh:datatype xsd:unsignedByte ;
        sh:maxInclusive 255 ;
        sh:message "Certainty threshold must be ≤ 255" ;
    ] .

# ── Paraconsistent Logic Constraints ─────────────────────────────────────────

q42:ParaconsistentConfigurationShape a sh:NodeShape ;
    sh:property [
        sh:path q42:maxIsolationSeverity ;
        sh:datatype xsd:unsignedByte ;
        sh:maxInclusive 255 ;
        sh:message "Isolation severity must be ≤ 255" ;
    ] ;
    sh:property [
        sh:path q42:mergeThreshold ;
        sh:datatype xsd:float ;
        sh:minInclusive 0.0 ;
        sh:maxInclusive 1.0 ;
        sh:message "Merge threshold must be between 0.0 and 1.0" ;
    ] .

# ── Temporal LTL Constraints ───────────────────────────────────────────────────

q42:LTLConfigurationShape a sh:NodeShape ;
    sh:property [
        sh:path q42:maxTraceLength ;
        sh:datatype xsd:integer ;
        sh:minInclusive 1 ;
        sh:maxInclusive 1000000 ;
        sh:message "Trace length must be between 1 and 1,000,000" ;
    ] ;
    sh:property [
        sh:path q42:allowedOperators ;
        sh:in ("G" "F" "X" "U" "R") ;
        sh:message "LTL operator must be valid" ;
    ] .

# ── Spatio-Temporal Constraints ─────────────────────────────────────────────

q42:SpatioTemporalConfigurationShape a sh:NodeShape ;
    sh:property [
        sh:path q42:maxSpatialRegions ;
        sh:datatype xsd:integer ;
        sh:minInclusive 1 ;
        sh:maxInclusive 1000 ;
        sh:message "Spatial regions must be between 1 and 1000" ;
    ] ;
    sh:property [
        sh:path q42:allowedSpatialRelations ;
        sh:in ("disconnected" "externally_connected" "partially_overlapping" "tangentially_proper_part") ;
        sh:message "Spatial relation must be valid RCC8 relation" ;
    ] .

# ── Graph Theory Constraints ─────────────────────────────────────────────────

q42:GraphConfigurationShape a sh:NodeShape ;
    sh:property [
        sh:path q42:maxNodes ;
        sh:datatype xsd:integer ;
        sh:minInclusive 1 ;
        sh:maxInclusive 10000000 ;
        sh:message "Graph nodes must be between 1 and 10 million" ;
    ] ;
    sh:property [
        sh:path q42:maxEdges ;
        sh:datatype xsd:integer ;
        sh:minInclusive 0 ;
        sh:maxInclusive 100000000 ;
        sh:message "Graph edges must be between 0 and 100 million" ;
    ] ;
    sh:property [
        sh:path q42:maxNodeDegree ;
        sh:datatype xsd:integer ;
        sh:minInclusive 1 ;
        sh:maxInclusive 100000 ;
        sh:message "Node degree must be between 1 and 100,000" ;
    ] .

# ── Calculus Constraints ─────────────────────────────────────────────────────

q42:CalculusConfigurationShape a sh:NodeShape ;
    sh:property [
        sh:path q42:maxGridPoints ;
        sh:datatype xsd:integer ;
        sh:minInclusive 1 ;
        sh:maxInclusive 1000000000 ;
        sh:message "Grid points must be between 1 and 1 billion" ;
    ] ;
    sh:property [
        sh:path q42:maxIntegrationSteps ;
        sh:datatype xsd:integer ;
        sh:minInclusive 1 ;
        sh:maxInclusive 100000000 ;
        sh:message "Integration steps must be between 1 and 100 million" ;
    ] ;
    sh:property [
        sh:path q42:allowedIntegrators ;
        sh:in ("simpsons" "trapezoidal" "rk4" "adaptive") ;
        sh:message "Integrator must be supported" ;
    ] .

q42:ODEConfigurationShape a sh:NodeShape ;
    sh:property [
        sh:path q42:maxOdeOrder ;
        sh:datatype xsd:unsignedByte ;
        sh:maxInclusive 10 ;
        sh:message "ODE order must be ≤ 10" ;
    ] ;
    sh:property [
        sh:path q42:maxStepSize ;
        sh:datatype xsd:float ;
        sh:minInclusive 1e-15 ;
        sh:maxInclusive 1.0 ;
        sh:message "Step size must be reasonable" ;
    ] ;
    sh:property [
        sh:path q42:minStepSize ;
        sh:datatype xsd:float ;
        sh:minInclusive 1e-15 ;
        sh:maxInclusive 1.0 ;
        sh:message "Step size must be reasonable" ;
    ] .

# ── Argumentation Theory Constraints ───────────────────────────────────────

q42:ArgumentationConfigurationShape a sh:NodeShape ;
    sh:property [
        sh:path q42:maxArguments ;
        sh:datatype xsd:integer ;
        sh:minInclusive 1 ;
        sh:maxInclusive 1000 ;
        sh:message "Arguments must be between 1 and 1000" ;
    ] ;
    sh:property [
        sh:path q42:maxAttackDepth ;
        sh:datatype xsd:unsignedByte ;
        sh:maxInclusive 20 ;
        sh:message "Attack depth must be ≤ 20" ;
    ] ;
    sh:property [
        sh:path q42:maxDefenseDepth ;
        sh:datatype xsd:unsignedByte ;
        sh:maxInclusive 20 ;
        sh:message "Defense depth must be ≤ 20" ;
    ] .

# ── Dialectical Logic Constraints ─────────────────────────────────────────────

q42:DialecticalConfigurationShape a sh:NodeShape ;
    sh:property [
        sh:path q42:maxThesisCount ;
        sh:datatype xsd:integer ;
        sh:minInclusive 1 ;
        sh:maxInclusive 100 ;
        sh:message "Theses must be between 1 and 100" ;
    ] ;
    sh:property [
        sh:path q42:maxSynthesisRounds ;
        sh:datatype xsd:integer ;
        sh:minInclusive 1 ;
        sh:maxInclusive 1000 ;
        sh:message "Synthesis rounds must be between 1 and 1000" ;
    ] .

# ── ASP Constraints ─────────────────────────────────────────────────────────

q42:ASPConfigurationShape a sh:NodeShape ;
    sh:property [
        sh:path q42:maxVariables ;
        sh:datatype xsd:integer ;
        sh:minInclusive 1 ;
        sh:maxInclusive 100000 ;
        sh:message "Variables must be between 1 and 100,000" ;
    ] ;
    sh:property [
        sh:path q42:maxClauses ;
        sh:datatype xsd:integer ;
        sh:minInclusive 1 ;
        sh:maxInclusive 1000000 ;
        sh:message "Clauses must be between 1 and 1 million" ;
    ] .

# ── Probabilistic Reasoning Constraints ─────────────────────────────────────

q42:ProbabilisticConfigurationShape a sh:NodeShape ;
    sh:property [
        sh:path q42:maxRandomVariables ;
        sh:datatype xsd:integer ;
        sh:minInclusive 1 ;
        sh:maxInclusive 100000 ;
        sh:message "Random variables must be between 1 and 100,000" ;
    ] ;
    sh:property [
        sh:path q42:maxProbabilityPrecision ;
        sh:datatype xsd:unsignedByte ;
        sh:maxInclusive 64 ;
        sh:message "Probability precision must be ≤ 64 bits" ;
    ] .
"#
}
