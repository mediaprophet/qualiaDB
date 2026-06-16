//! Q42 Volume Integration for 10D Tensor System
//!
//! Bridges the gap between the existing Q42 volume format (NQuin-based)
//! and the new 10D tensor system [q, v, w, x, y, z, t, α, μ, σ].

use crate::NQuin;
use super::Tensor10D;
use serde::{Deserialize, Serialize};

/// Tensor metadata that can be stored alongside NQuin data
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TensorMetadata {
    /// 10D tensor coordinates for this NQuin
    pub tensor: Tensor10D,
    /// Whether this NQuin has associated tensor data
    pub has_tensor: bool,
    /// Tensor data version
    pub tensor_version: u32,
}

impl Default for TensorMetadata {
    fn default() -> Self {
        Self {
            tensor: Tensor10D::default(),
            has_tensor: false,
            tensor_version: 1,
        }
    }
}

impl TensorMetadata {
    /// Create tensor metadata from NQuin and tensor coordinates
    pub fn from_nquin_and_tensor(nquin: &NQuin, tensor: Tensor10D) -> Self {
        Self {
            tensor,
            has_tensor: true,
            tensor_version: 1,
        }
    }
    
    /// Create tensor metadata from NQuin only (no tensor coordinates yet)
    pub fn from_nquin_only(nquin: &NQuin) -> Self {
        Self {
            tensor: Tensor10D::default(),
            has_tensor: false,
            tensor_version: 1,
        }
    }
}

/// Convert NQuin to Tensor10D using semantic mapping
impl Tensor10D {
    /// Convert NQuin to Tensor10D using semantic mapping
    pub fn from_nquin(nquin: &NQuin) -> Self {
        // Extract tensor coordinates from NQuin bit fields
        // This is a simplified mapping - real implementation would need
        // more sophisticated semantic analysis
        
        // q (Quantum Context): Extract from metadata or context field
        let q = Self::extract_quantum_context(nquin);
        
        // v (Topological Class): Extract from context or metadata
        let v = Self::extract_topological_class(nquin);
        
        // w (Manifold Index): Extract from context field
        let w = Self::extract_manifold_index(nquin);
        
        // x, y, z (Semantic Topology): Extract from object field or hash
        let (x, y, z) = Self::extract_semantic_coordinates(nquin);
        
        // t (Temporal State): Extract from metadata Lamport clock
        let t = Self::extract_temporal_state(nquin);
        
        // α, μ, σ (Spectral Payload): Extract from metadata or payload
        let (alpha, mu, sigma) = Self::extract_spectral_payload(nquin);
        
        Tensor10D::new(q, v, w, x, y, z, t, alpha, mu, sigma)
    }
    
    /// Extract quantum context from NQuin metadata
    fn extract_quantum_context(nquin: &NQuin) -> f32 {
        // Extract from context field or metadata
        // For now, default to ground truth (q = 0)
        0.0
    }
    
    /// Extract topological class from NQuin context
    fn extract_topological_class(nquin: &NQuin) -> f32 {
        // Extract from context field
        // For now, default to Euclidean (v = 0)
        0.0
    }
    
    /// Extract manifold index from NQuin context
    fn extract_manifold_index(nquin: &NQuin) -> f32 {
        // Extract from context field bits [0..55]
        // For now, default to medical domain (w = 0)
        0.0
    }
    
    /// Extract semantic coordinates from NQuin object field
    fn extract_semantic_coordinates(nquin: &NQuin) -> (f32, f32, f32) {
        // Extract from object field or use hash-based embedding
        // For now, use a simple hash-based approach
        let hash = nquin.object;
        
        // Use different bits of the hash for x, y, z coordinates
        let x = (hash & 0xFFFF) as f32 / 65535.0;
        let y = ((hash >> 16) & 0xFFFF) as f32 / 65535.0;
        let z = ((hash >> 32) & 0xFFFF) as f32 / 65535.0;
        
        (x, y, z)
    }
    
    /// Extract temporal state from NQuin metadata Lamport clock
    fn extract_temporal_state(nquin: &NQuin) -> f32 {
        // Extract from metadata field [32..60] (Lamport logical clock)
        let clock = nquin.metadata >> 32;
        clock as f32
    }
    
    /// Extract spectral payload from NQuin metadata
    fn extract_spectral_payload(nquin: &NQuin) -> (f32, f32, f32) {
        // Extract from metadata field [0..31] (modality payload)
        // For now, use simple bit extraction
        let payload = nquin.metadata & 0xFFFFFFFF;
        
        let alpha = (payload & 0xFF) as f32 / 255.0;  // Confidence/weight
        let mu = ((payload >> 8) & 0xFF) as f32 / 255.0;     // Modulation/metadata
        let sigma = ((payload >> 16) & 0xFF) as f32 / 255.0; // Spectral signature
        
        (alpha, mu, sigma)
    }
}

/// Q42 volume with 10D tensor support
pub struct Q42TensorVolume {
    /// Base NQuin storage
    nquins: Vec<NQuin>,
    /// Tensor metadata for each NQuin
    tensor_metadata: Vec<TensorMetadata>,
    /// Volume-level tensor configuration
    volume_config: TensorVolumeConfig,
}

/// Volume-level tensor configuration
#[repr(C)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TensorVolumeConfig {
    /// Enable 10D tensor operations
    pub tensor_enabled: bool,
    /// Default manifold index for new NQuins
    pub default_manifold: f32,
    /// Default topological class for new NQuins
    pub default_topology: f32,
    /// Tensor version for this volume
    pub tensor_version: u32,
}

impl Default for TensorVolumeConfig {
    fn default() -> Self {
        Self {
            tensor_enabled: true,
            default_manifold: 0.0, // Medical domain
            default_topology: 0.0, // Euclidean topology
            tensor_version: 1,
        }
    }
}

impl Q42TensorVolume {
    /// Create a new Q42 tensor volume
    pub fn new() -> Self {
        Self {
            nquins: Vec::new(),
            tensor_metadata: Vec::new(),
            volume_config: TensorVolumeConfig::default(),
        }
    }
    
    /// Create a new Q42 tensor volume with specific configuration
    pub fn with_config(config: TensorVolumeConfig) -> Self {
        Self {
            nquins: Vec::new(),
            tensor_metadata: Vec::new(),
            volume_config: config,
        }
    }
    
    /// Add an NQuin to the volume with tensor coordinates
    pub fn add_nquin_with_tensor(&mut self, nquin: NQuin, tensor: Tensor10D) {
        let index = self.nquins.len();
        self.nquins.push(nquin);
        self.tensor_metadata.push(TensorMetadata::from_nquin_and_tensor(
            &self.nquins[index],
            tensor
        ));
    }
    
    /// Add an NQuin to the volume without tensor coordinates
    pub fn add_nquin(&mut self, nquin: NQuin) {
        let index = self.nquins.len();
        self.nquins.push(nquin);
        self.tensor_metadata.push(TensorMetadata::from_nquin_only(
            &self.nquins[index]
        ));
    }
    
    /// Get tensor metadata for an NQuin by index
    pub fn get_tensor_metadata(&self, index: usize) -> Option<&TensorMetadata> {
        self.tensor_metadata.get(index)
    }
    
    /// Get all NQuins with tensor data
    pub fn get_tensorized_nquins(&self) -> Vec<(&NQuin, &TensorMetadata)> {
        self.nquins.iter()
            .zip(self.tensor_metadata.iter())
            .filter(|(_, meta)| meta.has_tensor)
            .collect()
    }
    
    /// Perform 10D tensor search within the volume
    pub fn tensor_search(&self, query: &Tensor10D, max_distance: f32) -> Vec<usize> {
        let mut results = Vec::new();
        
        for (i, metadata) in self.tensor_metadata.iter().enumerate() {
            if !metadata.has_tensor {
                continue;
            }
            
            // Calculate distance between query and stored tensor
            let distance = query.full_distance(&metadata.tensor);
            
            if distance <= max_distance {
                results.push(i);
            }
        }
        
        results
    }
    
    /// Perform temporal query (get state at specific time t)
    pub fn temporal_query(&self, target_t: f32, tolerance: f32) -> Vec<usize> {
        let mut results = Vec::new();
        
        for (i, metadata) in self.tensor_metadata.iter().enumerate() {
            if !metadata.has_tensor {
                continue;
            }
            
            // Check if temporal state matches within tolerance
            let t_diff = (metadata.tensor.t - target_t).abs();
            
            if t_diff <= tolerance {
                results.push(i);
            }
        }
        
        results
    }
    
    /// Perform cross-manifold query (search across multiple w domains)
    pub fn manifold_query(&self, target_w: f32, max_distance: f32) -> Vec<usize> {
        let mut results = Vec::new();
        
        for (i, metadata) in self.tensor_metadata.iter().enumerate() {
            if !metadata.has_tensor {
                continue;
            }
            
            // Check if manifold matches
            let w_diff = (metadata.tensor.w - target_w).abs();
            
            if w_diff <= 0.1 { // Allow small float tolerance
                // Check spatial distance
                let query = Tensor10D::new(
                    0.0, // q
                    metadata.tensor.v, // v
                    target_w, // w
                    metadata.tensor.x, // x
                    metadata.tensor.y, // y
                    metadata.tensor.z, // z
                    metadata.tensor.t, // t
                    metadata.tensor.alpha, // α
                    metadata.tensor.mu, // μ
                    metadata.tensor.sigma, // σ
                );
                
                let distance = query.spatial_distance(&metadata.tensor);
                if distance <= max_distance {
                    results.push(i);
                }
            }
        }
        
        results
    }
    
    /// Get volume configuration
    pub fn config(&self) -> &TensorVolumeConfig {
        &self.volume_config
    }
    
    /// Update volume configuration
    pub fn update_config(&mut self, config: TensorVolumeConfig) {
        self.volume_config = config;
    }
    
    /// Get the number of NQuins in the volume
    pub fn len(&self) -> usize {
        self.nquins.len()
    }
    
    /// Check if the volume is empty
    pub fn is_empty(&self) -> bool {
        self.nquins.is_empty()
    }
    
    /// Get the number of NQuins with tensor data
    pub fn tensor_count(&self) -> usize {
        self.tensor_metadata.iter().filter(|m| m.has_tensor).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_tensor_volume_creation() {
        let volume = Q42TensorVolume::new();
        assert!(volume.is_empty());
        assert_eq!(volume.len(), 0);
    }
    
    #[test]
    fn test_add_nquin_with_tensor() {
        let mut volume = Q42TensorVolume::new();
        
        let nquin = create_test_nquin(1);
        let tensor = Tensor10D::new(0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 0.8, 0.5, 0.3);
        
        volume.add_nquin_with_tensor(nquin, tensor);
        
        assert_eq!(volume.len(), 1);
        assert_eq!(volume.tensor_count(), 1);
        assert!(volume.get_tensor_metadata(0).unwrap().has_tensor);
    }
    
    #[test]
    fn test_add_nquin_without_tensor() {
        let mut volume = Q42TensorVolume::new();
        
        let nquin = create_test_nquin(2);
        volume.add_nquin(nquin);
        
        assert_eq!(volume.len(), 1);
        assert_eq!(volume.tensor_count(), 0);
        assert!(!volume.get_tensor_metadata(0).unwrap().has_tensor);
    }
    
    #[test]
    fn test_tensor_search() {
        let mut volume = Q42TensorVolume::new();
        
        // Add some NQuins with tensors
        for i in 0..5 {
            let nquin = create_test_nquin(i);
            let tensor = Tensor10D::new(
                0.0,              // q
                0.0,              // v (Euclidean)
                0.0,              // w (Medical)
                i as f32,        // x
                i as f32,        // y
                0.0,              // z
                i as f32,        // t
                1.0,              // α
                0.0,              // μ
                0.0,              // σ
            );
            volume.add_nquin_with_tensor(nquin, tensor);
        }
        
        // Search for tensors close to (2.0, 2.0, 0.0)
        let query = Tensor10D::new(0.0, 0.0, 0.0, 2.0, 2.0, 0.0, 0.0, 1.0, 0.0, 0.0);
        let results = volume.tensor_search(&query, 2.0);
        
        // Should find NQuin at index 2 (distance = 2.0)
        assert!(results.contains(&2));
    }
    
    #[test]
    fn test_temporal_query() {
        let mut volume = Q42TensorVolume::new();
        
        // Add NQuins with different temporal states
        for i in 0..5 {
            let nquin = create_test_nquin(i);
            let tensor = Tensor10D::new(
                0.0, 0.0, 0.0, 0.0, 0.0, 0.0, i as f32, 1.0, 0.0, 0.0
            );
            volume.add_nquin_with_tensor(nquin, tensor);
        }
        
        // Query for time t=2 with tolerance 0.1
        let results = volume.temporal_query(2.0, 0.1);
        
        assert!(results.contains(&2));
    }
    
    #[test]
    fn test_manifold_query() {
        let mut volume = Q42TensorVolume::new();
        
        // Add NQuins in different manifolds
        for i in 0..5 {
            let nquin = create_test_nquin(i);
            let w = (i % 3) as f32; // 0, 1, 2 (Medical, Legal, Personal)
            let tensor = Tensor10D::new(
                0.0, 0.0, w, i as f32, i as f32, 0.0, 0.0, 1.0, 0.0, 0.0
            );
            volume.add_nquin_with_tensor(nquin, tensor);
        }
        
        // Query for manifold w=1 (Legal domain)
        let results = volume.manifold_query(1.0, 5.0);
        
        // Should find NQuins at indices 1 and 4 (w=1)
        assert!(results.contains(&1));
        assert!(results.contains(&4));
    }
    
    fn create_test_nquin(id: u64) -> NQuin {
        // Create a simple test NQuin
        NQuin {
            subject: id,
            predicate: id,
            object: id,
            context: id,
            metadata: (id << 32) | id, // Simple metadata for testing
            parity: id ^ id ^ id ^ id,
        }
    }
}