//! Result-set serializers for the harness output: pretty JSON, CSV (header +
//! one row per result), and a human-readable stdout table. Pure code motion —
//! behaviour unchanged.

use super::*;

// ── Reporting ─────────────────────────────────────────────────────────────────

/// Pretty-printed JSON for a result set.
pub fn results_to_json(results: &[BenchResult]) -> String {
    serde_json::to_string_pretty(results).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"))
}

/// CSV (header + one row per result).
pub fn results_to_csv(results: &[BenchResult]) -> String {
    let mut s = String::new();
    s.push_str(
        "label,quantization,n_layer,mapped_bytes,prompt_tokens,output_tokens,\
cold_ttft_ms,cold_total_ms,warm_ttft_ms,warm_total_ms,\
load_ms,prefill_ms,prefill_tok_s,decode_ms,decode_tok_s,directml,gpu_ts,\
gpu_adapter,gpu_backend,gpu_device_type,gpu_adapter_features,gpu_enabled_features\n",
    );
    for r in results {
        s.push_str(&format!(
            "{},{},{},{},{},{},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.2},{:.3},{:.2},{},{},{},{},{},{},{}\n",
            r.label.replace(',', " "),
            r.quantization,
            r.model.n_layer,
            r.model.mapped_bytes,
            r.prompt_tokens,
            r.output_tokens,
            r.cold_ttft_ms,
            r.cold_total_ms,
            r.warm_ttft_ms,
            r.warm_total_ms,
            r.load_ms,
            r.prefill_ms,
            r.prefill_tok_s,
            r.decode_ms,
            r.decode_tok_s,
            r.model.directml_enabled,
            r.gpu_timestamp_supported,
            r.gpu.adapter.replace(',', " "),
            r.gpu.backend,
            r.gpu.device_type,
            r.gpu.adapter_feature_flags.replace(',', " "),
            r.gpu.enabled_feature_flags.replace(',', " "),
        ));
    }
    s
}

/// Human-readable table for stdout.
pub fn results_to_table(results: &[BenchResult]) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        "{:<22} {:>6} {:>10} {:>10} {:>10} {:>11} {:>11}\n",
        "model", "layers", "coldTTFT", "warmTTFT", "prefill/s", "decode/s", "decode_ms"
    ));
    s.push_str(&"-".repeat(86));
    s.push('\n');
    for r in results {
        s.push_str(&format!(
            "{:<22} {:>6} {:>9.0}m {:>9.0}m {:>10.1} {:>11.2} {:>10.1}\n",
            r.label,
            r.model.n_layer,
            r.cold_ttft_ms,
            r.warm_ttft_ms,
            r.prefill_tok_s,
            r.decode_tok_s,
            r.decode_ms,
        ));
    }
    s
}
