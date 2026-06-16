//! Zero-allocation quantum biology computation context
//!
//! This module provides the computation context with fixed-size buffers
//! for zero-heap compliance.

/// Zero-allocation quantum biology computation context
#[repr(C)]
pub struct QuantumBiologyContext {
    /// Input buffer for quantum parameters
    pub input_buffer: [u8; 1024],
    /// Output buffer for results
    pub output_buffer: [u8; 1024],
    /// Working buffer for intermediate computations
    pub working_buffer: [u8; 2048],
    /// Current buffer position
    pub buffer_pos: usize,
}

impl Default for QuantumBiologyContext {
    fn default() -> Self {
        Self {
            input_buffer: [0; 1024],
            output_buffer: [0; 1024],
            working_buffer: [0; 2048],
            buffer_pos: 0,
        }
    }
}