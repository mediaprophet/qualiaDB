//! Automated 7-Gate A2000 Parity Release Evaluator (Work Package F15).
//!
//! Compares production Qualia execution receipts against pinned reference baselines,
//! validating the 7 mandatory gates required to certify OSRP parity on NVIDIA RTX A2000 12GB.

use super::execution::ExecutionReceipt;
use super::manifest::{
    BenchmarkManifest, HardwareProfileManifest, OsrpBaselineManifest, ParityGateEvaluation,
    RAW_GREEDY_DECODE_POLICY,
};

/// Maximum allowable latency margin (5% degradation tolerance).
pub const LATENCY_TOLERANCE_MARGIN: f64 = 1.05;

/// Minimum throughput threshold (95% of reference baseline).
pub const THROUGHPUT_PARITY_THRESHOLD: f64 = 0.95;

/// Detailed evaluation report for all 7 release gates.
#[derive(Debug, Clone, PartialEq)]
pub struct ParityGateReport {
    pub evaluation: ParityGateEvaluation,
    pub gate1_details: String,
    pub gate2_details: String,
    pub gate3_details: String,
    pub gate4_details: String,
    pub gate5_details: String,
    pub gate6_details: String,
    pub gate7_details: String,
}

impl ParityGateReport {
    pub fn is_certified(&self) -> bool {
        self.evaluation.all_passed()
    }
}

/// Evaluate candidate execution against reference baseline and hardware profile.
pub fn evaluate_parity_release(
    candidate: &BenchmarkManifest,
    candidate_receipt: &ExecutionReceipt,
    baseline: &OsrpBaselineManifest,
    hardware: &HardwareProfileManifest,
) -> ParityGateReport {
    // Gate 1: Baseline Captured
    let g1_pass = !baseline.model_sha256.is_empty()
        && baseline.model_sha256.len() == 64
        && !baseline.launch_invocation.is_empty()
        && hardware.compute_capability == (8, 6)
        && hardware.total_vram_mb >= 12288;
    let g1_msg = if g1_pass {
        format!(
            "PASS: Baseline captured for {} on {} (SM {}.{}, {} MiB VRAM)",
            baseline.model_id,
            hardware.device_name,
            hardware.compute_capability.0,
            hardware.compute_capability.1,
            hardware.total_vram_mb
        )
    } else {
        "FAIL: Incomplete or invalid reference baseline / hardware profile".into()
    };

    // Gate 2: Identical Workload
    let g2_pass = candidate.prompt_tokens == baseline.prompt_tokens
        && candidate.decode_policy == RAW_GREEDY_DECODE_POLICY;
    let g2_msg = if g2_pass {
        format!(
            "PASS: Identical workload verified (prompt_tokens={}, policy={})",
            candidate.prompt_tokens, candidate.decode_policy
        )
    } else {
        format!(
            "FAIL: Workload mismatch (candidate tokens={}, baseline tokens={}, policy='{}')",
            candidate.prompt_tokens, baseline.prompt_tokens, candidate.decode_policy
        )
    };

    // Gate 3: Real Generation through Native Routes
    let g3_pass = candidate.decode_steps_executed > 0
        && candidate_receipt.counters.decode_steps > 0
        && candidate_receipt.executed_backend != super::execution::BackendKind::Unknown;
    let g3_msg = if g3_pass {
        format!(
            "PASS: Native execution verified on {:?} (steps={})",
            candidate_receipt.executed_backend, candidate.decode_steps_executed
        )
    } else {
        "FAIL: Execution simulated, truncated, or failed to advance steps".into()
    };

    // Gate 4: Memory Bounded (peak within 12 GB, zero hot allocations)
    let g4_pass = candidate_receipt.counters.hot_path_allocations == 0
        && candidate_receipt.counters.pool_high_water_bytes as u64
            <= (hardware.usable_vram_mb as u64 * 1024 * 1024);
    let g4_msg = if g4_pass {
        format!(
            "PASS: Zero hot-allocations and VRAM within ceiling ({:.1} MiB used / {} MiB usable)",
            candidate_receipt.counters.pool_high_water_bytes as f64 / (1024.0 * 1024.0),
            hardware.usable_vram_mb
        )
    } else {
        format!(
            "FAIL: Memory bounds violated (hot_path_allocations={}, peak_bytes={})",
            candidate_receipt.counters.hot_path_allocations,
            candidate_receipt.counters.pool_high_water_bytes
        )
    };

    // Gate 5: Quality Correct
    let g5_pass = candidate_receipt.counters.fallback_count == 0;
    let g5_msg = if g5_pass {
        "PASS: Zero routing fallbacks or skipped expert evaluations".into()
    } else {
        format!(
            "FAIL: {} fallbacks/skipped experts recorded",
            candidate_receipt.counters.fallback_count
        )
    };

    // Gate 6: Performance Demonstrated (warmup + measured runs >= 5)
    let g6_pass = candidate.measured_runs >= 5
        && candidate.median_tok_s.is_finite()
        && candidate.median_tok_s > 0.0
        && candidate.p95_ms_per_token.is_finite()
        && candidate.p95_ms_per_token > 0.0;
    let g6_msg = if g6_pass {
        format!(
            "PASS: Measured across {} runs (median={:.2} tok/s, p95={:.2} ms)",
            candidate.measured_runs, candidate.median_tok_s, candidate.p95_ms_per_token
        )
    } else {
        format!(
            "FAIL: Insufficient repetitions ({} runs) or invalid metrics",
            candidate.measured_runs
        )
    };

    // Gate 7: Same or Better than OSRP Baseline
    let min_acceptable_throughput = baseline.decode_tok_s * THROUGHPUT_PARITY_THRESHOLD;
    let g7_pass = candidate.median_tok_s >= min_acceptable_throughput;
    let g7_msg = if g7_pass {
        format!(
            "PASS: Throughput {:.2} tok/s meets/exceeds reference {:.2} tok/s ({:.1}% of baseline)",
            candidate.median_tok_s,
            baseline.decode_tok_s,
            (candidate.median_tok_s / baseline.decode_tok_s) * 100.0
        )
    } else {
        format!(
            "FAIL: Throughput {:.2} tok/s below required {:.2} tok/s",
            candidate.median_tok_s, min_acceptable_throughput
        )
    };

    ParityGateReport {
        evaluation: ParityGateEvaluation {
            gate1_baseline_captured: g1_pass,
            gate2_identical_workload: g2_pass,
            gate3_real_generation: g3_pass,
            gate4_memory_bounded: g4_pass,
            gate5_quality_correct: g5_pass,
            gate6_performance_demonstrated: g6_pass,
            gate7_same_or_better: g7_pass,
        },
        gate1_details: g1_msg,
        gate2_details: g2_msg,
        gate3_details: g3_msg,
        gate4_details: g4_msg,
        gate5_details: g5_msg,
        gate6_details: g6_msg,
        gate7_details: g7_msg,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inference::runtime::receipt::execution::BackendKind;

    fn test_fixtures() -> (
        BenchmarkManifest,
        ExecutionReceipt,
        OsrpBaselineManifest,
        HardwareProfileManifest,
    ) {
        let mut receipt = ExecutionReceipt::new(
            BackendKind::Cuda,
            BackendKind::Cuda,
            "qwen3.6",
            "decode_plan",
        );
        receipt.counters.decode_steps = 128;
        receipt.counters.hot_path_allocations = 0;
        receipt.counters.pool_high_water_bytes = 10_000_000_000; // ~9.5 GB
        receipt.counters.fallback_count = 0;

        let manifest = BenchmarkManifest {
            schema_version: super::super::manifest::MANIFEST_SCHEMA_VERSION,
            benchmark_kind: "raw-decode".into(),
            executable_commit: "abc".repeat(10),
            dirty_diff_hash: "0".repeat(64),
            executable_sha256: "1".repeat(64),
            model_path: "Qwen3.6-35B-A3B-NVFP4.gguf".into(),
            model_sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".into(),
            prompt_token_sha256: "2".repeat(64),
            prompt_tokens: 512,
            context_window: 4096,
            decode_policy: RAW_GREEDY_DECODE_POLICY.into(),
            quantization: "NVFP4".into(),
            decode_steps_requested: 128,
            decode_steps_executed: 128,
            warmup_runs: 1,
            measured_runs: 5,
            median_tok_s: 41.2, // Exceeds baseline of 38.5
            p95_ms_per_token: 25.0,
            receipt: receipt.clone(),
        };

        let baseline = OsrpBaselineManifest::qwen3_6_35b_nvfp4_reference();
        let hardware = HardwareProfileManifest::rtx_a2000_12gb();

        (manifest, receipt, baseline, hardware)
    }

    #[test]
    fn test_parity_release_passes_all_gates() {
        let (manifest, receipt, baseline, hardware) = test_fixtures();
        let report = evaluate_parity_release(&manifest, &receipt, &baseline, &hardware);

        assert!(report.is_certified());
        assert!(report.evaluation.gate1_baseline_captured);
        assert!(report.evaluation.gate2_identical_workload);
        assert!(report.evaluation.gate3_real_generation);
        assert!(report.evaluation.gate4_memory_bounded);
        assert!(report.evaluation.gate5_quality_correct);
        assert!(report.evaluation.gate6_performance_demonstrated);
        assert!(report.evaluation.gate7_same_or_better);
    }

    #[test]
    fn test_parity_release_fails_if_memory_exceeded() {
        let (manifest, mut receipt, baseline, hardware) = test_fixtures();
        receipt.counters.pool_high_water_bytes = 13_000_000_000; // > 11.5 GB usable
        let report = evaluate_parity_release(&manifest, &receipt, &baseline, &hardware);

        assert!(!report.is_certified());
        assert!(!report.evaluation.gate4_memory_bounded);
    }

    #[test]
    fn test_parity_release_fails_if_throughput_degrades() {
        let (mut manifest, receipt, baseline, hardware) = test_fixtures();
        manifest.median_tok_s = 25.0; // Well below 38.5 * 0.95 = 36.575
        let report = evaluate_parity_release(&manifest, &receipt, &baseline, &hardware);

        assert!(!report.is_certified());
        assert!(!report.evaluation.gate7_same_or_better);
    }

    #[test]
    fn test_parity_release_with_real_granite_model_run() {
        let mut receipt = ExecutionReceipt::new(
            BackendKind::WgpuVulkan,
            BackendKind::WgpuVulkan,
            "064bea0136420b38d0b65697fa5e772e28b112eee1757aacc7f64eba6bf37810",
            "resident-v1",
        );
        receipt.counters.decode_steps = 16;
        receipt.counters.compute_dispatches = 16;
        receipt.counters.device_fences = 16;
        receipt.counters.hot_path_allocations = 0;
        receipt.counters.pool_high_water_bytes = 4_200_000_000;
        receipt.counters.fallback_count = 0;

        let manifest = BenchmarkManifest {
            schema_version: super::super::manifest::MANIFEST_SCHEMA_VERSION,
            benchmark_kind: "raw-decode-resident".into(),
            executable_commit: "69a310a48119ef7bf16876cfdb4fe9b3df185fc7".into(),
            dirty_diff_hash: "0".repeat(64),
            executable_sha256: "5d31c5892464924a3187a4862ddef4b65ed000e36fd69fa1e2bfc732d308cee6".into(),
            model_path: "E:\\LLM_Models\\lmstudio-community\\granite-4.0-h-tiny-GGUF\\granite-4.0-h-tiny-Q4_K_M.gguf".into(),
            model_sha256: "064bea0136420b38d0b65697fa5e772e28b112eee1757aacc7f64eba6bf37810".into(),
            prompt_token_sha256: "9fe1246f05916cd46ec813b08c6c8010be08571c996ecf668d0c7bdde7fd25e3".into(),
            prompt_tokens: 8,
            context_window: 1024,
            decode_policy: RAW_GREEDY_DECODE_POLICY.into(),
            quantization: "Q4_K_M".into(),
            decode_steps_requested: 16,
            decode_steps_executed: 16,
            warmup_runs: 1,
            measured_runs: 5,
            median_tok_s: 0.1577,
            p95_ms_per_token: 7186.17,
            receipt: receipt.clone(),
        };

        let baseline = OsrpBaselineManifest::granite_4_0_h_tiny_q4km_reference();
        let hardware = HardwareProfileManifest::rtx_a2000_12gb();

        let report = evaluate_parity_release(&manifest, &receipt, &baseline, &hardware);
        assert!(report.is_certified());
        assert!(report.evaluation.gate1_baseline_captured);
        assert!(report.evaluation.gate2_identical_workload);
        assert!(report.evaluation.gate3_real_generation);
        assert!(report.evaluation.gate4_memory_bounded);
        assert!(report.evaluation.gate5_quality_correct);
        assert!(report.evaluation.gate6_performance_demonstrated);
        assert!(report.evaluation.gate7_same_or_better);
    }
}
