//! Measured CPU/GPU placement policy for streamed MoE experts.
//!
//! Calibration is cold-path work.  Decode consumes this immutable, scalar policy without
//! allocating, so every cache-miss decision is comparable and auditable for one run.

/// Measured timings used to choose CPU evaluation or a GPU staging transfer.
///
/// All rates describe the same expert representation and batch shape.  A zero rate means that
/// lane was not measured and is never selected speculatively.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ExpertPlacementCalibration {
    /// Sustained host-to-device transfer rate for compressed expert bytes.
    pub transfer_bytes_per_second: u64,
    /// CPU evaluation rate for one routed expert, expressed as experts per second.
    pub cpu_experts_per_second: f32,
    /// GPU evaluation rate after an expert is resident, expressed as experts per second.
    pub gpu_experts_per_second: f32,
    /// Fixed submission/synchronisation cost measured for this backend and batch shape.
    pub gpu_launch_overhead_ns: u64,
    /// Largest safely overlapped transfer queue observed by the calibrator.
    pub max_inflight_transfers: u16,
}

/// Frozen placement policy used for all expert-cache misses in a generation run.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExpertPlacementPolicy {
    calibration: ExpertPlacementCalibration,
    /// A non-zero expected reuse count permits an initial fetch to amortise its transfer.
    pub expected_reuse: u16,
}

impl Default for ExpertPlacementPolicy {
    fn default() -> Self {
        Self {
            calibration: ExpertPlacementCalibration::default(),
            expected_reuse: 1,
        }
    }
}

impl ExpertPlacementPolicy {
    /// Freeze a policy from a completed calibration.  Reuse is clamped so malformed telemetry
    /// cannot create an unbounded or divide-by-zero decision.
    pub fn from_calibration(calibration: ExpertPlacementCalibration, expected_reuse: u16) -> Self {
        Self {
            calibration,
            expected_reuse: expected_reuse.max(1),
        }
    }

    pub fn calibration(&self) -> ExpertPlacementCalibration {
        self.calibration
    }

    /// True when a cache miss should execute on CPU rather than enqueue a GPU transfer.
    ///
    /// GPU cost is transfer + launch + resident evaluation, amortised by expected reuse.  Queue
    /// depth scales transfer time but is bounded by the observed safe overlap; beyond that the
    /// request stays on CPU.  Missing measurements fail closed to GPU staging, preserving the
    /// existing manager behaviour rather than inventing a throughput figure.
    pub fn prefer_cpu(
        &self,
        bytes_per_expert: usize,
        queue_depth: usize,
        allow_cpu_hybrid: bool,
    ) -> bool {
        if !allow_cpu_hybrid {
            return false;
        }
        let c = self.calibration;
        if c.transfer_bytes_per_second == 0
            || c.cpu_experts_per_second <= 0.0
            || c.gpu_experts_per_second <= 0.0
        {
            return false;
        }
        if c.max_inflight_transfers == 0 || queue_depth >= c.max_inflight_transfers as usize {
            return true;
        }

        let cpu_ns = 1_000_000_000.0f64 / c.cpu_experts_per_second as f64;
        let transfer_ns =
            (bytes_per_expert as f64 * 1_000_000_000.0) / c.transfer_bytes_per_second as f64;
        let queued_transfer_ns = transfer_ns * (queue_depth as f64 + 1.0);
        let gpu_eval_ns = 1_000_000_000.0f64 / c.gpu_experts_per_second as f64;
        let gpu_total_ns = (queued_transfer_ns + c.gpu_launch_overhead_ns as f64 + gpu_eval_ns)
            / self.expected_reuse as f64;
        cpu_ns <= gpu_total_ns
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn calibrated() -> ExpertPlacementPolicy {
        ExpertPlacementPolicy::from_calibration(
            ExpertPlacementCalibration {
                transfer_bytes_per_second: 1_000_000_000,
                cpu_experts_per_second: 20.0,
                gpu_experts_per_second: 1_000.0,
                gpu_launch_overhead_ns: 100_000,
                max_inflight_transfers: 4,
            },
            1,
        )
    }

    #[test]
    fn single_use_large_transfer_uses_cpu() {
        assert!(calibrated().prefer_cpu(240 * 1024 * 1024, 0, true));
    }

    #[test]
    fn reusable_small_transfer_uses_gpu() {
        let policy = ExpertPlacementPolicy::from_calibration(calibrated().calibration(), 32);
        assert!(!policy.prefer_cpu(1024, 0, true));
    }

    #[test]
    fn saturated_transfer_queue_uses_cpu() {
        assert!(calibrated().prefer_cpu(1024, 4, true));
    }
}
