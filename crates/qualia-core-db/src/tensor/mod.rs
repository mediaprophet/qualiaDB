//! 10D Volumetric Tensor System
//!
//! Implements the 10-dimensional tensor coordinate system [q, v, w, x, y, z, t, α, μ, σ]
//! for the Q42 volumetric tensor system with zero-heap hot path guarantees.

pub mod bake_pipeline;
pub mod buffer_export;
pub mod coordinate;
pub mod gsr;
pub mod kv_provenance;
pub mod manifold;
pub mod payload;
pub mod q42_integration;
pub mod quantum;
pub mod resident_substrate;
pub mod spacetime;
pub mod spectral;
pub mod topology;
#[cfg(feature = "gpu-runtime")]
pub mod volume_gpu;

pub use bake_pipeline::ScaleTier;
pub use quantum::QuantumContextRange;
pub use spacetime::HierarchicalSpacetimeCoord;

use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

/// 10D Tensor coordinate system [q, v, w, x, y, z, t, α, μ, σ]
///
/// Zero-heap compatible, stack-allocated structure for hot path operations.
/// Uses fixed-size f32 values for GPU/SIMD compatibility and quantization.
/// In the `.10d` v2 identity model, this is the structured manifold address
/// component, interpreted with its declared domain/profile and typed parts
/// that describe shape and behavior. It is not a digest or a globally unique
/// identifier by itself; hashes may index or verify the identity but do not
/// replace these coordinates or their declared semantics.
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

    /// Coordinate Time — the 4th dimension of spacetime (T67 reconciled).
    ///
    /// `t` is a **Coordinate** axis: it participates in distance queries
    /// alongside `x`, `y`, `z`, `α`, `σ`. It is NOT the provenance
    /// carrier. The earlier "Provenance Ledger" label was incorrect and
    /// has been reconciled with `container_10d/axis_role.rs` (T67).
    ///
    /// Cross-reference: T14 (name the two tens) and T15 (resolve t vs μ).
    pub t: f32,

    /// Spectral Amplitude / Dynamic Range / Confidence Weight
    pub alpha: f32,

    /// Provenance Weight / Carrier — epistemic metadata lane (T67 reconciled).
    ///
    /// `μ` is the **CoordinateCarrier**: a dual-role axis that is both a
    /// measured coordinate AND the in-band provenance carrier. This is
    /// distinct from `t` (coordinate time). The provenance/consent lane
    /// is `μ`, not `t`. See `container_10d/axis_role.rs` for the
    /// normative axis-role taxonomy.
    ///
    /// Cross-reference: T14 (name the two tens) and T15 (resolve t vs μ).
    pub mu: f32,

    /// Spectral Signature / Logical Class Index
    pub sigma: f32,
}

impl Default for Tensor10D {
    fn default() -> Self {
        Self {
            q: 0.0,
            v: 0.0,
            w: 0.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
            t: 0.0,
            alpha: 1.0,
            mu: 0.0,
            sigma: 0.0,
        }
    }
}

impl Tensor10D {
    /// Creates a new tensor with specified coordinates
    #[inline]
    pub fn new(
        q: f32,
        v: f32,
        w: f32,
        x: f32,
        y: f32,
        z: f32,
        t: f32,
        alpha: f32,
        mu: f32,
        sigma: f32,
    ) -> Self {
        Self {
            q,
            v,
            w,
            x,
            y,
            z,
            t,
            alpha,
            mu,
            sigma,
        }
    }

    /// Creates a ground truth tensor (q = 0)
    #[inline]
    pub fn ground_truth(
        v: f32,
        w: f32,
        x: f32,
        y: f32,
        z: f32,
        t: f32,
        alpha: f32,
        mu: f32,
        sigma: f32,
    ) -> Self {
        Self {
            q: 0.0,
            v,
            w,
            x,
            y,
            z,
            t,
            alpha,
            mu,
            sigma,
        }
    }

    /// Creates a parallel context tensor (q > 0)
    #[inline]
    pub fn parallel_context(
        q: f32,
        v: f32,
        w: f32,
        x: f32,
        y: f32,
        z: f32,
        t: f32,
        alpha: f32,
        mu: f32,
        sigma: f32,
    ) -> Self {
        Self {
            q,
            v,
            w,
            x,
            y,
            z,
            t,
            alpha,
            mu,
            sigma,
        }
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
        match self.v as u32 {
            0 => self.euclidean_distance(other),
            1 => self.cyclic_distance(other),
            2 => self.hyperbolic_distance(other),
            3 => self.boundary_distance(other),
            4 => self.spherical_healpix_distance(other),
            5 => self.lorentzian_spacetime_distance(other),
            6 => self.multiply_connected_portal_distance(other),
            _ => self.boundary_distance(other),
        }
    }

    /// Euclidean distance (standard straight-line)
    #[inline]
    fn euclidean_distance(&self, other: &Self) -> f32 {
        let spatial = self.spatial_distance(other);
        let temporal = (self.t - other.t).abs();
        let spectral = ((self.alpha - other.alpha).powi(2)
            + (self.mu - other.mu).powi(2)
            + (self.sigma - other.sigma).powi(2))
        .sqrt();
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

    /// Spherical HEALPix distance (folds x, y, z)
    #[inline]
    fn spherical_healpix_distance(&self, other: &Self) -> f32 {
        topology::TopologyClass::SphericalHealpix
            .calculate_distance((self.x, self.y, self.z), (other.x, other.y, other.z))
    }

    /// Lorentzian spacetime distance (folds x, y, z, t)
    #[inline]
    fn lorentzian_spacetime_distance(&self, other: &Self) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        let dt = self.t - other.t;
        (-(dt * dt) + (dx * dx + dy * dy + dz * dz)).abs().sqrt()
    }

    /// Multiply-connected portal distance (folds x, y, z)
    #[inline]
    fn multiply_connected_portal_distance(&self, other: &Self) -> f32 {
        topology::TopologyClass::MultiplyConnectedPortal
            .calculate_distance((self.x, self.y, self.z), (other.x, other.y, other.z))
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
        let tensor = Tensor10D::default();
        assert_eq!(tensor.q, 0.0);
        assert_eq!(tensor.v, 0.0);
        let t1 = Tensor10D::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0);
        let bytes: &[u8] = bytemuck::bytes_of(&t1);
        assert_eq!(bytes.len(), std::mem::size_of::<Tensor10D>());
    }

    // P7.9 — Axis-completeness regression guards (option b)
    #[test]
    fn p7_9_v0_euclidean_alpha_changes_distance() {
        let base = Tensor10D::default();
        let mut perturbed = base;
        perturbed.alpha += 0.25;
        assert!(base.full_distance(&perturbed) > 1e-6);
    }

    #[test]
    fn p7_9_v0_euclidean_mu_changes_distance() {
        let base = Tensor10D::default();
        let mut perturbed = base;
        perturbed.mu += 0.25;
        assert!(base.full_distance(&perturbed) > 1e-6);
    }

    #[test]
    fn p7_9_v0_euclidean_sigma_changes_distance() {
        let base = Tensor10D::default();
        let mut perturbed = base;
        perturbed.sigma += 0.25;
        assert!(base.full_distance(&perturbed) > 1e-6);
    }

    #[test]
    fn p7_9_v0_euclidean_t_changes_distance() {
        let base = Tensor10D::default();
        let mut perturbed = base;
        perturbed.t += 0.25;
        assert!(base.full_distance(&perturbed) > 1e-6);
    }

    #[test]
    fn p7_9_v1_cyclic_alpha_does_not_change_distance() {
        let mut base = Tensor10D::default();
        base.v = 1.0;
        let mut perturbed = base;
        perturbed.alpha += 0.25;
        assert!(base.full_distance(&perturbed) < 1e-6);
    }

    #[test]
    fn p7_9_v1_cyclic_sigma_does_not_change_distance() {
        let mut base = Tensor10D::default();
        base.v = 1.0;
        let mut perturbed = base;
        perturbed.sigma += 0.25;
        assert!(base.full_distance(&perturbed) < 1e-6);
    }

    #[test]
    fn p7_9_v2_hyperbolic_t_does_not_change_distance() {
        let mut base = Tensor10D::default();
        base.v = 2.0;
        let mut perturbed = base;
        perturbed.t += 0.25;
        let d_base = base.full_distance(&base);
        let d_perturbed = base.full_distance(&perturbed);
        assert!((d_perturbed - d_base).abs() < 1e-6);
    }

    #[test]
    fn p7_9_v3_boundary_alpha_does_not_change_distance() {
        let mut base = Tensor10D::default();
        base.v = 3.0;
        let mut perturbed = base;
        perturbed.alpha += 0.25;
        assert!(base.full_distance(&perturbed) < 1e-6);
    }

    #[test]
    fn p7_9_v0_euclidean_q_does_not_change_distance() {
        let base = Tensor10D::default();
        let mut perturbed = base;
        perturbed.q += 0.25;
        assert!(base.full_distance(&perturbed) < 1e-6);
    }

    #[test]
    fn p7_9_v0_euclidean_w_does_not_change_distance() {
        let base = Tensor10D::default();
        let mut perturbed = base;
        perturbed.w += 0.25;
        assert!(base.full_distance(&perturbed) < 1e-6);
    }

    #[test]
    fn test_extended_topology_distance_dispatch() {
        let mut t1 = Tensor10D::default();
        let mut t2 = Tensor10D::default();

        // Spherical Healpix (v=4)
        t1.v = 4.0;
        t2.v = 4.0;
        t1.x = 1.0;
        t2.y = 1.0;
        assert!(t1.full_distance(&t2) > 1.0);

        // Lorentzian Spacetime (v=5)
        t1.v = 5.0;
        t2.v = 5.0;
        t1.x = 3.0;
        t1.y = 0.0;
        t2.x = 0.0;
        t2.y = 0.0;
        t1.t = 5.0;
        t2.t = 0.0;
        // ds^2 = -(5)^2 + 3^2 = -16 -> sqrt(16) = 4.0
        assert!((t1.full_distance(&t2) - 4.0).abs() < 1e-5);

        // Multiply Connected Portal (v=6)
        t1.v = 6.0;
        t2.v = 6.0;
        t1.x = 0.05;
        t2.x = 0.95;
        assert!((t1.full_distance(&t2) - 0.1).abs() < 1e-5);
    }
}
