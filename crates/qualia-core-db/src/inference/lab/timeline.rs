//! Decode timeline: host phase counters + optional GPU profile + tok/s.

use std::path::Path;
use std::time::Instant;

use crate::hardware_passport::measure_decode_proxy_tok_s;
use crate::llm_bench::{
    phase_snapshot, reset_phase_metrics, reset_resident_path_counts, resident_path_counts,
};
use crate::llm_gpu_profiler::{self, Phase};

#[derive(Debug, Clone)]
pub struct DecodeTimeline {
    pub model: String,
    pub tokens: u32,
    pub tok_s: Option<f64>,
    pub wall_ms: f64,
    pub resident_hits: u64,
    pub resident_fallbacks: u64,
    pub host_phase_ns: String,
    pub gpu_phase_ns: String,
    pub notes: String,
}

/// Run a short decode with phase counters; enable GPU timestamps if supported.
pub fn run_decode_timeline(model: &Path, tokens: u32) -> DecodeTimeline {
    let tokens = tokens.max(1).min(64);
    reset_phase_metrics();
    reset_resident_path_counts();
    llm_gpu_profiler::set_enabled(true);
    llm_gpu_profiler::reset();

    let t0 = Instant::now();
    let tok_s = measure_decode_proxy_tok_s(model, tokens);
    let wall_ms = t0.elapsed().as_secs_f64() * 1e3;
    let (hits, falls) = resident_path_counts();
    let snap = phase_snapshot();
    let host_phase_ns = format!(
        "{{\"load_ns\":{},\"prefill_ns\":{},\"prefill_tokens\":{},\"decode_ns\":{},\"decode_tokens\":{},\"decode_forward_ns\":{},\"decode_output_ns\":{}}}",
        snap.load_ns,
        snap.prefill_ns,
        snap.prefill_tokens,
        snap.decode_ns,
        snap.decode_tokens,
        snap.decode_forward_ns,
        snap.decode_output_ns
    );

    let mut gpu_parts = Vec::new();
    for pt in llm_gpu_profiler::snapshot() {
        if pt.calls > 0 || pt.total_ns > 0 {
            gpu_parts.push(format!(
                "\"{}\":{{\"ns\":{},\"calls\":{}}}",
                pt.phase.label(),
                pt.total_ns,
                pt.calls
            ));
        }
    }
    let gpu_phase_ns = format!("{{{}}}", gpu_parts.join(","));

    let mut notes = Vec::new();
    if !llm_gpu_profiler::enabled() {
        notes.push("GPU timestamps not active (device or flag)".into());
    }
    if falls > 0 {
        notes.push(format!("resident fallbacks={falls}"));
    }
    if hits == 0 && tok_s.is_some() {
        notes.push("no resident hits counted - check path".into());
    }
    let _ = Phase::COUNT;

    llm_gpu_profiler::set_enabled(false);

    DecodeTimeline {
        model: model.display().to_string(),
        tokens,
        tok_s,
        wall_ms,
        resident_hits: hits,
        resident_fallbacks: falls,
        host_phase_ns,
        gpu_phase_ns,
        notes: notes.join("; "),
    }
}

impl DecodeTimeline {
    pub fn format_report(&self) -> String {
        format!(
            "Decode timeline\n  model:     {}\n  tokens:    {}\n  tok_s:     {}\n  wall_ms:   {:.2}\n  resident:  hits={} fallbacks={}\n  host_ns:   {}\n  gpu_ns:    {}\n  notes:     {}\n",
            self.model,
            self.tokens,
            self.tok_s
                .map(|t| format!("{t:.4}"))
                .unwrap_or_else(|| "fail".into()),
            self.wall_ms,
            self.resident_hits,
            self.resident_fallbacks,
            self.host_phase_ns,
            self.gpu_phase_ns,
            self.notes
        )
    }
}
