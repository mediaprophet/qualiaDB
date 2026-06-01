#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use tauri::{SystemTray, SystemTrayMenu, SystemTrayMenuItem, CustomMenuItem, SystemTrayEvent, Manager};
use sysinfo::{System, SystemExt};

#[tauri::command]
fn profile_energy_circumstance() -> String {
    // 1. Gather System Information (Mocking advanced battery profiling for MVP)
    let mut sys = System::new_all();
    sys.refresh_all();
    
    // In a production environment, we'd use the `battery` crate or read /sys/class/power_supply
    // For now, we simulate the profile check using basic sysinfo.
    let total_mem = sys.total_memory() / 1024 / 1024; // MB
    let used_mem = sys.used_memory() / 1024 / 1024; // MB
    
    let is_energy_surplus = true; // Assume AC Power for simulation

    format!(
        "Energy Circumstance: {}\nTotal Mem: {}MB\nUsed Mem: {}MB\nSleep-Cycle Swarm Auth: {}",
        if is_energy_surplus { "AC_POWER" } else { "BATTERY_DEPLETED" },
        total_mem,
        used_mem,
        if is_energy_surplus { "GRANTED" } else { "DENIED" }
    )
}

#[tauri::command]
fn start_daemon() -> String {
    // This would initialize the core Qualia-DB daemon on 127.0.0.1:4848
    "Daemon Started".to_string()
}

fn main() {
    let quit = CustomMenuItem::new("quit".to_string(), "Quit Webizen Agent");
    let show = CustomMenuItem::new("show".to_string(), "Open Dashboard");
    let profile = CustomMenuItem::new("profile".to_string(), "Check Energy Circumstance");
    let toggle_daemon = CustomMenuItem::new("toggle".to_string(), "Start Local Daemon");

    let tray_menu = SystemTrayMenu::new()
        .add_item(show)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(toggle_daemon)
        .add_item(profile)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(quit);

    let system_tray = SystemTray::new().with_menu(tray_menu);

    tauri::Builder::default()
        .system_tray(system_tray)
        .on_system_tray_event(|app, event| match event {
            SystemTrayEvent::MenuItemClick { id, .. } => {
                match id.as_str() {
                    "quit" => {
                        std::process::exit(0);
                    }
                    "show" => {
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
        .invoke_handler(tauri::generate_handler![profile_energy_circumstance, start_daemon])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
