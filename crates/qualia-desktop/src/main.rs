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
            get_tax_suite, save_tax_suite, dispatch_tax_payment
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
