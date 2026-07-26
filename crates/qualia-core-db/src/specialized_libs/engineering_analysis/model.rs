use super::*;

/// Engineering library performance summary metrics
#[derive(Debug, Clone)]
pub struct EngineeringPerformanceMetrics {
    pub total_analyses: u64,
    pub average_computation_time: f64,
    /// Average solver accuracy / convergence rate across analyses. `None` = not measured —
    /// this summary does not track per-analysis error, so it must not fabricate a value
    /// (previously `new()` claimed a hardcoded 95% accuracy / 98% convergence from nothing).
    pub average_accuracy: Option<f64>,
    pub convergence_rate: Option<f64>,
}

/// Engineering operation result
#[derive(Debug, Clone)]
pub struct EngineeringOperationResult<T> {
    pub result: T,
    pub execution_time: u64,
    pub computational_cost: f64,
    /// Solver accuracy for this analysis. `None` = not computed (no error estimate is
    /// produced), rather than a fabricated per-analysis 0.85–0.95.
    pub accuracy: Option<f64>,
    pub convergence_info: ConvergenceInfo,
}

/// Convergence information
#[derive(Debug, Clone)]
pub struct ConvergenceInfo {
    pub converged: bool,
    pub iterations: u32,
    pub convergence_criterion: f64,
    pub final_error: f64,
}

/// Engineering model representation
#[derive(Debug, Clone)]
pub struct EngineeringModel {
    pub model_id: String,
    pub model_name: String,
    pub model_type: ModelType,
    pub geometry: Geometry,
    pub materials: HashMap<String, Material>,
    pub boundary_conditions: Vec<BoundaryCondition>,
    pub loads: Vec<Load>,
}

/// Model types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ModelType {
    Structural,
    Mechanical,
    Thermal,
    Fluid,
    Multiphysics,
}

/// Geometry
#[derive(Debug, Clone)]
pub struct Geometry {
    pub geometry_type: GeometryType,
    pub dimensions: Vec<f64>,
    pub features: Vec<GeometricFeature>,
}

/// Geometry types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GeometryType {
    Beam,
    Plate,
    Shell,
    Solid,
    Custom(String),
}

/// Geometric features
#[derive(Debug, Clone)]
pub struct GeometricFeature {
    pub feature_id: String,
    pub feature_type: FeatureType,
    pub feature_parameters: HashMap<String, f64>,
}

/// Feature types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FeatureType {
    Hole,
    Fillet,
    Chamfer,
    Rib,
}

/// Materials
#[derive(Debug, Clone)]
pub struct Material {
    pub material_id: String,
    pub material_name: String,
    pub material_properties: MaterialProperties,
}

/// Boundary conditions
#[derive(Debug, Clone)]
pub struct BoundaryCondition {
    pub condition_id: String,
    pub condition_type: BoundaryConditionType,
    pub condition_value: f64,
}

/// Boundary condition types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BoundaryConditionType {
    Fixed,
    Pinned,
    Roller,
    Displacement,
    Force,
    Pressure,
    Temperature,
    HeatFlux,
}

/// Loads
#[derive(Debug, Clone)]
pub struct Load {
    pub load_id: String,
    pub load_type: LoadType,
    pub load_magnitude: f64,
    pub load_direction: Vec<f64>,
    pub application_point: Vec<f64>,
}

/// Load distribution types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LoadDistributionType {
    Point,
    Distributed,
    Moment,
    Pressure,
    Thermal,
    Dynamic,
}

/// Analysis results
#[derive(Debug, Clone)]
pub struct AnalysisResults {
    pub results_id: String,
    pub analysis_type: AnalysisType,
    pub displacement_field: Vec<f64>,
    pub stress_field: Vec<f64>,
    pub strain_field: Vec<f64>,
    pub reaction_forces: Vec<f64>,
    pub safety_factor: f64,
    /// Steady-state temperature field (K) at the mesh nodes. Populated by thermal
    /// conduction analysis (`thermal_conduction`); empty for mechanical analyses.
    pub temperature_field: Vec<f64>,
    /// Heat-flux field (W/m²) at the mesh nodes, `q = −k·dT/dx`. Populated by
    /// thermal conduction analysis; empty for mechanical analyses.
    pub heat_flux_field: Vec<f64>,
}
// Supporting structs

impl EngineeringModel {
    pub fn new() -> Self {
        Self {
            model_id: "model_1".to_string(),
            model_name: "Test Model".to_string(),
            model_type: ModelType::Structural,
            geometry: Geometry::new(),
            materials: HashMap::new(),
            boundary_conditions: Vec::new(),
            loads: Vec::new(),
        }
    }
}

impl Geometry {
    pub fn new() -> Self {
        Self {
            geometry_type: GeometryType::Beam,
            dimensions: vec![1.0, 0.1, 0.1],
            features: Vec::new(),
        }
    }
}

impl GeometricFeature {
    pub fn new() -> Self {
        Self {
            feature_id: "feature_1".to_string(),
            feature_type: FeatureType::Hole,
            feature_parameters: HashMap::new(),
        }
    }
}

impl Material {
    pub fn new() -> Self {
        Self {
            material_id: "steel_1".to_string(),
            material_name: "Steel".to_string(),
            material_properties: MaterialProperties::new(),
        }
    }
}

impl MaterialProperties {
    pub fn new() -> Self {
        Self {
            youngs_modulus: 200000.0,
            poissons_ratio: 0.3,
            density: 7850.0,
            thermal_expansion: 12e-6,
            thermal_conductivity: 50.0,
            specific_heat: 500.0,
            yield_strength: 250.0,
            ultimate_strength: 400.0,
        }
    }
}

impl BoundaryCondition {
    pub fn new() -> Self {
        Self {
            condition_id: "bc_1".to_string(),
            condition_type: BoundaryConditionType::Fixed,
            condition_value: 0.0,
        }
    }
}

impl Load {
    pub fn new() -> Self {
        Self {
            load_id: "load_1".to_string(),
            load_type: LoadType::Point,
            load_magnitude: 1000.0,
            load_direction: vec![0.0, -1.0, 0.0],
            application_point: vec![1.0, 0.0, 0.0],
        }
    }
}

impl AnalysisResults {
    pub fn new() -> Self {
        Self {
            results_id: "results_1".to_string(),
            analysis_type: AnalysisType::LinearStatic,
            displacement_field: Vec::new(),
            stress_field: Vec::new(),
            strain_field: Vec::new(),
            reaction_forces: Vec::new(),
            // No analysis on a default-constructed value — 0, never a fabricated 2.5 safety factor.
            safety_factor: 0.0,
            temperature_field: Vec::new(),
            heat_flux_field: Vec::new(),
        }
    }
}

impl ReliabilityResults {
    pub fn new() -> Self {
        Self {
            results_id: "reliability_1".to_string(),
            reliability_index: 0.95,
            failure_probability: 0.05,
            mean_time_to_failure: 10000.0,
            maintenance_interval: 30,
        }
    }
}

impl EngineeringPerformanceMetrics {
    pub fn new() -> Self {
        Self {
            total_analyses: 0,
            average_computation_time: 0.0,
            average_accuracy: None,
            convergence_rate: None,
        }
    }
}
