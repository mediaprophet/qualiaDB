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



pub mod commands;
pub use commands::*;

use qualia_client_core::state::{init_app_state, dirs_default_path, AppState};
pub use commands::*;

fn main() {
    let app_state = init_app_state();
    let default_config = app_state.config.lock().unwrap().clone();
    let daemon_flag = app_state.daemon_running.clone();
    let vault_for_daemon = app_state.key_vault.clone();

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
        .invoke_handler(commands::get_invoke_handler())
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
