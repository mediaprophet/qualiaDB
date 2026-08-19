//! Multi-mode inference — keep parallel approaches, do not dump one pipeline.
//!
//! Timothy (2026-07-09): different *modes* rather than replacing the portable
//! resident path. Modes are selected by env / API; each has a honest role:
//!
//! | Mode | Role |
//! |------|------|
//! | **Portable** | wgpu resident decode/prefill (DX12/Vulkan/Metal) — default product path |
//! | **CudaTc** | Prefer forge CUDA WMMA / TC GEMM when dims allow; fall back to Portable |
//! | **QuantGraph** | Aggressive INT4/INT8 + **mid** hybrid hints + post graph repair |
//! | **FastVerify** | Ollama-like full-speed decode (no mid-token Sentinel tax) → **post-turn** CML/graph self-heal + HTML |
//!
//! Modes compose toggles already in `inference_bench`; they do not invent a
//! second engine. Graph-hybrid quality recovery is neuro-symbolic: LLM proposes,
//! graph + Logic VM grounds, domain engines compute exact subproblems.

use std::sync::atomic::{AtomicU8, Ordering};

/// Active inference approach for this process.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InferenceMode {
    /// wgpu single-fence resident decode/prefill — default, portable.
    Portable = 0,
    /// Prefer CUDA tensor-core dense GEMM via forge when eligible; else Portable.
    CudaTc = 1,
    /// Quantized front-end + graph/Webizen grounding for quality recovery.
    QuantGraph = 2,
    /// Full-speed decode (skip mid-token Sentinel rings) then post-turn verify/heal.
    FastVerify = 3,
}

impl InferenceMode {
    pub const ALL: [InferenceMode; 4] = [
        InferenceMode::Portable,
        InferenceMode::CudaTc,
        InferenceMode::QuantGraph,
        InferenceMode::FastVerify,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            InferenceMode::Portable => "portable",
            InferenceMode::CudaTc => "cuda",
            InferenceMode::QuantGraph => "quant-graph",
            InferenceMode::FastVerify => "fast-verify",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "portable" | "wgpu" | "default" | "0" => Some(Self::Portable),
            "cuda" | "cuda-tc" | "cudatc" | "tc" | "1" => Some(Self::CudaTc),
            "quant-graph" | "quant_graph" | "graph" | "hybrid" | "neuro-symbolic" | "2" => {
                Some(Self::QuantGraph)
            }
            "fast-verify" | "fast_verify" | "fastverify" | "ollama-like" | "post-verify"
            | "verify" | "3" => Some(Self::FastVerify),
            _ => None,
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            InferenceMode::Portable => {
                "wgpu resident decode/prefill (DX12/Vulkan/Metal); Q4_K SoA INT4 coop GEMV; INT8 KV default; no second DirectML device unless QUALIA_DIRECTML=1"
            }
            InferenceMode::CudaTc => {
                "CUDA-capable lane: resident wgpu decode by default (coherent); Q4_K SoA device GEMV when applicable; dense densify+TC decode GEMV is opt-in QUALIA_LLM_CUDA_TC_DECODE=1 (lab — was incoherent when default-on)"
            }
            InferenceMode::QuantGraph => {
                "INT4 (Q4_K SoA) front-end — A2000 bandwidth sweet spot — + graph/Webizen verify to recover quality; INT2 experimental with graph net"
            }
            InferenceMode::FastVerify => {
                "Ollama-like full-speed decode (no mid-token Sentinel) → post-turn quant-graph + CML/HTML self-heal before turn finalises"
            }
        }
    }
}

static MODE: AtomicU8 = AtomicU8::new(InferenceMode::Portable as u8);

#[inline]
fn atomic_inference_mode() -> InferenceMode {
    match MODE.load(Ordering::Relaxed) {
        1 => InferenceMode::CudaTc,
        2 => InferenceMode::QuantGraph,
        3 => InferenceMode::FastVerify,
        _ => InferenceMode::Portable,
    }
}

/// Resolve mode from `QUALIA_INFERENCE_MODE` at the cold configuration boundary.
///
/// A successful environment parse is published to [`MODE`]. Per-token predicates must read
/// that atomic via [`atomic_inference_mode`] instead of allocating a fresh environment string.
pub fn active_inference_mode() -> InferenceMode {
    // Env can override the configured atomic when this cold-boundary API is invoked.
    if let Ok(s) = std::env::var("QUALIA_INFERENCE_MODE") {
        if let Some(m) = InferenceMode::parse(&s) {
            MODE.store(m as u8, Ordering::Relaxed);
            return m;
        }
    }
    atomic_inference_mode()
}

/// Set process mode and apply associated toggles. Env still wins on next read if set.
pub fn set_inference_mode(mode: InferenceMode) {
    MODE.store(mode as u8, Ordering::Relaxed);
    apply_mode_toggles(mode);
    log::info!("LLM_MODE|active|{}|{}", mode.as_str(), mode.description());
}

/// Apply mode-specific defaults without changing the mode atom (used at first infer).
pub fn apply_mode_toggles(mode: InferenceMode) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use crate::llm_bench::{
            set_coop_gemv, set_resident_decode, set_resident_prefill, set_resident_weights,
        };
        // All modes keep resident paths ON by default; they differ in accel / grounding.
        set_resident_decode(true);
        set_resident_prefill(true);
        set_resident_weights(true);
        set_coop_gemv(true);
        match mode {
            InferenceMode::Portable => {
                // Explicit: do not force CUDA TC for dense forge calls.
            }
            InferenceMode::CudaTc => {
                crate::wgsl_forge::dispatch::ensure_cuda_runtime_path();
                // Default: resident mega-pass ON (measured ~6.5–7 tok/s on 3B).
                // QUALIA_LLM_CUDA_DECODE=1 opts into the layer-by-layer CUDA SoA path
                // (P4: device RoPE/KV/SDPA + sticky Q4_K_SOA). Lab / A-B only until
                // it beats resident. Device SDPA requires f32 KV (int8 indices differ).
                match std::env::var("QUALIA_LLM_CUDA_DECODE").ok().as_deref() {
                    Some("1") | Some("true") => {
                        // Keep resident_decode ON — the wgpu resident path handles mixed
                        // quant types (Q4_K_M has Q8_0 V, Q6_K down) and serves as fallback
                        // when the CUDA mega-pass can't run (requires all-Q4_K_SOA weights).
                        // The mega-pass is only attempted when the resident path returns None.
                        // Force f32 KV so device SDPA index formula matches host layout.
                        crate::llm_bench::set_kv_int8(false);
                        // SAFETY: process-local lab toggle; decode-proxy is single-threaded measure.
                        std::env::set_var("QUALIA_LLM_KV_INT8", "0");
                        log::info!(
                            "LLM_MODE|cuda|resident_decode=on|cuda_mega_pass_fallback|f32_kv|device_sdpa|lab_path"
                        );
                    }
                    _ => {
                        // Default excellence path: resident mega-pass + wgpu GEMV.
                        // Dense densify+TC for decode GEMV is OFF unless QUALIA_LLM_CUDA_TC_DECODE=1
                        // (that path was measured incoherent — garbage tokens — 2026-07-24).
                        log::info!(
                            "LLM_MODE|cuda|resident_decode=on|dense_tc_decode=off|use_q4k_soa_device_when_present"
                        );
                    }
                }
            }
            InferenceMode::QuantGraph => {
                // Prefer bandwidth-friendly layouts; graph verify is opt-in at agent layer.
                // FFN f16 promote stays off (measured slower on A2000 for Q4).
                // Refresh fact graph from bundled TSV / QUALIA_GROUNDING_FACTS.
                let n = crate::quant_graph_grounding::seed_facts_from_bundled();
                log::info!("LLM_MODE|quant-graph|facts_seeded|{n}");
            }
            InferenceMode::FastVerify => {
                // Same weight path as portable; quality is *post-turn* only.
                let n = crate::quant_graph_grounding::seed_facts_from_bundled();
                log::info!("LLM_MODE|fast-verify|post_turn_only|facts_seeded|{n}");
            }
        }
    }
    let _ = mode;
}

/// True when dense forge GEMM should try tensor-core path first.
#[inline]
pub fn prefer_tensor_core_gemm() -> bool {
    matches!(atomic_inference_mode(), InferenceMode::CudaTc)
}

/// True when agent should run graph / Webizen grounding after proposals.
/// FastVerify also grounds — but only post-turn (see `post_turn_verify_enabled`).
#[inline]
pub fn quant_graph_grounding_enabled() -> bool {
    matches!(
        active_inference_mode(),
        InferenceMode::QuantGraph | InferenceMode::FastVerify
    )
}

/// Mid-decode Webizen Sentinel / logit-ring governance active?
/// Off in FastVerify so decode matches Ollama-style uninterrupted generation.
#[inline]
pub fn sentinel_mid_decode_enabled() -> bool {
    // Explicit override.
    match std::env::var("QUALIA_SENTINEL_MID").ok().as_deref() {
        Some("1") | Some("true") | Some("on") => return true,
        Some("0") | Some("false") | Some("off") => return false,
        _ => {}
    }
    !matches!(active_inference_mode(), InferenceMode::FastVerify)
}

/// Post-turn verify + self-heal (graph/CML/HTML) before finalising the turn.
#[inline]
pub fn post_turn_verify_enabled() -> bool {
    matches!(
        active_inference_mode(),
        InferenceMode::FastVerify | InferenceMode::QuantGraph
    ) || matches!(
        std::env::var("QUALIA_POST_VERIFY").ok().as_deref(),
        Some("1") | Some("true") | Some("on")
    )
}

/// FastVerify defaults to returning plain healed text; HTML when this is true.
#[inline]
pub fn fast_verify_html_default() -> bool {
    matches!(
        std::env::var("QUALIA_RETURN_VERIFY_HTML").ok().as_deref(),
        Some("1") | Some("true") | Some("on")
    )
}

/// One-shot: resolve env, apply toggles, return mode (call from agent entry).
///
/// Order:
/// 1. Device path selector (passport-ranked backend + lane + quant) when `QUALIA_PATH_AUTO`
/// 2. Rights-mode quant-graph if still unset
/// 3. Active mode toggles (resident, coop GEMV, INT8 KV, …)
pub fn bootstrap_inference_mode() -> InferenceMode {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _plan = crate::inference_path_selector::bootstrap_optimal_inference_path();
        // Application profile (interactive / live-fast / batch overnight).
        let _app = crate::application_profile::bootstrap_application_profile();
    }
    // Rights-grade consumer default: FastVerify (speed + post-heal) unless pinned.
    if matches!(
        std::env::var("QUALIA_RIGHTS_MODE").ok().as_deref(),
        Some("1") | Some("true") | Some("on")
    ) {
        if std::env::var("QUALIA_INFERENCE_MODE").is_err()
            && std::env::var("QUALIA_APP_PROFILE").is_err()
        {
            set_inference_mode(InferenceMode::FastVerify);
        }
        log::info!("LLM_MODE|rights|fast-verify|post_turn_heal");
    }
    // Unpinned default on NVIDIA: FastVerify (smol ~60+ tok/s). Operator can pin
    // QUALIA_INFERENCE_MODE=cuda for large SoA models (3B ~7 tok/s measured).
    if std::env::var("QUALIA_INFERENCE_MODE").is_err()
        && std::env::var("QUALIA_APP_PROFILE").is_err()
        && std::env::var("QUALIA_RIGHTS_MODE").is_err()
    {
        set_inference_mode(InferenceMode::FastVerify);
        log::info!("LLM_MODE|default|fast-verify|consumer_speed");
    }
    let m = active_inference_mode();
    apply_mode_toggles(m);
    m
}

/// True when operator asked for rights-grade defaults (`QUALIA_RIGHTS_MODE`).
#[inline]
pub fn rights_mode_enabled() -> bool {
    matches!(
        std::env::var("QUALIA_RIGHTS_MODE").ok().as_deref(),
        Some("1") | Some("true") | Some("on")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_mode_names() {
        assert_eq!(
            InferenceMode::parse("portable"),
            Some(InferenceMode::Portable)
        );
        assert_eq!(InferenceMode::parse("CUDA"), Some(InferenceMode::CudaTc));
        assert_eq!(
            InferenceMode::parse("quant-graph"),
            Some(InferenceMode::QuantGraph)
        );
        assert_eq!(
            InferenceMode::parse("hybrid"),
            Some(InferenceMode::QuantGraph)
        );
        assert_eq!(
            InferenceMode::parse("fast-verify"),
            Some(InferenceMode::FastVerify)
        );
        assert_eq!(
            InferenceMode::parse("ollama-like"),
            Some(InferenceMode::FastVerify)
        );
        assert!(InferenceMode::parse("nope").is_none());
    }

    #[test]
    #[serial_test::serial]
    fn fast_verify_disables_mid_sentinel() {
        if std::env::var("QUALIA_INFERENCE_MODE").is_ok()
            || std::env::var("QUALIA_SENTINEL_MID").is_ok()
        {
            return;
        }
        set_inference_mode(InferenceMode::FastVerify);
        assert!(!sentinel_mid_decode_enabled());
        assert!(post_turn_verify_enabled());
        set_inference_mode(InferenceMode::Portable);
        assert!(sentinel_mid_decode_enabled());
    }

    #[test]
    #[serial_test::serial]
    fn set_and_read_without_env() {
        // Do not assert env-free if the machine has QUALIA_INFERENCE_MODE set.
        if std::env::var("QUALIA_INFERENCE_MODE").is_ok() {
            return;
        }
        set_inference_mode(InferenceMode::CudaTc);
        assert_eq!(active_inference_mode(), InferenceMode::CudaTc);
        set_inference_mode(InferenceMode::Portable);
        assert_eq!(active_inference_mode(), InferenceMode::Portable);
    }

    #[test]
    #[serial_test::serial]
    fn cold_env_publish_makes_hot_cuda_guard_zero_allocation() {
        let previous = std::env::var("QUALIA_INFERENCE_MODE").ok();
        std::env::set_var("QUALIA_INFERENCE_MODE", "cuda");
        set_inference_mode(InferenceMode::Portable);
        assert_eq!(active_inference_mode(), InferenceMode::CudaTc);
        crate::specialized_libs::computational_geometry::allocation_counter::assert_zero_alloc(
            "atomic_cuda_mode_guard",
            || assert!(prefer_tensor_core_gemm()),
        );
        match previous {
            Some(value) => {
                std::env::set_var("QUALIA_INFERENCE_MODE", value);
                let _ = active_inference_mode();
            }
            None => {
                std::env::remove_var("QUALIA_INFERENCE_MODE");
                set_inference_mode(InferenceMode::Portable);
            }
        }
    }
}
