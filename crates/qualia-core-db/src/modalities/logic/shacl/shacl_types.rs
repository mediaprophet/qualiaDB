//! SHACL Types and Enums
//!
//! This module contains the type definitions for SHACL validation,
//! including constraints, severity levels, and validation reports.

// ─── Severity ─────────────────────────────────────────────────────────────────

/// Maps to `sh:severity` in SHACL shapes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum ShaclSeverity {
    /// `sh:Violation` — halt ingestion, write rejection audit Quin.
    Violation,
    /// `sh:Warning` — emit diagnostic, continue ingestion.
    Warning,
    /// `sh:Info` — telemetry only, no terminal opcode.
    Info,
}

// ─── Scoring matrices / model IDs ────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProteinScoringMatrix {
    Blosum62 = 0,
    Blosum80 = 1,
    Pam250 = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClinicalRiskModel {
    Framingham = 0,
    Cha2ds2Vasc = 1,
    Score2 = 2,
    Ndis = 3,
}

// ─── Calculus modality ───────────────────────────────────────────────────────

/// Compute target for calculus operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalcComputeTarget {
    /// CPU-based Simpson's rule integration (OP_SIMPSONS_INTEGRATION)
    CpuSimpsons = 0,
    /// CPU-based trapezoidal rule integration (OP_TRAPEZOIDAL_INTEGRATION)
    CpuTrapezoidal = 1,
    /// GPU-accelerated integration via WebGPU (OP_GPU_INTEGRATION)
    Gpu = 2,
}

// ─── NodeKindType for strict node validation ───────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKindType {
    BlankNode,
    Iri,
    Literal,
    BlankNodeOrIri,
    BlankNodeOrLiteral,
    IriOrLiteral,
}

// ─── Property Path Expressions ─────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum PropertyPath {
    /// Simple property path: ex:name
    Predicate(String),
    /// Inverse property path: ^ex:parent
    Inverse(Box<PropertyPath>),
    /// Sequence path: ex:parent/ex:name
    Sequence(Vec<PropertyPath>),
    /// Alternative path: ex:father|ex:mother
    Alternative(Vec<PropertyPath>),
    /// Zero or more: ex:parent*
    ZeroOrMore(Box<PropertyPath>),
    /// One or more: ex:parent+
    OneOrMore(Box<PropertyPath>),
    /// Zero or one: ex:parent?
    ZeroOrOne(Box<PropertyPath>),
}

// ─── SHACL Target Selectors ─────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum ShaclTarget {
    /// Target class: sh:targetClass ex:Person
    TargetClass(String),
    /// Target objects of property: sh:targetObjectsOf ex:parent
    TargetObjectsOf(String),
    /// Target subjects of property: sh:targetSubjectsOf ex:child
    TargetSubjectsOf(String),
    /// Target node: sh:targetNode ex:john
    TargetNode(String),
}

// ─── Validation Report ───────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize)]
pub struct ValidationReport {
    pub conforms: bool,
    pub results: Vec<ValidationResult>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ValidationResult {
    pub severity: ShaclSeverity,
    pub focus_node: String,
    pub result_path: Option<String>,
    pub message: Option<String>,
    pub source_constraint: Option<String>,
    pub source_constraint_component: Option<String>,
    pub value: Option<String>,
}

// ─── ShaclConstraint (typed) ──────────────────────────────────────────────────

/// Full vocabulary of constraints the compiler understands.
/// Maps 1:1 onto SHACL shape properties + Qualia native extensions.
#[derive(Debug, Clone)]
pub enum ShaclConstraint {
    // ── Standard SHACL numeric ──────────────────────────────────────────────
    MinInclusive(f64),
    MaxInclusive(f64),
    MinExclusive(f64),
    MaxExclusive(f64),
    // ── Standard SHACL cardinality ──────────────────────────────────────────
    MinCount(u32),
    MaxCount(u32),
    // ── Standard SHACL string ───────────────────────────────────────────────
    MinLength(u32),
    MaxLength(u32),
    /// Regex pattern string. Stored as `q_hash` in the opcode.
    Pattern(String),
    // ── Standard SHACL value ────────────────────────────────────────────────
    DataType(String),
    NodeKind(String),
    /// Enumerate allowed values; generates one `CheckHasValue` per entry in OR logic.
    In(Vec<String>),
    HasValue(String),
    // ── Standard SHACL functions ───────────────────────────────────────────
    /// Compare two values for equality
    Equals(String),
    /// Compare two values for less than relationship
    LessThan(String),
    /// Compare two values for less than or equals relationship
    LessThanOrEquals(String),
    /// Compare two values for greater than relationship
    GreaterThan(String),
    /// Compare two values for greater than or equals relationship
    GreaterThanOrEquals(String),
    // ── Standard SHACL language and datatype ─────────────────────────────────
    /// Restrict to specific language tags
    LanguageIn(Vec<String>),
    /// Unique values constraint
    UniqueLang,
    /// Datatype-specific constraints
    DatatypeRange {
        min_inclusive: Option<f64>,
        max_inclusive: Option<f64>,
        min_exclusive: Option<f64>,
        max_exclusive: Option<f64>,
    },
    // ── Standard SHACL node validation ───────────────────────────────────────
    /// Validate node kind (blank node, IRI, literal)
    NodeKindStrict(NodeKindType),
    /// Validate class membership
    Class(String),
    /// Validate closed shape (no extra properties)
    Closed {
        ignored_properties: Vec<String>,
    },
    /// Reference to another node shape (generates `CheckNodeShape`).
    Node(String),
    /// All sub-shapes must pass.
    And(Vec<String>),
    /// At least one sub-shape must pass.
    Or(Vec<String>),
    /// Referenced shape must fail.
    Not(String),
    /// Exactly one sub-shape must pass.
    Xone(Vec<String>),
    // ── Standard SHACL property path constraints ─────────────────────────────
    /// Property path constraint with path expression
    PropertyPath {
        path: PropertyPath,
        constraint: Box<ShaclConstraint>,
    },
    /// Qualifier values for property paths
    QualifierValue {
        path: PropertyPath,
        value: String,
    },
    // ── Qualia native extensions ─────────────────────────────────────────────
    /// Deontic policy constraint (ODRL-based)
    DeonticPolicy {
        policy_id: String,
        obligation: String,
    },
    /// Epistemic logic constraint
    EpistemicConstraint {
        certainty_threshold: f32,
    },
    /// Temporal LTL constraint
    LtlConstraint {
        formula: String,
    },
    /// Paraconsistent logic constraint
    ParaconsistentConstraint {
        isolation_context: String,
    },
    /// Calculus constraint (numerical integration)
    CalculusConstraint {
        compute_target: CalcComputeTarget,
        tolerance: f64,
    },
    /// Graph theory constraint
    GraphConstraint {
        algorithm: String,
        parameter: f64,
    },
    /// Argumentation framework constraint
    ArgumentationConstraint {
        framework_type: String,
    },
    /// Dialectical logic constraint
    DialecticalConstraint {
        synthesis_type: String,
    },
    /// ASP constraint
    AspConstraint {
        stable_model_limit: u32,
    },
    /// Probabilistic constraint
    ProbabilisticConstraint {
        confidence_threshold: f32,
    },
    /// Diffusion constraint
    DiffusionConstraint {
        diffusion_rate: f32,
    },
    /// Linear logic constraint
    LinearLogicConstraint {
        resource_budget: u32,
    },
    /// Control theory constraint
    ControlFeedbackConstraint {
        feedback_gain: f32,
    },
    /// Interval reasoning constraint
    IntervalArithmeticConstraint {
        tolerance: f64,
    },
}

// ─── Compiled Shape ─────────────────────────────────────────────────────────

/// A compiled SHACL shape ready for bytecode generation.
#[derive(Debug, Clone)]
pub struct CompiledShape {
    pub shape_class: String,
    pub constraints: Vec<ShaclConstraint>,
    pub severity: ShaclSeverity,
    // Additional fields for backward compatibility
    pub property_path: String,
    pub opcodes: Vec<crate::webizen::SlgOpcode>,
    pub deactivated: bool,
    pub name: Option<String>,
}

impl CompiledShape {
    pub fn new(
        shape_class: String,
        constraints: Vec<ShaclConstraint>,
        severity: ShaclSeverity,
    ) -> Self {
        Self {
            shape_class,
            constraints,
            severity,
            property_path: String::new(),
            opcodes: Vec::new(),
            deactivated: false,
            name: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.constraints.is_empty()
    }

    /// Returns true if every numeric range check passes for a given f64 value.
    pub fn evaluate_numeric(&self, value: f64) -> bool {
        for op in &self.opcodes {
            match op {
                crate::webizen::SlgOpcode::CheckMinInclusive(min) if value < *min => return false,
                crate::webizen::SlgOpcode::CheckMaxInclusive(max) if value > *max => return false,
                crate::webizen::SlgOpcode::CheckMinExclusive(min) if value <= *min => return false,
                crate::webizen::SlgOpcode::CheckMaxExclusive(max) if value >= *max => return false,
                _ => {}
            }
        }
        true
    }
}
