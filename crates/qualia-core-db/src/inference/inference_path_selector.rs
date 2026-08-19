//! Device-optimal inference path selection.
//!
//! Design intent (Timothy): a **benchmark utility** identifies the fastest method on
//! *this* machine (wgpu DX12 / Vulkan / Metal / GL, optional CUDA lane, CPU) and the
//! product **picks that path** — not a static hierarchy.
//!
//! ## What is selected
//! | Axis | Candidates | Measured by |
//! |------|------------|-------------|
//! | **API backend** | dx12, vulkan, metal, gl | Hardware passport GEMV + decode-proxy |
//! | **Compute lane** | portable (resident wgpu), cuda (forge TC + Q4 SoA GEMV) | Capability flags + optional micro-policy |
//! | **Quant profile** | INT4 SoA weights, INT8 KV, quant-graph quality | A2000-class bandwidth + rights mode |
//! | **Prefill vs decode** | TC GEMM when batch/dims allow; GEMV for m=1 decode | Shape policy, not GEMV score alone |
//!
//! Vulkan and DX12 both use the **same resident weight plan** (VRAM-sticky multi-weight);
//! there is no separate "Vulkan multi-weight" — multi-weight without host RT is the
//! **resident decode** path. CUDA multi-weight is the optional densify/Q4 device slab.
//!
//! Env pins always win: `QUALIA_WGPU_BACKEND`, `QUALIA_INFERENCE_MODE`, `QUALIA_PATH_AUTO=0`.

#![cfg(not(target_arch = "wasm32"))]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

use crate::inference_modes::{set_inference_mode, InferenceMode};
use crate::llm_bench::{
    set_coop_gemv, set_ffn_fusion, set_kv_int8, set_resident_decode, set_resident_prefill,
    set_resident_weights,
};

/// How decode math is executed after the wgpu API backend is chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComputeLane {
    /// Single-fence resident decode/prefill on wgpu (DX12/Vulkan/Metal).
    PortableResident,
    /// Prefer CUDA TC prefill + Q4 device GEMV when eligible; else portable.
    CudaAccelerated,
}

impl ComputeLane {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PortableResident => "portable-resident",
            Self::CudaAccelerated => "cuda-accelerated",
        }
    }
}

/// Quantization / quality profile for consumer GPUs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuantProfile {
    /// INT4 SoA weights + INT8 KV (A2000 bandwidth sweet spot).
    Int4SoaInt8Kv,
    /// Same as above + quant-graph hybrid quality net.
    Int4SoaGraphHybrid,
}

impl QuantProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Int4SoaInt8Kv => "int4-soa+int8-kv",
            Self::Int4SoaGraphHybrid => "int4-soa+int8-kv+graph",
        }
    }
}

/// Full selected plan for this process / host.
#[derive(Debug, Clone, PartialEq)]
pub struct InferencePathPlan {
    /// `dx12` | `vulkan` | `metal` | `gl` | unknown
    pub wgpu_backend: Option<String>,
    pub compute_lane: ComputeLane,
    pub quant: QuantProfile,
    /// Use tensor-core dense GEMM for prefill when dims allow (CUDA lane).
    pub prefill_prefer_tc: bool,
    /// Decode uses GEMV (m=1); never pretend TC replaces decode.
    pub decode_is_gemv: bool,
    /// Human-readable reason chain.
    pub rationale: String,
    /// Passport circuit label that won, if any.
    pub winning_circuit: Option<String>,
    pub decode_proxy_tok_s: Option<f64>,
    pub gemv_ms: Option<f64>,
}

static APPLIED: AtomicBool = AtomicBool::new(false);
static LAST_PLAN: OnceLock<std::sync::Mutex<Option<InferencePathPlan>>> = OnceLock::new();

fn last_plan_slot() -> &'static std::sync::Mutex<Option<InferencePathPlan>> {
    LAST_PLAN.get_or_init(|| std::sync::Mutex::new(None))
}

/// Last plan applied (or resolved without apply).
pub fn last_inference_path_plan() -> Option<InferencePathPlan> {
    last_plan_slot().lock().ok().and_then(|g| g.clone())
}

/// Whether auto path selection is enabled (`QUALIA_PATH_AUTO` default on).
pub fn path_auto_enabled() -> bool {
    match std::env::var("QUALIA_PATH_AUTO").ok().as_deref() {
        Some("0") | Some("false") | Some("off") => false,
        _ => true,
    }
}

/// Resolve the optimal plan from passport + host capabilities (does not mutate process yet).
pub fn resolve_inference_path_plan() -> InferencePathPlan {
    let mut rationale = Vec::new();

    // ── 1. wgpu API backend from passport (or env pin) ─────────────────────
    let env_backend = std::env::var("QUALIA_WGPU_BACKEND")
        .ok()
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty());

    let (wgpu_backend, winning, decode_tok, gemv_ms) = if let Some(ref b) = env_backend {
        rationale.push(format!("wgpu backend pinned by QUALIA_WGPU_BACKEND={b}"));
        (Some(b.clone()), None, None, None)
    } else if let Some(p) =
        crate::hardware_passport::read_passport(&crate::hardware_passport::default_cache_path())
    {
        let best = p.matrix.best().cloned();
        let token = p.preferred_inference_backend.clone().or_else(|| {
            best.as_ref().and_then(|c| {
                crate::hardware_passport::backend_env_token(&c.backend).map(str::to_string)
            })
        });
        if let Some(ref c) = best {
            rationale.push(format!(
                "passport winner: {} [{}/{}] gemv={:.3}ms decode={:?}",
                c.label,
                format!("{:?}", c.kind),
                c.backend,
                c.ms_per_gemv,
                c.decode_proxy_tok_s
            ));
        } else {
            rationale.push("passport present but empty matrix".into());
        }
        (
            token,
            best.as_ref().map(|c| c.label.clone()),
            best.as_ref().and_then(|c| c.decode_proxy_tok_s),
            best.as_ref().map(|c| c.ms_per_gemv),
        )
    } else {
        rationale.push(
            "no hardware passport — run `qualia-cli llm passport --reprobe --decode-proxy <model> --apply-env-hint`"
                .into(),
        );
        // Platform defaults: Metal on Apple, else leave unset (gpu_context picks).
        let def = if cfg!(target_os = "macos") || cfg!(target_os = "ios") {
            rationale.push("default metal on Apple".into());
            Some("metal".into())
        } else {
            None
        };
        (def, None, None, None)
    };

    // ── 2. Compute lane: CUDA only when toolkit present + not slower policy ─
    let cuda_caps = crate::wgsl_forge::dispatch::caps().cuda;
    let mode_pin = std::env::var("QUALIA_INFERENCE_MODE").ok();
    let compute_lane = if let Some(ref m) = mode_pin {
        if matches!(
            m.to_ascii_lowercase().as_str(),
            "cuda" | "cuda-tc" | "cudatc" | "tc" | "1"
        ) {
            rationale.push("compute lane pinned by QUALIA_INFERENCE_MODE=cuda".into());
            ComputeLane::CudaAccelerated
        } else {
            rationale.push(format!("compute lane portable (mode pin {m})"));
            ComputeLane::PortableResident
        }
    } else if cuda_caps && prefer_cuda_lane_heuristic() {
        rationale.push(
            "CUDA toolkit detected — lane=cuda-accelerated for prefill TC / Q4 device GEMV; decode still GEMV-primary"
                .into(),
        );
        ComputeLane::CudaAccelerated
    } else {
        if cuda_caps {
            rationale.push(
                "CUDA present but portable-resident preferred (set QUALIA_INFERENCE_MODE=cuda to force)"
                    .into(),
            );
        } else {
            rationale.push("no CUDA — portable resident (wgpu multi-weight in VRAM)".into());
        }
        ComputeLane::PortableResident
    };

    // ── 3. Quant + quality ────────────────────────────────────────────────
    let rights = crate::inference_modes::rights_mode_enabled()
        || matches!(
            mode_pin
                .as_deref()
                .map(|s| s.to_ascii_lowercase())
                .as_deref(),
            Some("quant-graph")
                | Some("graph")
                | Some("hybrid")
                | Some("2")
                | Some("fast-verify")
                | Some("fast_verify")
                | Some("3")
        );
    // Prefer FastVerify when operator asks for ollama-like + post heal without mid-decode tax.
    if matches!(
        mode_pin
            .as_deref()
            .map(|s| s.to_ascii_lowercase())
            .as_deref(),
        Some("fast-verify") | Some("fast_verify") | Some("ollama-like") | Some("3")
    ) {
        rationale.push(
            "mode pin fast-verify: full-speed decode then post-turn CML/graph self-heal".into(),
        );
    }
    let quant = if rights {
        rationale.push("quant profile: INT4 SoA + INT8 KV + quant-graph hybrid".into());
        QuantProfile::Int4SoaGraphHybrid
    } else {
        rationale.push("quant profile: INT4 SoA weights + INT8 KV (consumer bandwidth)".into());
        QuantProfile::Int4SoaInt8Kv
    };

    let prefill_prefer_tc = matches!(compute_lane, ComputeLane::CudaAccelerated);
    if prefill_prefer_tc {
        rationale.push("prefill: prefer TC dense GEMM when m,n,k multiples of 16".into());
    }
    rationale
        .push("decode: always GEMV (m=1) — tensor cores do not replace single-token GEMV".into());
    rationale.push(
        "multi-weight: wgpu resident plan keeps layer weights in VRAM (Vulkan/DX12/Metal); CUDA slab is optional densify path"
            .into(),
    );

    InferencePathPlan {
        wgpu_backend,
        compute_lane,
        quant,
        prefill_prefer_tc,
        decode_is_gemv: true,
        rationale: rationale.join(" | "),
        winning_circuit: winning,
        decode_proxy_tok_s: decode_tok,
        gemv_ms,
    }
}

/// Heuristic: enable CUDA lane when not explicitly disabled and runtime path exists.
fn prefer_cuda_lane_heuristic() -> bool {
    // Default OFF for CUDA as primary decode — measured A2000 still favored portable resident.
    // Opt-in via QUALIA_PREFER_CUDA=1 or mode=cuda. Prefill can still use forge when mode=cuda.
    matches!(
        std::env::var("QUALIA_PREFER_CUDA").ok().as_deref(),
        Some("1") | Some("true") | Some("on")
    )
}

/// Apply plan to process toggles (once per process unless `force`).
///
/// Sets inference mode / bench flags. Does **not** set `QUALIA_WGPU_BACKEND` if already
/// set (gpu_context already honors env + passport). Optionally writes passport preferred
/// fields for the next process.
pub fn apply_inference_path_plan(plan: &InferencePathPlan, force: bool) -> bool {
    if force {
        APPLIED.store(true, Ordering::SeqCst);
    } else if APPLIED.swap(true, Ordering::SeqCst) {
        return false; // already applied this process
    }

    // Resident always on for product path.
    set_resident_decode(true);
    set_resident_prefill(true);
    set_resident_weights(true);
    set_coop_gemv(true);
    set_ffn_fusion(true);
    set_kv_int8(true);

    match plan.quant {
        QuantProfile::Int4SoaInt8Kv => {
            // Leave mode as portable unless compute lane says cuda.
        }
        QuantProfile::Int4SoaGraphHybrid => {
            if std::env::var("QUALIA_INFERENCE_MODE").is_err() {
                // Prefer post-turn FastVerify for consumer rights path when not pinned.
                if crate::inference_modes::rights_mode_enabled()
                    || matches!(
                        std::env::var("QUALIA_PREFER_FAST_VERIFY").ok().as_deref(),
                        Some("1") | Some("true")
                    )
                {
                    set_inference_mode(InferenceMode::FastVerify);
                } else {
                    set_inference_mode(InferenceMode::QuantGraph);
                }
            }
        }
    }

    match plan.compute_lane {
        ComputeLane::PortableResident => {
            if std::env::var("QUALIA_INFERENCE_MODE").is_err()
                && !matches!(plan.quant, QuantProfile::Int4SoaGraphHybrid)
            {
                set_inference_mode(InferenceMode::Portable);
            }
        }
        ComputeLane::CudaAccelerated => {
            if std::env::var("QUALIA_INFERENCE_MODE").is_err() {
                set_inference_mode(InferenceMode::CudaTc);
            }
            crate::wgsl_forge::dispatch::ensure_cuda_runtime_path();
        }
    }

    // Soft-set backend for child processes / logs if unset.
    if std::env::var("QUALIA_WGPU_BACKEND").is_err() {
        if let Some(ref b) = plan.wgpu_backend {
            // SAFETY: process-local config before GPU init; documented operator surface.
            std::env::set_var("QUALIA_WGPU_BACKEND", b);
            log::info!("path_select|set_env|QUALIA_WGPU_BACKEND={b}");
        }
    }

    log::info!(
        "path_select|applied|backend={:?}|lane={}|quant={}|prefill_tc={}|{}",
        plan.wgpu_backend,
        plan.compute_lane.as_str(),
        plan.quant.as_str(),
        plan.prefill_prefer_tc,
        plan.rationale
    );

    if let Ok(mut g) = last_plan_slot().lock() {
        *g = Some(plan.clone());
    }
    true
}

/// Resolve + apply if `QUALIA_PATH_AUTO` enabled. Called from `bootstrap_inference_mode`.
pub fn bootstrap_optimal_inference_path() -> InferencePathPlan {
    let plan = resolve_inference_path_plan();
    if path_auto_enabled() {
        apply_inference_path_plan(&plan, false);
    } else {
        log::info!("path_select|skipped|QUALIA_PATH_AUTO=0|{}", plan.rationale);
        if let Ok(mut g) = last_plan_slot().lock() {
            *g = Some(plan.clone());
        }
    }
    plan
}

/// Operator-facing summary (CLI / doctor).
pub fn format_path_plan(plan: &InferencePathPlan) -> String {
    format!(
        "InferencePathPlan\n  wgpu_backend:     {}\n  compute_lane:     {}\n  quant:            {}\n  prefill_TC:       {}\n  decode:           GEMV (m=1)\n  winning_circuit:  {}\n  decode_proxy:     {}\n  gemv_ms:          {}\n  rationale:\n    {}\n",
        plan.wgpu_backend.as_deref().unwrap_or("(platform default)"),
        plan.compute_lane.as_str(),
        plan.quant.as_str(),
        plan.prefill_prefer_tc,
        plan.winning_circuit.as_deref().unwrap_or("—"),
        plan.decode_proxy_tok_s
            .map(|t| format!("{t:.2} tok/s"))
            .unwrap_or_else(|| "—".into()),
        plan.gemv_ms
            .map(|m| format!("{m:.3}"))
            .unwrap_or_else(|| "—".into()),
        plan.rationale.replace(" | ", "\n    "),
    )
}

/// Probe passport (optional re-probe) and print/apply plan. Used by CLI.
pub fn run_path_select_cli(reprobe: bool, apply: bool) -> InferencePathPlan {
    if reprobe {
        let _ = crate::hardware_passport::load_or_probe(
            &crate::hardware_passport::default_cache_path(),
            crate::hardware_passport::PASSPORT_GEMV_N,
        );
    }
    let plan = resolve_inference_path_plan();
    if apply {
        apply_inference_path_plan(&plan, true);
        // Persist preferred backend into passport if we have one.
        if let Some(mut p) =
            crate::hardware_passport::read_passport(&crate::hardware_passport::default_cache_path())
        {
            if let Some(ref b) = plan.wgpu_backend {
                p.preferred_inference_backend = Some(b.clone());
            }
            let _ = crate::hardware_passport::write_passport(
                &p,
                &crate::hardware_passport::default_cache_path(),
            );
        }
    }
    plan
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[serial_test::serial]
    fn resolve_does_not_panic() {
        let p = resolve_inference_path_plan();
        assert!(p.decode_is_gemv);
        assert!(!p.rationale.is_empty());
    }

    #[test]
    fn format_contains_axes() {
        let p = InferencePathPlan {
            wgpu_backend: Some("dx12".into()),
            compute_lane: ComputeLane::PortableResident,
            quant: QuantProfile::Int4SoaInt8Kv,
            prefill_prefer_tc: false,
            decode_is_gemv: true,
            rationale: "test".into(),
            winning_circuit: Some("A2000".into()),
            decode_proxy_tok_s: Some(2.5),
            gemv_ms: Some(0.11),
        };
        let s = format_path_plan(&p);
        assert!(s.contains("dx12"));
        assert!(s.contains("portable-resident"));
        assert!(s.contains("GEMV"));
    }
}
