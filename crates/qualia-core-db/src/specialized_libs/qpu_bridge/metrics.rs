//! QPU Bridge - Quantum Processing Unit Bridge for Exact Quantum Computing
//!
//! This module provides a bridge to remote quantum computing resources (IBM Quantum API)
//! via the NativeQuantumDft module, enabling exact Hamiltonian mapping and quantum
//! calculations that cannot be approximated on classical hardware.
//!
//! Architecture:
//! - Time-metered proxy for IBM Quantum API
//! - Job submission and result retrieval
//! - Authentication and rate limiting
//! - Error handling and fallback mechanisms

use core::sync::atomic::{AtomicU32, AtomicU64};

#[repr(C)]
pub struct QPUMetrics {
    /// Total quantum operations
    pub(crate) total_operations: AtomicU64,
    /// Successful operations
    pub(crate) successful_operations: AtomicU64,
    /// Failed operations
    pub(crate) failed_operations: AtomicU64,
    /// Average quantum volume
    pub(crate) avg_quantum_volume: AtomicU32,
    /// Total quantum time
    pub(crate) total_quantum_time_us: AtomicU64,
    /// Cache hit rate
    pub(crate) cache_hit_rate: AtomicU32,
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq)]
pub enum QPUErrorCode {
    Success = 0,
    AuthenticationFailed = 1,
    RateLimited = 2,
    InvalidJob = 3,
    QueueFull = 4,
    NetworkError = 5,
    QuantumError = 6,
    Timeout = 7,
    InsufficientCredits = 8,
}

impl QPUMetrics {
    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            total_operations: AtomicU64::new(0),
            successful_operations: AtomicU64::new(0),
            failed_operations: AtomicU64::new(0),
            avg_quantum_volume: AtomicU32::new(0),
            total_quantum_time_us: AtomicU64::new(0),
            cache_hit_rate: AtomicU32::new(0),
        }
    }
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum QPUBridgeError {
    Success = 0,
    NotConnected = 1,
    AlreadyConnected = 2,
    AuthenticationFailed = 3,
    InvalidConfiguration = 4,
    RateLimited = 5,
    QueueFull = 6,
    JobNotFound = 7,
    JobNotCompleted = 8,
    JobFailed = 9,
    JobTimeout = 10,
    InvalidInput = 11,
    NetworkError = 12,
    QuantumError = 13,
}
