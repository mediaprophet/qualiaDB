//! WASM LLM runtime controls.
//!
//! The native `inference_bench` module owns filesystem-backed benchmark suites and
//! host timing harnesses. Browser LLM still needs the shared decode toggles and
//! phase counters, so this module provides those runtime controls without native
//! mmap/thread benchmark APIs.

use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

static DECODE_BUDGET_OVERRIDE: AtomicU32 = AtomicU32::new(0);
static INFERENCE_TIMEOUT_OVERRIDE_MS: AtomicU64 = AtomicU64::new(0);
static GPU_TOPK: AtomicBool = AtomicBool::new(true);
static TOPK_HITS: AtomicU64 = AtomicU64::new(0);
static ARGMAX_FALLBACKS: AtomicU64 = AtomicU64::new(0);
static RESIDENT_HITS: AtomicU64 = AtomicU64::new(0);
static RESIDENT_FALLBACKS: AtomicU64 = AtomicU64::new(0);
static SAMPLED_TOKENS: AtomicU64 = AtomicU64::new(0);
static SPEC_STEPS: AtomicU64 = AtomicU64::new(0);
static SPEC_DRAFTED: AtomicU64 = AtomicU64::new(0);
static SPEC_ACCEPTED: AtomicU64 = AtomicU64::new(0);
static RESIDENT_PREFILL_HITS: AtomicU64 = AtomicU64::new(0);
static RESIDENT_PREFILL_FALLBACKS: AtomicU64 = AtomicU64::new(0);
static FFN_F16: AtomicBool = AtomicBool::new(true);
static CPU_ATTENTION: AtomicBool = AtomicBool::new(false);
static PREFIX_CACHE_ENABLED: AtomicBool = AtomicBool::new(false);
static KV_POOL_BUDGET_MB: AtomicU32 = AtomicU32::new(0);
static PREFILL_CHUNK_SIZE_OVERRIDE: AtomicU32 = AtomicU32::new(0);

static LOAD_NS: AtomicU64 = AtomicU64::new(0);
static PREFILL_NS: AtomicU64 = AtomicU64::new(0);
static PREFILL_TOKENS: AtomicU64 = AtomicU64::new(0);
static DECODE_NS: AtomicU64 = AtomicU64::new(0);
static DECODE_TOKENS: AtomicU64 = AtomicU64::new(0);
static DECODE_FORWARD_NS: AtomicU64 = AtomicU64::new(0);
static DECODE_OUTPUT_NS: AtomicU64 = AtomicU64::new(0);
static DECODE_ATTN_NS: AtomicU64 = AtomicU64::new(0);
static DECODE_FFN_NS: AtomicU64 = AtomicU64::new(0);
static DECODE_EMPTY_RT_NS: AtomicU64 = AtomicU64::new(0);
static DECODE_EMPTY_RT_N: AtomicU64 = AtomicU64::new(0);

/// Shared `OnceLock` slot for `sampler`.
fn sampler_slot() -> &'static Mutex<Option<crate::sampler::SamplerConfig>> {
    static SLOT: OnceLock<Mutex<Option<crate::sampler::SamplerConfig>>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(None))
}

/// Shared `OnceLock` slot for `domino`.
fn domino_slot() -> &'static Mutex<Option<crate::inference::speculative_decode::DominoMasker>> {
    static SLOT: OnceLock<Mutex<Option<crate::inference::speculative_decode::DominoMasker>>> =
        OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(None))
}

#[inline]
/// Set the `decode_budget_override` flag/override.
pub fn set_decode_budget_override(n: u32) {
    DECODE_BUDGET_OVERRIDE.store(n, Ordering::Relaxed);
}

#[inline]
/// `decode_budget_fixed_tokens` bench helper.
pub fn decode_budget_fixed_tokens() -> bool {
    DECODE_BUDGET_OVERRIDE.load(Ordering::Relaxed) > 0
}

#[inline]
/// `decode_budget_override` bench helper.
pub fn decode_budget_override() -> u32 {
    DECODE_BUDGET_OVERRIDE.load(Ordering::Relaxed)
}

#[inline]
/// Set the `inference_timeout_override_ms` flag/override.
pub fn set_inference_timeout_override_ms(ms: u64) {
    INFERENCE_TIMEOUT_OVERRIDE_MS.store(ms, Ordering::Relaxed);
}

#[inline]
/// `inference_timeout_ms` bench helper.
pub fn inference_timeout_ms() -> u64 {
    let override_ms = INFERENCE_TIMEOUT_OVERRIDE_MS.load(Ordering::Relaxed);
    if override_ms > 0 {
        override_ms
    } else {
        30_000
    }
}

#[inline]
/// Set the `gpu_topk` flag/override.
pub fn set_gpu_topk(on: bool) {
    GPU_TOPK.store(on, Ordering::Relaxed);
}

#[inline]
/// Whether `gpu_topk` is enabled.
pub fn gpu_topk_enabled() -> bool {
    GPU_TOPK.load(Ordering::Relaxed)
}

#[inline]
/// Record a `topk_hit` bench metric event.
pub fn record_topk_hit() {
    TOPK_HITS.fetch_add(1, Ordering::Relaxed);
}

#[inline]
/// Record a `argmax_fallback` bench metric event.
pub fn record_argmax_fallback() {
    ARGMAX_FALLBACKS.fetch_add(1, Ordering::Relaxed);
}

#[inline]
/// Snapshot of the `output_path` counters.
pub fn output_path_counts() -> (u64, u64) {
    (
        TOPK_HITS.load(Ordering::Relaxed),
        ARGMAX_FALLBACKS.load(Ordering::Relaxed),
    )
}

#[inline]
/// Reset the `output_path_counts` counters.
pub fn reset_output_path_counts() {
    TOPK_HITS.store(0, Ordering::Relaxed);
    ARGMAX_FALLBACKS.store(0, Ordering::Relaxed);
}

#[inline]
/// Record a `resident_hit` bench metric event.
pub fn record_resident_hit() {
    RESIDENT_HITS.fetch_add(1, Ordering::Relaxed);
}

#[inline]
/// Record a `resident_fallback` bench metric event.
pub fn record_resident_fallback() {
    RESIDENT_FALLBACKS.fetch_add(1, Ordering::Relaxed);
}

#[inline]
/// Snapshot of the `resident_path` counters.
pub fn resident_path_counts() -> (u64, u64) {
    (
        RESIDENT_HITS.load(Ordering::Relaxed),
        RESIDENT_FALLBACKS.load(Ordering::Relaxed),
    )
}

#[inline]
/// Reset the `resident_path_counts` counters.
pub fn reset_resident_path_counts() {
    RESIDENT_HITS.store(0, Ordering::Relaxed);
    RESIDENT_FALLBACKS.store(0, Ordering::Relaxed);
}

#[inline]
/// Record a `sampled_token` bench metric event.
pub fn record_sampled_token() {
    SAMPLED_TOKENS.fetch_add(1, Ordering::Relaxed);
}

#[inline]
/// `sampled_token_count` bench helper.
pub fn sampled_token_count() -> u64 {
    SAMPLED_TOKENS.load(Ordering::Relaxed)
}

#[inline]
/// Reset the `sampled_token_count` counters.
pub fn reset_sampled_token_count() {
    SAMPLED_TOKENS.store(0, Ordering::Relaxed);
}

#[inline]
/// No-op stub — ternary FFN is not implemented on this build.
pub fn set_ternary_ffn(_on: bool) {}

#[inline]
/// Always `false` — ternary FFN is not implemented on this build.
pub fn ternary_ffn_enabled() -> bool {
    false
}

#[inline]
/// No-op stub — attention preprojection is not implemented on this build.
pub fn set_attention_preproject(_on: bool) {}

#[inline]
/// Always `false` — attention preprojection is not implemented.
pub fn attention_preproject_enabled() -> bool {
    false
}

#[inline]
/// No-op stub — attention-output fusion is not implemented on this build.
pub fn set_attention_o_fuse(_on: bool) {}

#[inline]
/// Always `false` — attention-output fusion is not implemented.
pub fn attention_o_fuse_enabled() -> bool {
    false
}

#[inline]
/// No-op stub — forge attention is not implemented on this build.
pub fn set_forge_attention(_on: bool) {}

#[inline]
/// Always `false` — forge attention is not implemented.
pub fn forge_attention_enabled() -> bool {
    false
}

#[inline]
/// No-op stub — resident weights are not implemented on this build.
pub fn set_resident_weights(_on: bool) {}

#[inline]
/// Always `false` — resident weights are not implemented.
pub fn resident_weights_enabled() -> bool {
    false
}

#[inline]
/// No-op stub — resident decode is not implemented on this build.
pub fn set_resident_decode(_on: bool) {}

#[inline]
/// Always `false` — resident decode is not implemented.
pub fn resident_decode_enabled() -> bool {
    false
}

#[inline]
/// No-op stub — resident prefill is not implemented on this build.
pub fn set_resident_prefill(_on: bool) {}

#[inline]
/// Always `false` — resident prefill is not implemented.
pub fn resident_prefill_enabled() -> bool {
    false
}

#[inline]
/// No-op stub — speculative decode is not implemented on this build.
pub fn set_spec_decode(_on: bool) {}

#[inline]
/// Always `false` — speculative decode is not implemented.
pub fn spec_decode_enabled() -> bool {
    false
}

#[inline]
/// Record a `spec_step` bench metric event.
pub fn record_spec_step(drafted: u64, accepted: u64) {
    SPEC_STEPS.fetch_add(1, Ordering::Relaxed);
    SPEC_DRAFTED.fetch_add(drafted, Ordering::Relaxed);
    SPEC_ACCEPTED.fetch_add(accepted, Ordering::Relaxed);
}

#[inline]
/// Snapshot of the `spec_decode` counters.
pub fn spec_decode_counts() -> (u64, u64, u64) {
    (
        SPEC_STEPS.load(Ordering::Relaxed),
        SPEC_DRAFTED.load(Ordering::Relaxed),
        SPEC_ACCEPTED.load(Ordering::Relaxed),
    )
}

#[inline]
/// Reset the `spec_decode_counts` counters.
pub fn reset_spec_decode_counts() {
    SPEC_STEPS.store(0, Ordering::Relaxed);
    SPEC_DRAFTED.store(0, Ordering::Relaxed);
    SPEC_ACCEPTED.store(0, Ordering::Relaxed);
}

#[inline]
/// Record a `resident_prefill_hit` bench metric event.
pub fn record_resident_prefill_hit() {
    RESIDENT_PREFILL_HITS.fetch_add(1, Ordering::Relaxed);
}

#[inline]
/// Record a `resident_prefill_fallback` bench metric event.
pub fn record_resident_prefill_fallback() {
    RESIDENT_PREFILL_FALLBACKS.fetch_add(1, Ordering::Relaxed);
}

#[inline]
/// Snapshot of the `resident_prefill` counters.
pub fn resident_prefill_counts() -> (u64, u64) {
    (
        RESIDENT_PREFILL_HITS.load(Ordering::Relaxed),
        RESIDENT_PREFILL_FALLBACKS.load(Ordering::Relaxed),
    )
}

#[inline]
/// Reset the `resident_prefill_counts` counters.
pub fn reset_resident_prefill_counts() {
    RESIDENT_PREFILL_HITS.store(0, Ordering::Relaxed);
    RESIDENT_PREFILL_FALLBACKS.store(0, Ordering::Relaxed);
}

#[inline]
/// No-op stub — int8 KV cache is not implemented on this build.
pub fn set_kv_int8(_on: bool) {}

#[inline]
/// Always `false` — int8 KV cache is not implemented.
pub fn kv_int8_enabled() -> bool {
    false
}

#[inline]
/// No-op stub — dictionary-compressed KV is not implemented on this build.
pub fn set_kv_dict(_on: bool) {}

#[inline]
/// Always `false` — dictionary KV compression is not implemented.
pub fn kv_dict_enabled() -> bool {
    false
}

/// Set the `sampler_config` flag/override.
pub fn set_sampler_config(cfg: Option<crate::sampler::SamplerConfig>) {
    if let Ok(mut slot) = sampler_slot().lock() {
        *slot = cfg;
    }
}

/// `sampler_config` bench helper.
pub fn sampler_config() -> Option<crate::sampler::SamplerConfig> {
    sampler_slot().lock().ok().and_then(|slot| slot.clone())
}

/// Install (or remove) the browser's constrained-decoding masker. Constructing
/// it is an authoring action; sampling reuses the masker's internal scratch.
pub fn set_domino_masker(masker: Option<crate::inference::speculative_decode::DominoMasker>) {
    if let Ok(mut slot) = domino_slot().lock() {
        *slot = masker;
    }
}

/// `domino_active` bench helper.
pub fn domino_active() -> bool {
    domino_slot()
        .lock()
        .map(|slot| slot.as_ref().is_some_and(|masker| masker.is_active()))
        .unwrap_or(false)
}

/// `domino_reset` bench helper.
pub fn domino_reset() {
    if let Ok(mut slot) = domino_slot().lock() {
        if let Some(masker) = slot.as_mut() {
            masker.reset();
        }
    }
}

/// `domino_feed_token` bench helper.
pub fn domino_feed_token(bytes: &[u8]) {
    if let Ok(mut slot) = domino_slot().lock() {
        if let Some(masker) = slot.as_mut() {
            masker.feed_token(bytes);
        }
    }
}

/// `domino_sample` bench helper.
pub fn domino_sample(
    state: &mut crate::sampler::SamplerState,
    logits: &mut [f32],
    context: &[u32],
) -> Option<u32> {
    let mut slot = domino_slot().lock().ok()?;
    let masker = slot.as_mut()?;
    if !masker.is_active() {
        return None;
    }
    masker.apply_mask_preserving(logits);
    Some(state.sample(logits, context))
}
#[inline]
/// No-op stub — FFN fusion is not implemented on this build.
pub fn set_ffn_fusion(_on: bool) {}

#[inline]
/// Always `false` — FFN fusion is not implemented.
pub fn ffn_fusion_enabled() -> bool {
    false
}

#[inline]
/// No-op stub — resident FFN fusion is not implemented on this build.
pub fn set_ffn_fusion_in_resident(_on: bool) {}

#[inline]
/// Always `false` — resident FFN fusion is not implemented.
pub fn ffn_fusion_in_resident() -> bool {
    false
}

#[inline]
/// Set the `ffn_f16` flag/override.
pub fn set_ffn_f16(on: bool) {
    FFN_F16.store(on, Ordering::Relaxed);
}

#[inline]
/// Whether `ffn_f16` is enabled.
pub fn ffn_f16_enabled() -> bool {
    FFN_F16.load(Ordering::Relaxed)
}

#[inline]
/// No-op stub — cooperative GEMV is not implemented on this build.
pub fn set_coop_gemv(_on: bool) {}

#[inline]
/// Always `false` — cooperative GEMV is not implemented.
pub fn coop_gemv_enabled() -> bool {
    false
}

#[inline]
/// Workgroup count heuristic for the cooperative GEMV path
/// (`n_out` clamped ≥ 1); the path itself is unimplemented.
pub fn coop_gemv_workgroups(n_out: u32) -> u32 {
    n_out.max(1)
}

#[inline]
/// No-op stub — cooperative-matrix GEMM is not implemented on this build.
pub fn set_coopmat_gemm(_on: bool) {}

#[inline]
/// Always `false` — cooperative-matrix GEMM is not implemented.
pub fn coopmat_gemm_enabled() -> bool {
    false
}

#[inline]
/// Always `false` — no usable cooperative-matrix device path.
pub fn coopmat_gemm_usable() -> bool {
    false
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GemmBackend {
    Naive,
    CoopGemv,
    Coopmat,
}

#[inline]
/// GEMM backend selection — always `Naive` on this build
/// (cooperative paths unimplemented).
pub fn select_gemm_backend(_m: usize, _k: usize, _n: usize) -> GemmBackend {
    GemmBackend::Naive
}

#[inline]
/// Set the `cpu_attention` flag/override.
pub fn set_cpu_attention(on: bool) {
    CPU_ATTENTION.store(on, Ordering::Relaxed);
}

#[inline]
/// Whether `cpu_attention` is enabled.
pub fn cpu_attention_enabled() -> bool {
    CPU_ATTENTION.load(Ordering::Relaxed)
}

#[inline]
/// Record a `empty_rt` bench metric event.
pub fn record_empty_rt(total_ns: u64, n: u64) {
    DECODE_EMPTY_RT_NS.store(total_ns, Ordering::Relaxed);
    DECODE_EMPTY_RT_N.store(n, Ordering::Relaxed);
}

#[inline]
/// `empty_rt` bench helper.
pub fn empty_rt() -> (u64, u64) {
    (
        DECODE_EMPTY_RT_NS.load(Ordering::Relaxed),
        DECODE_EMPTY_RT_N.load(Ordering::Relaxed),
    )
}

#[inline]
/// `gpu_wait_count` bench helper.
pub fn gpu_wait_count() -> u64 {
    #[cfg(any(not(target_arch = "wasm32"), feature = "portal", feature = "wasm-llm"))]
    {
        crate::gguf_bridge::gpu_wait_count()
    }
    #[cfg(not(any(not(target_arch = "wasm32"), feature = "portal", feature = "wasm-llm")))]
    {
        0
    }
}

#[inline]
/// Accumulate `decode_forward_ns` into the phase metrics.
pub fn add_decode_forward_ns(ns: u64) {
    DECODE_FORWARD_NS.fetch_add(ns, Ordering::Relaxed);
}

#[inline]
/// Accumulate `decode_output_ns` into the phase metrics.
pub fn add_decode_output_ns(ns: u64) {
    DECODE_OUTPUT_NS.fetch_add(ns, Ordering::Relaxed);
}

#[inline]
/// Accumulate `decode_attn_ns` into the phase metrics.
pub fn add_decode_attn_ns(ns: u64) {
    DECODE_ATTN_NS.fetch_add(ns, Ordering::Relaxed);
}

#[inline]
/// Accumulate `decode_ffn_ns` into the phase metrics.
pub fn add_decode_ffn_ns(ns: u64) {
    DECODE_FFN_NS.fetch_add(ns, Ordering::Relaxed);
}

#[inline]
/// `decode_attn_ffn` bench helper.
pub fn decode_attn_ffn() -> (u64, u64) {
    (
        DECODE_ATTN_NS.load(Ordering::Relaxed),
        DECODE_FFN_NS.load(Ordering::Relaxed),
    )
}

#[inline]
/// Reset the `phase_metrics` counters.
pub fn reset_phase_metrics() {
    LOAD_NS.store(0, Ordering::Relaxed);
    PREFILL_NS.store(0, Ordering::Relaxed);
    PREFILL_TOKENS.store(0, Ordering::Relaxed);
    DECODE_NS.store(0, Ordering::Relaxed);
    DECODE_TOKENS.store(0, Ordering::Relaxed);
    DECODE_FORWARD_NS.store(0, Ordering::Relaxed);
    DECODE_OUTPUT_NS.store(0, Ordering::Relaxed);
    DECODE_ATTN_NS.store(0, Ordering::Relaxed);
    DECODE_FFN_NS.store(0, Ordering::Relaxed);
    DECODE_EMPTY_RT_NS.store(0, Ordering::Relaxed);
    DECODE_EMPTY_RT_N.store(0, Ordering::Relaxed);
    #[cfg(any(not(target_arch = "wasm32"), feature = "portal", feature = "wasm-llm"))]
    crate::gguf_bridge::reset_gpu_wait_count();
}

#[inline]
/// Record a `load_ns` bench metric event.
pub fn record_load_ns(ns: u64) {
    LOAD_NS.store(ns, Ordering::Relaxed);
}

#[inline]
/// Record a `prefill` bench metric event.
pub fn record_prefill(ns: u64, tokens: u64) {
    PREFILL_NS.store(ns, Ordering::Relaxed);
    PREFILL_TOKENS.store(tokens, Ordering::Relaxed);
}

#[inline]
/// Record a `decode` bench metric event.
pub fn record_decode(ns: u64, tokens: u64) {
    DECODE_NS.store(ns, Ordering::Relaxed);
    DECODE_TOKENS.store(tokens, Ordering::Relaxed);
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct LlmPhaseSnapshot {
    pub load_ns: u64,
    pub prefill_ns: u64,
    pub prefill_tokens: u64,
    pub decode_ns: u64,
    pub decode_tokens: u64,
    pub decode_forward_ns: u64,
    pub decode_output_ns: u64,
}

#[inline]
/// `phase_snapshot` bench helper.
pub fn phase_snapshot() -> LlmPhaseSnapshot {
    LlmPhaseSnapshot {
        load_ns: LOAD_NS.load(Ordering::Relaxed),
        prefill_ns: PREFILL_NS.load(Ordering::Relaxed),
        prefill_tokens: PREFILL_TOKENS.load(Ordering::Relaxed),
        decode_ns: DECODE_NS.load(Ordering::Relaxed),
        decode_tokens: DECODE_TOKENS.load(Ordering::Relaxed),
        decode_forward_ns: DECODE_FORWARD_NS.load(Ordering::Relaxed),
        decode_output_ns: DECODE_OUTPUT_NS.load(Ordering::Relaxed),
    }
}

#[inline]
/// Set the prefix-cache enabled state.
pub fn set_prefix_cache_enabled(on: bool) {
    PREFIX_CACHE_ENABLED.store(on, Ordering::Relaxed);
}

#[inline]
/// Whether prefix caching is enabled.
pub fn prefix_cache_enabled() -> bool {
    PREFIX_CACHE_ENABLED.load(Ordering::Relaxed)
}

#[inline]
/// Set the KV pool budget in megabytes.
pub fn set_kv_pool_budget_mb(mb: u32) {
    KV_POOL_BUDGET_MB.store(mb, Ordering::Relaxed);
}

#[inline]
/// Return the configured KV pool budget in megabytes.
pub fn kv_pool_budget_mb() -> u32 {
    KV_POOL_BUDGET_MB.load(Ordering::Relaxed)
}

#[inline]
/// Set the prefill chunk-size override in tokens.
pub fn set_prefill_chunk_size_override(sz: u32) {
    PREFILL_CHUNK_SIZE_OVERRIDE.store(sz, Ordering::Relaxed);
}

#[inline]
/// Return the prefill chunk-size override in tokens.
pub fn prefill_chunk_size_override() -> u32 {
    PREFILL_CHUNK_SIZE_OVERRIDE.load(Ordering::Relaxed)
}

fn prompt_precision_slot() -> &'static Mutex<String> {
    static SLOT: OnceLock<Mutex<String>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(String::new()))
}

#[inline]
/// Set the prompt precision mode.
pub fn set_prompt_precision_mode(mode: &str) {
    if let Ok(mut slot) = prompt_precision_slot().lock() {
        *slot = mode.to_string();
    }
}

#[inline]
/// Return the configured prompt precision mode.
pub fn prompt_precision_mode() -> String {
    prompt_precision_slot()
        .lock()
        .map(|s| s.clone())
        .unwrap_or_default()
}
