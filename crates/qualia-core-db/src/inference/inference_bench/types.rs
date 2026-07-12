//! Benchmark config + result types: [`BenchConfig`] (a case to drive) and the
//! JSON/CSV-serializable [`BenchResult`] with its [`ModelMeta`] / [`BenchGpuMeta`]
//! sub-records. Pure code motion — behaviour unchanged.

use serde::Serialize;

// ── Config / result ───────────────────────────────────────────────────────────

/// One benchmark case: a model + prompt to drive through the real path.
#[derive(Debug, Clone)]
pub struct BenchConfig {
    /// Human-readable row label, e.g. "SmolLM2-360M Q8".
    pub label: String,
    /// Path to the GGUF on disk.
    pub model_path: String,
    /// Descriptive quantization tag for the report (e.g. "Q8_0").
    pub quantization: String,
    /// Prompt to run.
    pub prompt: String,
    /// Fixed decode-token count for a bounded, comparable measurement (0 = production default).
    pub decode_tokens: u32,
    /// Warm repeats to average over (≥1). Cold is always a single fresh run.
    pub warm_repeats: u32,
}

impl BenchConfig {
    pub fn new(
        label: impl Into<String>,
        model_path: impl Into<String>,
        quantization: impl Into<String>,
        prompt: impl Into<String>,
    ) -> Self {
        Self {
            label: label.into(),
            model_path: model_path.into(),
            quantization: quantization.into(),
            prompt: prompt.into(),
            decode_tokens: 64,
            warm_repeats: 3,
        }
    }
}

/// Model metadata captured at residency mount (best-effort).
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct ModelMeta {
    pub n_layer: u32,
    pub n_head: u32,
    pub n_kv_head: u32,
    pub mapped_bytes: u64,
    pub kv_cache_bytes: u64,
    pub directml_enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct BenchGpuMeta {
    pub adapter: String,
    pub backend: String,
    pub device_type: String,
    pub adapter_feature_flags: String,
    pub enabled_feature_flags: String,
    pub subgroup_min_size: u32,
    pub subgroup_max_size: u32,
    pub cooperative_matrix_tiles: usize,
}

impl BenchGpuMeta {
    // Widened from module-private `fn` to `pub(super)` so `runner::run_bench`
    // (a sibling submodule) can build this from the shared GPU context.
    pub(super) fn from_shared_context(ctx: &crate::gpu_context::SharedGpuContext) -> Self {
        let caps = &ctx.adapter_caps;
        Self {
            adapter: caps.name.clone(),
            backend: caps.backend_label().to_string(),
            device_type: caps.device_type_label().to_string(),
            adapter_feature_flags: caps.features.compact_flags(),
            enabled_feature_flags: ctx.enabled_features.compact_flags(),
            subgroup_min_size: caps.subgroup_min_size,
            subgroup_max_size: caps.subgroup_max_size,
            cooperative_matrix_tiles: caps.cooperative_matrix_tile_count,
        }
    }
}

/// A single benchmark row — JSON/CSV serializable.
#[derive(Debug, Clone, Serialize)]
pub struct BenchResult {
    pub label: String,
    pub model_path: String,
    pub quantization: String,
    pub model: ModelMeta,
    pub gpu: BenchGpuMeta,

    pub prompt_tokens: u64,
    pub output_tokens: u64,

    // Cold: model not resident (includes disk mmap + pipeline build + prefill).
    pub cold_ttft_ms: f64,
    pub cold_total_ms: f64,

    // Warm: model resident (mmap adopted; pipelines rebuilt per call by design).
    pub warm_ttft_ms: f64,
    pub warm_total_ms: f64,

    // Phase split from internal metrics (averaged over warm repeats).
    pub load_ms: f64,
    pub prefill_ms: f64,
    pub prefill_tok_s: f64,
    pub decode_ms: f64,
    pub decode_tok_s: f64,

    /// Whether GPU timestamp-query kernel isolation contributed to these numbers.
    /// `false` for A0.1 (wall-clock); set when A0.2 lands.
    pub gpu_timestamp_supported: bool,
    pub note: String,
}
