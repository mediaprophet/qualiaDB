#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use tauri::{
    CustomMenuItem, Manager, State,
    SystemTray, SystemTrayEvent, SystemTrayMenu, SystemTrayMenuItem,
};
use sysinfo::{System, Disks};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use qualia_core_db::rpc::{TaxRecipientSuite, route_tax_payment};
use qualia_core_db::ilp_dispatcher::{IlpDispatcher, HttpIlpTransport, DispatchResult};

mod ingestion;
mod q42_compiler;
mod llm_offload;

// ── Config ────────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
struct AgentConfig {
    storage_path: String,
    storage_quota_gb: u64,
    base_connectivity_cost_ilp: u64,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            storage_path: dirs_default_path(),
            storage_quota_gb: 10,
            base_connectivity_cost_ilp: 5000,
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

// ── App state ─────────────────────────────────────────────────────────────────

struct AppState {
    config:         Mutex<AgentConfig>,
    tax_suite:      Mutex<TaxRecipientSuite>,
    daemon_running: Arc<Mutex<bool>>,
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
async fn ingest_ontology(file_name: String) -> Result<ingestion::IngestionResult, String> {
    let result = ingestion::process_ontology(&file_name)?;
    q42_compiler::compile_to_q42(&file_name, &result.bookmarks)?;
    Ok(result)
}

#[tauri::command]
async fn export_to_solid(input_q42_path: String, output_dir_path: String) -> Result<String, String> {
    qualia_core_db::solid_ldp::SolidExporter::export_to_solid_pod(&input_q42_path, &output_dir_path)
        .map(|_| format!("Exported to {}", output_dir_path))
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn discover_models() -> Result<Vec<llm_offload::ModelInfo>, String> {
    llm_offload::discover_local_models().await
}

#[tauri::command]
async fn run_agent_inference(
    app_handle: tauri::AppHandle,
    prompt: String,
    model_name: String,
) -> Result<(), String> {
    tauri::async_runtime::spawn(async move {
        let _ = llm_offload::execute_agent_inference(app_handle, prompt, model_name).await;
    });
    Ok(())
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

// ── main ──────────────────────────────────────────────────────────────────────

fn main() {
    let default_config  = AgentConfig::default();
    let initial_suite   = load_suite_from_disk(&default_config.storage_path);
    let daemon_running  = Arc::new(Mutex::new(false));
    let daemon_flag     = daemon_running.clone();

    tauri::Builder::default()
        .manage(AppState {
            config:         Mutex::new(default_config),
            tax_suite:      Mutex::new(initial_suite),
            daemon_running,
        })
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

            // ── Start daemon ──────────────────────────────────────────────────
            let flag   = daemon_flag.clone();
            let tray_h = handle.clone();
            tauri::async_runtime::spawn(async move {
                *flag.lock().unwrap() = true;
                // Update tray item to show daemon is live
                if let Some(item) = tray_h.tray_handle().try_get_item("health") {
                    let _ = item.set_title("Daemon: running (:4242)");
                    let _ = item.set_enabled(false);
                }
                qualia_core_db::daemon::start_local_daemon(4242).await;
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
            profile_energy_circumstance, start_daemon, daemon_status,
            get_config, save_config, check_ollama_status,
            get_tax_suite, save_tax_suite, dispatch_tax_payment,
            accept_vault_handshake, receive_vault_job,
            ingest_pdf, ingest_ontology, export_to_solid,
            discover_models, run_agent_inference,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
