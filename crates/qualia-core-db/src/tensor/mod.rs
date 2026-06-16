//! 10D Volumetric Tensor System
//!
//! Implements the 10-dimensional tensor coordinate system [q, v, w, x, y, z, t, α, μ, σ]
//! for the Q42 volumetric tensor system with zero-heap hot path guarantees.

pub mod coordinate;
pub mod payload;
pub mod topology;
pub mod manifold;
pub mod spacetime;
pub mod spectral;
pub mod quantum;
pub mod hardware_tier;
pub mod gsr;
pub mod q42_integration;

use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

/// 10D Tensor coordinate system [q, v, w, x, y, z, t, α, μ, σ]
/// 
/// Zero-heap compatible, stack-allocated structure for hot path operations.
/// Uses fixed-size f32 values for GPU/SIMD compatibility and quantization.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct Tensor10D {
    /// Quantum Context / Superposition Index (10th dimension)
    /// q = 0: Collapsed Ground Truth
    /// q > 0: Parallel epistemic contexts, pending resolutions
    pub q: f32,
    
    /// Topological / Algebraic Variety Class
    /// v = 0: Euclidean, v = 1: Cyclic/Toroidal, v = 2: Hyperbolic/Tree, v = 3+: Boundary Cliques
    pub v: f32,
    
    /// Manifold / Domain Index (Multi-Head Bifurcation)
    /// w = 0: Medical, w = 1: Legal, w = 2: Personal, w = 3: Environmental, w = 4: Socioeconomic
    pub w: f32,
    
    /// Semantic Topology X coordinate
    pub x: f32,
    
    /// Semantic Topology Y coordinate
    pub y: f32,
    
    /// Semantic Topology Z coordinate
    pub z: f32,
    
    /// Temporal State / Provenance Ledger
    pub t: f32,
    
    /// Spectral Amplitude / Dynamic Range / Confidence Weight
    pub alpha: f32,
    
    /// Spectral Modulation / Phase / Metadata Carrier
    pub mu: f32,
    
    /// Spectral Signature / Logical Class Index
    pub sigma: f32,
}

impl Default for Tensor10D {
    fn default() -> Self {
        Self {
            q: 0.0,      // Ground truth by default
            v: 0.0,      // Euclidean topology by default
            w: 0.0,      // Medical domain by default
            x: 0.0,
            y: 0.0,
            z: 0.0,
            t: 0.0,      // Initial time slice
            alpha: 1.0,  // Full confidence/amplitude by default
            mu: 0.0,
            sigma: 0.0,
        }
    }
}

impl Tensor10D {
    /// Creates a new tensor with specified coordinates
    #[inline]
    pub fn new(q: f32, v: f32, w: f32, x: f32, y: f32, z: f32, t: f32, alpha: f32, mu: f32, sigma: f32) -> Self {
        Self { q, v, w, x, y, z, t, alpha, mu, sigma }
    }
    
    /// Creates a ground truth tensor (q = 0)
    #[inline]
    pub fn ground_truth(v: f32, w: f32, x: f32, y: f32, z: f32, t: f32, alpha: f32, mu: f32, sigma: f32) -> Self {
        Self { q: 0.0, v, w, x, y, z, t, alpha, mu, sigma }
    }
    
    /// Creates a parallel context tensor (q > 0)
    #[inline]
    pub fn parallel_context(q: f32, v: f32, w: f32, x: f32, y: f32, z: f32, t: f32, alpha: f32, mu: f32, sigma: f32) -> Self {
        Self { q, v, w, x, y, z, t, alpha, mu, sigma }
    }
    
    /// Returns true if this is a ground truth tensor (q = 0)
    #[inline]
    pub fn is_ground_truth(&self) -> bool {
        self.q == 0.0
    }
    
    /// Calculates Euclidean distance between spatial coordinates (x, y, z)
    #[inline]
    pub fn spatial_distance(&self, other: &Self) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
    
    /// Calculates full 10D distance considering topological adjustments
    #[inline]
    pub fn full_distance(&self, other: &Self) -> f32 {
        // Use topological class to determine distance metric
        match self.v as u32 {
            0 => self.euclidean_distance(other),  // Euclidean
            1 => self.cyclic_distance(other),    // Cyclic/Toroidal
            2 => self.hyperbolic_distance(other), // Hyperbolic/Tree
            _ => self.boundary_distance(other),   // Boundary Cliques
        }
    }
    
    /// Euclidean distance (standard straight-line)
    #[inline]
    fn euclidean_distance(&self, other: &Self) -> f32 {
        let spatial = self.spatial_distance(other);
        let temporal = (self.t - other.t).abs();
        let spectral = ((self.alpha - other.alpha).powi(2) + 
                       (self.mu - other.mu).powi(2) + 
                       (self.sigma - other.sigma).powi(2)).sqrt();
        (spatial.powi(2) + temporal.powi(2) + spectral.powi(2)).sqrt()
    }
    
    /// Cyclic distance (modulo arithmetic for toroidal topology)
    #[inline]
    fn cyclic_distance(&self, other: &Self) -> f32 {
        let dx = (self.x - other.x).abs().min(1.0 - (self.x - other.x).abs());
        let dy = (self.y - other.y).abs().min(1.0 - (self.y - other.y).abs());
        let dz = (self.z - other.z).abs().min(1.0 - (self.z - other.z).abs());
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
    
    /// Hyperbolic distance (exponential hierarchy)
    #[inline]
    fn hyperbolic_distance(&self, other: &Self) -> f32 {
        let dx = (self.x - other.x).abs();
        let dy = (self.y - other.y).abs();
        let dz = (self.z - other.z).abs();
        (dx.exp() + dy.exp() + dz.exp()).ln()
    }
    
    /// Boundary clique distance (byte comparison)
    #[inline]
    fn boundary_distance(&self, other: &Self) -> f32 {
        if self.v == other.v {
            0.0
        } else {
            1.0
        }
    }
    
    /// Returns true if this is a parallel context tensor (q > 0)
    #[inline]
    pub fn is_parallel_context(&self) -> bool {
        self.q > 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_tensor_default() {
        let tensor = Tensor10D::default();
        assert_eq!(tensor.q, 0.0);
        assert_eq!(tensor.v, 0.0);
        assert_eq!(tensor.w, 0.0);
        assert!(tensor.is_ground_truth());
        assert!(!tensor.is_parallel_context());
    }
    
    #[test]
    fn test_tensor_new() {
        let tensor = Tensor10D::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0);
        assert_eq!(tensor.q, 1.0);
        assert_eq!(tensor.v, 2.0);
        assert_eq!(tensor.w, 3.0);
        assert!(tensor.is_parallel_context());
    }
    
    #[test]
    fn test_ground_truth() {
        let tensor = Tensor10D::ground_truth(0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 1.0, 0.0, 0.0);
        assert!(tensor.is_ground_truth());
        assert!(!tensor.is_parallel_context());
        assert_eq!(tensor.q, 0.0);
    }
    
    #[test]
    fn test_spatial_distance() {
        let t1 = Tensor10D::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0);
        let t2 = Tensor10D::new(0.0, 0.0, 0.0, 3.0, 4.0, 0.0, 0.0, 1.0, 0.0, 0.0);
        let distance = t1.spatial_distance(&t2);
        assert!((distance - 5.0).abs() < 0.001); // 3-4-5 triangle
    }
    
    #[test]
    fn test_full_distance() {
        let t1 = Tensor10D::default();
        let t2 = Tensor10D::default();
        assert_eq!(t1.full_distance(&t2), 0.0);
    }
    
    #[test]
    fn test_pod_zeroable() {
        // Test that Tensor10D satisfies Pod and Zeroable traits
        let tensor = Tensor10D::default();
        assert_eq!(tensor.q, 0.0);
        assert_eq!(tensor.v, 0.0);
        
        // Test byte-level equality for Pod
        let t1 = Tensor10D::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0);
        let bytes: &[u8] = bytemuck::bytes_of(&t1);
        assert_eq!(bytes.len(), std::mem::size_of::<Tensor10D>());
    }
}
