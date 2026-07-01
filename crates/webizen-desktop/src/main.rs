#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::path::PathBuf;
use tauri::menu::{MenuBuilder, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;

pub mod commands;
pub mod runtime;
pub mod settings_server;
pub mod telemetry_bridge;
pub mod telemetry_hooks;
pub use commands::*;

use qualia_client_core::qapp_registry::QAPPS_DIR;
use qualia_client_core::state::{dirs_default_path, init_app_state};
use runtime::{spawn_runtime, RuntimeHandle};

type ProtocolResponse = tauri::http::Response<Vec<u8>>;

fn protocol_response(status: u16, mime: Option<&str>, body: Vec<u8>) -> ProtocolResponse {
    let mut builder = tauri::http::Response::builder().status(status);
    if let Some(mime) = mime {
        builder = builder.header(tauri::http::header::CONTENT_TYPE, mime);
    }
    builder
        .body(body)
        .expect("static protocol response metadata is valid")
}

fn diffusion_frame_response(app: &tauri::AppHandle, slot: u8) -> ProtocolResponse {
    match app.try_state::<RuntimeHandle>() {
        Some(runtime) => match runtime.frame_rgba(slot) {
            Some(frame) => protocol_response(200, Some("application/octet-stream"), frame),
            None => protocol_response(404, None, Vec::new()),
        },
        None => protocol_response(503, None, Vec::new()),
    }
}

fn render_preview_response(app: &tauri::AppHandle) -> ProtocolResponse {
    match app.try_state::<PreviewState>() {
        Some(state) => {
            let bytes = state
                .png
                .lock()
                .map(|guard| guard.clone())
                .unwrap_or_default();
            if bytes.is_empty() {
                protocol_response(404, None, Vec::new())
            } else {
                protocol_response(200, Some("image/png"), bytes)
            }
        }
        None => protocol_response(503, None, Vec::new()),
    }
}

fn webizen_protocol_response(
    app: &tauri::AppHandle,
    request: &tauri::http::Request<Vec<u8>>,
) -> ProtocolResponse {
    let path = request.uri().path().trim_start_matches('/');
    let segments: Vec<&str> = path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();

    match segments.as_slice() {
        ["diffusion", "frame", slot] => match slot.parse::<u8>() {
            Ok(slot) => diffusion_frame_response(app, slot),
            Err(_) => protocol_response(400, None, Vec::new()),
        },
        ["render", "preview.png"] => render_preview_response(app),
        _ => protocol_response(404, None, Vec::new()),
    }
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn main() {
    let app_state = init_app_state();
    let default_config = app_state.config.lock().unwrap().clone();
    let daemon_flag = app_state.daemon_running.clone();

    let vault_for_daemon = app_state.key_vault.clone();

    let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(100);

    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .register_uri_scheme_protocol("qualia", move |_app, request| {
            let path = request.uri().path().trim_start_matches('/');
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
                    protocol_response(200, Some(mime.as_ref()), data)
                }
                Err(_) => protocol_response(404, None, Vec::new()),
            }
        })
        .register_uri_scheme_protocol("webizen", move |context, request| {
            webizen_protocol_response(context.app_handle(), &request)
        })
        .manage(app_state.clone())
        .manage(PreviewState::default())
        .manage(RenderLoopState(std::sync::Arc::new(
            std::sync::atomic::AtomicBool::new(false),
        )))
        .manage(commands::ActiveAnchor(std::sync::Arc::new(
            std::sync::Mutex::new(None),
        )))
        .manage(commands::TemporalSlice(std::sync::Arc::new(
            std::sync::atomic::AtomicU64::new(0.0_f64.to_bits()),
        )))
        .manage(commands::binary_registry::BinaryNodeRegistry::new())
        .manage(telemetry_bridge::TelemetryBridge::new())
        .manage(commands::HostApiState(std::sync::Arc::new(std::sync::Mutex::new(None))))
        .setup(move |app| {
            let handle = app.handle();
            let daemon_status_item =
                MenuItem::with_id(app, "daemon_status", "Daemon Status", true, None::<&str>)?;
            let tray_menu = MenuBuilder::new(app)
                .text("show", "Open Webizen Studio")
                .separator()
                .text("settings", "Settings")
                .text("logs", "View Logs")
                .text("localhost_preview", "Open Settings Portal")
                .separator()
                .text("revoke", "Revoke Sessions")
                .separator()
                .item(&daemon_status_item)
                .text("toggle_ambient", "Toggle Ambient Visualization")
                .separator()
                .text("quit", "Quit")
                .build()?;
            let tx_for_tray = tx.clone();
            TrayIconBuilder::with_id("main")
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .on_menu_event(move |app, event| match event.id().as_ref() {
                    "show" | "logs" => show_main_window(app),
                    "settings" => {
                        show_main_window(app);
                        let _ = app.emit("open-settings", "settings");
                    }
                    "localhost_preview" => {
                        let _ = open::that("http://127.0.0.1:8080/");
                    }
                    "revoke" => {
                        let _ = tx_for_tray.try_send("REVOKE".to_string());
                    }
                    "toggle_ambient" => {
                        let _ = tx_for_tray.try_send("TOGGLE_AMBIENT".to_string());
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_main_window(tray.app_handle());
                    }
                })
                .build(app)?;

            let runtime_handle = spawn_runtime(handle.clone(), app_state.clone())
                .map_err(|err| -> Box<dyn std::error::Error> { err.into() })?;
            app.manage(runtime_handle);

            if let Err(e) = qualia_client_core::api::start_qualia_protocol() {
                eprintln!("Qapp loopback asset server failed to start: {e}");
            }

            qualia_client_core::local_job_scheduler::LocalJobScheduler::spawn_global_worker();

            let settings_port = settings_server::spawn_settings_server(app_state.clone());
            eprintln!("Settings portal ready at http://127.0.0.1:{settings_port}/");

            if let Ok(kv) = app_state.key_vault.lock() {
                if !kv.is_locked() {
                    let key_bytes = kv.get_master_key_bytes();
                    let signing_key = ed25519_dalek::SigningKey::from_bytes(&key_bytes);
                    let author_did_hash = qualia_core_db::q_hash("did:q42:local");
                    let owner_did = "did:q42:wellfair:owner".to_string();
                    let author_did = owner_did.clone();
                    let storage_path = app_state.config.lock().unwrap().storage_path.clone();
                    let wal_path =
                        std::path::PathBuf::from(storage_path).join("qualia_global.wal");
                    if let Ok(vault) = qualia_client_core::wellfair::vault::VaultService::open(
                        &wal_path,
                        author_did_hash,
                    ) {
                        let policy =
                            qualia_client_core::wellfair::policy::PolicyDecisionService::new();
                        let host_api = qualia_client_core::wellfair::api::WebizenHostApi::new(
                            vault,
                            policy,
                            signing_key,
                            owner_did,
                            author_did,
                        );
                        let state_arc = app.state::<commands::HostApiState>().0.clone();
                        if let Ok(mut host_guard) = state_arc.lock() {
                            *host_guard = Some(host_api);
                        };
                    }
                }
            }

            // Cold-path essentials: seed bundled ontologies when the queue is idle.
            if let Ok(job) = qualia_client_core::local_job_scheduler::LocalJobScheduler::global()
                .enqueue(qualia_client_core::local_job_scheduler::LocalJobKind::BundledOntologySeed {
                    ontology_id: None,
                })
            {
                eprintln!("Queued startup job: bundled ontology seed ({})", job.id);
            }

            // ── Phase 10: Native Hardware Orchestration (sysinfo) ─────────────
            let telemetry_handle = handle.clone();
            tauri::async_runtime::spawn(async move {
                let mut sys = sysinfo::System::new_all();
                loop {
                    sys.refresh_cpu_usage();
                    sys.refresh_memory();

                    let cpu_usage = sys.global_cpu_usage();
                    let mem_used = sys.used_memory() as f64 / 1024.0 / 1024.0 / 1024.0;

                    let _ = telemetry_handle.emit(
                        "hardware-telemetry",
                        serde_json::json!({
                            "cpu": format!("{:.1}%", cpu_usage),
                            "ram": format!("{:.2} GB", mem_used)
                        }),
                    );

                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            });

            // ── Spatial render preview daemon (toggle via toggle_render_loop) ─
            let render_daemon_handle = handle.clone();
            tauri::async_runtime::spawn(async move {
                use std::sync::atomic::Ordering;
                loop {
                    let active = render_daemon_handle
                        .try_state::<RenderLoopState>()
                        .map(|s| s.0.load(Ordering::SeqCst))
                        .unwrap_or(false);
                    if active {
                        let _ = commands::render_preview_tick(&render_daemon_handle).await;
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(750)).await;
                }
            });

            // ── Periodic Telemetry Collection for Ambient Visualization ───────
            let bridge_handle_telemetry = handle.clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    // Only collect and update if ambient visualization is enabled
                    if let Some(bridge) =
                        bridge_handle_telemetry.try_state::<telemetry_bridge::TelemetryBridge>()
                    {
                        if bridge.is_enabled() {
                            // Collect system telemetry (stack-allocated operation)
                            let telemetry = crate::telemetry_hooks::collect_system_telemetry();

                            // Update the bridge state (minimal CPU overhead)
                            bridge.set_telemetry(telemetry);
                        }
                    }

                    // Update at 30 FPS for smooth visualization (33.33ms)
                    tokio::time::sleep(std::time::Duration::from_millis(33)).await;
                }
            });

            // ── Start daemon ──────────────────────────────────────────────────
            // ── Start daemon ──────────────────────────────────────────────────────────
            let flag = daemon_flag.clone();
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
            qualia_client_core::api::set_active_daemon_port(final_port);

            let vault_clone = vault_for_daemon.clone();
            let daemon_status_for_runtime = daemon_status_item.clone();

            tauri::async_runtime::spawn(async move {
                *flag.lock().unwrap() = true;
                let _ = daemon_status_for_runtime
                    .set_text(format!("Daemon: running (:{final_port})"));

                let control_tx = qualia_core_db::daemon::start_local_daemon_with_options(
                    final_port,
                    false,
                    vault_clone,
                    false,
                )
                .await;

                // Forward tray commands to daemon
                while let Some(cmd) = rx.recv().await {
                    // Ambient toggle temporarily disabled - commands not properly configured
                    // if cmd == "TOGGLE_AMBIENT" {
                    //     if let Some(bridge) =
                    //         bridge_handle_tray.try_state::<telemetry_bridge::TelemetryBridge>()
                    //     {
                    //         let new_state = bridge.toggle();
                    //         eprintln!(
                    //             "Ambient visualization: {}",
                    //             if new_state { "enabled" } else { "disabled" }
                    //         );
                    //     }
                    // }
                    let _ = control_tx.send(cmd).await;
                }

                *flag.lock().unwrap() = false;
                let _ = daemon_status_for_runtime.set_text("Daemon: stopped");
            });

            // ── Auto-update check ─────────────────────────────────────────────
            let upd_h = handle.clone();
            tauri::async_runtime::spawn(async move {
                let updater = match upd_h.updater() {
                    Ok(updater) => updater,
                    Err(error) => {
                        eprintln!("Update check skipped: {error}");
                        return;
                    }
                };
                match updater.check().await {
                    Ok(Some(update)) => {
                        let _ = update.download_and_install(|_, _| {}, || {}).await;
                    }
                    Err(error) => eprintln!("Update check skipped: {error}"),
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
