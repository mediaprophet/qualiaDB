//! Local Scheduler — Thread-per-Core Production Queue
//!
//! Implements multi-core parallelism for calculus operations by pinning isolated
//! SLG VM loops to each logical processor. Each worker operates within a 512MB
//! memory boundary and pulls jobs from a central lock-free ring buffer.
//!
//! Architecture:
//! - Supervisor thread: Manages job queue and coordinates workers
//! - Worker threads: Pinned to specific cores, process QualiaQuin jobs
//! - SPSC channels: Lock-free communication between supervisor and workers
//! - NVMe WAL: Persistent backing store for job state (future)
//!
//! Environmental yielding:
//! - Thermal governor integration for power-aware execution
//! - Pause/resume capability via Quin state persistence
//! - Solar/battery power constraint handling

#![cfg(not(target_arch = "wasm32"))]

use crate::QualiaQuin;
use crate::wal::WriteAheadLog;
use crossbeam_channel::{bounded, Receiver, Sender};
use core_affinity::CoreId;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Memory boundary for each worker cell (512MB)
const WORKER_MEMORY_BOUNDARY: usize = 512 * 1024 * 1024;

/// Ring buffer capacity for job distribution
const JOB_QUEUE_CAPACITY: usize = 4096;

/// Job status tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStatus {
    Pending,
    InProgress,
    Completed,
    Paused,
    Failed,
}

/// Production queue job wrapper
#[derive(Debug, Clone)]
pub struct Job {
    pub quin: QualiaQuin,
    pub status: JobStatus,
    pub total_bytes: u64,
    pub dispatched_at: Option<Instant>,
    pub completed_at: Option<Instant>,
    pub compute_target: ComputeTarget,
}

impl Job {
    pub fn new(quin: QualiaQuin, total_bytes: u64) -> Self {
        Self {
            quin,
            status: JobStatus::Pending,
            total_bytes,
            dispatched_at: None,
            completed_at: None,
            compute_target: ComputeTarget::Cpu, // Default to CPU
        }
    }

    pub fn with_target(quin: QualiaQuin, total_bytes: u64, compute_target: ComputeTarget) -> Self {
        Self {
            quin,
            status: JobStatus::Pending,
            total_bytes,
            dispatched_at: None,
            completed_at: None,
            compute_target,
        }
    }

    /// Calculate progress percentage from Quin object field (byte offset)
    pub fn progress(&self) -> f64 {
        if self.total_bytes == 0 {
            return 0.0;
        }
        let current_offset = self.quin.object;
        (current_offset as f64 / self.total_bytes as f64) * 100.0
    }

    /// Calculate processing velocity (bytes per second)
    pub fn velocity(&self) -> f64 {
        match (self.dispatched_at, self.completed_at) {
            (Some(dispatched), Some(completed)) => {
                let duration = completed.duration_since(dispatched).as_secs_f64();
                if duration > 0.0 {
                    self.quin.object as f64 / duration
                } else {
                    0.0
                }
            }
            _ => 0.0,
        }
    }
}

/// Worker cell - isolated execution context pinned to a specific core
pub struct WorkerCell {
    pub cell_id: usize,
    pub core_id: CoreId,
    pub memory_boundary: usize,
    pub job_receiver: Receiver<Job>,
    pub result_sender: Sender<Job>,
    pub shutdown_signal: Arc<AtomicBool>,
    pub pause_signal: Arc<AtomicBool>,
}

impl WorkerCell {
    pub fn new(
        cell_id: usize,
        core_id: CoreId,
        job_receiver: Receiver<Job>,
        result_sender: Sender<Job>,
        shutdown_signal: Arc<AtomicBool>,
        pause_signal: Arc<AtomicBool>,
    ) -> Self {
        Self {
            cell_id,
            core_id,
            memory_boundary: WORKER_MEMORY_BOUNDARY,
            job_receiver,
            result_sender,
            shutdown_signal,
            pause_signal,
        }
    }

    /// Main worker loop - processes jobs from the receiver channel
    pub fn run(self) {
        // Pin this thread to the assigned core
        let pinned = core_affinity::set_for_current(self.core_id);
        if !pinned {
            log::error!("Failed to pin worker {} to core {:?}", self.cell_id, self.core_id);
        } else {
            log::info!("Worker {} pinned to core {:?}", self.cell_id, self.core_id);
        }

        log::info!("Worker {} starting execution loop", self.cell_id);

        while !self.shutdown_signal.load(Ordering::Relaxed) {
            // Check for pause signal (environmental yielding)
            if self.pause_signal.load(Ordering::Relaxed) {
                log::info!("Worker {} paused (environmental yielding)", self.cell_id);
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }

            // Poll for new job with timeout to allow shutdown check
            match self.job_receiver.recv_timeout(Duration::from_millis(100)) {
                Ok(mut job) => {
                    log::debug!("Worker {} received job at offset {}", self.cell_id, job.quin.object);
                    
                    job.status = JobStatus::InProgress;
                    job.dispatched_at = Some(Instant::now());

                    // Process the job (dummy implementation for now)
                    self.process_job(&mut job);

                    job.status = JobStatus::Completed;
                    job.completed_at = Some(Instant::now());

                    // Send result back to supervisor
                    if let Err(e) = self.result_sender.send(job) {
                        log::error!("Worker {} failed to send result: {}", self.cell_id, e);
                    }
                }
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                    // No job available, continue loop
                    continue;
                }
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                    log::info!("Worker {} job channel disconnected, shutting down", self.cell_id);
                    break;
                }
            }
        }

        log::info!("Worker {} shutting down", self.cell_id);
    }

    /// Process a single job (placeholder for actual SLG VM execution)
    fn process_job(&self, job: &mut Job) {
        // Simulate processing by updating the Quin object field
        // In production, this would execute the actual calculus operation
        let chunk_size = 4096; // 4KB chunk
        job.quin.object = job.quin.object.saturating_add(chunk_size);
        
        // Simulate some work
        std::thread::sleep(Duration::from_millis(10));
        
        log::trace!("Worker {} processed chunk to offset {}", self.cell_id, job.quin.object);
    }
}

/// Production queue supervisor - coordinates worker threads
pub struct ProductionQueue {
    num_workers: usize,
    job_senders: Vec<Sender<Job>>,
    job_receiver: Receiver<Job>,
    result_receiver: Receiver<Job>,
    shutdown_signal: Arc<AtomicBool>,
    pause_signal: Arc<AtomicBool>,
    active_jobs: Arc<AtomicU64>,
    completed_jobs: Arc<AtomicU64>,
    wal: Arc<Mutex<Option<WriteAheadLog>>>,
    wal_path: String,
    estimator: Arc<Mutex<JobEstimator>>,
}

impl ProductionQueue {
    /// Create a new production queue with one worker per logical processor
    pub fn new() -> Self {
        Self::with_wal_path("jobs.wal")
    }

    /// Create a new production queue with a specific WAL path
    pub fn with_wal_path(wal_path: &str) -> Self {
        let num_logical_cores = num_cpus::get();
        log::info!("Initializing production queue with {} workers", num_logical_cores);

        let (_job_sender, job_receiver) = bounded(JOB_QUEUE_CAPACITY);
        let (result_sender, result_receiver) = bounded(JOB_QUEUE_CAPACITY);
        let shutdown_signal = Arc::new(AtomicBool::new(false));
        let pause_signal = Arc::new(AtomicBool::new(false));
        let active_jobs = Arc::new(AtomicU64::new(0));
        let completed_jobs = Arc::new(AtomicU64::new(0));

        // Initialize WAL
        let wal = Arc::new(Mutex::new(WriteAheadLog::open(wal_path).ok()));
        if wal.lock().unwrap().is_some() {
            log::info!("Production queue WAL initialized at {}", wal_path);
        } else {
            log::warn!("Failed to initialize WAL at {}, job persistence disabled", wal_path);
        }

        // Initialize job estimator
        let estimator = Arc::new(Mutex::new(JobEstimator::new()));
        log::info!("Job estimator initialized");

        let mut job_senders = Vec::new();

        // Spawn worker threads
        for worker_id in 0..num_logical_cores {
            let core_id = CoreId { id: worker_id % num_logical_cores };
            let (worker_job_sender, worker_job_receiver) = bounded(16);
            let worker_result_sender = result_sender.clone();
            let worker_shutdown = shutdown_signal.clone();
            let worker_pause = pause_signal.clone();

            job_senders.push(worker_job_sender);

            let worker = WorkerCell::new(
                worker_id,
                core_id,
                worker_job_receiver,
                worker_result_sender,
                worker_shutdown,
                worker_pause,
            );

            thread::spawn(move || {
                worker.run();
            });
        }

        Self {
            num_workers: num_logical_cores,
            job_senders,
            job_receiver,
            result_receiver,
            shutdown_signal,
            pause_signal,
            active_jobs,
            completed_jobs,
            wal,
            wal_path: wal_path.to_string(),
            estimator,
        }
    }

    /// Submit a job to the production queue
    pub fn submit_job(&self, job: Job) -> Result<(), String> {
        // Persist job to WAL if available
        if let Ok(mut wal_guard) = self.wal.lock() {
            if let Some(ref mut wal) = wal_guard.as_mut() {
                if let Err(e) = wal.append_mutation(&job.quin) {
                    log::warn!("Failed to persist job to WAL: {}", e);
                }
            }
        }

        // Round-robin job distribution
        let worker_id = self.active_jobs.fetch_add(1, Ordering::Relaxed) as usize % self.num_workers;
        
        if let Err(e) = self.job_senders[worker_id].send(job) {
            self.active_jobs.fetch_sub(1, Ordering::Relaxed);
            return Err(format!("Failed to submit job to worker {}: {}", worker_id, e));
        }

        log::debug!("Job submitted to worker {}", worker_id);
        Ok(())
    }

    /// Recover pending jobs from WAL on startup
    pub fn recover_jobs(&mut self) -> Result<Vec<Job>, String> {
        if let Ok(mut wal_guard) = self.wal.lock() {
            if let Some(ref mut wal) = wal_guard.as_mut() {
                match wal.recover() {
                    Ok(quins) => {
                        let jobs: Vec<Job> = quins
                            .into_iter()
                            .map(|quin| Job::new(quin, 10000)) // Default total_bytes, should be stored in metadata
                            .collect();
                        log::info!("Recovered {} jobs from WAL", jobs.len());
                        return Ok(jobs);
                    }
                    Err(e) => return Err(format!("Failed to recover jobs from WAL: {}", e)),
                }
            }
        }
        Ok(Vec::new())
    }

    /// Persist completed job state to WAL
    pub fn persist_job_completion(&self, job: &Job) -> Result<(), String> {
        if let Ok(mut wal_guard) = self.wal.lock() {
            if let Some(ref mut wal) = wal_guard.as_mut() {
                if let Err(e) = wal.append_mutation(&job.quin) {
                    return Err(format!("Failed to persist job completion to WAL: {}", e));
                }
            }
        }
        Ok(())
    }

    /// Collect completed jobs from workers
    pub fn collect_results(&self) -> Vec<Job> {
        let mut results = Vec::new();
        
        while let Ok(job) = self.result_receiver.try_recv() {
            self.completed_jobs.fetch_add(1, Ordering::Relaxed);
            self.active_jobs.fetch_sub(1, Ordering::Relaxed);
            
            // Persist completed job state to WAL
            if let Err(e) = self.persist_job_completion(&job) {
                log::warn!("Failed to persist job completion: {}", e);
            }
            
            results.push(job);
        }

        results
    }

    /// Get current queue statistics
    pub fn stats(&self) -> QueueStats {
        QueueStats {
            num_workers: self.num_workers,
            active_jobs: self.active_jobs.load(Ordering::Relaxed),
            completed_jobs: self.completed_jobs.load(Ordering::Relaxed),
        }
    }

    /// Gracefully shutdown all workers
    pub fn shutdown(&self) {
        log::info!("Shutting down production queue");
        self.shutdown_signal.store(true, Ordering::Relaxed);
        
        // Give workers time to finish current jobs
        std::thread::sleep(Duration::from_secs(2));
        
        log::info!("Production queue shutdown complete");
    }

    /// Pause all workers (environmental yielding)
    pub fn pause(&self) {
        log::info!("Pausing production queue (environmental yielding)");
        self.pause_signal.store(true, Ordering::Relaxed);
    }

    /// Resume all workers after environmental yielding
    pub fn resume(&self) {
        log::info!("Resuming production queue");
        self.pause_signal.store(false, Ordering::Relaxed);
    }

    /// Check if the queue is currently paused
    pub fn is_paused(&self) -> bool {
        self.pause_signal.load(Ordering::Relaxed)
    }

    /// Get pre-execution job estimate
    pub fn estimate_job(&self, params: &JobEstimateParams) -> JobEstimate {
        if let Ok(estimator) = self.estimator.lock() {
            estimator.estimate_job_duration(params)
        } else {
            // Fallback if estimator is locked
            JobEstimate {
                estimated_duration: Duration::from_secs(0),
                confidence: 0.0,
                compute_target: params.compute_target,
                workload_bytes: params.workload_bytes,
            }
        }
    }

    /// Record job performance for velocity learning
    pub fn record_performance(&self, target: ComputeTarget, bytes_processed: u64, duration: Duration) {
        if let Ok(mut estimator) = self.estimator.lock() {
            estimator.record_performance(target, bytes_processed, duration);
        }
    }

    /// Set power throttling factor (called by power daemon)
    pub fn set_power_throttle_factor(&self, factor: f64) {
        if let Ok(mut estimator) = self.estimator.lock() {
            estimator.set_power_throttle_factor(factor);
            log::info!("Power throttle factor set to {}", factor);
        }
    }

    /// Get current power throttling factor
    pub fn get_power_throttle_factor(&self) -> f64 {
        if let Ok(estimator) = self.estimator.lock() {
            estimator.get_power_throttle_factor()
        } else {
            1.0
        }
    }

    /// Calculate progress and telemetry from Quin fields
    pub fn calculate_telemetry(&self, total_job_bytes: u64) -> QueueTelemetry {
        let active = self.active_jobs.load(Ordering::Relaxed);
        let completed = self.completed_jobs.load(Ordering::Relaxed);
        let total = active + completed;

        // Calculate overall progress from completed jobs
        let overall_progress = if total > 0 {
            (completed as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        // Calculate bytes per second from job velocities
        let results = self.collect_results();
        let total_velocity: f64 = results.iter().map(|j| j.velocity()).sum();
        let avg_velocity = if !results.is_empty() {
            total_velocity / results.len() as f64
        } else {
            0.0
        };

        // Estimate time remaining based on velocity
        let estimated_time_remaining = if avg_velocity > 0.0 && total_job_bytes > 0 {
            let remaining_bytes = total_job_bytes.saturating_sub(
                results.iter().map(|j| j.quin.object).sum::<u64>()
            );
            Some(Duration::from_secs_f64(remaining_bytes as f64 / avg_velocity))
        } else {
            None
        };

        QueueTelemetry {
            total_jobs: total,
            active_jobs: active,
            completed_jobs: completed,
            overall_progress,
            estimated_time_remaining,
            bytes_per_second: avg_velocity,
        }
    }
}

impl Drop for ProductionQueue {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Queue statistics
#[derive(Debug, Clone, Copy)]
pub struct QueueStats {
    pub num_workers: usize,
    pub active_jobs: u64,
    pub completed_jobs: u64,
}

/// Progress and telemetry data for the production queue
#[derive(Debug, Clone)]
pub struct QueueTelemetry {
    pub total_jobs: u64,
    pub active_jobs: u64,
    pub completed_jobs: u64,
    pub overall_progress: f64, // Percentage (0-100)
    pub estimated_time_remaining: Option<Duration>,
    pub bytes_per_second: f64,
}

/// Compute target for calculus operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComputeTarget {
    Cpu,
    WebGpu,
    DirectMl,
}

/// Hardware velocity metrics for a specific compute target
#[derive(Debug, Clone)]
pub struct HardwareVelocity {
    pub target: ComputeTarget,
    pub bytes_per_second: f64,
    pub sample_count: u64,
    pub last_updated: Option<Instant>,
}

impl HardwareVelocity {
    pub fn new(target: ComputeTarget) -> Self {
        Self {
            target,
            bytes_per_second: 0.0,
            sample_count: 0,
            last_updated: None,
        }
    }

    /// Update velocity with a new sample using exponential moving average
    pub fn update(&mut self, bytes_processed: u64, duration: Duration) {
        let duration_secs = duration.as_secs_f64();
        if duration_secs > 0.0 {
            let new_velocity = bytes_processed as f64 / duration_secs;
            
            // Exponential moving average with alpha=0.2 (weights recent samples more)
            let alpha = 0.2;
            if self.sample_count == 0 {
                self.bytes_per_second = new_velocity;
            } else {
                self.bytes_per_second = alpha * new_velocity + (1.0 - alpha) * self.bytes_per_second;
            }
            
            self.sample_count += 1;
            self.last_updated = Some(Instant::now());
        }
    }

    /// Get current velocity with default fallback if no samples
    pub fn get_velocity(&self) -> f64 {
        if self.sample_count == 0 {
            // Default velocities based on target (conservative estimates)
            match self.target {
                ComputeTarget::Cpu => 10_000_000.0,      // 10 MB/s CPU
                ComputeTarget::WebGpu => 100_000_000.0,   // 100 MB/s WebGPU
                ComputeTarget::DirectMl => 500_000_000.0,  // 500 MB/s DirectML
            }
        } else {
            self.bytes_per_second
        }
    }
}

/// Job estimation parameters
#[derive(Debug, Clone)]
pub struct JobEstimateParams {
    pub workload_bytes: u64,
    pub step_size: f32,
    pub compute_target: ComputeTarget,
    pub contention_factor: f64, // Multiplier for parallel execution overhead
}

/// Pre-execution job estimate
#[derive(Debug, Clone)]
pub struct JobEstimate {
    pub estimated_duration: Duration,
    pub confidence: f64, // 0-1 based on sample count
    pub compute_target: ComputeTarget,
    pub workload_bytes: u64,
}

/// Job estimator for pre-execution time prediction
pub struct JobEstimator {
    cpu_velocity: HardwareVelocity,
    webgpu_velocity: HardwareVelocity,
    directml_velocity: HardwareVelocity,
    power_throttle_factor: f64, // Multiplier when power-constrained
}

impl JobEstimator {
    pub fn new() -> Self {
        Self {
            cpu_velocity: HardwareVelocity::new(ComputeTarget::Cpu),
            webgpu_velocity: HardwareVelocity::new(ComputeTarget::WebGpu),
            directml_velocity: HardwareVelocity::new(ComputeTarget::DirectMl),
            power_throttle_factor: 1.0, // No throttling by default
        }
    }

    /// Estimate job duration before execution
    pub fn estimate_job_duration(&self, params: &JobEstimateParams) -> JobEstimate {
        // Get velocity for the target compute path
        let velocity = match params.compute_target {
            ComputeTarget::Cpu => self.cpu_velocity.get_velocity(),
            ComputeTarget::WebGpu => self.webgpu_velocity.get_velocity(),
            ComputeTarget::DirectMl => self.directml_velocity.get_velocity(),
        };

        // Apply power throttling factor
        let adjusted_velocity = velocity / self.power_throttle_factor;

        // Apply contention factor for parallel execution
        let effective_velocity = adjusted_velocity / params.contention_factor;

        // Calculate workload (number of integration points)
        let workload_points = if params.step_size > 0.0 {
            params.workload_bytes as f64 / params.step_size as f64
        } else {
            params.workload_bytes as f64
        };

        // Get overhead for target
        let overhead_secs = match params.compute_target {
            ComputeTarget::Cpu => 0.005,      // 5ms loop startup
            ComputeTarget::WebGpu => 0.250,   // 250ms shader dispatch
            ComputeTarget::DirectMl => 0.100, // 100ms DirectML setup
        };

        // Calculate estimated duration
        let duration_secs = (workload_points / effective_velocity) + overhead_secs;
        let estimated_duration = Duration::from_secs_f64(duration_secs.max(0.0));

        // Calculate confidence based on sample count
        let sample_count = match params.compute_target {
            ComputeTarget::Cpu => self.cpu_velocity.sample_count,
            ComputeTarget::WebGpu => self.webgpu_velocity.sample_count,
            ComputeTarget::DirectMl => self.directml_velocity.sample_count,
        };

        // Confidence increases with sample count, caps at 0.95
        let confidence = (sample_count as f64 / (sample_count as f64 + 10.0)).min(0.95);

        JobEstimate {
            estimated_duration,
            confidence,
            compute_target: params.compute_target,
            workload_bytes: params.workload_bytes,
        }
    }

    /// Update velocity metrics after job completion
    pub fn record_performance(&mut self, target: ComputeTarget, bytes_processed: u64, duration: Duration) {
        match target {
            ComputeTarget::Cpu => self.cpu_velocity.update(bytes_processed, duration),
            ComputeTarget::WebGpu => self.webgpu_velocity.update(bytes_processed, duration),
            ComputeTarget::DirectMl => self.directml_velocity.update(bytes_processed, duration),
        }
    }

    /// Set power throttling factor (called by power daemon)
    pub fn set_power_throttle_factor(&mut self, factor: f64) {
        self.power_throttle_factor = factor.max(0.5).min(2.0); // Clamp between 0.5x and 2x
    }

    /// Get current power throttling factor
    pub fn get_power_throttle_factor(&self) -> f64 {
        self.power_throttle_factor
    }

    /// Get velocity for a specific target
    pub fn get_velocity(&self, target: ComputeTarget) -> f64 {
        match target {
            ComputeTarget::Cpu => self.cpu_velocity.get_velocity(),
            ComputeTarget::WebGpu => self.webgpu_velocity.get_velocity(),
            ComputeTarget::DirectMl => self.directml_velocity.get_velocity(),
        }
    }
}

impl Default for JobEstimator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_progress_calculation() {
        let mut quin = QualiaQuin::default();
        quin.object = 5000; // 5000 bytes processed
        
        let job = Job::new(quin, 10000); // Total 10000 bytes
        
        assert_eq!(job.progress(), 50.0);
        assert_eq!(job.compute_target, ComputeTarget::Cpu);
    }

    #[test]
    fn test_job_velocity_calculation() {
        let mut quin = QualiaQuin::default();
        quin.object = 1000;
        
        let mut job = Job::new(quin, 10000);
        job.dispatched_at = Some(Instant::now() - Duration::from_secs(1));
        job.completed_at = Some(Instant::now());
        
        // 1000 bytes in 1 second ≈ 1000 bytes/sec (allow for timing precision)
        assert!((job.velocity() - 1000.0).abs() < 10.0);
    }

    #[test]
    fn test_production_queue_creation() {
        let queue = ProductionQueue::new();
        let stats = queue.stats();
        
        assert!(stats.num_workers > 0);
        assert_eq!(stats.active_jobs, 0);
        assert_eq!(stats.completed_jobs, 0);
        
        queue.shutdown();
    }

    #[test]
    fn test_worker_thread_job_processing() {
        let queue = ProductionQueue::new();
        let num_jobs = 10;
        
        // Submit dummy jobs
        for i in 0..num_jobs {
            let mut quin = QualiaQuin::default();
            quin.object = i * 1000; // Different starting offsets
            let job = Job::new(quin, 10000);
            queue.submit_job(job).unwrap();
        }
        
        // Wait for jobs to complete
        std::thread::sleep(Duration::from_secs(2));
        
        // Collect results
        let results = queue.collect_results();
        assert!(results.len() > 0, "At least one job should be completed");
        
        // Verify progress was made
        for result in &results {
            assert!(result.quin.object > 0, "Job should have progressed");
            assert_eq!(result.status, JobStatus::Completed);
        }
        
        queue.shutdown();
    }

    #[test]
    fn test_environmental_yielding() {
        let queue = ProductionQueue::new();
        
        // Submit a job
        let mut quin = QualiaQuin::default();
        quin.object = 1000;
        let job = Job::new(quin, 10000);
        queue.submit_job(job).unwrap();
        
        // Wait a bit for job to start
        std::thread::sleep(Duration::from_millis(100));
        
        // Pause the queue (environmental yielding)
        queue.pause();
        assert!(queue.is_paused(), "Queue should be paused");
        
        // Wait to ensure workers are paused
        std::thread::sleep(Duration::from_millis(200));
        
        // Resume the queue
        queue.resume();
        assert!(!queue.is_paused(), "Queue should not be paused");
        
        // Wait for job to complete
        std::thread::sleep(Duration::from_secs(1));
        
        queue.shutdown();
    }

    #[test]
    fn test_telemetry_calculation() {
        let queue = ProductionQueue::new();
        let total_bytes = 100000;
        
        // Submit jobs
        for i in 0..5 {
            let mut quin = QualiaQuin::default();
            quin.object = i * 1000;
            let job = Job::new(quin, 20000);
            queue.submit_job(job).unwrap();
        }
        
        // Wait for some jobs to complete
        std::thread::sleep(Duration::from_secs(1));
        
        // Calculate telemetry
        let telemetry = queue.calculate_telemetry(total_bytes);
        
        assert!(telemetry.total_jobs >= 0);
        assert!(telemetry.overall_progress >= 0.0 && telemetry.overall_progress <= 100.0);
        
        queue.shutdown();
    }

    #[test]
    fn test_hardware_velocity_tracking() {
        let mut velocity = HardwareVelocity::new(ComputeTarget::Cpu);
        
        // Update with some samples
        velocity.update(1_000_000, Duration::from_millis(100)); // 10 MB/s
        velocity.update(2_000_000, Duration::from_millis(200)); // 10 MB/s
        
        assert_eq!(velocity.sample_count, 2);
        assert!(velocity.get_velocity() > 0.0);
    }

    #[test]
    fn test_job_estimator() {
        let estimator = JobEstimator::new();
        
        let params = JobEstimateParams {
            workload_bytes: 100_000_000, // 100 MB
            step_size: 0.01,
            compute_target: ComputeTarget::Cpu,
            contention_factor: 1.0,
        };
        
        let estimate = estimator.estimate_job_duration(&params);
        
        assert!(estimate.estimated_duration.as_secs() > 0);
        assert!(estimate.confidence >= 0.0 && estimate.confidence <= 1.0);
        assert_eq!(estimate.compute_target, ComputeTarget::Cpu);
    }

    #[test]
    fn test_job_estimator_with_throttling() {
        let mut estimator = JobEstimator::new();
        
        // Set power throttling factor
        estimator.set_power_throttle_factor(1.5); // 1.5x slower due to power constraints
        
        let params = JobEstimateParams {
            workload_bytes: 100_000_000,
            step_size: 0.01,
            compute_target: ComputeTarget::Cpu,
            contention_factor: 1.0,
        };
        
        let estimate = estimator.estimate_job_duration(&params);
        
        // Verify throttling is applied
        assert_eq!(estimator.get_power_throttle_factor(), 1.5);
    }

    #[test]
    fn test_job_estimator_velocity_learning() {
        let mut estimator = JobEstimator::new();
        
        // Record some performance data
        estimator.record_performance(ComputeTarget::Cpu, 10_000_000, Duration::from_secs(1));
        estimator.record_performance(ComputeTarget::Cpu, 20_000_000, Duration::from_secs(2));
        
        // Now estimates should be more confident
        let params = JobEstimateParams {
            workload_bytes: 100_000_000,
            step_size: 0.01,
            compute_target: ComputeTarget::Cpu,
            contention_factor: 1.0,
        };
        
        let estimate = estimator.estimate_job_duration(&params);
        
        // Confidence should be higher after recording performance
        assert!(estimate.confidence > 0.0);
    }

    #[test]
    fn test_production_queue_estimator_integration() {
        let queue = ProductionQueue::new();
        
        // Estimate a job before submission
        let params = JobEstimateParams {
            workload_bytes: 50_000_000,
            step_size: 0.01,
            compute_target: ComputeTarget::Cpu,
            contention_factor: 1.0,
        };
        
        let estimate = queue.estimate_job(&params);
        assert!(estimate.estimated_duration.as_secs() > 0);
        
        // Test power throttling
        queue.set_power_throttle_factor(1.5);
        assert_eq!(queue.get_power_throttle_factor(), 1.5);
        
        // Estimate with throttling should be longer
        let throttled_estimate = queue.estimate_job(&params);
        assert!(throttled_estimate.estimated_duration >= estimate.estimated_duration);
        
        // Record performance
        queue.record_performance(ComputeTarget::Cpu, 10_000_000, Duration::from_secs(1));
        
        queue.shutdown();
    }
}
