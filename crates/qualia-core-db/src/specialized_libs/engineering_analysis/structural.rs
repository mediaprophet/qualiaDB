use super::*;


/// Structural analyzer for structural engineering analysis
pub struct StructuralAnalyzer {
    pub(super) finite_element_solver: FiniteElementSolver,
    structural_dynamics: StructuralDynamics,
    buckling_analysis: BucklingAnalysis,
    vibration_analysis: VibrationAnalysis,
    model_store: HashMap<String, EngineeringModel>,
    /// Phase 2 linear-algebra library used for FEA matrix assembly / solves.
    linear_algebra: Option<Arc<Mutex<LinearAlgebraLibrary>>>,
}

/// Finite element solver
pub struct FiniteElementSolver {
    mesh_generator: MeshGenerator,
    element_library: ElementLibrary,
    solver_engine: SolverEngine,
    post_processor: PostProcessor,
    /// ZNS zone manager for zero-copy mesh / element storage.
    zns_manager: Option<Arc<Mutex<ZnsZoneManager>>>,
}

/// Mesh generator
pub struct MeshGenerator {
    mesh_types: HashMap<String, MeshType>,
    mesh_algorithms: HashMap<String, MeshAlgorithm>,
    mesh_quality: MeshQuality,
}

/// Mesh types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MeshType {
    /// Triangular mesh
    Triangular,
    /// Quadrilateral mesh
    Quadrilateral,
    /// Tetrahedral mesh
    Tetrahedral,
    /// Hexahedral mesh
    Hexahedral,
    /// Mixed mesh
    Mixed,
    /// Structured mesh
    Structured,
    /// Unstructured mesh
    Unstructured,
}

/// Mesh algorithms
#[derive(Debug, Clone)]
pub struct MeshAlgorithm {
    pub algorithm_id: String,
    pub algorithm_name: String,
    pub algorithm_type: MeshAlgorithmType,
    pub parameters: MeshAlgorithmParameters,
}

/// Mesh algorithm types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MeshAlgorithmType {
    Delaunay,
    AdvancingFront,
    Octree,
    Cartesian,
    Custom(String),
}

/// Mesh algorithm parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshAlgorithmParameters {
    pub element_size: f64,
    pub refinement_level: u32,
    pub quality_criteria: Vec<QualityCriterion>,
}

/// Quality criteria
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityCriterion {
    pub criterion_name: String,
    pub minimum_value: f64,
    pub maximum_value: f64,
}

/// Mesh quality
pub struct MeshQuality {
    pub quality_metrics: HashMap<String, QualityMetric>,
    pub quality_assessment: QualityAssessment,
}

/// Quality metrics
#[derive(Debug, Clone)]
pub struct QualityMetric {
    pub metric_name: String,
    pub metric_value: f64,
    pub metric_type: MetricType,
}

/// Metric types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MetricType {
    AspectRatio,
    Skewness,
    Orthogonality,
    Jacobian,
}

/// Quality assessment
#[derive(Debug, Clone)]
pub struct QualityAssessment {
    pub overall_quality: f64,
    pub quality_grade: QualityGrade,
    pub recommendations: Vec<String>,
}

/// Quality grades
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QualityGrade {
    Excellent,
    Good,
    Fair,
    Poor,
}

/// Element library
pub struct ElementLibrary {
    elements: HashMap<String, Element>,
    element_properties: HashMap<String, ElementProperties>,
}

/// Elements
#[derive(Debug, Clone)]
pub struct Element {
    pub element_id: String,
    pub element_name: String,
    pub element_type: ElementType,
    pub nodes: Vec<Node>,
    pub properties: ElementProperties,
}

/// Element types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ElementType {
    /// 1D elements
    Truss,
    Beam,
    Frame,
    /// 2D elements
    Shell,
    Plate,
    Membrane,
    /// 3D elements
    Solid,
    Tetrahedron,
    Hexahedron,
    /// Special elements
    Mass,
    Spring,
    Damper,
}

/// Nodes
#[derive(Debug, Clone)]
pub struct Node {
    pub node_id: String,
    pub coordinates: Vec<f64>,
    pub degrees_of_freedom: Vec<DOF>,
    pub constraints: Vec<Constraint>,
}

/// Degrees of freedom
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DOF {
    UX,
    UY,
    UZ,
    ROTX,
    ROTY,
    ROTZ,
    Temperature,
    Pressure,
}

/// Constraints
#[derive(Debug, Clone)]
pub struct Constraint {
    pub constraint_id: String,
    pub constraint_type: ConstraintType,
    pub constraint_value: f64,
}

/// Constraint types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConstraintType {
    Fixed,
    Pinned,
    Roller,
    Displacement,
    Rotation,
    Temperature,
}

/// Element properties
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementProperties {
    pub material_properties: MaterialProperties,
    pub geometric_properties: GeometricProperties,
    pub section_properties: SectionProperties,
}

/// Material properties
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialProperties {
    pub youngs_modulus: f64,
    pub poissons_ratio: f64,
    pub density: f64,
    pub thermal_expansion: f64,
    pub thermal_conductivity: f64,
    pub specific_heat: f64,
    pub yield_strength: f64,
    pub ultimate_strength: f64,
}

/// Geometric properties
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeometricProperties {
    pub area: f64,
    pub volume: f64,
    pub perimeter: f64,
    pub surface_area: f64,
}

/// Section properties
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionProperties {
    pub moment_of_inertia: Vec<f64>,
    pub torsional_constant: f64,
    pub section_modulus: Vec<f64>,
    pub shear_center: Vec<f64>,
}

/// Solver engine
pub struct SolverEngine {
    solvers: HashMap<String, Solver>,
    solver_parameters: SolverParameters,
    convergence_criteria: ConvergenceCriteria,
}

/// Solvers
#[derive(Debug, Clone)]
pub struct Solver {
    pub solver_id: String,
    pub solver_name: String,
    pub solver_type: SolverType,
    pub capabilities: SolverCapabilities,
}

/// Solver types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SolverType {
    Direct,
    Iterative,
    Eigenvalue,
    Transient,
    Nonlinear,
}

/// Solver capabilities
#[derive(Debug, Clone)]
pub struct SolverCapabilities {
    pub max_dof: u64,
    pub supported_element_types: Vec<ElementType>,
    pub analysis_types: Vec<AnalysisType>,
}

/// Analysis types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AnalysisType {
    LinearStatic,
    NonlinearStatic,
    LinearDynamic,
    NonlinearDynamic,
    Thermal,
    Buckling,
    Vibration,
}

/// Solver parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolverParameters {
    pub tolerance: f64,
    pub max_iterations: u32,
    pub convergence_acceleration: ConvergenceAcceleration,
}

/// Convergence acceleration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConvergenceAcceleration {
    None,
    Jacobi,
    GaussSeidel,
    SOR,
    Multigrid,
}

/// Convergence criteria
pub struct ConvergenceCriteria {
    pub criteria_type: ConvergenceType,
    pub tolerance: f64,
    pub max_iterations: u32,
}

/// Convergence types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConvergenceType {
    Residual,
    Energy,
    Displacement,
    Force,
}

/// Post processor
pub struct PostProcessor {
    result_extractors: HashMap<String, ResultExtractor>,
    visualization_engine: VisualizationEngine,
    report_generator: ReportGenerator,
}

/// Result extractors
#[derive(Debug, Clone)]
pub struct ResultExtractor {
    pub extractor_id: String,
    pub extractor_name: String,
    pub result_type: ResultType,
    pub extraction_method: ExtractionMethod,
}

/// Result types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ResultType {
    Displacement,
    Stress,
    Strain,
    Force,
    Reaction,
    Energy,
    Temperature,
    HeatFlux,
}

/// Extraction methods
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExtractionMethod {
    Nodal,
    Elemental,
    Gaussian,
    Custom(String),
}

/// Visualization engine
#[derive(Debug, Clone)]
pub struct VisualizationEngine {
    visualization_types: HashMap<String, VisualizationType>,
    rendering_engine: RenderingEngine,
}

/// Visualization types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VisualizationType {
    Contour,
    Vector,
    Deformed,
    Animation,
    Custom(String),
}

/// Rendering engine
#[derive(Debug, Clone)]
pub struct RenderingEngine {
    pub engine_type: RenderingEngineType,
    pub rendering_options: RenderingOptions,
}

/// Rendering engine types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RenderingEngineType {
    OpenGL,
    Vulkan,
    DirectX,
    Software,
}

/// Rendering options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderingOptions {
    pub color_map: String,
    pub scale_factor: f64,
    pub line_width: f64,
    pub transparency: f64,
}

/// Report generator
pub struct ReportGenerator {
    report_templates: HashMap<String, ReportTemplate>,
    export_formats: Vec<ExportFormat>,
}

/// Report templates
#[derive(Debug, Clone)]
pub struct ReportTemplate {
    pub template_id: String,
    pub template_name: String,
    pub template_type: TemplateType,
    pub sections: Vec<ReportSection>,
}

/// Template types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TemplateType {
    Summary,
    Detailed,
    Technical,
    Executive,
}

/// Report sections
#[derive(Debug, Clone)]
pub struct ReportSection {
    pub section_id: String,
    pub section_name: String,
    pub section_content: SectionContent,
}

/// Section content
#[derive(Debug, Clone)]
pub struct SectionContent {
    pub content_type: ContentType,
    pub data: Vec<u8>,
    pub format: ContentFormat,
}

/// Content types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ContentType {
    Text,
    Table,
    Chart,
    Image,
}

/// Content formats
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ContentFormat {
    Text,
    HTML,
    PDF,
    CSV,
}

/// Export formats
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExportFormat {
    PDF,
    HTML,
    CSV,
    JSON,
    XML,
}
// Supporting implementations

impl StructuralAnalyzer {
    pub fn new() -> Self {
        Self {
            finite_element_solver: FiniteElementSolver::new(),
            structural_dynamics: StructuralDynamics::new(),
            buckling_analysis: BucklingAnalysis::new(),
            vibration_analysis: VibrationAnalysis::new(),
            model_store: HashMap::new(),
            linear_algebra: None,
        }
    }

    /// Attach the Phase 2 linear-algebra library for FEA matrix operations.
    pub fn attach_linear_algebra(&mut self, lib: Option<Arc<Mutex<LinearAlgebraLibrary>>>) {
        self.linear_algebra = lib;
    }

    pub fn store_model(&mut self, model: EngineeringModel) {
        self.model_store.insert(model.model_id.clone(), model);
    }

    pub fn initialize(&mut self) -> Result<(), EngineeringError> {
        self.finite_element_solver.initialize()?;
        self.structural_dynamics.initialize()?;
        Ok(())
    }

    pub fn validate_model(&self, model: &EngineeringModel) -> Result<(), EngineeringError> {
        if model.geometry.dimensions.is_empty() {
            return Err(EngineeringError::ValidationError(
                "Model must have dimensions".to_string(),
            ));
        }
        Ok(())
    }

    pub fn analyze(
        &mut self,
        model: &EngineeringModel,
        analysis_type: AnalysisType,
    ) -> Result<AnalysisResults, EngineeringError> {
        // REAL first-principles axial strength-of-materials (a real member analysis, not full FEA):
        //   stress σ = F / A,  strain ε = σ / E,  axial deflection δ = F·L / (A·E),
        //   factor of safety FoS = σ_yield / |σ|.
        // The safety_factor is GENUINELY COMPUTED from the material yield strength and the applied
        // stress — never a fabricated constant (previously a hardcoded 2.5). Missing inputs are
        // reported as InsufficientData, not silently defaulted.
        let material = model.materials.values().next().ok_or_else(|| {
            EngineeringError::InsufficientData(
                "model has no material; cannot compute stress / factor of safety".to_string(),
            )
        })?;
        let mp = &material.material_properties;
        let e = mp.youngs_modulus;
        let sy = mp.yield_strength;

        let dims = &model.geometry.dimensions;
        if dims.len() < 2 || dims.iter().take(2).any(|&d| !(d > 0.0)) {
            return Err(EngineeringError::InsufficientData(
                "geometry needs at least two positive cross-section dimensions to form an area"
                    .to_string(),
            ));
        }
        let area = dims[0] * dims[1]; // cross-sectional area (m²)
        let length = dims.get(2).copied().filter(|&l| l > 0.0).unwrap_or(dims[0]); // member length (m)

        if model.loads.is_empty() {
            return Err(EngineeringError::InsufficientData(
                "model has no loads; cannot compute stress".to_string(),
            ));
        }
        let force: f64 = model.loads.iter().map(|l| l.load_magnitude).sum(); // total axial load (N)

        let stress = force / area; // Pa
        let strain = if e > 0.0 { stress / e } else { 0.0 };
        let displacement = if e > 0.0 {
            force * length / (area * e)
        } else {
            f64::INFINITY
        };
        let safety_factor = if stress.abs() > 0.0 && sy > 0.0 {
            sy / stress.abs()
        } else if stress.abs() == 0.0 {
            f64::INFINITY // no load ⇒ unbounded margin
        } else {
            0.0 // no yield strength supplied ⇒ no defined margin
        };

        match analysis_type {
            AnalysisType::LinearStatic => Ok(AnalysisResults {
                results_id: "structural_axial".to_string(),
                analysis_type,
                displacement_field: vec![displacement],
                stress_field: vec![stress],
                strain_field: vec![strain],
                reaction_forces: vec![-force], // static equilibrium reaction
                safety_factor,
                temperature_field: Vec::new(), // mechanical analysis — no thermal output
                heat_flux_field: Vec::new(),
            }),
            AnalysisType::Buckling => {
                // Euler elastic critical buckling of the same prismatic member,
                // weak-axis second moment of area I = min(b·h³, h·b³)/12 from the
                // two cross-section dimensions, pinned–pinned effective length K=1:
                //   P_cr = π²·E·I / (K·L)².
                // The reported `safety_factor` is the buckling LOAD FACTOR
                // λ = P_cr / |P_applied| — the multiplier on the axial load at
                // which the member buckles (this is exactly the physical margin
                // against buckling, so it fits the `safety_factor` field). The
                // critical load itself is exposed via
                // `BucklingAnalysis::analyze_from_model`.
                let b = dims[0];
                let h = dims[1];
                let i_weak = (b * h * h * h).min(h * b * b * b) / 12.0;
                let k_factor = 1.0_f64;
                let le = k_factor * length;
                let p_cr = std::f64::consts::PI.powi(2) * e * i_weak / (le * le);
                let load_factor = if force.abs() > 0.0 {
                    p_cr / force.abs()
                } else {
                    f64::INFINITY
                };
                Ok(AnalysisResults {
                    results_id: "structural_buckling_euler".to_string(),
                    analysis_type,
                    displacement_field: vec![displacement],
                    stress_field: vec![stress],
                    strain_field: vec![strain],
                    reaction_forces: vec![-force],
                    safety_factor: load_factor,
                    temperature_field: Vec::new(),
                    heat_flux_field: Vec::new(),
                })
            }
            // Modal/dynamic results cannot be represented in the scalar-field
            // `AnalysisResults` shape — they are eigenmodes, not a stress/
            // displacement field. They are genuinely computed, but through the
            // dedicated methods that return the right result types:
            //   Vibration           → VibrationAnalysis::analyze_free / ModalAnalysis::analyze_modal
            //   Thermal             → ThermalAnalyzer::analyze
            //   Nonlinear/Dynamic   → require a full FE time/nonlinear solver (not built)
            _ => Err(EngineeringError::NotImplemented(format!(
                "structural {:?} is not available through the AnalysisResults facade; \
                 use VibrationAnalysis::analyze_free / ModalAnalysis::analyze_modal for \
                 modal & free-vibration results, ThermalAnalyzer for thermal, and a full \
                 finite-element solver for nonlinear/dynamic response",
                analysis_type
            ))),
        }
    }

    pub fn list_analysis_types(&self) -> Vec<String> {
        vec![
            "LinearStatic".to_string(),
            "NonlinearStatic".to_string(),
            "LinearDynamic".to_string(),
        ]
    }

    pub fn get_model(&self, model_id: &str) -> Option<EngineeringModel> {
        self.model_store.get(model_id).cloned()
    }

    pub fn get_performance_metrics(&self) -> EngineeringPerformanceMetrics {
        EngineeringPerformanceMetrics::new()
    }

    /// Borrow the buckling-analysis sub-analyzer.
    pub fn buckling_analysis(&self) -> &BucklingAnalysis {
        &self.buckling_analysis
    }

    /// Mutably borrow the buckling-analysis sub-analyzer.
    pub fn buckling_analysis_mut(&mut self) -> &mut BucklingAnalysis {
        &mut self.buckling_analysis
    }

    /// Borrow the vibration-analysis sub-analyzer.
    pub fn vibration_analysis(&self) -> &VibrationAnalysis {
        &self.vibration_analysis
    }

    /// Mutably borrow the vibration-analysis sub-analyzer.
    pub fn vibration_analysis_mut(&mut self) -> &mut VibrationAnalysis {
        &mut self.vibration_analysis
    }
}

impl FiniteElementSolver {
    pub fn new() -> Self {
        Self {
            mesh_generator: MeshGenerator::new(),
            element_library: ElementLibrary::new(),
            solver_engine: SolverEngine::new(),
            post_processor: PostProcessor::new(),
            zns_manager: None,
        }
    }

    /// Attach a ZNS zone manager for zero-copy mesh / element storage.
    pub fn attach_zns_manager(&mut self, manager: Option<Arc<Mutex<ZnsZoneManager>>>) {
        self.zns_manager = manager;
    }

    pub fn initialize(&mut self) -> Result<(), EngineeringError> {
        self.mesh_generator.initialize()?;
        self.element_library.initialize()?;
        self.solver_engine.initialize()?;
        self.post_processor.initialize()?;
        Ok(())
    }
}

impl MeshGenerator {
    pub fn new() -> Self {
        Self {
            mesh_types: HashMap::new(),
            mesh_algorithms: HashMap::new(),
            mesh_quality: MeshQuality::new(),
        }
    }

    /// Populate the mesh-type and mesh-algorithm registries with the standard
    /// engineering set. The `MeshType` enum exposes Triangular, Quadrilateral,
    /// Tetrahedral, Hexahedral, Mixed, Structured and Unstructured (there are no
    /// Prism/Pyramid variants, so those two requested topologies are represented
    /// by the closest available enum members — Mixed for prism/pyramid hybrid
    /// meshes).
    pub fn initialize(&mut self) -> Result<(), EngineeringError> {
        self.mesh_types
            .insert("triangular".to_string(), MeshType::Triangular);
        self.mesh_types
            .insert("quadrilateral".to_string(), MeshType::Quadrilateral);
        self.mesh_types
            .insert("tetrahedral".to_string(), MeshType::Tetrahedral);
        self.mesh_types
            .insert("hexahedral".to_string(), MeshType::Hexahedral);
        self.mesh_types.insert("prism".to_string(), MeshType::Mixed);
        self.mesh_types
            .insert("pyramid".to_string(), MeshType::Mixed);
        self.mesh_types.insert("mixed".to_string(), MeshType::Mixed);
        self.mesh_types
            .insert("structured".to_string(), MeshType::Structured);
        self.mesh_types
            .insert("unstructured".to_string(), MeshType::Unstructured);

        let default_params = MeshAlgorithmParameters {
            element_size: 1.0,
            refinement_level: 1,
            quality_criteria: Vec::new(),
        };
        self.mesh_algorithms.insert(
            "delaunay".to_string(),
            MeshAlgorithm {
                algorithm_id: "algo_delaunay".to_string(),
                algorithm_name: "Delaunay Triangulation".to_string(),
                algorithm_type: MeshAlgorithmType::Delaunay,
                parameters: default_params.clone(),
            },
        );
        self.mesh_algorithms.insert(
            "advancing_front".to_string(),
            MeshAlgorithm {
                algorithm_id: "algo_advancing_front".to_string(),
                algorithm_name: "Advancing Front".to_string(),
                algorithm_type: MeshAlgorithmType::AdvancingFront,
                parameters: default_params.clone(),
            },
        );
        self.mesh_algorithms.insert(
            "octree".to_string(),
            MeshAlgorithm {
                algorithm_id: "algo_octree".to_string(),
                algorithm_name: "Octree Decomposition".to_string(),
                algorithm_type: MeshAlgorithmType::Octree,
                parameters: default_params.clone(),
            },
        );
        self.mesh_algorithms.insert(
            "structured".to_string(),
            MeshAlgorithm {
                algorithm_id: "algo_structured".to_string(),
                algorithm_name: "Structured Grid".to_string(),
                algorithm_type: MeshAlgorithmType::Custom("Structured".to_string()),
                parameters: default_params.clone(),
            },
        );
        self.mesh_algorithms.insert(
            "unstructured".to_string(),
            MeshAlgorithm {
                algorithm_id: "algo_unstructured".to_string(),
                algorithm_name: "Unstructured Mesh".to_string(),
                algorithm_type: MeshAlgorithmType::Custom("Unstructured".to_string()),
                parameters: default_params,
            },
        );

        Ok(())
    }

    /// Look up a registered mesh type by name.
    pub fn get_mesh_type(&self, name: &str) -> Option<&MeshType> {
        self.mesh_types.get(name)
    }

    /// Look up a registered mesh algorithm by name.
    pub fn get_algorithm(&self, name: &str) -> Option<&MeshAlgorithm> {
        self.mesh_algorithms.get(name)
    }

    /// List the names of all registered mesh types.
    pub fn list_mesh_types(&self) -> Vec<String> {
        let mut names: Vec<String> = self.mesh_types.keys().cloned().collect();
        names.sort();
        names
    }

    /// List the names of all registered mesh algorithms.
    pub fn list_algorithms(&self) -> Vec<String> {
        let mut names: Vec<String> = self.mesh_algorithms.keys().cloned().collect();
        names.sort();
        names
    }

    /// Borrow the mesh-quality sub-component.
    pub fn mesh_quality(&self) -> &MeshQuality {
        &self.mesh_quality
    }

    /// Mutably borrow the mesh-quality sub-component.
    pub fn mesh_quality_mut(&mut self) -> &mut MeshQuality {
        &mut self.mesh_quality
    }
}

impl MeshQuality {
    pub fn new() -> Self {
        Self {
            quality_metrics: HashMap::new(),
            quality_assessment: QualityAssessment::new(),
        }
    }

    /// Register a quality metric under `metric.metric_name`.
    pub fn add_metric(&mut self, metric: QualityMetric) {
        self.quality_metrics
            .insert(metric.metric_name.clone(), metric);
    }

    /// Look up a registered quality metric by name.
    pub fn get_metric(&self, name: &str) -> Option<&QualityMetric> {
        self.quality_metrics.get(name)
    }

    /// List the names of all registered quality metrics.
    pub fn list_metrics(&self) -> Vec<String> {
        let mut names: Vec<String> = self.quality_metrics.keys().cloned().collect();
        names.sort();
        names
    }

    /// Borrow the quality-assessment summary.
    pub fn quality_assessment(&self) -> &QualityAssessment {
        &self.quality_assessment
    }
}

impl QualityAssessment {
    pub fn new() -> Self {
        Self {
            overall_quality: 0.95,
            quality_grade: QualityGrade::Excellent,
            recommendations: Vec::new(),
        }
    }
}

impl ElementLibrary {
    pub fn new() -> Self {
        Self {
            elements: HashMap::new(),
            element_properties: HashMap::new(),
        }
    }

    /// Populate the library with the standard finite-element types used in
    /// structural / mechanical FEA. Each element is registered with a default
    /// isotropic material (steel-like), unit geometry, and the DOF set appropriate
    /// to its kinematics.
    pub fn initialize(&mut self) -> Result<(), EngineeringError> {
        // Shared default properties (steel-like, unit section).
        let default_props = ElementProperties {
            material_properties: MaterialProperties {
                youngs_modulus: 200_000.0,
                poissons_ratio: 0.3,
                density: 7850.0,
                thermal_expansion: 1.2e-5,
                thermal_conductivity: 50.0,
                specific_heat: 500.0,
                yield_strength: 250.0,
                ultimate_strength: 400.0,
            },
            geometric_properties: GeometricProperties {
                area: 1.0,
                volume: 1.0,
                perimeter: 4.0,
                surface_area: 6.0,
            },
            section_properties: SectionProperties {
                moment_of_inertia: vec![1.0 / 12.0, 1.0 / 12.0, 1.0 / 12.0],
                torsional_constant: 1.0 / 12.0,
                section_modulus: vec![1.0 / 6.0, 1.0 / 6.0, 1.0 / 6.0],
                shear_center: vec![0.0, 0.0, 0.0],
            },
        };

        // Helper: build `count` nodes each carrying `dofs` degrees of freedom.
        let make_nodes = |count: usize, dofs: &[DOF]| -> Vec<Node> {
            (0..count)
                .map(|i| Node {
                    node_id: format!("n{i}"),
                    coordinates: vec![i as f64, 0.0, 0.0],
                    degrees_of_freedom: dofs.to_vec(),
                    constraints: Vec::new(),
                })
                .collect()
        };

        // truss_2node: 2 nodes, 2 DOF/node (UX, UY)
        let truss = Element {
            element_id: "truss_2node".to_string(),
            element_name: "2-Node Truss".to_string(),
            element_type: ElementType::Truss,
            nodes: make_nodes(2, &[DOF::UX, DOF::UY]),
            properties: default_props.clone(),
        };
        self.elements.insert("truss_2node".to_string(), truss);
        self.element_properties
            .insert("truss_2node".to_string(), default_props.clone());

        // beam_2node: 2 nodes, 3 DOF/node (UX, UY, ROTZ)
        let beam = Element {
            element_id: "beam_2node".to_string(),
            element_name: "2-Node Beam".to_string(),
            element_type: ElementType::Beam,
            nodes: make_nodes(2, &[DOF::UX, DOF::UY, DOF::ROTZ]),
            properties: default_props.clone(),
        };
        self.elements.insert("beam_2node".to_string(), beam);
        self.element_properties
            .insert("beam_2node".to_string(), default_props.clone());

        // quad_4node: quadrilateral shell, 4 nodes, 2 DOF/node (UX, UY)
        let quad = Element {
            element_id: "quad_4node".to_string(),
            element_name: "4-Node Quadrilateral Shell".to_string(),
            element_type: ElementType::Shell,
            nodes: make_nodes(4, &[DOF::UX, DOF::UY]),
            properties: default_props.clone(),
        };
        self.elements.insert("quad_4node".to_string(), quad);
        self.element_properties
            .insert("quad_4node".to_string(), default_props.clone());

        // hex_8node: hexahedral solid, 8 nodes, 3 DOF/node (UX, UY, UZ)
        let hex = Element {
            element_id: "hex_8node".to_string(),
            element_name: "8-Node Hexahedral Solid".to_string(),
            element_type: ElementType::Hexahedron,
            nodes: make_nodes(8, &[DOF::UX, DOF::UY, DOF::UZ]),
            properties: default_props.clone(),
        };
        self.elements.insert("hex_8node".to_string(), hex);
        self.element_properties
            .insert("hex_8node".to_string(), default_props.clone());

        // tet_4node: tetrahedral solid, 4 nodes, 3 DOF/node (UX, UY, UZ)
        let tet = Element {
            element_id: "tet_4node".to_string(),
            element_name: "4-Node Tetrahedral Solid".to_string(),
            element_type: ElementType::Tetrahedron,
            nodes: make_nodes(4, &[DOF::UX, DOF::UY, DOF::UZ]),
            properties: default_props.clone(),
        };
        self.elements.insert("tet_4node".to_string(), tet);
        self.element_properties
            .insert("tet_4node".to_string(), default_props.clone());

        // shell_8node: shell element, 8 nodes, 6 DOF/node (UX, UY, UZ, ROTX, ROTY, ROTZ)
        let shell = Element {
            element_id: "shell_8node".to_string(),
            element_name: "8-Node Shell".to_string(),
            element_type: ElementType::Shell,
            nodes: make_nodes(
                8,
                &[DOF::UX, DOF::UY, DOF::UZ, DOF::ROTX, DOF::ROTY, DOF::ROTZ],
            ),
            properties: default_props.clone(),
        };
        self.elements.insert("shell_8node".to_string(), shell);
        self.element_properties
            .insert("shell_8node".to_string(), default_props);

        Ok(())
    }

    /// Look up a registered element definition by name.
    pub fn get_element(&self, name: &str) -> Option<&Element> {
        self.elements.get(name)
    }

    /// Look up the properties registered for an element by name.
    pub fn get_properties(&self, name: &str) -> Option<&ElementProperties> {
        self.element_properties.get(name)
    }

    /// List the names of all registered elements.
    pub fn list_elements(&self) -> Vec<String> {
        let mut names: Vec<String> = self.elements.keys().cloned().collect();
        names.sort();
        names
    }
}

impl SolverEngine {
    pub fn new() -> Self {
        Self {
            solvers: HashMap::new(),
            solver_parameters: SolverParameters::new(),
            convergence_criteria: ConvergenceCriteria::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), EngineeringError> {
        Ok(())
    }

    /// Register a solver under `solver.solver_id`.
    pub fn add_solver(&mut self, solver: Solver) {
        self.solvers.insert(solver.solver_id.clone(), solver);
    }

    /// Look up a registered solver by id.
    pub fn get_solver(&self, id: &str) -> Option<&Solver> {
        self.solvers.get(id)
    }

    /// List the ids of all registered solvers.
    pub fn list_solvers(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.solvers.keys().cloned().collect();
        ids.sort();
        ids
    }

    /// Borrow the solver parameters.
    pub fn solver_parameters(&self) -> &SolverParameters {
        &self.solver_parameters
    }

    /// Mutably borrow the solver parameters.
    pub fn solver_parameters_mut(&mut self) -> &mut SolverParameters {
        &mut self.solver_parameters
    }

    /// Borrow the convergence criteria.
    pub fn convergence_criteria(&self) -> &ConvergenceCriteria {
        &self.convergence_criteria
    }

    /// Mutably borrow the convergence criteria.
    pub fn convergence_criteria_mut(&mut self) -> &mut ConvergenceCriteria {
        &mut self.convergence_criteria
    }
}

impl SolverParameters {
    pub fn new() -> Self {
        Self {
            tolerance: 1e-6,
            max_iterations: 1000,
            convergence_acceleration: ConvergenceAcceleration::None,
        }
    }
}

impl ConvergenceCriteria {
    pub fn new() -> Self {
        Self {
            criteria_type: ConvergenceType::Residual,
            tolerance: 1e-6,
            max_iterations: 1000,
        }
    }
}

impl PostProcessor {
    pub fn new() -> Self {
        Self {
            result_extractors: HashMap::new(),
            visualization_engine: VisualizationEngine::new(),
            report_generator: ReportGenerator::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), EngineeringError> {
        self.visualization_engine.initialize()?;
        self.report_generator.initialize()?;
        Ok(())
    }

    /// Register a result extractor under `extractor.extractor_id`.
    pub fn add_extractor(&mut self, extractor: ResultExtractor) {
        self.result_extractors
            .insert(extractor.extractor_id.clone(), extractor);
    }

    /// Look up a registered result extractor by id.
    pub fn get_extractor(&self, id: &str) -> Option<&ResultExtractor> {
        self.result_extractors.get(id)
    }

    /// List the ids of all registered result extractors.
    pub fn list_extractors(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.result_extractors.keys().cloned().collect();
        ids.sort();
        ids
    }
}

impl VisualizationEngine {
    pub fn new() -> Self {
        Self {
            visualization_types: HashMap::new(),
            rendering_engine: RenderingEngine::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), EngineeringError> {
        Ok(())
    }

    /// Register a visualization type under `name`.
    pub fn add_visualization_type(&mut self, name: impl Into<String>, vtype: VisualizationType) {
        self.visualization_types.insert(name.into(), vtype);
    }

    /// Look up a registered visualization type by name.
    pub fn get_visualization_type(&self, name: &str) -> Option<&VisualizationType> {
        self.visualization_types.get(name)
    }

    /// List the names of all registered visualization types.
    pub fn list_visualization_types(&self) -> Vec<String> {
        let mut names: Vec<String> = self.visualization_types.keys().cloned().collect();
        names.sort();
        names
    }

    /// Borrow the rendering engine.
    pub fn rendering_engine(&self) -> &RenderingEngine {
        &self.rendering_engine
    }

    /// Mutably borrow the rendering engine.
    pub fn rendering_engine_mut(&mut self) -> &mut RenderingEngine {
        &mut self.rendering_engine
    }
}

impl RenderingEngine {
    pub fn new() -> Self {
        Self {
            engine_type: RenderingEngineType::OpenGL,
            rendering_options: RenderingOptions::new(),
        }
    }
}

impl RenderingOptions {
    pub fn new() -> Self {
        Self {
            color_map: "jet".to_string(),
            scale_factor: 1.0,
            line_width: 1.0,
            transparency: 0.0,
        }
    }
}

impl ReportGenerator {
    pub fn new() -> Self {
        Self {
            report_templates: HashMap::new(),
            export_formats: vec![ExportFormat::PDF, ExportFormat::HTML],
        }
    }

    pub fn initialize(&mut self) -> Result<(), EngineeringError> {
        Ok(())
    }

    /// Register a report template under `template.template_id`.
    pub fn add_template(&mut self, template: ReportTemplate) {
        self.report_templates
            .insert(template.template_id.clone(), template);
    }

    /// Look up a registered report template by id.
    pub fn get_template(&self, id: &str) -> Option<&ReportTemplate> {
        self.report_templates.get(id)
    }

    /// List the ids of all registered report templates.
    pub fn list_templates(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.report_templates.keys().cloned().collect();
        ids.sort();
        ids
    }

    /// Borrow the supported export formats.
    pub fn export_formats(&self) -> &[ExportFormat] {
        &self.export_formats
    }

    /// Add a supported export format.
    pub fn add_export_format(&mut self, format: ExportFormat) {
        if !self.export_formats.contains(&format) {
            self.export_formats.push(format);
        }
    }
}

