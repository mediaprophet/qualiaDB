//! QPU Bridge for Remote Quantum Computing
//!
//! This module defines the QPU bridge for IBM Quantum API integration.

/// QPU Bridge for Remote Quantum Computing
#[repr(C)]
pub struct QPUBridge {
    /// IBM Quantum API endpoint
    pub api_endpoint: [u8; 256],
    /// Authentication token hash
    pub auth_hash: [u8; 32],
    /// Current job ID
    pub job_id: [u8; 64],
    /// Bridge state
    pub bridge_state: QPUBridgeState,
}

/// QPU Bridge States
#[repr(u8)]
#[derive(Clone, Copy, PartialEq)]
pub enum QPUBridgeState {
    Disconnected = 0,
    Connecting = 1,
    Connected = 2,
    Submitting = 3,
    Computing = 4,
    Retrieving = 5,
}

/// QPU Job Parameters (fixed-size, no allocation)
#[repr(C)]
#[derive(Clone, Copy)]
pub struct QPUJobParams {
    /// Job ID
    pub job_id: [u8; 64],
    /// Computation type
    pub computation_type: u32,
    /// Input data pointer
    pub input_data: *const u8,
    /// Input size
    pub input_size: usize,
}