//! Runtime-config toggles read by the real inference path (decode / prefill /
//! attention / GEMM selection). Each is a process-global flag with a `set_*` /
//! `*_enabled` pair; env vars override where documented. Also the GEMM backend
//! selector. Pure code motion — behaviour unchanged.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Mutex;

// ── Decode budget override ────────────────────────────────────────────────────
// A bounded, fixed decode-token count gives stable, comparable tok/s. The real
// decode loop reads this once per call; 0 = use the production `DECODE_TOKEN_BUDGET`.

static DECODE_BUDGET_OVERRIDE: AtomicU32 = AtomicU32::new(0);

/// Set a fixed decode-token budget for benchmarking (0 = production default).
#[inline]
pub fn set_decode_budget_override(n: u32) {
    DECODE_BUDGET_OVERRIDE.store(n, Ordering::Relaxed);
}

/// When budget override is active, ignore EOS so A/B runs a fixed token count
/// (prevents early-stop from inflating/deflating tok/s on short prompts).
#[inline]
pub fn decode_budget_fixed_tokens() -> bool {
    DECODE_BUDGET_OVERRIDE.load(Ordering::Relaxed) > 0
}

/// Current decode-budget override (0 = none).
#[inline]
pub fn decode_budget_override() -> u32 {
    DECODE_BUDGET_OVERRIDE.load(Ordering::Relaxed)
}

// ── Wall-clock inference timeout override (batch / overnight jobs) ────────────
// 0 = use production `INFERENCE_TIMEOUT_MS` (30s interactive). Batch profile may
// raise this to hours for multi-system health differential analysis overnight.
static INFERENCE_TIMEOUT_OVERRIDE_MS: AtomicU64 = AtomicU64::new(0);

/// Set wall-clock decode timeout in ms (0 = production default 30_000).
#[inline]
pub fn set_inference_timeout_override_ms(ms: u64) {
    INFERENCE_TIMEOUT_OVERRIDE_MS.store(ms, Ordering::Relaxed);
}

/// Effective timeout: override if non-zero, else env `QUALIA_INFERENCE_TIMEOUT_MS`, else 30s.
#[inline]
pub fn inference_timeout_ms() -> u64 {
    let o = INFERENCE_TIMEOUT_OVERRIDE_MS.load(Ordering::Relaxed);
    if o > 0 {
        return o;
    }
    if let Ok(s) = std::env::var("QUALIA_INFERENCE_TIMEOUT_MS") {
        if let Ok(v) = s.parse::<u64>() {
            if v > 0 {
                return v;
            }
        }
    }
    30_000
}

// ── A1a GPU top-k toggle (D18) ────────────────────────────────────────────────
// Default ON after widening the output gate and adding the allocation-free top-1 path. The decode
// loop reads only block winners from the GPU instead of full vocabulary chunks when no sieve mask is
// active.
// Set `QUALIA_LLM_GPU_TOPK=0` to force the full-logit argmax fallback.
static GPU_TOPK: AtomicBool = AtomicBool::new(true);

/// Enable/disable the GPU top-k decode path (`QUALIA_LLM_GPU_TOPK`).
#[inline]
pub fn set_gpu_topk(on: bool) {
    GPU_TOPK.store(on, Ordering::Relaxed);
}

/// Whether the GPU top-k decode path is active. The env var overrides the flag in BOTH directions
/// (`0`/`false` → off, `1`/`true` → on); otherwise the process default (ON) applies.
#[inline]
pub fn gpu_topk_enabled() -> bool {
    match std::env::var("QUALIA_LLM_GPU_TOPK").ok().as_deref() {
        Some("0") | Some("false") => false,
        Some("1") | Some("true") => true,
        _ => GPU_TOPK.load(Ordering::Relaxed),
    }
}

// ── A1b ternary-FFN toggle (D3/D7) ────────────────────────────────────────────
// Additive, default-OFF: when a `.q42` ternary container is booted, routes its FFN
// GEMMs through the resident 2-bit GPU kernel (`TernaryFfnResident`). OFF runs the
// SAME ternary weights via the CPU oracle — so ON-vs-OFF isolates the GPU-kernel win
// on identical weights, and ternary-container-vs-Q8 (a0) is the headline FFN number.
static TERNARY_FFN: AtomicBool = AtomicBool::new(false);

/// Enable/disable the resident 2-bit GPU ternary-FFN path (`QUALIA_LLM_TERNARY_FFN`).
#[inline]
pub fn set_ternary_ffn(on: bool) {
    TERNARY_FFN.store(on, Ordering::Relaxed);
}

/// Whether the GPU ternary-FFN path is active (atomic flag OR the env var). When false, ternary
/// FFN GEMMs fall back to the CPU oracle (correct, slower) — the toggle's OFF baseline.
#[inline]
pub fn ternary_ffn_enabled() -> bool {
    TERNARY_FFN.load(Ordering::Relaxed)
        || matches!(
            std::env::var("QUALIA_LLM_TERNARY_FFN").ok().as_deref(),
            Some("1") | Some("true")
        )
}

// Native attention projection split: K/V matmuls run through cooperative GEMV, then the attention
// shader consumes the projected rows via `proj_row_stride` only for RoPE + KV-cache writes. Default
// ON after the no-readback fused K/V path replaced the old diagnostic readback implementation.
// Set `QUALIA_LLM_PREPROJECT_ATTN=0` to force the legacy in-attention projection path.
static ATTN_PREPROJECT: AtomicBool = AtomicBool::new(true);

#[inline]
pub fn set_attention_preproject(on: bool) {
    ATTN_PREPROJECT.store(on, Ordering::Relaxed);
}

#[inline]
pub fn attention_preproject_enabled() -> bool {
    // W5b Phase 4b: the dict-coded write happens in `write_kv_head` (the attention pass); the fused
    // pre-projection bypasses it, so force it off in dict mode.
    if kv_dict_enabled() {
        return false;
    }
    match std::env::var("QUALIA_LLM_PREPROJECT_ATTN").ok().as_deref() {
        Some("0") | Some("false") => false,
        Some("1") | Some("true") => true,
        _ => ATTN_PREPROJECT.load(Ordering::Relaxed),
    }
}

// Native attention tail fusion: Q-attention writes its output to a GPU buffer, and o_proj consumes
// that buffer directly after K and V are both present in the KV cache. Default ON (native): removes
// one submit->wait round-trip per layer while preserving token identity against the proven readback
// path. Set `QUALIA_LLM_FUSE_ATTN_O=0` to force the older Q-readback + o_proj path.
static ATTN_O_FUSE: AtomicBool = AtomicBool::new(true);

#[inline]
pub fn set_attention_o_fuse(on: bool) {
    ATTN_O_FUSE.store(on, Ordering::Relaxed);
}

#[inline]
pub fn attention_o_fuse_enabled() -> bool {
    // W5b Phase 4b: the fused Q+O tail reads K/V from the cache on the GPU path; keep the plain
    // read_k/read_v (dict-aware) path in dict mode.
    if kv_dict_enabled() {
        return false;
    }
    match std::env::var("QUALIA_LLM_FUSE_ATTN_O").ok().as_deref() {
        Some("0") | Some("false") => false,
        Some("1") | Some("true") => true,
        _ => ATTN_O_FUSE.load(Ordering::Relaxed),
    }
}

// ── Phase 2: resident weights toggle ──────────────────────────────────────────
// Default ON (native). Each layer's q/k/v/o/gate/up/down weight is uploaded to its own resident
// VRAM buffer once (keyed by the GGUF tensor byte_offset) and reused every token, instead of
// re-`write_buffer`ing the (up to ~50 MB for a 3B FFN tensor) weight into the shared GEMM buffer
// on every GEMM, every token. For a 3B F16 model that re-upload is ~5 GB/token of PCIe traffic —
// the decode bottleneck. Set `QUALIA_LLM_RESIDENT_WEIGHTS=0` to force the per-token re-upload (the
// A/B OFF baseline) — useful for measuring the win or on VRAM-constrained GPUs.
static RESIDENT_WEIGHTS: AtomicBool = AtomicBool::new(true);

/// Enable/disable the resident per-tensor weight buffers (`QUALIA_LLM_RESIDENT_WEIGHTS`).
#[inline]
pub fn set_resident_weights(on: bool) {
    RESIDENT_WEIGHTS.store(on, Ordering::Relaxed);
}

/// Whether native GEMM should bind resident per-tensor weight buffers (upload-once) rather than
/// re-uploading the weight every token. Env forces either direction; otherwise the atomic flag.
#[inline]
pub fn resident_weights_enabled() -> bool {
    match std::env::var("QUALIA_LLM_RESIDENT_WEIGHTS").ok().as_deref() {
        Some("0") | Some("false") => false,
        Some("1") | Some("true") => true,
        _ => RESIDENT_WEIGHTS.load(Ordering::Relaxed),
    }
}

// ── Resident-token decode toggle (single fence per token) ─────────────────────
// Default ON (native). Keeps the hidden state in VRAM for the WHOLE token: all 32 layers
// (RMSNorm/residuals as GPU elem ops) + output norm + chunked logits top-1 are encoded into ONE
// command submit with ONE blocking fence and a ~400 B candidate readback — replacing the legacy
// ~107 submit→wait round-trips/token (measured ~24% pure fence time on SmolLM2-360M, A2000,
// Vulkan). Any per-model ineligibility (unsupported quant, no resident logits, sieve mask, CPU
// attention) falls back to the legacy per-layer path. `QUALIA_LLM_RESIDENT_DECODE=0` forces the
// legacy path (the A/B baseline + the differential-test comparator).
static RESIDENT_DECODE: AtomicBool = AtomicBool::new(true);

/// Enable/disable the resident-token single-fence decode (`QUALIA_LLM_RESIDENT_DECODE`).
#[inline]
pub fn set_resident_decode(on: bool) {
    RESIDENT_DECODE.store(on, Ordering::Relaxed);
}

/// Whether native decode should run the GPU-resident single-fence token path.
#[inline]
pub fn resident_decode_enabled() -> bool {
    match std::env::var("QUALIA_LLM_RESIDENT_DECODE").ok().as_deref() {
        Some("0") | Some("false") => false,
        Some("1") | Some("true") => true,
        _ => RESIDENT_DECODE.load(Ordering::Relaxed),
    }
}

// ── W3: resident single-fence-per-chunk prefill toggle ────────────────────────
// Default ON (verified). When on (and the model is eligible — GPU-eligible weights, coop GEMV, no
// active sparse-attention route), each prefill chunk of ≤PREFILL_CHUNK_SIZE prompt tokens populates
// the KV cache in ONE command submit / ONE fence (all 32 layers batched + resident hidden state)
// instead of the legacy per-layer + per-token Q/FFN loop (~640 submit→wait round-trips for a 10-token
// prompt). Delivers TTFT and the batched-forward primitive W6a-verify needs; on a fast discrete GPU
// the steady-state win is latent (prefill is compute-bound), the fence win lands on
// edge/mobile/under-load. Passed the `a3a` gate on SmolLM2-360M Q8 (A2000): the batched RMSNorm
// reduces in the same sequential order as the legacy CPU path, so the KV it writes is BYTE-IDENTICAL
// → decode-identical, with the int8 KV cache both ON and OFF. Any ineligibility falls back to the
// legacy `dispatch_prefill_chunk` path unchanged. `QUALIA_LLM_RESIDENT_PREFILL=0` forces legacy.
static RESIDENT_PREFILL: AtomicBool = AtomicBool::new(true);

/// Enable/disable the resident single-fence-per-chunk prefill path (`QUALIA_LLM_RESIDENT_PREFILL`).
#[inline]
pub fn set_resident_prefill(on: bool) {
    RESIDENT_PREFILL.store(on, Ordering::Relaxed);
}

/// Whether native prefill should run the GPU-resident single-fence-per-chunk arena.
#[inline]
pub fn resident_prefill_enabled() -> bool {
    match std::env::var("QUALIA_LLM_RESIDENT_PREFILL").ok().as_deref() {
        Some("0") | Some("false") => false,
        Some("1") | Some("true") => true,
        _ => RESIDENT_PREFILL.load(Ordering::Relaxed),
    }
}

// ── W6a: prompt-lookup speculative decode toggle (ADR 0010) ───────────────────
// Default ON (ADR 0010, directed by Timothy). When on (and no sieve/sampler/route is active), the
// decode loop drafts the next few tokens by n-gram prompt-lookup, verifies them in ONE batched
// forward (`verify_draft_batch`), and emits the longest greedily-agreeing prefix + the model's own
// correction token. Output matches ordinary decode everywhere except rare genuine near-ties (the a1a
// phenomenon: the model is ambivalent, both tokens equally valid, and a ULP-level difference between
// the batched and single-token forwards flips the pick). The win is pure latency on repetitive /
// quoting / structured / code text (several tokens per forward, measured ~3–12×); on novel text it
// drafts little and costs ~nothing.
//
// This is the MODE SWITCH. Change modes three ways: (1) env `QUALIA_LLM_SPEC_DECODE=0` (off) / `=1`
// (on) at launch — the desktop/daemon reads it; (2) `set_spec_decode(bool)` at runtime (the UI /
// host calls this); (3) `spec_decode_enabled()` to read the effective mode. The env var, when set,
// overrides the runtime flag in BOTH directions. See ADR 0010 for the rationale.
static SPEC_DECODE: AtomicBool = AtomicBool::new(true);

/// Enable/disable prompt-lookup speculative decode (`QUALIA_LLM_SPEC_DECODE`). Runtime mode switch —
/// the desktop UI / host calls this to flip between exact single-token decode and speculative decode.
#[inline]
pub fn set_spec_decode(on: bool) {
    SPEC_DECODE.store(on, Ordering::Relaxed);
}

/// Whether the decode loop should run prompt-lookup speculative decode (the effective mode: env var
/// wins if set, else the runtime flag). Read this to reflect the current mode in a UI.
#[inline]
pub fn spec_decode_enabled() -> bool {
    match std::env::var("QUALIA_LLM_SPEC_DECODE").ok().as_deref() {
        Some("0") | Some("false") => false,
        Some("1") | Some("true") => true,
        _ => SPEC_DECODE.load(Ordering::Relaxed),
    }
}

// ── W5a: int8 KV cache toggle ─────────────────────────────────────────────────
// Default ON (verified). When on (and head_dim % 4 == 0), the KV cache is stored as packed int8
// lanes + one f32 scale per (slot, kv_head) instead of f32 — ~3.8× less KV memory (80→21 MiB @
// ctx 1024) and ~3.8× less attention memory bandwidth (the decode bottleneck). Passed the gate on
// SmolLM2-360M Q8: ΔPPL +0.05% (≪ the 5% bar), coherent, Vulkan-parity tok/s (see `w5a_int8_kv`).
// Read once at model load (`ensure_kv_cache`), so set it BEFORE the model loads. Models with
// head_dim not a multiple of 4 transparently fall back to the f32 layout. `QUALIA_LLM_KV_INT8=0`
// forces the f32 KV cache (the A/B baseline).
static KV_INT8: AtomicBool = AtomicBool::new(true);

/// Enable/disable the int8 KV cache (`QUALIA_LLM_KV_INT8`).
#[inline]
pub fn set_kv_int8(on: bool) {
    KV_INT8.store(on, Ordering::Relaxed);
}

/// Whether the KV cache should be int8-quantized.
#[inline]
pub fn kv_int8_enabled() -> bool {
    match std::env::var("QUALIA_LLM_KV_INT8").ok().as_deref() {
        Some("0") | Some("false") => false,
        Some("1") | Some("true") => true,
        _ => KV_INT8.load(Ordering::Relaxed),
    }
}

// ── W5b Phase 4b: sparse-dictionary KV cache ──────────────────────────────────
// Store each KV vector as its k-sparse dictionary code instead of f32/int8, reconstructing in the
// attention shader on read (~3-4× smaller than int8). Read once at model load (`ensure_kv_cache`); a
// certified dictionary must be installed (`kv_dict_runtime::load_certified`) whose head_dim matches the
// model, else the layout transparently falls back. Default OFF — it trades memory for reconstruct
// compute, so it only wins on memory-bound / long-context targets (see the Phase 4b plan).
static KV_DICT: AtomicBool = AtomicBool::new(false);

/// Enable/disable the sparse-dictionary KV cache (`QUALIA_LLM_KV_DICT`).
#[inline]
pub fn set_kv_dict(on: bool) {
    KV_DICT.store(on, Ordering::Relaxed);
}

/// Whether the KV cache should use the installed sparse dictionary.
#[inline]
pub fn kv_dict_enabled() -> bool {
    match std::env::var("QUALIA_LLM_KV_DICT").ok().as_deref() {
        Some("0") | Some("false") => false,
        Some("1") | Some("true") => true,
        _ => KV_DICT.load(Ordering::Relaxed),
    }
}

// ── W2: exact sampler config ──────────────────────────────────────────────────
// Process-global sampler config, read ONCE at decode start (like the decode budget). `None` ⇒
// greedy argmax (the pre-W2 default; a1a/a1c/a1d byte-identical). `Some(cfg)` with cfg.temperature
// > 0 activates the CPU sampling chain in `crate::sampler`. Set per-request by the host/MCP layer.
static SAMPLER_CONFIG: Mutex<Option<crate::sampler::SamplerConfig>> = Mutex::new(None);

/// Install the decode sampler config (`None` restores greedy argmax).
#[inline]
pub fn set_sampler_config(cfg: Option<crate::sampler::SamplerConfig>) {
    if let Ok(mut g) = SAMPLER_CONFIG.lock() {
        // A greedy config is equivalent to None — normalize so the decode loop can skip the
        // full-logits readback entirely when nothing non-greedy is requested.
        *g = cfg.filter(|c| !c.is_greedy());
    }
}

/// The active decode sampler config, if a non-greedy one is installed.
#[inline]
pub fn sampler_config() -> Option<crate::sampler::SamplerConfig> {
    SAMPLER_CONFIG.lock().ok().and_then(|g| *g)
}

// ── R9: DOMINO constrained-decoding toggle ────────────────────────────────────
// Process-global DOMINO masker. When active, the decode loop applies the
// grammar constraint mask before sampling. The masker owns a token trie
// built from the vocabulary (cold path). Set per-request by the host layer
// when VibeScript constrained generation is requested.
static DOMINO_MASKER: Mutex<Option<crate::inference::DominoMasker>> = Mutex::new(None);

/// Install a DOMINO masker for constrained decoding. The masker must be
/// built from the tokenizer vocabulary. Pass `None` to disable.
pub fn set_domino_masker(masker: Option<crate::inference::DominoMasker>) {
    if let Ok(mut g) = DOMINO_MASKER.lock() {
        *g = masker;
    }
}

/// Whether DOMINO constrained decoding is active.
pub fn domino_active() -> bool {
    DOMINO_MASKER
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|m| m.is_active()))
        .unwrap_or(false)
}

/// Apply the DOMINO mask to logits if a masker is installed and active.
/// Returns the sampled token using `sample_constrained`, or `None` if no
/// masker is installed (caller should use plain `sample`).
pub fn domino_sample(
    sampler: &mut crate::sampler::SamplerState,
    logits: &mut [f32],
    ctx: &[u32],
) -> Option<u32> {
    let mut g = DOMINO_MASKER.lock().ok()?;
    let masker = g.as_mut()?;
    if !masker.is_active() {
        return None;
    }
    Some(sampler.sample_constrained(logits, ctx, masker))
}

/// Feed a decoded token's bytes back into the DOMINO grammar state machine
/// so the grammar advances for the next token's mask. No-op when no masker
/// is installed or it is inactive (T53/W11).
///
/// This MUST be called after every token selected under constrained decoding,
/// otherwise the grammar state never advances and every token gets masked
/// against the initial state.
pub fn domino_feed_token(bytes: &[u8]) {
    if let Ok(mut g) = DOMINO_MASKER.lock() {
        if let Some(masker) = g.as_mut() {
            masker.feed_token(bytes);
        }
    }
}

/// Reset the DOMINO grammar state to the initial state (e.g. for a new
/// generation turn). No-op when no masker is installed (T53/W11).
pub fn domino_reset() {
    if let Ok(mut g) = DOMINO_MASKER.lock() {
        if let Some(masker) = g.as_mut() {
            masker.reset();
        }
    }
}

// ── Phase 3: FFN fusion toggle ────────────────────────────────────────────────
// Default ON (native). Runs the whole pre-norm FFN — gate GEMM, up GEMM, GPU SiLU·mul,
// down GEMM — in ONE command submit with intermediates kept in VRAM, so a layer costs ONE
// submit→wait round-trip instead of three (the gate/up/down readbacks + CPU SiLU·mul between
// them). Requires resident weights (it binds resident weight buffers); falls back to the
// per-GEMM path when resident is off or a tensor is GPU-ineligible. `QUALIA_LLM_FFN_FUSION=0`
// forces the per-GEMM path (the A/B OFF baseline).
static FFN_FUSION: AtomicBool = AtomicBool::new(true);

/// Enable/disable the fused single-submit FFN path (`QUALIA_LLM_FFN_FUSION`).
#[inline]
pub fn set_ffn_fusion(on: bool) {
    FFN_FUSION.store(on, Ordering::Relaxed);
}

/// Whether the native FFN should run fused (one submit/layer) rather than three GEMM round-trips.
#[inline]
pub fn ffn_fusion_enabled() -> bool {
    match std::env::var("QUALIA_LLM_FFN_FUSION").ok().as_deref() {
        Some("0") | Some("false") => false,
        Some("1") | Some("true") => true,
        _ => FFN_FUSION.load(Ordering::Relaxed),
    }
}

/// Set when a resident decode plan was built with `fused_ffn.wgsl` in the mega-pass (T-A1).
static FFN_FUSION_IN_RESIDENT: AtomicBool = AtomicBool::new(false);

#[inline]
pub fn set_ffn_fusion_in_resident(on: bool) {
    FFN_FUSION_IN_RESIDENT.store(on, Ordering::Relaxed);
}

/// True when the last-built resident plan wires fused FFN expansion (not just the flag).
#[inline]
pub fn ffn_fusion_in_resident() -> bool {
    FFN_FUSION_IN_RESIDENT.load(Ordering::Relaxed)
}

// ── FFN quant → f16 promotion (opt-in; bandwidth vs dequant trade-off) ─
// Default OFF. Microbench on small GEMMs favoured f16, but full Llama-3.2-3B on
// A2000 12GB measured **slower** with FFN f16 (~2.1 tok/s) than Q4_K SoA (~2.6)
// — 4× weight traffic outweighs dequant savings when memory-bound. Opt in with
// `QUALIA_LLM_FFN_F16=1` on higher-bandwidth GPUs / smaller FFN dims.
static FFN_F16: AtomicBool = AtomicBool::new(false);

/// Enable/disable FFN quant→f16 promotion at resident-plan build (`QUALIA_LLM_FFN_F16`).
#[inline]
pub fn set_ffn_f16(on: bool) {
    FFN_F16.store(on, Ordering::Relaxed);
}

/// Whether FFN weights should be promoted to f16 in VRAM for decode/prefill GEMV.
#[inline]
pub fn ffn_f16_enabled() -> bool {
    match std::env::var("QUALIA_LLM_FFN_F16").ok().as_deref() {
        Some("0") | Some("false") => false,
        Some("1") | Some("true") => true,
        _ => FFN_F16.load(Ordering::Relaxed),
    }
}

// ── 0.0.21: cooperative GEMV kernel toggle ────────────────────────────────────
// Default ON (native), verified. Routes native GEMV work through the cooperative
// one-workgroup-per-row kernel (`coop_gemv`: coalesced reads + per-thread dequant +
// shared-memory reduction) instead of the naive 1-thread/row `main`. The fused FFN path also selects
// this cooperative entry point for gate/up/down GEMMs, so decode keeps one FFN readback per layer
// while using the faster row reducer. The naive GEMV is the measured decode bottleneck
// (compute/ALU-bound: uncoalesced strided reads + serial accumulate; it also makes Q4_K slower than
// F16). `QUALIA_LLM_COOP_GEMV=0` forces the naive kernel (the A/B OFF baseline).
// Earlier A2000 / Llama-3.2-3B-F16 per-GEMM verification: 2.39→3.22 tok/s (+35% over naive).
static COOP_GEMV: AtomicBool = AtomicBool::new(true);

/// Enable/disable the cooperative GEMV decode path (`QUALIA_LLM_COOP_GEMV`).
#[inline]
pub fn set_coop_gemv(on: bool) {
    COOP_GEMV.store(on, Ordering::Relaxed);
}

/// Whether native GEMM should run the cooperative `coop_gemv` kernel rather than the naive
/// 1-thread/row `main`. Env forces either direction; otherwise the atomic flag (default OFF).
#[inline]
pub fn coop_gemv_enabled() -> bool {
    match std::env::var("QUALIA_LLM_COOP_GEMV").ok().as_deref() {
        Some("0") | Some("false") => false,
        Some("1") | Some("true") => true,
        _ => COOP_GEMV.load(Ordering::Relaxed),
    }
}

/// Rows per workgroup for multi-row coop GEMV (`coop_gemv_mr` in fused_transformer.wgsl).
/// Must stay in lock-step with WGSL `COOP_ROWS`.
pub const COOP_GEMV_ROWS: u32 = 8;

/// Workgroup count for coop GEMV dispatch: `ceil(n_out / COOP_GEMV_ROWS)`.
#[inline]
pub fn coop_gemv_workgroups(n_out: u32) -> u32 {
    n_out.div_ceil(COOP_GEMV_ROWS).max(1)
}

// ── W8: coopmat (tensor-core) GEMM selection seam ─────────────────────────────
// Default OFF. The forge already has the self-activating coopmat path
// (`wgsl_forge::gemm_f32_tc` → `coopmat_usable()` runtime probe): on wgpu 29.0.3 the WGSL coopmat
// multiply returns zeros (#9741), so `coopmat_usable()` is `false` and the tier stays dormant; it
// self-activates the moment a wgpu release / soft-fork carries the fix. This toggle is the
// INFERENCE-side seam: when on AND coopmat is genuinely usable AND the matmul dims fit the 8×8×8 tile
// (m,n,k multiples of 8 — so batched prefill, not the m=1 decode GEMV), the GEMM backend selector
// reports `Coopmat`. Until the inference-side coopmat kernel is wired (which needs the wgpu fix), an
// eligible `Coopmat` selection logs its readiness and falls back to `CoopGemv` — the plumbing is ready
// and visible, self-activating with the forge. `QUALIA_LLM_COOPMAT=1` arms it.
static COOPMAT_GEMM: AtomicBool = AtomicBool::new(false);

/// Arm/disarm the coopmat (tensor-core) GEMM selection seam (`QUALIA_LLM_COOPMAT`).
#[inline]
pub fn set_coopmat_gemm(on: bool) {
    COOPMAT_GEMM.store(on, Ordering::Relaxed);
}

/// Whether the coopmat GEMM seam is armed (env forces either direction; else the flag). Being armed
/// does not mean coopmat runs — see [`coopmat_gemm_usable`], which also requires the hardware probe.
#[inline]
pub fn coopmat_gemm_enabled() -> bool {
    match std::env::var("QUALIA_LLM_COOPMAT").ok().as_deref() {
        Some("0") | Some("false") => false,
        Some("1") | Some("true") => true,
        _ => COOPMAT_GEMM.load(Ordering::Relaxed),
    }
}

/// Whether coopmat is BOTH armed and genuinely usable on this device right now (the forge runtime
/// probe passes — i.e. the wgpu #9741 fix is present). `false` on wgpu 29.0.3. Feature-guarded: with
/// `wgsl-forge` off there is no probe, so coopmat is never usable.
#[inline]
pub fn coopmat_gemm_usable() -> bool {
    if !coopmat_gemm_enabled() {
        return false;
    }
    #[cfg(all(feature = "wgsl-forge", not(target_arch = "wasm32")))]
    {
        crate::wgsl_forge::coopmat_usable()
    }
    #[cfg(not(all(feature = "wgsl-forge", not(target_arch = "wasm32"))))]
    {
        false
    }
}

/// GEMM backend the inference layer selects for a given matmul shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GemmBackend {
    /// Naive 1-thread/row `main` kernel (the correctness floor / A-B baseline).
    Naive,
    /// Cooperative one-workgroup-per-row `coop_gemv` (coalesced reads + shared-mem reduce).
    CoopGemv,
    /// WGSL cooperative-matrix (tensor-core) tile — selected only when armed, usable, and the dims
    /// fit the 8×8×8 tile. Self-activates via the forge when wgpu #9741 lands.
    Coopmat,
}

/// Select the GEMM backend for an `m×k×n` matmul: coopmat when armed+usable and all dims are 8-mult
/// (the tensor-core tile — batched prefill, not the m=1 decode GEMV); else cooperative GEMV when
/// enabled; else naive. Pure + total, so it is unit-tested without a GPU.
#[inline]
pub fn select_gemm_backend(m: usize, k: usize, n: usize) -> GemmBackend {
    let tile_fits = m % 8 == 0 && k % 8 == 0 && n % 8 == 0 && m.min(k).min(n) > 0;
    if coopmat_gemm_usable() && tile_fits {
        GemmBackend::Coopmat
    } else if coop_gemv_enabled() {
        GemmBackend::CoopGemv
    } else {
        GemmBackend::Naive
    }
}

// ── 0.0.21: resident-activation decode toggle ─────────────────────────────────
// readback happens, after the final layer — replacing the legacy 2 readbacks/layer (each forced by
// ── #48 correctness path: CPU attention reference ─────────────────────────────
// Route native attention through the wasm-proven CPU SDPA (`cpu_attention_pass`) instead of the
// GPU attention shader (whose output is currently unbounded). Correct-but-slower; opt-in.
static CPU_ATTENTION: AtomicBool = AtomicBool::new(false);

/// Enable/disable the native CPU-attention reference path (`QUALIA_LLM_CPU_ATTENTION`).
#[inline]
pub fn set_cpu_attention(on: bool) {
    CPU_ATTENTION.store(on, Ordering::Relaxed);
}

/// Whether native attention should use the CPU reference.
///
/// **Default OFF** (use the GPU attention path) — as of #49 the GPU path also honors `norm_weight`
/// for prefill K/V and produces coherent output, and it is faster. The CPU SDPA reference remains
/// available as a correctness fallback / cross-check via `QUALIA_LLM_CPU_ATTENTION=1` or
/// [`set_cpu_attention`].
#[inline]
pub fn cpu_attention_enabled() -> bool {
    CPU_ATTENTION.load(Ordering::Relaxed)
        || matches!(
            std::env::var("QUALIA_LLM_CPU_ATTENTION").ok().as_deref(),
            Some("1") | Some("true")
        )
}
