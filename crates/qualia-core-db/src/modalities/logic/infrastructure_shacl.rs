//! SHACL Extensions for Domains, Obfuscation, and Solvers
//!
//! This module provides SHACL constraint extensions for:
//! - Domain-specific engines (biological, chemical, physical, financial, mathematical, geospatial)
//! - Data obfuscation and semantic stripping
//! - Zero-allocation solver library

use crate::webizen::SlgOpcode;

// ── Domain-Specific Constraints ───────────────────────────────────────────────

/// `q42:BiologicalDomainConfiguration` — validates biological domain parameters
#[derive(Debug, Clone)]
pub struct BiologicalDomainConfiguration {
    pub max_sequence_length: u32,      // Maximum DNA/RNA/protein sequence length
    pub max_gene_count: u32,            // Maximum number of genes in analysis
    pub allowed_sequence_types: Vec<String>, // ["dna", "rna", "protein"]
    pub require_quality_score: bool,    // Require sequence quality validation
}

/// `q42:ChemicalDomainConfiguration` — validates chemical domain parameters
#[derive(Debug, Clone)]
pub struct ChemicalDomainConfiguration {
    pub max_molecular_weight: f64,      // Maximum molecular weight (Da)
    pub max_atom_count: u32,            // Maximum atoms per molecule
    pub allowed_element_types: Vec<String>, // Periodic table symbols
    pub require_valence_validation: bool, // Require chemical valence checks
}

/// `q42:PhysicalDomainConfiguration` — validates physical domain parameters
#[derive(Debug, Clone)]
pub struct PhysicalDomainConfiguration {
    pub max_temperature_kelvin: f64,    // Maximum temperature (K)
    pub max_pressure_pascal: f64,       // Maximum pressure (Pa)
    pub allowed_energy_units: Vec<String>, // ["joule", "electron_volt", "calorie"]
    pub require_dimensional_consistency: bool, // Require unit consistency
}

/// `q42:FinancialDomainConfiguration` — validates financial domain parameters
#[derive(Debug, Clone)]
pub struct FinancialDomainConfiguration {
    pub max_transaction_value: f64,     // Maximum transaction value
    pub max_portfolio_size: f64,        // Maximum portfolio value
    pub allowed_currencies: Vec<String>, // Currency codes
    pub require_compliance_check: bool,  // Require regulatory compliance
}

/// `q42:MathematicalDomainConfiguration` — validates mathematical domain parameters
#[derive(Debug, Clone)]
pub struct MathematicalDomainConfiguration {
    pub max_precision_bits: u8,         // Maximum precision (bits)
    pub allowed_number_types: Vec<String>, // ["integer", "float", "complex", "rational"]
    pub max_expression_depth: u8,       // Maximum expression nesting depth
    pub require_well_formedness: bool,  // Require well-formed expressions
}

/// `q42:GeospatialDomainConfiguration` — validates geospatial domain parameters
#[derive(Debug, Clone)]
pub struct GeospatialDomainConfiguration {
    pub max_coordinate_precision: u8,   // Decimal places for coordinates
    pub allowed_coordinate_systems: Vec<String>, // ["wgs84", "utm", "mercator"]
    pub max_area_sq_km: f64,            // Maximum area in square kilometers
    pub require_valid_geometry: bool,    // Require valid geometric shapes
}

// ── Obfuscation Constraints ─────────────────────────────────────────────────

/// `q42:ObfuscationConfiguration` — validates obfuscation parameters
#[derive(Debug, Clone)]
pub struct ObfuscationConfiguration {
    pub max_obfuscation_depth: u8,      // Maximum obfuscation transformation depth
    pub allowed_obfuscation_methods: Vec<String>, // ["polynomial", "semantic", "domain"]
    pub require_reversibility: bool,    // Require reversible transformations
    pub min_entropy_bits: u8,           // Minimum entropy after obfuscation
}

/// `q42:PolynomialObfuscationConfiguration` — validates polynomial obfuscation parameters
#[derive(Debug, Clone)]
pub struct PolynomialObfuscationConfiguration {
    pub max_polynomial_degree: u8,      // Maximum polynomial degree
    pub allowed_coefficient_types: Vec<String>, // ["integer", "float", "rational"]
    pub require_irreducibility: bool,   // Require irreducible polynomials
}

/// `q42:SemanticStripperConfiguration` — validates semantic stripping parameters
#[derive(Debug, Clone)]
pub struct SemanticStripperConfiguration {
    pub max_context_depth: u8,          // Maximum context stripping depth
    pub allowed_context_types: Vec<String>, // ["clinical", "financial", "personal"]
    pub require_anonymity: bool,        // Require complete anonymity
}

/// `q42:DomainTransformerConfiguration` — validates domain transformation parameters
#[derive(Debug, Clone)]
pub struct DomainTransformerConfiguration {
    pub allowed_target_domains: Vec<String>, // Target transformation domains
    pub max_transformation_chain_length: u8,  // Maximum chain of transformations
    pub require_preservation: bool,     // Require mathematical structure preservation
}

/// `q42:HybridStateConfiguration` — validates hybrid state management parameters
#[derive(Debug, Clone)]
pub struct HybridStateConfiguration {
    pub max_state_size_bytes: u32,      // Maximum state size in bytes
    pub allowed_state_domains: Vec<String>, // ["classical", "quantum", "hybrid"]
    pub require_synchronization: bool,  // Require state synchronization
}

// ── Solver Constraints ─────────────────────────────────────────────────────

/// `q42:SolverConfiguration` — validates general solver parameters
#[derive(Debug, Clone)]
pub struct SolverConfiguration {
    pub max_iterations: u32,            // Maximum solver iterations
    pub convergence_tolerance: f64,     // Convergence threshold
    pub max_step_size: f64,             // Maximum step size
    pub min_step_size: f64,             // Minimum step size
    pub allowed_solver_types: Vec<String>, // ["calculus", "linear_algebra", "optimization"]
}

/// `q42:CalculusSolverConfiguration` — validates calculus solver parameters
#[derive(Debug, Clone)]
pub struct CalculusSolverConfiguration {
    pub max_ode_order: u8,             // Maximum ODE order
    pub allowed_integrators: Vec<String>, // ["runge_kutta", "simpsons", "shooting"]
    pub require_stability_check: bool,  // Require numerical stability check
}

/// `q42:LinearAlgebraSolverConfiguration` — validates linear algebra solver parameters
#[derive(Debug, Clone)]
pub struct LinearAlgebraSolverConfiguration {
    pub max_matrix_dimension: u16,      // Maximum matrix dimension
    pub allowed_decompositions: Vec<String>, // ["lu", "qr", "svd", "eigen"]
    pub require_condition_number_check: bool, // Require condition number validation
}

/// `q42:OptimizationSolverConfiguration` — validates optimization solver parameters
#[derive(Debug, Clone)]
pub struct OptimizationSolverConfiguration {
    pub max_variables: u32,            // Maximum number of variables
    pub allowed_algorithms: Vec<String>, // ["nelder_mead", "newton_raphson", "levenberg_marquardt"]
    pub require_convexity_check: bool,   // Require convexity validation
}

/// `q42:QuantumOptimizerConfiguration` — validates quantum optimizer parameters
#[derive(Debug, Clone)]
pub struct QuantumOptimizerConfiguration {
    pub max_qubits: u16,                // Maximum number of qubits
    pub allowed_algorithms: Vec<String>, // ["qaoa", "spsa", "variational"]
    pub require_error_correction: bool, // Require error correction
}

/// `q42:SymbolicLogicSolverConfiguration` — validates symbolic logic solver parameters
#[derive(Debug, Clone)]
pub struct SymbolicLogicSolverConfiguration {
    pub max_clause_count: u32,          // Maximum number of clauses
    pub max_variable_count: u32,       // Maximum number of variables
    pub allowed_logic_types: Vec<String>, // ["defeasible", "classical", "modal"]
}

// ── Geometric Algebra Constraints ─────────────────────────────────────────────

/// `q42:GeometricAlgebraConfiguration` — validates geometric algebra parameters
#[derive(Debug, Clone)]
pub struct GeometricAlgebraConfiguration {
    pub max_dimension: u8,              // Maximum geometric algebra dimension (3D, 4D, etc.)
    pub allowed_algebras: Vec<String>,  // ["pga", "cga", "conformal"]
    pub require_normalization: bool,    // Require multivector normalization
}

// ── Opcode Generation Functions ───────────────────────────────────────────────

impl BiologicalDomainConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![
            SlgOpcode::CheckMaxInclusive(self.max_sequence_length as f64),
            SlgOpcode::CheckMaxInclusive(self.max_gene_count as f64),
        ]
    }
}

impl ChemicalDomainConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![
            SlgOpcode::CheckMaxInclusive(self.max_molecular_weight),
            SlgOpcode::CheckMaxInclusive(self.max_atom_count as f64),
        ]
    }
}

impl PhysicalDomainConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![
            SlgOpcode::CheckMaxInclusive(self.max_temperature_kelvin),
            SlgOpcode::CheckMaxInclusive(self.max_pressure_pascal),
        ]
    }
}

impl FinancialDomainConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![
            SlgOpcode::CheckMaxInclusive(self.max_transaction_value),
            SlgOpcode::CheckMaxInclusive(self.max_portfolio_size),
        ]
    }
}

impl MathematicalDomainConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![
            SlgOpcode::CheckMaxInclusive(self.max_precision_bits as f64),
            SlgOpcode::CheckMaxInclusive(self.max_expression_depth as f64),
        ]
    }
}

impl GeospatialDomainConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![
            SlgOpcode::CheckMaxInclusive(self.max_coordinate_precision as f64),
            SlgOpcode::CheckMaxInclusive(self.max_area_sq_km),
        ]
    }
}

impl ObfuscationConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![
            SlgOpcode::CheckMaxInclusive(self.max_obfuscation_depth as f64),
            SlgOpcode::CheckMinInclusive(self.min_entropy_bits as f64),
        ]
    }
}

impl PolynomialObfuscationConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![SlgOpcode::CheckMaxInclusive(self.max_polynomial_degree as f64)]
    }
}

impl SemanticStripperConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![SlgOpcode::CheckMaxInclusive(self.max_context_depth as f64)]
    }
}

impl DomainTransformerConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![SlgOpcode::CheckMaxInclusive(self.max_transformation_chain_length as f64)]
    }
}

impl HybridStateConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![SlgOpcode::CheckMaxInclusive(self.max_state_size_bytes as f64)]
    }
}

impl SolverConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![
            SlgOpcode::CheckMaxInclusive(self.max_iterations as f64),
            SlgOpcode::CheckMaxInclusive(self.convergence_tolerance),
            SlgOpcode::CheckMaxInclusive(self.max_step_size),
            SlgOpcode::CheckMinInclusive(self.min_step_size),
        ]
    }
}

impl CalculusSolverConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![SlgOpcode::CheckMaxInclusive(self.max_ode_order as f64)]
    }
}

impl LinearAlgebraSolverConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![SlgOpcode::CheckMaxInclusive(self.max_matrix_dimension as f64)]
    }
}

impl OptimizationSolverConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![SlgOpcode::CheckMaxInclusive(self.max_variables as f64)]
    }
}

impl QuantumOptimizerConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![SlgOpcode::CheckMaxInclusive(self.max_qubits as f64)]
    }
}

impl SymbolicLogicSolverConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![
            SlgOpcode::CheckMaxInclusive(self.max_clause_count as f64),
            SlgOpcode::CheckMaxInclusive(self.max_variable_count as f64),
        ]
    }
}

impl GeometricAlgebraConfiguration {
    pub fn to_opcodes(&self) -> Vec<SlgOpcode> {
        vec![SlgOpcode::CheckMaxInclusive(self.max_dimension as f64)]
    }
}

// ── SHACL TTL Vocabulary for Domains, Obfuscation, and Solvers ─────────────

/// Returns comprehensive SHACL TTL vocabulary for domains, obfuscation, and solvers
pub fn get_infrastructure_shacl_ttl() -> &'static str {
    r#"
@prefix q42: <https://qualia.network/q42#> .
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

# ── Domain-Specific Constraints ─────────────────────────────────────────────

q42:BiologicalDomainConfigurationShape a sh:NodeShape ;
    sh:property [
        sh:path q42:maxSequenceLength ;
        sh:datatype xsd:integer ;
        sh:minInclusive 1 ;
        sh:maxInclusive 1000000 ;
        sh:message "Sequence length must be between 1 and 1,000,000" ;
    ] ;
    sh:property [
        sh:path q42:allowedSequenceTypes ;
        sh:in ("dna" "rna" "protein") ;
        sh:message "Sequence type must be valid" ;
    ] .

q42:ChemicalDomainConfigurationShape a sh:NodeShape ;
    sh:property [
        sh:path q42:maxMolecularWeight ;
        sh:datatype xsd:float ;
        sh:minInclusive 1.0 ;
        sh:maxInclusive 1000000.0 ;
        sh:message "Molecular weight must be between 1 and 1,000,000 Da" ;
    ] ;
    sh:property [
        sh:path q42:maxAtomCount ;
        sh:datatype xsd:integer ;
        sh:minInclusive 1 ;
        sh:maxInclusive 10000 ;
        sh:message "Atom count must be between 1 and 10,000" ;
    ] .

q42:PhysicalDomainConfigurationShape a sh:NodeShape ;
    sh:property [
        sh:path q42:maxTemperatureKelvin ;
        sh:datatype xsd:float ;
        sh:minInclusive 0.0 ;
        sh:maxInclusive 1e6 ;
        sh:message "Temperature must be between 0K and 1,000,000K" ;
    ] ;
    sh:property [
        sh:path q42:maxPressurePascal ;
        sh:datatype xsd:float ;
        sh:minInclusive 0.0 ;
        sh:maxInclusive 1e12 ;
        sh:message "Pressure must be reasonable" ;
    ] .

q42:FinancialDomainConfigurationShape a sh:NodeShape ;
    sh:property [
        sh:path q42:maxTransactionValue ;
        sh:datatype xsd:float ;
        sh:minInclusive 0.0 ;
        sh:maxInclusive 1e15 ;
        sh:message "Transaction value must be reasonable" ;
    ] ;
    sh:property [
        sh:path q42:allowedCurrencies ;
        sh:message "Currency must be valid ISO 4217 code" ;
    ] .

q42:MathematicalDomainConfigurationShape a sh:NodeShape ;
    sh:property [
        sh:path q42:maxPrecisionBits ;
        sh:datatype xsd:unsignedByte ;
        sh:maxInclusive 128 ;
        sh:message "Precision must be ≤ 128 bits" ;
    ] ;
    sh:property [
        sh:path q42:maxExpressionDepth ;
        sh:datatype xsd:unsignedByte ;
        sh:maxInclusive 50 ;
        sh:message "Expression depth must be ≤ 50" ;
    ] .

q42:GeospatialDomainConfigurationShape a sh:NodeShape ;
    sh:property [
        sh:path q42:maxCoordinatePrecision ;
        sh:datatype xsd:unsignedByte ;
        sh:maxInclusive 15 ;
        sh:message "Coordinate precision must be ≤ 15 decimal places" ;
    ] ;
    sh:property [
        sh:path q42:allowedCoordinateSystems ;
        sh:in ("wgs84" "utm" "mercator" "geographic") ;
        sh:message "Coordinate system must be valid" ;
    ] .

# ── Obfuscation Constraints ─────────────────────────────────────────

q42:ObfuscationConfigurationShape a sh:NodeShape ;
    sh:property [
        sh:path q42:maxObfuscationDepth ;
        sh:datatype xsd:unsignedByte ;
        sh:maxInclusive 10 ;
        sh:message "Obfuscation depth must be ≤ 10" ;
    ] ;
    sh:property [
        sh:path q42:minEntropyBits ;
        sh:datatype xsd:unsignedByte ;
        sh:minInclusive 64 ;
        sh:message "Entropy must be ≥ 64 bits" ;
    ] .

q42:PolynomialObfuscationConfigurationShape a sh:NodeShape ;
    sh:property [
        sh:path q42:maxPolynomialDegree ;
        sh:datatype xsd:unsignedByte ;
        sh:maxInclusive 20 ;
        sh:message "Polynomial degree must be ≤ 20" ;
    ] .

q42:SemanticStripperConfigurationShape a sh:NodeShape ;
    sh:property [
        sh:path q42:maxContextDepth ;
        sh:datatype xsd:unsignedByte ;
        sh:maxInclusive 10 ;
        sh:message "Context depth must be ≤ 10" ;
    ] ;
    sh:property [
        sh:path q42:allowedContextTypes ;
        sh:in ("clinical" "financial" "personal" "legal") ;
        sh:message "Context type must be valid" ;
    ] .

q42:HybridStateConfigurationShape a sh:NodeShape ;
    sh:property [
        sh:path q42:maxStateSizeBytes ;
        sh:datatype xsd:integer ;
        sh:minInclusive 1 ;
        sh:maxInclusive 1048576 ;
        sh:message "State size must be between 1 and 1MB" ;
    ] .

# ── Solver Constraints ─────────────────────────────────────────────────

q42:SolverConfigurationShape a sh:NodeShape ;
    sh:property [
        sh:path q42:maxIterations ;
        sh:datatype xsd:integer ;
        sh:minInclusive 1 ;
        sh:maxInclusive 1000000 ;
        sh:message "Iterations must be between 1 and 1,000,000" ;
    ] ;
    sh:property [
        sh:path q42:convergenceTolerance ;
        sh:datatype xsd:float ;
        sh:minInclusive 1e-15 ;
        sh:maxInclusive 1.0 ;
        sh:message "Tolerance must be reasonable" ;
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

q42:CalculusSolverConfigurationShape a sh:NodeShape ;
    sh:property [
        sh:path q42:maxOdeOrder ;
        sh:datatype xsd:unsignedByte ;
        sh:maxInclusive 10 ;
        sh:message "ODE order must be ≤ 10" ;
    ] .

q42:LinearAlgebraSolverConfigurationShape a sh:NodeShape ;
    sh:property [
        sh:path q42:maxMatrixDimension ;
        sh:datatype xsd:unsignedShort ;
        sh:maxInclusive 1000 ;
        sh:message "Matrix dimension must be ≤ 1000" ;
    ] .

q42:OptimizationSolverConfigurationShape a sh:NodeShape ;
    sh:property [
        sh:path q42:maxVariables ;
        sh:datatype xsd:integer ;
        sh:minInclusive 1 ;
        sh:maxInclusive 100000 ;
        sh:message "Variables must be between 1 and 100,000" ;
    ] .

q42:QuantumOptimizerConfigurationShape a sh:NodeShape ;
    sh:property [
        sh:path q42:maxQubits ;
        sh:datatype xsd:unsignedShort ;
        sh:maxInclusive 1000 ;
        sh:message "Qubits must be ≤ 1000" ;
    ] .

q42:SymbolicLogicSolverConfigurationShape a sh:NodeShape ;
    sh:property [
        sh:path q42:maxClauseCount ;
        sh:datatype xsd:integer ;
        sh:minInclusive 1 ;
        sh:maxInclusive 1000000 ;
        sh:message "Clauses must be between 1 and 1,000,000" ;
    ] ;
    sh:property [
        sh:path q42:maxVariableCount ;
        sh:datatype xsd:integer ;
        sh:minInclusive 1 ;
        sh:maxInclusive 100000 ;
        sh:message "Variables must be between 1 and 100,000" ;
    ] .

# ── Geometric Algebra Constraints ─────────────────────────────────────────

q42:GeometricAlgebraConfigurationShape a sh:NodeShape ;
    sh:property [
        sh:path q42:maxDimension ;
        sh:datatype xsd:unsignedByte ;
        sh:minInclusive 2 ;
        sh:maxInclusive 8 ;
        sh:message "Dimension must be between 2 and 8" ;
    ] ;
    sh:property [
        sh:path q42:allowedAlgebras ;
        sh:in ("pga" "cga" "conformal" "projective") ;
        sh:message "Algebra type must be valid" ;
    ] .
"#
}