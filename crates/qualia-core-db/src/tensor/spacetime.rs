//! Spacetime dimensions (x, y, z, t) for semantic topology and temporal ledger

use serde::{Deserialize, Serialize};

/// Spacetime coordinates
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SpacetimeCoord {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub t: f32,
}

impl Default for SpacetimeCoord {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0, z: 0.0, t: 0.0 }
    }
}

impl SpacetimeCoord {
    pub fn new(x: f32, y: f32, z: f32, t: f32) -> Self {
        Self { x, y, z, t }
    }
    
    pub fn spatial_distance(&self, other: &Self) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}