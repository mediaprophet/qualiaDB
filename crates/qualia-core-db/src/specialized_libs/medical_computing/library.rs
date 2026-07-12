use super::*;

/// Medical Computing Library Manager
pub struct MedicalComputingLibrary {
    patient_manager: PatientManager,
    clinical_analyzer: ClinicalAnalyzer,
    medical_imaging: MedicalImaging,
    drug_discovery: DrugDiscovery,
    compliance_monitor: MedicalComplianceMonitor,
}
impl MedicalComputingLibrary {
    /// Create new medical computing library
    pub fn new() -> Self {
        Self {
            patient_manager: PatientManager::new(),
            clinical_analyzer: ClinicalAnalyzer::new(),
            medical_imaging: MedicalImaging::new(),
            drug_discovery: DrugDiscovery::new(),
            compliance_monitor: MedicalComplianceMonitor::new(),
        }
    }

    /// Initialize the library
    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        // Initialize patient manager
        self.patient_manager.initialize()?;

        // Initialize clinical analyzer
        self.clinical_analyzer.initialize()?;

        // Initialize medical imaging
        self.medical_imaging.initialize()?;

        // Initialize drug discovery
        self.drug_discovery.initialize()?;

        // Initialize compliance monitor
        self.compliance_monitor.initialize()?;

        // Seed default patient for testing
        let default_patient = Patient::new();
        let _ = self.patient_manager.create_patient(default_patient);

        Ok(())
    }

    /// Create a new patient record
    pub fn create_patient_record(
        &mut self,
        patient: Patient,
    ) -> Result<MedicalOperationResult<Patient>, MedicalError> {
        let start_time = std::time::Instant::now();

        // Validate patient data
        self.patient_manager.validate_patient(&patient)?;

        // Create patient record
        let created_patient = self.patient_manager.create_patient(patient)?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(MedicalOperationResult {
            result: created_patient,
            execution_time,
            privacy_score: None,
            compliance_status: ComplianceStatus::Compliant,
            audit_trail: Vec::new(),
        })
    }

    /// Analyze clinical data
    pub fn analyze_clinical_data(
        &mut self,
        patient_id: &str,
        data_type: ClinicalDataType,
    ) -> Result<MedicalOperationResult<ClinicalAnalysis>, MedicalError> {
        let start_time = std::time::Instant::now();

        // Get patient data
        let patient = self.patient_manager.get_patient(patient_id)?;

        // Analyze clinical data
        let analysis = self.clinical_analyzer.analyze_data(&patient, data_type)?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(MedicalOperationResult {
            result: analysis,
            execution_time,
            privacy_score: None,
            compliance_status: ComplianceStatus::Compliant,
            audit_trail: Vec::new(),
        })
    }

    /// Real transparent Bayesian differential over a caller-supplied, non-authoritative
    /// knowledge base. Returns a ranked epistemic proposal (never a diagnosis); the honest
    /// label lives in `DifferentialProposal::epistemic_status`.
    pub fn analyze_differential(
        &mut self,
        observed_findings: &[String],
        knowledge_base: &DiagnosticKnowledgeBase,
    ) -> Result<MedicalOperationResult<DifferentialProposal>, MedicalError> {
        let start_time = std::time::Instant::now();
        let proposal = self
            .clinical_analyzer
            .analyze_differential(observed_findings, knowledge_base)?;
        let execution_time = start_time.elapsed().as_millis() as u64;
        Ok(MedicalOperationResult {
            result: proposal,
            execution_time,
            privacy_score: None,
            compliance_status: ComplianceStatus::Compliant,
            audit_trail: Vec::new(),
        })
    }

    /// Real 2-D DSP over a caller-provided intensity grid (statistics, histogram,
    /// window/level, threshold segmentation, Sobel edge magnitude). The result is
    /// honestly labeled signal processing, never a diagnosis.
    pub fn analyze_medical_image_grid(
        &mut self,
        data: &[f64],
        width: usize,
        height: usize,
        bins: usize,
        threshold: SegmentationThreshold,
        window: Option<(f64, f64)>,
    ) -> Result<MedicalOperationResult<ImageAnalysisResult>, MedicalError> {
        let start_time = std::time::Instant::now();
        let result = self
            .medical_imaging
            .analyze_grid(data, width, height, bins, threshold, window)?;
        let execution_time = start_time.elapsed().as_millis() as u64;
        Ok(MedicalOperationResult {
            result,
            execution_time,
            privacy_score: None,
            compliance_status: ComplianceStatus::Compliant,
            audit_trail: Vec::new(),
        })
    }

    /// Process medical image
    pub fn process_medical_image(
        &mut self,
        image: MedicalImage,
        processing_type: ImageProcessingType,
    ) -> Result<MedicalOperationResult<ProcessedImage>, MedicalError> {
        let start_time = std::time::Instant::now();

        // Validate image
        self.medical_imaging.validate_image(&image)?;

        // Process image
        let processed_image = self
            .medical_imaging
            .process_image(&image, processing_type)?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(MedicalOperationResult {
            result: processed_image,
            execution_time,
            privacy_score: None,
            compliance_status: ComplianceStatus::Compliant,
            audit_trail: Vec::new(),
        })
    }

    /// Screen compounds by rule-based filtering + Tanimoto similarity ranking.
    /// `query_smiles` is an optional reference structure for similarity ranking. The
    /// result is honestly labeled (see `ScreeningProposal::epistemic_status`) — it is
    /// NOT a binding-affinity or efficacy prediction.
    pub fn screen_compounds(
        &mut self,
        compounds: Vec<Compound>,
        target: DrugTarget,
        query_smiles: Option<&str>,
    ) -> Result<MedicalOperationResult<ScreeningProposal>, MedicalError> {
        let start_time = std::time::Instant::now();

        // Validate compounds
        self.drug_discovery.validate_compounds(&compounds)?;

        // Screen compounds
        let results = self
            .drug_discovery
            .screen_compounds(&compounds, &target, query_smiles)?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(MedicalOperationResult {
            result: results,
            execution_time,
            privacy_score: None,
            compliance_status: ComplianceStatus::Compliant,
            audit_trail: Vec::new(),
        })
    }

    /// Check compliance
    pub fn check_compliance(
        &mut self,
        compliance_type: ComplianceType,
    ) -> Result<MedicalOperationResult<ComplianceReport>, MedicalError> {
        let start_time = std::time::Instant::now();

        // Check compliance
        let report = self.compliance_monitor.check_compliance(compliance_type)?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(MedicalOperationResult {
            result: report,
            execution_time,
            privacy_score: None,
            compliance_status: ComplianceStatus::Compliant,
            audit_trail: Vec::new(),
        })
    }

    /// Get performance statistics
    pub fn get_performance_stats(&self) -> MedicalPerformanceMetrics {
        self.patient_manager.get_performance_metrics()
    }

    /// List all patients
    pub fn list_patients(&self) -> Vec<String> {
        self.patient_manager.list_patients()
    }

    /// Get patient information
    pub fn get_patient_info(&self, patient_id: &str) -> Option<Patient> {
        self.patient_manager.get_patient(patient_id).ok()
    }
}
