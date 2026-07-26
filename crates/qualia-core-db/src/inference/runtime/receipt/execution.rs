use serde::{Deserialize, Serialize};

pub const RECEIPT_SCHEMA_VERSION: u16 = 2;
pub const COUNTER_DECODE_STEPS: u64 = 1 << 0;
pub const COUNTER_GRAPH_LAUNCHES: u64 = 1 << 1;
pub const COUNTER_COMPUTE_DISPATCHES: u64 = 1 << 2;
pub const COUNTER_DEVICE_FENCES: u64 = 1 << 3;
pub const COUNTER_HOST_TO_DEVICE_BYTES: u64 = 1 << 4;
pub const COUNTER_DEVICE_TO_HOST_BYTES: u64 = 1 << 5;
pub const COUNTER_FALLBACKS: u64 = 1 << 6;
pub const COUNTER_HOT_ALLOCATIONS: u64 = 1 << 7;
pub const COUNTER_COMPILE_CALLS: u64 = 1 << 8;
pub const COUNTER_IMMUTABLE_UPLOAD_BYTES: u64 = 1 << 9;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackendKind {
    Cpu,
    WgpuDx12,
    WgpuVulkan,
    WgpuMetal,
    Cuda,
    Metal,
    Unknown,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionCounters {
    pub decode_steps: u64,
    pub graph_launches: u64,
    pub compute_dispatches: u64,
    pub device_fences: u64,
    pub host_to_device_bytes: u64,
    pub device_to_host_bytes: u64,
    pub fallback_count: u64,
    pub hot_path_allocations: u64,
    pub compile_calls: u64,
    pub immutable_upload_bytes: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactCleanupCounters {
    pub temp_created_bytes: u64,
    pub temp_removed_bytes: u64,
    pub temp_retained_bytes: u64,
    pub temp_cleanup_failures: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionReceipt {
    pub schema_version: u16,
    pub requested_backend: BackendKind,
    pub executed_backend: BackendKind,
    pub model_instance_id: String,
    pub prepared_plan_id: String,
    pub graph_hash: String,
    /// Cold backend tuning record. Empty for runtimes without an explicit tuning profile.
    #[serde(default)]
    pub tuning_profile: String,
    pub stop_reason: String,
    /// Bit set means the corresponding [`ExecutionCounters`] value was measured or proven.
    /// An unset bit distinguishes "unknown" from a measured zero.
    pub counter_coverage: u64,
    pub counters: ExecutionCounters,
    pub artifacts: ArtifactCleanupCounters,
}

impl ExecutionReceipt {
    pub fn new(
        requested_backend: BackendKind,
        executed_backend: BackendKind,
        model_instance_id: impl Into<String>,
        prepared_plan_id: impl Into<String>,
    ) -> Self {
        Self {
            schema_version: RECEIPT_SCHEMA_VERSION,
            requested_backend,
            executed_backend,
            model_instance_id: model_instance_id.into(),
            prepared_plan_id: prepared_plan_id.into(),
            graph_hash: String::new(),
            tuning_profile: String::new(),
            stop_reason: String::new(),
            counter_coverage: 0,
            counters: ExecutionCounters::default(),
            artifacts: ArtifactCleanupCounters::default(),
        }
    }

    pub fn backend_matches_request(&self) -> bool {
        self.requested_backend == self.executed_backend && self.counters.fallback_count == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipt_round_trips_and_rejects_fallback_as_match() {
        let mut receipt =
            ExecutionReceipt::new(BackendKind::Cuda, BackendKind::Cuda, "model-1", "plan-1");
        receipt.counters.decode_steps = 256;
        let json = serde_json::to_string(&receipt).unwrap();
        let decoded: ExecutionReceipt = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, receipt);
        assert!(decoded.backend_matches_request());

        receipt.counters.fallback_count = 1;
        assert!(!receipt.backend_matches_request());
    }

    #[test]
    fn schema_one_receipt_without_tuning_profile_remains_readable() {
        let receipt =
            ExecutionReceipt::new(BackendKind::Cuda, BackendKind::Cuda, "model-1", "plan-1");
        let mut value = serde_json::to_value(receipt).unwrap();
        value["schema_version"] = serde_json::json!(1);
        value
            .as_object_mut()
            .unwrap()
            .remove("tuning_profile")
            .unwrap();

        let decoded: ExecutionReceipt = serde_json::from_value(value).unwrap();
        assert_eq!(decoded.schema_version, 1);
        assert!(decoded.tuning_profile.is_empty());
    }
}
