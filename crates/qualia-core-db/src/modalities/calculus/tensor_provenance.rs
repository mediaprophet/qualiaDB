//! Tensor Provenance Tracking
//!
//! Implements provenance-linked tensor state representation in the graph.
//! Tracks the origin, transformations, and data lineage of tensor states
//! for reproducibility and auditability of numerical computations.
//!
//! ## Architecture
//!
//! - **Provenance Chain**: Each tensor state maintains a chain of parent states
//! - **Operation Tracking**: Records the operation that produced each state
//! - **Metadata Persistence**: Stores provenance metadata in Quin fields
//! - **Graph Integration**: Links tensor states through subject/object relationships
//!
//! ## Usage
//!
//! ```no_run
//! use qualia_core_db::modalities::calculus::tensor_provenance::{TensorState, TensorProvenance};
//!
//! let state = TensorState::new([1.0, 2.0, 3.0]);
//! let transformed = state.apply_operation("rk4_step", &params);
//! let provenance = transformed.get_provenance();
//! ```

use crate::NQuin;
use std::collections::HashMap;

// ─── Tensor State ─────────────────────────────────────────────────────────────

/// Represents a tensor state with provenance tracking
///
/// Each tensor state includes the data itself and metadata about
/// how it was produced, enabling full reproducibility.
#[derive(Debug, Clone)]
pub struct TensorState {
    /// Tensor data (flattened into a vector)
    pub data: Vec<f64>,
    /// Tensor shape (dimensions)
    pub shape: Vec<usize>,
    /// Provenance information
    pub provenance: TensorProvenance,
    /// Unique identifier for this state
    pub state_id: u64,
}

impl TensorState {
    /// Creates a new tensor state with no provenance (root state)
    pub fn new(data: Vec<f64>, shape: Vec<usize>) -> Self {
        let state_id = Self::generate_state_id(&data, &shape);
        Self {
            data,
            shape,
            provenance: TensorProvenance::Root {
                source: "initial_state".to_string(),
                timestamp: Self::current_timestamp(),
            },
            state_id,
        }
    }
    
    /// Creates a tensor state from a scalar value (1D tensor of size 1)
    pub fn from_scalar(value: f64) -> Self {
        Self::new(vec![value], vec![1])
    }
    
    /// Applies an operation to create a new tensor state with provenance
    pub fn apply_operation(&self, operation: &str, params: &HashMap<String, f64>) -> Self {
        let new_data = self.compute_operation(operation, params);
        let new_shape = self.infer_shape(operation, &new_data);
        let state_id = Self::generate_state_id(&new_data, &new_shape);
        
        Self {
            data: new_data,
            shape: new_shape,
            provenance: TensorProvenance::Derived {
                parent_id: self.state_id,
                operation: operation.to_string(),
                params: params.clone(),
                timestamp: Self::current_timestamp(),
            },
            state_id,
        }
    }
    
    /// Computes the result of an operation on the tensor data
    fn compute_operation(&self, operation: &str, params: &HashMap<String, f64>) -> Vec<f64> {
        match operation {
            "rk4_step" => self.rk4_step(params),
            "scale" => self.scale(params),
            "add" => self.add(params),
            "multiply" => self.multiply(params),
            "transpose" => self.transpose(),
            "reduce_sum" => self.reduce_sum(),
            _ => self.data.clone(), // Identity operation for unknown ops
        }
    }
    
    /// RK4 ODE step operation
    fn rk4_step(&self, params: &HashMap<String, f64>) -> Vec<f64> {
        let step_size = params.get("step_size").copied().unwrap_or(0.01);
        let lambda = params.get("lambda").copied().unwrap_or(0.5);
        
        // Apply exponential decay: y' = -λy
        self.data.iter()
            .map(|&y| y * (-lambda * step_size).exp())
            .collect()
    }
    
    /// Scale operation
    fn scale(&self, params: &HashMap<String, f64>) -> Vec<f64> {
        let factor = params.get("factor").copied().unwrap_or(1.0);
        self.data.iter().map(|&x| x * factor).collect()
    }
    
    /// Add operation
    fn add(&self, params: &HashMap<String, f64>) -> Vec<f64> {
        let value = params.get("value").copied().unwrap_or(0.0);
        self.data.iter().map(|&x| x + value).collect()
    }
    
    /// Multiply operation
    fn multiply(&self, params: &HashMap<String, f64>) -> Vec<f64> {
        let value = params.get("value").copied().unwrap_or(1.0);
        self.data.iter().map(|&x| x * value).collect()
    }
    
    /// Transpose operation (for 2D tensors)
    fn transpose(&self) -> Vec<f64> {
        if self.shape.len() == 2 {
            let rows = self.shape[0];
            let cols = self.shape[1];
            let mut transposed = vec![0.0; self.data.len()];
            
            for i in 0..rows {
                for j in 0..cols {
                    transposed[j * rows + i] = self.data[i * cols + j];
                }
            }
            transposed
        } else {
            self.data.clone()
        }
    }
    
    /// Reduce sum operation
    fn reduce_sum(&self) -> Vec<f64> {
        vec![self.data.iter().sum()]
    }
    
    /// Infers the shape of the result tensor
    fn infer_shape(&self, operation: &str, _data: &Vec<f64>) -> Vec<usize> {
        match operation {
            "reduce_sum" => vec![1],
            "transpose" => {
                if self.shape.len() == 2 {
                    vec![self.shape[1], self.shape[0]]
                } else {
                    self.shape.clone()
                }
            }
            _ => self.shape.clone(),
        }
    }
    
    /// Gets the provenance chain as a vector
    pub fn get_provenance_chain(&self) -> Vec<TensorProvenance> {
        let chain = vec![self.provenance.clone()];
        // In a full implementation, this would recursively traverse parent states
        chain
    }
    
    /// Converts the tensor state to a Quin for graph storage
    pub fn to_quin(&self) -> NQuin {
        let mut quin = NQuin::default();
        quin.subject = self.state_id;
        
        // Pack tensor metadata into object field
        // For simplicity, we store the first element and length
        if !self.data.is_empty() {
            quin.object = self.data[0].to_bits() as u64;
        }
        
        // Store data length in metadata
        quin.metadata = self.data.len() as u64;
        
        // Store provenance hash in context
        quin.context = self.provenance_hash();
        
        quin
    }
    
    /// Computes a hash of the provenance for graph linking
    fn provenance_hash(&self) -> u64 {
        match &self.provenance {
            TensorProvenance::Root { source, timestamp } => {
                let combined = format!("{}:{}", source, timestamp);
                crate::q_hash(&combined)
            }
            TensorProvenance::Derived { parent_id, operation, params, timestamp } => {
                let param_str: String = params.iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect::<Vec<_>>()
                    .join(",");
                let combined = format!("{}:{}:{}:{}", parent_id, operation, param_str, timestamp);
                crate::q_hash(&combined)
            }
        }
    }
    
    /// Generates a unique state ID from data and shape
    fn generate_state_id(data: &Vec<f64>, shape: &Vec<usize>) -> u64 {
        let data_hash: u64 = data.iter()
            .map(|&x| x.to_bits())
            .fold(0u64, |acc, x| acc.wrapping_add(x));
        
        let shape_hash: u64 = shape.iter()
            .fold(0u64, |acc, &x| acc.wrapping_add(x as u64));
        
        data_hash.wrapping_mul(31).wrapping_add(shape_hash)
    }
    
    /// Gets the current timestamp
    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }
    
    /// Gets the provenance information
    pub fn get_provenance(&self) -> &TensorProvenance {
        &self.provenance
    }
}

// ─── Tensor Provenance ───────────────────────────────────────────────────────

/// Provenance information for a tensor state
#[derive(Debug, Clone)]
pub enum TensorProvenance {
    /// Root state with no parent
    Root {
        source: String,
        timestamp: u64,
    },
    /// Derived state from a parent operation
    Derived {
        parent_id: u64,
        operation: String,
        params: HashMap<String, f64>,
        timestamp: u64,
    },
}

impl TensorProvenance {
    /// Checks if this is a root state
    pub fn is_root(&self) -> bool {
        matches!(self, TensorProvenance::Root { .. })
    }
    
    /// Gets the parent ID if this is a derived state
    pub fn parent_id(&self) -> Option<u64> {
        match self {
            TensorProvenance::Derived { parent_id, .. } => Some(*parent_id),
            _ => None,
        }
    }
    
    /// Gets the operation name if this is a derived state
    pub fn operation(&self) -> Option<&str> {
        match self {
            TensorProvenance::Derived { operation, .. } => Some(operation),
            _ => None,
        }
    }
}

// ─── Provenance Graph ───────────────────────────────────────────────────────

/// Graph structure for tracking tensor state relationships
pub struct ProvenanceGraph {
    /// Map of state IDs to tensor states
    states: HashMap<u64, TensorState>,
    /// Edges representing parent-child relationships
    edges: Vec<(u64, u64)>, // (parent_id, child_id)
}

impl ProvenanceGraph {
    /// Creates a new empty provenance graph
    pub fn new() -> Self {
        Self {
            states: HashMap::new(),
            edges: Vec::new(),
        }
    }
    
    /// Adds a tensor state to the graph
    pub fn add_state(&mut self, state: TensorState) {
        let state_id = state.state_id;
        
        // Add edge if this is a derived state
        if let Some(parent_id) = state.provenance.parent_id() {
            self.edges.push((parent_id, state_id));
        }
        
        self.states.insert(state_id, state);
    }
    
    /// Gets a tensor state by ID
    pub fn get_state(&self, state_id: u64) -> Option<&TensorState> {
        self.states.get(&state_id)
    }
    
    /// Gets the lineage (chain of parent states) for a given state
    pub fn get_lineage(&self, state_id: u64) -> Vec<u64> {
        let mut lineage = Vec::new();
        let mut current_id = state_id;
        
        while let Some(state) = self.states.get(&current_id) {
            lineage.push(current_id);
            
            if let Some(parent_id) = state.provenance.parent_id() {
                current_id = parent_id;
            } else {
                break;
            }
        }
        
        lineage
    }
    
    /// Gets all children of a given state
    pub fn get_children(&self, state_id: u64) -> Vec<u64> {
        self.edges.iter()
            .filter(|(parent, _)| *parent == state_id)
            .map(|(_, child)| *child)
            .collect()
    }
    
    /// Validates the provenance graph for consistency
    pub fn validate(&self) -> Result<(), String> {
        // Check that all edges reference valid states
        for (parent_id, child_id) in &self.edges {
            if !self.states.contains_key(parent_id) {
                return Err(format!("Parent state {} not found in graph", parent_id));
            }
            if !self.states.contains_key(child_id) {
                return Err(format!("Child state {} not found in graph", child_id));
            }
        }
        
        // Check for cycles (simple check: no state should be its own ancestor)
        for state_id in self.states.keys() {
            let lineage = self.get_lineage(*state_id);
            if lineage.len() != lineage.iter().collect::<std::collections::HashSet<_>>().len() {
                return Err(format!("Cycle detected in lineage of state {}", state_id));
            }
        }
        
        Ok(())
    }
}

impl Default for ProvenanceGraph {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tensor_state_creation() {
        let state = TensorState::new(vec![1.0, 2.0, 3.0], vec![3]);
        assert_eq!(state.data.len(), 3);
        assert!(state.provenance.is_root());
    }

    #[test]
    fn test_tensor_state_from_scalar() {
        let state = TensorState::from_scalar(5.0);
        assert_eq!(state.data, vec![5.0]);
        assert_eq!(state.shape, vec![1]);
    }

    #[test]
    fn test_apply_operation_rk4_step() {
        let state = TensorState::new(vec![1.0, 2.0, 3.0], vec![3]);
        let mut params = HashMap::new();
        params.insert("step_size".to_string(), 0.01);
        params.insert("lambda".to_string(), 0.5);
        
        let transformed = state.apply_operation("rk4_step", &params);
        assert!(!transformed.provenance.is_root());
        assert_eq!(transformed.provenance.operation(), Some("rk4_step"));
    }

    #[test]
    fn test_apply_operation_scale() {
        let state = TensorState::new(vec![1.0, 2.0, 3.0], vec![3]);
        let mut params = HashMap::new();
        params.insert("factor".to_string(), 2.0);
        
        let transformed = state.apply_operation("scale", &params);
        assert_eq!(transformed.data, vec![2.0, 4.0, 6.0]);
    }

    #[test]
    fn test_apply_operation_add() {
        let state = TensorState::new(vec![1.0, 2.0, 3.0], vec![3]);
        let mut params = HashMap::new();
        params.insert("value".to_string(), 10.0);
        
        let transformed = state.apply_operation("add", &params);
        assert_eq!(transformed.data, vec![11.0, 12.0, 13.0]);
    }

    #[test]
    fn test_provenance_chain() {
        let state1 = TensorState::new(vec![1.0], vec![1]);
        let mut params = HashMap::new();
        params.insert("factor".to_string(), 2.0);
        let state2 = state1.apply_operation("scale", &params);
        
        let chain = state2.get_provenance_chain();
        // Current implementation only returns the current state's provenance
        // Full chain traversal would require graph access
        assert_eq!(chain.len(), 1);
        assert!(!chain[0].is_root());
        assert_eq!(chain[0].operation(), Some("scale"));
    }

    #[test]
    fn test_provenance_graph() {
        let mut graph = ProvenanceGraph::new();
        
        let state1 = TensorState::new(vec![1.0], vec![1]);
        let state1_id = state1.state_id;
        graph.add_state(state1.clone());
        
        let mut params = HashMap::new();
        params.insert("factor".to_string(), 2.0);
        let state2 = TensorState::from_scalar(1.0).apply_operation("scale", &params);
        let state2_id = state2.state_id;
        graph.add_state(state2);
        
        assert!(graph.get_state(state1_id).is_some());
        assert!(graph.get_state(state2_id).is_some());
    }

    #[test]
    fn test_provenance_graph_lineage() {
        let mut graph = ProvenanceGraph::new();
        
        let state1 = TensorState::new(vec![1.0], vec![1]);
        let state1_id = state1.state_id;
        graph.add_state(state1.clone());
        
        let mut params = HashMap::new();
        params.insert("factor".to_string(), 2.0);
        let state2 = state1.apply_operation("scale", &params);
        let state2_id = state2.state_id;
        graph.add_state(state2);
        
        let lineage = graph.get_lineage(state2_id);
        assert_eq!(lineage.len(), 2);
        assert!(lineage.contains(&state1_id));
        assert!(lineage.contains(&state2_id));
    }

    #[test]
    fn test_provenance_graph_validate() {
        let mut graph = ProvenanceGraph::new();
        
        let state1 = TensorState::new(vec![1.0], vec![1]);
        graph.add_state(state1.clone());
        
        let mut params = HashMap::new();
        params.insert("factor".to_string(), 2.0);
        let state2 = state1.apply_operation("scale", &params);
        graph.add_state(state2);
        
        assert!(graph.validate().is_ok());
    }

    #[test]
    fn test_tensor_to_quin() {
        let state = TensorState::new(vec![1.0, 2.0, 3.0], vec![3]);
        let quin = state.to_quin();
        
        assert_eq!(quin.subject, state.state_id);
        assert_eq!(quin.metadata, 3);
    }

    #[test]
    fn test_reduce_sum() {
        let state = TensorState::new(vec![1.0, 2.0, 3.0], vec![3]);
        let reduced = state.apply_operation("reduce_sum", &HashMap::new());
        
        assert_eq!(reduced.data, vec![6.0]);
        assert_eq!(reduced.shape, vec![1]);
    }

    #[test]
    fn test_transpose_2d() {
        let state = TensorState::new(vec![1.0, 2.0, 3.0, 4.0], vec![2, 2]);
        let transposed = state.apply_operation("transpose", &HashMap::new());
        
        assert_eq!(transposed.data, vec![1.0, 3.0, 2.0, 4.0]);
        assert_eq!(transposed.shape, vec![2, 2]);
    }
}
