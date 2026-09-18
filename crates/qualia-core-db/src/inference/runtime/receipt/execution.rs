use serde::{Deserialize, Serialize};

pub const RECEIPT_SCHEMA_VERSION: u16 = 3;
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

// Schema 3 coverage bits (Work Package F8)
#[allow(dead_code)]
pub const COUNTER_SUBMITTED_PREFILL_TOKENS: u64 = 1 << 10;
#[allow(dead_code)]
pub const COUNTER_COMMITTED_PREFILL_TOKENS: u64 = 1 << 11;
#[allow(dead_code)]
pub const COUNTER_CACHED_PREFILL_TOKENS: u64 = 1 << 12;
#[allow(dead_code)]
pub const COUNTER_QUEUED_REQUESTS: u64 = 1 << 13;
#[allow(dead_code)]
pub const COUNTER_ADMITTED_REQUESTS: u64 = 1 << 14;
#[allow(dead_code)]
pub const COUNTER_COW_COPIES: u64 = 1 << 15;
#[allow(dead_code)]
pub const COUNTER_EVICTIONS: u64 = 1 << 16;
#[allow(dead_code)]
pub const COUNTER_UNIQUE_FREED_PAGES: u64 = 1 << 17;
#[allow(dead_code)]
pub const COUNTER_POOL_HIGH_WATER_BYTES: u64 = 1 << 18;
#[allow(dead_code)]
pub const COUNTER_CANCELLATIONS: u64 = 1 << 19;
#[allow(dead_code)]
pub const COUNTER_GRAPH_BUCKET_TOKENS: u64 = 1 << 20;
#[allow(dead_code)]
pub const COUNTER_GRAPH_PADDING_TOKENS: u64 = 1 << 21;
#[allow(dead_code)]
pub const COUNTER_REBUILD_OUTCOMES: u64 = 1 << 22;
#[allow(dead_code)]
pub const COUNTER_QUEUE_DELAY_US: u64 = 1 << 23;
#[allow(dead_code)]
pub const COUNTER_TIME_TO_FIRST_TOKEN_US: u64 = 1 << 24;
#[allow(dead_code)]
pub const COUNTER_INTER_TOKEN_LATENCY_US: u64 = 1 << 25;

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

    // Schema 3 extensions
    #[serde(default)]
    pub submitted_prefill_tokens: u64,
    #[serde(default)]
    pub committed_prefill_tokens: u64,
    #[serde(default)]
    pub cached_prefill_tokens: u64,
    #[serde(default)]
    pub queued_requests: u64,
    #[serde(default)]
    pub admitted_requests: u64,
    #[serde(default)]
    pub cow_copies: u64,
    #[serde(default)]
    pub evictions: u64,
    #[serde(default)]
    pub unique_freed_pages: u64,
    #[serde(default)]
    pub pool_high_water_bytes: u64,
    #[serde(default)]
    pub cancellations: u64,
    #[serde(default)]
    pub graph_bucket_tokens: u64,
    #[serde(default)]
    pub graph_padding_tokens: u64,
    #[serde(default)]
    pub rebuild_outcomes: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestLatencyTelemetry {
    #[serde(default)]
    pub queue_delay_us: u64,
    #[serde(default)]
    pub time_to_first_token_us: u64,
    #[serde(default)]
    pub inter_token_latency_us: u64,
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
    #[serde(default)]
    pub latency: RequestLatencyTelemetry,
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
            latency: RequestLatencyTelemetry::default(),
            artifacts: ArtifactCleanupCounters::default(),
        }
    }

    pub fn mark_measured(&mut self, flag: u64) {
        self.counter_coverage |= flag;
    }

    pub fn is_measured(&self, flag: u64) -> bool {
        (self.counter_coverage & flag) == flag
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

    #[test]
    fn schema_3_coverage_and_latency_round_trips() {
        let mut receipt =
            ExecutionReceipt::new(BackendKind::Cuda, BackendKind::Cuda, "qwen3-35b", "plan-42");
        receipt.counters.submitted_prefill_tokens = 512;
        receipt.counters.committed_prefill_tokens = 512;
        receipt.counters.cached_prefill_tokens = 256;
        receipt.counters.admitted_requests = 1;
        receipt.counters.pool_high_water_bytes = 1024 * 1024 * 128;
        receipt.latency.queue_delay_us = 1500;
        receipt.latency.time_to_first_token_us = 35000;
        receipt.latency.inter_token_latency_us = 12000;

        receipt.mark_measured(COUNTER_SUBMITTED_PREFILL_TOKENS);
        receipt.mark_measured(COUNTER_COMMITTED_PREFILL_TOKENS);
        receipt.mark_measured(COUNTER_CACHED_PREFILL_TOKENS);
        receipt.mark_measured(COUNTER_ADMITTED_REQUESTS);
        receipt.mark_measured(COUNTER_POOL_HIGH_WATER_BYTES);
        receipt.mark_measured(COUNTER_TIME_TO_FIRST_TOKEN_US);

        assert!(receipt.is_measured(COUNTER_SUBMITTED_PREFILL_TOKENS));
        assert!(receipt.is_measured(COUNTER_TIME_TO_FIRST_TOKEN_US));
        assert!(!receipt.is_measured(COUNTER_COW_COPIES));

        let json = serde_json::to_string(&receipt).unwrap();
        let decoded: ExecutionReceipt = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, receipt);
        assert_eq!(decoded.schema_version, 3);
        assert_eq!(decoded.latency.time_to_first_token_us, 35000);
        assert_eq!(decoded.counters.cached_prefill_tokens, 256);
    }

    #[test]
    fn schema_2_receipt_deserializes_cleanly() {
        // Schema 2 JSON without Schema 3 counters or latency field
        let json_schema_2 = r#"{
            "schema_version": 2,
            "requested_backend": "cuda",
            "executed_backend": "cuda",
            "model_instance_id": "model-legacy",
            "prepared_plan_id": "plan-legacy",
            "graph_hash": "",
            "tuning_profile": "tuned-opt",
            "stop_reason": "stop_token",
            "counter_coverage": 1,
            "counters": {
                "decode_steps": 100,
                "graph_launches": 10,
                "compute_dispatches": 20,
                "device_fences": 5,
                "host_to_device_bytes": 1000,
                "device_to_host_bytes": 500,
                "fallback_count": 0,
                "hot_path_allocations": 0,
                "compile_calls": 1,
                "immutable_upload_bytes": 4096
            },
            "artifacts": {
                "temp_created_bytes": 0,
                "temp_removed_bytes": 0,
                "temp_retained_bytes": 0,
                "temp_cleanup_failures": 0
            }
        }"#;

        let decoded: ExecutionReceipt = serde_json::from_str(json_schema_2).unwrap();
        assert_eq!(decoded.schema_version, 2);
        assert_eq!(decoded.counters.decode_steps, 100);
        assert_eq!(decoded.counters.submitted_prefill_tokens, 0);
        assert_eq!(decoded.latency.time_to_first_token_us, 0);
    }
}
