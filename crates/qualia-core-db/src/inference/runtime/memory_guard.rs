//! Fail-closed host-memory pressure policy for native inference.
//!
//! Platform probes provide committed and available-memory samples; this module
//! makes the decision deterministic. It deliberately never tries to force a
//! giant working set resident: that is not a Windows residency guarantee and
//! can make system-wide pressure worse.

/// A point-in-time sample supplied by a platform monitor.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HostMemorySample {
    /// Immediately available host memory, not virtual address-space size.
    pub available_host_bytes: u64,
    /// Commit attributed to the supervised inference process/job.
    pub inference_commit_bytes: u64,
    /// Number of active inference requests at this sample point.
    pub active_requests: u32,
}

/// Cold-configured host-memory thresholds for one native runtime.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryPressurePolicy {
    /// Below this, no new prefill/decode requests may be admitted.
    pub stop_admission_available_bytes: u64,
    /// Below this, active work must drain at a token boundary.
    pub drain_available_bytes: u64,
    /// Commit ceiling for the supervised inference job/process.
    pub max_inference_commit_bytes: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryPolicyError {
    ZeroThreshold,
    InvalidThresholdOrder,
}

impl MemoryPressurePolicy {
    pub const fn new(
        stop_admission_available_bytes: u64,
        drain_available_bytes: u64,
        max_inference_commit_bytes: u64,
    ) -> Result<Self, MemoryPolicyError> {
        if stop_admission_available_bytes == 0
            || drain_available_bytes == 0
            || max_inference_commit_bytes == 0
        {
            return Err(MemoryPolicyError::ZeroThreshold);
        }
        if drain_available_bytes >= stop_admission_available_bytes {
            return Err(MemoryPolicyError::InvalidThresholdOrder);
        }
        Ok(Self {
            stop_admission_available_bytes,
            drain_available_bytes,
            max_inference_commit_bytes,
        })
    }
}

/// Required reaction before an allocation failure or OS-level instability.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryPressureAction {
    Continue,
    /// Reject new work and release only reconstructible caches outside active
    /// request state (for example, cold prefix/P﻿LE neighbourhood entries).
    StopAdmissionAndShedCaches,
    /// Stop admission, drain at token boundaries, checkpoint compatible state,
    /// then release the model/runtime allocation through normal lifecycle code.
    DrainAndCheckpoint,
}

impl MemoryPressureAction {
    pub const fn blocks_admission(self) -> bool {
        !matches!(self, Self::Continue)
    }

    pub const fn requires_drain(self) -> bool {
        matches!(self, Self::DrainAndCheckpoint)
    }
}

/// Pure, zero-allocation pressure evaluator.
#[derive(Clone, Copy, Debug)]
pub struct MemoryPressureGuard {
    policy: MemoryPressurePolicy,
}

impl MemoryPressureGuard {
    pub const fn new(policy: MemoryPressurePolicy) -> Self {
        Self { policy }
    }

    pub const fn policy(&self) -> MemoryPressurePolicy {
        self.policy
    }

    pub const fn evaluate(&self, sample: HostMemorySample) -> MemoryPressureAction {
        if sample.available_host_bytes <= self.policy.drain_available_bytes
            || sample.inference_commit_bytes >= self.policy.max_inference_commit_bytes
        {
            MemoryPressureAction::DrainAndCheckpoint
        } else if sample.available_host_bytes <= self.policy.stop_admission_available_bytes {
            MemoryPressureAction::StopAdmissionAndShedCaches
        } else {
            MemoryPressureAction::Continue
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GIB: u64 = 1024 * 1024 * 1024;

    #[test]
    fn transitions_before_host_memory_is_exhausted() {
        let policy = MemoryPressurePolicy::new(8 * GIB, 4 * GIB, 52 * GIB).unwrap();
        let guard = MemoryPressureGuard::new(policy);

        assert_eq!(
            guard.evaluate(HostMemorySample {
                available_host_bytes: 12 * GIB,
                inference_commit_bytes: 30 * GIB,
                active_requests: 1,
            }),
            MemoryPressureAction::Continue
        );
        assert_eq!(
            guard.evaluate(HostMemorySample {
                available_host_bytes: 7 * GIB,
                inference_commit_bytes: 30 * GIB,
                active_requests: 1,
            }),
            MemoryPressureAction::StopAdmissionAndShedCaches
        );
        assert_eq!(
            guard.evaluate(HostMemorySample {
                available_host_bytes: 3 * GIB,
                inference_commit_bytes: 30 * GIB,
                active_requests: 1,
            }),
            MemoryPressureAction::DrainAndCheckpoint
        );
    }

    #[test]
    fn commit_ceiling_drains_even_when_available_memory_looks_healthy() {
        let policy = MemoryPressurePolicy::new(8, 4, 100).unwrap();
        let guard = MemoryPressureGuard::new(policy);
        assert!(guard
            .evaluate(HostMemorySample {
                available_host_bytes: 1000,
                inference_commit_bytes: 100,
                active_requests: 1,
            })
            .requires_drain());
    }
}
