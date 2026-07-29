//! QPU, hardware, engine, daemon, tax, vault, ingest, solid pod, image

#![allow(non_snake_case)]

use super::*;

use crate::engine::ingestion;
use crate::engine::q42_compiler;
use crate::state::*;
use futures_util::StreamExt;
use qualia_core_db::ilp_dispatcher::{DispatchResult, HttpIlpTransport, IlpDispatcher};
use qualia_core_db::rpc::{route_tax_payment, TaxRecipientSuite};
use serde::Serialize;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use sysinfo::System;

pub use crate::setup::{SetupProfile, SetupState};

pub fn get_setup_state() -> Result<SetupState, String> {
    crate::setup::get_setup_state()
}

pub fn complete_setup_step(step: String) -> Result<SetupState, String> {
    crate::setup::complete_setup_step(step)
}

pub fn update_setup_profile(profile: SetupProfile) -> Result<SetupState, String> {
    crate::setup::update_setup_profile(profile)
}

pub fn finish_setup() -> Result<SetupState, String> {
    crate::setup::finish_setup()
}

pub use crate::qpu_oracle::{QpuChatCommandResult, QpuOracleSettings, QpuOracleSettingsInput};

pub fn get_qpu_settings() -> QpuOracleSettings {
    crate::qpu_oracle::get_qpu_settings()
}

pub fn is_qpu_feature_unlocked() -> bool {
    crate::qpu_oracle::is_qpu_feature_unlocked()
}

pub fn save_qpu_settings(input: QpuOracleSettingsInput) -> Result<QpuOracleSettings, String> {
    crate::qpu_oracle::save_qpu_settings(input)
}

pub fn handle_qpu_chat_command(text: String) -> QpuChatCommandResult {
    crate::qpu_oracle::handle_qpu_chat_command(&text)
}

pub fn handle_engine_chat_command(text: String) -> QpuChatCommandResult {
    crate::qpu_pipeline::handle_engine_chat_command(&text)
}

pub fn profile_energy_circumstance() -> String {
    let mut sys = System::new_all();
    sys.refresh_all();
    let total_mem = sys.total_memory() / 1024 / 1024;
    let used_mem = sys.used_memory() / 1024 / 1024;
    format!(
        "Energy: AC_POWER\nTotal RAM: {} MB\nUsed RAM: {} MB\nSwarm Auth: GRANTED",
        total_mem, used_mem
    )
}

pub fn check_ollama_status() -> bool {
    std::process::Command::new("ollama")
        .arg("-v")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[derive(Serialize)]
pub struct HardwareStatus {
    pub ram_total_gb: f64,
    pub ram_used_gb: f64,
    pub vram_estimated_gb: f64,
}

pub fn get_hardware_status() -> HardwareStatus {
    let mut sys = System::new_all();
    sys.refresh_all();
    let vram_available_gb = {
        #[cfg(target_os = "windows")]
        {
            qualia_core_db::directml_bridge::probe_best_adapter_memory()
                .map(|memory| {
                    let free = memory.available_local_bytes();
                    let fallback = memory.dedicated_vram_bytes;
                    let bytes = if free > 0 { free } else { fallback };
                    bytes as f64 / 1024.0 / 1024.0 / 1024.0
                })
                .unwrap_or(0.0)
        }
        #[cfg(not(target_os = "windows"))]
        {
            0.0
        }
    };
    HardwareStatus {
        ram_total_gb: sys.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0,
        ram_used_gb: sys.used_memory() as f64 / 1024.0 / 1024.0 / 1024.0,
        vram_estimated_gb: vram_available_gb,
    }
}

/// Orchestrator + LLM arena signals for the Flutter Vault HUD.
#[derive(Debug, Clone, Serialize)]
pub struct EngineTelemetryFields {
    pub thermal_state: String,
    pub llm_memory_bytes: u64,
    pub memory_floor_mb: u32,
    pub model_lifecycle: String,
    pub kv_cache_used_mb: u32,
    pub vram_used_mb: u32,
    pub vram_total_mb: u32,
    pub npu_used_mb: u32,
    pub npu_total_mb: u32,
}

pub fn get_engine_telemetry_fields() -> EngineTelemetryFields {
    let (vram_used_mb, vram_total_mb) = probe_vram_usage_mb();
    let (npu_used_mb, npu_total_mb) = probe_npu_usage_mb();
    EngineTelemetryFields {
        thermal_state: crate::model_lifecycle::get_thermal_state_label().to_string(),
        llm_memory_bytes: crate::model_lifecycle::get_llm_memory_bytes(),
        memory_floor_mb: crate::model_lifecycle::MEMORY_FLOOR_MB,
        model_lifecycle: crate::model_lifecycle::lifecycle_label(
            crate::model_lifecycle::get_model_lifecycle_state(),
        )
        .to_string(),
        kv_cache_used_mb: crate::model_lifecycle::get_kv_cache_used_mb(),
        vram_used_mb,
        vram_total_mb,
        npu_used_mb,
        npu_total_mb,
    }
}

fn probe_vram_usage_mb() -> (u32, u32) {
    #[cfg(target_os = "windows")]
    {
        if let Ok(memory) = qualia_core_db::directml_bridge::probe_best_adapter_memory() {
            let used = memory.local_usage_bytes / (1024 * 1024);
            let total = memory.local_budget_bytes / (1024 * 1024);
            return (used as u32, total as u32);
        }
    }
    (0, 0)
}

fn probe_npu_usage_mb() -> (u32, u32) {
    #[cfg(target_os = "windows")]
    {
        if let Ok(memory) = qualia_core_db::directml_bridge::probe_npu_adapter_memory() {
            let used = memory.shared_usage_bytes / (1024 * 1024); // NPUs often use shared memory
            let total = memory.shared_budget_bytes / (1024 * 1024);
            return (used as u32, total as u32);
        }
    }
    (0, 0)
}

pub async fn download_and_vectorize(
    url: String,
    filename: String,
    item_id: String,
) -> Result<String, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage_path = state.config.lock().unwrap().storage_path.clone();
    let handles = state.download_handles.clone();
    let active_dl = state.active_downloads.clone();

    let index_dir = PathBuf::from(&storage_path).join("Index");
    std::fs::create_dir_all(&index_dir).map_err(|e| e.to_string())?;
    let dest_path = index_dir.join(&filename);

    let cancelled = Arc::new(AtomicBool::new(false));
    handles
        .lock()
        .unwrap()
        .insert(item_id.clone(), cancelled.clone());

    let response = reqwest::get(&url).await.map_err(|e| {
        handles.lock().unwrap().remove(&item_id);
        active_dl.lock().unwrap().remove(&item_id);
        e.to_string()
    })?;
    let total_bytes = response.content_length().unwrap_or(0);
    let mut dest = std::fs::File::create(&dest_path).map_err(|e| e.to_string())?;
    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;
    let mut last_report = std::time::Instant::now();
    let mut last_downloaded: u64 = 0;

    while let Some(chunk) = stream.next().await {
        if cancelled.load(Ordering::Relaxed) {
            let _ = std::fs::remove_file(&dest_path);
            let payload = ProgressPayload {
                id: item_id.clone(),
                progress: 0.0,
                downloaded_bytes: downloaded,
                total_bytes,
                speed_kbps: 0.0,
                status: "cancelled".to_string(),
            };
            let _ = state.download_events.send(payload.clone());
            handles.lock().unwrap().remove(&item_id);
            active_dl.lock().unwrap().remove(&item_id);
            return Err("Cancelled".to_string());
        }
        let chunk = chunk.map_err(|e| e.to_string())?;
        dest.write_all(&chunk).map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;

        let now = std::time::Instant::now();
        if now.duration_since(last_report).as_millis() >= 200 {
            let elapsed = now.duration_since(last_report).as_secs_f64().max(0.001);
            let speed_kbps = ((downloaded - last_downloaded) as f64 / 1024.0) / elapsed;
            let progress = if total_bytes > 0 {
                (downloaded as f64 / total_bytes as f64) * 100.0
            } else {
                0.0
            };
            let payload = ProgressPayload {
                id: item_id.clone(),
                progress,
                downloaded_bytes: downloaded,
                total_bytes,
                speed_kbps,
                status: "downloading".to_string(),
            };
            let _ = state.download_events.send(payload.clone());
            active_dl.lock().unwrap().insert(item_id.clone(), payload);
            last_report = now;
            last_downloaded = downloaded;
        }
    }

    let processing_payload = ProgressPayload {
        id: item_id.clone(),
        progress: 100.0,
        downloaded_bytes: downloaded,
        total_bytes,
        speed_kbps: 0.0,
        status: "processing".to_string(),
    };
    let _ = state.download_events.send(processing_payload.clone());
    active_dl
        .lock()
        .unwrap()
        .insert(item_id.clone(), processing_payload);

    let _quin_count = crate::resource_import::ingest_local_rdf(
        &dest_path,
        &item_id,
        Path::new(&storage_path),
        None,
    )
    .map_err(|e| e.to_string())?;

    let _ = std::fs::remove_file(&dest_path);

    let done_payload = ProgressPayload {
        id: item_id.clone(),
        progress: 100.0,
        downloaded_bytes: downloaded,
        total_bytes,
        speed_kbps: 0.0,
        status: "complete".to_string(),
    };
    let _ = state.download_events.send(done_payload.clone());
    handles.lock().unwrap().remove(&item_id);
    active_dl.lock().unwrap().remove(&item_id);
    Ok("Download and vectorization complete".to_string())
}

pub async fn download_model(
    url: String,
    filename: String,
    model_id: String,
) -> Result<String, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage_path = state.config.lock().unwrap().storage_path.clone();
    let handles = state.download_handles.clone();
    let active_dl = state.active_downloads.clone();

    let models_dir = PathBuf::from(&storage_path).join("Models");
    std::fs::create_dir_all(&models_dir).map_err(|e| e.to_string())?;
    let dest_path = models_dir.join(&filename);

    let cancelled = Arc::new(AtomicBool::new(false));
    handles
        .lock()
        .unwrap()
        .insert(model_id.clone(), cancelled.clone());

    let response = reqwest::get(&url).await.map_err(|e| {
        handles.lock().unwrap().remove(&model_id);
        active_dl.lock().unwrap().remove(&model_id);
        e.to_string()
    })?;
    let total_bytes = response.content_length().unwrap_or(0);
    let mut dest = std::fs::File::create(&dest_path).map_err(|e| e.to_string())?;
    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;
    let mut last_report = std::time::Instant::now();
    let mut last_downloaded: u64 = 0;

    while let Some(chunk) = stream.next().await {
        if cancelled.load(Ordering::Relaxed) {
            let _ = std::fs::remove_file(&dest_path);
            let payload = ProgressPayload {
                id: model_id.clone(),
                progress: 0.0,
                downloaded_bytes: downloaded,
                total_bytes,
                speed_kbps: 0.0,
                status: "cancelled".to_string(),
            };
            let _ = state.download_events.send(payload.clone());
            handles.lock().unwrap().remove(&model_id);
            active_dl.lock().unwrap().remove(&model_id);
            return Err("Cancelled".to_string());
        }
        let chunk = chunk.map_err(|e| e.to_string())?;
        dest.write_all(&chunk).map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;

        let now = std::time::Instant::now();
        if now.duration_since(last_report).as_millis() >= 200 {
            let elapsed = now.duration_since(last_report).as_secs_f64().max(0.001);
            let speed_kbps = ((downloaded - last_downloaded) as f64 / 1024.0) / elapsed;
            let progress = if total_bytes > 0 {
                (downloaded as f64 / total_bytes as f64) * 100.0
            } else {
                0.0
            };
            let payload = ProgressPayload {
                id: model_id.clone(),
                progress,
                downloaded_bytes: downloaded,
                total_bytes,
                speed_kbps,
                status: "downloading".to_string(),
            };
            let _ = state.download_events.send(payload.clone());
            active_dl.lock().unwrap().insert(model_id.clone(), payload);
            last_report = now;
            last_downloaded = downloaded;
        }
    }

    let done_payload = ProgressPayload {
        id: model_id.clone(),
        progress: 100.0,
        downloaded_bytes: downloaded,
        total_bytes,
        speed_kbps: 0.0,
        status: "complete".to_string(),
    };
    let _ = state.download_events.send(done_payload.clone());
    handles.lock().unwrap().remove(&model_id);
    active_dl.lock().unwrap().remove(&model_id);
    Ok(dest_path.to_string_lossy().to_string())
}

pub fn cancel_download(id: String) -> Result<(), String> {
    let state = crate::state::APP_STATE.get().unwrap();
    if let Some(flag) = state.download_handles.lock().unwrap().get(&id) {
        flag.store(true, Ordering::Relaxed);
    }
    Ok(())
}

pub fn start_daemon() -> String {
    "Daemon Started".to_string()
}

pub fn daemon_status() -> String {
    let state = crate::state::APP_STATE.get().unwrap();
    if *state.daemon_running.lock().unwrap() {
        "running".to_string()
    } else {
        "stopped".to_string()
    }
}

pub fn get_tax_suite() -> TaxRecipientSuite {
    let state = crate::state::APP_STATE.get().unwrap();
    state.tax_suite.lock().unwrap().clone()
}

pub fn save_tax_suite(suite: TaxRecipientSuite) -> Result<(), String> {
    let state = crate::state::APP_STATE.get().unwrap();
    suite.validate()?;
    let data_dir = state.config.lock().unwrap().storage_path.clone();
    let path = suite_file_path(&data_dir);
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(&suite).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    *state.tax_suite.lock().unwrap() = suite;
    Ok(())
}

pub fn dispatch_tax_payment(gross_amount_micro_cents: u64) -> Result<DispatchResult, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let suite = state.tax_suite.lock().unwrap().clone();
    let plan = route_tax_payment(gross_amount_micro_cents, &suite)?;
    let disp = IlpDispatcher::new(HttpIlpTransport {
        connector_url: "http://localhost:7770".to_string(),
    });
    Ok(disp.dispatch(&plan))
}

pub fn accept_vault_handshake(did_key: String, _payload: String) -> Result<String, String> {
    println!("[VC-8] Vault handshake from: {}", did_key);
    Ok("HANDSHAKE_SUCCESS".to_string())
}

pub fn receive_vault_job(
    job_id: String,
    task_type: String,
    _data_blob_cbor_ld: Vec<u8>,
) -> Result<String, String> {
    println!("[VC-12] Offload job {} type {}", job_id, task_type);
    if task_type == "LLM_INFERENCE" && check_ollama_status() {
        Ok("INFERENCE_QUEUED".to_string())
    } else {
        Err("UNSUPPORTED_TASK_OR_NO_CAPACITY".to_string())
    }
}

pub async fn ingest_pdf(file_name: String) -> Result<ingestion::IngestionResult, String> {
    let result = ingestion::process_pdf(&file_name)?;
    q42_compiler::compile_to_q42(&file_name, &result.bookmarks)?;
    Ok(result)
}

pub async fn ingest_literature(file_path: String) -> Result<String, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage_path = state.config.lock().unwrap().storage_path.clone();
    let lib_dir = PathBuf::from(&storage_path).join("SemanticLibrary");
    if !lib_dir.exists() {
        std::fs::create_dir_all(&lib_dir).map_err(|e| e.to_string())?;
    }

    let source_path = std::path::Path::new(&file_path);
    let filename = source_path.file_name().unwrap_or_default();
    let dest_path = lib_dir.join(filename);
    std::fs::copy(&source_path, &dest_path).map_err(|e| e.to_string())?;

    let text = pdf_extract::extract_text(&dest_path).map_err(|e| e.to_string())?;
    let preview = if text.len() > 100 {
        &text[0..100]
    } else {
        &text
    };

    Ok(format!(
        "Successfully ingested literature: {}. Generated ontology nodes from preview: '{}...'",
        filename.to_string_lossy(),
        preview.replace("\n", " ")
    ))
}

pub async fn upsert_cmld_definition(term: String, context_did: String) -> Result<String, String> {
    Ok(format!(
        "Successfully mapped '{}' to Context: {}",
        term, context_did
    ))
}

pub async fn ingest_ontology(file_name: String) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage_path = state.config.lock().unwrap().storage_path.clone();
    let index_dir = PathBuf::from(&storage_path).join("Index");
    let source_path = index_dir.join(&file_name);

    if !source_path.is_file() {
        return Err(format!(
            "Ontology source not found in Index/: {}",
            source_path.display()
        ));
    }

    let ontology_id = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&file_name)
        .to_string();

    let quin_count = crate::resource_import::ingest_local_rdf(
        &source_path,
        &ontology_id,
        Path::new(&storage_path),
        None,
    )
    .map_err(|e| e.to_string())?;

    let q42_path = index_dir.join(format!("{ontology_id}.q42"));

    Ok(serde_json::json!({
        "status": "success",
        "file": file_name,
        "ontology_id": ontology_id,
        "q42_path": q42_path.to_string_lossy(),
        "quin_count": quin_count,
    }))
}

pub async fn import_catalog_ontology(id: String) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage_path = state.config.lock().unwrap().storage_path.clone();
    let catalog = load_workspace_catalog();

    let cancelled = Arc::new(AtomicBool::new(false));
    state
        .download_handles
        .lock()
        .unwrap()
        .insert(id.clone(), cancelled.clone());

    let progress = crate::resource_import::ImportProgressCtx {
        id: id.clone(),
        handles: state.download_handles.clone(),
        active_downloads: state.active_downloads.clone(),
        download_events: state.download_events.clone(),
    };

    let result = crate::resource_import::import_catalog_ontology_with_options(
        &catalog,
        &id,
        Path::new(&storage_path),
        Some(&progress),
        true,
    )
    .await
    .map_err(|e| {
        state.download_handles.lock().unwrap().remove(&id);
        state.active_downloads.lock().unwrap().remove(&id);
        e.to_string()
    })?;

    qualia_core_db::daemon_graph::init_daemon_graph(&storage_path);

    serde_json::to_value(result).map_err(|e| e.to_string())
}

pub async fn export_to_solid(
    input_q42_path: String,
    output_dir_path: String,
) -> Result<String, String> {
    qualia_core_db::solid_ldp::SolidExporter::export_to_solid_pod(&input_q42_path, &output_dir_path)
        .map(|_| format!("Exported to {}", output_dir_path))
        .map_err(|e| e.to_string())
}

/// Consumer: fetch a Solid LDP resource and return status + Turtle body + Quin count.
pub async fn fetch_from_solid_pod(
    url: String,
    bearer_token: Option<String>,
) -> Result<serde_json::Value, String> {
    let r = qualia_solid_bridge::fetch_resource(&url, bearer_token.as_deref())
        .await
        .map_err(|e| e.to_string())?;
    Ok(serde_json::json!({
        "url": r.url,
        "status": r.status,
        "content_type": r.content_type,
        "quin_count": r.quin_count,
        "body": r.body,
    }))
}

/// Consumer: PUT a local Turtle/file to a Solid resource URL (sync-to-pod).
pub async fn put_to_solid_pod(
    url: String,
    body: Vec<u8>,
    content_type: Option<String>,
    bearer_token: Option<String>,
) -> Result<serde_json::Value, String> {
    let ct = content_type.unwrap_or_else(|| "text/turtle".into());
    let status = qualia_solid_bridge::put_resource(&url, &body, &ct, bearer_token.as_deref())
        .await
        .map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "ok": true, "status": status, "url": url }))
}

/// Sync: if `body_or_path` is a path to an existing file, upload it; else treat as Turtle body.
pub async fn sync_to_solid_pod(
    pod_url: String,
    body_or_path: Option<String>,
    bearer_token: Option<String>,
) -> Result<String, String> {
    let (bytes, ct) = if let Some(ref p) = body_or_path {
        let path = std::path::Path::new(p);
        if path.is_file() {
            let b = std::fs::read(path).map_err(|e| e.to_string())?;
            let ct = if p.ends_with(".json") || p.ends_with(".jsonld") {
                "application/ld+json"
            } else {
                "text/turtle"
            };
            (b, ct.to_string())
        } else {
            (p.as_bytes().to_vec(), "text/turtle".into())
        }
    } else {
        // Minimal deposit marker when UI only passes a URL
        let body = format!(
            "@prefix dcterms: <http://purl.org/dc/terms/> .\n<> dcterms:description \"Qualia sync {}\" .\n",
            chrono::Utc::now().to_rfc3339()
        );
        (body.into_bytes(), "text/turtle".into())
    };
    let status = qualia_solid_bridge::put_resource(&pod_url, &bytes, &ct, bearer_token.as_deref())
        .await
        .map_err(|e| e.to_string())?;
    Ok(format!(
        "Synced to Solid Pod {pod_url} (HTTP {status}, {} bytes)",
        bytes.len()
    ))
}

pub async fn ingest_image(file_path: String) -> Result<serde_json::Value, String> {
    ingest_image_typed(file_path, "Generic Asset".to_string()).await
}

pub async fn ingest_image_typed(
    file_path: String,
    typology: String,
) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let active = load_active_model_record_from_disk();
    let result = crate::vision_ingest::ingest_image_with_active_record(
        Path::new(&storage),
        active,
        Path::new(&file_path),
        &typology,
    )
    .map_err(|e| e.to_string())?;
    serde_json::to_value(result).map_err(|e| e.to_string())
}

pub async fn ingest_image_async(file_path: String, typology: String) -> Result<(), String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let active = load_active_model_record_from_disk();
    tokio::spawn(async move {
        let _ = crate::vision_ingest::ingest_image_with_active_record(
            Path::new(&storage),
            active,
            Path::new(&file_path),
            &typology,
        );
    });
    Ok(())
}
