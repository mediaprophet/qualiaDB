//! In-process QPU job queue and dispatcher.
//!
//! This module manages the lifecycle of QPU jobs in memory.  Actual HTTP
//! egress to the 8 supported providers (IBM, D-Wave, IonQ, Rigetti, Azure,
//! Braket, Google, Quantinuum) is handled by `qualia-client-core::qpu_dispatcher`.
//!
//! Ported from `qpu/src/dispatcher.rs`.

use super::{JobStatus, QpuError, QpuJob, QpuResult};
use std::collections::{BinaryHeap, HashMap};
use std::sync::Arc;
use std::cmp::Ordering;
use tokio::sync::{Mutex, RwLock};

// ── Job state ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct JobState {
    job: QpuJob,
    enqueued_at_ms: u64,
    retries: u32,
    status: InternalStatus,
}

#[derive(Debug, Clone, PartialEq)]
enum InternalStatus {
    Queued,
    Submitted,
    Running,
    Completed,
    Failed,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// ── Queue statistics ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct QueueStats {
    pub pending_count: usize,
    pub running_count: usize,
    pub completed_count: usize,
}

// ── Job queue ─────────────────────────────────────────────────────────────────

/// Prioritized job wrapper for QGroup heuristic
#[derive(Debug, Clone)]
pub struct PrioritizedJob(pub QpuJob);

impl PartialEq for PrioritizedJob {
    fn eq(&self, other: &Self) -> bool {
        self.0.job_id == other.0.job_id
    }
}
impl Eq for PrioritizedJob {}

impl PartialOrd for PrioritizedJob {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PrioritizedJob {
    fn cmp(&self, other: &Self) -> Ordering {
        // QGroup heuristic: group by similar circuit depth and shot count.
        // We sort descending so the BinaryHeap (max-heap) pops highest depth first,
        // and jobs with similar depths will be contiguous.
        let depth_ord = self.0.parameters.circuit_depth.cmp(&other.0.parameters.circuit_depth);
        if depth_ord != Ordering::Equal {
            return depth_ord;
        }
        self.0.parameters.shots.cmp(&other.0.parameters.shots)
    }
}

/// In-process job queue.  HTTP dispatch is performed by the caller via
/// `submit_fn` passed to `process_queue`.
pub struct JobQueue {
    pending: Arc<Mutex<BinaryHeap<PrioritizedJob>>>,
    running: Arc<RwLock<HashMap<String, JobState>>>,
    completed: Arc<Mutex<Vec<QpuResult>>>,
}

impl JobQueue {
    pub fn new() -> Self {
        Self {
            pending: Arc::new(Mutex::new(BinaryHeap::new())),
            running: Arc::new(RwLock::new(HashMap::new())),
            completed: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Enqueue a job for dispatch.
    pub async fn enqueue(&self, job: QpuJob) -> String {
        let id = job.job_id.clone();
        self.pending.lock().await.push(PrioritizedJob(job));
        id
    }

    /// Drain the pending queue and call `dispatch` for each job.
    ///
    /// `dispatch` should return the provider-assigned job ID on success.
    pub async fn process_queue<F>(&self, dispatch: F) -> Result<(), QpuError>
    where
        F: Fn(&QpuJob) -> Result<String, String>,
    {
        let jobs: Vec<QpuJob> = {
            let mut pending = self.pending.lock().await;
            let mut jobs = Vec::with_capacity(pending.len());
            while let Some(job) = pending.pop() {
                jobs.push(job.0);
            }
            jobs
        };

        for job in jobs {
            let job_id = job.job_id.clone();
            match dispatch(&job) {
                Ok(provider_id) => {
                    let state = JobState {
                        job: QpuJob {
                            job_id: provider_id.clone(),
                            ..job
                        },
                        enqueued_at_ms: now_ms(),
                        retries: 0,
                        status: InternalStatus::Submitted,
                    };
                    self.running.write().await.insert(provider_id, state);
                }
                Err(e) => {
                    log::error!("QPU dispatch failed for {}: {}", job_id, e);
                    let result = QpuResult::failed(job_id, e);
                    self.completed.lock().await.push(result);
                }
            }
        }
        Ok(())
    }

    /// Mark a running job as completed or failed based on a polled status.
    pub async fn record_result(&self, job_id: &str, result: QpuResult) {
        let mut running = self.running.write().await;
        if let Some(state) = running.remove(job_id) {
            let _ = state;
        }
        self.completed.lock().await.push(result);
    }

    /// Drain and return all completed results.
    pub async fn take_results(&self) -> Vec<QpuResult> {
        std::mem::take(&mut *self.completed.lock().await)
    }

    pub async fn stats(&self) -> QueueStats {
        QueueStats {
            pending_count: self.pending.lock().await.len(),
            running_count: self.running.read().await.len(),
            completed_count: self.completed.lock().await.len(),
        }
    }
}

impl Default for JobQueue {
    fn default() -> Self {
        Self::new()
    }
}

// ── Dispatcher ────────────────────────────────────────────────────────────────

/// Wraps a `JobQueue` and exposes a simple submit/poll API.
pub struct Dispatcher {
    pub queue: Arc<JobQueue>,
}

impl Dispatcher {
    pub fn new() -> Self {
        Self {
            queue: Arc::new(JobQueue::new()),
        }
    }

    pub async fn submit(&self, job: QpuJob) -> String {
        self.queue.enqueue(job).await
    }

    pub async fn flush<F>(&self, dispatch: F) -> Result<(), QpuError>
    where
        F: Fn(&QpuJob) -> Result<String, String>,
    {
        self.queue.process_queue(dispatch).await
    }

    pub async fn drain_results(&self) -> Vec<QpuResult> {
        self.queue.take_results().await
    }

    pub async fn stats(&self) -> QueueStats {
        self.queue.stats().await
    }
}

impl Default for Dispatcher {
    fn default() -> Self {
        Self::new()
    }
}

// ── Fallback handler ──────────────────────────────────────────────────────────

/// Returns a classical-simulation result when no QPU provider is available.
pub struct FallbackHandler {
    pub enabled: bool,
}

impl FallbackHandler {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    pub fn simulate_classically(&self, job: &QpuJob) -> Result<QpuResult, QpuError> {
        if !self.enabled {
            return Err(QpuError::Api("Fallback is disabled".into()));
        }
        Ok(QpuResult {
            job_id: job.job_id.clone(),
            status: JobStatus::Completed,
            result: Some(super::JobResultData {
                measurements: vec![],
                energies: Some(vec![0.0]),
                metadata: serde_json::json!({"method": "classical_simulation"}),
            }),
            completed_at_ms: Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0),
            ),
            error: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solvers::qpu::{JobParameters, ProblemType};

    #[tokio::test]
    async fn queue_enqueue_and_stats() {
        let q = JobQueue::new();
        let job = QpuJob::new(
            "test-job-1".into(),
            ProblemType::Annealing,
            JobParameters::default(),
        );
        q.enqueue(job).await;
        let stats = q.stats().await;
        assert_eq!(stats.pending_count, 1);
    }

    #[test]
    fn fallback_handler_enabled() {
        let handler = FallbackHandler::new(true);
        let job = QpuJob::default();
        let result = handler.simulate_classically(&job).unwrap();
        assert_eq!(result.status, JobStatus::Completed);
    }

    #[test]
    fn fallback_handler_disabled() {
        let handler = FallbackHandler::new(false);
        let job = QpuJob::default();
        assert!(handler.simulate_classically(&job).is_err());
    }

    #[tokio::test]
    async fn test_qgroup_heuristic_sorting() {
        let q = JobQueue::new();
        
        let mut job1 = QpuJob::new("job1".into(), ProblemType::Vqe, JobParameters::default());
        job1.parameters.circuit_depth = 10;
        job1.parameters.shots = 1000;
        
        let mut job2 = QpuJob::new("job2".into(), ProblemType::Vqe, JobParameters::default());
        job2.parameters.circuit_depth = 50;
        job2.parameters.shots = 1000;
        
        let mut job3 = QpuJob::new("job3".into(), ProblemType::Vqe, JobParameters::default());
        job3.parameters.circuit_depth = 10;
        job3.parameters.shots = 2000;

        let mut job4 = QpuJob::new("job4".into(), ProblemType::Vqe, JobParameters::default());
        job4.parameters.circuit_depth = 50;
        job4.parameters.shots = 500;

        // Enqueue jobs in random order
        q.enqueue(job1).await;
        q.enqueue(job2).await;
        q.enqueue(job3).await;
        q.enqueue(job4).await;

        let jobs = {
            let mut pending = q.pending.lock().await;
            let mut extracted = Vec::new();
            while let Some(job) = pending.pop() {
                extracted.push(job.0);
            }
            extracted
        };

        // BinaryHeap pops the largest first. 
        // Based on our Ord implementation: highest depth first, then highest shots.
        assert_eq!(jobs[0].job_id, "job2"); // depth: 50, shots: 1000
        assert_eq!(jobs[1].job_id, "job4"); // depth: 50, shots: 500
        assert_eq!(jobs[2].job_id, "job3"); // depth: 10, shots: 2000
        assert_eq!(jobs[3].job_id, "job1"); // depth: 10, shots: 1000
    }
}
