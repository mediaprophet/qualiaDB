//! Manifold bifurcation (w) for multi-head attention

use serde::{Deserialize, Serialize};

/// Manifold domain variants
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ManifoldDomain {
    /// Biological/Medical
    Medical = 0,
    /// Legal/Jurisdictional
    Legal = 1,
    /// Personal/Agency
    Personal = 2,
    /// Environmental/Sensor
    Environmental = 3,
    /// Socioeconomic/Wellbeing
    Socioeconomic = 4,
}

impl Default for ManifoldDomain {
    fn default() -> Self {
        ManifoldDomain::Medical
    }
}

impl ManifoldDomain {
    pub fn from_index(index: f32) -> Self {
        match index as u32 {
            0 => ManifoldDomain::Medical,
            1 => ManifoldDomain::Legal,
            2 => ManifoldDomain::Personal,
            3 => ManifoldDomain::Environmental,
            4 => ManifoldDomain::Socioeconomic,
            _ => ManifoldDomain::Medical, // Default fallback
        }
    }

    pub fn to_index(&self) -> f32 {
        *self as u32 as f32
    }
}
