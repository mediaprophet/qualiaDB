//! GPU Compute Pipeline for Quantum Approximations
//!
//! This module defines the GPU pipeline structures for quantum computations.

/// GPU Compute Pipeline for Quantum Approximations
#[repr(C)]
pub struct QuantumGPUPipeline {
    /// WebGPU compute shader handle
    pub shader_handle: u32,
    /// Buffer for quantum matrices
    pub matrix_buffer: *mut u8,
    /// Buffer size in bytes
    pub buffer_size: usize,
    /// Current computation state
    pub computation_state: GPUComputationState,
}

/// GPU Computation States
#[repr(u8)]
#[derive(Clone, Copy, PartialEq)]
pub enum GPUComputationState {
    Idle = 0,
    Computing = 1,
    Ready = 2,
    Error = 3,
}

/// GPU Shader Parameters (fixed-size, no allocation)
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GPUShaderParams {
    /// Computation type
    pub computation_type: u32,
    /// Buffer pointer
    pub buffer_ptr: *mut u8,
    /// Buffer size
    pub buffer_size: usize,
}