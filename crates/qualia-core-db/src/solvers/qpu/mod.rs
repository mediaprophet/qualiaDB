//! QPU (Quantum Processing Unit) solver integration.
//!
//! Moved from the standalone `qpu/` crate.  Config-file loading and the old
//! single-endpoint `QpuClient` have been removed; authentication and HTTP
//! egress are now handled by `qualia-client-core::qpu_oracle` and
//! `qualia-client-core::qpu_dispatcher`.

pub mod dispatcher;
pub mod pre_solver;

use serde::{Deserialize, Serialize};

// ── Problem type ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProblemType {
    /// Quantum annealing (QUBO)
    Annealing,
    /// Gate-model quantum circuit
    GateModel,
    /// Variational Quantum Eigensolver
    Vqe,
    /// Quantum Approximate Optimisation Algorithm
    Qaoa,
}

// ── Job types ─────────────────────────────────────────────────────────────────

/// Parameters for a QPU job submission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobParameters {
    pub num_qubits: u32,
    /// Hamiltonian JSON (annealing problems)
    pub hamiltonian: Option<String>,
    /// Circuit JSON (gate-model problems)
    pub circuit: Option<String>,
    pub shots: u32,
    pub extra: serde_json::Value,
}

impl Default for JobParameters {
    fn default() -> Self {
        Self {
            num_qubits: 1,
            hamiltonian: None,
            circuit: None,
            shots: 1000,
            extra: serde_json::Value::Null,
        }
    }
}

/// A QPU job ready for dispatch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QpuJob {
    pub job_id: String,
    pub problem_type: ProblemType,
    pub parameters: JobParameters,
    /// Unix timestamp ms (wall-clock submission time)
    pub created_at_ms: u64,
}

impl QpuJob {
    pub fn new(job_id: String, problem_type: ProblemType, parameters: JobParameters) -> Self {
        Self {
            job_id,
            problem_type,
            parameters,
            created_at_ms: current_time_ms(),
        }
    }
}

impl Default for QpuJob {
    fn default() -> Self {
        Self {
            job_id: "default_job".into(),
            problem_type: ProblemType::Annealing,
            parameters: JobParameters::default(),
            created_at_ms: 0,
        }
    }
}

// ── Result types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Timeout,
}

/// Measurement result from a QPU run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Measurement {
    pub bitstring: String,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResultData {
    pub measurements: Vec<Measurement>,
    pub energies: Option<Vec<f64>>,
    pub metadata: serde_json::Value,
}

/// Completed or failed QPU job result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QpuResult {
    pub job_id: String,
    pub status: JobStatus,
    pub result: Option<JobResultData>,
    pub completed_at_ms: Option<u64>,
    pub error: Option<String>,
}

impl QpuResult {
    pub fn failed(job_id: String, msg: String) -> Self {
        Self {
            job_id,
            status: JobStatus::Failed,
            result: None,
            completed_at_ms: Some(current_time_ms()),
            error: Some(msg),
        }
    }
}

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum QpuError {
    Api(String),
    Network(String),
    JobFailed(String),
    Timeout,
    NotUnlocked,
}

impl std::fmt::Display for QpuError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Api(s) => write!(f, "QPU API error: {}", s),
            Self::Network(s) => write!(f, "QPU network error: {}", s),
            Self::JobFailed(s) => write!(f, "QPU job failed: {}", s),
            Self::Timeout => write!(f, "QPU job timed out"),
            Self::NotUnlocked => {
                write!(f, "QPU Oracle not unlocked — affirm commitment in Settings")
            }
        }
    }
}

impl std::error::Error for QpuError {}

// ── Utilities ─────────────────────────────────────────────────────────────────

fn current_time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
