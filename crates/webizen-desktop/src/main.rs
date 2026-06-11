#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::path::PathBuf;
use tauri::{Manager, SystemTray, SystemTrayMenu, SystemTrayMenuItem, CustomMenuItem, SystemTrayEvent};

pub mod commands;
pub use commands::*;

pub use commands::*;
use qualia_client_core::qapp_registry::QAPPS_DIR;
use qualia_client_core::state::{dirs_default_path, init_app_state, AppState};

fn main() {
    let app_state = init_app_state();
    let default_config = app_state.config.lock().unwrap().clone();
    let daemon_flag = app_state.daemon_running.clone();
    let vault_for_daemon = app_state.key_vault.clone();

    let vault_for_daemon = app_state.key_vault.clone();

    // Create system tray menu
    let show = CustomMenuItem::new("show".to_string(), "Show Window");
    let hide = CustomMenuItem::new("hide".to_string(), "Hide Window");
    let settings = CustomMenuItem::new("settings".to_string(), "Settings");
    let logs = CustomMenuItem::new("logs".to_string(), "View Logs");
    let daemon_status = CustomMenuItem::new("daemon_status".to_string(), "Daemon Status");
    let quit = CustomMenuItem::new("quit".to_string(), "Quit");

    let tray_menu = SystemTrayMenu::new()
        .add_item(show)
        .add_item(hide)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(settings)
        .add_item(logs)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(daemon_status)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(quit);

    let system_tray = SystemTray::new().with_menu(tray_menu);

    tauri::Builder::default()
        .system_tray(system_tray)
        .on_system_tray_event(|app, event| match event {
            SystemTrayEvent::LeftClick {
                position: _,
                size: _,
                ..
            } => {
                let window = app.get_window("main").unwrap();
                if window.is_visible().unwrap() {
                    let _ = window.hide();
                } else {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            SystemTrayEvent::MenuItemClick { id, .. } => {
                let window = app.get_window("main").unwrap();
                match id.as_str() {
                    "show" => {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                    "hide" => {
                        let _ = window.hide();
                    }
                    "settings" => {
                        let _ = window.show();
                        let _ = window.set_focus();
                        // Navigate to settings page
                        let _ = window.eval("window.location.href = '/settings'");
                    }
                    "logs" => {
                        let _ = window.show();
                        let _ = window.set_focus();
                        // Navigate to settings page and show logs
                        let _ = window.eval("window.location.href = '/settings'; setTimeout(() => { document.querySelector('button:nth-child(2)')?.click(); }, 100)");
                    }
                    "daemon_status" => {
                        // Show daemon status in a simple alert
                        let _ = window.show();
                        let _ = window.set_focus();
                        let _ = window.eval("alert('Daemon Status: Running on port 4242\\n\\nThis feature will show detailed daemon health information in a future update.')");
                    }
                    "quit" => {
                        std::process::exit(0);
                    }
                    _ => {}
                }
            }
            _ => {}
        })
        .register_uri_scheme_protocol("qualia", move |_app, request| {
            let path = request
                .uri()
                .strip_prefix("qualia://localhost/")
                .unwrap_or("");
            let safe_path: PathBuf = PathBuf::from(path)
                .components()
                .filter(|c| matches!(c, std::path::Component::Normal(_)))
                .collect();
            let full_path = PathBuf::from(dirs_default_path())
                .join(QAPPS_DIR)
                .join(safe_path);

            match std::fs::read(&full_path) {
                Ok(data) => {
                    let mime = mime_guess::from_path(&full_path).first_or_octet_stream();
                    tauri::http::ResponseBuilder::new()
                        .mimetype(mime.as_ref())
                        .status(200)
                        .body(data)
                }
                Err(_) => tauri::http::ResponseBuilder::new()
                    .status(404)
                    .body(Vec::new()),
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

            if let Err(e) = qualia_client_core::api::start_qualia_protocol() {
                eprintln!("Qapp loopback asset server failed to start: {e}");
            }

            // ── Phase 10: Native Hardware Orchestration (sysinfo) ─────────────
            let telemetry_handle = handle.clone();
            tauri::async_runtime::spawn(async move {
                let mut sys = sysinfo::System::new_all();
                loop {
                    sys.refresh_cpu();
                    sys.refresh_memory();

                    let cpu_usage = sys.global_cpu_info().cpu_usage();
                    let mem_used = sys.used_memory() as f64 / 1024.0 / 1024.0 / 1024.0;

                    let _ = telemetry_handle.emit_all(
                        "hardware-telemetry",
                        serde_json::json!({
                            "cpu": format!("{:.1}%", cpu_usage),
                            "ram": format!("{:.2} GB", mem_used)
                        }),
                    );

                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            });

            // ── Start daemon ──────────────────────────────────────────────────
            // ── Start daemon ──────────────────────────────────────────────────────────
            let flag = daemon_flag.clone();
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
                eprintln!(
                    "Port {} is in use, trying {}...",
                    target_port,
                    target_port + 1
                );
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
                if let Some(item) = tray_h.tray_handle().try_get_item("daemon_status") {
                    let _ = item.set_title(&format!("Daemon: running (:{})", final_port));
                }
                qualia_core_db::daemon::start_local_daemon(final_port, vault_clone).await;
                *flag.lock().unwrap() = false;
                if let Some(item) = tray_h.tray_handle().try_get_item("daemon_status") {
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
    fn test_generate_qapp_credential() {
        let qapp_name = "com.qualia.testqapp".to_string();
        let credential = generate_qapp_credential(qapp_name);

        println!("Generated Credential: {}", credential);
        assert_eq!(credential, "did:qualia:qapp:com.qualia.testqapp:signed_vc");
    }
}