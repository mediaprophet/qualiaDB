//! Experiment execution engine — wraps the existing `run_bench` harness with
//! quality verification, thermal sampling, and structured result collection.
//!
//! Each experiment produces an `ExperimentResult` containing:
//! - `BenchResult` (latency, tok/s, phase split)
//! - `VerifiedTurn` (post-turn quality checks)
//! - Thermal snapshot (GPU temp/power before + after)
//! - VRAM usage from the GPU context ledger
//! - CBOR config snapshot for reproduction

use std::time::{SystemTime, UNIX_EPOCH};

use crate::gpu_context::global_vram_ledger;
use crate::inference::runtime::receipt::execution::{
    BackendKind, ExecutionReceipt, COUNTER_COMMITTED_PREFILL_TOKENS, COUNTER_DECODE_STEPS,
    COUNTER_SUBMITTED_PREFILL_TOKENS, COUNTER_TIME_TO_FIRST_TOKEN_US,
};
use crate::inference_bench::{
    run_bench, set_attention_o_fuse, set_attention_preproject, set_coop_gemv,
    set_decode_budget_override, set_ffn_fusion, set_gpu_topk, set_kv_dict, set_kv_int8,
    set_kv_pool_budget_mb, set_prefill_chunk_size_override, set_prefix_cache_enabled,
    set_prompt_precision_mode, set_resident_decode, set_resident_prefill, set_resident_weights,
    set_spec_decode, set_ternary_ffn,
};
use crate::post_turn_verify::verify_and_heal_turn;
use crate::thermal_telemetry::sample_gpu_thermal;

use super::experiment_adapter::*;

pub use etalon_inference_lab::experiment::*;
pub trait ExperimentConfigExt {
    fn apply_toggles(&self);
}
impl ExperimentConfigExt for ExperimentConfig {
    /// Apply the configuration's toggle values to the runtime.
    /// This reads known parameter names from the config and sets the corresponding toggles.
    fn apply_toggles(&self) {
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
        if let Some(v) = self.config.get_bool("prefix_cache_enabled") {
            set_prefix_cache_enabled(v);
        }
        if let Some(v) = self.config.get_int("kv_pool_budget_mb") {
            set_kv_pool_budget_mb(v as u32);
        }
        if let Some(v) = self.config.get_int("prefill_chunk_size") {
            set_prefill_chunk_size_override(v as u32);
        }
        if let Some(s) = self.config.get_string("prompt_precision_mode") {
            set_prompt_precision_mode(s);
        }
    }
}

/// Run one experiment end-to-end: apply toggles, run benchmark, verify quality,
/// sample thermal, collect results.
pub fn run_experiment(cfg: &ExperimentConfig) -> ExperimentResult {
    let _state = super::state::TrialState::capture();
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
    let thermal = capture_thermal(thermal_before, thermal_after, elapsed);
    let energy_state = if thermal_before.is_some() && thermal_after.is_some() {
        etalon_inference_lab::runner::MetricState::Estimated(thermal.energy_j)
    } else {
        etalon_inference_lab::runner::MetricState::Unavailable
    };

    // Collect phase metrics
    let phase = crate::inference_bench::phase_snapshot();
    let phase_serde = phase_serde(phase);

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
        Ok(r) => (Some(bench_serde(&r)), None),
        Err(e) => (None, Some(e)),
    };

    let receipt = if let Some(ref br) = bench {
        let mut rc = ExecutionReceipt::new(BackendKind::Unknown, BackendKind::Unknown, "", "");
        rc.counters.decode_steps = br.output_tokens;
        rc.mark_measured(COUNTER_DECODE_STEPS);
        rc.counters.submitted_prefill_tokens = br.prompt_tokens;
        rc.mark_measured(COUNTER_SUBMITTED_PREFILL_TOKENS);
        rc.counters.committed_prefill_tokens = br.prompt_tokens;
        rc.mark_measured(COUNTER_COMMITTED_PREFILL_TOKENS);
        rc.counters.pool_high_water_bytes = vram_used;
        // A ledger snapshot is not a measured high-water mark.

        let ttft_us = (br.cold_ttft_ms * 1000.0) as u64;
        let itl_us = if br.output_tokens > 1 {
            ((br.decode_ms / (br.output_tokens - 1) as f64) * 1000.0) as u64
        } else {
            0
        };
        rc.latency.time_to_first_token_us = ttft_us;
        rc.mark_measured(COUNTER_TIME_TO_FIRST_TOKEN_US);
        rc.latency.inter_token_latency_us = itl_us;
        // Inter-token latency is derived from aggregate decode duration.
        Some(rc)
    } else {
        None
    };

    ExperimentResult {
        config_hash,
        hypothesis_id: cfg.hypothesis_id.clone(),
        evaluation_mode: cfg.evaluation_mode,
        receipt,
        bench,
        phase: phase_serde,
        quality,
        thermal,
        vram_used,
        config_cbor,
        timestamp_ms,
        seed: cfg.seed,
        error,
        evidence: TrialEvidence {
            schema_version: 1,
            requested_token_budget: Some(cfg.decode_tokens),
            actual_token_budget: Some(cfg.decode_tokens),
            quality_is_token_proxy: true,
            energy_j: energy_state,
            ..Default::default()
        },
    }
}

/// Run an experiment with quality verification (uses `decode_with_metrics` to get
/// the actual generated text, then runs `verify_and_heal_turn` on it).
pub fn run_experiment_with_quality(cfg: &ExperimentConfig) -> ExperimentResult {
    let _state = super::state::TrialState::capture();
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
    let thermal = capture_thermal(thermal_before, thermal_after, elapsed);
    let energy_state = if thermal_before.is_some() && thermal_after.is_some() {
        etalon_inference_lab::runner::MetricState::Estimated(thermal.energy_j)
    } else {
        etalon_inference_lab::runner::MetricState::Unavailable
    };

    let phase = crate::inference_bench::phase_snapshot();
    let phase_serde = phase_serde(phase);

    let vram_used = global_vram_ledger().used_bytes();

    let mut evidence = TrialEvidence {
        energy_j: energy_state,
        schema_version: 1,
        requested_token_budget: Some(cfg.decode_tokens),
        actual_token_budget: Some(decode_tokens),
        ..Default::default()
    };
    let (bench, quality, error) = match result {
        Ok((text, tok_s)) => {
            // Run post-turn verification on the generated text
            let repair_start = std::time::Instant::now();
            let verified = verify_and_heal_turn(&cfg.prompt, &text);
            let qs = quality_from_verified(&verified);
            evidence.raw_output = Some(text);
            evidence.repair_duration_ms = etalon_inference_lab::runner::MetricState::Measured(
                repair_start.elapsed().as_secs_f64() * 1000.0,
            );
            if verified.repaired {
                evidence.repaired_output = Some(verified.final_text.clone());
                evidence.repaired_quality = Some(qs.clone());
            } else {
                evidence.raw_quality = Some(qs.clone());
            }

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
        Err(e) => (None, QualityScore::default(), Some(e)),
    };

    set_decode_budget_override(0);

    let receipt = if let Some(ref br) = bench {
        let mut rc = ExecutionReceipt::new(BackendKind::Unknown, BackendKind::Unknown, "", "");
        rc.counters.decode_steps = br.output_tokens;
        rc.mark_measured(COUNTER_DECODE_STEPS);
        rc.counters.submitted_prefill_tokens = br.prompt_tokens;
        rc.mark_measured(COUNTER_SUBMITTED_PREFILL_TOKENS);
        rc.counters.committed_prefill_tokens = br.prompt_tokens;
        rc.mark_measured(COUNTER_COMMITTED_PREFILL_TOKENS);
        rc.counters.pool_high_water_bytes = vram_used;
        // A ledger snapshot is not a measured high-water mark.

        let ttft_us = (br.warm_total_ms * 1000.0) as u64;
        let itl_us = if br.output_tokens > 1 {
            ((br.decode_ms / (br.output_tokens - 1) as f64) * 1000.0) as u64
        } else {
            0
        };
        rc.latency.time_to_first_token_us = ttft_us;
        // Total elapsed time is retained as a legacy proxy, not TTFT evidence.
        rc.latency.inter_token_latency_us = itl_us;
        // Inter-token latency is derived from aggregate decode duration.
        Some(rc)
    } else {
        None
    };

    ExperimentResult {
        config_hash,
        hypothesis_id: cfg.hypothesis_id.clone(),
        evaluation_mode: cfg.evaluation_mode,
        receipt,
        bench,
        phase: phase_serde,
        quality,
        thermal,
        vram_used,
        config_cbor,
        timestamp_ms,
        seed: cfg.seed,
        error,
        evidence,
    }
}

pub struct QualiaExecutor;
impl etalon_inference_lab::runner::ExperimentExecutor for QualiaExecutor {
    fn execute(&mut self, cfg: &ExperimentConfig, with_quality: bool) -> ExperimentResult {
        if with_quality {
            run_experiment_with_quality(cfg)
        } else {
            run_experiment(cfg)
        }
    }
}

#[cfg(test)]
#[path = "experiment_tests.rs"]
mod tests;
