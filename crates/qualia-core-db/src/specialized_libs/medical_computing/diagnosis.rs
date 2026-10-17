use super::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Clinical analyzer for medical data analysis
pub struct ClinicalAnalyzer {
    diagnostic_engine: DiagnosticEngine,
    risk_assessment: ClinicalRiskAssessment,
    treatment_planner: TreatmentPlanner,
    outcome_predictor: OutcomePredictor,
}

/// Diagnostic engine
pub struct DiagnosticEngine {
    diagnostic_algorithms: HashMap<String, DiagnosticAlgorithm>,
    symptom_analyzer: SymptomAnalyzer,
    lab_interpreter: LabInterpreter,
}

/// Diagnostic algorithms
#[derive(Debug, Clone)]
pub struct DiagnosticAlgorithm {
    pub algorithm_id: String,
    pub algorithm_name: String,
    pub algorithm_type: DiagnosticAlgorithmType,
    pub accuracy: f64,
}

/// Diagnostic algorithm types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DiagnosticAlgorithmType {
    RuleBased,
    MachineLearning,
    Bayesian,
    NeuralNetwork,
}

/// Symptom analyzer
pub struct SymptomAnalyzer {
    symptom_patterns: HashMap<String, SymptomPattern>,
    symptom_correlations: HashMap<String, SymptomCorrelation>,
}

/// Symptom patterns
#[derive(Debug, Clone)]
pub struct SymptomPattern {
    pub pattern_id: String,
    pub pattern_name: String,
    pub symptoms: Vec<String>,
    pub associated_conditions: Vec<String>,
}

/// Symptom correlations
#[derive(Debug, Clone)]
pub struct SymptomCorrelation {
    pub correlation_id: String,
    pub symptom1: String,
    pub symptom2: String,
    pub correlation_coefficient: f64,
}

/// Lab interpreter
pub struct LabInterpreter {
    reference_ranges: HashMap<String, ReferenceRange>,
    abnormality_detector: AbnormalityDetector,
}

/// Abnormality detector
#[derive(Debug, Clone)]
pub struct AbnormalityDetector {
    detection_algorithms: HashMap<String, DetectionAlgorithm>,
    severity_assessment: SeverityAssessment,
}

/// Severity assessment
#[derive(Debug, Clone)]
pub struct SeverityAssessment {
    assessment_criteria: HashMap<String, AssessmentCriterion>,
    scoring_system: ScoringSystem,
}

/// Assessment criteria
#[derive(Debug, Clone)]
pub struct AssessmentCriterion {
    pub criterion_id: String,
    pub criterion_name: String,
    pub weight: f64,
    pub threshold: f64,
}

/// Scoring system
#[derive(Debug, Clone)]
pub struct ScoringSystem {
    pub system_id: String,
    pub system_name: String,
    pub scoring_algorithm: ScoringAlgorithm,
}

/// Scoring algorithms
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ScoringAlgorithm {
    WeightedSum,
    Bayesian,
    FuzzyLogic,
    NeuralNetwork,
}

/// Clinical risk assessment
pub struct ClinicalRiskAssessment {
    risk_models: HashMap<String, ClinicalRiskModel>,
    risk_factors: HashMap<String, ClinicalRiskFactor>,
}

/// Clinical risk models
#[derive(Debug, Clone)]
pub struct ClinicalRiskModel {
    pub model_id: String,
    pub model_name: String,
    pub model_type: ClinicalRiskModelType,
    pub validation_results: ValidationResults,
}

/// Clinical risk model types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ClinicalRiskModelType {
    Cardiovascular,
    Cancer,
    Diabetes,
    Respiratory,
    Custom(String),
}

/// Validation results
#[derive(Debug, Clone)]
pub struct ValidationResults {
    pub accuracy: f64,
    pub sensitivity: f64,
    pub specificity: f64,
    pub auc: f64,
}

/// Clinical risk factors
#[derive(Debug, Clone)]
pub struct ClinicalRiskFactor {
    pub factor_id: String,
    pub factor_name: String,
    pub factor_category: FactorCategory,
    pub factor_weight: f64,
}

/// Factor categories
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FactorCategory {
    Demographic,
    Lifestyle,
    Medical,
    Genetic,
    Environmental,
}

/// Treatment planner
pub struct TreatmentPlanner {
    treatment_guidelines: HashMap<String, TreatmentGuideline>,
    decision_support: DecisionSupport,
}

/// Treatment guidelines
#[derive(Debug, Clone)]
pub struct TreatmentGuideline {
    pub guideline_id: String,
    pub guideline_name: String,
    pub guideline_type: GuidelineType,
    pub recommendations: Vec<TreatmentRecommendation>,
}

/// Guideline types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GuidelineType {
    Clinical,
    Protocol,
    StandardOfCare,
    BestPractice,
}

/// Treatment recommendations
#[derive(Debug, Clone)]
pub struct TreatmentRecommendation {
    pub recommendation_id: String,
    pub condition: String,
    pub treatment: String,
    pub evidence_level: EvidenceLevel,
    pub strength: RecommendationStrength,
}

/// Evidence levels
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EvidenceLevel {
    LevelA,
    LevelB,
    LevelC,
    ExpertOpinion,
}

/// Recommendation strength
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RecommendationStrength {
    Strong,
    Moderate,
    Weak,
    ExpertConsensus,
}

/// Decision support
pub struct DecisionSupport {
    decision_trees: HashMap<String, DecisionTree>,
    scoring_systems: HashMap<String, ScoringSystem>,
}

/// Decision trees
#[derive(Debug, Clone)]
pub struct DecisionTree {
    pub tree_id: String,
    pub tree_name: String,
    pub root_node: DecisionNode,
}

/// Decision nodes
#[derive(Debug, Clone)]
pub struct DecisionNode {
    pub node_id: String,
    pub node_type: NodeType,
    pub condition: Option<String>,
    pub threshold: Option<f64>,
    pub children: Vec<DecisionNode>,
    pub outcome: Option<String>,
}

/// Node types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NodeType {
    Root,
    Decision,
    Leaf,
}

/// Outcome predictor
pub struct OutcomePredictor {
    prediction_models: HashMap<String, PredictionModel>,
    outcome_metrics: HashMap<String, OutcomeMetric>,
}

/// Prediction models
#[derive(Debug, Clone)]
pub struct PredictionModel {
    pub model_id: String,
    pub model_name: String,
    pub model_type: PredictionModelType,
    pub performance_metrics: ModelPerformanceMetrics,
}

/// Prediction model types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PredictionModelType {
    Survival,
    Response,
    Recurrence,
    Complication,
}

/// Model performance metrics
#[derive(Debug, Clone)]
pub struct ModelPerformanceMetrics {
    pub accuracy: f64,
    pub precision: f64,
    pub recall: f64,
    pub f1_score: f64,
}

/// Outcome metrics
#[derive(Debug, Clone)]
pub struct OutcomeMetric {
    pub metric_id: String,
    pub metric_name: String,
    pub metric_type: OutcomeMetricType,
    pub measurement_method: MeasurementMethod,
}

/// Outcome metric types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OutcomeMetricType {
    Mortality,
    Morbidity,
    QualityOfLife,
    FunctionalStatus,
}

/// Measurement methods
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MeasurementMethod {
    Scale,
    Binary,
    Continuous,
    Categorical,
}
impl ClinicalAnalyzer {
    /// Create a `ClinicalAnalyzer` with default sub-components.
    pub fn new() -> Self {
        Self {
            diagnostic_engine: DiagnosticEngine::new(),
            risk_assessment: ClinicalRiskAssessment::new(),
            treatment_planner: TreatmentPlanner::new(),
            outcome_predictor: OutcomePredictor::new(),
        }
    }

    /// Initialise the `ClinicalAnalyzer` sub-components; propagates the domain error.
    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        self.diagnostic_engine.initialize()?;
        self.risk_assessment.initialize()?;
        self.treatment_planner.initialize()?;
        self.outcome_predictor.initialize()?;
        Ok(())
    }

    /// Patient-only convenience entry point. The real differential engine
    /// ([`Self::analyze_differential`]) requires a CALLER-SUPPLIED knowledge base
    /// (priors + per-finding likelihoods); this overload is handed no such KB, so it
    /// fails closed with `InsufficientData` rather than fabricating a diagnosis or a
    /// confidence. Supply findings + a knowledge base via `analyze_differential` to get
    /// a real, ranked, honestly-labeled proposal.
    pub fn analyze_data(
        &mut self,
        _patient: &Patient,
        _data_type: ClinicalDataType,
    ) -> Result<ClinicalAnalysis, MedicalError> {
        Err(MedicalError::InsufficientData(
            "clinical diagnostic analysis (analyze_clinical_data): no knowledge base was \
             supplied through the patient-only path. Use analyze_differential(findings, \
             knowledge_base) with a caller-supplied, non-authoritative KB to obtain a ranked \
             epistemic proposal. Refusing to emit a fabricated diagnosis or confidence."
                .to_string(),
        ))
    }

    /// Real transparent Bayesian differential over a **caller-supplied, non-authoritative**
    /// knowledge base. Delegates to [`super::analyze_differential`]. Returns a ranked
    /// epistemic proposal (never a diagnosis); the honest label lives in
    /// `DifferentialProposal::epistemic_status`.
    pub fn analyze_differential(
        &self,
        observed_findings: &[String],
        kb: &super::DiagnosticKnowledgeBase,
    ) -> Result<super::DifferentialProposal, MedicalError> {
        super::analyze_differential(observed_findings, kb)
    }
}

impl DiagnosticEngine {
    /// Create a `DiagnosticEngine` with default sub-components.
    pub fn new() -> Self {
        Self {
            diagnostic_algorithms: HashMap::new(),
            symptom_analyzer: SymptomAnalyzer::new(),
            lab_interpreter: LabInterpreter::new(),
        }
    }

    /// Initialise the `DiagnosticEngine` sub-components; propagates the domain error.
    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        Ok(())
    }

    /// Register `diagnostic_algorithm` in the `DiagnosticEngine` collection.
    pub fn add_diagnostic_algorithm(&mut self, algorithm: DiagnosticAlgorithm) {
        self.diagnostic_algorithms
            .insert(algorithm.algorithm_id.clone(), algorithm);
    }

    /// Borrow `diagnostic_algorithm` by id, or `None` when absent.
    pub fn get_diagnostic_algorithm(&self, algorithm_id: &str) -> Option<&DiagnosticAlgorithm> {
        self.diagnostic_algorithms.get(algorithm_id)
    }

    /// Borrow the `symptom_analyzer` component.
    pub fn symptom_analyzer(&self) -> &SymptomAnalyzer {
        &self.symptom_analyzer
    }

    /// Borrow the `lab_interpreter` component.
    pub fn lab_interpreter(&self) -> &LabInterpreter {
        &self.lab_interpreter
    }
}

impl SymptomAnalyzer {
    /// Create a `SymptomAnalyzer` with default sub-components.
    pub fn new() -> Self {
        Self {
            symptom_patterns: HashMap::new(),
            symptom_correlations: HashMap::new(),
        }
    }

    /// Register `symptom_pattern` in the `SymptomAnalyzer` collection.
    pub fn add_symptom_pattern(&mut self, pattern: SymptomPattern) {
        self.symptom_patterns
            .insert(pattern.pattern_id.clone(), pattern);
    }

    /// Borrow `symptom_pattern` by id, or `None` when absent.
    pub fn get_symptom_pattern(&self, pattern_id: &str) -> Option<&SymptomPattern> {
        self.symptom_patterns.get(pattern_id)
    }

    /// Register `symptom_correlation` in the `SymptomAnalyzer` collection.
    pub fn add_symptom_correlation(&mut self, correlation: SymptomCorrelation) {
        self.symptom_correlations
            .insert(correlation.correlation_id.clone(), correlation);
    }

    /// Borrow `symptom_correlation` by id, or `None` when absent.
    pub fn get_symptom_correlation(&self, correlation_id: &str) -> Option<&SymptomCorrelation> {
        self.symptom_correlations.get(correlation_id)
    }
}

impl LabInterpreter {
    /// Create a `LabInterpreter` with default sub-components.
    pub fn new() -> Self {
        Self {
            reference_ranges: HashMap::new(),
            abnormality_detector: AbnormalityDetector::new(),
        }
    }

    /// Register `reference_range` in the `LabInterpreter` collection.
    pub fn add_reference_range(&mut self, test_code: &str, range: ReferenceRange) {
        self.reference_ranges.insert(test_code.to_string(), range);
    }

    /// Borrow `reference_range` by id, or `None` when absent.
    pub fn get_reference_range(&self, test_code: &str) -> Option<&ReferenceRange> {
        self.reference_ranges.get(test_code)
    }

    /// Borrow the `abnormality_detector` component.
    pub fn abnormality_detector(&self) -> &AbnormalityDetector {
        &self.abnormality_detector
    }

    /// Interpret `result` against the reference ranges; returns a `ResultStatus`.
    pub fn interpret_result(&self, result: &LabResult) -> ResultStatus {
        if let Some(range) = self.reference_ranges.get(&result.test_code) {
            if result.value < range.minimum || result.value > range.maximum {
                ResultStatus::Abnormal
            } else {
                ResultStatus::Normal
            }
        } else {
            result.status.clone()
        }
    }
}

impl AbnormalityDetector {
    /// Create a `AbnormalityDetector` with default sub-components.
    pub fn new() -> Self {
        Self {
            detection_algorithms: HashMap::new(),
            severity_assessment: SeverityAssessment::new(),
        }
    }

    /// Register `detection_algorithm` in the `AbnormalityDetector` collection.
    pub fn add_detection_algorithm(&mut self, algorithm: DetectionAlgorithm) {
        self.detection_algorithms
            .insert(algorithm.algorithm_id.clone(), algorithm);
    }

    /// Borrow `detection_algorithm` by id, or `None` when absent.
    pub fn get_detection_algorithm(&self, algorithm_id: &str) -> Option<&DetectionAlgorithm> {
        self.detection_algorithms.get(algorithm_id)
    }

    /// Borrow the `severity_assessment` component.
    pub fn severity_assessment(&self) -> &SeverityAssessment {
        &self.severity_assessment
    }
}

impl SeverityAssessment {
    /// Create a `SeverityAssessment` with default sub-components.
    pub fn new() -> Self {
        Self {
            assessment_criteria: HashMap::new(),
            scoring_system: ScoringSystem::new(),
        }
    }

    /// Register `criterion` in the `SeverityAssessment` collection.
    pub fn add_criterion(&mut self, criterion: AssessmentCriterion) {
        self.assessment_criteria
            .insert(criterion.criterion_id.clone(), criterion);
    }

    /// Borrow `criterion` by id, or `None` when absent.
    pub fn get_criterion(&self, criterion_id: &str) -> Option<&AssessmentCriterion> {
        self.assessment_criteria.get(criterion_id)
    }

    /// Borrow the `scoring_system` component.
    pub fn scoring_system(&self) -> &ScoringSystem {
        &self.scoring_system
    }
}

impl ScoringSystem {
    /// Create a `ScoringSystem` with default sub-components.
    pub fn new() -> Self {
        Self {
            system_id: "system_1".to_string(),
            system_name: "Clinical Scoring System".to_string(),
            scoring_algorithm: ScoringAlgorithm::WeightedSum,
        }
    }
}

impl ClinicalRiskAssessment {
    /// Create a `ClinicalRiskAssessment` with default sub-components.
    pub fn new() -> Self {
        Self {
            risk_models: HashMap::new(),
            risk_factors: HashMap::new(),
        }
    }

    /// Initialise the `ClinicalRiskAssessment` sub-components; propagates the domain error.
    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        Ok(())
    }

    /// Register `risk_model` in the `ClinicalRiskAssessment` collection.
    pub fn add_risk_model(&mut self, model: ClinicalRiskModel) {
        self.risk_models.insert(model.model_id.clone(), model);
    }

    /// Borrow `risk_model` by id, or `None` when absent.
    pub fn get_risk_model(&self, model_id: &str) -> Option<&ClinicalRiskModel> {
        self.risk_models.get(model_id)
    }

    /// Register `risk_factor` in the `ClinicalRiskAssessment` collection.
    pub fn add_risk_factor(&mut self, factor: ClinicalRiskFactor) {
        self.risk_factors.insert(factor.factor_id.clone(), factor);
    }

    /// Borrow `risk_factor` by id, or `None` when absent.
    pub fn get_risk_factor(&self, factor_id: &str) -> Option<&ClinicalRiskFactor> {
        self.risk_factors.get(factor_id)
    }

    /// Borrow the `compute_risk_score` component.
    pub fn compute_risk_score(&self, factor_ids: &[String]) -> f64 {
        if factor_ids.is_empty() {
            return 0.0;
        }
        let total_weight: f64 = factor_ids
            .iter()
            .filter_map(|id| self.risk_factors.get(id))
            .map(|f| f.factor_weight)
            .sum();
        total_weight / factor_ids.len() as f64
    }
}

impl TreatmentPlanner {
    /// Create a `TreatmentPlanner` with default sub-components.
    pub fn new() -> Self {
        Self {
            treatment_guidelines: HashMap::new(),
            decision_support: DecisionSupport::new(),
        }
    }

    /// Initialise the `TreatmentPlanner` sub-components; propagates the domain error.
    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        Ok(())
    }

    /// Register `treatment_guideline` in the `TreatmentPlanner` collection.
    pub fn add_treatment_guideline(&mut self, guideline: TreatmentGuideline) {
        self.treatment_guidelines
            .insert(guideline.guideline_id.clone(), guideline);
    }

    /// Borrow `treatment_guideline` by id, or `None` when absent.
    pub fn get_treatment_guideline(&self, guideline_id: &str) -> Option<&TreatmentGuideline> {
        self.treatment_guidelines.get(guideline_id)
    }

    /// Borrow the `decision_support` component.
    pub fn decision_support(&self) -> &DecisionSupport {
        &self.decision_support
    }
}

impl DecisionSupport {
    /// Create a `DecisionSupport` with default sub-components.
    pub fn new() -> Self {
        Self {
            decision_trees: HashMap::new(),
            scoring_systems: HashMap::new(),
        }
    }

    /// Register `decision_tree` in the `DecisionSupport` collection.
    pub fn add_decision_tree(&mut self, tree: DecisionTree) {
        self.decision_trees.insert(tree.tree_id.clone(), tree);
    }

    /// Borrow `decision_tree` by id, or `None` when absent.
    pub fn get_decision_tree(&self, tree_id: &str) -> Option<&DecisionTree> {
        self.decision_trees.get(tree_id)
    }

    /// Register `scoring_system` in the `DecisionSupport` collection.
    pub fn add_scoring_system(&mut self, system: ScoringSystem) {
        self.scoring_systems
            .insert(system.system_id.clone(), system);
    }

    /// Borrow `scoring_system` by id, or `None` when absent.
    pub fn get_scoring_system(&self, system_id: &str) -> Option<&ScoringSystem> {
        self.scoring_systems.get(system_id)
    }
}

impl OutcomePredictor {
    /// Create a `OutcomePredictor` with default sub-components.
    pub fn new() -> Self {
        Self {
            prediction_models: HashMap::new(),
            outcome_metrics: HashMap::new(),
        }
    }

    /// Initialise the `OutcomePredictor` sub-components; propagates the domain error.
    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        Ok(())
    }

    /// Register `prediction_model` in the `OutcomePredictor` collection.
    pub fn add_prediction_model(&mut self, model_id: &str, model: PredictionModel) {
        self.prediction_models.insert(model_id.to_string(), model);
    }

    /// Borrow `prediction_model` by id, or `None` when absent.
    pub fn get_prediction_model(&self, model_id: &str) -> Option<&PredictionModel> {
        self.prediction_models.get(model_id)
    }

    /// Register `outcome_metric` in the `OutcomePredictor` collection.
    pub fn add_outcome_metric(&mut self, metric_id: &str, value: OutcomeMetric) {
        self.outcome_metrics.insert(metric_id.to_string(), value);
    }

    /// Borrow `outcome_metric` by id, or `None` when absent.
    pub fn get_outcome_metric(&self, metric_id: &str) -> Option<&OutcomeMetric> {
        self.outcome_metrics.get(metric_id)
    }
}
