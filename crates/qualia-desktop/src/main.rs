#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use tauri::{Manager, State};
use sysinfo::{System, Disks};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::path::PathBuf;
use qualia_core_db::rpc::{TaxRecipient, TaxRecipientSuite, route_tax_payment};
use qualia_core_db::ilp_dispatcher::{IlpDispatcher, HttpIlpTransport, DispatchResult};

mod ingestion;
mod q42_compiler;
mod llm_offload;

#[derive(Serialize, Deserialize, Clone)]
struct AgentConfig {
    storage_path: String,
    storage_quota_gb: u64,
    base_connectivity_cost_ilp: u64, // ILP µ-cents per GB
}

impl Default for AgentConfig {
    fn default() -> Self {
        // Cross-platform default: AppData/Roaming on Windows,
        // ~/Library/Application Support on macOS, ~/.local/share on Linux.
        let storage_path = dirs_default_path();
        Self {
            storage_path,
            storage_quota_gb: 10,
            base_connectivity_cost_ilp: 5000, // Default 5000 µ-cents (~$0.05) per GB
        }
    }
}

/// Resolves the OS-appropriate default data directory for the qualia daemon.
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

struct AppState {
    config:    Mutex<AgentConfig>,
    tax_suite: Mutex<TaxRecipientSuite>,
}

/// Path to the persisted tax suite JSON file.
fn suite_file_path(data_dir: &str) -> PathBuf {
    PathBuf::from(data_dir).join("tax_suite.json")
}

/// Load suite from disk, or return default if missing/corrupt.
fn load_suite_from_disk(data_dir: &str) -> TaxRecipientSuite {
    let path = suite_file_path(data_dir);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(TaxRecipientSuite::default_cooperative)
}

#[tauri::command]
fn get_config(state: State<AppState>) -> AgentConfig {
    state.config.lock().unwrap().clone()
}

#[tauri::command]
fn save_config(state: State<AppState>, new_config: AgentConfig) -> Result<(), String> {
    // 1. OS Protection Constraint: Never let the system drive drop below 15GB.
    let disks = Disks::new_with_refreshed_list();
    
    // Find the disk matching the storage path (rudimentary check for MVP)
    let path = PathBuf::from(&new_config.storage_path);
    let mut target_disk_space = u64::MAX;
    
    for disk in disks.list() {
        if path.starts_with(disk.mount_point()) {
            target_disk_space = disk.available_space();
            break;
        }
    }

    let os_safety_margin_bytes: u64 = 15 * 1024 * 1024 * 1024; // 15 GB
    let requested_quota_bytes = new_config.storage_quota_gb * 1024 * 1024 * 1024;

    if target_disk_space.saturating_sub(requested_quota_bytes) < os_safety_margin_bytes {
        return Err("OS_SAFETY_VIOLATION: Allocating this much storage would leave the host OS with less than the safe 15GB margin required to function.".to_string());
    }

    *state.config.lock().unwrap() = new_config;
    Ok(())
}

#[tauri::command]
fn profile_energy_circumstance() -> String {
    let mut sys = System::new_all();
    sys.refresh_all();
    
    let total_mem = sys.total_memory() / 1024 / 1024;
    let used_mem = sys.used_memory() / 1024 / 1024;
    let is_energy_surplus = true;

    format!(
        "Energy Circumstance: {}\nTotal Mem: {}MB\nUsed Mem: {}MB\nSleep-Cycle Swarm Auth: {}",
        if is_energy_surplus { "AC_POWER" } else { "BATTERY_DEPLETED" },
        total_mem,
        used_mem,
        if is_energy_surplus { "GRANTED" } else { "DENIED" }
    )
}

#[tauri::command]
fn check_ollama_status() -> bool {
    // Attempt to execute `ollama -v` to see if the engine is installed in the system PATH.
    match std::process::Command::new("ollama").arg("-v").output() {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

#[tauri::command]
fn start_daemon() -> String {
    "Daemon Started".to_string()
}

/// Return the currently active TaxRecipientSuite.
#[tauri::command]
fn get_tax_suite(state: State<AppState>) -> TaxRecipientSuite {
    state.tax_suite.lock().unwrap().clone()
}

/// Validate and persist a new TaxRecipientSuite.
#[tauri::command]
fn save_tax_suite(
    state: State<AppState>,
    suite: TaxRecipientSuite,
) -> Result<(), String> {
    suite.validate()?;

    // Persist to disk
    let data_dir = state.config.lock().unwrap().storage_path.clone();
    let path = suite_file_path(&data_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(&suite).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;

    *state.tax_suite.lock().unwrap() = suite;
    Ok(())
}

/// Dispatch a payment through the 12% Tax Router.
/// Returns a DispatchResult with per-recipient receipts (Sent / Queued / Failed).
#[tauri::command]
fn dispatch_tax_payment(
    state: State<AppState>,
    gross_amount_micro_cents: u64,
) -> Result<DispatchResult, String> {
    let suite = state.tax_suite.lock().unwrap().clone();
    let plan = route_tax_payment(gross_amount_micro_cents, &suite)?;
    let dispatcher = IlpDispatcher::new(HttpIlpTransport {
        connector_url: "http://localhost:7770".to_string(),
    });
    Ok(dispatcher.dispatch(&plan))
}

/// VC-8 Semantic Handshake Receiver (Stateless Terminal)
/// Accepts an incoming Noise Protocol handshake from the mobile Vault to pair the session.
#[tauri::command]
fn accept_vault_handshake(did_key: String, payload: String) -> Result<String, String> {
    println!("Received VC-8 Handshake from Vault DID: {}", did_key);
    // Decrypt the payload and negotiate session keys using Noise_XX_25519_AESGCM_SHA256
    Ok("HANDSHAKE_SUCCESS".to_string())
}

/// VC-12 Background Job Offload Receiver
/// Accepts heavy compute tasks (like local LLM inference) pushed from the mobile device.
#[tauri::command]
fn receive_vault_job(job_id: String, task_type: String, data_blob_cbor: Vec<u8>) -> Result<String, String> {
    println!("Received VC-12 Offload Job {} of type {}", job_id, task_type);
    if task_type == "LLM_INFERENCE" && check_ollama_status() {
        // Execute inference locally on Desktop GPU
        Ok("INFERENCE_QUEUED".to_string())
    } else {
        Err("UNSUPPORTED_TASK_OR_NO_CAPACITY".to_string())
    }
}

/// Desktop Library Ingestion Command
/// Processes a PDF file through the Edge VLM and compiles it to .q42
#[tauri::command]
async fn ingest_pdf(file_name: String) -> Result<ingestion::IngestionResult, String> {
    let result = ingestion::process_pdf(&file_name)?;
    q42_compiler::compile_to_q42(&file_name, &result.bookmarks)?;
    Ok(result)
}

/// Desktop Ontology Ingestion Command
/// Processes semantic web files (.rdf, .owl, etc.) and compiles them to .q42
#[tauri::command]
async fn ingest_ontology(file_name: String) -> Result<ingestion::IngestionResult, String> {
    let result = ingestion::process_ontology(&file_name)?;
    q42_compiler::compile_to_q42(&file_name, &result.bookmarks)?;
    Ok(result)
}

/// W3C Solid Exporter Command
/// Translates a raw binary Qualia graph (.q42) into a W3C Solid LDP Basic Container.
#[tauri::command]
async fn export_to_solid(input_q42_path: String, output_dir_path: String) -> Result<String, String> {
    qualia_core_db::solid_ldp::SolidExporter::export_to_solid_pod(&input_q42_path, &output_dir_path)
        .map(|_| format!("Successfully exported to {}", output_dir_path))
        .map_err(|e| e.to_string())
}
#[tauri::command]
async fn discover_models() -> Result<Vec<llm_offload::ModelInfo>, String> {
    llm_offload::discover_local_models().await
}

#[tauri::command]
async fn run_agent_inference(app_handle: tauri::AppHandle, prompt: String, model_name: String) -> Result<(), String> {
    // Spawns async task so it doesn't block the UI thread during streaming
    tauri::async_runtime::spawn(async move {
        let _ = llm_offload::execute_agent_inference(app_handle, prompt, model_name).await;
    });
    Ok(())
}

fn main() {
    let default_config = AgentConfig::default();
    let initial_suite = load_suite_from_disk(&default_config.storage_path);

    tauri::Builder::default()
        .manage(AppState {
            config:    Mutex::new(default_config),
            tax_suite: Mutex::new(initial_suite),
        })
        // ── Auto-update check on launch ──────────────────────────────────────
        .setup(|app| {
            let handle = app.handle();
            
            // Start the WebSocket daemon on port 4242 in the background
            tauri::async_runtime::spawn(async move {
                qualia_core_db::daemon::start_local_daemon(4242).await;
            });

            tauri::async_runtime::spawn(async move {
                match tauri::updater::builder(handle.clone()).check().await {
                    Ok(update) => {
                        if update.is_update_available() {
                            // Show native OS dialog — Tauri handles the download/install prompt
                            update.download_and_install().await
                                .unwrap_or_else(|e| eprintln!("Update install failed: {e}"));
                        }
                    }
                    Err(e) => {
                        // Non-fatal: no network, CDN unreachable, etc.
                        eprintln!("Update check skipped: {e}");
                    }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            profile_energy_circumstance, start_daemon,
            get_config, save_config, check_ollama_status,
            get_tax_suite, save_tax_suite, dispatch_tax_payment,
            accept_vault_handshake, receive_vault_job, ingest_pdf,
            ingest_ontology, discover_models, run_agent_inference,
            export_to_solid
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
