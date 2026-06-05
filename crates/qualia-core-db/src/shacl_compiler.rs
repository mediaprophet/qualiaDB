//! SHACL → Webizen Bytecode Compiler.
//!
//! Translates SHACL shape constraints into deterministic `SlgOpcode` sequences
//! that execute inside the Webizen SLG VM before data is committed to `.q42`.
//!
//! ## Typed API (preferred)
//! ```no_run
//! use qualia_core_db::shacl_compiler::{ShaclCompiler, ShaclConstraint, ShaclSeverity};
//! let compiler = ShaclCompiler::new();
//! let shape = compiler.compile(
//!     "fhir:Observation",
//!     "health:restingHeartRate",
//!     ShaclConstraint::MinInclusive(20.0),
//!     ShaclSeverity::Violation,
//! );
//! ```
//!
//! ## String API (backward compatible)
//! ```no_run
//! let opcodes = compiler.compile_shape("fhir:Observation", "health:restingHeartRate", "minInclusive", 20.0);
//! ```

use crate::webizen::SlgOpcode;

// ─── Severity ─────────────────────────────────────────────────────────────────

/// Maps to `sh:severity` in SHACL shapes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    Pam250   = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClinicalRiskModel {
    Framingham  = 0,
    Cha2ds2Vasc = 1,
    Score2      = 2,
    Ndis        = 3,
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
    // ── Standard SHACL shape composition ────────────────────────────────────
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
    // ── Qualia native: physics ───────────────────────────────────────────────
    ThermoMetropolisStep,
    SolveOdeDynamics,
    DftGroundState,
    PredictReceptorBinding,
    // ── Qualia native: biosciences ───────────────────────────────────────────
    /// Legacy form (routes to NucleotideAlignment).
    BioSequenceAlignment,
    AlignNucleotideSequence { gap_open: f32, gap_extend: f32 },
    AlignProteinSequence { matrix: ProteinScoringMatrix },
    ComputeKmerFrequency { k: u8 },
    ComputeMetaboliteSimilarity { min_tanimoto: f32 },
    ValidateFastaRecord,
    EvaluateGeneExpression { fold_change_threshold: f32 },
    // ── Qualia native: biomedical ────────────────────────────────────────────
    ComputeRiskScore { model: ClinicalRiskModel },
    EvaluateLongitudinalTrend { window_days: u32 },
    EvaluateDrugInteraction,
    CheckContraindication,
    ValidateFhirObservation { loinc_code: String },
    // ── Qualia native: economics ─────────────────────────────────────────────
    MonteCarloVar,
    SolveGeometricBrownianMotion,

    // ── Qualia native: organic chemistry ─────────────────────────────────────
    ValidateSmiles,
    ValidateInchi,
    /// Max allowed molecular weight in Da.
    ComputeMolecularWeight { max_da: f64 },
    /// Max allowed LogP value.
    ComputeLogP { max_logp: f64 },
    /// Max allowed TPSA in Å².
    ComputeTPSA { max_tpsa: f64 },
    EvaluateLipinski,
    EvaluateVeber,
    EvaluateGhose,
    EvaluateEgan,
    DetectFunctionalGroups,
    ComputePka,
    ComputeChiralCenters,
    GenerateCircularFingerprint { radius: u8 },
    ComputeArrheniusRate { temp_k: f64 },
    ComputeGibbsEnergy,
    ComputeEquilibrium,
    ComputeHendersonHasselbalch,
    ComputeAtomEconomy,
    ComputeEFactor,
    ComputeGreenMetrics,

    // ── Qualia native: deontic and epistemic ─────────────────────────────────
    DeonticObligate,
    DeonticPermit,
    DeonticForbid,
    DeonticNotExpired { now_unix: u32 },
    EpistemicKnowledge { min_certainty: u8 },
    EpistemicBelief { min_certainty: u8 },
    CommonKnowledge,
}

// ─── CompiledShape ────────────────────────────────────────────────────────────

/// A fully compiled shape ready to load into the SLG VM.
#[derive(Debug, Clone)]
pub struct CompiledShape {
    pub target_class: String,
    pub property_path: String,
    pub severity: ShaclSeverity,
    pub opcodes: Vec<SlgOpcode>,
}

impl CompiledShape {
    /// Returns true if every numeric range check passes for a given f64 value.
    pub fn evaluate_numeric(&self, value: f64) -> bool {
        for op in &self.opcodes {
            match op {
                SlgOpcode::CheckMinInclusive(min) if value < *min => return false,
                SlgOpcode::CheckMaxInclusive(max) if value > *max => return false,
                SlgOpcode::CheckMinExclusive(min) if value <= *min => return false,
                SlgOpcode::CheckMaxExclusive(max) if value >= *max => return false,
                _ => {}
            }
        }
        true
    }
}

// ─── ShaclCompiler ────────────────────────────────────────────────────────────

pub struct ShaclCompiler;

impl ShaclCompiler {
    pub fn new() -> Self { ShaclCompiler }

    /// Typed compile — preferred API.
    pub fn compile(
        &self,
        target_class: &str,
        property_path: &str,
        constraint: ShaclConstraint,
        severity: ShaclSeverity,
    ) -> CompiledShape {
        let mut opcodes = Vec::new();
        Self::push_constraint(&constraint, &mut opcodes);
        Self::push_terminal(severity, &mut opcodes);
        CompiledShape {
            target_class: target_class.to_string(),
            property_path: property_path.to_string(),
            severity,
            opcodes,
        }
    }

    /// Backward-compatible string-based API.  Now correctly threads `value` through.
    pub fn compile_shape(
        &self,
        target_class: &str,
        property_path: &str,
        constraint_type: &str,
        value: f32,
    ) -> Vec<SlgOpcode> {
        let constraint = Self::parse_str(constraint_type, value);
        let shape = self.compile(target_class, property_path, constraint, ShaclSeverity::Violation);
        shape.opcodes
    }

    /// Compiles a JSON-LD shape node into a QualiaQuin with embedded sensitivity byte.
    pub fn compile_json_node(&self, shape_json: &serde_json::Value) -> Result<crate::QualiaQuin, String> {
        let mut quin = crate::QualiaQuin::default();

        // ── Sensitivity label ────────────────────────────────────────────────
        let sensitivity = match shape_json.get("webizen:SensitivityLabel").and_then(|v| v.as_str()) {
            Some("Restricted") => crate::QualiaQuin::SENSITIVITY_RESTRICTED,
            Some("Classified")  => crate::QualiaQuin::SENSITIVITY_CLASSIFIED,
            _                   => crate::QualiaQuin::SENSITIVITY_PUBLIC,
        };
        quin.set_sensitivity_byte(sensitivity);

        // ── Subject / predicate / object hashes from @id / @type / value ────
        if let Some(id) = shape_json.get("@id").and_then(|v| v.as_str()) {
            quin.subject = crate::q_hash(id);
        }
        if let Some(typ) = shape_json.get("@type").and_then(|v| v.as_str()) {
            quin.predicate = crate::q_hash(typ);
        }
        if let Some(val) = shape_json.get("sh:targetClass").and_then(|v| v.as_str()) {
            quin.object = crate::q_hash(val);
        }

        // ── Routing lane from sh:severity ────────────────────────────────────
        if let Some(sev) = shape_json.get("sh:severity").and_then(|v| v.as_str()) {
            let lane_bits: u64 = match sev {
                "sh:Warning"   => 0b01,
                "sh:Violation" => 0b10,
                "sh:Info"      => 0b00,
                _              => 0b00,
            };
            quin.metadata |= lane_bits << 61;
        }

        // ── Lamport clock from optional timestamp ────────────────────────────
        if let Some(ts) = shape_json.get("qualia:timestamp").and_then(|v| v.as_u64()) {
            quin.set_lamport_clock((ts & 0x1FFF_FFFF) as u32);
        }

        Ok(quin)
    }

    /// Compiles a complete Turtle-style property shape map (e.g. from parsed biomarker-vocabulary.ttl).
    /// Returns one `CompiledShape` per SHACL constraint found in the map.
    pub fn compile_property_map(
        &self,
        target_class: &str,
        property_path: &str,
        constraints: &[(String, serde_json::Value)],
    ) -> Vec<CompiledShape> {
        let mut shapes = Vec::new();
        for (key, val) in constraints {
            let constraint_opt = match key.as_str() {
                "sh:minInclusive"  => val.as_f64().map(ShaclConstraint::MinInclusive),
                "sh:maxInclusive"  => val.as_f64().map(ShaclConstraint::MaxInclusive),
                "sh:minExclusive"  => val.as_f64().map(ShaclConstraint::MinExclusive),
                "sh:maxExclusive"  => val.as_f64().map(ShaclConstraint::MaxExclusive),
                "sh:minCount"      => val.as_u64().map(|n| ShaclConstraint::MinCount(n as u32)),
                "sh:maxCount"      => val.as_u64().map(|n| ShaclConstraint::MaxCount(n as u32)),
                "sh:minLength"     => val.as_u64().map(|n| ShaclConstraint::MinLength(n as u32)),
                "sh:maxLength"     => val.as_u64().map(|n| ShaclConstraint::MaxLength(n as u32)),
                "sh:pattern"       => val.as_str().map(|s| ShaclConstraint::Pattern(s.to_string())),
                "sh:hasValue"      => val.as_str().map(|s| ShaclConstraint::HasValue(s.to_string())),
                "sh:node"          => val.as_str().map(|s| ShaclConstraint::Node(s.to_string())),
                _ => None,
            };
            let severity = ShaclSeverity::Violation; // default; override per shape
            if let Some(c) = constraint_opt {
                shapes.push(self.compile(target_class, property_path, c, severity));
            }
        }
        shapes
    }

    // ── Internal helpers ─────────────────────────────────────────────────────

    fn push_constraint(c: &ShaclConstraint, ops: &mut Vec<SlgOpcode>) {
        match c {
            // Standard numeric
            ShaclConstraint::MinInclusive(v)   => ops.push(SlgOpcode::CheckMinInclusive(*v)),
            ShaclConstraint::MaxInclusive(v)   => ops.push(SlgOpcode::CheckMaxInclusive(*v)),
            ShaclConstraint::MinExclusive(v)   => ops.push(SlgOpcode::CheckMinExclusive(*v)),
            ShaclConstraint::MaxExclusive(v)   => ops.push(SlgOpcode::CheckMaxExclusive(*v)),
            // Cardinality
            ShaclConstraint::MinCount(n)       => ops.push(SlgOpcode::CheckMinCount(*n)),
            ShaclConstraint::MaxCount(n)       => ops.push(SlgOpcode::CheckMaxCount(*n)),
            // String
            ShaclConstraint::MinLength(n)      => ops.push(SlgOpcode::CheckMinLength(*n)),
            ShaclConstraint::MaxLength(n)      => ops.push(SlgOpcode::CheckMaxLength(*n)),
            ShaclConstraint::Pattern(re)       => ops.push(SlgOpcode::CheckPattern(crate::q_hash(re))),
            // Values
            ShaclConstraint::DataType(_)       => ops.push(SlgOpcode::Unify),
            ShaclConstraint::NodeKind(_)       => ops.push(SlgOpcode::CheckSubsumption),
            ShaclConstraint::HasValue(v)       => ops.push(SlgOpcode::CheckHasValue(crate::q_hash(v))),
            // Shape composition
            ShaclConstraint::Node(s)           => ops.push(SlgOpcode::CheckNodeShape(crate::q_hash(s))),
            ShaclConstraint::Not(s)            => ops.push(SlgOpcode::CheckNotShape(crate::q_hash(s))),
            ShaclConstraint::And(shapes) => {
                for s in shapes { ops.push(SlgOpcode::CheckNodeShape(crate::q_hash(s))); }
            }
            ShaclConstraint::Or(shapes) => {
                // OR: only the first match needed — emit CheckNodeShape for each; VM evaluates lazily
                for s in shapes { ops.push(SlgOpcode::CheckNodeShape(crate::q_hash(s))); }
            }
            ShaclConstraint::Xone(shapes) => {
                for s in shapes { ops.push(SlgOpcode::CheckNodeShape(crate::q_hash(s))); }
            }
            ShaclConstraint::In(values) => {
                // Emit one CheckHasValue per allowed value; VM passes if any match
                for v in values { ops.push(SlgOpcode::CheckHasValue(crate::q_hash(v))); }
            }
            // Physics
            ShaclConstraint::ThermoMetropolisStep     => ops.push(SlgOpcode::NativeThermodynamics),
            ShaclConstraint::SolveOdeDynamics         => ops.push(SlgOpcode::NativeOdeSolver),
            ShaclConstraint::DftGroundState           => ops.push(SlgOpcode::NativeQuantumDft),
            ShaclConstraint::PredictReceptorBinding   => ops.push(SlgOpcode::NativeReceptorBinding),
            // Biosciences
            ShaclConstraint::BioSequenceAlignment
            | ShaclConstraint::AlignNucleotideSequence { .. } => ops.push(SlgOpcode::NativeNucleotideAlign),
            ShaclConstraint::AlignProteinSequence { matrix } => ops.push(SlgOpcode::NativeProteinAlign(*matrix as u8)),
            ShaclConstraint::ComputeKmerFrequency { k }  => ops.push(SlgOpcode::NativeKmerFrequency(*k)),
            ShaclConstraint::ComputeMetaboliteSimilarity { .. } => ops.push(SlgOpcode::NativeMetaboliteSimilarity),
            ShaclConstraint::ValidateFastaRecord         => ops.push(SlgOpcode::NativeFastaValidation),
            ShaclConstraint::EvaluateGeneExpression { .. } => ops.push(SlgOpcode::NativeGeneExpression),
            // Biomedical
            ShaclConstraint::ComputeRiskScore { model }  => ops.push(SlgOpcode::NativeClinicalRisk(*model as u8)),
            ShaclConstraint::EvaluateLongitudinalTrend { window_days } => ops.push(SlgOpcode::NativeLongitudinalTrend(*window_days)),
            ShaclConstraint::EvaluateDrugInteraction     => ops.push(SlgOpcode::NativeDrugInteraction),
            ShaclConstraint::CheckContraindication       => ops.push(SlgOpcode::NativeContraindication),
            ShaclConstraint::ValidateFhirObservation { loinc_code } => {
                ops.push(SlgOpcode::NativeFhirObservation(crate::q_hash(loinc_code)));
            }
            // Economics
            ShaclConstraint::MonteCarloVar
            | ShaclConstraint::SolveGeometricBrownianMotion => ops.push(SlgOpcode::NativeEconomics),

            // Organic chemistry
            ShaclConstraint::ValidateSmiles                     => ops.push(SlgOpcode::NativeSmilesValidation),
            ShaclConstraint::ValidateInchi                      => ops.push(SlgOpcode::NativeInchiValidation),
            ShaclConstraint::ComputeMolecularWeight { max_da }  => ops.push(SlgOpcode::NativeMolecularWeight(max_da.to_bits())),
            ShaclConstraint::ComputeLogP { max_logp }           => ops.push(SlgOpcode::NativeLogP((max_logp * 100.0) as u32)),
            ShaclConstraint::ComputeTPSA { max_tpsa }           => ops.push(SlgOpcode::NativeTPSA(*max_tpsa as u32)),
            ShaclConstraint::EvaluateLipinski                   => ops.push(SlgOpcode::NativeLipinskiFilter),
            ShaclConstraint::EvaluateVeber                      => ops.push(SlgOpcode::NativeVeberFilter),
            ShaclConstraint::EvaluateGhose                      => ops.push(SlgOpcode::NativeGhoseFilter),
            ShaclConstraint::EvaluateEgan                       => ops.push(SlgOpcode::NativeEganFilter),
            ShaclConstraint::DetectFunctionalGroups             => ops.push(SlgOpcode::NativeFunctionalGroups),
            ShaclConstraint::ComputePka                         => ops.push(SlgOpcode::NativePkaEstimate),
            ShaclConstraint::ComputeChiralCenters               => ops.push(SlgOpcode::NativeChiralCenters),
            ShaclConstraint::GenerateCircularFingerprint { radius } => ops.push(SlgOpcode::NativeCircularFingerprint(*radius)),
            ShaclConstraint::ComputeArrheniusRate { temp_k }    => ops.push(SlgOpcode::NativeArrhenius(*temp_k as u32)),
            ShaclConstraint::ComputeGibbsEnergy                 => ops.push(SlgOpcode::NativeGibbsEnergy),
            ShaclConstraint::ComputeEquilibrium                 => ops.push(SlgOpcode::NativeEquilibrium),
            ShaclConstraint::ComputeHendersonHasselbalch        => ops.push(SlgOpcode::NativeHendersonHasselbalch),
            ShaclConstraint::ComputeAtomEconomy                 => ops.push(SlgOpcode::NativeAtomEconomy),
            ShaclConstraint::ComputeEFactor                     => ops.push(SlgOpcode::NativeEFactor),
            ShaclConstraint::ComputeGreenMetrics                => ops.push(SlgOpcode::NativeGreenMetrics),

            // Deontic and Epistemic
            ShaclConstraint::DeonticObligate
            | ShaclConstraint::DeonticPermit
            | ShaclConstraint::DeonticForbid
            | ShaclConstraint::DeonticNotExpired { .. } => ops.push(SlgOpcode::NativeDeonticEval),
            ShaclConstraint::EpistemicKnowledge { min_certainty } => ops.push(SlgOpcode::NativeEpistemicEval(*min_certainty)),
            ShaclConstraint::EpistemicBelief { min_certainty } => ops.push(SlgOpcode::NativeEpistemicEval(*min_certainty)),
            ShaclConstraint::CommonKnowledge => ops.push(SlgOpcode::NativeEpistemicEval(0)),
        }
    }

    fn push_terminal(severity: ShaclSeverity, ops: &mut Vec<SlgOpcode>) {
        match severity {
            ShaclSeverity::Violation => ops.push(SlgOpcode::Halt),
            ShaclSeverity::Warning   => ops.push(SlgOpcode::WarnOnly),
            ShaclSeverity::Info      => {}
        }
    }

    /// Public entry point for the wasm_bridge — same as the internal parser.
    pub fn parse_constraint_pub(constraint_type: &str, value: f32) -> ShaclConstraint {
        Self::parse_str(constraint_type, value)
    }

    /// Parses a constraint from the legacy string + f32 API.
    fn parse_str(constraint_type: &str, value: f32) -> ShaclConstraint {
        match constraint_type {
            "minInclusive"  | "sh:minInclusive"  => ShaclConstraint::MinInclusive(value as f64),
            "maxInclusive"  | "sh:maxInclusive"  => ShaclConstraint::MaxInclusive(value as f64),
            "minExclusive"  | "sh:minExclusive"  => ShaclConstraint::MinExclusive(value as f64),
            "maxExclusive"  | "sh:maxExclusive"  => ShaclConstraint::MaxExclusive(value as f64),
            "minCount"      | "sh:minCount"      => ShaclConstraint::MinCount(value as u32),
            "maxCount"      | "sh:maxCount"      => ShaclConstraint::MaxCount(value as u32),
            "minLength"     | "sh:minLength"     => ShaclConstraint::MinLength(value as u32),
            "maxLength"     | "sh:maxLength"     => ShaclConstraint::MaxLength(value as u32),
            "datatype"      | "sh:datatype"      => ShaclConstraint::DataType(String::new()),
            "qualia:thermoMetropolisStep"         => ShaclConstraint::ThermoMetropolisStep,
            "qualia:solveOdeDynamics"             => ShaclConstraint::SolveOdeDynamics,
            "qualia:dftGroundState"               => ShaclConstraint::DftGroundState,
            "qualia:bioSequenceAlignment"         => ShaclConstraint::BioSequenceAlignment,
            "qualia:alignNucleotideSequence"      => ShaclConstraint::AlignNucleotideSequence { gap_open: value, gap_extend: value * 0.5 },
            "qualia:alignProteinSequence"         => ShaclConstraint::AlignProteinSequence { matrix: ProteinScoringMatrix::Blosum62 },
            "qualia:computeKmerFrequency"         => ShaclConstraint::ComputeKmerFrequency { k: value as u8 },
            "qualia:computeMetaboliteSimilarity"  => ShaclConstraint::ComputeMetaboliteSimilarity { min_tanimoto: value },
            "qualia:validateFastaRecord"          => ShaclConstraint::ValidateFastaRecord,
            "qualia:evaluateGeneExpression"       => ShaclConstraint::EvaluateGeneExpression { fold_change_threshold: value },
            "qualia:predictReceptorBinding"       => ShaclConstraint::PredictReceptorBinding,
            "qualia:computeRiskScore:framingham"  => ShaclConstraint::ComputeRiskScore { model: ClinicalRiskModel::Framingham },
            "qualia:computeRiskScore:cha2ds2"     => ShaclConstraint::ComputeRiskScore { model: ClinicalRiskModel::Cha2ds2Vasc },
            "qualia:computeRiskScore:score2"      => ShaclConstraint::ComputeRiskScore { model: ClinicalRiskModel::Score2 },
            "qualia:evaluateLongitudinalTrend"    => ShaclConstraint::EvaluateLongitudinalTrend { window_days: value as u32 },
            "qualia:evaluateDrugInteraction"      => ShaclConstraint::EvaluateDrugInteraction,
            "qualia:checkContraindication"        => ShaclConstraint::CheckContraindication,
            "qualia:validateFhirObservation"      => ShaclConstraint::ValidateFhirObservation { loinc_code: String::new() },
            "qualia:monteCarloVaR" | "qualia:solveGeometricBrownianMotion" => ShaclConstraint::MonteCarloVar,
            // Organic chemistry
            "qualia:validateSmiles"              => ShaclConstraint::ValidateSmiles,
            "qualia:validateInchi"               => ShaclConstraint::ValidateInchi,
            "qualia:computeMolecularWeight"      => ShaclConstraint::ComputeMolecularWeight { max_da: value as f64 },
            "qualia:computeLogP"                 => ShaclConstraint::ComputeLogP { max_logp: value as f64 },
            "qualia:computeTPSA"                 => ShaclConstraint::ComputeTPSA { max_tpsa: value as f64 },
            "qualia:evaluateLipinski"            => ShaclConstraint::EvaluateLipinski,
            "qualia:evaluateVeber"               => ShaclConstraint::EvaluateVeber,
            "qualia:evaluateGhose"               => ShaclConstraint::EvaluateGhose,
            "qualia:evaluateEgan"                => ShaclConstraint::EvaluateEgan,
            "qualia:detectFunctionalGroups"      => ShaclConstraint::DetectFunctionalGroups,
            "qualia:computePka"                  => ShaclConstraint::ComputePka,
            "qualia:computeChiralCenters"        => ShaclConstraint::ComputeChiralCenters,
            "qualia:generateCircularFingerprint" => ShaclConstraint::GenerateCircularFingerprint { radius: value as u8 },
            "qualia:computeArrheniusRate"        => ShaclConstraint::ComputeArrheniusRate { temp_k: value as f64 },
            "qualia:computeGibbsEnergy"          => ShaclConstraint::ComputeGibbsEnergy,
            "qualia:computeEquilibrium"          => ShaclConstraint::ComputeEquilibrium,
            "qualia:computeHendersonHasselbalch" => ShaclConstraint::ComputeHendersonHasselbalch,
            "qualia:computeAtomEconomy"          => ShaclConstraint::ComputeAtomEconomy,
            "qualia:computeEFactor"              => ShaclConstraint::ComputeEFactor,
            "qualia:computeGreenMetrics"         => ShaclConstraint::ComputeGreenMetrics,
            "qualia:deonticObligate"             => ShaclConstraint::DeonticObligate,
            "qualia:deonticPermit"               => ShaclConstraint::DeonticPermit,
            "qualia:deonticForbid"               => ShaclConstraint::DeonticForbid,
            "qualia:deonticNotExpired"           => ShaclConstraint::DeonticNotExpired { now_unix: value as u32 },
            "qualia:epistemicKnowledge"          => ShaclConstraint::EpistemicKnowledge { min_certainty: value as u8 },
            "qualia:epistemicBelief"             => ShaclConstraint::EpistemicBelief { min_certainty: value as u8 },
            "qualia:commonKnowledge"             => ShaclConstraint::CommonKnowledge,
            other => {
                eprintln!("[ShaclCompiler] unknown constraint: {other}");
                ShaclConstraint::DataType(other.to_string())
            }
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn compiler() -> ShaclCompiler { ShaclCompiler::new() }

    #[test]
    fn min_inclusive_passes_value_through() {
        let shape = compiler().compile("fhir:Observation", "health:heartRate", ShaclConstraint::MinInclusive(20.0), ShaclSeverity::Violation);
        assert!(shape.evaluate_numeric(60.0), "60 bpm should pass min 20");
        assert!(!shape.evaluate_numeric(10.0), "10 bpm should fail min 20");
    }

    #[test]
    fn max_inclusive_blocks_high_value() {
        let shape = compiler().compile("fhir:Observation", "health:heartRate", ShaclConstraint::MaxInclusive(300.0), ShaclSeverity::Violation);
        assert!(shape.evaluate_numeric(100.0));
        assert!(!shape.evaluate_numeric(301.0));
    }

    #[test]
    fn legacy_string_api_threads_value() {
        let ops = compiler().compile_shape("test:Shape", "test:prop", "minInclusive", 42.0);
        assert!(ops.contains(&SlgOpcode::CheckMinInclusive(42.0)));
    }

    #[test]
    fn severity_warning_emits_warn_only() {
        let shape = compiler().compile("a", "b", ShaclConstraint::MinInclusive(0.0), ShaclSeverity::Warning);
        assert!(shape.opcodes.contains(&SlgOpcode::WarnOnly));
        assert!(!shape.opcodes.contains(&SlgOpcode::Halt));
    }

    #[test]
    fn bioscience_constraints_produce_correct_opcodes() {
        let nc = compiler().compile("bio:Sequence", "bio:query", ShaclConstraint::AlignNucleotideSequence { gap_open: -11.0, gap_extend: -1.0 }, ShaclSeverity::Violation);
        assert!(nc.opcodes.contains(&SlgOpcode::NativeNucleotideAlign));

        let pc = compiler().compile("bio:Protein", "bio:seq", ShaclConstraint::AlignProteinSequence { matrix: ProteinScoringMatrix::Blosum62 }, ShaclSeverity::Violation);
        assert!(pc.opcodes.contains(&SlgOpcode::NativeProteinAlign(0)));
    }

    #[test]
    fn biomedical_risk_opcode_correct_model_id() {
        let s = compiler().compile("health:Patient", "health:cvdRisk", ShaclConstraint::ComputeRiskScore { model: ClinicalRiskModel::Cha2ds2Vasc }, ShaclSeverity::Warning);
        assert!(s.opcodes.contains(&SlgOpcode::NativeClinicalRisk(1)));
        assert!(s.opcodes.contains(&SlgOpcode::WarnOnly));
    }

    #[test]
    fn fhir_observation_opcode_encodes_loinc() {
        let s = compiler().compile("fhir:Obs", "fhir:value", ShaclConstraint::ValidateFhirObservation { loinc_code: "4548-4".into() }, ShaclSeverity::Violation);
        let expected_hash = crate::q_hash("4548-4");
        assert!(s.opcodes.contains(&SlgOpcode::NativeFhirObservation(expected_hash)));
    }

    #[test]
    fn compile_json_node_sensitivity_restricted() {
        let json = serde_json::json!({
            "@id": "https://example.org/shape1",
            "@type": "sh:NodeShape",
            "sh:targetClass": "fhir:Observation",
            "webizen:SensitivityLabel": "Restricted",
            "sh:severity": "sh:Warning",
        });
        let quin = compiler().compile_json_node(&json).unwrap();
        assert_eq!(quin.get_sensitivity_byte(), crate::QualiaQuin::SENSITIVITY_RESTRICTED);
    }
}
