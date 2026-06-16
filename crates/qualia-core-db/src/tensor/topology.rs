//! Topological classes (v) for geometric physics rules

use serde::{Deserialize, Serialize};

/// Topological class variants
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TopologyClass {
    /// Euclidean (flat semantic proximity, standard distance)
    Euclidean = 0,
    /// Cyclic / Toroidal (feedback loops, circadian rhythms, periodic states)
    Cyclic = 1,
    /// Hyperbolic / Tree (hierarchies, family trees, taxonomies)
    Hyperbolic = 2,
    /// Sovereign Boundary Cliques / Community Classes
    BoundaryCliques = 3,
}

impl Default for TopologyClass {
    fn default() -> Self {
        TopologyClass::Euclidean
    }
}

impl TopologyClass {
    /// Calculate distance between two points based on topology class
    pub fn calculate_distance(&self, p1: (f32, f32, f32), p2: (f32, f32, f32)) -> f32 {
        match self {
            TopologyClass::Euclidean => {
                let dx = p1.0 - p2.0;
                let dy = p1.1 - p2.1;
                let dz = p1.2 - p2.2;
                (dx * dx + dy * dy + dz * dz).sqrt()
            },
            TopologyClass::Cyclic => {
                // Modulo-based distance for cyclic topology
                let dx = (p1.0 - p2.0).abs().min(1.0 - (p1.0 - p2.0).abs());
                let dy = (p1.1 - p2.1).abs().min(1.0 - (p1.1 - p2.1).abs());
                let dz = (p1.2 - p2.2).abs().min(1.0 - (p1.2 - p2.2).abs());
                (dx * dx + dy * dy + dz * dz).sqrt()
            },
            TopologyClass::Hyperbolic => {
                // Exponential distance for tree/hierarchy topology
                let dx = (p1.0 - p2.0).abs();
                let dy = (p1.1 - p2.1).abs();
                let dz = (p1.2 - p2.2).abs();
                (dx.exp() + dy.exp() + dz.exp()).ln()
            },
            TopologyClass::BoundaryCliques => {
                // Byte comparison for clique membership
                if p1 == p2 { 0.0 } else { 1.0 }
            },
        }
    }
}