use super::*;

/// Physics field data
#[derive(Debug, Clone)]
pub struct PhysicsField {
    pub field_id: String,
    pub field_type: FieldType,
    pub dimensions: Vec<usize>,
    pub data: Vec<f64>,
    pub metadata: FieldMetadata,
}

/// Field types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FieldType {
    /// Scalar field
    Scalar,
    /// Vector field
    Vector,
    /// Tensor field
    Tensor,
    /// Matrix field
    Matrix,
}

/// Field metadata
#[derive(Debug, Clone)]
pub struct FieldMetadata {
    pub field_name: String,
    pub physical_quantity: String,
    pub units: String,
    pub time_step: u64,
    pub iteration: u64,
}

/// Simulation representation
#[derive(Debug, Clone)]
pub struct Simulation {
    pub config: SimulationConfig,
    pub current_time: f64,
    pub current_step: u64,
    pub fields: HashMap<String, PhysicsField>,
    pub mesh: Option<Mesh>,
    pub status: SimulationStatus,
}

/// Simulation status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SimulationStatus {
    Created,
    Initialized,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

/// Mesh representation
#[derive(Debug, Clone)]
pub struct Mesh {
    pub mesh_id: String,
    pub mesh_type: MeshType,
    pub dimensions: Vec<usize>,
    pub nodes: Vec<MeshNode>,
    pub elements: Vec<MeshElement>,
    pub quality_metrics: MeshQualityMetrics,
}

/// Simulation mesh node
#[derive(Debug, Clone)]
pub struct SimulationMeshNode {
    pub node_id: String,
    pub coordinates: Vec<f64>,
    pub node_type: MeshNodeType,
    pub boundary_type: Option<BoundaryType>,
}

/// Mesh node types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MeshNodeType {
    Interior,
    Boundary,
    Corner,
    Edge,
}

/// Mesh element
#[derive(Debug, Clone)]
pub struct MeshElement {
    pub element_id: String,
    pub element_type: MeshElementType,
    pub node_ids: Vec<String>,
    pub element_data: Vec<f64>,
}

/// Mesh element types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MeshElementType {
    Triangle,
    Quadrilateral,
    Tetrahedron,
    Hexahedron,
    Prism,
    Pyramid,
}
