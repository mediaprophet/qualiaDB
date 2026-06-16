//! Spectral-Logical Payload [α, μ, σ] implementation

use serde::{Deserialize, Serialize};

/// Spectral-Logical payload [α, μ, σ]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SpectralPayload {
    /// Amplitude / Dynamic Range / Confidence Weight
    pub alpha: f32,
    /// Modulation / Phase / Metadata Carrier
    pub mu: f32,
    /// Spectral Signature / Logical Class Index
    pub sigma: f32,
}

impl Default for SpectralPayload {
    fn default() -> Self {
        Self {
            alpha: 1.0,  // Full confidence by default
            mu: 0.0,
            sigma: 0.0,
        }
    }
}

impl SpectralPayload {
    pub fn new(alpha: f32, mu: f32, sigma: f32) -> Self {
        Self { alpha, mu, sigma }
    }
}