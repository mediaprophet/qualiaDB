use super::experiment::*;
use crate::inference_bench::{BenchResult, LlmPhaseSnapshot};
use crate::post_turn_verify::VerifiedTurn;
use crate::thermal_telemetry::GpuThermalSample;
pub(super) fn bench_serde(r: &BenchResult) -> BenchResultSerde {
    BenchResultSerde {
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

pub(super) fn phase_serde(s: LlmPhaseSnapshot) -> PhaseSnapshotSerde {
    PhaseSnapshotSerde {
        load_ns: s.load_ns,
        prefill_ns: s.prefill_ns,
        prefill_tokens: s.prefill_tokens,
        decode_ns: s.decode_ns,
        decode_tokens: s.decode_tokens,
        decode_forward_ns: s.decode_forward_ns,
        decode_output_ns: s.decode_output_ns,
    }
}

pub(super) fn capture_thermal(
    before: Option<GpuThermalSample>,
    after: Option<GpuThermalSample>,
    duration_s: f64,
) -> ThermalSnapshot {
    let (before_temp_c, before_power_w) = before.map(|s| (s.temp_c, s.power_w)).unwrap_or((0, 0.0));
    let (after_temp_c, after_power_w) = after.map(|s| (s.temp_c, s.power_w)).unwrap_or((0, 0.0));
    let avg_power = (before_power_w + after_power_w) / 2.0;
    ThermalSnapshot {
        before_temp_c,
        before_power_w,
        after_temp_c,
        after_power_w,
        energy_j: avg_power * duration_s,
    }
}

/// `QualityScore::from_verified`.
pub(super) fn quality_from_verified(v: &VerifiedTurn) -> QualityScore {
    let total = v.checks.len();
    let passed = v.checks.iter().filter(|c| c.ok).count();
    QualityScore {
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

impl etalon_inference_lab::experiment::VerifiedQuality for VerifiedTurn {
    fn quality_score(&self) -> QualityScore {
        quality_from_verified(self)
    }
}
impl From<&BenchResult> for BenchResultSerde {
    fn from(value: &BenchResult) -> Self {
        bench_serde(value)
    }
}
impl From<LlmPhaseSnapshot> for PhaseSnapshotSerde {
    fn from(value: LlmPhaseSnapshot) -> Self {
        phase_serde(value)
    }
}
