//! Spectral processing and analysis for [α, μ, σ] payload

use serde::{Deserialize, Serialize};

/// Spectral decomposition result
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SpectralDecomposition {
    pub amplitude: f32,
    pub phase: f32,
    pub frequency: f32,
}

impl Default for SpectralDecomposition {
    fn default() -> Self {
        Self {
            amplitude: 0.0,
            phase: 0.0,
            frequency: 0.0,
        }
    }
}

impl SpectralDecomposition {
    pub fn new(amplitude: f32, phase: f32, frequency: f32) -> Self {
        Self {
            amplitude,
            phase,
            frequency,
        }
    }
}
