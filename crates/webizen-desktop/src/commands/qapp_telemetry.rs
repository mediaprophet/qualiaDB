//! QApp analysis contract and LLM telemetry

#![allow(non_snake_case)]

use tauri::command;

// ── QApp ↔ QualiaDB analysis contract ───────────────────────────────────────────
// Mirrors `webizen-studio/src/components/qapp_engine.rs`. The discipline QApps call
#[command]
pub fn wellfair_get_model_lifecycle_status() -> Result<String, String> {
    let state = qualia_client_core::model_lifecycle::get_model_lifecycle_state();
    Ok(qualia_client_core::model_lifecycle::lifecycle_label(state).to_string())
}

#[command]
pub fn wellfair_force_model_lifecycle_phase(phase: u8) -> Result<String, String> {
    // Honest: arbitrary phase overrides are not exposed. Real transitions go through
    // activate / unload / scrub on the orchestrator. Report current state only.
    let current = qualia_client_core::model_lifecycle::lifecycle_label(
        qualia_client_core::model_lifecycle::get_model_lifecycle_state(),
    );
    let _ = phase;
    Err(format!(
        "Force phase is not implemented (requested phase={phase}). \
         Current lifecycle={current}. Activate a model or unload to change state."
    ))
}

/// Live LLM / backend telemetry for Studio HUD (no static marketing numbers).
///
/// `tokens_per_sec` is the last **measured** chat/decode throughput this process,
/// or 0 with `tokens_per_sec_source: "none"` when no turn has completed yet.
#[command]
pub fn wellfair_get_llm_telemetry() -> Result<serde_json::Value, String> {
    let engine = qualia_client_core::api::get_engine_telemetry_fields();
    let backend_settings = qualia_client_core::inference_backend::load_inference_backend_settings();
    let backend = qualia_client_core::inference_backend::backend_label(&backend_settings);
    let backend_kind = format!("{:?}", backend_settings.backend);

    let loaded_model = qualia_client_core::api::get_active_model()
        .or_else(qualia_client_core::api::load_active_model_from_disk)
        .map(|p| {
            p.rsplit(['/', '\\'])
                .next()
                .unwrap_or(p.as_str())
                .to_string()
        })
        .unwrap_or_else(|| "none".to_string());

    let (tokens_per_sec, tokens_per_sec_source) =
        match qualia_client_core::model_lifecycle::get_last_decode_tok_s() {
            Some(t) => (t, "last_completed_turn"),
            None => (0.0, "none"),
        };

    let vram_usage_gb = engine.vram_used_mb as f64 / 1024.0;
    let vram_total_gb = engine.vram_total_mb as f64 / 1024.0;

    let ollama_note = if matches!(
        backend_settings.backend,
        qualia_client_core::chat_agents::AgentBackendKind::Ollama
    ) {
        Some("Optional Ollama harness — not the Qualia in-process engine.")
    } else {
        None
    };

    Ok(serde_json::json!({
        "tokens_per_sec": tokens_per_sec,
        "tokens_per_sec_source": tokens_per_sec_source,
        "tokens_per_sec_at_unix": qualia_client_core::model_lifecycle::get_last_decode_tok_s_at_unix(),
        "vram_usage_gb": vram_usage_gb,
        "vram_total_gb": vram_total_gb,
        "vram_used_mb": engine.vram_used_mb,
        "vram_total_mb": engine.vram_total_mb,
        "loaded_model": loaded_model,
        "model_lifecycle": engine.model_lifecycle,
        "thermal_state": engine.thermal_state,
        "llm_memory_bytes": engine.llm_memory_bytes,
        "kv_cache_used_mb": engine.kv_cache_used_mb,
        "inference_backend": backend,
        "inference_backend_kind": backend_kind,
        "ollama_optional_note": ollama_note,
        "honesty": "live_probe_or_last_measured",
    }))
}
// this via `invoke("qapp_analyze", { request })` when running in the desktop webview;
// the plain-browser demo uses the studio-side deterministic stub instead.

#[derive(Debug, Clone, serde::Deserialize)]
pub struct QappAnalysisRequest {
    pub discipline: String,
    pub fields: Vec<(String, String)>,
    pub notes: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct QappAnalysisResult {
    pub summary: String,
    pub assertions: Vec<String>,
    pub provenance_hash: String,
    pub engine: String,
    pub graph_nodes: usize,
    pub q42_quins: usize,
    pub evidence_weight: f32,
    pub forge_schema_version: u32,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct QualiaComputeProfile {
    pub engine_version: String,
    pub forge_schema_version: u32,
    pub wgpu_api_version: String,
    pub naga_api_version: String,
    pub cudarc_api_version: String,
    pub backend_override: Option<String>,
    pub adapter_name: String,
    pub backend: String,
    pub device_type: String,
    pub vendor_hex: String,
    pub device_hex: String,
    pub driver: String,
    pub driver_info: String,
    pub recommendation: String,
    pub preferred_forge_target: String,
    pub active_forge_target: String,
    pub fallback_note: Option<String>,
    pub features: String,
    pub enabled_features: String,
    pub subgroup_range: String,
    pub cooperative_matrix_tile_count: usize,
    pub max_buffer_size_mib: u64,
    pub max_storage_buffer_binding_size_mib: u64,
    pub max_compute_workgroup_storage_size: u32,
    pub max_compute_invocations_per_workgroup: u32,
    pub max_compute_workgroup_size_x: u32,
    pub max_compute_workgroups_per_dimension: u32,
    pub timestamps_supported: bool,
    pub timestamp_period_ns: f32,
    pub q42_graph_bridge: bool,
    pub available_modules: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ForgePhysicsCertification {
    pub engine_version: String,
    pub forge_schema_version: u32,
    pub kernel: String,
    pub backend: String,
    pub particle_count: usize,
    pub certified: bool,
    pub max_abs_error: f32,
    pub momentum_drift: f32,
    pub elapsed_ms: f64,
    pub q42_provenance: String,
    pub sample_positions: Vec<[f32; 3]>,
    pub note: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ForgeKernelProbe {
    pub kernel: String,
    pub shape: String,
    pub output_elements: usize,
    pub elapsed_ms: f64,
    pub max_abs_error: f32,
    pub certified: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ForgeComputeProbe {
    pub engine_version: String,
    pub forge_schema_version: u32,
    pub backend: String,
    pub initialization_ms: f64,
    pub total_kernel_ms: f64,
    pub all_certified: bool,
    pub q42_provenance: String,
    pub kernels: Vec<ForgeKernelProbe>,
    pub note: String,
}

pub fn qapp_slug(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    out.trim_matches('_').to_string()
}
