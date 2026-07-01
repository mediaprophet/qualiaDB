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


/// QPU Bridge Manager - Main interface for quantum computing operations
///
/// This struct manages connections to remote quantum computing resources while
/// maintaining strict zero-allocation invariants and security requirements.
use super::*;

impl From<QPUSubmissionState> for QPUJobStatus {
    fn from(state: QPUSubmissionState) -> Self {
        match state {
            QPUSubmissionState::Idle => QPUJobStatus::Queued,
            QPUSubmissionState::Submitting => QPUJobStatus::Queued,
            QPUSubmissionState::Waiting => QPUJobStatus::Running,
            QPUSubmissionState::Retrieving => QPUJobStatus::Running,
        }
    }
}

impl core::fmt::Display for QPUBridgeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            QPUBridgeError::Success => write!(f, "Success"),
            QPUBridgeError::NotConnected => write!(f, "Not connected"),
            QPUBridgeError::AlreadyConnected => write!(f, "Already connected"),
            QPUBridgeError::AuthenticationFailed => write!(f, "Authentication failed"),
            QPUBridgeError::InvalidConfiguration => write!(f, "Invalid configuration"),
            QPUBridgeError::RateLimited => write!(f, "Rate limited"),
            QPUBridgeError::QueueFull => write!(f, "Queue full"),
            QPUBridgeError::JobNotFound => write!(f, "Job not found"),
            QPUBridgeError::JobNotCompleted => write!(f, "Job not completed"),
            QPUBridgeError::JobFailed => write!(f, "Job failed"),
            QPUBridgeError::JobTimeout => write!(f, "Job timeout"),
            QPUBridgeError::InvalidInput => write!(f, "Invalid input"),
            QPUBridgeError::NetworkError => write!(f, "Network error"),
            QPUBridgeError::QuantumError => write!(f, "Quantum error"),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use crate::solvers::qpu::pre_solver as problem;
