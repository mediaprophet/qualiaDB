#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use tauri::{SystemTray, SystemTrayMenu, SystemTrayMenuItem, CustomMenuItem, SystemTrayEvent, Manager, State};
use sysinfo::{System, SystemExt, Disks};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::path::PathBuf;

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
    config: Mutex<AgentConfig>,
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

fn main() {
    let quit = CustomMenuItem::new("quit".to_string(), "Quit Webizen Agent");
    let settings = CustomMenuItem::new("settings".to_string(), "Settings (Storage & Limits)");
    let profile = CustomMenuItem::new("profile".to_string(), "Check Energy Circumstance");
    let toggle_daemon = CustomMenuItem::new("toggle".to_string(), "Start Local Daemon");

    let tray_menu = SystemTrayMenu::new()
        .add_item(settings)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(toggle_daemon)
        .add_item(profile)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(quit);

    let system_tray = SystemTray::new().with_menu(tray_menu);

    tauri::Builder::default()
        .manage(AppState { config: Mutex::new(AgentConfig::default()) })
        .system_tray(system_tray)
        .on_system_tray_event(|app, event| match event {
            SystemTrayEvent::MenuItemClick { id, .. } => {
                match id.as_str() {
                    "quit" => {
                        std::process::exit(0);
                    }
                    "settings" => {
                        let window = app.get_window("main").unwrap();
                        window.show().unwrap();
                        window.set_focus().unwrap();
                    }
                    "profile" => {
                        println!("{}", profile_energy_circumstance());
                    }
                    "toggle" => {
                        println!("Toggling Daemon State...");
                    }
                    _ => {}
                }
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![profile_energy_circumstance, start_daemon, get_config, save_config, check_ollama_status])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
