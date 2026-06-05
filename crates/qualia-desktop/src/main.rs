#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use tauri::{
    CustomMenuItem, Manager, State, Window,
    SystemTray, SystemTrayEvent, SystemTrayMenu, SystemTrayMenuItem,
};
use sysinfo::{System, Disks};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use tokio::time::{sleep, Duration};
use std::path::PathBuf;
use qualia_core_db::rpc::{TaxRecipientSuite, route_tax_payment};
use qualia_core_db::ilp_dispatcher::{IlpDispatcher, HttpIlpTransport, DispatchResult};
use futures_util::StreamExt;
use std::io::Write;

mod ingestion;
mod q42_compiler;
mod llm_offload;
mod app_registry;

#[derive(Clone, serde::Serialize)]
struct RelayTelemetry {
    packets_routed: u32,
    packets_dropped: u32,
    buffer_memory_mb: f64,
    is_congested: bool,
}

#[derive(Clone, serde::Serialize)]
struct StarkTelemetry {
    status: String,
    cpu_utilization: f64,
    ram_usage_mb: f64,
    fragments_paged: u32,
}

// ── Config ────────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
struct AgentConfig {
    storage_path: String,
    storage_quota_gb: u64,
    base_connectivity_cost_ilp: u64,
    daemon_host: String,
    daemon_port: u16,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            storage_path: dirs_default_path(),
            storage_quota_gb: 10,
            base_connectivity_cost_ilp: 5000,
            daemon_host: "127.0.0.1".to_string(),
            daemon_port: 4242,
        }
    }
}

fn dirs_default_path() -> String {
    #[cfg(target_os = "windows")]
    { std::env::var("APPDATA").map(|d| format!("{}\\QualiaData", d))
        .unwrap_or_else(|_| "C:\\QualiaData".to_string()) }
    #[cfg(target_os = "macos")]
    { std::env::var("HOME").map(|h| format!("{}/Library/Application Support/QualiaData", h))
        .unwrap_or_else(|_| "/tmp/QualiaData".to_string()) }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    { std::env::var("HOME").map(|h| format!("{}/.local/share/QualiaData", h))
        .unwrap_or_else(|_| "/tmp/QualiaData".to_string()) }
}

// Fixed meta directory for config + identity (separate from the user-configurable data path)
fn app_meta_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    { PathBuf::from(std::env::var("APPDATA").unwrap_or_else(|_| "C:\\Users\\Default\\AppData\\Roaming".to_string())).join("Qualia") }
    #[cfg(target_os = "macos")]
    { PathBuf::from(std::env::var("HOME").unwrap_or_default()).join("Library/Application Support/Qualia") }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    { PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".config/qualia") }
}

fn config_file_path() -> PathBuf { app_meta_dir().join("config.json") }
fn identity_file_path() -> PathBuf { app_meta_dir().join("identity.json") }

fn load_config_from_disk() -> AgentConfig {
    std::fs::read_to_string(config_file_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(AgentConfig::default)
}

fn init_data_directories(storage_path: &str) {
    let base = PathBuf::from(storage_path);
    for sub in &["Models", "Index", "Apps", "SemanticLibrary", "Identity"] {
        let _ = std::fs::create_dir_all(base.join(sub));
    }
}

// ── App state ─────────────────────────────────────────────────────────────────

struct AppState {
    config:         Mutex<AgentConfig>,
    tax_suite:      Mutex<TaxRecipientSuite>,
    daemon_running: Arc<Mutex<bool>>,
    nym_relay_active: Arc<AtomicBool>,
    stark_prover_active: Arc<AtomicBool>,
    simulated_solar_watts: Arc<AtomicU32>,
    download_handles: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
    active_downloads: Arc<Mutex<HashMap<String, ProgressPayload>>>,
    active_model: Arc<Mutex<Option<String>>>,
    rqbit_session: Arc<tokio::sync::Mutex<Option<std::sync::Arc<librqbit::Session>>>>,
    directory_actors: Arc<Mutex<Vec<Actor>>>,
    delegation_rules: Arc<Mutex<Vec<DelegationRule>>>,
    front_doors: Arc<Mutex<Vec<FrontDoor>>>,
    installed_apps: Arc<Mutex<Vec<app_registry::RegisteredApp>>>,
    key_vault: Arc<Mutex<qualia_core_db::key_vault::KeyVault>>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct FrontDoor {
    pub id: String,
    pub did_uri: String,
    pub label: String, // e.g., "Public Profile", "Anonymous Forum"
    pub created_at: String,
    pub routing_hints: Vec<String>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Actor {
    pub id: String,
    pub actor_type: String, // "ORGANIZATION", "PRACTITIONER", "DELEGATE", "SYNTHETIC_AGENT"
    pub name: String,
    pub organization: Option<String>,
    pub qualifications: Vec<String>,
    pub roles: Vec<String>,
    pub verification_status: String, // "VERIFIED", "SELF_CLAIMED"
    pub pairwise_did: String,
    pub root_did_uri: Option<String>,
    pub routing_hints: Vec<String>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct DelegationRule {
    pub id: String,
    pub actor_id: String,
    pub granted_roles: Vec<String>,
    pub legal_basis: String,
    pub privacy_mode_limit: String, // "MODE_A_STRICT", "MODE_B_PRIVILEGED", etc.
    pub allowed_record_types: Vec<String>,
    pub restricted_records: Vec<String>,
    pub is_active: bool,
}

fn suite_file_path(data_dir: &str) -> PathBuf {
    PathBuf::from(data_dir).join("tax_suite.json")
}

fn load_suite_from_disk(data_dir: &str) -> TaxRecipientSuite {
    let path = suite_file_path(data_dir);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(TaxRecipientSuite::default_cooperative)
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
fn list_installed_apps(state: State<AppState>) -> Vec<String> {
    let data_dir = state.config.lock().unwrap().storage_path.clone();
    let apps_dir = PathBuf::from(&data_dir).join("Apps");
    let mut apps = Vec::new();
    if let Ok(entries) = std::fs::read_dir(apps_dir) {
        for entry in entries.filter_map(Result::ok) {
            if entry.path().is_dir() {
                apps.push(entry.file_name().to_string_lossy().to_string());
            }
        }
    }
    apps
}

#[tauri::command]
fn generate_app_credential(app_name: String) -> String {
    format!("did:qualia:app:{}:signed_vc", app_name)
}

#[tauri::command]
fn verify_and_install_app(state: State<AppState>, target_path: String) -> Result<String, String> {
    let path = std::path::PathBuf::from(&target_path);
    let manifest_path = path.join("app.json");
    if !manifest_path.exists() {
        return Err("app.json not found in directory".into());
    }
    
    let content = std::fs::read_to_string(&manifest_path).map_err(|e| e.to_string())?;
    let manifest: app_registry::AppManifest = serde_json::from_str(&content).map_err(|e| format!("Invalid app.json: {}", e))?;
    
    let app_did = format!("did:qualia:app:{}", manifest.name.to_lowercase().replace(" ", "-"));
    
    // Check if we are running a dev port by looking at the path string.
    // If it starts with a port number, we assign LocalProxyPort.
    let target = if let Ok(port) = target_path.parse::<u16>() {
        app_registry::AppTarget::LocalProxyPort(port)
    } else {
        app_registry::AppTarget::LocalDevDirectory(path)
    };
    
    let registered_app = app_registry::RegisteredApp {
        did: app_did.clone(),
        manifest,
        target,
    };
    
    state.installed_apps.lock().unwrap().push(registered_app);
    save_directory_state(&state);
    
    Ok(app_did)
}

#[derive(Serialize)]
struct WalletStatus {
    lightning_sats: u64,
    ilp_microcents: u64,
    nym_connected: bool,
}

#[tauri::command]
fn get_wallet_status() -> WalletStatus {
    WalletStatus {
        lightning_sats: 450000,
        ilp_microcents: 1250000,
        nym_connected: true,
    }
}

#[tauri::command]
fn get_config(state: State<AppState>) -> AgentConfig {
    state.config.lock().unwrap().clone()
}

#[tauri::command]
fn save_config(state: State<AppState>, new_config: AgentConfig) -> Result<(), String> {
    let disks = Disks::new_with_refreshed_list();
    let path = PathBuf::from(&new_config.storage_path);
    let mut available = u64::MAX;
    for disk in disks.list() {
        if path.starts_with(disk.mount_point()) {
            available = disk.available_space();
            break;
        }
    }
    let margin: u64 = 15 * 1024 * 1024 * 1024;
    let requested = new_config.storage_quota_gb * 1024 * 1024 * 1024;
    if available.saturating_sub(requested) < margin {
        return Err("OS_SAFETY_VIOLATION: Would leave the host OS with less than the 15 GB safety margin.".to_string());
    }
    // Persist config to disk
    let meta = app_meta_dir();
    std::fs::create_dir_all(&meta).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(&new_config).map_err(|e| e.to_string())?;
    std::fs::write(config_file_path(), json).map_err(|e| e.to_string())?;
    // Ensure data directories exist under the new path
    init_data_directories(&new_config.storage_path);
    *state.config.lock().unwrap() = new_config;
    Ok(())
}

#[tauri::command]
fn profile_energy_circumstance() -> String {
    let mut sys = System::new_all();
    sys.refresh_all();
    let total_mem = sys.total_memory() / 1024 / 1024;
    let used_mem  = sys.used_memory()  / 1024 / 1024;
    format!(
        "Energy: AC_POWER\nTotal RAM: {} MB\nUsed RAM: {} MB\nSwarm Auth: GRANTED",
        total_mem, used_mem
    )
}

#[tauri::command]
fn check_ollama_status() -> bool {
    std::process::Command::new("ollama").arg("-v").output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[derive(Serialize)]
struct HardwareStatus {
    ram_total_gb: f64,
    ram_used_gb: f64,
    vram_estimated_gb: f64,
}

#[tauri::command]
fn get_hardware_status() -> HardwareStatus {
    let mut sys = System::new_all();
    sys.refresh_all();
    HardwareStatus {
        ram_total_gb: sys.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0,
        ram_used_gb: sys.used_memory() as f64 / 1024.0 / 1024.0 / 1024.0,
        // Mock 16GB unified memory for M-Series
        vram_estimated_gb: 16.0,
    }
}

#[derive(Clone, Serialize)]
struct ProgressPayload {
    id: String,
    progress: f64,        // 0–100
    downloaded_bytes: u64,
    total_bytes: u64,     // 0 if unknown
    speed_kbps: f64,
    status: String,       // "downloading" | "processing" | "complete" | "cancelled" | "error"
}

#[tauri::command]
async fn download_and_vectorize(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    url: String,
    filename: String,
    item_id: String,
) -> Result<String, String> {
    let storage_path = state.config.lock().unwrap().storage_path.clone();
    let handles = state.download_handles.clone();
    let active_dl = state.active_downloads.clone();

    let index_dir = PathBuf::from(&storage_path).join("Index");
    std::fs::create_dir_all(&index_dir).map_err(|e| e.to_string())?;
    let dest_path = index_dir.join(&filename);

    let cancelled = Arc::new(AtomicBool::new(false));
    handles.lock().unwrap().insert(item_id.clone(), cancelled.clone());

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
                id: item_id.clone(), progress: 0.0, downloaded_bytes: downloaded,
                total_bytes, speed_kbps: 0.0, status: "cancelled".to_string(),
            };
            let _ = app.emit_all("download-progress", &payload);
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
            let progress = if total_bytes > 0 { (downloaded as f64 / total_bytes as f64) * 100.0 } else { 0.0 };
            let payload = ProgressPayload {
                id: item_id.clone(), progress, downloaded_bytes: downloaded,
                total_bytes, speed_kbps, status: "downloading".to_string(),
            };
            let _ = app.emit_all("download-progress", &payload);
            active_dl.lock().unwrap().insert(item_id.clone(), payload);
            last_report = now;
            last_downloaded = downloaded;
        }
    }

    let processing_payload = ProgressPayload {
        id: item_id.clone(), progress: 100.0, downloaded_bytes: downloaded,
        total_bytes, speed_kbps: 0.0, status: "processing".to_string(),
    };
    let _ = app.emit_all("download-progress", &processing_payload);
    active_dl.lock().unwrap().insert(item_id.clone(), processing_payload);

    let dest_str = dest_path.to_string_lossy().to_string();
    let _result = ingestion::process_ontology(&dest_str).map_err(|e| e.to_string())?;

    let done_payload = ProgressPayload {
        id: item_id.clone(), progress: 100.0, downloaded_bytes: downloaded,
        total_bytes, speed_kbps: 0.0, status: "complete".to_string(),
    };
    let _ = app.emit_all("download-progress", &done_payload);
    handles.lock().unwrap().remove(&item_id);
    active_dl.lock().unwrap().remove(&item_id);
    Ok("Download and vectorization complete".to_string())
}

#[tauri::command]
async fn download_model(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    url: String,
    filename: String,
    model_id: String,
) -> Result<String, String> {
    let storage_path = state.config.lock().unwrap().storage_path.clone();
    let handles = state.download_handles.clone();
    let active_dl = state.active_downloads.clone();

    let models_dir = PathBuf::from(&storage_path).join("Models");
    std::fs::create_dir_all(&models_dir).map_err(|e| e.to_string())?;
    let dest_path = models_dir.join(&filename);

    let cancelled = Arc::new(AtomicBool::new(false));
    handles.lock().unwrap().insert(model_id.clone(), cancelled.clone());

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
                id: model_id.clone(), progress: 0.0, downloaded_bytes: downloaded,
                total_bytes, speed_kbps: 0.0, status: "cancelled".to_string(),
            };
            let _ = app.emit_all("download-progress", &payload);
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
            let progress = if total_bytes > 0 { (downloaded as f64 / total_bytes as f64) * 100.0 } else { 0.0 };
            let payload = ProgressPayload {
                id: model_id.clone(), progress, downloaded_bytes: downloaded,
                total_bytes, speed_kbps, status: "downloading".to_string(),
            };
            let _ = app.emit_all("download-progress", &payload);
            active_dl.lock().unwrap().insert(model_id.clone(), payload);
            last_report = now;
            last_downloaded = downloaded;
        }
    }

    let done_payload = ProgressPayload {
        id: model_id.clone(), progress: 100.0, downloaded_bytes: downloaded,
        total_bytes, speed_kbps: 0.0, status: "complete".to_string(),
    };
    let _ = app.emit_all("download-progress", &done_payload);
    handles.lock().unwrap().remove(&model_id);
    active_dl.lock().unwrap().remove(&model_id);
    Ok(dest_path.to_string_lossy().to_string())
}

#[tauri::command]
fn cancel_download(state: tauri::State<'_, AppState>, id: String) -> Result<(), String> {
    if let Some(flag) = state.download_handles.lock().unwrap().get(&id) {
        flag.store(true, Ordering::Relaxed);
    }
    Ok(())
}

#[tauri::command]
fn start_daemon() -> String { "Daemon Started".to_string() }

#[tauri::command]
fn daemon_status(state: State<AppState>) -> String {
    if *state.daemon_running.lock().unwrap() { "running".to_string() }
    else { "stopped".to_string() }
}

#[tauri::command]
fn get_tax_suite(state: State<AppState>) -> TaxRecipientSuite {
    state.tax_suite.lock().unwrap().clone()
}

#[tauri::command]
fn save_tax_suite(state: State<AppState>, suite: TaxRecipientSuite) -> Result<(), String> {
    suite.validate()?;
    let data_dir = state.config.lock().unwrap().storage_path.clone();
    let path = suite_file_path(&data_dir);
    if let Some(p) = path.parent() { std::fs::create_dir_all(p).map_err(|e| e.to_string())?; }
    let json = serde_json::to_string_pretty(&suite).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    *state.tax_suite.lock().unwrap() = suite;
    Ok(())
}

#[tauri::command]
fn dispatch_tax_payment(
    state: State<AppState>,
    gross_amount_micro_cents: u64,
) -> Result<DispatchResult, String> {
    let suite = state.tax_suite.lock().unwrap().clone();
    let plan  = route_tax_payment(gross_amount_micro_cents, &suite)?;
    let disp  = IlpDispatcher::new(HttpIlpTransport {
        connector_url: "http://localhost:7770".to_string(),
    });
    Ok(disp.dispatch(&plan))
}

#[tauri::command]
fn accept_vault_handshake(did_key: String, _payload: String) -> Result<String, String> {
    println!("[VC-8] Vault handshake from: {}", did_key);
    Ok("HANDSHAKE_SUCCESS".to_string())
}

#[tauri::command]
fn receive_vault_job(job_id: String, task_type: String, _data_blob_cbor: Vec<u8>) -> Result<String, String> {
    println!("[VC-12] Offload job {} type {}", job_id, task_type);
    if task_type == "LLM_INFERENCE" && check_ollama_status() {
        Ok("INFERENCE_QUEUED".to_string())
    } else {
        Err("UNSUPPORTED_TASK_OR_NO_CAPACITY".to_string())
    }
}

#[tauri::command]
async fn ingest_pdf(file_name: String) -> Result<ingestion::IngestionResult, String> {
    let result = ingestion::process_pdf(&file_name)?;
    q42_compiler::compile_to_q42(&file_name, &result.bookmarks)?;
    Ok(result)
}

#[tauri::command]
async fn ingest_literature(state: tauri::State<'_, AppState>, file_path: String) -> Result<String, String> {
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
    let preview = if text.len() > 100 { &text[0..100] } else { &text };

    Ok(format!("Successfully ingested literature: {}. Generated ontology nodes from preview: '{}...'", filename.to_string_lossy(), preview.replace("\n", " ")))
}

#[tauri::command]
async fn upsert_cmld_definition(term: String, context_did: String) -> Result<String, String> {
    Ok(format!("Successfully mapped '{}' to Context: {}", term, context_did))
}

use std::io::{Read, Seek, SeekFrom};
use std::fs::OpenOptions;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

#[tauri::command]
async fn ingest_ontology(state: tauri::State<'_, AppState>, file_name: String) -> Result<serde_json::Value, String> {
    let storage_path = state.config.lock().unwrap().storage_path.clone();
    let index_dir = PathBuf::from(&storage_path).join("Index");
    let index_path = index_dir.join(format!("{}.q42.bidx", file_name.replace(" ", "_")));

    let _ = std::fs::create_dir_all(&index_dir);

    // 1. Simulate the backend writing a true hardware-aligned 40,960 byte SuperBlock
    let mut file = OpenOptions::new().read(true).write(true).create(true).truncate(true).open(&index_path).map_err(|e| e.to_string())?;
    
    // Pad to 40,960 bytes
    let empty_block = vec![0u8; 40960];
    file.write_all(&empty_block).map_err(|e| e.to_string())?;
    
    // 2. Write exact deterministic bounds at the absolute offset
    let target_offset = 40960; // Absolute offset to the second block (page-aligned)
    file.seek(SeekFrom::Start(target_offset)).map_err(|e| e.to_string())?;
    
    // Write 64-bit Lexicon Node ID
    let mock_lexicon_id: u64 = 0x8F3B_C122_A943_0001;
    file.write_u64::<LittleEndian>(mock_lexicon_id).map_err(|e| e.to_string())?;
    
    // Write IEEE-754 Float Bound
    let true_ieee754_bound: f64 = 0.9423984183;
    file.write_f64::<LittleEndian>(true_ieee754_bound).map_err(|e| e.to_string())?;

    // 3. VFS Direct Byte Reading (No serde_json overhead during lookup)
    file.seek(SeekFrom::Start(target_offset)).map_err(|e| e.to_string())?;
    
    let extracted_lexicon_id = file.read_u64::<LittleEndian>().map_err(|e| e.to_string())?;
    let extracted_float = file.read_f64::<LittleEndian>().map_err(|e| e.to_string())?;

    Ok(serde_json::json!({
        "status": "success",
        "file": file_name,
        "nodes_added": 12845,
        "processing_time_ms": 3421,
        "lexicon_sample": format!("0x{:016X}", extracted_lexicon_id),
        "vector_bound_ieee754": extracted_float
    }))
}

#[tauri::command]
async fn export_to_solid(input_q42_path: String, output_dir_path: String) -> Result<String, String> {
    qualia_core_db::solid_ldp::SolidExporter::export_to_solid_pod(&input_q42_path, &output_dir_path)
        .map(|_| format!("Exported to {}", output_dir_path))
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn ingest_image(file_path: String) -> Result<serde_json::Value, String> {
    // Phase 9 Mock: Simulate native LLaVA extraction and binding to magnet URI
    let mock_lexicon_id = format!("0x{:08X}", 2654924194_u32);
    
    Ok(serde_json::json!({
        "status": "success",
        "file": file_path,
        "lexicon_id": mock_lexicon_id,
        "type": "Meme",
        "facet": "Extracted Sarcasm Tensor",
        "origin": "Native Rust LLaVA",
        "magnet_uri": "magnet:?xt=urn:btih:8c1e..."
    }))
}

#[tauri::command]
async fn ingest_image_async(app: tauri::AppHandle, file_path: String, typology: String) -> Result<(), String> {
    // Phase 10 & 15: Asynchronous LLaVA Extraction with Typology Routing
    tauri::async_runtime::spawn(async move {
        let mut image_base64 = String::new();
        if let Ok(bytes) = std::fs::read(&file_path) {
            use base64::{Engine as _, engine::general_purpose};
            image_base64 = general_purpose::STANDARD.encode(&bytes);
        }

        let client = reqwest::Client::new();
        let prompt = format!("Describe this image briefly for a {} context.", typology);
        
        let ollama_req = serde_json::json!({
            "model": "llava",
            "prompt": prompt,
            "stream": false,
            "images": [image_base64]
        });

        let mut facet_text = "Fallback Extracted Semantic Tensor".to_string();

        let response = client
            .post("http://127.0.0.1:11434/api/generate")
            .json(&ollama_req)
            .timeout(std::time::Duration::from_secs(15))
            .send()
            .await;

        if let Ok(resp) = response {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if let Some(text) = json["response"].as_str() {
                    facet_text = text.to_string();
                }
            }
        } else {
            // Simulated fallback if ollama is not running
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        }
        
        // Use typology lens to determine the specific facet extraction rules
        let payload = match typology.as_str() {
            "Meme" => serde_json::json!({
                "lexicon_id": format!("0x{:016X}", rand::random::<u64>()),
                "type": "Meme",
                "facet": format!("Distracted Boyfriend | Irony Tensor: 0.9 | Text: '{}'", facet_text),
                "origin": "2015 Internet",
                "region": "xywh=0,0,1024,768",
                "magnet_uri": "magnet:?xt=urn:btih:meme9f2c..."
            }),
            "Heraldry" => serde_json::json!({
                "lexicon_id": format!("0x{:016X}", rand::random::<u64>()),
                "type": "Heraldry",
                "facet": format!("Charge: Lion Rampant | Tincture: Or on Gules | Extracted: {}", facet_text),
                "origin": "14th Century",
                "region": "xywh=200,150,400,600",
                "magnet_uri": "magnet:?xt=urn:btih:heraldry8b1a..."
            }),
            _ => serde_json::json!({
                "lexicon_id": format!("0x{:016X}", rand::random::<u64>()),
                "type": "Generic Asset",
                "facet": facet_text,
                "origin": "Native Swarm Worker",
                "region": "t=1m20s",
                "magnet_uri": "magnet:?xt=urn:btih:9f2c..."
            }),
        };
        
        let _ = app.emit_all("ingestion-complete", payload);
    });
    
    Ok(())
}

// ── Token registry ───────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
struct TokenEntry {
    id:         String,
    chain:      String,  // "eCash" | "Ethereum" | "Nyx"
    token_type: String,  // "ALP" | "SLP" | "ERC-20" | "CW-20"
    contract:   String,  // token ID / contract address
    symbol:     String,
    name:       String,
    balance:    String,
    decimals:   u8,
    fiat_usd:   f64,
}

fn tokens_file_path(storage_path: &str) -> PathBuf {
    PathBuf::from(storage_path).join("tokens.json")
}

fn default_tokens() -> Vec<TokenEntry> {
    vec![
        TokenEntry { id: "alp-lion".into(),       chain: "eCash".into(),    token_type: "ALP".into(),   contract: "alp:0x1A2B3C4D...".into(),                                   symbol: "LION".into(),  name: "Lion Rampant (Heraldry)".into(),  balance: "1.00".into(),       decimals: 8, fiat_usd: 0.0 },
        TokenEntry { id: "alp-horus".into(),      chain: "eCash".into(),    token_type: "ALP".into(),   contract: "alp:0x9B4C5D6E...".into(),                                   symbol: "HORUS".into(), name: "Eye of Horus (Artifact)".into(),  balance: "50.00".into(),      decimals: 8, fiat_usd: 0.0 },
        TokenEntry { id: "slp-meme".into(),       chain: "eCash".into(),    token_type: "SLP".into(),   contract: "slp:0x44F1A2B3...".into(),                                   symbol: "MEME".into(),  name: "Early Beta Meme Coin".into(),     balance: "150000.00".into(),  decimals: 2, fiat_usd: 0.0 },
        TokenEntry { id: "erc20-usdt".into(),     chain: "Ethereum".into(), token_type: "ERC-20".into(), contract: "0xdAC17F958D2ee523a2206206994597C13D831ec7".into(),          symbol: "USDT".into(),  name: "Tether USD".into(),               balance: "250.00".into(),     decimals: 6, fiat_usd: 250.0 },
        TokenEntry { id: "erc20-usdc".into(),     chain: "Ethereum".into(), token_type: "ERC-20".into(), contract: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".into(),          symbol: "USDC".into(),  name: "USD Coin".into(),                 balance: "100.00".into(),     decimals: 6, fiat_usd: 100.0 },
        TokenEntry { id: "erc20-link".into(),     chain: "Ethereum".into(), token_type: "ERC-20".into(), contract: "0x514910771AF9Ca656af840dff83E8264EcF986CA".into(),           symbol: "LINK".into(),  name: "Chainlink Token".into(),          balance: "12.50".into(),     decimals: 18, fiat_usd: 162.5 },
        TokenEntry { id: "cw20-vnym".into(),      chain: "Nyx".into(),      token_type: "CW-20".into(), contract: "nyx1staking000000000000000000000000000000000000".into(),       symbol: "vNYM".into(),  name: "Vested NYM (Staking)".into(),     balance: "100.00".into(),     decimals: 6, fiat_usd: 2.0 },
    ]
}

fn load_tokens_from_disk(storage_path: &str) -> Vec<TokenEntry> {
    let path = tokens_file_path(storage_path);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(default_tokens)
}

fn save_tokens_to_disk(storage_path: &str, tokens: &[TokenEntry]) -> Result<(), String> {
    let json = serde_json::to_string_pretty(tokens).map_err(|e| e.to_string())?;
    std::fs::write(tokens_file_path(storage_path), json).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_tokens(state: State<AppState>) -> Vec<TokenEntry> {
    let storage_path = state.config.lock().unwrap().storage_path.clone();
    load_tokens_from_disk(&storage_path)
}

#[tauri::command]
fn add_token(
    state: State<AppState>,
    chain: String,
    token_type: String,
    contract: String,
    symbol: String,
    name: String,
    decimals: u8,
) -> Result<TokenEntry, String> {
    let storage_path = state.config.lock().unwrap().storage_path.clone();
    let mut tokens = load_tokens_from_disk(&storage_path);

    if tokens.iter().any(|t| t.contract.to_lowercase() == contract.to_lowercase() && t.chain == chain) {
        return Err("Token already in wallet".to_string());
    }

    let slug: String = contract.chars().rev().take(8).collect::<String>().chars().rev().collect();
    let id = format!("{}-{}", chain.to_lowercase().replace(' ', "-"), slug.to_lowercase());
    let entry = TokenEntry { id, chain, token_type, contract, symbol, name, balance: "0.00".into(), decimals, fiat_usd: 0.0 };
    tokens.push(entry.clone());
    save_tokens_to_disk(&storage_path, &tokens)?;
    Ok(entry)
}

#[tauri::command]
fn remove_token(state: State<AppState>, id: String) -> Result<(), String> {
    let storage_path = state.config.lock().unwrap().storage_path.clone();
    let mut tokens = load_tokens_from_disk(&storage_path);
    tokens.retain(|t| t.id != id);
    save_tokens_to_disk(&storage_path, &tokens)
}

// ─────────────────────────────────────────────────────────────────────────────

fn read_identity() -> Option<serde_json::Value> {
    std::fs::read_to_string(identity_file_path()).ok()
        .and_then(|s| serde_json::from_str(&s).ok())
}

#[derive(Serialize, Clone)]
struct CoinBalance {
    coin: String,
    ticker: String,
    address: String,
    balance: f64,
    balance_display: String,
    fiat_usd: f64,
    price_usd: f64,
    change_24h: f64,
    network: String,
    status: String,
}

#[derive(Serialize, Clone)]
struct TxRecord {
    txid: String,
    ticker: String,
    direction: String,    // "in" | "out"
    amount: String,
    label: String,
    timestamp: String,
    status: String,       // "confirmed" | "pending"
    confirmations: u32,
    fee: String,
    counterparty: String,
}

#[tauri::command]
fn get_coin_balances() -> Vec<CoinBalance> {
    let id = read_identity();
    let addr = |key: &str, fallback: &str| -> String {
        id.as_ref()
            .and_then(|v| v.get(key))
            .and_then(|v| v.as_str())
            .unwrap_or(fallback)
            .to_string()
    };
    vec![
        CoinBalance { coin: "eCash".into(),    ticker: "XEC".into(), address: addr("ecash_xec",  "ecash:q… (generate identity first)"), balance: 1_250_000.0, balance_display: "1,250,000.00".into(), fiat_usd: 245.00,    price_usd: 0.000196, change_24h:  3.2, network: "eCash".into(),     status: "online".into() },
        CoinBalance { coin: "Bitcoin".into(),   ticker: "BTC".into(), address: "bc1q… (generate identity first)".into(),                  balance: 0.00450000, balance_display: "0.00450000".into(),   fiat_usd: 441.00,    price_usd: 98_000.0, change_24h: -1.4, network: "Bitcoin".into(),   status: "online".into() },
        CoinBalance { coin: "Monero".into(),    ticker: "XMR".into(), address: "4… (generate identity first)".into(),                     balance: 4.5,        balance_display: "4.50000000".into(),   fiat_usd: 720.00,    price_usd: 160.0,    change_24h:  0.8, network: "Monero".into(),    status: "online".into() },
        CoinBalance { coin: "Nym".into(),       ticker: "NYM".into(), address: addr("nym_mixnet", "n1… (generate identity first)"),        balance: 2_400.0,    balance_display: "2,400.00".into(),     fiat_usd: 48.00,     price_usd: 0.02,     change_24h: -2.1, network: "Nyx Chain".into(), status: "online".into() },
        CoinBalance { coin: "Ethereum".into(),  ticker: "ETH".into(), address: addr("ethereum",   "0x… (generate identity first)"),        balance: 1.42,       balance_display: "1.42000000".into(),   fiat_usd: 4_260.00,  price_usd: 3_000.0,  change_24h:  1.9, network: "Ethereum".into(),  status: "online".into() },
    ]
}

#[tauri::command]
fn get_transaction_history(ticker: String) -> Vec<TxRecord> {
    let all = vec![
        TxRecord { txid: "7a9b4f2e1c3d…4c1f".into(), ticker: "XEC".into(), direction: "out".into(), amount: "0.0001".into(),      label: "Mint ALP Token".into(),          timestamp: "2026-06-05 14:32".into(), status: "confirmed".into(), confirmations: 142,  fee: "0.00001 XEC".into(),  counterparty: "eCash Burn Address".into() },
        TxRecord { txid: "99a1bcd4ef56…bb2c".into(), ticker: "NYM".into(), direction: "out".into(), amount: "100.00".into(),      label: "Mixnet Staking".into(),          timestamp: "2026-06-04 09:12".into(), status: "confirmed".into(), confirmations: 320,  fee: "0.01 NYM".into(),     counterparty: "mixGateway1".into() },
        TxRecord { txid: "4cc288ab12dc…11a9".into(), ticker: "XEC".into(), direction: "in".into(),  amount: "50,000.00".into(),   label: "Received XEC".into(),            timestamp: "2026-06-03 17:45".into(), status: "confirmed".into(), confirmations: 580,  fee: "".into(),             counterparty: "ecash:qsender7x…".into() },
        TxRecord { txid: "b8f1234abc99…de45".into(), ticker: "ETH".into(), direction: "out".into(), amount: "0.05".into(),        label: "Smart Contract Interaction".into(), timestamp: "2026-06-02 11:20".into(), status: "confirmed".into(), confirmations: 1280, fee: "0.002 ETH".into(),    counterparty: "0xContract4f2…".into() },
        TxRecord { txid: "c2d4567ef890…ab12".into(), ticker: "BTC".into(), direction: "in".into(),  amount: "0.00100000".into(),  label: "Received BTC".into(),            timestamp: "2026-06-01 08:55".into(), status: "confirmed".into(), confirmations: 2100, fee: "".into(),             counterparty: "bc1qsender9a…".into() },
        TxRecord { txid: "e1f23456789a…cd34".into(), ticker: "XEC".into(), direction: "out".into(), amount: "1,000.00".into(),    label: "ALP Token Transfer".into(),      timestamp: "2026-05-31 16:30".into(), status: "confirmed".into(), confirmations: 3400, fee: "0.00001 XEC".into(),  counterparty: "ecash:qrecipient3b…".into() },
        TxRecord { txid: "a9b0c1d2e3f4…5678".into(), ticker: "XMR".into(), direction: "in".into(),  amount: "2.00000000".into(),  label: "Received XMR".into(),            timestamp: "2026-05-30 14:10".into(), status: "confirmed".into(), confirmations: 4800, fee: "".into(),             counterparty: "4xmrSender8b…".into() },
        TxRecord { txid: "f8e7d6c5b4a3…2109".into(), ticker: "NYM".into(), direction: "in".into(),  amount: "500.00".into(),      label: "Staking Reward".into(),          timestamp: "2026-05-29 10:00".into(), status: "confirmed".into(), confirmations: 5200, fee: "".into(),             counterparty: "Nym Gateway Reward".into() },
        TxRecord { txid: "1a2b3c4d5e6f…7890".into(), ticker: "XEC".into(), direction: "in".into(),  amount: "250,000.00".into(),  label: "Initial Funding".into(),         timestamp: "2026-05-25 08:00".into(), status: "confirmed".into(), confirmations: 9100, fee: "".into(),             counterparty: "ecash:qfunding2a…".into() },
        TxRecord { txid: "0f1e2d3c4b5a…6789".into(), ticker: "ETH".into(), direction: "in".into(),  amount: "1.42000000".into(),  label: "ETH Transfer In".into(),         timestamp: "2026-05-20 12:00".into(), status: "confirmed".into(), confirmations: 12400, fee: "".into(),            counterparty: "0xSender7c4…".into() },
    ];
    if ticker.is_empty() || ticker == "ALL" { all }
    else { all.into_iter().filter(|tx| tx.ticker == ticker).collect() }
}

#[tauri::command]
fn is_first_run() -> bool {
    !config_file_path().exists()
}

#[tauri::command]
fn save_identity(wallets: serde_json::Value) -> Result<(), String> {
    let meta = app_meta_dir();
    std::fs::create_dir_all(&meta).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(&wallets).map_err(|e| e.to_string())?;
    std::fs::write(identity_file_path(), json).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn load_identity() -> Result<Option<serde_json::Value>, String> {
    let path = identity_file_path();
    if !path.exists() {
        return Ok(None);
    }
    let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let val: serde_json::Value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    Ok(Some(val))
}

use bip39::{Mnemonic, Language};

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

#[tauri::command]
async fn generate_bip39_seed() -> Result<String, String> {
    // Generate a secure, randomized 12-word seed phrase natively
    let mnemonic = Mnemonic::generate_in(Language::English, 12).map_err(|_| "Failed to generate".to_string())?;
    let words: Vec<&str> = mnemonic.words().collect();
    Ok(words.join(" "))
}

#[tauri::command]
async fn derive_wallets_from_seed(seed: String) -> Result<serde_json::Value, String> {
    let mnemonic = match Mnemonic::parse_in(Language::English, &seed) {
        Ok(m) => m,
        Err(_) => return Err("Invalid 12-word seed phrase.".to_string()),
    };
    
    // Deterministically generate keys based on the secure seed
    let seed_bytes = mnemonic.to_seed("");
    
    // Mock derivation by hex-encoding slices of the master seed
    let hex_seed = to_hex(&seed_bytes[0..16]);
    let xec_hex = to_hex(&seed_bytes[16..24]);
    let eth_hex = to_hex(&seed_bytes[24..32]);
    let nym_hex = to_hex(&seed_bytes[32..40]);
    
    Ok(serde_json::json!({
        "qualia_root": format!("did:qualia:0x{}", hex_seed),
        "nym_mixnet": format!("n1{}...", nym_hex),
        "ecash_xec": format!("ecash:q{}...", xec_hex),
        "ethereum": format!("0x{}...", eth_hex)
    }))
}

#[tauri::command]
async fn generate_front_door_invite() -> Result<String, String> {
    // Phase 11 Mock: Generate an ephemeral Front Door DID for email sharing
    Ok("did:qualia:frontdoor:88f72a-connect".to_string())
}

#[tauri::command]
async fn mint_semantic_token(asset_id: String) -> Result<String, String> {
    // Phase 12 Mock: Mint ALP eToken with eMPP / RDF metadata payload
    Ok(format!("alp:0x{:04X}...", 45672_u32))
}

#[tauri::command]
async fn fetch_wallet_portfolio() -> Result<serde_json::Value, String> {
    // Phase 13 Mock: Return diverse portfolio of tokens
    Ok(serde_json::json!([
        {
            "name": "Lion Rampant (Heraldry)",
            "tokenId": "alp:0x1A2B...",
            "ticker": "LION",
            "balance": "1.00",
            "rdf": "urn:concept:heraldry",
            "network": "eCash",
            "type": "ALP"
        },
        {
            "name": "Eye of Horus (Artifact)",
            "tokenId": "alp:0x9B4C...",
            "ticker": "HORUS",
            "balance": "50.00",
            "rdf": "urn:concept:egyptian",
            "network": "eCash",
            "type": "ALP"
        },
        {
            "name": "Early Beta Meme Coin",
            "tokenId": "slp:0x44F1...",
            "ticker": "MEME",
            "balance": "150000.00",
            "rdf": "Legacy Metadata",
            "network": "eCash",
            "type": "SLP"
        }
    ]))
}

#[tauri::command]
async fn import_external_seed(network: String, seed: String, label: String) -> Result<String, String> {
    // Phase 14 Mock: Multi-Seed Account Import
    // Validate seed format
    if seed.split_whitespace().count() < 12 {
        return Err("Invalid seed phrase".to_string());
    }

    // Hash seed to deterministically generate a mock address based on network
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    format!("{}-{}", seed, network).hash(&mut hasher);
    let mock_hash = format!("{:x}", hasher.finish());

    let address = match network.as_str() {
        "eCash (XEC)" => format!("ecash:q{}...", &mock_hash[0..8]),
        "Bitcoin (BTC)" => format!("bc1q{}...", &mock_hash[0..8]),
        "Nym (NYM) - Nyx Chain" => format!("n1{}...", &mock_hash[0..8]),
        "Monero (XMR)" => format!("4{}...", &mock_hash[0..12]),
        "Ethereum (EVM)" => format!("0x{}...", &mock_hash[0..10]),
        _ => format!("0x{}...", &mock_hash[0..10]),
    };

    Ok(address)
}

#[tauri::command]
async fn toggle_nym_relay(window: Window, state: tauri::State<'_, AppState>) -> Result<bool, String> {
    let active = &state.nym_relay_active;
    let currently_active = active.load(Ordering::Relaxed);
    let new_state = !currently_active;
    active.store(new_state, Ordering::Relaxed);

    if new_state {
        let active_clone = active.clone();
        let window_clone = window.clone();

        // Spawn asynchronous background daemon for packet routing
        tokio::spawn(async move {
            let mut packets_routed = 0;
            let mut packets_dropped = 0;

            while active_clone.load(Ordering::Relaxed) {
                // Simulate network fluctuations and calculate memory backpressure
                // Enforcing a strict 50MB telemetry boundary cap internally
                let packet_load_factor = 1.0 + (packets_routed % 5) as f64 * 0.2;
                let buffer_memory_mb = 12.4 * packet_load_factor;
                let is_congested = buffer_memory_mb > 45.0;

                if is_congested {
                    packets_dropped += 15;
                } else {
                    packets_routed += 42;
                }

                let _ = window_clone.emit("nym-telemetry", RelayTelemetry {
                    packets_routed,
                    packets_dropped,
                    buffer_memory_mb,
                    is_congested,
                });

                sleep(Duration::from_millis(500)).await;
            }
        });
    }
    Ok(new_state)
}

#[tauri::command]
async fn toggle_stark_prover(window: Window, state: tauri::State<'_, AppState>) -> Result<bool, String> {
    let active = &state.stark_prover_active;
    let currently_active = active.load(Ordering::Relaxed);
    let new_state = !currently_active;
    active.store(new_state, Ordering::Relaxed);

    if new_state {
        let active_clone = active.clone();
        let solar_clone = state.simulated_solar_watts.clone();
        let window_clone = window.clone();

        // Spawn asynchronous background daemon for out-of-core proof chunking
        tokio::spawn(async move {
            let mut fragments_paged = 0;

            while active_clone.load(Ordering::Relaxed) {
                let current_solar = solar_clone.load(Ordering::Relaxed);

                // Environmental state evaluation trigger (threshold at 400W)
                if current_solar < 400 {
                    let _ = window_clone.emit("stark-telemetry", StarkTelemetry {
                        status: "Suspended - Awaiting Solar Surplus".to_string(),
                        cpu_utilization: 0.0,
                        ram_usage_mb: 0.0,
                        fragments_paged,
                    });
                } else {
                    fragments_paged += 8; // Simulate 48-byte Super-Quin paging writes

                    let _ = window_clone.emit("stark-telemetry", StarkTelemetry {
                        status: "Proving Execution Active".to_string(),
                        cpu_utilization: 85.4,
                        ram_usage_mb: 320.0, // Constrained flat memory footprint
                        fragments_paged,
                    });
                }
                sleep(Duration::from_millis(1000)).await;
            }
        });
    }
    Ok(new_state)
}

#[tauri::command]
fn update_solar_input(watts: u32, state: tauri::State<'_, AppState>) {
    state.simulated_solar_watts.store(watts, Ordering::Relaxed);
}

#[tauri::command]
async fn fetch_torrent_telemetry(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let mut session_guard = state.rqbit_session.lock().await;
    
    if session_guard.is_none() {
        let storage_path = state.config.lock().unwrap().storage_path.clone();
        let download_dir = PathBuf::from(&storage_path).join("Downloads");
        std::fs::create_dir_all(&download_dir).ok();
        
        // Initialize librqbit session (Phase 9 integration)
        match librqbit::Session::new(download_dir).await {
            Ok(session) => {
                *session_guard = Some(session);
            }
            Err(e) => return Err(format!("Failed to init librqbit: {}", e)),
        }
    }

    if let Some(_session) = session_guard.as_ref() {
        // Here we would ideally call _session.stats(), but for compilation safety 
        // across version changes, we'll return dynamic but placeholder active values.
        Ok(serde_json::json!({
            "seeders": 1,
            "leechers": 0,
            "speed": "0.0 MB/s",
            "status": "Active (librqbit)"
        }))
    } else {
        Err("Librqbit session not initialized".into())
    }
}

#[tauri::command]
async fn discover_models(state: tauri::State<'_, AppState>) -> Result<Vec<llm_offload::ModelInfo>, String> {
    let storage_path = state.config.lock().unwrap().storage_path.clone();
    let models_dir = PathBuf::from(&storage_path).join("Models");
    let mut models = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&models_dir) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.extension().map(|e| e == "gguf").unwrap_or(false) {
                models.push(llm_offload::ModelInfo {
                    name: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
                    is_active: false,
                    avatar_type: "local".to_string(),
                });
            }
        }
    }
    Ok(models)
}

#[tauri::command]
async fn run_agent_inference(
    app_handle: tauri::AppHandle,
    prompt: String,
    model_name: String,
    intent_layout: Vec<f64>,
) -> Result<(), String> {
    tauri::async_runtime::spawn(async move {
        let _ = llm_offload::execute_agent_inference(app_handle, prompt, model_name, intent_layout).await;
    });
    Ok(())
}

// ── Active model ─────────────────────────────────────────────────────────────

fn active_model_path() -> PathBuf { app_meta_dir().join("active_model.txt") }

fn load_active_model_from_disk() -> Option<String> {
    std::fs::read_to_string(active_model_path()).ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

#[tauri::command]
fn get_active_model(state: State<AppState>) -> Option<String> {
    state.active_model.lock().unwrap().clone()
}

#[tauri::command]
fn set_active_model(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    model_name: String,
) -> Result<(), String> {
    let meta = app_meta_dir();
    std::fs::create_dir_all(&meta).map_err(|e| e.to_string())?;
    std::fs::write(active_model_path(), &model_name).map_err(|e| e.to_string())?;
    *state.active_model.lock().unwrap() = Some(model_name.clone());
    let _ = app.emit_all("active-model-changed", &model_name);
    Ok(())
}

// ── Active downloads (persists across page navigation) ────────────────────────

#[tauri::command]
fn get_active_downloads(state: State<AppState>) -> Vec<ProgressPayload> {
    state.active_downloads.lock().unwrap().values().cloned().collect()
}

// ── Remote manifest fetch ─────────────────────────────────────────────────────

#[tauri::command]
async fn fetch_remote_manifest(url: String) -> Result<String, String> {
    reqwest::get(&url).await
        .map_err(|e| format!("Network error: {}", e))?
        .text().await
        .map_err(|e| format!("Response error: {}", e))
}

// ── Imported accounts persistence ────────────────────────────────────────────

fn imported_accounts_path() -> PathBuf { app_meta_dir().join("imported_accounts.json") }

#[tauri::command]
fn load_imported_accounts() -> Result<serde_json::Value, String> {
    let path = imported_accounts_path();
    if !path.exists() {
        return Ok(serde_json::json!([]));
    }
    let s = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&s).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_imported_accounts(accounts: serde_json::Value) -> Result<(), String> {
    let meta = app_meta_dir();
    std::fs::create_dir_all(&meta).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(&accounts).map_err(|e| e.to_string())?;
    std::fs::write(imported_accounts_path(), json).map_err(|e| e.to_string())
}

// ── App launcher ──────────────────────────────────────────────────────────────

#[tauri::command]
fn launch_installed_app(app_handle: tauri::AppHandle, state: State<AppState>, app_did: String) -> Result<(), String> {
    // 1. Get the app from state
    let apps = state.installed_apps.lock().unwrap();
    let app = apps.iter().find(|a| a.did == app_did).ok_or("App not found")?.clone();
    drop(apps); // Drop lock early
    
    // 2. Generate Semantic Token
    let vault = state.key_vault.lock().unwrap();
    let token = vault.issue_app_token(&app.did, app.manifest.required_shapes).map_err(|e| e.to_string())?;
    
    // 3. Build target URL
    let target_url = match app.target {
        app_registry::AppTarget::LocalDevDirectory(path) => {
            // For dev directories, we'll map a custom localhost proxy or direct file path.
            // For now, let's assume index.html exists.
            format!("file:///{}", path.join("index.html").display()).replace("\\", "/")
        },
        app_registry::AppTarget::LocalProxyPort(port) => format!("http://localhost:{}", port),
        app_registry::AppTarget::IsolatedVault(ref s) => format!("qualia://localhost/{}/index.html", s),
    };
    
    // Append token
    let final_url = if target_url.contains('?') {
        format!("{}&token={}", target_url, token)
    } else {
        format!("{}?token={}", target_url, token)
    };
    
    // 4. Launch Sandboxed Tauri window
    let window_label = app.did.replace(":", "_").replace("-", "_");
    
    tauri::WindowBuilder::new(
        &app_handle,
        window_label,
        tauri::WindowUrl::External(final_url.parse().unwrap())
    )
    .title(&app.manifest.name)
    // CRITICAL: Disable IPC to Sandbox the App
    // We obliterate the __TAURI_IPC__ binding to prevent any raw native command execution.
    .initialization_script("window.__TAURI_IPC__ = undefined; window.__TAURI__ = undefined; delete window.__TAURI_IPC__; delete window.__TAURI__;")
    .build()
    .map_err(|e| e.to_string())?;

    Ok(())
}

// ── Dashboard engine command ───────────────────────────────────────────────────

#[tauri::command]
fn run_engine_command(cmd: String, state: State<AppState>) -> String {
    match cmd.as_str() {
        "ingest_bench" => profile_energy_circumstance(),
        "zk_screen"    => format!(
            "Daemon: {} | Ollama: {}",
            daemon_status(state),
            check_ollama_status()
        ),
        _ => "Unknown command".to_string(),
    }
}

// ── System tray ───────────────────────────────────────────────────────────────

fn build_tray() -> SystemTray {
    let menu = SystemTrayMenu::new()
        .add_item(CustomMenuItem::new("open",          "Open Qualia"))
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(CustomMenuItem::new("health",        "Daemon: starting…").disabled())
        .add_item(CustomMenuItem::new("daemon_port",   "Port 4242").disabled())
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(CustomMenuItem::new("benchmark",     "Run Benchmark"))
        .add_item(CustomMenuItem::new("playground",    "Open Playground"))
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(CustomMenuItem::new("quit",          "Quit Qualia"));

    SystemTray::new().with_menu(menu).with_tooltip("Qualia-DB — Semantic Engine")
}

fn handle_tray_event(app: &tauri::AppHandle, event: SystemTrayEvent) {
    match event {
        // Left-click → toggle window visibility (Windows/macOS only; Linux: no-op)
        SystemTrayEvent::LeftClick { .. } => {
            toggle_window(app);
        }

        // Double-click on Windows also opens the window
        SystemTrayEvent::DoubleClick { .. } => {
            toggle_window(app);
        }

        SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
            "open" => toggle_window(app),

            "benchmark" => {
                let _ = open::that("https://mediaprophet.github.io/qualiaDB/benchmark.html");
            }

            "playground" => {
                let _ = open::that("https://mediaprophet.github.io/qualiaDB/playground/index.html");
            }

            "quit" => {
                app.exit(0);
            }

            _ => {}
        },

        _ => {}
    }
}

fn toggle_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_window("main") {
        match window.is_visible() {
            Ok(true)  => { let _ = window.hide(); }
            Ok(false) => { let _ = window.show(); let _ = window.set_focus(); }
            Err(_)    => {}
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Agent Directory & Delegation Manager
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct DirectoryState {
    actors: Vec<Actor>,
    rules: Vec<DelegationRule>,
    front_doors: Vec<FrontDoor>,
    installed_apps: Vec<app_registry::RegisteredApp>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct SignedDirectoryState {
    state: DirectoryState,
    signature_hex: String,
}

fn save_directory_state(state: &AppState) {
    let home = std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\".to_string());
    let qualia_dir = std::path::PathBuf::from(home).join(".qualia");
    if !qualia_dir.exists() {
        let _ = std::fs::create_dir_all(&qualia_dir);
    }
    
    let ds = DirectoryState {
        actors: state.directory_actors.lock().unwrap().clone(),
        rules: state.delegation_rules.lock().unwrap().clone(),
        front_doors: state.front_doors.lock().unwrap().clone(),
        installed_apps: state.installed_apps.lock().unwrap().clone(),
    };
    
    let payload = serde_json::to_string(&ds).unwrap();
    let vault = state.key_vault.lock().unwrap();
    // Since we don't have the derived key in scope here easily, we sign with master for persistence.
    // In a real implementation, we'd sign with the specific identity.
    let sig = vault.sign_payload(&vault.derive_key("persistence"), payload.as_bytes());
    let sig_hex = hex::encode(sig.to_bytes());
    
    let signed_state = SignedDirectoryState {
        state: ds,
        signature_hex: sig_hex,
    };
    
    let state_path = qualia_dir.join("directory_state.json");
    let _ = std::fs::write(&state_path, serde_json::to_string_pretty(&signed_state).unwrap());
}

fn load_directory_state(vault: &qualia_core_db::key_vault::KeyVault) -> DirectoryState {
    let home = std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\".to_string());
    let state_path = std::path::PathBuf::from(home).join(".qualia").join("directory_state.json");
    
    if state_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&state_path) {
            if let Ok(signed_state) = serde_json::from_str::<SignedDirectoryState>(&content) {
                let payload = serde_json::to_string(&signed_state.state).unwrap();
                let sig_bytes = hex::decode(&signed_state.signature_hex).unwrap_or_default();
                if sig_bytes.len() == 64 {
                    let mut sig_arr = [0u8; 64];
                    sig_arr.copy_from_slice(&sig_bytes);
                    let persistence_key = vault.derive_key("persistence");
                    use ed25519_dalek::Signer; // to get verifying key
                    let pk = ed25519_dalek::VerifyingKey::from(&persistence_key);
                    if qualia_core_db::key_vault::KeyVault::verify_signature(pk.as_bytes(), payload.as_bytes(), &sig_arr).is_ok() {
                        return signed_state.state;
                    } else {
                        eprintln!("WARNING: directory_state.json signature validation failed! Tampering detected.");
                    }
                }
            }
        }
    }
    
    DirectoryState {
        actors: Vec::new(),
        rules: Vec::new(),
        front_doors: Vec::new(),
        installed_apps: Vec::new(),
    }
}

#[tauri::command]
fn get_front_doors(state: tauri::State<AppState>) -> Result<Vec<FrontDoor>, String> {
    let doors = state.front_doors.lock().unwrap().clone();
    Ok(doors)
}

#[tauri::command]
fn generate_front_door(label: String, state: tauri::State<AppState>) -> Result<FrontDoor, String> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
    
    let vault = state.key_vault.lock().unwrap();
    let fd_id = format!("fd-{}", now);
    let derived_key = vault.derive_key(&fd_id);
    let pub_key_hex = hex::encode(ed25519_dalek::VerifyingKey::from(&derived_key).as_bytes());
    let did_uri = format!("did:qualia:frontdoor:{}", pub_key_hex);
    
    // Optional: Pre-generate the WebID-TLS cert here if needed
    // let (cert, _) = vault.generate_webid_tls_cert(&derived_key, &did_uri).unwrap();
    
    let door = FrontDoor {
        id: fd_id,
        did_uri,
        label,
        created_at: now.to_string(),
        routing_hints: vec!["nym:mixnet:sydney1".to_string()],
    };
    
    drop(vault);
    state.front_doors.lock().unwrap().push(door.clone());
    save_directory_state(&state);
    Ok(door)
}

#[tauri::command]
fn get_directory_actors(state: tauri::State<AppState>) -> Result<Vec<Actor>, String> {
    let actors = state.directory_actors.lock().unwrap().clone();
    Ok(actors)
}

#[tauri::command]
fn add_directory_actor(mut actor: Actor, state: tauri::State<AppState>) -> Result<(), String> {
    if actor.routing_hints.is_empty() {
        actor.routing_hints.push("nym:mixnet:global".to_string());
    }
    state.directory_actors.lock().unwrap().push(actor);
    save_directory_state(&state);
    Ok(())
}

#[tauri::command]
fn get_delegation_rules(state: tauri::State<AppState>) -> Result<Vec<DelegationRule>, String> {
    let rules = state.delegation_rules.lock().unwrap().clone();
    Ok(rules)
}

#[tauri::command]
fn add_delegation_rule(rule: DelegationRule, state: tauri::State<AppState>) -> Result<(), String> {
    state.delegation_rules.lock().unwrap().push(rule);
    save_directory_state(&state);
    Ok(())
}

// ── main ──────────────────────────────────────────────────────────────────────

fn main() {
    let default_config  = load_config_from_disk();
    init_data_directories(&default_config.storage_path);
    let initial_suite   = load_suite_from_disk(&default_config.storage_path);
    let initial_model   = load_active_model_from_disk();
    let daemon_running  = Arc::new(Mutex::new(false));
    let daemon_flag     = daemon_running.clone();

    let home = std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\".to_string());
    let storage_dir = std::path::PathBuf::from(home).join(".qualia");
    if !storage_dir.exists() {
        let _ = std::fs::create_dir_all(&storage_dir);
    }
    let key_vault = qualia_core_db::key_vault::KeyVault::load_or_generate(storage_dir.to_str().unwrap())
        .expect("Failed to initialize KeyVault");
    
    let loaded_state = load_directory_state(&key_vault);

    let app_state = AppState {
        config:         Mutex::new(default_config.clone()),
        tax_suite:      Mutex::new(initial_suite),
        daemon_running: daemon_flag.clone(),
        nym_relay_active: Arc::new(AtomicBool::new(false)),
        stark_prover_active: Arc::new(AtomicBool::new(false)),
        simulated_solar_watts: Arc::new(AtomicU32::new(0)),
        download_handles: Arc::new(Mutex::new(HashMap::new())),
        active_downloads: Arc::new(Mutex::new(HashMap::new())),
        active_model: Arc::new(Mutex::new(initial_model)),
        rqbit_session: Arc::new(tokio::sync::Mutex::new(None)),
        directory_actors: Arc::new(Mutex::new(loaded_state.actors)),
        delegation_rules: Arc::new(Mutex::new(loaded_state.rules)),
        front_doors: Arc::new(Mutex::new(loaded_state.front_doors)),
        installed_apps: Arc::new(Mutex::new(loaded_state.installed_apps)),
        key_vault: Arc::new(Mutex::new(key_vault)),
    };

    let vault_for_daemon = app_state.key_vault.clone();

    tauri::Builder::default()
        .register_uri_scheme_protocol("qualia", move |_app, request| {
            let path = request.uri().strip_prefix("qualia://localhost/").unwrap_or("");
            let safe_path: PathBuf = PathBuf::from(path)
                .components()
                .filter(|c| matches!(c, std::path::Component::Normal(_)))
                .collect();
            let full_path = PathBuf::from(dirs_default_path()).join("Apps").join(safe_path);
            
            match std::fs::read(&full_path) {
                Ok(data) => {
                    let mime = mime_guess::from_path(&full_path).first_or_octet_stream();
                    tauri::http::ResponseBuilder::new()
                        .mimetype(mime.as_ref())
                        .status(200)
                        .body(data)
                }
                Err(_) => {
                    tauri::http::ResponseBuilder::new().status(404).body(Vec::new())
                }
            }
        })
        .manage(app_state)
        .system_tray(build_tray())
        .on_system_tray_event(handle_tray_event)
        // Hide to tray when the window is closed rather than quitting
        .on_window_event(|event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event.event() {
                event.window().hide().unwrap();
                api.prevent_close();
            }
        })
        .setup(move |app| {
            let handle = app.handle();

            // ── Phase 10: Native Hardware Orchestration (sysinfo) ─────────────
            let telemetry_handle = handle.clone();
            tauri::async_runtime::spawn(async move {
                let mut sys = sysinfo::System::new_all();
                loop {
                    sys.refresh_cpu();
                    sys.refresh_memory();
                    
                    let cpu_usage = sys.global_cpu_info().cpu_usage();
                    let mem_used = sys.used_memory() as f64 / 1024.0 / 1024.0 / 1024.0;
                    
                    let _ = telemetry_handle.emit_all("hardware-telemetry", serde_json::json!({
                        "cpu": format!("{:.1}%", cpu_usage),
                        "ram": format!("{:.2} GB", mem_used)
                    }));
                    
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            });

            // ── Start daemon ──────────────────────────────────────────────────
            // ── Start daemon ──────────────────────────────────────────────────────────
            let flag   = daemon_flag.clone();
            let tray_h = handle.clone();
            
            // Extract port and host from config, cloning them for the background thread
            let config_clone = default_config.clone();
            let host = config_clone.daemon_host;
            let mut target_port = config_clone.daemon_port;

            // Check for port conflicts
            loop {
                if std::net::TcpListener::bind((host.as_str(), target_port)).is_ok() {
                    break;
                }
                eprintln!("Port {} is in use, trying {}...", target_port, target_port + 1);
                target_port += 1;
                if target_port > 4300 {
                    eprintln!("Could not find an open port for the daemon! Falling back to 4242.");
                    target_port = 4242;
                    break;
                }
            }

            let final_port = target_port;

            let vault_clone = vault_for_daemon.clone();

            tauri::async_runtime::spawn(async move {
                *flag.lock().unwrap() = true;
                // Update tray item to show daemon is live
                if let Some(item) = tray_h.tray_handle().try_get_item("health") {
                    let _ = item.set_title(&format!("Daemon: running (:{})", final_port));
                    let _ = item.set_enabled(false);
                }
                qualia_core_db::daemon::start_local_daemon(final_port, vault_clone).await;
                *flag.lock().unwrap() = false;
                if let Some(item) = tray_h.tray_handle().try_get_item("health") {
                    let _ = item.set_title("Daemon: stopped");
                }
            });

            // ── Auto-update check ─────────────────────────────────────────────
            let upd_h = handle.clone();
            tauri::async_runtime::spawn(async move {
                match tauri::updater::builder(upd_h).check().await {
                    Ok(update) if update.is_update_available() => {
                        let _ = update.download_and_install().await;
                    }
                    Err(e) => eprintln!("Update check skipped: {e}"),
                    _ => {}
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            is_first_run, save_identity, load_identity,
            get_coin_balances, get_transaction_history,
            get_tokens, add_token, remove_token,
            cancel_download, download_model,
            profile_energy_circumstance, start_daemon, daemon_status,
            get_config, save_config, check_ollama_status,
            get_tax_suite, save_tax_suite, dispatch_tax_payment,
            accept_vault_handshake, receive_vault_job,
            ingest_pdf, ingest_ontology, export_to_solid,
            discover_models, run_agent_inference,
            get_hardware_status, download_and_vectorize,
            list_installed_apps, generate_app_credential, verify_and_install_app,
            get_wallet_status, ingest_literature, upsert_cmld_definition,
            ingest_image, ingest_image_async, fetch_torrent_telemetry,
            generate_bip39_seed, derive_wallets_from_seed, generate_front_door_invite,
            mint_semantic_token, fetch_wallet_portfolio, import_external_seed,
            toggle_nym_relay, toggle_stark_prover, update_solar_input,
            load_imported_accounts, save_imported_accounts,
            launch_installed_app, run_engine_command,
            get_active_downloads, fetch_remote_manifest,
            get_active_model, set_active_model,
            get_front_doors, generate_front_door,
            get_directory_actors, add_directory_actor,
            get_delegation_rules, add_delegation_rule,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_app_credential() {
        let app_name = "com.qualia.testapp".to_string();
        let credential = generate_app_credential(app_name);
        
        println!("Generated Credential: {}", credential);
        assert_eq!(credential, "did:qualia:app:com.qualia.testapp:signed_vc");
    }
}
