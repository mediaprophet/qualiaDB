//! Experiment execution engine — wraps the existing `run_bench` harness with
//! quality verification, thermal sampling, and structured result collection.
//!
//! Each experiment produces an `ExperimentResult` containing:
//! - `BenchResult` (latency, tok/s, phase split)
//! - `VerifiedTurn` (post-turn quality checks)
//! - Thermal snapshot (GPU temp/power before + after)
//! - VRAM usage from the GPU context ledger
//! - CBOR config snapshot for reproduction

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::inference_bench::{
    BenchConfig, BenchResult, LlmPhaseSnapshot, run_bench,
    set_decode_budget_override, set_gpu_topk, set_ternary_ffn, set_kv_dict,
    set_attention_preproject, set_attention_o_fuse, set_spec_decode,
    set_ffn_fusion, set_coop_gemv, set_kv_int8, set_resident_decode,
    set_resident_prefill, set_resident_weights,
};
use crate::post_turn_verify::{verify_and_heal_turn, VerifiedTurn};
use crate::thermal_telemetry::{sample_gpu_thermal, GpuThermalSample};
use crate::gpu_context::global_vram_ledger;

use super::config_space::{Configuration, ConfigurationSpace};

/// Thermal reading before and after an experiment.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct ThermalSnapshot {
    pub before_temp_c: u32,
    pub before_power_w: f64,
    pub after_temp_c: u32,
    pub after_power_w: f64,
    /// Energy estimate in joules: avg_power_w * duration_s.
    pub energy_j: f64,
}

impl ThermalSnapshot {
    fn capture(before: Option<GpuThermalSample>, after: Option<GpuThermalSample>, duration_s: f64) -> Self {
        let (before_temp_c, before_power_w) = before
            .map(|s| (s.temp_c, s.power_w))
            .unwrap_or((0, 0.0));
        let (after_temp_c, after_power_w) = after
            .map(|s| (s.temp_c, s.power_w))
            .unwrap_or((0, 0.0));
        let avg_power = (before_power_w + after_power_w) / 2.0;
        Self {
            before_temp_c,
            before_power_w,
            after_temp_c,
            after_power_w,
            energy_j: avg_power * duration_s,
        }
    }
}

/// Serializable subset of `BenchResult` (the original only derives `Serialize`).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BenchResultSerde {
    pub label: String,
    pub quantization: String,
    pub prompt_tokens: u64,
    pub output_tokens: u64,
    pub cold_ttft_ms: f64,
    pub cold_total_ms: f64,
    pub warm_ttft_ms: f64,
    pub warm_total_ms: f64,
    pub load_ms: f64,
    pub prefill_ms: f64,
    pub prefill_tok_s: f64,
    pub decode_ms: f64,
    pub decode_tok_s: f64,
    pub mapped_bytes: u64,
    pub kv_cache_bytes: u64,
}

impl From<&BenchResult> for BenchResultSerde {
    fn from(r: &BenchResult) -> Self {
        Self {
            label: r.label.clone(),
            quantization: r.quantization.clone(),
            prompt_tokens: r.prompt_tokens,
            output_tokens: r.output_tokens,
            cold_ttft_ms: r.cold_ttft_ms,
            cold_total_ms: r.cold_total_ms,
            warm_ttft_ms: r.warm_ttft_ms,
            warm_total_ms: r.warm_total_ms,
            load_ms: r.load_ms,
            prefill_ms: r.prefill_ms,
            prefill_tok_s: r.prefill_tok_s,
            decode_ms: r.decode_ms,
            decode_tok_s: r.decode_tok_s,
            mapped_bytes: r.model.mapped_bytes,
            kv_cache_bytes: r.model.kv_cache_bytes,
        }
    }
}

/// The full result of one experiment run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentResult {
    /// FNV-1a hash of the configuration CBOR (dedup key).
    pub config_hash: u64,
    /// Links to the hypothesis backlog (if any).
    pub hypothesis_id: Option<String>,
    /// The benchmark result (latency, tok/s, phase split).
    pub bench: Option<BenchResultSerde>,
    /// Intra-decode phase breakdown.
    pub phase: PhaseSnapshotSerde,
    /// Post-turn quality checks.
    pub quality: QualityScore,
    /// GPU thermal before/after.
    pub thermal: ThermalSnapshot,
    /// VRAM used (bytes) from the GPU context ledger.
    pub vram_used: u64,
    /// CBOR config snapshot for reproduction.
    pub config_cbor: Vec<u8>,
    /// Unix epoch timestamp (ms).
    pub timestamp_ms: u64,
    /// Sampler seed for reproducibility.
    pub seed: u64,
    /// Error message if the experiment failed.
    pub error: Option<String>,
}

/// Serializable phase snapshot (the atomic counters are copied into plain u64s).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct PhaseSnapshotSerde {
    pub load_ns: u64,
    pub prefill_ns: u64,
    pub prefill_tokens: u64,
    pub decode_ns: u64,
    pub decode_tokens: u64,
    pub decode_forward_ns: u64,
    pub decode_output_ns: u64,
}

impl From<LlmPhaseSnapshot> for PhaseSnapshotSerde {
    fn from(s: LlmPhaseSnapshot) -> Self {
        Self {
            load_ns: s.load_ns,
            prefill_ns: s.prefill_ns,
            prefill_tokens: s.prefill_tokens,
            decode_ns: s.decode_ns,
            decode_tokens: s.decode_tokens,
            decode_forward_ns: s.decode_forward_ns,
            decode_output_ns: s.decode_output_ns,
        }
    }
}

/// Quality score derived from `VerifiedTurn` checks.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QualityScore {
    /// Fraction of checks that passed (0.0 to 1.0).
    pub pass_rate: f64,
    /// Total number of checks.
    pub total_checks: usize,
    /// Number of checks that passed.
    pub passed: usize,
    /// Whether the draft was repaired by the graph.
    pub repaired: bool,
    /// Final text length (chars).
    pub text_len: usize,
}

impl QualityScore {
    pub fn from_verified(v: &VerifiedTurn) -> Self {
        let total = v.checks.len();
        let passed = v.checks.iter().filter(|c| c.ok).count();
        Self {
            pass_rate: if total > 0 {
                passed as f64 / total as f64
            } else {
                1.0
            },
            total_checks: total,
            passed,
            repaired: v.repaired,
            text_len: v.final_text.len(),
        }
    }

    /// A composite quality score in [0, 1]: pass_rate weighted by whether repair was needed.
    pub fn composite(&self) -> f64 {
        let base = self.pass_rate;
        if self.repaired {
            base * 0.9
        } else {
            base
        }
    }
}

/// Configuration for one experiment run.
#[derive(Debug, Clone)]
pub struct ExperimentConfig {
    /// The configuration space this experiment belongs to.
    pub space: ConfigurationSpace,
    /// The concrete configuration to test.
    pub config: Configuration,
    /// Model path for the benchmark.
    pub model_path: String,
    /// Quantization label.
    pub quantization: String,
    /// Prompt to run.
    pub prompt: String,
    /// Decode token budget (0 = production default).
    pub decode_tokens: u32,
    /// Warm repeats.
    pub warm_repeats: u32,
    /// Sampler seed.
    pub seed: u64,
    /// Linked hypothesis ID.
    pub hypothesis_id: Option<String>,
}

impl ExperimentConfig {
    /// Build a `BenchConfig` from this experiment config.
    pub fn to_bench_config(&self) -> BenchConfig {
        BenchConfig {
            label: format!("{}_h{:016x}", self.space.name, self.config.hash()),
            model_path: self.model_path.clone(),
            quantization: self.quantization.clone(),
            prompt: self.prompt.clone(),
            decode_tokens: self.decode_tokens,
            warm_repeats: self.warm_repeats,
        }
    }

    /// Apply the configuration's toggle values to the runtime.
    /// This reads known parameter names from the config and sets the corresponding toggles.
    pub fn apply_toggles(&self) {
        if let Some(v) = self.config.get_bool("gpu_topk") {
            set_gpu_topk(v);
        }
        if let Some(v) = self.config.get_bool("ternary_ffn") {
            set_ternary_ffn(v);
        }
        if let Some(v) = self.config.get_bool("kv_dict") {
            set_kv_dict(v);
        }
        if let Some(v) = self.config.get_bool("attention_preproject") {
            set_attention_preproject(v);
        }
        if let Some(v) = self.config.get_bool("attention_o_fuse") {
            set_attention_o_fuse(v);
        }
        if let Some(v) = self.config.get_bool("spec_decode") {
            set_spec_decode(v);
        }
        if let Some(v) = self.config.get_bool("ffn_fusion") {
            set_ffn_fusion(v);
        }
        if let Some(v) = self.config.get_bool("coop_gemv") {
            set_coop_gemv(v);
        }
        if let Some(v) = self.config.get_bool("kv_int8") {
            set_kv_int8(v);
        }
        if let Some(v) = self.config.get_bool("resident_decode") {
            set_resident_decode(v);
        }
        if let Some(v) = self.config.get_bool("resident_prefill") {
            set_resident_prefill(v);
        }
        if let Some(v) = self.config.get_bool("resident_weights") {
            set_resident_weights(v);
        }
    }
}

/// Run one experiment end-to-end: apply toggles, run benchmark, verify quality,
/// sample thermal, collect results.
pub fn run_experiment(cfg: &ExperimentConfig) -> ExperimentResult {
    let config_hash = cfg.config.hash();
    let config_cbor = cfg.config.to_cbor();
    let timestamp_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    // Apply toggle configuration
    cfg.apply_toggles();
    set_decode_budget_override(cfg.decode_tokens);

    // Thermal sample before
    let thermal_before = sample_gpu_thermal();
    let t_start = std::time::Instant::now();

    // Run the benchmark
    let bench_result = run_bench(&cfg.to_bench_config());

    let elapsed = t_start.elapsed().as_secs_f64();

    // Thermal sample after
    let thermal_after = sample_gpu_thermal();
    let thermal = ThermalSnapshot::capture(thermal_before, thermal_after, elapsed);

    // Collect phase metrics
    let phase = crate::inference_bench::phase_snapshot();
    let phase_serde = PhaseSnapshotSerde::from(phase);

    // VRAM usage from the global VRAM ledger
    let vram_used = global_vram_ledger().used_bytes();

    // Quality check: run post-turn verify on the output
    let quality = match &bench_result {
        Ok(br) => {
            // We don't have the actual generated text from run_bench (it only returns metrics),
            // so we use a quality proxy from the bench result itself.
            // In a full implementation, we'd run decode_with_metrics to get the text.
            // For now, we score based on whether the benchmark succeeded and produced tokens.
            let has_output = br.output_tokens > 0;
            QualityScore {
                pass_rate: if has_output { 1.0 } else { 0.0 },
                total_checks: 1,
                passed: if has_output { 1 } else { 0 },
                repaired: false,
                text_len: 0,
            }
        }
        Err(_) => QualityScore {
            pass_rate: 0.0,
            total_checks: 1,
            passed: 0,
            repaired: false,
            text_len: 0,
        },
    };

    // Restore defaults
    set_decode_budget_override(0);

    let (bench, error) = match bench_result {
        Ok(r) => (Some(BenchResultSerde::from(&r)), None),
        Err(e) => (None, Some(e)),
    };

    ExperimentResult {
        config_hash,
        hypothesis_id: cfg.hypothesis_id.clone(),
        bench,
        phase: phase_serde,
        quality,
        thermal,
        vram_used,
        config_cbor,
        timestamp_ms,
        seed: cfg.seed,
        error,
    }
}

/// Run an experiment with quality verification (uses `decode_with_metrics` to get
/// the actual generated text, then runs `verify_and_heal_turn` on it).
pub fn run_experiment_with_quality(cfg: &ExperimentConfig) -> ExperimentResult {
    use crate::inference_bench::{decode_with_metrics, reset_phase_metrics};

    let config_hash = cfg.config.hash();
    let config_cbor = cfg.config.to_cbor();
    let timestamp_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    cfg.apply_toggles();

    let thermal_before = sample_gpu_thermal();
    let t_start = std::time::Instant::now();

    reset_phase_metrics();

    let decode_tokens = cfg.decode_tokens.max(8).min(128);
    let result = decode_with_metrics(&cfg.model_path, &cfg.prompt, decode_tokens);

    let elapsed = t_start.elapsed().as_secs_f64();
    let thermal_after = sample_gpu_thermal();
    let thermal = ThermalSnapshot::capture(thermal_before, thermal_after, elapsed);

    let phase = crate::inference_bench::phase_snapshot();
    let phase_serde = PhaseSnapshotSerde::from(phase);

    let vram_used = global_vram_ledger().used_bytes();

    let (bench, quality, error) = match result {
        Ok((text, tok_s)) => {
            // Run post-turn verification on the generated text
            let verified = verify_and_heal_turn(&cfg.prompt, &text);
            let qs = QualityScore::from_verified(&verified);

            // Build a serializable bench result from the decode metrics
            let bench = BenchResultSerde {
                label: format!("{}_h{:016x}", cfg.space.name, config_hash),
                quantization: cfg.quantization.clone(),
                prompt_tokens: phase_serde.prefill_tokens,
                output_tokens: phase_serde.decode_tokens,
                cold_ttft_ms: 0.0,
                cold_total_ms: 0.0,
                warm_ttft_ms: 0.0,
                warm_total_ms: elapsed * 1000.0,
                load_ms: phase_serde.load_ns as f64 / 1_000_000.0,
                prefill_ms: phase_serde.prefill_ns as f64 / 1_000_000.0,
                prefill_tok_s: if phase_serde.prefill_ns > 0 && phase_serde.prefill_tokens > 0 {
                    phase_serde.prefill_tokens as f64 / (phase_serde.prefill_ns as f64 / 1e9)
                } else {
                    0.0
                },
                decode_ms: phase_serde.decode_ns as f64 / 1_000_000.0,
                decode_tok_s: tok_s,
                mapped_bytes: 0,
                kv_cache_bytes: 0,
            };
            (Some(bench), qs, None)
        }
        Err(e) => (
            None,
            QualityScore::default(),
            Some(e),
        ),
    };

    set_decode_budget_override(0);

    ExperimentResult {
        config_hash,
        hypothesis_id: cfg.hypothesis_id.clone(),
        bench,
        phase: phase_serde,
        quality,
        thermal,
        vram_used,
        config_cbor,
        timestamp_ms,
        seed: cfg.seed,
        error,
    }
}

/// Append-only JSONL experiment log.
pub fn append_experiment_jsonl(path: &Path, result: &ExperimentResult) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let line = serde_json::to_string(result).map_err(|e| e.to_string())?;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("open {}: {e}", path.display()))?;
    use std::io::Write;
    writeln!(file, "{line}").map_err(|e| e.to_string())?;
    Ok(())
}

/// Load all experiment results from a JSONL log.
pub fn load_experiment_log(path: &Path) -> Result<Vec<ExperimentResult>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut results = Vec::new();
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<ExperimentResult>(line) {
            Ok(r) => results.push(r),
            Err(e) => log::warn!("experiment_log|skip_line|{e}"),
        }
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inference::lab::config_space::{ConfigurationSpace, ParameterDef};

    #[test]
    fn quality_score_from_verified() {
        use crate::post_turn_verify::{VerifyCheck, VerifiedTurn};
        let v = VerifiedTurn {
            final_text: "Paris".into(),
            display_html: String::new(),
            cml_turtle: String::new(),
            repaired: false,
            checks: vec![
                VerifyCheck { id: "a".into(), ok: true, detail: "ok".into() },
                VerifyCheck { id: "b".into(), ok: true, detail: "ok".into() },
            ],
            grounding_reason: None,
        };
        let qs = QualityScore::from_verified(&v);
        assert_eq!(qs.pass_rate, 1.0);
        assert_eq!(qs.total_checks, 2);
        assert!(!qs.repaired);
    }

    #[test]
    fn quality_score_with_repair() {
        use crate::post_turn_verify::{VerifyCheck, VerifiedTurn};
        let v = VerifiedTurn {
            final_text: "Paris".into(),
            display_html: String::new(),
            cml_turtle: String::new(),
            repaired: true,
            checks: vec![
                VerifyCheck { id: "a".into(), ok: true, detail: "ok".into() },
                VerifyCheck { id: "b".into(), ok: false, detail: "fail".into() },
            ],
            grounding_reason: Some("capital".into()),
        };
        let qs = QualityScore::from_verified(&v);
        assert_eq!(qs.pass_rate, 0.5);
        assert!(qs.repaired);
        assert!((qs.composite() - 0.45).abs() < 1e-9);
    }

    #[test]
    fn experiment_config_builds_bench_config() {
        let space = ConfigurationSpace::new("test")
            .with("gpu_topk", ParameterDef::Bool);
        let cfg = space.build_from_normalized(&[1.0]);
        let ec = ExperimentConfig {
            space,
            config: cfg,
            model_path: "/dev/null".into(),
            quantization: "Q8_0".into(),
            prompt: "Hello".into(),
            decode_tokens: 16,
            warm_repeats: 1,
            seed: 42,
            hypothesis_id: Some("H-001".into()),
        };
        let bc = ec.to_bench_config();
        assert_eq!(bc.quantization, "Q8_0");
        assert_eq!(bc.decode_tokens, 16);
        assert!(bc.label.contains("test_"));
    }

    #[test]
    fn jsonl_log_roundtrip() {
        let tmp = std::env::temp_dir().join(format!("qualia_lab_test_{}.jsonl", std::process::id()));
        let result = ExperimentResult {
            config_hash: 42,
            hypothesis_id: Some("H-001".into()),
            bench: None,
            phase: PhaseSnapshotSerde::default(),
            quality: QualityScore::default(),
            thermal: ThermalSnapshot::default(),
            vram_used: 0,
            config_cbor: vec![0xA1, 0x01],
            timestamp_ms: 12345,
            seed: 99,
            error: Some("test".into()),
        };
        append_experiment_jsonl(&tmp, &result).unwrap();
        let loaded = load_experiment_log(&tmp).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].config_hash, 42);
        assert_eq!(loaded[0].seed, 99);
        let _ = std::fs::remove_file(&tmp);
    }
}
