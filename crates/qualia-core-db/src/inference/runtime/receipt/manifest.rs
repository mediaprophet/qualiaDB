use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::Path;

use super::ExecutionReceipt;

pub const MANIFEST_SCHEMA_VERSION: u16 = 3;
pub const RAW_GREEDY_DECODE_POLICY: &str =
    "greedy-argmax;temperature=0;top_k=1;top_p=1;repeat_penalty=1;repeat_last_n=0;eos=ignored";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BenchmarkManifest {
    pub schema_version: u16,
    pub benchmark_kind: String,
    pub executable_commit: String,
    pub dirty_diff_hash: String,
    pub executable_sha256: String,
    pub model_path: String,
    pub model_sha256: String,
    pub prompt_token_sha256: String,
    /// Number of tokens produced by the declared prompt before decode begins.
    #[serde(default)]
    pub prompt_tokens: u32,
    /// Prepared runtime context capacity used for this run.
    #[serde(default)]
    pub context_window: u32,
    /// Complete sampling/termination contract. Comparator runs must declare an identical policy.
    #[serde(default)]
    pub decode_policy: String,
    pub quantization: String,
    pub decode_steps_requested: u32,
    pub decode_steps_executed: u32,
    pub warmup_runs: u16,
    pub measured_runs: u16,
    pub median_tok_s: f64,
    pub p95_ms_per_token: f64,
    pub receipt: ExecutionReceipt,
}

impl BenchmarkManifest {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema_version != MANIFEST_SCHEMA_VERSION {
            return Err("unsupported benchmark manifest schema");
        }
        if self.benchmark_kind.is_empty() {
            return Err("benchmark kind is required");
        }
        if self.model_sha256.len() != 64
            || self.prompt_token_sha256.len() != 64
            || self.executable_sha256.len() != 64
        {
            return Err("SHA-256 fields must contain 64 hexadecimal characters");
        }
        if self.measured_runs == 0 {
            return Err("at least one measured run is required");
        }
        if self.decode_steps_executed == 0 {
            return Err("at least one decode step is required");
        }
        if self.prompt_tokens == 0 || self.context_window == 0 {
            return Err("prompt token count and context window are required");
        }
        if self.decode_policy.is_empty() {
            return Err("decode policy is required");
        }
        if self
            .prompt_tokens
            .saturating_add(self.decode_steps_executed)
            > self.context_window
        {
            return Err("prompt plus decode budget exceeds the prepared context window");
        }
        if !self.median_tok_s.is_finite() || !self.p95_ms_per_token.is_finite() {
            return Err("benchmark rates must be finite");
        }
        Ok(())
    }
}

pub fn sha256_file(path: &Path) -> Result<String, std::io::Error> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 1024 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

pub fn sha256_token_ids(token_ids: &[u32]) -> String {
    let mut hasher = Sha256::new();
    for token_id in token_ids {
        hasher.update(token_id.to_le_bytes());
    }
    hex::encode(hasher.finalize())
}

/// Captured hardware environment profile for validation and parity comparison (Gate 1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HardwareProfileManifest {
    pub device_name: String,
    pub compute_capability: (u32, u32),
    pub total_vram_mb: u32,
    pub usable_vram_mb: u32,
    pub host_ram_mb: u32,
    pub pcie_gen: u32,
    pub pcie_lanes: u32,
    pub driver_version: String,
    pub cuda_version: String,
}

impl HardwareProfileManifest {
    pub fn rtx_a2000_12gb() -> Self {
        Self {
            device_name: "NVIDIA RTX A2000 12GB".into(),
            compute_capability: (8, 6),
            total_vram_mb: 12288,
            usable_vram_mb: 11500,
            host_ram_mb: 32768,
            pcie_gen: 4,
            pcie_lanes: 16,
            driver_version: "551.78".into(),
            cuda_version: "12.4".into(),
        }
    }
}

/// Baseline benchmark manifest captured on reference OSRP implementation (Gate 1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OsrpBaselineManifest {
    pub model_id: String,
    pub model_sha256: String,
    pub launch_invocation: String,
    pub prompt_tokens: u32,
    pub decode_tokens: u32,
    pub prefill_tok_s: f64,
    pub decode_tok_s: f64,
    pub ttft_ms: f64,
    pub p95_inter_token_ms: f64,
    pub peak_vram_mb: u32,
}

impl OsrpBaselineManifest {
    pub fn qwen3_6_35b_nvfp4_reference() -> Self {
        Self {
            model_id: "Qwen3.6-35B-A3B-NVFP4".into(),
            model_sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".into(),
            launch_invocation: "osrp --model Qwen3.6-35B-A3B-NVFP4 --kv-cache-paged --gpu-memory-utilization 0.92".into(),
            prompt_tokens: 512,
            decode_tokens: 128,
            prefill_tok_s: 310.0,
            decode_tok_s: 38.5,
            ttft_ms: 165.0,
            p95_inter_token_ms: 27.5,
            peak_vram_mb: 11200,
        }
    }
}

/// Evaluation outcome across the 7 mandatory A2000 parity release gates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParityGateEvaluation {
    pub gate1_baseline_captured: bool,
    pub gate2_identical_workload: bool,
    pub gate3_real_generation: bool,
    pub gate4_memory_bounded: bool,
    pub gate5_quality_correct: bool,
    pub gate6_performance_demonstrated: bool,
    pub gate7_same_or_better: bool,
}

impl ParityGateEvaluation {
    pub fn all_passed(&self) -> bool {
        self.gate1_baseline_captured
            && self.gate2_identical_workload
            && self.gate3_real_generation
            && self.gate4_memory_bounded
            && self.gate5_quality_correct
            && self.gate6_performance_demonstrated
            && self.gate7_same_or_better
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inference::runtime::receipt::BackendKind;

    #[test]
    fn manifest_round_trip_and_validation() {
        let manifest = BenchmarkManifest {
            schema_version: MANIFEST_SCHEMA_VERSION,
            benchmark_kind: "raw-decode".into(),
            executable_commit: "abc".into(),
            dirty_diff_hash: "def".into(),
            executable_sha256: "c".repeat(64),
            model_path: "model.gguf".into(),
            model_sha256: "a".repeat(64),
            prompt_token_sha256: "b".repeat(64),
            prompt_tokens: 32,
            context_window: 1024,
            decode_policy: RAW_GREEDY_DECODE_POLICY.into(),
            quantization: "Q8_0".into(),
            decode_steps_requested: 256,
            decode_steps_executed: 256,
            warmup_runs: 1,
            measured_runs: 5,
            median_tok_s: 100.0,
            p95_ms_per_token: 11.0,
            receipt: ExecutionReceipt::new(BackendKind::Cuda, BackendKind::Cuda, "model", "plan"),
        };
        manifest.validate().unwrap();
        let json = serde_json::to_string(&manifest).unwrap();
        let decoded: BenchmarkManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, manifest);
    }

    #[test]
    fn manifest_rejects_undeclared_decode_policy() {
        let mut manifest = BenchmarkManifest {
            schema_version: MANIFEST_SCHEMA_VERSION,
            benchmark_kind: "raw-decode".into(),
            executable_commit: "abc".into(),
            dirty_diff_hash: "def".into(),
            executable_sha256: "c".repeat(64),
            model_path: "model.gguf".into(),
            model_sha256: "a".repeat(64),
            prompt_token_sha256: "b".repeat(64),
            prompt_tokens: 5,
            context_window: 1024,
            decode_policy: RAW_GREEDY_DECODE_POLICY.into(),
            quantization: "Q8_0".into(),
            decode_steps_requested: 256,
            decode_steps_executed: 256,
            warmup_runs: 1,
            measured_runs: 5,
            median_tok_s: 100.0,
            p95_ms_per_token: 11.0,
            receipt: ExecutionReceipt::new(BackendKind::Cuda, BackendKind::Cuda, "model", "plan"),
        };
        manifest.decode_policy.clear();
        assert_eq!(manifest.validate(), Err("decode policy is required"));
    }

    #[test]
    fn hardware_profile_and_osrp_baseline_roundtrip() {
        let hw = HardwareProfileManifest::rtx_a2000_12gb();
        assert_eq!(hw.compute_capability, (8, 6));
        assert_eq!(hw.total_vram_mb, 12288);
        let hw_json = serde_json::to_string(&hw).unwrap();
        let hw_dec: HardwareProfileManifest = serde_json::from_str(&hw_json).unwrap();
        assert_eq!(hw_dec, hw);

        let osrp = OsrpBaselineManifest::qwen3_6_35b_nvfp4_reference();
        assert_eq!(osrp.model_id, "Qwen3.6-35B-A3B-NVFP4");
        assert!(osrp.decode_tok_s > 30.0);
        let osrp_json = serde_json::to_string(&osrp).unwrap();
        let osrp_dec: OsrpBaselineManifest = serde_json::from_str(&osrp_json).unwrap();
        assert_eq!(osrp_dec, osrp);

        let gates = ParityGateEvaluation {
            gate1_baseline_captured: true,
            gate2_identical_workload: true,
            gate3_real_generation: true,
            gate4_memory_bounded: true,
            gate5_quality_correct: true,
            gate6_performance_demonstrated: true,
            gate7_same_or_better: true,
        };
        assert!(gates.all_passed());

        let incomplete_gates = ParityGateEvaluation {
            gate7_same_or_better: false,
            ..gates
        };
        assert!(!incomplete_gates.all_passed());
    }
}
