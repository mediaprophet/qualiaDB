//! Buffer alignment utilities for GPU transfer
//!
//! Provides 64-byte alignment utilities for zero-copy GPU transfers using bytemuck.

use bytemuck::{Pod, Zeroable};

/// 64-byte aligned buffer for GPU transfer (specific type only)
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct AlignedBufferF32 {
    data: [f32; 16], // 64 bytes
}

impl AlignedBufferF32 {
    /// Create a new aligned buffer
    pub fn new(data: [f32; 16]) -> Self {
        Self { data }
    }

    /// Get a reference to the data
    pub fn as_ref(&self) -> &[f32; 16] {
        &self.data
    }

    /// Convert to bytes for GPU transfer
    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(&self.data)
    }
}

/// RenderQuin: 64-byte aligned struct for GPU transfer of PGA Motors and semantic data
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct RenderQuin {
    // PGA Motor (32 bytes)
    pub motor: [f32; 8],
    // Semantic data (32 bytes)
    pub semantic_id: u64,
    pub intensity: f32,
    pub confidence: f32,
    pub timestamp: f32,
    pub _pad: [u8; 12],
}

// Compile-time assertion to ensure RenderQuin is 64 bytes
const _: [(); 64] = [(); std::mem::size_of::<RenderQuin>()];

impl RenderQuin {
    /// Create a new RenderQuin
    pub fn new(
        motor: [f32; 8],
        semantic_id: u64,
        intensity: f32,
        confidence: f32,
        timestamp: f32,
    ) -> Self {
        Self {
            motor,
            semantic_id,
            intensity,
            confidence,
            timestamp,
            _pad: [0; 12],
        }
    }

    /// Create a default RenderQuin with identity motor
    pub fn default() -> Self {
        Self {
            motor: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0], // Identity motor
            semantic_id: 0,
            intensity: 0.0,
            confidence: 0.0,
            timestamp: 0.0,
            _pad: [0; 12],
        }
    }
}

impl Default for RenderQuin {
    fn default() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aligned_buffer_size() {
        let buffer = AlignedBufferF32::new([0.0; 16]);
        assert_eq!(std::mem::size_of_val(&buffer), 64);
    }

    #[test]
    fn test_render_quin_size() {
        assert_eq!(std::mem::size_of::<RenderQuin>(), 64);
    }

    #[test]
    fn test_render_quin_bytes() {
        let quin = RenderQuin::new(
            [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
            123,
            0.5,
            0.9,
            1000.0,
        );
        let bytes = bytemuck::bytes_of(&quin);
        assert_eq!(bytes.len(), 64);
    }

    #[test]
    fn test_render_quin_default() {
        let quin = RenderQuin::default();
        assert_eq!(quin.motor[0], 1.0); // Identity motor scalar
        assert_eq!(quin.semantic_id, 0);
    }
}
